use axum::{extract::{Query, State}, Json};
use serde::{Deserialize, Serialize};

use crate::{auth::TenantId, errors::{ApiError, ApiResult}, state::AppState};

#[derive(Deserialize)]
pub struct AuditQuery {
    pub limit:  Option<i64>,
    pub action: Option<String>,
}

#[derive(Serialize)]
pub struct AuditEntry {
    pub id:              String,
    pub action:          String,
    pub peer_verify_key: Option<String>,
    pub details:         Option<String>,
    pub timestamp:       i64,
}

pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
    Query(params): Query<AuditQuery>,
) -> ApiResult<Json<Vec<AuditEntry>>> {
    let limit  = params.limit.unwrap_or(50).min(500);
    let action = params.action.clone();
    let db     = state.db.clone();
    let tid    = tenant.0.clone();

    let entries = tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        let mut stmt = if let Some(act) = action {
            let pattern = format!("%{act}%");
            let mut s = conn.prepare(
                "SELECT id,action,peer_verify_key,details,timestamp
                 FROM audit_entries WHERE tenant_id=?1 AND action LIKE ?2
                 ORDER BY timestamp DESC LIMIT ?3"
            ).map_err(|e| ApiError::Internal(e.to_string()))?;
            let rows = s.query_map(rusqlite::params![tid, pattern, limit], |r| Ok(AuditEntry {
                id:              r.get(0)?,
                action:          r.get(1)?,
                peer_verify_key: r.get(2)?,
                details:         r.get(3)?,
                timestamp:       r.get(4)?,
            })).map_err(|e| ApiError::Internal(e.to_string()))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| ApiError::Internal(e.to_string()))
        } else {
            let mut s = conn.prepare(
                "SELECT id,action,peer_verify_key,details,timestamp
                 FROM audit_entries WHERE tenant_id=?1
                 ORDER BY timestamp DESC LIMIT ?2"
            ).map_err(|e| ApiError::Internal(e.to_string()))?;
            let rows = s.query_map(rusqlite::params![tid, limit], |r| Ok(AuditEntry {
                id:              r.get(0)?,
                action:          r.get(1)?,
                peer_verify_key: r.get(2)?,
                details:         r.get(3)?,
                timestamp:       r.get(4)?,
            })).map_err(|e| ApiError::Internal(e.to_string()))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| ApiError::Internal(e.to_string()))
        }?;
        Ok::<_, ApiError>(stmt)
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(entries))
}
