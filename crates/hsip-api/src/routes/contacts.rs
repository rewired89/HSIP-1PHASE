use axum::{
    extract::{Path, State},
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::TenantId,
    db::now_ms,
    errors::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Deserialize)]
pub struct AddContactRequest {
    pub nickname: String,
    pub verify_key: String,
}

#[derive(Serialize)]
pub struct ContactRecord {
    pub id: String,
    pub nickname: String,
    pub verify_key: String,
    pub added_at: i64,
}

pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<Vec<ContactRecord>>> {
    let rows = sqlx::query(
        "SELECT id, nickname, verify_key, added_at FROM contacts
         WHERE tenant_id=$1 ORDER BY nickname ASC",
    )
    .bind(&tenant.0)
    .fetch_all(&state.db)
    .await?;

    let contacts = rows
        .iter()
        .map(|r| -> Result<ContactRecord, sqlx::Error> {
            Ok(ContactRecord {
                id: r.try_get(0)?,
                nickname: r.try_get(1)?,
                verify_key: r.try_get(2)?,
                added_at: r.try_get(3)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(contacts))
}

pub async fn add(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(req): Json<AddContactRequest>,
) -> ApiResult<Json<ContactRecord>> {
    if req.nickname.trim().is_empty() {
        return Err(ApiError::BadRequest("nickname is required".into()));
    }

    // Validate the key is a valid Ed25519 public key (32 bytes, base64)
    let key_bytes = BASE64
        .decode(&req.verify_key)
        .map_err(|_| ApiError::BadRequest("verify_key must be valid base64".into()))?;
    let key_arr: [u8; 32] = key_bytes.try_into().map_err(|_| {
        ApiError::BadRequest("verify_key must be 32 bytes (Ed25519 public key)".into())
    })?;
    VerifyingKey::from_bytes(&key_arr)
        .map_err(|_| ApiError::BadRequest("verify_key is not a valid Ed25519 public key".into()))?;

    let id = Uuid::new_v4().to_string();
    let now = now_ms();

    // Upsert: if key already exists for this tenant, update the nickname
    sqlx::query(
        "INSERT INTO contacts (id, tenant_id, nickname, verify_key, added_at)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT(tenant_id, verify_key) DO UPDATE SET nickname = excluded.nickname",
    )
    .bind(&id)
    .bind(&tenant.0)
    .bind(req.nickname.trim())
    .bind(&req.verify_key)
    .bind(now)
    .execute(&state.db)
    .await?;

    // Re-fetch to get the real id (may differ if upsert hit existing row)
    let row = sqlx::query(
        "SELECT id, nickname, verify_key, added_at FROM contacts
         WHERE tenant_id=$1 AND verify_key=$2",
    )
    .bind(&tenant.0)
    .bind(&req.verify_key)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(ContactRecord {
        id: row.try_get(0)?,
        nickname: row.try_get(1)?,
        verify_key: row.try_get(2)?,
        added_at: row.try_get(3)?,
    }))
}

pub async fn remove(
    State(state): State<AppState>,
    tenant: TenantId,
    Path(contact_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM contacts WHERE id=$1 AND tenant_id=$2")
        .bind(&contact_id)
        .bind(&tenant.0)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!(
            "Contact {contact_id} not found"
        )));
    }

    Ok(Json(serde_json::json!({ "removed": contact_id })))
}
