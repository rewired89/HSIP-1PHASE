//! Platform-operator console API (the company running HSIP, not a
//! customer). Gated by `HSP_OPERATOR_KEY`; 404 when that is not configured.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;

use crate::{
    auth,
    db::{month_key, now_ms},
    errors::{ApiError, ApiResult},
    plans,
    state::AppState,
};

pub async fn overview(State(s): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    auth::operator(&s, &headers)?;
    let month = month_key(now_ms());
    let rows = sqlx::query(
        "SELECT o.id, o.name, o.slug, o.plan, o.status, o.created_at,
                (SELECT COUNT(*) FROM memberships m WHERE m.org_id = o.id),
                (SELECT COUNT(*) FROM streams st WHERE st.org_id = o.id AND st.kind = 'user'),
                (SELECT COUNT(*) FROM proofs p WHERE p.org_id = o.id),
                (SELECT u.email FROM memberships m JOIN users u ON u.id = m.user_id
                   WHERE m.org_id = o.id AND m.role = 'owner' ORDER BY m.created_at ASC LIMIT 1),
                (SELECT MAX(p.created_at) FROM proofs p
                   WHERE p.org_id = o.id AND p.stream_id IN (SELECT id FROM streams WHERE kind = 'user'))
         FROM organizations o ORDER BY o.created_at DESC LIMIT 500",
    )
    .fetch_all(&s.db)
    .await?;
    let mut orgs = Vec::new();
    for r in &rows {
        let id: String = r.try_get(0)?;
        let used: i64 =
            sqlx::query("SELECT proofs FROM usage_monthly WHERE org_id = $1 AND month = $2")
                .bind(&id)
                .bind(&month)
                .fetch_optional(&s.db)
                .await?
                .map(|x| x.try_get(0))
                .transpose()?
                .unwrap_or(0);
        orgs.push(json!({
            "id": id, "name": r.try_get::<String,_>(1)?, "slug": r.try_get::<String,_>(2)?, "plan": r.try_get::<String,_>(3)?,
            "status": r.try_get::<String,_>(4)?, "created_at": r.try_get::<i64,_>(5)?, "members": r.try_get::<i64,_>(6)?,
            "streams": r.try_get::<i64,_>(7)?, "proofs_total": r.try_get::<i64,_>(8)?, "proofs_month": used,
            "owner_email": r.try_get::<Option<String>,_>(9)?, "last_proof_at": r.try_get::<Option<i64>,_>(10)?,
        }));
    }
    let users: i64 = sqlx::query("SELECT COUNT(*) FROM users")
        .fetch_one(&s.db)
        .await?
        .try_get(0)?;
    let proofs: i64 = sqlx::query("SELECT COUNT(*) FROM proofs")
        .fetch_one(&s.db)
        .await?
        .try_get(0)?;
    let unanchored: i64 = sqlx::query("SELECT COUNT(*) FROM proofs WHERE anchor_id IS NULL")
        .fetch_one(&s.db)
        .await?
        .try_get(0)?;
    let anchors: i64 = sqlx::query("SELECT COUNT(*) FROM anchors")
        .fetch_one(&s.db)
        .await?
        .try_get(0)?;
    let pending_ots: i64 = sqlx::query("SELECT COUNT(*) FROM anchors WHERE ots_status = 'pending'")
        .fetch_one(&s.db)
        .await?
        .try_get(0)?;
    let unreachable_ots: i64 =
        sqlx::query("SELECT COUNT(*) FROM anchors WHERE ots_status = 'calendar_unreachable'")
            .fetch_one(&s.db)
            .await?
            .try_get(0)?;
    let leads: i64 = sqlx::query("SELECT COUNT(*) FROM leads")
        .fetch_one(&s.db)
        .await?
        .try_get(0)?;
    Ok(Json(json!({
        "orgs": orgs,
        "totals": { "orgs": orgs.len(), "users": users, "proofs": proofs, "unanchored": unanchored, "anchors": anchors,
                    "ots_pending": pending_ots, "ots_unreachable": unreachable_ots, "leads": leads },
        "ots_enabled": s.cfg.ots_enabled,
    })))
}

#[derive(Deserialize)]
pub struct PlanReq {
    pub plan: String,
}

pub async fn set_plan(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(r): Json<PlanReq>,
) -> ApiResult<Json<Value>> {
    auth::operator(&s, &headers)?;
    if !plans::exists(&r.plan) {
        return Err(ApiError::BadRequest("unknown plan".into()));
    }
    let res = sqlx::query("UPDATE organizations SET plan = $1 WHERE id = $2")
        .bind(&r.plan)
        .bind(&id)
        .execute(&s.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound("organization not found".into()));
    }
    crate::audit::log(&s, &id, "operator", "billing.plan_changed", &r.plan, "").await;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct StatusReq {
    pub status: String,
}

pub async fn set_status(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(r): Json<StatusReq>,
) -> ApiResult<Json<Value>> {
    auth::operator(&s, &headers)?;
    if r.status != "active" && r.status != "suspended" {
        return Err(ApiError::BadRequest(
            "status must be `active` or `suspended`".into(),
        ));
    }
    let res = sqlx::query("UPDATE organizations SET status = $1 WHERE id = $2")
        .bind(&r.status)
        .bind(&id)
        .execute(&s.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound("organization not found".into()));
    }
    crate::audit::log(&s, &id, "operator", "org.status_changed", &r.status, "").await;
    Ok(Json(json!({ "ok": true })))
}

pub async fn leads(State(s): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    auth::operator(&s, &headers)?;
    let rows = sqlx::query("SELECT id, org_id, email, company, message, kind, created_at FROM leads ORDER BY created_at DESC LIMIT 200")
        .fetch_all(&s.db).await?;
    let mut out = Vec::new();
    for r in &rows {
        out.push(json!({
            "id": r.try_get::<String,_>(0)?, "org_id": r.try_get::<Option<String>,_>(1)?, "email": r.try_get::<String,_>(2)?,
            "company": r.try_get::<String,_>(3)?, "message": r.try_get::<String,_>(4)?, "kind": r.try_get::<String,_>(5)?,
            "created_at": r.try_get::<i64,_>(6)?,
        }));
    }
    Ok(Json(json!({ "leads": out })))
}
