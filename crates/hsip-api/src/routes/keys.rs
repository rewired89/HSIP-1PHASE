use axum::{extract::{Path, State}, Json};
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::{TenantId, hash_key}, db::now_ms, errors::{ApiError, ApiResult}, state::AppState};

#[derive(Deserialize)]
pub struct CreateKeyRequest {
    pub name: Option<String>,
}

#[derive(Serialize)]
pub struct CreateKeyResponse {
    pub id:         String,
    pub key:        String,
    pub name:       String,
    pub created_at: i64,
}

#[derive(Serialize)]
pub struct KeyRecord {
    pub id:         String,
    pub name:       String,
    pub created_at: i64,
    pub active:     bool,
}

pub async fn create(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(req): Json<CreateKeyRequest>,
) -> ApiResult<Json<CreateKeyResponse>> {
    let raw_key  = gen_key();
    let key_hash = hash_key(&raw_key);
    let name     = req.name.unwrap_or_else(|| "default".into());
    let now      = now_ms();
    let id       = Uuid::new_v4().to_string();
    let db       = state.db.clone();
    let tid      = tenant.0.clone();
    let kid      = id.clone();
    let kname    = name.clone();

    tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        conn.execute(
            "INSERT INTO api_keys (id,tenant_id,key_hash,name,created_at,active)
             VALUES (?1,?2,?3,?4,?5,1)",
            rusqlite::params![kid, tid, key_hash, kname, now],
        ).map_err(|e| ApiError::Internal(e.to_string()))?;
        Ok::<_, ApiError>(())
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(CreateKeyResponse { id, key: raw_key, name, created_at: now }))
}

pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<Vec<KeyRecord>>> {
    let db  = state.db.clone();
    let tid = tenant.0.clone();

    let keys = tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        let mut stmt = conn.prepare(
            "SELECT id,name,created_at,active FROM api_keys WHERE tenant_id=?1 ORDER BY created_at DESC"
        ).map_err(|e| ApiError::Internal(e.to_string()))?;
        let rows = stmt.query_map(rusqlite::params![tid], |r| Ok(KeyRecord {
            id:         r.get(0)?,
            name:       r.get(1)?,
            created_at: r.get(2)?,
            active:     r.get::<_, i64>(3)? != 0,
        })).map_err(|e| ApiError::Internal(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| ApiError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(keys))
}

pub async fn revoke(
    State(state): State<AppState>,
    tenant: TenantId,
    Path(key_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let db  = state.db.clone();
    let tid = tenant.0.clone();
    let kid = key_id.clone();

    let affected = tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        conn.execute(
            "UPDATE api_keys SET active=0 WHERE id=?1 AND tenant_id=?2",
            rusqlite::params![kid, tid],
        ).map_err(|e| ApiError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    if affected == 0 {
        return Err(ApiError::NotFound(format!("Key {key_id} not found")));
    }

    Ok(Json(serde_json::json!({ "revoked": key_id })))
}

fn gen_key() -> String {
    let bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen::<u8>()).collect();
    format!("hsip_{}", hex::encode(bytes))
}
