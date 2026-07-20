//! Periodic persistence of the in-memory rate-limit / AI-agent-velocity
//! DashMaps (`AppState.rate_limiter`, `.agent_tracker`, `.sandbox_rate`) so
//! a restart doesn't silently reset abuse-detection counters. A key
//! mid-way toward the 1000 req/min auto-revoke threshold (`auth.rs`) would
//! otherwise get a clean slate on every restart — a real gap for any
//! deployment that restarts often (crashes, container rescheduling,
//! routine deploys), not just a deliberate evasion vector.
//!
//! Deliberately a periodic snapshot, not a write-through on every request:
//! these DashMaps exist specifically so the hot auth path never blocks on
//! the database. Adding a DB write to every single authenticated request
//! would defeat that. Instead the current contents are upserted to the
//! `rate_limit_state` table every [`SNAPSHOT_INTERVAL_SECS`], and reloaded
//! once at startup before the server accepts traffic.
//!
//! **Residual risk, by design:** state since the last snapshot is lost on
//! a crash or unclean restart — this bounds the staleness, it doesn't
//! eliminate it. The windows involved are short (60s for rate limit /
//! velocity, 1h for sandbox provisioning) relative to the snapshot
//! interval, so the practical exposure is small and bounded, the same
//! "narrow, acknowledged, recoverable" tradeoff this codebase already
//! accepts for the master-key-rotation staging-file window and the
//! signing-to-anchoring gap in decision attestations.

use sqlx::Row;
use std::sync::atomic::Ordering;

use crate::{
    db::{now_ms, Db},
    state::{AppState, RateWindow, VelocityRecord},
};

/// How often the in-memory state is flushed to the database.
pub const SNAPSHOT_INTERVAL_SECS: u64 = 30;

const RATE_LIMIT_KIND: &str = "rate_limit";
const AGENT_VELOCITY_KIND: &str = "agent_velocity";
const SANDBOX_RATE_KIND: &str = "sandbox_rate";

// Mirrors auth.rs::WINDOW_MS and routes::sandbox::HOUR_MS. Duplicated
// rather than imported — those are private constants scoped to their own
// modules' window-rollover logic, whereas this is only a "is this
// persisted row still live" check on the way back in, not a shared source
// of truth for the window length itself.
const RATE_WINDOW_MS: i64 = 60_000;
const SANDBOX_WINDOW_MS: i64 = 3_600_000;

/// Loads persisted rate-limit/velocity state back into the in-memory
/// DashMaps. Called once at startup, before the server accepts traffic.
///
/// Rows whose window has already expired are skipped rather than
/// restored — they would reset to a fresh window on first use anyway, so
/// there's nothing meaningful to carry forward.
pub async fn load(db: &Db, state: &AppState) -> anyhow::Result<()> {
    let now = now_ms();
    let rows = sqlx::query(
        "SELECT kind, state_key, count, anomaly_count, window_start_ms FROM rate_limit_state",
    )
    .fetch_all(db)
    .await?;

    let mut restored = 0u64;
    for row in &rows {
        let kind: String = row.try_get(0)?;
        let key: String = row.try_get(1)?;
        let count: i64 = row.try_get(2)?;
        let anomaly_count: i64 = row.try_get(3)?;
        let window_start_ms: i64 = row.try_get(4)?;

        match kind.as_str() {
            RATE_LIMIT_KIND if now - window_start_ms < RATE_WINDOW_MS => {
                state
                    .rate_limiter
                    .insert(key, RateWindow::from_parts(count as u64, window_start_ms));
                restored += 1;
            }
            AGENT_VELOCITY_KIND if now - window_start_ms < RATE_WINDOW_MS => {
                state.agent_tracker.insert(
                    key,
                    VelocityRecord::from_parts(count as u64, anomaly_count as u64, window_start_ms),
                );
                restored += 1;
            }
            SANDBOX_RATE_KIND if now - window_start_ms < SANDBOX_WINDOW_MS => {
                state
                    .sandbox_rate
                    .insert(key, RateWindow::from_parts(count as u64, window_start_ms));
                restored += 1;
            }
            _ => {}
        }
    }

    if restored > 0 {
        tracing::info!(
            restored,
            total_rows = rows.len(),
            "restored rate-limit/velocity state from last snapshot"
        );
    }
    Ok(())
}

/// Snapshots the current contents of all three trackers to the database.
/// One row per live key/IP, upserted — a tracker entry that no longer
/// exists in memory (evicted on revoke, or simply never re-triggered)
/// just leaves its last-known row in place rather than deleting it; a
/// handful of stale rows for inactive keys is harmless and not worth the
/// extra query to prune.
pub async fn snapshot(db: &Db, state: &AppState) -> anyhow::Result<()> {
    let now = now_ms();

    for entry in state.rate_limiter.iter() {
        upsert(
            db,
            RATE_LIMIT_KIND,
            entry.key(),
            entry.value().count.load(Ordering::SeqCst),
            0,
            entry.value().window_start_ms.load(Ordering::SeqCst),
            now,
        )
        .await?;
    }

    for entry in state.agent_tracker.iter() {
        upsert(
            db,
            AGENT_VELOCITY_KIND,
            entry.key(),
            entry.value().request_count.load(Ordering::SeqCst),
            entry.value().anomaly_count.load(Ordering::SeqCst),
            entry.value().window_start_ms.load(Ordering::SeqCst),
            now,
        )
        .await?;
    }

    for entry in state.sandbox_rate.iter() {
        upsert(
            db,
            SANDBOX_RATE_KIND,
            entry.key(),
            entry.value().count.load(Ordering::SeqCst),
            0,
            entry.value().window_start_ms.load(Ordering::SeqCst),
            now,
        )
        .await?;
    }

    Ok(())
}

async fn upsert(
    db: &Db,
    kind: &str,
    key: &str,
    count: u64,
    anomaly_count: u64,
    window_start_ms: i64,
    now: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO rate_limit_state (kind, state_key, count, anomaly_count, window_start_ms, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT (kind, state_key) DO UPDATE SET
           count = excluded.count,
           anomaly_count = excluded.anomaly_count,
           window_start_ms = excluded.window_start_ms,
           updated_at = excluded.updated_at",
    )
    .bind(kind)
    .bind(key)
    .bind(count as i64)
    .bind(anomaly_count as i64)
    .bind(window_start_ms)
    .bind(now)
    .execute(db)
    .await?;
    Ok(())
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

    #[tokio::test]
    async fn snapshot_then_load_restores_live_windows() {
        let db = test_db().await;
        let state = AppState::new(db.clone(), vec![0u8; 32]);

        state
            .rate_limiter
            .insert("key-a".to_string(), RateWindow::new(now_ms()));
        state
            .rate_limiter
            .get("key-a")
            .unwrap()
            .count
            .store(42, Ordering::SeqCst);

        state
            .agent_tracker
            .insert("agent-b".to_string(), VelocityRecord::new(now_ms()));
        {
            let rec = state.agent_tracker.get("agent-b").unwrap();
            rec.request_count.store(7, Ordering::SeqCst);
            rec.anomaly_count.store(2, Ordering::SeqCst);
        }

        state
            .sandbox_rate
            .insert("1.2.3.4".to_string(), RateWindow::new(now_ms()));
        state
            .sandbox_rate
            .get("1.2.3.4")
            .unwrap()
            .count
            .store(3, Ordering::SeqCst);

        snapshot(&db, &state)
            .await
            .expect("snapshot should succeed");

        // Fresh AppState sharing the same DB — simulates a restart.
        let restarted = AppState::new(db.clone(), vec![0u8; 32]);
        load(&db, &restarted).await.expect("load should succeed");

        assert_eq!(
            restarted
                .rate_limiter
                .get("key-a")
                .unwrap()
                .count
                .load(Ordering::SeqCst),
            42
        );
        assert_eq!(
            restarted
                .agent_tracker
                .get("agent-b")
                .unwrap()
                .request_count
                .load(Ordering::SeqCst),
            7
        );
        assert_eq!(
            restarted
                .agent_tracker
                .get("agent-b")
                .unwrap()
                .anomaly_count
                .load(Ordering::SeqCst),
            2
        );
        assert_eq!(
            restarted
                .sandbox_rate
                .get("1.2.3.4")
                .unwrap()
                .count
                .load(Ordering::SeqCst),
            3
        );
    }

    #[tokio::test]
    async fn load_skips_expired_windows() {
        let db = test_db().await;
        let state = AppState::new(db.clone(), vec![0u8; 32]);

        // A window that started well outside the live window must not be
        // restored — it would just reset on first use anyway.
        let stale_start = now_ms() - 10 * 60 * 1000; // 10 minutes ago
        state.rate_limiter.insert(
            "stale-key".to_string(),
            RateWindow::from_parts(99, stale_start),
        );

        snapshot(&db, &state)
            .await
            .expect("snapshot should succeed");

        let restarted = AppState::new(db.clone(), vec![0u8; 32]);
        load(&db, &restarted).await.expect("load should succeed");

        assert!(
            restarted.rate_limiter.get("stale-key").is_none(),
            "an expired window must not be restored"
        );
    }

    #[tokio::test]
    async fn snapshot_upserts_rather_than_duplicating() {
        let db = test_db().await;
        let state = AppState::new(db.clone(), vec![0u8; 32]);

        state
            .rate_limiter
            .insert("key-a".to_string(), RateWindow::new(now_ms()));
        snapshot(&db, &state).await.unwrap();

        state
            .rate_limiter
            .get("key-a")
            .unwrap()
            .count
            .store(5, Ordering::SeqCst);
        snapshot(&db, &state).await.unwrap();

        let row_count: i64 = sqlx::query("SELECT COUNT(*) FROM rate_limit_state")
            .fetch_one(&db)
            .await
            .unwrap()
            .try_get(0)
            .unwrap();
        assert_eq!(
            row_count, 1,
            "repeated snapshots of the same key must upsert, not accumulate rows"
        );

        let restarted = AppState::new(db.clone(), vec![0u8; 32]);
        load(&db, &restarted).await.unwrap();
        assert_eq!(
            restarted
                .rate_limiter
                .get("key-a")
                .unwrap()
                .count
                .load(Ordering::SeqCst),
            5
        );
    }
}
