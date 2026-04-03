use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{auth::TenantId, errors::ApiResult, state::AppState};

#[derive(Deserialize)]
pub struct AuditQuery {
    pub limit: Option<i64>,
    pub action: Option<String>,
}

#[derive(Serialize)]
pub struct AuditEntry {
    pub id: String,
    pub action: String,
    pub peer_verify_key: Option<String>,
    pub details: Option<String>,
    pub timestamp: i64,
}

pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
    Query(params): Query<AuditQuery>,
) -> ApiResult<Json<Vec<AuditEntry>>> {
    let limit = params.limit.unwrap_or(50).min(500);
    let action = params.action.clone();

    let rows = if let Some(act) = action {
        let pattern = format!("%{act}%");
        sqlx::query(
            "SELECT id, action, peer_verify_key, details, timestamp
             FROM audit_entries WHERE tenant_id=? AND action LIKE ?
             ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(&tenant.0)
        .bind(&pattern)
        .bind(limit)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query(
            "SELECT id, action, peer_verify_key, details, timestamp
             FROM audit_entries WHERE tenant_id=?
             ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(&tenant.0)
        .bind(limit)
        .fetch_all(&state.db)
        .await?
    };

    let entries = rows
        .iter()
        .map(|r| -> Result<AuditEntry, sqlx::Error> {
            Ok(AuditEntry {
                id: r.try_get(0)?,
                action: r.try_get(1)?,
                peer_verify_key: r.try_get(2)?,
                details: r.try_get(3)?,
                timestamp: r.try_get(4)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(entries))
}
