use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::atomic::Ordering;

use crate::{auth::TenantId, errors::{ApiError, ApiResult}, state::AppState};

#[derive(Serialize)]
pub struct AgentStats {
    pub key_id:          String,
    pub name:            String,
    pub active:          bool,
    pub request_count:   u64,
    pub anomaly_count:   u64,
    pub window_start_ms: i64,
}

pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<Vec<AgentStats>>> {
    let db  = state.db.clone();
    let tid = tenant.0.clone();

    // Load all ai_agent keys from DB
    let agent_keys: Vec<(String, String, bool)> = tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, active FROM api_keys
             WHERE tenant_id = ?1 AND agent_type = 'ai_agent'
             ORDER BY created_at DESC"
        ).map_err(|e| ApiError::Internal(e.to_string()))?;

        let rows = stmt.query_map(rusqlite::params![tid], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)? != 0))
        }).map_err(|e| ApiError::Internal(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| ApiError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    // Enrich with in-memory velocity stats
    let stats: Vec<AgentStats> = agent_keys.into_iter().map(|(key_id, name, active)| {
        let (request_count, anomaly_count, window_start_ms) =
            if let Some(rec) = state.agent_tracker.get(&key_id) {
                (
                    rec.request_count.load(Ordering::Relaxed),
                    rec.anomaly_count.load(Ordering::Relaxed),
                    rec.window_start_ms.load(Ordering::Relaxed),
                )
            } else {
                (0, 0, 0)
            };
        AgentStats { key_id, name, active, request_count, anomaly_count, window_start_ms }
    }).collect();

    Ok(Json(stats))
}
