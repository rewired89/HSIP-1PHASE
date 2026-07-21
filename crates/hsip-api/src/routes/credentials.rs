use axum::{
    extract::{Path, State},
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::BTreeMap;
use uuid::Uuid;

use super::identity::load_signing_key;
use crate::{
    auth::TenantId,
    db::now_ms,
    errors::{ApiError, ApiResult},
    metrics,
    state::AppState,
};

#[derive(Deserialize)]
pub struct IssueRequest {
    pub claim: String,
    pub user_token: String,
    pub ttl_seconds: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CredentialPayload {
    pub id: String,
    pub claim: String,
    pub user_token: String,
    pub issuer_verify_key: String,
    pub issued_at: i64,
    pub expires_at: i64,
}

#[derive(Serialize)]
pub struct IssueResponse {
    pub credential: CredentialPayload,
    pub signature: String,
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub credential: CredentialPayload,
    pub signature: String,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub valid: bool,
    pub claim: String,
    pub expired: bool,
    pub revoked: bool,
    pub expires_at: i64,
}

#[derive(Serialize)]
pub struct CredentialRecord {
    pub id: String,
    pub claim: String,
    pub user_token: String,
    pub issuer_verify_key: String,
    pub issued_at: i64,
    pub expires_at: i64,
    pub revoked: bool,
}

/// M1: Produce a stable canonical JSON string for signing/verification.
/// Uses BTreeMap to guarantee alphabetical key ordering, making the output
/// deterministic across all languages and implementations.
fn canonical_json(payload: &CredentialPayload) -> Result<String, ApiError> {
    let mut map = BTreeMap::new();
    map.insert("claim", serde_json::Value::String(payload.claim.clone()));
    map.insert(
        "expires_at",
        serde_json::Value::Number(payload.expires_at.into()),
    );
    map.insert("id", serde_json::Value::String(payload.id.clone()));
    map.insert(
        "issued_at",
        serde_json::Value::Number(payload.issued_at.into()),
    );
    map.insert(
        "issuer_verify_key",
        serde_json::Value::String(payload.issuer_verify_key.clone()),
    );
    map.insert(
        "user_token",
        serde_json::Value::String(payload.user_token.clone()),
    );
    serde_json::to_string(&map).map_err(|e| {
        tracing::error!(error = %e, "failed to serialize credential claim map");
        ApiError::Internal("internal server error".into())
    })
}

pub async fn issue(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(req): Json<IssueRequest>,
) -> ApiResult<Json<IssueResponse>> {
    // Validate claim length
    if req.claim.len() > 64 {
        return Err(ApiError::BadRequest(
            "claim must be 64 characters or fewer".into(),
        ));
    }
    // Validate user_token length
    if req.user_token.len() > 256 {
        return Err(ApiError::BadRequest(
            "user_token must be 256 characters or fewer".into(),
        ));
    }

    // C1: load and decrypt the signing key
    let master_key = state.master_key.read().await;
    let signing_key = load_signing_key(&state.db, &tenant.0, &master_key).await?;
    let verify_b64 = BASE64.encode(signing_key.verifying_key().to_bytes());

    let now = now_ms();
    let ttl_ms = req.ttl_seconds.unwrap_or(86400) * 1000;
    let expires_at = now + ttl_ms;
    let cred_id = Uuid::new_v4().to_string();

    let payload = CredentialPayload {
        id: cred_id.clone(),
        claim: req.claim.clone(),
        user_token: req.user_token.clone(),
        issuer_verify_key: verify_b64.clone(),
        issued_at: now,
        expires_at,
    };

    // M1: sign canonical JSON (alphabetically-sorted keys)
    let canonical = canonical_json(&payload)?;
    let signature = signing_key.sign(canonical.as_bytes());
    let sig_b64 = BASE64.encode(signature.to_bytes());

    sqlx::query(
        "INSERT INTO credentials
         (id, tenant_id, claim, user_token, issuer_verify_key, issued_at, expires_at, signature, revoked)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0)",
    )
    .bind(&cred_id)
    .bind(&tenant.0)
    .bind(&req.claim)
    .bind(&req.user_token)
    .bind(&verify_b64)
    .bind(now)
    .bind(expires_at)
    .bind(&sig_b64)
    .execute(&state.db)
    .await?;

    crate::audit_log::record_best_effort(
        &state.db,
        &tenant.0,
        "credential.issued",
        None,
        Some(&req.claim),
        now,
    )
    .await;

    metrics::CREDENTIALS_ISSUED.inc();

    Ok(Json(IssueResponse {
        credential: payload,
        signature: sig_b64,
    }))
}

pub async fn verify(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(req): Json<VerifyRequest>,
) -> ApiResult<Json<VerifyResponse>> {
    let now = now_ms();
    let expired = now > req.credential.expires_at;

    // C3: revocation check uses credential ID only (globally unique UUID).
    // Do NOT filter by tenant_id — the verifying tenant may differ from the issuing tenant.
    // The cryptographic signature already proves authenticity.
    let revoked_row = sqlx::query("SELECT revoked FROM credentials WHERE id = $1")
        .bind(&req.credential.id)
        .fetch_optional(&state.db)
        .await?;

    let revoked = revoked_row
        .map(|r| r.try_get::<i64, _>(0).unwrap_or(0) != 0)
        .unwrap_or(false);

    let key_bytes: [u8; 32] = BASE64
        .decode(&req.credential.issuer_verify_key)
        .map_err(|_| ApiError::BadRequest("Invalid issuer_verify_key".into()))?
        .try_into()
        .map_err(|_| ApiError::BadRequest("issuer_verify_key must be 32 bytes".into()))?;

    let verifying_key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|_| ApiError::BadRequest("Invalid Ed25519 key".into()))?;

    let sig_bytes: [u8; 64] = BASE64
        .decode(&req.signature)
        .map_err(|_| ApiError::BadRequest("Invalid signature".into()))?
        .try_into()
        .map_err(|_| ApiError::BadRequest("Signature must be 64 bytes".into()))?;

    let signature = Signature::from_bytes(&sig_bytes);
    // M1: verify against canonical JSON (same alphabetical ordering used at issuance)
    let canonical = canonical_json(&req.credential)?;
    let sig_valid = verifying_key
        .verify(canonical.as_bytes(), &signature)
        .is_ok();

    let valid = sig_valid && !expired && !revoked;
    let result = if valid { "valid" } else { "invalid" };
    metrics::CREDENTIALS_VERIFIED
        .with_label_values(&[result])
        .inc();

    let action = if valid {
        "credential.verified"
    } else {
        "credential.verification_failed"
    };
    crate::audit_log::record_best_effort(
        &state.db,
        &tenant.0,
        action,
        None,
        Some(&req.credential.claim),
        now,
    )
    .await;

    Ok(Json(VerifyResponse {
        valid,
        claim: req.credential.claim,
        expired,
        revoked,
        expires_at: req.credential.expires_at,
    }))
}

pub async fn revoke(
    State(state): State<AppState>,
    tenant: TenantId,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let now = now_ms();
    let result = sqlx::query("UPDATE credentials SET revoked = 1 WHERE id = $1 AND tenant_id = $2")
        .bind(&id)
        .bind(&tenant.0)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Credential not found".into()));
    }

    crate::audit_log::record_best_effort(
        &state.db,
        &tenant.0,
        "credential.revoked",
        None,
        Some(&id),
        now,
    )
    .await;

    Ok(Json(serde_json::json!({ "revoked": true, "id": id })))
}

pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<Vec<CredentialRecord>>> {
    let rows = sqlx::query(
        "SELECT id, claim, user_token, issuer_verify_key, issued_at, expires_at, revoked
         FROM credentials WHERE tenant_id = $1
         ORDER BY issued_at DESC LIMIT 100",
    )
    .bind(&tenant.0)
    .fetch_all(&state.db)
    .await?;

    let records = rows
        .iter()
        .map(|r| -> Result<CredentialRecord, sqlx::Error> {
            Ok(CredentialRecord {
                id: r.try_get(0)?,
                claim: r.try_get(1)?,
                user_token: r.try_get(2)?,
                issuer_verify_key: r.try_get(3)?,
                issued_at: r.try_get(4)?,
                expires_at: r.try_get(5)?,
                revoked: r.try_get::<i64, _>(6)? != 0,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(records))
}
