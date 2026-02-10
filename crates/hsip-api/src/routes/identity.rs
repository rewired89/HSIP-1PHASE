use axum::{extract::State, Json};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde::Serialize;
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
    let db  = state.db.clone();
    let tid = tenant.0.clone();

    let existing: Option<(String, i64)> = tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        match conn.query_row(
            "SELECT verify_key_b64, created_at FROM identities WHERE tenant_id = ?",
            rusqlite::params![tid],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
        ) {
            Ok(row)                                   => Ok(Some(row)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e)                                    => Err(ApiError::Internal(e.to_string())),
        }
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    if let Some((verify_key, created_at)) = existing {
        return Ok(Json(IdentityResponse { tenant_id: tenant.0, verify_key, created_at }));
    }

    let signing_key = SigningKey::generate(&mut OsRng);
    let verify_key  = signing_key.verifying_key();
    let signing_b64 = BASE64.encode(signing_key.to_bytes());
    let verify_b64  = BASE64.encode(verify_key.to_bytes());
    let now         = now_ms();
    let audit_id    = Uuid::new_v4().to_string();
    let db          = state.db.clone();
    let tid         = tenant.0.clone();
    let vb64        = verify_b64.clone();

    tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        conn.execute(
            "INSERT INTO identities (tenant_id, signing_key_b64, verify_key_b64, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![tid, signing_b64, vb64, now],
        ).map_err(|e| ApiError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT INTO audit_entries (id, tenant_id, action, details, timestamp)
             VALUES (?1, ?2, 'identity.created', ?3, ?4)",
            rusqlite::params![audit_id, tid, vb64, now],
        ).map_err(|e| ApiError::Internal(e.to_string()))?;
        Ok::<_, ApiError>(())
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(IdentityResponse { tenant_id: tenant.0, verify_key: verify_b64, created_at: now }))
}

pub async fn get(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<IdentityResponse>> {
    let db  = state.db.clone();
    let tid = tenant.0.clone();

    let (verify_key, created_at) = tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        match conn.query_row(
            "SELECT verify_key_b64, created_at FROM identities WHERE tenant_id = ?",
            rusqlite::params![tid],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
        ) {
            Ok(row) => Ok(row),
            Err(rusqlite::Error::QueryReturnedNoRows) =>
                Err(ApiError::NotFound("No identity. POST /v1/identity to create one.".into())),
            Err(e)  => Err(ApiError::Internal(e.to_string())),
        }
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(IdentityResponse { tenant_id: tenant.0, verify_key, created_at }))
}
