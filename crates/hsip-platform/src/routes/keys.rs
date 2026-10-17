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
    audit, auth, crypto,
    db::now_ms,
    errors::{ApiError, ApiResult},
    state::AppState,
};

pub async fn list(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    m.require_admin()?;
    let rows = sqlx::query(
        "SELECT k.id, k.name, k.scope, k.key_prefix, k.stream_id, st.name, k.created_at, k.last_used_at, k.revoked_at
         FROM api_keys k LEFT JOIN streams st ON st.id = k.stream_id WHERE k.org_id = $1 ORDER BY k.created_at DESC",
    )
    .bind(&org).fetch_all(&s.db).await?;
    let mut out = Vec::new();
    for r in &rows {
        out.push(json!({
            "id": r.try_get::<String,_>(0)?, "name": r.try_get::<String,_>(1)?, "scope": r.try_get::<String,_>(2)?,
            "key_prefix": r.try_get::<String,_>(3)?, "stream_id": r.try_get::<Option<String>,_>(4)?,
            "stream_name": r.try_get::<Option<String>,_>(5)?, "created_at": r.try_get::<i64,_>(6)?,
            "last_used_at": r.try_get::<Option<i64>,_>(7)?, "revoked_at": r.try_get::<Option<i64>,_>(8)?,
        }));
    }
    Ok(Json(json!({ "keys": out })))
}

#[derive(Deserialize)]
pub struct CreateReq {
    pub name: String,
    /// `ingest` records proofs; `read` fetches them.
    pub scope: String,
    pub stream_id: Option<String>,
}

pub async fn create(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
    Json(r): Json<CreateReq>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    m.require_admin()?;
    let name = r.name.trim();
    if name.is_empty() || name.len() > 80 {
        return Err(ApiError::BadRequest(
            "key name must be 1-80 characters".into(),
        ));
    }
    if r.scope != "ingest" && r.scope != "read" {
        return Err(ApiError::BadRequest(
            "scope must be `ingest` or `read`".into(),
        ));
    }
    if let Some(sid) = &r.stream_id {
        let kind: Option<String> =
            sqlx::query("SELECT kind FROM streams WHERE id = $1 AND org_id = $2")
                .bind(sid)
                .bind(&org)
                .fetch_optional(&s.db)
                .await?
                .map(|x| x.try_get(0))
                .transpose()?;
        match kind.as_deref() {
            Some("user") => {}
            _ => return Err(ApiError::BadRequest("stream not found".into())),
        }
    }
    let token = crypto::random_token("hsp_");
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO api_keys (id, org_id, stream_id, name, scope, key_prefix, key_hash, created_by, created_at, last_used_at, revoked_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,NULL,NULL)")
        .bind(&id).bind(&org).bind(r.stream_id.clone()).bind(name).bind(&r.scope)
        .bind(&token[..12]).bind(crypto::sha256_hex(&token)).bind(&m.user.id).bind(now_ms())
        .execute(&s.db).await?;
    audit::log(&s, &org, &m.actor(), "apikey.created", name, &r.scope).await;
    Ok(Json(
        json!({ "id": id, "token": token, "name": name, "scope": r.scope, "note": "Copy this key now. It is shown once and stored only as a hash." }),
    ))
}

pub async fn revoke(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((org, id)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    m.require_admin()?;
    let res = sqlx::query(
        "UPDATE api_keys SET revoked_at = $1 WHERE id = $2 AND org_id = $3 AND revoked_at IS NULL",
    )
    .bind(now_ms())
    .bind(&id)
    .bind(&org)
    .execute(&s.db)
    .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound("API key not found".into()));
    }
    audit::log(&s, &org, &m.actor(), "apikey.revoked", &id, "").await;
    Ok(Json(json!({ "ok": true })))
}
