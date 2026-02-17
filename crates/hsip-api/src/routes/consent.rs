use axum::{extract::{Path, State}, Json};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{auth::TenantId, db::now_ms, errors::{ApiError, ApiResult}, state::AppState};

#[derive(Deserialize)]
pub struct GrantRequest {
    pub peer_verify_key: String,
    pub ttl_ms:          Option<i64>,
}

#[derive(Deserialize)]
pub struct RevokeRequest {
    pub peer_verify_key: String,
}

#[derive(Serialize, Clone)]
pub struct ConsentRecord {
    pub id:              String,
    pub peer_verify_key: String,
    pub status:          String,
    pub granted_at:      Option<i64>,
    pub expires_at:      Option<i64>,
    pub revoked_at:      Option<i64>,
    pub created_at:      i64,
}

pub async fn grant(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(req): Json<GrantRequest>,
) -> ApiResult<Json<ConsentRecord>> {
    let now  = now_ms();
    let ttl  = req.ttl_ms.unwrap_or(3_600_000);
    let exp  = now + ttl;
    let id   = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO consents (id, tenant_id, peer_verify_key, status, granted_at, expires_ms, created_at)
         VALUES (?, ?, ?, 'granted', ?, ?, ?)
         ON CONFLICT (tenant_id, peer_verify_key)
         DO UPDATE SET status='granted', granted_at=excluded.granted_at,
                       expires_ms=excluded.expires_ms, revoked_at=NULL",
    )
    .bind(&id)
    .bind(&tenant.0)
    .bind(&req.peer_verify_key)
    .bind(now)
    .bind(exp)
    .bind(now)
    .execute(&state.db)
    .await?;

    let audit_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO audit_entries (id, tenant_id, action, peer_verify_key, details, timestamp)
         VALUES (?, ?, 'consent.granted', ?, ?, ?)",
    )
    .bind(&audit_id)
    .bind(&tenant.0)
    .bind(&req.peer_verify_key)
    .bind(format!("expires_at={exp}"))
    .bind(now)
    .execute(&state.db)
    .await?;

    Ok(Json(ConsentRecord {
        id,
        peer_verify_key: req.peer_verify_key,
        status: "granted".into(),
        granted_at: Some(now),
        expires_at: Some(exp),
        revoked_at: None,
        created_at: now,
    }))
}

pub async fn revoke(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(req): Json<RevokeRequest>,
) -> ApiResult<Json<ConsentRecord>> {
    let now = now_ms();

    let result = sqlx::query(
        "UPDATE consents SET status='revoked', revoked_at=?
         WHERE tenant_id=? AND peer_verify_key=?",
    )
    .bind(now)
    .bind(&tenant.0)
    .bind(&req.peer_verify_key)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("No consent for peer {}", req.peer_verify_key)));
    }

    let row = sqlx::query(
        "SELECT id, granted_at, expires_ms, created_at
         FROM consents WHERE tenant_id=? AND peer_verify_key=?",
    )
    .bind(&tenant.0)
    .bind(&req.peer_verify_key)
    .fetch_one(&state.db)
    .await?;

    let audit_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO audit_entries (id, tenant_id, action, peer_verify_key, timestamp)
         VALUES (?, ?, 'consent.revoked', ?, ?)",
    )
    .bind(&audit_id)
    .bind(&tenant.0)
    .bind(&req.peer_verify_key)
    .bind(now)
    .execute(&state.db)
    .await?;

    Ok(Json(ConsentRecord {
        id:              row.try_get(0)?,
        peer_verify_key: req.peer_verify_key,
        status:          "revoked".into(),
        granted_at:      row.try_get(1)?,
        expires_at:      row.try_get(2)?,
        revoked_at:      Some(now),
        created_at:      row.try_get(3)?,
    }))
}

pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<Vec<ConsentRecord>>> {
    let rows = sqlx::query(
        "SELECT id, peer_verify_key, status, granted_at, expires_ms, revoked_at, created_at
         FROM consents WHERE tenant_id=? ORDER BY created_at DESC",
    )
    .bind(&tenant.0)
    .fetch_all(&state.db)
    .await?;

    let records = rows.iter().map(|r| -> Result<ConsentRecord, sqlx::Error> {
        Ok(ConsentRecord {
            id:              r.try_get(0)?,
            peer_verify_key: r.try_get(1)?,
            status:          r.try_get(2)?,
            granted_at:      r.try_get(3)?,
            expires_at:      r.try_get(4)?,
            revoked_at:      r.try_get(5)?,
            created_at:      r.try_get(6)?,
        })
    }).collect::<Result<Vec<_>, _>>()?;

    Ok(Json(records))
}

pub async fn get(
    State(state): State<AppState>,
    tenant: TenantId,
    Path(peer_key): Path<String>,
) -> ApiResult<Json<ConsentRecord>> {
    let row = sqlx::query(
        "SELECT id, peer_verify_key, status, granted_at, expires_ms, revoked_at, created_at
         FROM consents WHERE tenant_id=? AND peer_verify_key=?",
    )
    .bind(&tenant.0)
    .bind(&peer_key)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("No consent for peer {peer_key}")))?;

    Ok(Json(ConsentRecord {
        id:              row.try_get(0)?,
        peer_verify_key: row.try_get(1)?,
        status:          row.try_get(2)?,
        granted_at:      row.try_get(3)?,
        expires_at:      row.try_get(4)?,
        revoked_at:      row.try_get(5)?,
        created_at:      row.try_get(6)?,
    }))
}
