use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::Row;
use std::sync::atomic::Ordering;

use crate::{auth::TenantId, errors::ApiResult, state::AppState};

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
    let rows = sqlx::query(
        "SELECT id, name, active FROM api_keys
         WHERE tenant_id = ? AND agent_type = 'ai_agent'
         ORDER BY created_at DESC",
    )
    .bind(&tenant.0)
    .fetch_all(&state.db)
    .await?;

    let agent_keys: Vec<(String, String, bool)> = rows.iter()
        .map(|r| -> Result<_, sqlx::Error> {
            Ok((
                r.try_get::<String, _>(0)?,
                r.try_get::<String, _>(1)?,
                r.try_get::<i64, _>(2)? != 0,
            ))
        })
        .collect::<Result<_, _>>()?;

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
