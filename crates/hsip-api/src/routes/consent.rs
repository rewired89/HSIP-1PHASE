use axum::{extract::{Path, State}, Json};
use serde::{Deserialize, Serialize};
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
    let now    = now_ms();
    let ttl    = req.ttl_ms.unwrap_or(3_600_000);
    let exp    = now + ttl;
    let id     = Uuid::new_v4().to_string();
    let db     = state.db.clone();
    let tid    = tenant.0.clone();
    let peer   = req.peer_verify_key.clone();
    let eid    = id.clone();

    tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        conn.execute(
            "INSERT INTO consents (id, tenant_id, peer_verify_key, status, granted_at, expires_ms, created_at)
             VALUES (?1,?2,?3,'granted',?4,?5,?6)
             ON CONFLICT(tenant_id, peer_verify_key)
             DO UPDATE SET status='granted', granted_at=excluded.granted_at,
                           expires_ms=excluded.expires_ms, revoked_at=NULL",
            rusqlite::params![eid, tid, peer, now, exp, now],
        ).map_err(|e| ApiError::Internal(e.to_string()))?;
        let audit = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO audit_entries (id,tenant_id,action,peer_verify_key,details,timestamp)
             VALUES (?1,?2,'consent.granted',?3,?4,?5)",
            rusqlite::params![audit, tid, peer, format!("expires_at={exp}"), now],
        ).map_err(|e| ApiError::Internal(e.to_string()))?;
        Ok::<_, ApiError>(())
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(ConsentRecord {
        id, peer_verify_key: req.peer_verify_key,
        status: "granted".into(), granted_at: Some(now),
        expires_at: Some(exp), revoked_at: None, created_at: now,
    }))
}

pub async fn revoke(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(req): Json<RevokeRequest>,
) -> ApiResult<Json<ConsentRecord>> {
    let now  = now_ms();
    let db   = state.db.clone();
    let tid  = tenant.0.clone();
    let peer = req.peer_verify_key.clone();

    let record: ConsentRecord = tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        let affected = conn.execute(
            "UPDATE consents SET status='revoked', revoked_at=?1
             WHERE tenant_id=?2 AND peer_verify_key=?3",
            rusqlite::params![now, tid, peer],
        ).map_err(|e| ApiError::Internal(e.to_string()))?;

        if affected == 0 {
            return Err(ApiError::NotFound(format!("No consent for peer {peer}")));
        }

        let row = conn.query_row(
            "SELECT id, granted_at, expires_ms, created_at FROM consents
             WHERE tenant_id=?1 AND peer_verify_key=?2",
            rusqlite::params![tid, peer],
            |r| Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<i64>>(1)?,
                r.get::<_, Option<i64>>(2)?,
                r.get::<_, i64>(3)?,
            )),
        ).map_err(|e| ApiError::Internal(e.to_string()))?;

        let audit = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO audit_entries (id,tenant_id,action,peer_verify_key,timestamp)
             VALUES (?1,?2,'consent.revoked',?3,?4)",
            rusqlite::params![audit, tid, peer, now],
        ).map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(ConsentRecord {
            id: row.0, peer_verify_key: peer,
            status: "revoked".into(), granted_at: row.1,
            expires_at: row.2, revoked_at: Some(now), created_at: row.3,
        })
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(record))
}

pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<Vec<ConsentRecord>>> {
    let db  = state.db.clone();
    let tid = tenant.0.clone();

    let records = tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        let mut stmt = conn.prepare(
            "SELECT id,peer_verify_key,status,granted_at,expires_ms,revoked_at,created_at
             FROM consents WHERE tenant_id=?1 ORDER BY created_at DESC"
        ).map_err(|e| ApiError::Internal(e.to_string()))?;

        let rows = stmt.query_map(rusqlite::params![tid], |r| Ok(ConsentRecord {
            id:              r.get(0)?,
            peer_verify_key: r.get(1)?,
            status:          r.get(2)?,
            granted_at:      r.get(3)?,
            expires_at:      r.get(4)?,
            revoked_at:      r.get(5)?,
            created_at:      r.get(6)?,
        })).map_err(|e| ApiError::Internal(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| ApiError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(records))
}

pub async fn get(
    State(state): State<AppState>,
    tenant: TenantId,
    Path(peer_key): Path<String>,
) -> ApiResult<Json<ConsentRecord>> {
    let db   = state.db.clone();
    let tid  = tenant.0.clone();
    let peer = peer_key.clone();

    let record = tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|_| ApiError::Internal("lock".into()))?;
        match conn.query_row(
            "SELECT id,peer_verify_key,status,granted_at,expires_ms,revoked_at,created_at
             FROM consents WHERE tenant_id=?1 AND peer_verify_key=?2",
            rusqlite::params![tid, peer],
            |r| Ok(ConsentRecord {
                id:              r.get(0)?,
                peer_verify_key: r.get(1)?,
                status:          r.get(2)?,
                granted_at:      r.get(3)?,
                expires_at:      r.get(4)?,
                revoked_at:      r.get(5)?,
                created_at:      r.get(6)?,
            }),
        ) {
            Ok(row) => Ok(row),
            Err(rusqlite::Error::QueryReturnedNoRows) =>
                Err(ApiError::NotFound(format!("No consent for peer {peer}"))),
            Err(e)  => Err(ApiError::Internal(e.to_string())),
        }
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(record))
}
