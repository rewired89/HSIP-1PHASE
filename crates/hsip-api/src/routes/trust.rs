use axum::{
    extract::{Path, State},
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::TenantId,
    db::now_ms,
    errors::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Serialize)]
pub struct TrustedPeer {
    pub id: String,
    pub label: String,
    pub verify_key: String,
    pub added_at: i64,
}

#[derive(Deserialize)]
pub struct AddPeerRequest {
    pub label: String,
    pub verify_key: String,
}

#[derive(Deserialize)]
pub struct TrustVerifyRequest {
    pub label: String,
    pub content: String,
    pub signature: String,
}

#[derive(Serialize)]
pub struct TrustVerifyResponse {
    pub verified: bool,
    pub label: String,
    pub verify_key: String,
    pub timestamp: i64,
}

/// POST /v1/trust/peer
/// Add or update a trusted peer identified by their Ed25519 verify key.
pub async fn add(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(body): Json<AddPeerRequest>,
) -> ApiResult<Json<TrustedPeer>> {
    let key_bytes: [u8; 32] = BASE64
        .decode(&body.verify_key)
        .map_err(|_| ApiError::BadRequest("Invalid verify_key encoding".into()))?
        .try_into()
        .map_err(|_| ApiError::BadRequest("verify_key must be 32 bytes".into()))?;
    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|_| ApiError::BadRequest("Invalid Ed25519 verify key".into()))?;

    let id = Uuid::new_v4().to_string();
    let now = now_ms();

    sqlx::query(
        "INSERT INTO trusted_peers (id, tenant_id, label, verify_key, added_at)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT(tenant_id, verify_key) DO UPDATE SET label = excluded.label",
    )
    .bind(&id)
    .bind(&tenant.0)
    .bind(&body.label)
    .bind(&body.verify_key)
    .bind(now)
    .execute(&state.db)
    .await?;

    crate::audit_log::record_best_effort(
        &state.db,
        &tenant.0,
        "trust.peer_added",
        Some(&body.verify_key),
        Some(&body.label),
        now,
    )
    .await;

    Ok(Json(TrustedPeer {
        id,
        label: body.label,
        verify_key: body.verify_key,
        added_at: now,
    }))
}

/// GET /v1/trust/peers
pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<Vec<TrustedPeer>>> {
    let rows = sqlx::query(
        "SELECT id, label, verify_key, added_at FROM trusted_peers
         WHERE tenant_id = $1 ORDER BY added_at DESC",
    )
    .bind(&tenant.0)
    .fetch_all(&state.db)
    .await?;

    let peers = rows
        .iter()
        .map(|r| -> Result<TrustedPeer, sqlx::Error> {
            Ok(TrustedPeer {
                id: r.try_get(0)?,
                label: r.try_get(1)?,
                verify_key: r.try_get(2)?,
                added_at: r.try_get(3)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(peers))
}

/// DELETE /v1/trust/peers/:id
pub async fn remove(
    State(state): State<AppState>,
    tenant: TenantId,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let row =
        sqlx::query("SELECT verify_key, label FROM trusted_peers WHERE id = $1 AND tenant_id = $2")
            .bind(&id)
            .bind(&tenant.0)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Trusted peer {id} not found")))?;

    let vk: String = row.try_get(0)?;
    let label: String = row.try_get(1)?;

    sqlx::query("DELETE FROM trusted_peers WHERE id = $1 AND tenant_id = $2")
        .bind(&id)
        .bind(&tenant.0)
        .execute(&state.db)
        .await?;

    let now = now_ms();
    crate::audit_log::record_best_effort(
        &state.db,
        &tenant.0,
        "trust.peer_removed",
        Some(&vk),
        Some(&label),
        now,
    )
    .await;

    Ok(Json(serde_json::json!({ "removed": id })))
}

/// POST /v1/trust/verify
/// Verify a signed message from a trusted peer identified by label.
pub async fn verify(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(req): Json<TrustVerifyRequest>,
) -> ApiResult<Json<TrustVerifyResponse>> {
    let row =
        sqlx::query("SELECT verify_key FROM trusted_peers WHERE tenant_id = $1 AND label = $2")
            .bind(&tenant.0)
            .bind(&req.label)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("No trusted peer with label \"{}\"", req.label))
            })?;

    let vk_b64: String = row.try_get(0)?;

    let key_bytes: [u8; 32] = BASE64
        .decode(&vk_b64)
        .map_err(|_| ApiError::Internal("stored key corrupt".into()))?
        .try_into()
        .map_err(|_| ApiError::Internal("stored key wrong length".into()))?;

    let verifying_key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|_| ApiError::Internal("stored key invalid".into()))?;

    let sig_bytes: [u8; 64] = BASE64
        .decode(&req.signature)
        .map_err(|_| ApiError::BadRequest("Invalid signature encoding".into()))?
        .try_into()
        .map_err(|_| ApiError::BadRequest("Signature must be 64 bytes".into()))?;

    let signature = Signature::from_bytes(&sig_bytes);
    let verified = verifying_key
        .verify(req.content.as_bytes(), &signature)
        .is_ok();

    let now = now_ms();
    let action = if verified {
        "trust.verify_ok"
    } else {
        "trust.verify_failed"
    };
    crate::audit_log::record_best_effort(
        &state.db,
        &tenant.0,
        action,
        Some(&vk_b64),
        Some(&req.label),
        now,
    )
    .await;

    Ok(Json(TrustVerifyResponse {
        verified,
        label: req.label,
        verify_key: vk_b64,
        timestamp: now,
    }))
}
