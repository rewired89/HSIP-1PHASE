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

// Replay protection (opt-in via x-hsip-timestamp + x-hsip-nonce headers).
// Tolerance window applies in both directions (clock skew); nonces are
// retained for double this so a nonce can't become valid again the instant
// it's swept while its timestamp is still inside the tolerance window.
const REPLAY_TOLERANCE_SECS: i64 = 300; // 5 minutes
const REPLAY_NONCE_MAX_LEN: usize = 128;

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

        // Opt-in replay protection: only checked when the caller sends both
        // x-hsip-timestamp and x-hsip-nonce. Absent headers = zero behavior
        // change from before this existed.
        check_replay_protection(&key_id, parts, state)?;

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

/// Opt-in replay protection. If neither `x-hsip-timestamp` nor `x-hsip-nonce`
/// is present, this is a no-op — existing callers are entirely unaffected.
/// If a caller sends both, the timestamp must be within
/// `REPLAY_TOLERANCE_SECS` of server time and the (key_id, nonce) pair must
/// not have been seen before within that window. A signature alone can't
/// stop a captured request from being replayed verbatim; this closes that
/// gap for callers who opt in without breaking anyone who doesn't.
fn check_replay_protection(key_id: &str, parts: &Parts, state: &AppState) -> Result<(), ApiError> {
    let ts_header = parts
        .headers
        .get("x-hsip-timestamp")
        .and_then(|v| v.to_str().ok());
    let nonce_header = parts
        .headers
        .get("x-hsip-nonce")
        .and_then(|v| v.to_str().ok());

    let (ts_str, nonce) = match (ts_header, nonce_header) {
        (None, None) => return Ok(()),
        (Some(t), Some(n)) => (t, n),
        _ => {
            metrics::REPLAY_REJECTED
                .with_label_values(&["malformed_headers"])
                .inc();
            return Err(ApiError::BadRequest(
                "x-hsip-timestamp and x-hsip-nonce must both be present, or neither".into(),
            ));
        }
    };

    if nonce.is_empty() || nonce.len() > REPLAY_NONCE_MAX_LEN {
        metrics::REPLAY_REJECTED
            .with_label_values(&["malformed_headers"])
            .inc();
        return Err(ApiError::BadRequest(format!(
            "x-hsip-nonce must be 1-{REPLAY_NONCE_MAX_LEN} characters"
        )));
    }

    let ts: i64 = ts_str.parse().map_err(|_| {
        metrics::REPLAY_REJECTED
            .with_label_values(&["malformed_headers"])
            .inc();
        ApiError::BadRequest("x-hsip-timestamp must be a Unix timestamp in seconds".into())
    })?;

    let now_secs = now_ms() / 1000;
    if (now_secs - ts).abs() > REPLAY_TOLERANCE_SECS {
        metrics::REPLAY_REJECTED
            .with_label_values(&["timestamp_out_of_window"])
            .inc();
        return Err(ApiError::Unauthorized(format!(
            "x-hsip-timestamp outside the allowed {REPLAY_TOLERANCE_SECS}s window"
        )));
    }

    let dedup_key = format!("{key_id}:{nonce}");
    let expiry_ms = now_ms() + REPLAY_TOLERANCE_SECS * 2 * 1000;
    match state.replay_nonces.entry(dedup_key) {
        dashmap::mapref::entry::Entry::Occupied(_) => {
            metrics::REPLAY_REJECTED
                .with_label_values(&["duplicate_nonce"])
                .inc();
            Err(ApiError::Unauthorized(
                "Duplicate x-hsip-nonce for this key — request already processed".into(),
            ))
        }
        dashmap::mapref::entry::Entry::Vacant(v) => {
            v.insert(expiry_ms);
            Ok(())
        }
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
            // `pending_revocation` (in-memory, already set above) is what's
            // actually blocking this key right now. This DB write is what
            // makes that durable past a process restart. It was previously
            // fire-and-forget with errors discarded — if it failed or the
            // process crashed before it landed, the key would silently
            // become valid again on restart with no record of why. Retry a
            // few times and, if it still fails, say so loudly instead of
            // staying silent about a security-relevant write that didn't
            // happen.
            let mut revoked = false;
            for attempt in 1..=3u32 {
                match sqlx::query("UPDATE api_keys SET active=0 WHERE id=?")
                    .bind(&kid)
                    .execute(&db)
                    .await
                {
                    Ok(_) => {
                        revoked = true;
                        break;
                    }
                    Err(e) => {
                        tracing::warn!(
                            key_id = %kid, attempt, error = %e,
                            "auto-revoke DB write failed, retrying"
                        );
                        if attempt < 3 {
                            tokio::time::sleep(std::time::Duration::from_millis(
                                100 * attempt as u64,
                            ))
                            .await;
                        }
                    }
                }
            }

            if revoked {
                let _ = crate::audit_log::record(
                    &db,
                    &tid,
                    "agent.auto_revoked",
                    None,
                    Some(&kid),
                    now,
                )
                .await;
            } else {
                metrics::AGENT_ANOMALIES
                    .with_label_values(&["auto_revoke_db_write_failed"])
                    .inc();
                tracing::error!(
                    key_id = %kid, tenant_id = %tid,
                    "AUTO-REVOKE DB WRITE FAILED after 3 attempts — key {kid} is blocked \
                     in-memory only (pending_revocation) and will silently become valid \
                     again on process restart until this DB write succeeds. Check DB \
                     connectivity now."
                );
                // Best-effort: still try to leave a trace even though the
                // revocation itself didn't land.
                let _ = crate::audit_log::record(
                    &db,
                    &tid,
                    "agent.auto_revoke_failed",
                    None,
                    Some(&kid),
                    now,
                )
                .await;
            }
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
