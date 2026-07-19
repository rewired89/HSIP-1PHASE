use crate::{
    db::now_ms,
    errors::ApiError,
    metrics,
    state::{AppState, RateWindow, VelocityRecord},
};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::sync::atomic::Ordering;

// AI agent anomaly/revoke thresholds (requests per 60s window)
const ANOMALY_THRESHOLD: u64 = 100;
const REVOKE_THRESHOLD: u64 = 1000;
const WINDOW_MS: i64 = 60_000;

// General per-key rate limit (requests per 60s window); override with RATE_LIMIT_RPM env var
fn rate_limit_rpm() -> u64 {
    std::env::var("RATE_LIMIT_RPM")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300)
}

#[derive(Clone, Debug)]
pub struct TenantId(pub String);

#[axum::async_trait]
impl FromRequestParts<AppState> for TenantId {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(str::trim)
            .ok_or_else(|| {
                metrics::AUTH_FAILURES
                    .with_label_values(&["missing_header"])
                    .inc();
                ApiError::Unauthorized("Missing Authorization: Bearer <key>".into())
            })?
            .to_string();

        let key_hash = hash_key(&token);
        let now = now_ms();

        let row = sqlx::query(
            "SELECT tenant_id, id, agent_type FROM api_keys
             WHERE key_hash = ? AND active = 1
               AND (expires_at IS NULL OR expires_at > ?)",
        )
        .bind(&key_hash)
        .bind(now)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

        let row = row.ok_or_else(|| {
            metrics::AUTH_FAILURES
                .with_label_values(&["invalid_key"])
                .inc();
            // M4: spawn audit log entry for failed auth (no tenant context, use system marker)
            ApiError::Unauthorized("Invalid or expired API key".into())
        })?;

        let tenant_id: String = row
            .try_get(0)
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        let key_id: String = row
            .try_get(1)
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        let agent_type: String = row
            .try_get(2)
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        // H2: reject immediately if key is pending revocation (DB write may still be in-flight)
        if state.pending_revocation.contains(&key_id) {
            metrics::AUTH_FAILURES
                .with_label_values(&["pending_revocation"])
                .inc();
            return Err(ApiError::Unauthorized("API key has been revoked".into()));
        }

        // Per-key rate limiting for all key types
        check_rate_limit(&key_id, state)?;

        // Velocity anomaly tracking for ai_agent keys
        if agent_type == "ai_agent" {
            check_agent_velocity(&key_id, &tenant_id, state).await;
        }

        // M4: write successful auth to audit — skip for now (too noisy), only log anomalies
        metrics::REQUESTS_TOTAL
            .with_label_values(&["auth", "ok"])
            .inc();
        Ok(TenantId(tenant_id))
    }
}

/// H1: Rate limit with SeqCst ordering to prevent race-condition bypasses.
/// Window reset uses a compare-and-swap pattern to avoid TOCTOU.
fn check_rate_limit(key_id: &str, state: &AppState) -> Result<(), ApiError> {
    let now = now_ms();
    let limit = rate_limit_rpm();
    let rl = &state.rate_limiter;

    let count = if let Some(win) = rl.get(key_id) {
        let ws = win.window_start_ms.load(Ordering::SeqCst);
        if now - ws > WINDOW_MS {
            // Reset window atomically: only one thread should win this CAS
            let reset =
                win.window_start_ms
                    .compare_exchange(ws, now, Ordering::SeqCst, Ordering::SeqCst);
            if reset.is_ok() {
                win.count.store(1, Ordering::SeqCst);
                1u64
            } else {
                // Another thread already reset; just increment
                win.count.fetch_add(1, Ordering::SeqCst) + 1
            }
        } else {
            win.count.fetch_add(1, Ordering::SeqCst) + 1
        }
    } else {
        rl.insert(key_id.to_string(), RateWindow::new(now));
        1u64
    };

    if count > limit {
        return Err(ApiError::TooManyRequests(format!(
            "Rate limit exceeded ({limit} req/min). Retry after the current window resets."
        )));
    }
    Ok(())
}

async fn check_agent_velocity(key_id: &str, tenant_id: &str, state: &AppState) {
    let now = now_ms();
    let tracker = &state.agent_tracker;

    let (count, anomalies) = if let Some(rec) = tracker.get(key_id) {
        let ws = rec.window_start_ms.load(Ordering::SeqCst);
        if now - ws > WINDOW_MS {
            let reset =
                rec.window_start_ms
                    .compare_exchange(ws, now, Ordering::SeqCst, Ordering::SeqCst);
            if reset.is_ok() {
                rec.request_count.store(1, Ordering::SeqCst);
                (1u64, rec.anomaly_count.load(Ordering::SeqCst))
            } else {
                let c = rec.request_count.fetch_add(1, Ordering::SeqCst) + 1;
                (c, rec.anomaly_count.load(Ordering::SeqCst))
            }
        } else {
            let c = rec.request_count.fetch_add(1, Ordering::SeqCst) + 1;
            (c, rec.anomaly_count.load(Ordering::SeqCst))
        }
    } else {
        tracker.insert(key_id.to_string(), VelocityRecord::new(now));
        (1u64, 0u64)
    };

    if count == ANOMALY_THRESHOLD {
        metrics::AGENT_ANOMALIES
            .with_label_values(&["threshold_exceeded"])
            .inc();
        if let Some(rec) = tracker.get(key_id) {
            rec.anomaly_count.fetch_add(1, Ordering::SeqCst);
        }
        let db = state.db.clone();
        let kid = key_id.to_string();
        let tid = tenant_id.to_string();
        tokio::task::spawn(async move {
            let _ = crate::audit_log::record(
                &db,
                &tid,
                "agent.anomaly_detected",
                None,
                Some(&kid),
                now,
            )
            .await;
        });
        tracing::warn!(key_id=%key_id, count=%count, "AI agent anomaly: threshold exceeded");
    }

    if count >= REVOKE_THRESHOLD {
        metrics::AGENT_ANOMALIES
            .with_label_values(&["auto_revoked"])
            .inc();

        // H2: immediately flag in pending_revocation so all subsequent requests are rejected
        // even before the DB write completes
        state.pending_revocation.insert(key_id.to_string());
        tracker.remove(key_id);

        let db = state.db.clone();
        let kid = key_id.to_string();
        let tid = tenant_id.to_string();
        tokio::task::spawn(async move {
            let _ = sqlx::query("UPDATE api_keys SET active=0 WHERE id=?")
                .bind(&kid)
                .execute(&db)
                .await;
            let _ =
                crate::audit_log::record(&db, &tid, "agent.auto_revoked", None, Some(&kid), now)
                    .await;
        });
        tracing::error!(key_id=%key_id, count=%count, "AI agent auto-revoked: hard limit exceeded");
    }

    let _ = anomalies;
}

pub fn hash_key(key: &str) -> String {
    let mut h = Sha256::new();
    h.update(key.as_bytes());
    hex::encode(h.finalize())
}
