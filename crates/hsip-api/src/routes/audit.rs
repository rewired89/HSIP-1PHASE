use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{audit_log::ChainRow, auth::TenantId, errors::ApiResult, state::AppState};

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
    /// BLAKE3 chain fields — `None` for entries written before the audit
    /// hash chain existed (see `audit_log` module docs).
    pub prev_hash: Option<String>,
    pub entry_hash: Option<String>,
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
            "SELECT id, action, peer_verify_key, details, timestamp, prev_hash, entry_hash
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
            "SELECT id, action, peer_verify_key, details, timestamp, prev_hash, entry_hash
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
                prev_hash: r.try_get(5)?,
                entry_hash: r.try_get(6)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(entries))
}

#[derive(Serialize)]
pub struct AuditVerifyResponse {
    /// True if every chained entry's hash matches its recomputed value and
    /// every `prev_hash` correctly links to the entry before it.
    pub valid: bool,
    /// Number of chained entries (non-NULL entry_hash) checked.
    pub checked: usize,
    /// Number of entries written before the hash chain existed — not
    /// covered by this check.
    pub unchained: usize,
    /// id of the first entry where the chain breaks, if any.
    pub first_break_id: Option<String>,
}

/// `GET /v1/audit/verify` — recomputes this tenant's BLAKE3 audit hash
/// chain server-side and reports whether it's intact. A `valid: false`
/// result means a row was altered, deleted, or reordered after being
/// written — evidence of database-level tampering that application-level
/// access controls alone cannot detect.
pub async fn verify_chain(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<AuditVerifyResponse>> {
    let rows = sqlx::query(
        "SELECT id, tenant_id, action, peer_verify_key, details, timestamp, prev_hash, entry_hash
         FROM audit_entries WHERE tenant_id=?
         ORDER BY timestamp ASC",
    )
    .bind(&tenant.0)
    .fetch_all(&state.db)
    .await?;

    let chain_rows = rows
        .iter()
        .map(|r| -> Result<ChainRow, sqlx::Error> {
            Ok(ChainRow {
                id: r.try_get(0)?,
                tenant_id: r.try_get(1)?,
                action: r.try_get(2)?,
                peer_verify_key: r.try_get(3)?,
                details: r.try_get(4)?,
                timestamp: r.try_get(5)?,
                prev_hash: r.try_get(6)?,
                entry_hash: r.try_get(7)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let result = crate::audit_log::verify_chain(&chain_rows);

    Ok(Json(AuditVerifyResponse {
        valid: result.valid,
        checked: result.checked,
        unchained: result.unchained,
        first_break_id: result.first_break_id,
    }))
}
