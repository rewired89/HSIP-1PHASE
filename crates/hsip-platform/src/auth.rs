//! Authentication and authorization. Three kinds of caller:
//! - console users: session token `hss_...`, role-checked per organization
//! - machines: API key `hsp_...`, scoped to ingest or read (optionally one stream)
//! - the platform operator: `HSP_OPERATOR_KEY`
//!
//! Tokens are stored only as SHA-256 hashes.

use axum::http::HeaderMap;
use sqlx::Row;

use crate::{
    crypto,
    db::now_ms,
    errors::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Owner,
    Admin,
    Developer,
    /// Read-only, can export evidence.
    Auditor,
}

impl Role {
    pub fn parse(s: &str) -> Option<Role> {
        match s {
            "owner" => Some(Role::Owner),
            "admin" => Some(Role::Admin),
            "developer" => Some(Role::Developer),
            "auditor" => Some(Role::Auditor),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Owner => "owner",
            Role::Admin => "admin",
            Role::Developer => "developer",
            Role::Auditor => "auditor",
        }
    }
    pub fn is_admin(self) -> bool {
        matches!(self, Role::Owner | Role::Admin)
    }
    pub fn can_write(self) -> bool {
        !matches!(self, Role::Auditor)
    }
}

pub fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|t| !t.is_empty())
}

#[derive(Debug, Clone)]
pub struct SessionUser {
    pub id: String,
    pub email: String,
    pub name: String,
    pub token_hash: String,
}

pub async fn session_user(state: &AppState, headers: &HeaderMap) -> ApiResult<SessionUser> {
    let token = bearer(headers).ok_or_else(|| ApiError::Unauthorized("sign in required".into()))?;
    if !token.starts_with("hss_") {
        return Err(ApiError::Unauthorized("sign in required".into()));
    }
    let th = crypto::sha256_hex(token);
    let row = sqlx::query(
        "SELECT u.id, u.email, u.name, s.expires_at FROM sessions s JOIN users u ON u.id = s.user_id WHERE s.token_hash = $1",
    )
    .bind(&th)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::Unauthorized("session expired, sign in again".into()))?;
    let exp: i64 = row.try_get(3)?;
    if exp < now_ms() {
        let _ = sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(&th)
            .execute(&state.db)
            .await;
        return Err(ApiError::Unauthorized(
            "session expired, sign in again".into(),
        ));
    }
    Ok(SessionUser {
        id: row.try_get(0)?,
        email: row.try_get(1)?,
        name: row.try_get(2)?,
        token_hash: th,
    })
}

#[derive(Debug, Clone)]
pub struct Member {
    pub user: SessionUser,
    pub org_id: String,
    pub role: Role,
}

impl Member {
    pub fn require_admin(&self) -> ApiResult<()> {
        if self.role.is_admin() {
            Ok(())
        } else {
            Err(ApiError::Forbidden(
                "this action needs the admin or owner role".into(),
            ))
        }
    }
    pub fn require_write(&self) -> ApiResult<()> {
        if self.role.can_write() {
            Ok(())
        } else {
            Err(ApiError::Forbidden("auditors have read-only access".into()))
        }
    }
    pub fn require_owner(&self) -> ApiResult<()> {
        if self.role == Role::Owner {
            Ok(())
        } else {
            Err(ApiError::Forbidden(
                "this action needs the owner role".into(),
            ))
        }
    }
    /// Audit-trail label for this person.
    pub fn actor(&self) -> String {
        format!("user:{}", self.user.email)
    }
}

/// Resolves the session and confirms membership in `org_id`. A non-member
/// gets 404, not 403: organization ids are not confirmable by outsiders.
pub async fn member(state: &AppState, headers: &HeaderMap, org_id: &str) -> ApiResult<Member> {
    let user = session_user(state, headers).await?;
    let row = sqlx::query("SELECT role FROM memberships WHERE org_id = $1 AND user_id = $2")
        .bind(org_id)
        .bind(&user.id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("organization not found".into()))?;
    let role: String = row.try_get(0)?;
    let role = Role::parse(&role).ok_or_else(ApiError::internal)?;
    Ok(Member {
        user,
        org_id: org_id.to_string(),
        role,
    })
}

#[derive(Debug, Clone)]
pub struct ApiKeyAuth {
    pub key_id: String,
    pub name: String,
    pub org_id: String,
    pub stream_id: Option<String>,
    pub scope: String,
}

pub async fn api_key(state: &AppState, headers: &HeaderMap) -> ApiResult<ApiKeyAuth> {
    let token = bearer(headers).ok_or_else(|| ApiError::Unauthorized("missing API key".into()))?;
    if !token.starts_with("hsp_") {
        return Err(ApiError::Unauthorized("invalid API key".into()));
    }
    let th = crypto::sha256_hex(token);
    let row = sqlx::query(
        "SELECT id, name, org_id, stream_id, scope, revoked_at FROM api_keys WHERE key_hash = $1",
    )
    .bind(&th)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::Unauthorized("invalid API key".into()))?;
    let revoked: Option<i64> = row.try_get(5)?;
    if revoked.is_some() {
        return Err(ApiError::Unauthorized("API key has been revoked".into()));
    }
    let auth = ApiKeyAuth {
        key_id: row.try_get(0)?,
        name: row.try_get(1)?,
        org_id: row.try_get(2)?,
        stream_id: row.try_get(3)?,
        scope: row.try_get(4)?,
    };
    // 1,200 calls/min per key. Cheap guard, not a billing meter.
    if !state.allow(&format!("key:{}", auth.key_id), 1200, 60_000) {
        return Err(ApiError::TooManyRequests(
            "rate limit exceeded for this API key".into(),
        ));
    }
    // Touch last_used at most once a minute per key.
    if state.allow(&format!("touch:{}", auth.key_id), 1, 60_000) {
        let _ = sqlx::query("UPDATE api_keys SET last_used_at = $1 WHERE id = $2")
            .bind(now_ms())
            .bind(&auth.key_id)
            .execute(&state.db)
            .await;
    }
    Ok(auth)
}

pub fn operator(state: &AppState, headers: &HeaderMap) -> ApiResult<()> {
    let expected = state
        .cfg
        .operator_key
        .as_deref()
        .ok_or_else(|| ApiError::NotFound("not found".into()))?;
    let given =
        bearer(headers).ok_or_else(|| ApiError::Unauthorized("operator key required".into()))?;
    if crypto::ct_eq(given, expected) {
        Ok(())
    } else {
        Err(ApiError::Unauthorized("operator key required".into()))
    }
}

pub fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "local".into())
}
