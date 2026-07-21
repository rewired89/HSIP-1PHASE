//! Aggregates conditions that need a human operator's attention — the
//! things the rest of this codebase can detect but cannot fix by itself.
//!
//! Exists because "can this recover automatically?" has real, honest "no"
//! answers in a few places in this codebase (an incomplete master key
//! rotation, a node with zero root-admin keys, an anchor batch that gave up
//! being auto-upgraded), and HSIP has no push-based alerting of its own —
//! it's a self-hosted product, not a service that emails or pages anyone.
//! Without something like this, an operator (whether that's one person
//! running HSIP on a desktop, or a business running it behind real
//! monitoring) would only discover these states by directly reading the
//! database or grepping logs. See `routes::admin::system_health` for the
//! `GET /v1/admin/system-health` endpoint this backs, and
//! `metrics::SYSTEM_HEALTH_ISSUES` for the Prometheus-visible counterpart.

use sqlx::Row;

use crate::anchor_job::MAX_PENDING_UPGRADE_AGE_MS;
use crate::db::{now_ms, Db};

#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthIssue {
    pub code: &'static str,
    /// `"critical"` (the system cannot recover from this on its own and an
    /// operator should act) or `"warning"` (self-contained, informational —
    /// nothing is broken, but worth knowing about).
    pub severity: &'static str,
    pub summary: String,
    pub detail: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SystemHealth {
    /// `true` only when `issues` is empty. Deliberately simple — the
    /// `severity` field on each issue is what lets a caller decide how
    /// urgently to react; this flag just answers "is there anything to
    /// look at at all."
    pub healthy: bool,
    pub checked_at_ms: i64,
    pub issues: Vec<HealthIssue>,
}

/// Runs every check and returns the aggregated result. Cheap enough to call
/// on demand (a filesystem stat and a couple of `COUNT(*)` queries) — see
/// `routes::admin::system_health` for the live, on-request path, and
/// `main.rs` for the periodic background refresh that keeps
/// `metrics::SYSTEM_HEALTH_ISSUES` current even when nobody's polling the
/// API.
pub async fn check(db: &Db, master_key_path: Option<&str>) -> SystemHealth {
    let mut issues = Vec::new();

    if let Some(issue) = check_master_key_rotation_incomplete(master_key_path) {
        issues.push(issue);
    }
    if let Some(issue) = check_zero_root_admins(db).await {
        issues.push(issue);
    }
    if let Some(issue) = check_abandoned_ots_anchors(db).await {
        issues.push(issue);
    }

    SystemHealth {
        healthy: issues.is_empty(),
        checked_at_ms: now_ms(),
        issues,
    }
}

/// Same as [`check`], but also refreshes `metrics::SYSTEM_HEALTH_ISSUES` so
/// `/metrics` reflects the current state. Used by both the on-demand
/// `GET /v1/admin/system-health` route and the periodic background refresh
/// in `main.rs` — kept separate from `check` itself so the pure check logic
/// stays easy to unit test without touching global metric state.
pub async fn check_and_update_metrics(db: &Db, master_key_path: Option<&str>) -> SystemHealth {
    let health = check(db, master_key_path).await;

    let critical = health
        .issues
        .iter()
        .filter(|i| i.severity == "critical")
        .count();
    let warning = health
        .issues
        .iter()
        .filter(|i| i.severity == "warning")
        .count();
    crate::metrics::SYSTEM_HEALTH_ISSUES
        .with_label_values(&["critical"])
        .set(critical as f64);
    crate::metrics::SYSTEM_HEALTH_ISSUES
        .with_label_values(&["warning"])
        .set(warning as f64);

    health
}

/// `routes::admin::rotate_master_key` writes a `{path}.rotating` staging
/// file, commits the DB under the new key, then renames the staging file
/// onto `path` — deliberately left in place if that rename fails, so a
/// crash mid-rotation is recoverable rather than silently losing the new
/// key. A leftover staging file means that recovery step never happened:
/// the database is re-encrypted under a key this process doesn't currently
/// have on disk under its real path.
fn check_master_key_rotation_incomplete(master_key_path: Option<&str>) -> Option<HealthIssue> {
    let path = master_key_path?;
    let staging_path = format!("{path}.rotating");
    if !std::path::Path::new(&staging_path).exists() {
        return None;
    }
    Some(HealthIssue {
        code: "master_key_rotation_incomplete",
        severity: "critical",
        summary: "A master key rotation did not finish".to_string(),
        detail: format!(
            "{staging_path} exists, meaning a rotation committed the database under a new key \
             but crashed before renaming the staging file onto {path}. Verify {staging_path} is \
             the expected new key (its fingerprint should match the rotation response or log \
             line), then move it onto {path} manually. See THREAT_MODEL.md's Master Key \
             Rotation section for the full recovery story."
        ),
    })
}

/// `routes::admin::require_root_admin` gates master key rotation and
/// root-admin grant/revoke on `is_root_admin = 1`. If that ever reaches
/// zero — which the grant/revoke endpoints already refuse to allow via
/// their own API, but direct database tampering could still cause — there
/// is no recovery path through the API at all, only editing `api_keys`
/// directly. Worth surfacing loudly rather than only documenting.
async fn check_zero_root_admins(db: &Db) -> Option<HealthIssue> {
    let count: i64 =
        sqlx::query("SELECT COUNT(*) FROM api_keys WHERE is_root_admin = 1 AND active = 1")
            .fetch_one(db)
            .await
            .ok()?
            .try_get(0)
            .ok()?;

    if count > 0 {
        return None;
    }
    Some(HealthIssue {
        code: "zero_root_admins",
        severity: "critical",
        summary: "No active root-admin key exists".to_string(),
        detail: "Master key rotation and root-admin grant/revoke are unreachable through the \
                  API in this state. There is no recovery path except editing the api_keys \
                  table directly to set is_root_admin=1 on some active key."
            .to_string(),
    })
}

/// `anchor_job::upgrade_one_anchor` stops auto-polling a `decision_anchors`/
/// `audit_anchors` row once it's been `ots_status = 'pending'` for longer
/// than `MAX_PENDING_UPGRADE_AGE_MS` — see that module for why (an
/// unbounded retry-forever loop found during a QA review). The anchor data
/// itself is still fully valid; this only means it will never reach
/// `ots_status = 'confirmed'` without a human manually checking it against
/// the calendar.
async fn check_abandoned_ots_anchors(db: &Db) -> Option<HealthIssue> {
    let decision_count = count_stale_pending(db, "decision_anchors").await;
    let audit_count = count_stale_pending(db, "audit_anchors").await;
    let total = decision_count + audit_count;
    if total == 0 {
        return None;
    }
    Some(HealthIssue {
        code: "ots_anchors_abandoned",
        severity: "warning",
        summary: format!("{total} anchor batch(es) stopped being auto-upgraded"),
        detail: format!(
            "{decision_count} decision batch(es) and {audit_count} audit-log batch(es) have \
             been ots_status='pending' for longer than MAX_PENDING_UPGRADE_AGE_MS (7 days) and \
             are no longer automatically re-checked against their calendar. Their signature and \
             Merkle proof still verify — nothing is corrupted — but they will never reach \
             ots_status='confirmed' on their own. See metrics::ANCHOR_UPGRADE_STALE."
        ),
    })
}

/// `table` is always a hardcoded literal from this module's own two call
/// sites, never external input — same no-injection-risk reasoning already
/// applied to `anchor_job::upgrade_one_anchor` and
/// `bin/hsip_migrate.rs`'s table-driven copy.
async fn count_stale_pending(db: &Db, table: &'static str) -> i64 {
    let cutoff = now_ms() - MAX_PENDING_UPGRADE_AGE_MS;
    let sql =
        format!("SELECT COUNT(*) FROM {table} WHERE ots_status = 'pending' AND created_at < $1");
    match sqlx::query(&sql).bind(cutoff).fetch_one(db).await {
        Ok(row) => row.try_get(0).unwrap_or(0),
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_db() -> Db {
        let db_url = format!(
            "sqlite:file:{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        sqlx::any::install_default_drivers();
        crate::db::init(&db_url).await.expect("db init")
    }

    async fn insert_bootstrap_admin(db: &Db) {
        let tenant_id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        sqlx::query("INSERT INTO tenants (id, name, created_at) VALUES ($1, 'default', $2)")
            .bind(&tenant_id)
            .bind(now)
            .execute(db)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO api_keys (id, tenant_id, key_hash, name, agent_type, role, \
             is_root_admin, created_at, active) \
             VALUES ($1, $2, 'hash', 'admin', 'human', 'owner', 1, $3, 1)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&tenant_id)
        .bind(now)
        .execute(db)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn healthy_when_nothing_is_wrong() {
        let db = test_db().await;
        insert_bootstrap_admin(&db).await;

        let health = check(&db, None).await;
        assert!(health.healthy);
        assert!(health.issues.is_empty());
    }

    #[tokio::test]
    async fn detects_zero_root_admins() {
        let db = test_db().await;
        // No bootstrap admin inserted — a node with no root admin at all.

        let health = check(&db, None).await;
        assert!(!health.healthy);
        assert!(health
            .issues
            .iter()
            .any(|i| i.code == "zero_root_admins" && i.severity == "critical"));
    }

    #[tokio::test]
    async fn detects_incomplete_master_key_rotation() {
        let db = test_db().await;
        insert_bootstrap_admin(&db).await;

        let dir = std::env::temp_dir().join(format!("hsip-health-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let key_path = dir.join("master.key");
        let staging_path = dir.join("master.key.rotating");
        std::fs::write(&staging_path, "leftover-staging-key").unwrap();

        let health = check(&db, Some(key_path.to_str().unwrap())).await;
        assert!(!health.healthy);
        assert!(health
            .issues
            .iter()
            .any(|i| i.code == "master_key_rotation_incomplete" && i.severity == "critical"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn detects_abandoned_ots_anchors() {
        let db = test_db().await;
        insert_bootstrap_admin(&db).await;

        let eight_days_ago = now_ms() - 8 * 24 * 60 * 60 * 1000;
        sqlx::query(
            "INSERT INTO decision_anchors \
             (id, merkle_root, leaf_count, anchor_signature, anchor_verify_key, ots_status, created_at) \
             VALUES ($1, 'root', 1, 'sig', 'verify_key', 'pending', $2)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(eight_days_ago)
        .execute(&db)
        .await
        .unwrap();

        let health = check(&db, None).await;
        assert!(!health.healthy);
        assert!(health
            .issues
            .iter()
            .any(|i| i.code == "ots_anchors_abandoned" && i.severity == "warning"));
    }

    #[tokio::test]
    async fn recent_pending_anchor_is_not_flagged() {
        let db = test_db().await;
        insert_bootstrap_admin(&db).await;

        sqlx::query(
            "INSERT INTO decision_anchors \
             (id, merkle_root, leaf_count, anchor_signature, anchor_verify_key, ots_status, created_at) \
             VALUES ($1, 'root', 1, 'sig', 'verify_key', 'pending', $2)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(now_ms())
        .execute(&db)
        .await
        .unwrap();

        let health = check(&db, None).await;
        assert!(health.healthy);
    }

    /// `check_and_update_metrics` must actually reflect the *current* state
    /// in the gauge, not just accumulate — a resolved issue should bring
    /// the count back down, which a plain counter could never do (this is
    /// exactly why `SYSTEM_HEALTH_ISSUES` is a `GaugeVec`, not a
    /// `CounterVec`).
    #[tokio::test]
    async fn check_and_update_metrics_reflects_current_state() {
        let db = test_db().await;
        // No bootstrap admin — one critical issue (zero_root_admins).
        let unhealthy = check_and_update_metrics(&db, None).await;
        assert!(!unhealthy.healthy);
        assert_eq!(
            crate::metrics::SYSTEM_HEALTH_ISSUES
                .with_label_values(&["critical"])
                .get(),
            1.0
        );

        // Resolve it, then re-check — the gauge must drop back to zero.
        insert_bootstrap_admin(&db).await;
        let healthy = check_and_update_metrics(&db, None).await;
        assert!(healthy.healthy);
        assert_eq!(
            crate::metrics::SYSTEM_HEALTH_ISSUES
                .with_label_values(&["critical"])
                .get(),
            0.0
        );
    }
}
