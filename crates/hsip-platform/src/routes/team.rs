use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    audit,
    auth::{self, Role},
    crypto,
    db::now_ms,
    engine,
    errors::{ApiError, ApiResult},
    plans,
    state::AppState,
};

const INVITE_TTL_MS: i64 = 7 * 24 * 3600 * 1000;

pub async fn list(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
) -> ApiResult<Json<Value>> {
    auth::member(&s, &headers, &org).await?;
    let rows = sqlx::query(
        "SELECT u.id, u.email, u.name, m.role, m.created_at, u.last_login_at FROM memberships m JOIN users u ON u.id = m.user_id
         WHERE m.org_id = $1 ORDER BY m.created_at ASC",
    )
    .bind(&org).fetch_all(&s.db).await?;
    let mut members = Vec::new();
    for r in &rows {
        members.push(json!({
            "user_id": r.try_get::<String,_>(0)?, "email": r.try_get::<String,_>(1)?, "name": r.try_get::<String,_>(2)?,
            "role": r.try_get::<String,_>(3)?, "joined_at": r.try_get::<i64,_>(4)?, "last_login_at": r.try_get::<Option<i64>,_>(5)?,
        }));
    }
    let inv = sqlx::query("SELECT id, email, role, created_at, expires_at FROM invites WHERE org_id = $1 AND accepted_at IS NULL AND expires_at > $2 ORDER BY created_at DESC")
        .bind(&org).bind(now_ms()).fetch_all(&s.db).await?;
    let mut invites = Vec::new();
    for r in &inv {
        invites.push(json!({
            "id": r.try_get::<String,_>(0)?, "email": r.try_get::<String,_>(1)?, "role": r.try_get::<String,_>(2)?,
            "created_at": r.try_get::<i64,_>(3)?, "expires_at": r.try_get::<i64,_>(4)?,
        }));
    }
    Ok(Json(json!({ "members": members, "invites": invites })))
}

#[derive(Deserialize)]
pub struct InviteReq {
    pub email: String,
    pub role: String,
}

pub async fn invite(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
    Json(r): Json<InviteReq>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    m.require_admin()?;
    let role = Role::parse(&r.role).ok_or_else(|| {
        ApiError::BadRequest("role must be owner, admin, developer or auditor".into())
    })?;
    if role == Role::Owner {
        m.require_owner()?;
    }
    let email = r.email.trim().to_lowercase();
    if email.len() < 5 || email.len() > 254 || !email.contains('@') {
        return Err(ApiError::BadRequest("enter a valid email address".into()));
    }
    if sqlx::query("SELECT 1 FROM memberships m JOIN users u ON u.id = m.user_id WHERE m.org_id = $1 AND u.email = $2")
        .bind(&org).bind(&email).fetch_optional(&s.db).await?.is_some()
    {
        return Err(ApiError::Conflict("this person is already a member".into()));
    }
    // A fresh invite replaces any earlier pending one for the same address.
    sqlx::query("DELETE FROM invites WHERE org_id = $1 AND email = $2 AND accepted_at IS NULL")
        .bind(&org)
        .bind(&email)
        .execute(&s.db)
        .await?;

    let info = engine::org_info(&s.db, &org).await?;
    let plan = plans::get(&info.plan);
    let members: i64 = sqlx::query("SELECT COUNT(*) FROM memberships WHERE org_id = $1")
        .bind(&org)
        .fetch_one(&s.db)
        .await?
        .try_get(0)?;
    let pending: i64 = sqlx::query("SELECT COUNT(*) FROM invites WHERE org_id = $1 AND accepted_at IS NULL AND expires_at > $2")
        .bind(&org).bind(now_ms()).fetch_one(&s.db).await?.try_get(0)?;
    if members + pending >= plan.seats {
        return Err(ApiError::PaymentRequired(format!(
            "the {} plan includes {} seats; upgrade to add more people",
            plan.name, plan.seats
        )));
    }
    let token = crypto::random_token("hsi_");
    let id = Uuid::new_v4().to_string();
    let now = now_ms();
    sqlx::query("INSERT INTO invites (id, org_id, email, role, token_hash, invited_by, created_at, expires_at, accepted_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,NULL)")
        .bind(&id).bind(&org).bind(&email).bind(role.as_str()).bind(crypto::sha256_hex(&token)).bind(&m.user.id).bind(now).bind(now + INVITE_TTL_MS)
        .execute(&s.db).await?;
    audit::log(&s, &org, &m.actor(), "team.invited", &email, role.as_str()).await;
    Ok(Json(json!({
        "id": id, "email": email, "role": role.as_str(),
        "invite_url": format!("{}/invite/{}", s.cfg.public_url, token),
        "note": "Email delivery is not configured. Send this link to the invitee yourself; it expires in 7 days.",
    })))
}

pub async fn revoke_invite(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((org, id)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    m.require_admin()?;
    let res =
        sqlx::query("DELETE FROM invites WHERE id = $1 AND org_id = $2 AND accepted_at IS NULL")
            .bind(&id)
            .bind(&org)
            .execute(&s.db)
            .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound("invitation not found".into()));
    }
    audit::log(&s, &org, &m.actor(), "team.invite_revoked", &id, "").await;
    Ok(Json(json!({ "ok": true })))
}

async fn owner_count(s: &AppState, org: &str) -> ApiResult<i64> {
    Ok(
        sqlx::query("SELECT COUNT(*) FROM memberships WHERE org_id = $1 AND role = 'owner'")
            .bind(org)
            .fetch_one(&s.db)
            .await?
            .try_get(0)?,
    )
}

async fn target_role(s: &AppState, org: &str, uid: &str) -> ApiResult<Role> {
    let r = sqlx::query("SELECT role FROM memberships WHERE org_id = $1 AND user_id = $2")
        .bind(org)
        .bind(uid)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("member not found".into()))?;
    Role::parse(&r.try_get::<String, _>(0)?).ok_or_else(ApiError::internal)
}

#[derive(Deserialize)]
pub struct RoleReq {
    pub role: String,
}

pub async fn set_role(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((org, uid)): Path<(String, String)>,
    Json(r): Json<RoleReq>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    m.require_admin()?;
    let new_role = Role::parse(&r.role).ok_or_else(|| {
        ApiError::BadRequest("role must be owner, admin, developer or auditor".into())
    })?;
    let cur = target_role(&s, &org, &uid).await?;
    // Only owners can create or demote owners.
    if (new_role == Role::Owner || cur == Role::Owner) && m.role != Role::Owner {
        return Err(ApiError::Forbidden(
            "only an owner can change owner roles".into(),
        ));
    }
    if cur == Role::Owner && new_role != Role::Owner && owner_count(&s, &org).await? <= 1 {
        return Err(ApiError::Conflict(
            "an organization needs at least one owner".into(),
        ));
    }
    sqlx::query("UPDATE memberships SET role = $1 WHERE org_id = $2 AND user_id = $3")
        .bind(new_role.as_str())
        .bind(&org)
        .bind(&uid)
        .execute(&s.db)
        .await?;
    audit::log(
        &s,
        &org,
        &m.actor(),
        "team.role_changed",
        &uid,
        new_role.as_str(),
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}

pub async fn remove(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((org, uid)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    let leaving_self = m.user.id == uid;
    if !leaving_self {
        m.require_admin()?;
    }
    let cur = target_role(&s, &org, &uid).await?;
    if cur == Role::Owner {
        if !leaving_self {
            m.require_owner()?;
        }
        if owner_count(&s, &org).await? <= 1 {
            return Err(ApiError::Conflict(
                "an organization needs at least one owner".into(),
            ));
        }
    }
    sqlx::query("DELETE FROM memberships WHERE org_id = $1 AND user_id = $2")
        .bind(&org)
        .bind(&uid)
        .execute(&s.db)
        .await?;
    audit::log(&s, &org, &m.actor(), "team.member_removed", &uid, "").await;
    Ok(Json(json!({ "ok": true })))
}
