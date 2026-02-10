use axum::{extract::State, Json};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Verifier, Signature};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::TenantId, db::now_ms, errors::{ApiError, ApiResult}, state::AppState};

#[derive(Deserialize)]
pub struct SignRequest {
    pub content:         String,
    pub peer_verify_key: Option<String>,
}

#[derive(Serialize)]
pub struct SignResponse {
    pub id:        String,
    pub content:   String,
    pub signature: String,
    pub timestamp: i64,
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub content:         String,
    pub signature:       String,
    pub peer_verify_key: String,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub verified:        bool,
    pub peer_verify_key: String,
    pub timestamp:       i64,
}

#[derive(Serialize)]
pub struct MessageRecord {
    pub id:              String,
    pub peer_verify_key: String,
    pub direction:       String,
    pub content:         String,
    pub signature:       String,
    pub timestamp:       i64,
    pub verified:        bool,
}

pub async fn sign(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(req): Json<SignRequest>,
) -> ApiResult<Json<SignResponse>> {
    let db  = state.db.clone();
    let tid = tenant.0.clone();

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

    let key_bytes: [u8; 32] = BASE64.decode(&signing_b64)
        .map_err(|e| ApiError::Internal(format!("key decode: {e}")))?
        .try_into()
        .map_err(|_| ApiError::Internal("bad key length".into()))?;

    let signing_key = SigningKey::from_bytes(&key_bytes);
    let signature   = signing_key.sign(req.content.as_bytes());
    let sig_b64     = BASE64.encode(signature.to_bytes());
    let now         = now_ms();
    let msg_id      = Uuid::new_v4().to_string();
    let peer        = req.peer_verify_key.clone().unwrap_or_default();

    let db   = state.db.clone();
    let tid  = tenant.0.clone();
    let mid  = msg_id.clone();
    let cont = req.content.clone();
    let sig  = sig_b64.clone();
    let p    = peer.clone();

    tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        conn.execute(
            "INSERT INTO messages (id,tenant_id,peer_verify_key,direction,content,signature,timestamp,verified)
             VALUES (?1,?2,?3,'outbound',?4,?5,?6,1)",
            rusqlite::params![mid, tid, p, cont, sig, now],
        ).map_err(|e| ApiError::Internal(e.to_string()))?;
        let aid = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO audit_entries (id,tenant_id,action,peer_verify_key,timestamp)
             VALUES (?1,?2,'message.signed',?3,?4)",
            rusqlite::params![aid, tid, p, now],
        ).map_err(|e| ApiError::Internal(e.to_string()))?;
        Ok::<_, ApiError>(())
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(SignResponse { id: msg_id, content: req.content, signature: sig_b64, timestamp: now }))
}

pub async fn verify(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(req): Json<VerifyRequest>,
) -> ApiResult<Json<VerifyResponse>> {
    let key_bytes: [u8; 32] = BASE64.decode(&req.peer_verify_key)
        .map_err(|_| ApiError::BadRequest("Invalid peer_verify_key encoding".into()))?
        .try_into()
        .map_err(|_| ApiError::BadRequest("peer_verify_key must be 32 bytes".into()))?;

    let verifying_key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|_| ApiError::BadRequest("Invalid Ed25519 verify key".into()))?;

    let sig_bytes: [u8; 64] = BASE64.decode(&req.signature)
        .map_err(|_| ApiError::BadRequest("Invalid signature encoding".into()))?
        .try_into()
        .map_err(|_| ApiError::BadRequest("Signature must be 64 bytes".into()))?;

    let signature = Signature::from_bytes(&sig_bytes);
    let verified  = verifying_key.verify(req.content.as_bytes(), &signature).is_ok();
    let now       = now_ms();
    let msg_id    = Uuid::new_v4().to_string();
    let v_int: i64 = verified as i64;

    let db   = state.db.clone();
    let tid  = tenant.0.clone();
    let mid  = msg_id.clone();
    let cont = req.content.clone();
    let sig  = req.signature.clone();
    let peer = req.peer_verify_key.clone();

    tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        conn.execute(
            "INSERT INTO messages (id,tenant_id,peer_verify_key,direction,content,signature,timestamp,verified)
             VALUES (?1,?2,?3,'inbound',?4,?5,?6,?7)",
            rusqlite::params![mid, tid, peer, cont, sig, now, v_int],
        ).map_err(|e| ApiError::Internal(e.to_string()))?;
        let action = if verified { "message.verified" } else { "message.verification_failed" };
        let aid = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO audit_entries (id,tenant_id,action,peer_verify_key,timestamp)
             VALUES (?1,?2,?3,?4,?5)",
            rusqlite::params![aid, tid, action, peer, now],
        ).map_err(|e| ApiError::Internal(e.to_string()))?;
        Ok::<_, ApiError>(())
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(VerifyResponse { verified, peer_verify_key: req.peer_verify_key, timestamp: now }))
}

pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<Vec<MessageRecord>>> {
    let db  = state.db.clone();
    let tid = tenant.0.clone();

    let records = tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        let mut stmt = conn.prepare(
            "SELECT id,peer_verify_key,direction,content,signature,timestamp,verified
             FROM messages WHERE tenant_id=?1 ORDER BY timestamp DESC LIMIT 100"
        ).map_err(|e| ApiError::Internal(e.to_string()))?;

        let rows = stmt.query_map(rusqlite::params![tid], |r| Ok(MessageRecord {
            id:              r.get(0)?,
            peer_verify_key: r.get(1)?,
            direction:       r.get(2)?,
            content:         r.get(3)?,
            signature:       r.get(4)?,
            timestamp:       r.get(5)?,
            verified:        r.get::<_, i64>(6)? != 0,
        })).map_err(|e| ApiError::Internal(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| ApiError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(records))
}
