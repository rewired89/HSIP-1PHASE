use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use sha2::{Sha256, Digest};
use std::sync::atomic::Ordering;
use crate::{db::now_ms, errors::ApiError, metrics, state::{AppState, VelocityRecord}};

// Requests per 60-second window before anomaly is logged
const ANOMALY_THRESHOLD: u64  = 100;
// Requests per 60-second window before key is auto-revoked
const REVOKE_THRESHOLD:  u64  = 1000;
const WINDOW_MS:         i64  = 60_000;

#[derive(Clone, Debug)]
pub struct TenantId(pub String);

#[axum::async_trait]
impl FromRequestParts<AppState> for TenantId {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts.headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(str::trim)
            .ok_or_else(|| {
                metrics::AUTH_FAILURES.with_label_values(&["missing_header"]).inc();
                ApiError::Unauthorized("Missing Authorization: Bearer <key>".into())
            })?
            .to_string();

        let key_hash = hash_key(&token);
        let db       = state.db.clone();

        let (tenant_id, key_id, agent_type) = tokio::task::spawn_blocking(move || {
            let conn = db.lock().map_err(|_| ApiError::Internal("db lock poisoned".into()))?;
            match conn.query_row(
                "SELECT tenant_id, id, agent_type FROM api_keys WHERE key_hash = ? AND active = 1",
                rusqlite::params![key_hash],
                |row| Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                )),
            ) {
                Ok(row) => Ok(row),
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    Err(ApiError::Unauthorized("Invalid API key".into()))
                }
                Err(e) => Err(ApiError::Internal(e.to_string())),
            }
        })
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))??;

        // Velocity tracking for ai_agent keys
        if agent_type == "ai_agent" {
            check_agent_velocity(&key_id, &tenant_id, state).await;
        }

        metrics::REQUESTS_TOTAL.with_label_values(&["auth", "ok"]).inc();
        Ok(TenantId(tenant_id))
    }
}

async fn check_agent_velocity(key_id: &str, tenant_id: &str, state: &AppState) {
    let now = now_ms();
    let tracker = &state.agent_tracker;

    let (count, anomalies) = if let Some(rec) = tracker.get(key_id) {
        let window_start = rec.window_start_ms.load(Ordering::Relaxed);
        if now - window_start > WINDOW_MS {
            // Reset window
            rec.window_start_ms.store(now, Ordering::Relaxed);
            rec.request_count.store(1, Ordering::Relaxed);
            (1u64, rec.anomaly_count.load(Ordering::Relaxed))
        } else {
            let c = rec.request_count.fetch_add(1, Ordering::Relaxed) + 1;
            (c, rec.anomaly_count.load(Ordering::Relaxed))
        }
    } else {
        tracker.insert(key_id.to_string(), VelocityRecord::new(now));
        (1u64, 0u64)
    };

    if count == ANOMALY_THRESHOLD {
        metrics::AGENT_ANOMALIES.with_label_values(&["threshold_exceeded"]).inc();
        let db  = state.db.clone();
        let kid = key_id.to_string();
        let tid = tenant_id.to_string();
        if let Some(rec) = tracker.get(&kid) {
            rec.anomaly_count.fetch_add(1, Ordering::Relaxed);
        }
        tokio::task::spawn(async move {
            if let Ok(db) = db.lock() {
                let id = uuid::Uuid::new_v4().to_string();
                let _ = db.execute(
                    "INSERT INTO audit_entries (id,tenant_id,action,details,timestamp)
                     VALUES (?1,?2,'agent.anomaly_detected',?3,?4)",
                    rusqlite::params![id, tid, kid, now],
                );
            }
        });
        tracing::warn!(key_id=%key_id, count=%count, "AI agent anomaly: request threshold exceeded");
    }

    if count >= REVOKE_THRESHOLD {
        metrics::AGENT_ANOMALIES.with_label_values(&["auto_revoked"]).inc();
        let db  = state.db.clone();
        let kid = key_id.to_string();
        let tid = tenant_id.to_string();
        tokio::task::spawn(async move {
            if let Ok(db) = db.lock() {
                let _ = db.execute(
                    "UPDATE api_keys SET active=0 WHERE id=?",
                    rusqlite::params![kid],
                );
                let id = uuid::Uuid::new_v4().to_string();
                let _ = db.execute(
                    "INSERT INTO audit_entries (id,tenant_id,action,details,timestamp)
                     VALUES (?1,?2,'agent.auto_revoked',?3,?4)",
                    rusqlite::params![id, tid, kid, now],
                );
            }
        });
        tracing::error!(key_id=%key_id, count=%count, "AI agent auto-revoked: exceeded hard limit");
        let _ = tracker.remove(key_id);
    }

    let _ = anomalies; // used in future for escalation logic
}

pub fn hash_key(key: &str) -> String {
    let mut h = Sha256::new();
    h.update(key.as_bytes());
    hex::encode(h.finalize())
}
