use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::Row;

use crate::{auth::TenantId, db::now_ms, errors::ApiResult, metrics, state::AppState};

#[derive(Serialize)]
pub struct EraseResponse {
    pub erased: bool,
    pub tenant_id: String,
    pub timestamp: i64,
    pub tables_cleared: Vec<String>,
}

/// POST /v1/tenant/erase
/// GDPR Article 17 — Right to erasure ("right to be forgotten").
/// Deletes all data for the calling tenant. This action is irreversible.
pub async fn erase(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<EraseResponse>> {
    let now = now_ms();
    let tid = &tenant.0;

    // Delete in dependency order
    sqlx::query("DELETE FROM credentials    WHERE tenant_id = ?")
        .bind(tid)
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM messages       WHERE tenant_id = ?")
        .bind(tid)
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM consents       WHERE tenant_id = ?")
        .bind(tid)
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM identities     WHERE tenant_id = ?")
        .bind(tid)
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM audit_entries  WHERE tenant_id = ?")
        .bind(tid)
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM api_keys       WHERE tenant_id = ?")
        .bind(tid)
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM tenants        WHERE id        = ?")
        .bind(tid)
        .execute(&state.db)
        .await?;

    // Remove in-memory state
    state.agent_tracker.retain(|_, _| false);
    state.rate_limiter.retain(|_, _| false);

    metrics::ACTIVE_TENANTS.dec();

    tracing::info!(tenant_id=%tid, "GDPR erasure completed");

    Ok(Json(EraseResponse {
        erased: true,
        tenant_id: tenant.0.clone(),
        timestamp: now,
        tables_cleared: vec![
            "credentials".into(),
            "messages".into(),
            "consents".into(),
            "identities".into(),
            "audit_entries".into(),
            "api_keys".into(),
            "tenants".into(),
        ],
    }))
}

/// GET /v1/tenant
/// Returns basic info about the calling tenant.
pub async fn info(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query("SELECT name, created_at FROM tenants WHERE id = ?")
        .bind(&tenant.0)
        .fetch_one(&state.db)
        .await?;

    let name: String = row.try_get(0)?;
    let created_at: i64 = row.try_get(1)?;

    Ok(Json(serde_json::json!({
        "tenant_id":  tenant.0,
        "name":       name,
        "created_at": created_at,
    })))
}
