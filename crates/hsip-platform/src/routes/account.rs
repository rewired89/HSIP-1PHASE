//! Sign-up, sign-in, sessions and invitation acceptance.

use axum::{extract::State, http::HeaderMap, Json};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    audit, auth, crypto,
    db::now_ms,
    errors::{ApiError, ApiResult},
    state::AppState,
};

const SESSION_TTL_MS: i64 = 14 * 24 * 3600 * 1000;

#[derive(Serialize)]
pub struct OrgSummary {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub role: String,
    pub plan: String,
}

fn valid_email(e: &str) -> bool {
    let e = e.trim();
    e.len() <= 254
        && e.len() >= 5
        && e.matches('@').count() == 1
        && !e.starts_with('@')
        && !e.ends_with('@')
        && e.contains('.')
}

fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let t = out.trim_matches('-');
    if t.is_empty() {
        "org".into()
    } else {
        t.chars().take(40).collect()
    }
}

pub async fn orgs_for_user(state: &AppState, user_id: &str) -> ApiResult<Vec<OrgSummary>> {
    let rows = sqlx::query(
        "SELECT o.id, o.name, o.slug, m.role, o.plan FROM memberships m JOIN organizations o ON o.id = m.org_id
         WHERE m.user_id = $1 ORDER BY o.created_at ASC",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;
    rows.iter()
        .map(|r| {
            Ok(OrgSummary {
                id: r.try_get(0)?,
                name: r.try_get(1)?,
                slug: r.try_get(2)?,
                role: r.try_get(3)?,
                plan: r.try_get(4)?,
            })
        })
        .collect::<Result<_, sqlx::Error>>()
        .map_err(Into::into)
}

/// Creates an organization with its signing key, owner membership and
/// system audit stream. Returns the new organization id.
pub async fn create_org(
    state: &AppState,
    user_id: &str,
    user_email: &str,
    name: &str,
) -> ApiResult<String> {
    let name = name.trim();
    if name.len() < 2 || name.len() > 80 {
        return Err(ApiError::BadRequest(
            "organization name must be 2-80 characters".into(),
        ));
    }
    let key = SigningKey::generate(&mut OsRng);
    let enc = crypto::encrypt_secret(&state.master_key, &key.to_bytes())?;
    let vk = B64.encode(key.verifying_key().to_bytes());
    let id = Uuid::new_v4().to_string();
    let base = slugify(name);
    let mut slug = base.clone();
    for _ in 0..5 {
        let taken = sqlx::query("SELECT 1 FROM organizations WHERE slug = $1")
            .bind(&slug)
            .fetch_optional(&state.db)
            .await?;
        if taken.is_none() {
            break;
        }
        slug = format!("{base}-{}", &Uuid::new_v4().to_string()[..4]);
    }
    let now = now_ms();
    let mut tx = state.db.begin().await?;
    sqlx::query("INSERT INTO organizations (id, name, slug, plan, status, signing_key_enc, verify_key_b64, created_at) VALUES ($1,$2,$3,'free','active',$4,$5,$6)")
        .bind(&id).bind(name).bind(&slug).bind(&enc).bind(&vk).bind(now)
        .execute(&mut *tx).await
        .map_err(|e| match e {
            sqlx::Error::Database(d) if d.is_unique_violation() => ApiError::Conflict("organization name is taken, try another".into()),
            other => other.into(),
        })?;
    sqlx::query(
        "INSERT INTO memberships (org_id, user_id, role, created_at) VALUES ($1,$2,'owner',$3)",
    )
    .bind(&id)
    .bind(user_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO streams (id, org_id, name, slug, kind, description, archived, created_at) VALUES ($1,$2,'Platform activity','platform-activity','system','Signed record of account, key, team and export actions. Maintained by the platform.',0,$3)")
        .bind(Uuid::new_v4().to_string()).bind(&id).bind(now).execute(&mut *tx).await?;
    tx.commit().await?;
    audit::log(
        state,
        &id,
        &format!("user:{user_email}"),
        "org.created",
        name,
        "",
    )
    .await;
    Ok(id)
}

async fn new_session(state: &AppState, user_id: &str) -> ApiResult<String> {
    let token = crypto::random_token("hss_");
    let now = now_ms();
    sqlx::query(
        "INSERT INTO sessions (token_hash, user_id, created_at, expires_at) VALUES ($1,$2,$3,$4)",
    )
    .bind(crypto::sha256_hex(&token))
    .bind(user_id)
    .bind(now)
    .bind(now + SESSION_TTL_MS)
    .execute(&state.db)
    .await?;
    Ok(token)
}

#[derive(Deserialize)]
pub struct SignupReq {
    pub email: String,
    pub password: String,
    pub name: String,
    pub org_name: String,
}

fn check_password(p: &str) -> ApiResult<()> {
    if p.len() < 10 || p.len() > 200 {
        return Err(ApiError::BadRequest(
            "password must be 10-200 characters".into(),
        ));
    }
    Ok(())
}

pub async fn signup(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(r): Json<SignupReq>,
) -> ApiResult<Json<Value>> {
    if !s.cfg.signup_open {
        return Err(ApiError::Forbidden(
            "sign-up is closed on this deployment; ask your administrator for an invitation".into(),
        ));
    }
    if !s.allow(
        &format!("signup:{}", auth::client_ip(&headers)),
        10,
        3_600_000,
    ) {
        return Err(ApiError::TooManyRequests(
            "too many sign-ups from this address, try later".into(),
        ));
    }
    let email = r.email.trim().to_lowercase();
    if !valid_email(&email) {
        return Err(ApiError::BadRequest("enter a valid email address".into()));
    }
    check_password(&r.password)?;
    let name = r.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(ApiError::BadRequest("name must be 1-100 characters".into()));
    }
    let uid = Uuid::new_v4().to_string();
    let hash = crypto::hash_password(&r.password, s.cfg.fast_password_hash)?;
    sqlx::query("INSERT INTO users (id, email, name, password_hash, created_at, last_login_at) VALUES ($1,$2,$3,$4,$5,$5)")
        .bind(&uid).bind(&email).bind(name).bind(&hash).bind(now_ms())
        .execute(&s.db).await
        .map_err(|e| match e {
            sqlx::Error::Database(d) if d.is_unique_violation() => ApiError::Conflict("an account with this email already exists, sign in instead".into()),
            other => other.into(),
        })?;
    let org_id = create_org(&s, &uid, &email, &r.org_name).await?;
    let token = new_session(&s, &uid).await?;
    Ok(Json(json!({
        "token": token,
        "user": { "id": uid, "email": email, "name": name },
        "orgs": orgs_for_user(&s, &uid).await?,
        "org_id": org_id,
    })))
}

#[derive(Deserialize)]
pub struct LoginReq {
    pub email: String,
    pub password: String,
}

pub async fn login(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(r): Json<LoginReq>,
) -> ApiResult<Json<Value>> {
    let email = r.email.trim().to_lowercase();
    let ip = auth::client_ip(&headers);
    if !s.allow(&format!("login:{email}"), 10, 15 * 60_000)
        || !s.allow(&format!("loginip:{ip}"), 60, 15 * 60_000)
    {
        return Err(ApiError::TooManyRequests(
            "too many sign-in attempts, wait a few minutes".into(),
        ));
    }
    let bad = || ApiError::Unauthorized("incorrect email or password".into());
    let row = sqlx::query("SELECT id, name, password_hash FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(bad)?;
    let hash: String = row.try_get(2)?;
    if !crypto::verify_password(&r.password, &hash) {
        return Err(bad());
    }
    let uid: String = row.try_get(0)?;
    let name: String = row.try_get(1)?;
    sqlx::query("UPDATE users SET last_login_at = $1 WHERE id = $2")
        .bind(now_ms())
        .bind(&uid)
        .execute(&s.db)
        .await?;
    let token = new_session(&s, &uid).await?;
    let orgs = orgs_for_user(&s, &uid).await?;
    for o in &orgs {
        audit::log(
            &s,
            &o.id,
            &format!("user:{email}"),
            "user.login",
            &email,
            "",
        )
        .await;
    }
    Ok(Json(
        json!({ "token": token, "user": { "id": uid, "email": email, "name": name }, "orgs": orgs }),
    ))
}

pub async fn logout(State(s): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let u = auth::session_user(&s, &headers).await?;
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(&u.token_hash)
        .execute(&s.db)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn me(State(s): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let u = auth::session_user(&s, &headers).await?;
    Ok(Json(json!({
        "user": { "id": u.id, "email": u.email, "name": u.name },
        "orgs": orgs_for_user(&s, &u.id).await?,
    })))
}

#[derive(Deserialize)]
pub struct CreateOrgReq {
    pub name: String,
}

/// An existing user starting a second organization.
pub async fn new_org(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(r): Json<CreateOrgReq>,
) -> ApiResult<Json<Value>> {
    let u = auth::session_user(&s, &headers).await?;
    if !s.cfg.signup_open {
        return Err(ApiError::Forbidden(
            "creating organizations is disabled on this deployment".into(),
        ));
    }
    if !s.allow(&format!("neworg:{}", u.id), 5, 3_600_000) {
        return Err(ApiError::TooManyRequests(
            "too many organizations created, try later".into(),
        ));
    }
    let id = create_org(&s, &u.id, &u.email, &r.name).await?;
    Ok(Json(
        json!({ "org_id": id, "orgs": orgs_for_user(&s, &u.id).await? }),
    ))
}

#[derive(Deserialize)]
pub struct AcceptReq {
    pub token: String,
    pub name: Option<String>,
    pub password: String,
}

/// Public invite lookup so the console can show "Join Acme as auditor".
pub async fn invite_preview(
    State(s): State<AppState>,
    axum::extract::Path(token): axum::extract::Path<String>,
) -> ApiResult<Json<Value>> {
    let row = sqlx::query("SELECT i.email, i.role, i.expires_at, i.accepted_at, o.name FROM invites i JOIN organizations o ON o.id = i.org_id WHERE i.token_hash = $1")
        .bind(crypto::sha256_hex(&token))
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("invitation not found".into()))?;
    let exp: i64 = row.try_get(2)?;
    let accepted: Option<i64> = row.try_get(3)?;
    if accepted.is_some() || exp < now_ms() {
        return Err(ApiError::NotFound("invitation is no longer valid".into()));
    }
    let email: String = row.try_get(0)?;
    let exists = sqlx::query("SELECT 1 FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&s.db)
        .await?
        .is_some();
    Ok(Json(
        json!({ "email": email, "role": row.try_get::<String, _>(1)?, "org_name": row.try_get::<String, _>(4)?, "has_account": exists }),
    ))
}

pub async fn accept_invite(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(r): Json<AcceptReq>,
) -> ApiResult<Json<Value>> {
    if !s.allow(
        &format!("accept:{}", auth::client_ip(&headers)),
        20,
        3_600_000,
    ) {
        return Err(ApiError::TooManyRequests(
            "too many attempts, try later".into(),
        ));
    }
    let th = crypto::sha256_hex(&r.token);
    let inv = sqlx::query("SELECT id, org_id, email, role, expires_at, accepted_at FROM invites WHERE token_hash = $1")
        .bind(&th)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("invitation not found".into()))?;
    let (inv_id, org_id, email, role): (String, String, String, String) = (
        inv.try_get(0)?,
        inv.try_get(1)?,
        inv.try_get(2)?,
        inv.try_get(3)?,
    );
    let exp: i64 = inv.try_get(4)?;
    let accepted: Option<i64> = inv.try_get(5)?;
    if accepted.is_some() || exp < now_ms() {
        return Err(ApiError::NotFound("invitation is no longer valid".into()));
    }

    let existing = sqlx::query("SELECT id, name, password_hash FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&s.db)
        .await?;
    let (uid, name) = match existing {
        Some(u) => {
            let hash: String = u.try_get(2)?;
            if !crypto::verify_password(&r.password, &hash) {
                return Err(ApiError::Unauthorized(
                    "incorrect password for the existing account".into(),
                ));
            }
            (u.try_get::<String, _>(0)?, u.try_get::<String, _>(1)?)
        }
        None => {
            check_password(&r.password)?;
            let name = r.name.clone().unwrap_or_default();
            let name = name.trim();
            if name.is_empty() || name.len() > 100 {
                return Err(ApiError::BadRequest(
                    "name is required to create your account".into(),
                ));
            }
            let uid = Uuid::new_v4().to_string();
            let hash = crypto::hash_password(&r.password, s.cfg.fast_password_hash)?;
            sqlx::query("INSERT INTO users (id, email, name, password_hash, created_at, last_login_at) VALUES ($1,$2,$3,$4,$5,$5)")
                .bind(&uid).bind(&email).bind(name).bind(&hash).bind(now_ms()).execute(&s.db).await?;
            (uid, name.to_string())
        }
    };
    let now = now_ms();
    // Atomic claim: only one concurrent accept can flip accepted_at from NULL.
    let claimed =
        sqlx::query("UPDATE invites SET accepted_at = $1 WHERE id = $2 AND accepted_at IS NULL")
            .bind(now)
            .bind(&inv_id)
            .execute(&s.db)
            .await?;
    if claimed.rows_affected() == 0 {
        return Err(ApiError::NotFound("invitation is no longer valid".into()));
    }
    sqlx::query("INSERT INTO memberships (org_id, user_id, role, created_at) VALUES ($1,$2,$3,$4) ON CONFLICT (org_id, user_id) DO NOTHING")
        .bind(&org_id).bind(&uid).bind(&role).bind(now).execute(&s.db).await?;
    audit::log(
        &s,
        &org_id,
        &format!("user:{email}"),
        "team.invite_accepted",
        &email,
        &role,
    )
    .await;
    let token = new_session(&s, &uid).await?;
    Ok(Json(json!({
        "token": token,
        "user": { "id": uid, "email": email, "name": name },
        "orgs": orgs_for_user(&s, &uid).await?,
        "org_id": org_id,
    })))
}
