use axum::{extract::State, Json};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde::Serialize;
use sqlx::Row;

use crate::{
    auth::TenantId,
    db::now_ms,
    errors::{ApiError, ApiResult},
    key_encryption::{decrypt_signing_key, encrypt_signing_key},
    state::AppState,
};

#[derive(Serialize)]
pub struct IdentityResponse {
    pub tenant_id: String,
    pub verify_key: String,
    pub created_at: i64,
}

pub async fn create_or_get(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<IdentityResponse>> {
    let row = sqlx::query("SELECT verify_key_b64, created_at FROM identities WHERE tenant_id = $1")
        .bind(&tenant.0)
        .fetch_optional(&state.db)
        .await?;

    if let Some(row) = row {
        let verify_key: String = row.try_get(0)?;
        let created_at: i64 = row.try_get(1)?;
        return Ok(Json(IdentityResponse {
            tenant_id: tenant.0,
            verify_key,
            created_at,
        }));
    }

    let signing_key = SigningKey::generate(&mut OsRng);
    let verify_key = signing_key.verifying_key();
    // C1: encrypt the private key before storing
    let encrypted_b64 = {
        let master_key = state.master_key.read().await;
        encrypt_signing_key(&signing_key.to_bytes(), &master_key)
    };
    let verify_b64 = BASE64.encode(verify_key.to_bytes());
    let now = now_ms();

    sqlx::query(
        "INSERT INTO identities (tenant_id, signing_key_b64, verify_key_b64, created_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(&tenant.0)
    .bind(&encrypted_b64)
    .bind(&verify_b64)
    .bind(now)
    .execute(&state.db)
    .await?;

    crate::audit_log::record_best_effort(
        &state.db,
        &tenant.0,
        "identity.created",
        None,
        Some(&verify_b64),
        now,
    )
    .await;

    Ok(Json(IdentityResponse {
        tenant_id: tenant.0,
        verify_key: verify_b64,
        created_at: now,
    }))
}

pub async fn get(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<IdentityResponse>> {
    let row = sqlx::query("SELECT verify_key_b64, created_at FROM identities WHERE tenant_id = $1")
        .bind(&tenant.0)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound("No identity. POST /v1/identity to create one.".into())
        })?;

    let verify_key: String = row.try_get(0)?;
    let created_at: i64 = row.try_get(1)?;

    Ok(Json(IdentityResponse {
        tenant_id: tenant.0,
        verify_key,
        created_at,
    }))
}

/// M5: Rotate the tenant's Ed25519 signing key.
/// Generates a new keypair, stores it encrypted, and marks old credentials
/// issued under the previous key as rotated in the audit log.
/// Existing credentials signed under the old key remain verifiable by their
/// issuer_verify_key field — this endpoint does NOT revoke them automatically.
pub async fn rotate(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<IdentityResponse>> {
    // Require existing identity before rotation
    let existing = sqlx::query("SELECT verify_key_b64 FROM identities WHERE tenant_id = $1")
        .bind(&tenant.0)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| {
            ApiError::BadRequest("No identity to rotate. POST /v1/identity first.".into())
        })?;

    let old_verify_key: String = existing.try_get(0)?;

    // Generate new keypair
    let new_signing_key = SigningKey::generate(&mut OsRng);
    let new_verify_key = new_signing_key.verifying_key();
    let new_encrypted = {
        let master_key = state.master_key.read().await;
        encrypt_signing_key(&new_signing_key.to_bytes(), &master_key)
    };
    let new_verify_b64 = BASE64.encode(new_verify_key.to_bytes());
    let now = now_ms();

    sqlx::query(
        "UPDATE identities SET signing_key_b64 = $1, verify_key_b64 = $2 WHERE tenant_id = $3",
    )
    .bind(&new_encrypted)
    .bind(&new_verify_b64)
    .bind(&tenant.0)
    .execute(&state.db)
    .await?;

    crate::audit_log::record_best_effort(
        &state.db,
        &tenant.0,
        "identity.key_rotated",
        None,
        Some(&format!(
            "old_key={old_verify_key} new_key={new_verify_b64}"
        )),
        now,
    )
    .await;

    Ok(Json(IdentityResponse {
        tenant_id: tenant.0,
        verify_key: new_verify_b64,
        created_at: now,
    }))
}

/// Load and decrypt the signing key for a tenant. Used by credential issuance.
pub async fn load_signing_key(
    db: &crate::db::Db,
    tenant_id: &str,
    master_key: &[u8],
) -> ApiResult<SigningKey> {
    let row = sqlx::query("SELECT signing_key_b64 FROM identities WHERE tenant_id = $1")
        .bind(tenant_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ApiError::BadRequest("No identity. POST /v1/identity first.".into()))?;

    let encrypted_b64: String = row.try_get(0)?;

    let key_bytes = decrypt_signing_key(&encrypted_b64, master_key).map_err(|e| {
        tracing::error!(error = %e, "key decryption failed");
        ApiError::Internal("internal server error".into())
    })?;

    Ok(SigningKey::from_bytes(&key_bytes))
}
