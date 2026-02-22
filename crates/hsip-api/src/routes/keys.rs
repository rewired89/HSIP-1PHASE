use axum::{extract::{Path, State}, Json};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{auth::{TenantId, hash_key}, db::now_ms, errors::{ApiError, ApiResult}, state::AppState};

#[derive(Deserialize)]
pub struct CreateKeyRequest {
    pub name:       Option<String>,
    pub agent_type: Option<String>, // "human" | "service" | "ai_agent"
    pub expires_in_days: Option<i64>,
}

#[derive(Serialize)]
pub struct CreateKeyResponse {
    pub id:         String,
    pub key:        String,
    pub name:       String,
    pub agent_type: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Serialize)]
pub struct KeyRecord {
    pub id:         String,
    pub name:       String,
    pub agent_type: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub active:     bool,
}

pub async fn create(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(req): Json<CreateKeyRequest>,
) -> ApiResult<Json<CreateKeyResponse>> {
    let raw_key    = gen_key();
    let key_hash   = hash_key(&raw_key);
    let name       = req.name.unwrap_or_else(|| "default".into());
    let agent_type = req.agent_type
        .as_deref()
        .filter(|t| ["human", "service", "ai_agent"].contains(t))
        .unwrap_or("human")
        .to_string();
    let now        = now_ms();
    let id         = Uuid::new_v4().to_string();
    let expires_at = req.expires_in_days.map(|d| now + d * 86_400_000);

    sqlx::query(
        "INSERT INTO api_keys (id, tenant_id, key_hash, name, agent_type, created_at, expires_at, active)
         VALUES (?, ?, ?, ?, ?, ?, ?, 1)",
    )
    .bind(&id)
    .bind(&tenant.0)
    .bind(&key_hash)
    .bind(&name)
    .bind(&agent_type)
    .bind(now)
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    Ok(Json(CreateKeyResponse { id, key: raw_key, name, agent_type, created_at: now, expires_at }))
}

pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<Vec<KeyRecord>>> {
    let rows = sqlx::query(
        "SELECT id, name, agent_type, created_at, expires_at, active
         FROM api_keys WHERE tenant_id = ? ORDER BY created_at DESC",
    )
    .bind(&tenant.0)
    .fetch_all(&state.db)
    .await?;

    let keys = rows.iter().map(|r| -> Result<KeyRecord, sqlx::Error> {
        Ok(KeyRecord {
            id:         r.try_get(0)?,
            name:       r.try_get(1)?,
            agent_type: r.try_get(2)?,
            created_at: r.try_get(3)?,
            expires_at: r.try_get(4)?,
            active:     r.try_get::<i64, _>(5)? != 0,
        })
    }).collect::<Result<Vec<_>, _>>()?;

    Ok(Json(keys))
}

pub async fn revoke(
    State(state): State<AppState>,
    tenant: TenantId,
    Path(key_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let result = sqlx::query(
        "UPDATE api_keys SET active=0 WHERE id=? AND tenant_id=?",
    )
    .bind(&key_id)
    .bind(&tenant.0)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("Key {key_id} not found")));
    }

    state.agent_tracker.remove(&key_id);
    state.rate_limiter.remove(&key_id);
    // Also clear from pending_revocation if present
    state.pending_revocation.remove(&key_id);

    Ok(Json(serde_json::json!({ "revoked": key_id })))
}

/// L1: Use OsRng explicitly for cryptographic key generation.
fn gen_key() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    format!("hsip_{}", hex::encode(bytes))
}
