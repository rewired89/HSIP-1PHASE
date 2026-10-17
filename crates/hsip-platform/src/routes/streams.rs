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
    audit, auth,
    db::now_ms,
    engine,
    errors::{ApiError, ApiResult},
    plans,
    state::AppState,
};

fn slug_of(name: &str) -> String {
    let mut out = String::new();
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let t = out.trim_matches('-').to_string();
    if t.is_empty() {
        "stream".into()
    } else {
        t.chars().take(48).collect()
    }
}

pub async fn list(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
) -> ApiResult<Json<Value>> {
    auth::member(&s, &headers, &org).await?;
    let stats = sqlx::query("SELECT stream_id, COUNT(*), MAX(created_at), COUNT(anchor_id) FROM proofs WHERE org_id = $1 GROUP BY stream_id")
        .bind(&org)
        .fetch_all(&s.db)
        .await?;
    let mut map = std::collections::HashMap::new();
    for r in &stats {
        let sid: String = r.try_get(0)?;
        let cnt: i64 = r.try_get(1)?;
        let last: Option<i64> = r.try_get(2)?;
        let anchored: i64 = r.try_get(3)?;
        map.insert(sid, (cnt, last, anchored));
    }
    let rows = sqlx::query("SELECT id, name, slug, kind, description, archived, created_at FROM streams WHERE org_id = $1 ORDER BY created_at ASC")
        .bind(&org)
        .fetch_all(&s.db)
        .await?;
    let mut out = Vec::new();
    for r in &rows {
        let id: String = r.try_get(0)?;
        let (cnt, last, anchored) = map.get(&id).copied().unwrap_or((0, None, 0));
        out.push(json!({
            "id": id, "name": r.try_get::<String,_>(1)?, "slug": r.try_get::<String,_>(2)?,
            "kind": r.try_get::<String,_>(3)?, "description": r.try_get::<String,_>(4)?,
            "archived": r.try_get::<i64,_>(5)? != 0, "created_at": r.try_get::<i64,_>(6)?,
            "proof_count": cnt, "last_proof_at": last, "anchored_count": anchored,
        }));
    }
    Ok(Json(json!({ "streams": out })))
}

#[derive(Deserialize)]
pub struct CreateReq {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

pub async fn create(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
    Json(r): Json<CreateReq>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    m.require_write()?;
    let name = r.name.trim();
    if name.len() < 2 || name.len() > 80 {
        return Err(ApiError::BadRequest(
            "stream name must be 2-80 characters".into(),
        ));
    }
    if r.description.len() > 300 {
        return Err(ApiError::BadRequest(
            "description must be at most 300 characters".into(),
        ));
    }
    let info = engine::org_info(&s.db, &org).await?;
    let plan = plans::get(&info.plan);
    let count: i64 = sqlx::query(
        "SELECT COUNT(*) FROM streams WHERE org_id = $1 AND kind = 'user' AND archived = 0",
    )
    .bind(&org)
    .fetch_one(&s.db)
    .await?
    .try_get(0)?;
    if count >= plan.streams {
        return Err(ApiError::PaymentRequired(format!(
            "the {} plan allows {} streams; upgrade to add more",
            plan.name, plan.streams
        )));
    }
    let id = Uuid::new_v4().to_string();
    let base = slug_of(name);
    let mut slug = base.clone();
    for _ in 0..5 {
        if sqlx::query("SELECT 1 FROM streams WHERE org_id = $1 AND slug = $2")
            .bind(&org)
            .bind(&slug)
            .fetch_optional(&s.db)
            .await?
            .is_none()
        {
            break;
        }
        slug = format!("{base}-{}", &Uuid::new_v4().to_string()[..4]);
    }
    sqlx::query("INSERT INTO streams (id, org_id, name, slug, kind, description, archived, created_at) VALUES ($1,$2,$3,$4,'user',$5,0,$6)")
        .bind(&id).bind(&org).bind(name).bind(&slug).bind(r.description.trim()).bind(now_ms())
        .execute(&s.db).await?;
    audit::log(&s, &org, &m.actor(), "stream.created", name, "").await;
    Ok(Json(json!({ "id": id, "slug": slug, "name": name })))
}

#[derive(Deserialize)]
pub struct UpdateReq {
    pub name: Option<String>,
    pub description: Option<String>,
    pub archived: Option<bool>,
}

pub async fn update(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((org, sid)): Path<(String, String)>,
    Json(r): Json<UpdateReq>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    m.require_admin()?;
    let cur = sqlx::query(
        "SELECT name, description, archived, kind FROM streams WHERE id = $1 AND org_id = $2",
    )
    .bind(&sid)
    .bind(&org)
    .fetch_optional(&s.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("stream not found".into()))?;
    if cur.try_get::<String, _>(3)? == "system" {
        return Err(ApiError::Forbidden(
            "the platform activity stream cannot be modified".into(),
        ));
    }
    let name = r
        .name
        .map(|n| n.trim().to_string())
        .unwrap_or(cur.try_get(0)?);
    if name.len() < 2 || name.len() > 80 {
        return Err(ApiError::BadRequest(
            "stream name must be 2-80 characters".into(),
        ));
    }
    let desc = r
        .description
        .map(|d| d.trim().to_string())
        .unwrap_or(cur.try_get(1)?);
    if desc.len() > 300 {
        return Err(ApiError::BadRequest(
            "description must be at most 300 characters".into(),
        ));
    }
    let archived = r.archived.map(i64::from).unwrap_or(cur.try_get(2)?);
    sqlx::query("UPDATE streams SET name = $1, description = $2, archived = $3 WHERE id = $4 AND org_id = $5")
        .bind(&name).bind(&desc).bind(archived).bind(&sid).bind(&org).execute(&s.db).await?;
    audit::log(
        &s,
        &org,
        &m.actor(),
        "stream.updated",
        &name,
        if archived != 0 { "archived" } else { "" },
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}
