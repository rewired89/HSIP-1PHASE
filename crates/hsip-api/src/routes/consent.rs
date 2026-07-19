use axum::{
    extract::{Path, Query, State},
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
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
pub struct GrantRequest {
    pub peer_verify_key: String,
    pub ttl_ms: Option<i64>,
}

#[derive(Deserialize)]
pub struct RevokeRequest {
    pub peer_verify_key: String,
}

/// L3: Pagination query params
#[derive(Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Serialize, Clone)]
pub struct ConsentRecord {
    pub id: String,
    pub peer_verify_key: String,
    /// Effective status: "granted", "expired", or "revoked"
    pub status: String,
    pub granted_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub revoked_at: Option<i64>,
    pub created_at: i64,
}

/// M2: Validate that peer_verify_key is a valid Base64-encoded 32-byte Ed25519 public key.
fn validate_peer_key(peer_verify_key: &str) -> ApiResult<()> {
    if peer_verify_key.len() > 128 {
        return Err(ApiError::BadRequest("peer_verify_key too long".into()));
    }
    let decoded = BASE64
        .decode(peer_verify_key)
        .map_err(|_| ApiError::BadRequest("peer_verify_key must be Base64-encoded".into()))?;
    if decoded.len() != 32 {
        return Err(ApiError::BadRequest(
            "peer_verify_key must decode to exactly 32 bytes (Ed25519 public key)".into(),
        ));
    }
    Ok(())
}

/// H5: Compute effective status from DB values and current time.
fn effective_status(db_status: &str, expires_ms: Option<i64>, now: i64) -> String {
    if db_status == "revoked" {
        return "revoked".into();
    }
    if let Some(exp) = expires_ms {
        if now > exp {
            return "expired".into();
        }
    }
    db_status.into()
}

pub async fn grant(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(req): Json<GrantRequest>,
) -> ApiResult<Json<ConsentRecord>> {
    // M2: validate peer key format
    validate_peer_key(&req.peer_verify_key)?;

    let now = now_ms();
    let ttl = req.ttl_ms.unwrap_or(3_600_000);
    let exp = now + ttl;
    let id = Uuid::new_v4().to_string();

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

    crate::audit_log::record(
        &state.db,
        &tenant.0,
        "consent.granted",
        Some(&req.peer_verify_key),
        Some(&format!("expires_at={exp}")),
        now,
    )
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
    // M2: validate peer key format
    validate_peer_key(&req.peer_verify_key)?;

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
        return Err(ApiError::NotFound(format!(
            "No consent for peer {}",
            req.peer_verify_key
        )));
    }

    let row = sqlx::query(
        "SELECT id, granted_at, expires_ms, created_at
         FROM consents WHERE tenant_id=? AND peer_verify_key=?",
    )
    .bind(&tenant.0)
    .bind(&req.peer_verify_key)
    .fetch_one(&state.db)
    .await?;

    crate::audit_log::record(
        &state.db,
        &tenant.0,
        "consent.revoked",
        Some(&req.peer_verify_key),
        None,
        now,
    )
    .await?;

    Ok(Json(ConsentRecord {
        id: row.try_get(0)?,
        peer_verify_key: req.peer_verify_key,
        status: "revoked".into(),
        granted_at: row.try_get(1)?,
        expires_at: row.try_get(2)?,
        revoked_at: Some(now),
        created_at: row.try_get(3)?,
    }))
}

/// L3: paginated consent list with H5 expiry enforcement
pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
    Query(params): Query<PaginationParams>,
) -> ApiResult<Json<Vec<ConsentRecord>>> {
    let limit = params.limit.clamp(1, 200);
    let offset = params.offset.max(0);
    let now = now_ms();

    let rows = sqlx::query(
        "SELECT id, peer_verify_key, status, granted_at, expires_ms, revoked_at, created_at
         FROM consents WHERE tenant_id=? ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(&tenant.0)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let records = rows
        .iter()
        .map(|r| -> Result<ConsentRecord, sqlx::Error> {
            let db_status: String = r.try_get(2)?;
            let expires_ms: Option<i64> = r.try_get(4)?;
            Ok(ConsentRecord {
                id: r.try_get(0)?,
                peer_verify_key: r.try_get(1)?,
                // H5: compute effective status at query time
                status: effective_status(&db_status, expires_ms, now),
                granted_at: r.try_get(3)?,
                expires_at: expires_ms,
                revoked_at: r.try_get(5)?,
                created_at: r.try_get(6)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(records))
}

/// H5: enforce expiry on single-peer lookup
pub async fn get(
    State(state): State<AppState>,
    tenant: TenantId,
    Path(peer_key): Path<String>,
) -> ApiResult<Json<ConsentRecord>> {
    let now = now_ms();

    let row = sqlx::query(
        "SELECT id, peer_verify_key, status, granted_at, expires_ms, revoked_at, created_at
         FROM consents WHERE tenant_id=? AND peer_verify_key=?",
    )
    .bind(&tenant.0)
    .bind(&peer_key)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("No consent for peer {peer_key}")))?;

    let db_status: String = row.try_get(2)?;
    let expires_ms: Option<i64> = row.try_get(4)?;

    Ok(Json(ConsentRecord {
        id: row.try_get(0)?,
        peer_verify_key: row.try_get(1)?,
        // H5: effective status considers expiry
        status: effective_status(&db_status, expires_ms, now),
        granted_at: row.try_get(3)?,
        expires_at: expires_ms,
        revoked_at: row.try_get(5)?,
        created_at: row.try_get(6)?,
    }))
}
