use axum::{
    extract::{Extension, Path, State},
    http::HeaderMap,
    Json,
};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::{hash_key, TenantId},
    db::{now_ms, Db},
    errors::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Deserialize)]
pub struct CreateKeyRequest {
    pub name: Option<String>,
    pub agent_type: Option<String>, // "human" | "service" | "ai_agent"
    pub expires_in_days: Option<i64>,
    /// "owner" | "member" (default). Only an owner-role key can create a
    /// new owner — see `create()`'s privilege check, which already means a
    /// caller with sufficient privilege to reach this field at all is
    /// already an owner.
    pub role: Option<String>,
}

/// Resolves the `role` ('owner' | 'member' | legacy NULL, which behaves as
/// "no key management privilege") of the API key that authenticated this
/// request, scoped to its own tenant. Mirrors the same
/// re-parse-Authorization-header pattern used by
/// `routes::admin::require_root_admin` and
/// `routes::consent::resolve_granting_key_type` — `TenantId` only carries
/// the resolved tenant_id, not the calling key's own attributes.
async fn resolve_caller_role(
    db: &Db,
    headers: &HeaderMap,
    tenant_id: &str,
) -> ApiResult<Option<String>> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .ok_or_else(|| ApiError::Unauthorized("Missing Authorization: Bearer <key>".into()))?;
    let key_hash = hash_key(token);
    let row = sqlx::query("SELECT role FROM api_keys WHERE key_hash = $1 AND tenant_id = $2")
        .bind(&key_hash)
        .bind(tenant_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ApiError::Internal("authenticated key vanished mid-request".into()))?;
    Ok(row.try_get(0)?)
}

#[derive(Serialize)]
pub struct CreateKeyResponse {
    pub id: String,
    pub key: String,
    pub name: String,
    pub agent_type: String,
    pub role: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Serialize)]
pub struct KeyRecord {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub role: Option<String>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub active: bool,
}

pub async fn create(
    State(state): State<AppState>,
    tenant: TenantId,
    headers: HeaderMap,
    Json(req): Json<CreateKeyRequest>,
) -> ApiResult<Json<CreateKeyResponse>> {
    // Only an owner-role key can mint new keys in this tenant. Previously
    // any active key — including a low-privilege ai_agent key — could
    // create a fresh 'human' key for itself with no restriction at all.
    let caller_role = resolve_caller_role(&state.db, &headers, &tenant.0).await?;
    if caller_role.as_deref() != Some("owner") {
        return Err(ApiError::Unauthorized(
            "Only an owner-role key can create new keys in this tenant.".into(),
        ));
    }

    let raw_key = gen_key();
    let key_hash = hash_key(&raw_key);
    let name = req.name.unwrap_or_else(|| "default".into());
    let agent_type = req
        .agent_type
        .as_deref()
        .filter(|t| ["human", "service", "ai_agent"].contains(t))
        .unwrap_or("human")
        .to_string();
    let role = req
        .role
        .as_deref()
        .filter(|r| ["owner", "member"].contains(r))
        .unwrap_or("member")
        .to_string();
    let now = now_ms();
    let id = Uuid::new_v4().to_string();
    let expires_at = req.expires_in_days.map(|d| now + d * 86_400_000);

    sqlx::query(
        "INSERT INTO api_keys (id, tenant_id, key_hash, name, agent_type, role, created_at, expires_at, active)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1)",
    )
    .bind(&id)
    .bind(&tenant.0)
    .bind(&key_hash)
    .bind(&name)
    .bind(&agent_type)
    .bind(&role)
    .bind(now)
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    crate::audit_log::record_best_effort(
        &state.db,
        &tenant.0,
        "key.created",
        None,
        Some(&format!(
            "id={id} name={name} agent_type={agent_type} role={role}"
        )),
        now,
    )
    .await;

    Ok(Json(CreateKeyResponse {
        id,
        key: raw_key,
        name,
        agent_type,
        role,
        created_at: now,
        expires_at,
    }))
}

pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<Vec<KeyRecord>>> {
    let rows = sqlx::query(
        "SELECT id, name, agent_type, role, created_at, expires_at, active
         FROM api_keys WHERE tenant_id = $1 ORDER BY created_at DESC",
    )
    .bind(&tenant.0)
    .fetch_all(&state.db)
    .await?;

    let keys = rows
        .iter()
        .map(|r| -> Result<KeyRecord, sqlx::Error> {
            Ok(KeyRecord {
                id: r.try_get(0)?,
                name: r.try_get(1)?,
                agent_type: r.try_get(2)?,
                role: r.try_get(3)?,
                created_at: r.try_get(4)?,
                expires_at: r.try_get(5)?,
                active: r.try_get::<i64, _>(6)? != 0,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(keys))
}

pub async fn revoke(
    State(state): State<AppState>,
    tenant: TenantId,
    headers: HeaderMap,
    Path(key_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    // Only an owner-role key can revoke keys in this tenant. Previously any
    // active key — including a low-privilege ai_agent key — could revoke
    // ANY other key in the same tenant, including the tenant's own owner
    // key, with no restriction at all.
    let caller_role = resolve_caller_role(&state.db, &headers, &tenant.0).await?;
    if caller_role.as_deref() != Some("owner") {
        return Err(ApiError::Unauthorized(
            "Only an owner-role key can revoke keys in this tenant.".into(),
        ));
    }

    let target = sqlx::query("SELECT role, active FROM api_keys WHERE id=$1 AND tenant_id=$2")
        .bind(&key_id)
        .bind(&tenant.0)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Key {key_id} not found")))?;
    let target_role: Option<String> = target.try_get(0)?;
    let target_active: i64 = target.try_get(1)?;

    // Refuse to revoke a tenant's last remaining active owner — otherwise
    // the tenant becomes permanently unable to manage its own keys (nobody
    // left who can create or revoke keys in it, including recovering from
    // this mistake).
    if target_active != 0 && target_role.as_deref() == Some("owner") {
        let owner_count: i64 = sqlx::query(
            "SELECT COUNT(*) FROM api_keys WHERE tenant_id=$1 AND role='owner' AND active=1",
        )
        .bind(&tenant.0)
        .fetch_one(&state.db)
        .await?
        .try_get(0)?;
        if owner_count <= 1 {
            return Err(ApiError::Conflict(
                "Cannot revoke the last owner-role key in this tenant — it would lock the \
                 tenant out of managing its own keys."
                    .into(),
            ));
        }
    }

    let result = sqlx::query("UPDATE api_keys SET active=0 WHERE id=$1 AND tenant_id=$2")
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

    crate::audit_log::record_best_effort(
        &state.db,
        &tenant.0,
        "key.revoked",
        None,
        Some(&format!("id={key_id}")),
        now_ms(),
    )
    .await;

    Ok(Json(serde_json::json!({ "revoked": key_id })))
}

#[derive(Deserialize)]
pub struct BindClientCertRequest {
    /// If true, clears any existing binding on the target key instead of
    /// setting one — the target key goes back to bearer-token-only auth.
    /// Mutually exclusive with actually binding: when `clear` is true the
    /// caller's own presented certificate (if any) is ignored.
    pub clear: Option<bool>,
}

#[derive(Serialize)]
pub struct BindClientCertResponse {
    pub id: String,
    pub bound_client_cert_fingerprint: Option<String>,
}

/// `POST /v1/keys/:id/bind-client-cert` — binds (or, with `{"clear":true}`,
/// clears) `api_keys.bound_client_cert_fingerprint` on an existing key in
/// the caller's own tenant. Owner-role gated, same as `create`/`revoke`
/// above — key management in general is an owner-only privilege in this
/// tenant, and binding a cert is exactly that.
///
/// The fingerprint bound is always the *caller's own* current connection's
/// presented client certificate (from `mtls::ClientCertAcceptor`, via the
/// `ClientCertFingerprint` request extension) — there is no way to bind an
/// arbitrary fingerprint string the caller doesn't actually possess, which
/// would let an owner brick another key by binding it to a certificate
/// nobody can ever present. To bind key B while authenticated as key A
/// (both owner-role, same tenant), the caller must be connected over mTLS
/// with the certificate intended for B; the server has no way to tell whose
/// intent that is beyond "this connection presented this cert" — the same
/// trust boundary every other owner-role key-management action in this file
/// already operates under (an owner can already revoke or reconfigure any
/// key in its own tenant).
pub async fn bind_client_cert(
    State(state): State<AppState>,
    tenant: TenantId,
    headers: HeaderMap,
    Path(key_id): Path<String>,
    presented: Option<Extension<crate::mtls::ClientCertFingerprint>>,
    Json(req): Json<BindClientCertRequest>,
) -> ApiResult<Json<BindClientCertResponse>> {
    let caller_role = resolve_caller_role(&state.db, &headers, &tenant.0).await?;
    if caller_role.as_deref() != Some("owner") {
        return Err(ApiError::Unauthorized(
            "Only an owner-role key can bind or clear a client-certificate requirement on a key \
             in this tenant."
                .into(),
        ));
    }

    let clear = req.clear.unwrap_or(false);
    let fingerprint: Option<String> = if clear {
        None
    } else {
        let fp = presented.and_then(|Extension(f)| f.0).ok_or_else(|| {
            ApiError::BadRequest(
                "No TLS client certificate was presented on this connection — connect over \
                     mTLS (client_ca_path configured, a matching client cert presented) to bind \
                     one, or pass {\"clear\":true} to remove an existing binding instead."
                    .into(),
            )
        })?;
        Some(fp)
    };

    let result = sqlx::query(
        "UPDATE api_keys SET bound_client_cert_fingerprint = $1 WHERE id = $2 AND tenant_id = $3",
    )
    .bind(&fingerprint)
    .bind(&key_id)
    .bind(&tenant.0)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("Key {key_id} not found")));
    }

    crate::audit_log::record_best_effort(
        &state.db,
        &tenant.0,
        if clear {
            "key.cert_unbound"
        } else {
            "key.cert_bound"
        },
        None,
        Some(&format!("id={key_id}")),
        now_ms(),
    )
    .await;

    Ok(Json(BindClientCertResponse {
        id: key_id,
        bound_client_cert_fingerprint: fingerprint,
    }))
}

/// L1: Use OsRng explicitly for cryptographic key generation.
fn gen_key() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    format!("hsip_{}", hex::encode(bytes))
}
