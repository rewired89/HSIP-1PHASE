use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    audit, auth,
    db::{month_key, now_ms},
    engine,
    errors::{ApiError, ApiResult},
    plans,
    state::AppState,
};

/// Short, human-comparable fingerprint of an organization's public key.
pub fn fingerprint(verify_key_b64: &str) -> String {
    let raw = B64.decode(verify_key_b64).unwrap_or_default();
    let h = hex::encode(Sha256::digest(raw));
    h.as_bytes()
        .chunks(4)
        .take(8)
        .map(|c| String::from_utf8_lossy(c).to_string())
        .collect::<Vec<_>>()
        .join(":")
}

async fn usage_numbers(s: &AppState, org: &str, plan: &plans::Plan) -> ApiResult<Value> {
    let month = month_key(now_ms());
    let used = engine::month_usage(&s.db, org, &month).await?;
    let streams: i64 = sqlx::query(
        "SELECT COUNT(*) FROM streams WHERE org_id = $1 AND kind = 'user' AND archived = 0",
    )
    .bind(org)
    .fetch_one(&s.db)
    .await?
    .try_get(0)?;
    let members: i64 = sqlx::query("SELECT COUNT(*) FROM memberships WHERE org_id = $1")
        .bind(org)
        .fetch_one(&s.db)
        .await?
        .try_get(0)?;
    let pending: i64 = sqlx::query("SELECT COUNT(*) FROM invites WHERE org_id = $1 AND accepted_at IS NULL AND expires_at > $2")
        .bind(org).bind(now_ms()).fetch_one(&s.db).await?.try_get(0)?;
    let hist = sqlx::query(
        "SELECT month, proofs FROM usage_monthly WHERE org_id = $1 ORDER BY month DESC LIMIT 6",
    )
    .bind(org)
    .fetch_all(&s.db)
    .await?;
    let mut history = Vec::new();
    for r in hist.iter().rev() {
        history
            .push(json!({ "month": r.try_get::<String,_>(0)?, "proofs": r.try_get::<i64,_>(1)? }));
    }
    Ok(json!({
        "plan": plan,
        "month": month,
        "proofs_used": used,
        "streams_used": streams,
        "seats_used": members + pending,
        "history": history,
    }))
}

pub async fn get(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    let r = sqlx::query("SELECT name, slug, plan, status, verify_key_b64, created_at FROM organizations WHERE id = $1")
        .bind(&org).fetch_one(&s.db).await?;
    let plan_id: String = r.try_get(2)?;
    let vk: String = r.try_get(4)?;
    Ok(Json(json!({
        "org": {
            "id": org, "name": r.try_get::<String,_>(0)?, "slug": r.try_get::<String,_>(1)?,
            "plan": plan_id, "status": r.try_get::<String,_>(3)?,
            "verify_key": vk, "fingerprint": fingerprint(&vk), "created_at": r.try_get::<i64,_>(5)?,
        },
        "role": m.role.as_str(),
        "usage": usage_numbers(&s, &org, plans::get(&plan_id)).await?,
        "anchor_verify_key": crate::state::base64_std(&s.anchor_key.verifying_key().to_bytes()),
        "ots_enabled": s.cfg.ots_enabled,
    })))
}

#[derive(Deserialize)]
pub struct RenameReq {
    pub name: String,
}

pub async fn rename(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
    Json(r): Json<RenameReq>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    m.require_admin()?;
    let name = r.name.trim();
    if name.len() < 2 || name.len() > 80 {
        return Err(ApiError::BadRequest(
            "organization name must be 2-80 characters".into(),
        ));
    }
    sqlx::query("UPDATE organizations SET name = $1 WHERE id = $2")
        .bind(name)
        .bind(&org)
        .execute(&s.db)
        .await?;
    audit::log(&s, &org, &m.actor(), "org.renamed", name, "").await;
    Ok(Json(json!({ "ok": true })))
}

pub async fn usage(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
) -> ApiResult<Json<Value>> {
    auth::member(&s, &headers, &org).await?;
    let info = engine::org_info(&s.db, &org).await?;
    Ok(Json(usage_numbers(&s, &org, plans::get(&info.plan)).await?))
}

pub async fn overview(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
) -> ApiResult<Json<Value>> {
    auth::member(&s, &headers, &org).await?;
    let now = now_ms();
    let base = "FROM proofs p JOIN streams st ON st.id = p.stream_id WHERE p.org_id = $1 AND st.kind = 'user'";

    let total: i64 = sqlx::query(&format!("SELECT COUNT(*) {base}"))
        .bind(&org)
        .fetch_one(&s.db)
        .await?
        .try_get(0)?;
    let anchored: i64 = sqlx::query(&format!(
        "SELECT COUNT(*) {base} AND p.anchor_id IS NOT NULL"
    ))
    .bind(&org)
    .fetch_one(&s.db)
    .await?
    .try_get(0)?;

    let hour = 3_600_000_i64;
    let day = 86_400_000_i64;
    let h_start = now / hour - 23;
    let d_start = now / day - 29;

    let mut hourly = vec![0_i64; 24];
    for r in sqlx::query(&format!(
        "SELECT p.created_at / {hour} AS b, COUNT(*) {base} AND p.created_at >= $2 GROUP BY b"
    ))
    .bind(&org)
    .bind(h_start * hour)
    .fetch_all(&s.db)
    .await?
    {
        let b: i64 = r.try_get(0)?;
        let i = b - h_start;
        if (0..24).contains(&i) {
            hourly[i as usize] = r.try_get(1)?;
        }
    }
    let mut daily = vec![0_i64; 30];
    for r in sqlx::query(&format!(
        "SELECT p.created_at / {day} AS b, COUNT(*) {base} AND p.created_at >= $2 GROUP BY b"
    ))
    .bind(&org)
    .bind(d_start * day)
    .fetch_all(&s.db)
    .await?
    {
        let b: i64 = r.try_get(0)?;
        let i = b - d_start;
        if (0..30).contains(&i) {
            daily[i as usize] = r.try_get(1)?;
        }
    }

    let top = |col: &'static str| {
        let sql = format!("SELECT p.{col}, COUNT(*) AS c {base} AND p.created_at >= $2 GROUP BY p.{col} ORDER BY c DESC LIMIT 6");
        let (s2, org2) = (s.clone(), org.clone());
        async move {
            let rows = sqlx::query(&sql)
                .bind(&org2)
                .bind(now - 30 * day)
                .fetch_all(&s2.db)
                .await?;
            let mut out = Vec::new();
            for r in &rows {
                out.push(
                    json!({ "name": r.try_get::<String,_>(0)?, "count": r.try_get::<i64,_>(1)? }),
                );
            }
            Ok::<_, ApiError>(out)
        }
    };
    let top_actions = top("action").await?;
    let top_actors = top("actor").await?;

    let cols = engine::PROOF_COLS
        .split(", ")
        .map(|c| format!("p.{}", c.trim()))
        .collect::<Vec<_>>()
        .join(", ");
    let recent_rows = sqlx::query(&format!(
        "SELECT {cols} {base} ORDER BY p.created_at DESC, p.seq DESC LIMIT 8"
    ))
    .bind(&org)
    .fetch_all(&s.db)
    .await?;
    let recent: Vec<engine::ProofSummary> = recent_rows
        .iter()
        .map(|r| engine::ProofRow::from_row(r).map(|p| engine::ProofSummary::from(&p)))
        .collect::<Result<_, _>>()?;

    let info = engine::org_info(&s.db, &org).await?;
    let usage = usage_numbers(&s, &org, plans::get(&info.plan)).await?;
    Ok(Json(json!({
        "total_proofs": total,
        "anchored_proofs": anchored,
        "pending_anchor": total - anchored,
        "hourly": { "start_ms": h_start * hour, "counts": hourly },
        "daily": { "start_ms": d_start * day, "counts": daily },
        "top_actions": top_actions,
        "top_actors": top_actors,
        "recent": recent,
        "usage": usage,
    })))
}

#[derive(Deserialize)]
pub struct PlanRequest {
    pub plan: String,
    #[serde(default)]
    pub message: String,
}

/// Upgrades are sales-assisted: this records the request for the operator.
pub async fn request_plan(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
    Json(r): Json<PlanRequest>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    m.require_admin()?;
    if !plans::exists(&r.plan) {
        return Err(ApiError::BadRequest("unknown plan".into()));
    }
    if r.message.len() > 1000 {
        return Err(ApiError::BadRequest(
            "message must be at most 1000 characters".into(),
        ));
    }
    let info = engine::org_info(&s.db, &org).await?;
    sqlx::query("INSERT INTO leads (id, org_id, email, company, message, kind, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(Uuid::new_v4().to_string()).bind(&org).bind(&m.user.email).bind(&info.name)
        .bind(format!("[{}] {}", r.plan, r.message.trim())).bind("plan_request").bind(now_ms())
        .execute(&s.db).await?;
    audit::log(&s, &org, &m.actor(), "billing.plan_requested", &r.plan, "").await;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct ServiceRequest {
    pub service: String,
    #[serde(default)]
    pub message: String,
}

/// Interest in an enterprise service. Recorded for the operator; nothing is
/// provisioned automatically.
pub async fn request_service(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
    Json(r): Json<ServiceRequest>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    m.require_admin()?;
    if !plans::service_exists(&r.service) {
        return Err(ApiError::BadRequest("unknown service".into()));
    }
    if r.message.len() > 1000 {
        return Err(ApiError::BadRequest(
            "message must be at most 1000 characters".into(),
        ));
    }
    let info = engine::org_info(&s.db, &org).await?;
    sqlx::query("INSERT INTO leads (id, org_id, email, company, message, kind, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(Uuid::new_v4().to_string())
        .bind(&org)
        .bind(&m.user.email)
        .bind(&info.name)
        .bind(format!("[{}] {}", r.service, r.message.trim()))
        .bind("service_request")
        .bind(now_ms())
        .execute(&s.db)
        .await?;
    audit::log(
        &s,
        &org,
        &m.actor(),
        "billing.service_requested",
        &r.service,
        "",
    )
    .await;
    Ok(Json(json!({ "ok": true })))
}
