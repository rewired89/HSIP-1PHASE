use axum::{extract::State, Json};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::TenantId,
    db::now_ms,
    errors::{ApiError, ApiResult},
    key_encryption::decrypt_signing_key,
    metrics,
    state::AppState,
};

#[derive(Deserialize)]
pub struct SignRequest {
    pub content: String,
    pub peer_verify_key: Option<String>,
}

#[derive(Serialize)]
pub struct SignResponse {
    pub id: String,
    pub content: String,
    pub signature: String,
    pub timestamp: i64,
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub content: String,
    pub signature: String,
    pub peer_verify_key: String,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub verified: bool,
    pub peer_verify_key: String,
    pub timestamp: i64,
}

#[derive(Serialize)]
pub struct MessageRecord {
    pub id: String,
    pub peer_verify_key: String,
    pub direction: String,
    pub content: String,
    pub signature: String,
    pub timestamp: i64,
    pub verified: bool,
}

pub async fn sign(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(req): Json<SignRequest>,
) -> ApiResult<Json<SignResponse>> {
    let row = sqlx::query("SELECT signing_key_b64 FROM identities WHERE tenant_id = $1")
        .bind(&tenant.0)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::BadRequest("No identity. POST /v1/identity first.".into()))?;

    let encrypted_b64: String = row.try_get(0)?;

    // The signing key is stored encrypted — decrypt it with the master key.
    let key_bytes = {
        let master_key = state.master_key.read().await;
        decrypt_signing_key(&encrypted_b64, &master_key)
    }
    .map_err(|e| {
        tracing::error!(error = %e, "key decryption failed");
        ApiError::Internal("internal server error".into())
    })?;

    let signing_key = SigningKey::from_bytes(&key_bytes);
    let signature = signing_key.sign(req.content.as_bytes());
    let sig_b64 = BASE64.encode(signature.to_bytes());
    let now = now_ms();
    let msg_id = Uuid::new_v4().to_string();
    let peer = req.peer_verify_key.clone().unwrap_or_default();

    sqlx::query(
        "INSERT INTO messages (id, tenant_id, peer_verify_key, direction, content, signature, timestamp, verified)
         VALUES ($1, $2, $3, 'outbound', $4, $5, $6, 1)",
    )
    .bind(&msg_id)
    .bind(&tenant.0)
    .bind(&peer)
    .bind(&req.content)
    .bind(&sig_b64)
    .bind(now)
    .execute(&state.db)
    .await?;

    crate::audit_log::record(
        &state.db,
        &tenant.0,
        "message.signed",
        Some(&peer),
        None,
        now,
    )
    .await?;

    metrics::MESSAGES_SIGNED.inc();

    Ok(Json(SignResponse {
        id: msg_id,
        content: req.content,
        signature: sig_b64,
        timestamp: now,
    }))
}

pub async fn verify(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(req): Json<VerifyRequest>,
) -> ApiResult<Json<VerifyResponse>> {
    let key_bytes: [u8; 32] = BASE64
        .decode(&req.peer_verify_key)
        .map_err(|_| ApiError::BadRequest("Invalid peer_verify_key encoding".into()))?
        .try_into()
        .map_err(|_| ApiError::BadRequest("peer_verify_key must be 32 bytes".into()))?;

    let verifying_key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|_| ApiError::BadRequest("Invalid Ed25519 verify key".into()))?;

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
    let msg_id = Uuid::new_v4().to_string();
    let v_int: i64 = verified as i64;

    sqlx::query(
        "INSERT INTO messages (id, tenant_id, peer_verify_key, direction, content, signature, timestamp, verified)
         VALUES ($1, $2, $3, 'inbound', $4, $5, $6, $7)",
    )
    .bind(&msg_id)
    .bind(&tenant.0)
    .bind(&req.peer_verify_key)
    .bind(&req.content)
    .bind(&req.signature)
    .bind(now)
    .bind(v_int)
    .execute(&state.db)
    .await?;

    let action = if verified {
        "message.verified"
    } else {
        "message.verification_failed"
    };
    crate::audit_log::record(
        &state.db,
        &tenant.0,
        action,
        Some(&req.peer_verify_key),
        None,
        now,
    )
    .await?;

    Ok(Json(VerifyResponse {
        verified,
        peer_verify_key: req.peer_verify_key,
        timestamp: now,
    }))
}

pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<Vec<MessageRecord>>> {
    let rows = sqlx::query(
        "SELECT id, peer_verify_key, direction, content, signature, timestamp, verified
         FROM messages WHERE tenant_id=$1 ORDER BY timestamp DESC LIMIT 100",
    )
    .bind(&tenant.0)
    .fetch_all(&state.db)
    .await?;

    let records = rows
        .iter()
        .map(|r| -> Result<MessageRecord, sqlx::Error> {
            Ok(MessageRecord {
                id: r.try_get(0)?,
                peer_verify_key: r.try_get(1)?,
                direction: r.try_get(2)?,
                content: r.try_get(3)?,
                signature: r.try_get(4)?,
                timestamp: r.try_get(5)?,
                verified: r.try_get::<i64, _>(6)? != 0,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(records))
}
