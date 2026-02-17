use axum::{extract::State, Json};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{auth::TenantId, db::now_ms, errors::{ApiError, ApiResult}, state::AppState};

#[derive(Serialize)]
pub struct IdentityResponse {
    pub tenant_id:  String,
    pub verify_key: String,
    pub created_at: i64,
}

pub async fn create_or_get(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<IdentityResponse>> {
    let row = sqlx::query(
        "SELECT verify_key_b64, created_at FROM identities WHERE tenant_id = ?",
    )
    .bind(&tenant.0)
    .fetch_optional(&state.db)
    .await?;

    if let Some(row) = row {
        let verify_key: String = row.try_get(0)?;
        let created_at: i64   = row.try_get(1)?;
        return Ok(Json(IdentityResponse { tenant_id: tenant.0, verify_key, created_at }));
    }

    let signing_key = SigningKey::generate(&mut OsRng);
    let verify_key  = signing_key.verifying_key();
    let signing_b64 = BASE64.encode(signing_key.to_bytes());
    let verify_b64  = BASE64.encode(verify_key.to_bytes());
    let now         = now_ms();
    let audit_id    = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO identities (tenant_id, signing_key_b64, verify_key_b64, created_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind(&tenant.0)
    .bind(&signing_b64)
    .bind(&verify_b64)
    .bind(now)
    .execute(&state.db)
    .await?;

    sqlx::query(
        "INSERT INTO audit_entries (id, tenant_id, action, details, timestamp)
         VALUES (?, ?, 'identity.created', ?, ?)",
    )
    .bind(&audit_id)
    .bind(&tenant.0)
    .bind(&verify_b64)
    .bind(now)
    .execute(&state.db)
    .await?;

    Ok(Json(IdentityResponse { tenant_id: tenant.0, verify_key: verify_b64, created_at: now }))
}

pub async fn get(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<IdentityResponse>> {
    let row = sqlx::query(
        "SELECT verify_key_b64, created_at FROM identities WHERE tenant_id = ?",
    )
    .bind(&tenant.0)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("No identity. POST /v1/identity to create one.".into()))?;

    let verify_key: String = row.try_get(0)?;
    let created_at: i64   = row.try_get(1)?;

    Ok(Json(IdentityResponse { tenant_id: tenant.0, verify_key, created_at }))
}
