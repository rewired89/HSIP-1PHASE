use axum::{extract::{Path, State}, Json};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Verifier, Signature};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::TenantId, db::now_ms, errors::{ApiError, ApiResult}, state::AppState};

// ── request / response types ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct IssueRequest {
    pub claim:       String,   // e.g. "age_over_18", "kyc_verified", "iso_27001"
    pub user_token:  String,   // opaque hash — caller's blind identifier for the subject
    pub ttl_seconds: Option<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct CredentialPayload {
    pub id:                String,
    pub claim:             String,
    pub user_token:        String,
    pub issuer_verify_key: String,
    pub issued_at:         i64,
    pub expires_at:        i64,
}

#[derive(Serialize)]
pub struct IssueResponse {
    pub credential: CredentialPayload,
    pub signature:  String,
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub credential: CredentialPayload,
    pub signature:  String,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub valid:      bool,
    pub claim:      String,
    pub expired:    bool,
    pub revoked:    bool,
    pub expires_at: i64,
}

#[derive(Serialize)]
pub struct CredentialRecord {
    pub id:                String,
    pub claim:             String,
    pub user_token:        String,
    pub issuer_verify_key: String,
    pub issued_at:         i64,
    pub expires_at:        i64,
    pub revoked:           bool,
}

// ── POST /v1/credentials/issue ───────────────────────────────────────────────

pub async fn issue(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(req): Json<IssueRequest>,
) -> ApiResult<Json<IssueResponse>> {
    let db  = state.db.clone();
    let tid = tenant.0.clone();

    // load tenant signing key
    let signing_b64: String = tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        match conn.query_row(
            "SELECT signing_key_b64 FROM identities WHERE tenant_id = ?",
            rusqlite::params![tid],
            |r| r.get::<_, String>(0),
        ) {
            Ok(k)  => Ok(k),
            Err(rusqlite::Error::QueryReturnedNoRows) =>
                Err(ApiError::BadRequest("No identity. POST /v1/identity first.".into())),
            Err(e) => Err(ApiError::Internal(e.to_string())),
        }
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    // load issuer verify key
    let db  = state.db.clone();
    let tid = tenant.0.clone();
    let verify_b64: String = tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        conn.query_row(
            "SELECT verify_key_b64 FROM identities WHERE tenant_id = ?",
            rusqlite::params![tid],
            |r| r.get::<_, String>(0),
        ).map_err(|e| ApiError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    let key_bytes: [u8; 32] = BASE64.decode(&signing_b64)
        .map_err(|e| ApiError::Internal(format!("key decode: {e}")))?
        .try_into()
        .map_err(|_| ApiError::Internal("bad key length".into()))?;

    let signing_key = SigningKey::from_bytes(&key_bytes);
    let now         = now_ms();
    let ttl_ms      = req.ttl_seconds.unwrap_or(86400) * 1000; // default 24h
    let expires_at  = now + ttl_ms;
    let cred_id     = Uuid::new_v4().to_string();

    let payload = CredentialPayload {
        id:                cred_id.clone(),
        claim:             req.claim.clone(),
        user_token:        req.user_token.clone(),
        issuer_verify_key: verify_b64.clone(),
        issued_at:         now,
        expires_at,
    };

    // sign canonical JSON of the payload
    let canonical = serde_json::to_string(&payload)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let signature  = signing_key.sign(canonical.as_bytes());
    let sig_b64    = BASE64.encode(signature.to_bytes());

    let db   = state.db.clone();
    let tid  = tenant.0.clone();
    let cid  = cred_id.clone();
    let clm  = req.claim.clone();
    let utok = req.user_token.clone();
    let vkey = verify_b64.clone();
    let sig  = sig_b64.clone();

    tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        conn.execute(
            "INSERT INTO credentials
             (id, tenant_id, claim, user_token, issuer_verify_key, issued_at, expires_at, signature, revoked)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,0)",
            rusqlite::params![cid, tid, clm, utok, vkey, now, expires_at, sig],
        ).map_err(|e| ApiError::Internal(e.to_string()))?;
        let aid = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO audit_entries (id, tenant_id, action, details, timestamp)
             VALUES (?1,?2,'credential.issued',?3,?4)",
            rusqlite::params![aid, tid, clm, now],
        ).map_err(|e| ApiError::Internal(e.to_string()))?;
        Ok::<_, ApiError>(())
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(IssueResponse { credential: payload, signature: sig_b64 }))
}

// ── POST /v1/credentials/verify ──────────────────────────────────────────────

pub async fn verify(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(req): Json<VerifyRequest>,
) -> ApiResult<Json<VerifyResponse>> {
    let now     = now_ms();
    let expired = now > req.credential.expires_at;

    // check if revoked in our DB (optional — verifier may not be the issuer)
    let db    = state.db.clone();
    let cid   = req.credential.id.clone();
    let tid   = tenant.0.clone();
    let revoked: bool = tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        match conn.query_row(
            "SELECT revoked FROM credentials WHERE id = ? AND tenant_id = ?",
            rusqlite::params![cid, tid],
            |r| r.get::<_, i64>(0),
        ) {
            Ok(v)  => Ok::<bool, ApiError>(v != 0),
            Err(_) => Ok::<bool, ApiError>(false), // not in our DB — treat as not revoked
        }
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    // verify Ed25519 signature
    let key_bytes: [u8; 32] = BASE64.decode(&req.credential.issuer_verify_key)
        .map_err(|_| ApiError::BadRequest("Invalid issuer_verify_key".into()))?
        .try_into()
        .map_err(|_| ApiError::BadRequest("issuer_verify_key must be 32 bytes".into()))?;

    let verifying_key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|_| ApiError::BadRequest("Invalid Ed25519 key".into()))?;

    let sig_bytes: [u8; 64] = BASE64.decode(&req.signature)
        .map_err(|_| ApiError::BadRequest("Invalid signature".into()))?
        .try_into()
        .map_err(|_| ApiError::BadRequest("Signature must be 64 bytes".into()))?;

    let signature = Signature::from_bytes(&sig_bytes);
    let canonical = serde_json::to_string(&req.credential)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let sig_valid = verifying_key.verify(canonical.as_bytes(), &signature).is_ok();

    let valid = sig_valid && !expired && !revoked;

    // audit
    let db    = state.db.clone();
    let tid   = tenant.0.clone();
    let claim = req.credential.claim.clone();
    let action = if valid { "credential.verified" } else { "credential.verification_failed" };
    tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        let aid  = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO audit_entries (id, tenant_id, action, details, timestamp)
             VALUES (?1,?2,?3,?4,?5)",
            rusqlite::params![aid, tid, action, claim, now],
        ).map_err(|e| ApiError::Internal(e.to_string()))?;
        Ok::<_, ApiError>(())
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(VerifyResponse {
        valid,
        claim:      req.credential.claim,
        expired,
        revoked,
        expires_at: req.credential.expires_at,
    }))
}

// ── DELETE /v1/credentials/:id (revoke) ──────────────────────────────────────

pub async fn revoke(
    State(state): State<AppState>,
    tenant: TenantId,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let db  = state.db.clone();
    let tid = tenant.0.clone();
    let cid = id.clone();
    let now = now_ms();

    tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        let rows = conn.execute(
            "UPDATE credentials SET revoked = 1 WHERE id = ? AND tenant_id = ?",
            rusqlite::params![cid, tid],
        ).map_err(|e| ApiError::Internal(e.to_string()))?;
        if rows == 0 {
            return Err(ApiError::NotFound("Credential not found".into()));
        }
        let aid = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO audit_entries (id, tenant_id, action, details, timestamp)
             VALUES (?1,?2,'credential.revoked',?3,?4)",
            rusqlite::params![aid, tid, cid, now],
        ).map_err(|e| ApiError::Internal(e.to_string()))?;
        Ok::<_, ApiError>(())
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(serde_json::json!({ "revoked": true, "id": id })))
}

// ── GET /v1/credentials ───────────────────────────────────────────────────────

pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<Vec<CredentialRecord>>> {
    let db  = state.db.clone();
    let tid = tenant.0.clone();

    let records = tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        let mut stmt = conn.prepare(
            "SELECT id, claim, user_token, issuer_verify_key, issued_at, expires_at, revoked
             FROM credentials WHERE tenant_id = ?1
             ORDER BY issued_at DESC LIMIT 100"
        ).map_err(|e| ApiError::Internal(e.to_string()))?;

        let rows = stmt.query_map(rusqlite::params![tid], |r| Ok(CredentialRecord {
            id:                r.get(0)?,
            claim:             r.get(1)?,
            user_token:        r.get(2)?,
            issuer_verify_key: r.get(3)?,
            issued_at:         r.get(4)?,
            expires_at:        r.get(5)?,
            revoked:           r.get::<_, i64>(6)? != 0,
        })).map_err(|e| ApiError::Internal(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| ApiError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(records))
}
