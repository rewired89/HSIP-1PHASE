//! Permissions ("mandates"): store a signed statement of what an agent may
//! do, and report which recorded proofs fall outside it. The check itself is
//! `hsip_proof::evaluate_mandate`, a pure function anyone can re-run offline
//! with the mandate and the proofs; the platform only stores and supplies them.

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use hsip_proof::{self as hp, Mandate};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;

use crate::{
    audit, auth,
    db::now_ms,
    engine::{self, AnchorCache, ProofRow, PROOF_COLS},
    errors::{ApiError, ApiResult},
    plans,
    state::AppState,
};

/// `POST /v1/orgs/:org/mandates`: store a mandate signed by its grantor.
pub async fn create(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
    Json(m): Json<Mandate>,
) -> ApiResult<Json<Value>> {
    let who = auth::member(&s, &headers, &org).await?;
    who.require_write()?;
    if !hp::verify_mandate(&m) {
        return Err(ApiError::BadRequest(
            "the permission's signature does not verify; sign it with `hsip-proof mandate-sign`"
                .into(),
        ));
    }
    if m.body.mandate_id.is_empty() || m.body.mandate_id.len() > 80 || m.body.agent.is_empty() {
        return Err(ApiError::BadRequest(
            "mandate_id (up to 80 characters) and agent are required".into(),
        ));
    }
    let taken = sqlx::query("SELECT 1 FROM mandates WHERE org_id = $1 AND mandate_id = $2")
        .bind(&org)
        .bind(&m.body.mandate_id)
        .fetch_optional(&s.db)
        .await?
        .is_some();
    if taken {
        return Err(ApiError::Conflict(
            "a permission with that mandate_id already exists".into(),
        ));
    }
    sqlx::query("INSERT INTO mandates (org_id, mandate_id, agent, grantor_key, mandate_json, created_by, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(&org)
        .bind(&m.body.mandate_id)
        .bind(&m.body.agent)
        .bind(&m.body.grantor_key)
        .bind(serde_json::to_string(&m)?)
        .bind(&who.user.id)
        .bind(now_ms())
        .execute(&s.db)
        .await?;
    audit::log(
        &s,
        &org,
        &who.actor(),
        "mandate.created",
        &m.body.mandate_id,
        &m.body.agent,
    )
    .await;
    Ok(Json(json!({ "ok": true, "mandate_id": m.body.mandate_id })))
}

/// `GET /v1/orgs/:org/mandates`
pub async fn list(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
) -> ApiResult<Json<Value>> {
    auth::member(&s, &headers, &org).await?;
    let rows = sqlx::query("SELECT mandate_id, agent, grantor_key, mandate_json, revoked_at, created_at FROM mandates WHERE org_id = $1 ORDER BY created_at DESC LIMIT 200")
        .bind(&org)
        .fetch_all(&s.db)
        .await?;
    let mut out = Vec::new();
    for r in rows {
        let m: Value = serde_json::from_str(&r.try_get::<String, _>(3)?)?;
        out.push(json!({
            "mandate_id": r.try_get::<String,_>(0)?,
            "agent": r.try_get::<String,_>(1)?,
            "grantor_key": r.try_get::<String,_>(2)?,
            "revoked_at": r.try_get::<Option<i64>,_>(4)?,
            "created_at": r.try_get::<i64,_>(5)?,
            "mandate": m,
        }));
    }
    Ok(Json(json!({ "mandates": out })))
}

/// `POST /v1/orgs/:org/mandates/:id/revoke`: records from now on count as
/// outside the permission in this platform's report. The grantor's own signed
/// `valid_until_ms` is what an offline checker sees.
pub async fn revoke(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((org, id)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    let who = auth::member(&s, &headers, &org).await?;
    who.require_admin()?;
    let n = sqlx::query("UPDATE mandates SET revoked_at = $1 WHERE org_id = $2 AND mandate_id = $3 AND revoked_at IS NULL")
        .bind(now_ms())
        .bind(&org)
        .bind(&id)
        .execute(&s.db)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(ApiError::NotFound(
            "no active permission with that id".into(),
        ));
    }
    audit::log(&s, &org, &who.actor(), "mandate.revoked", &id, "").await;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct ReportQuery {
    pub stream_id: String,
    pub from_ms: Option<i64>,
    pub to_ms: Option<i64>,
}

/// `GET /v1/orgs/:org/mandates/:id/report?stream_id=`: which of this agent's
/// records in the stream fall outside the permission.
pub async fn report(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((org, id)): Path<(String, String)>,
    Query(q): Query<ReportQuery>,
) -> ApiResult<Json<Value>> {
    auth::member(&s, &headers, &org).await?;
    let row = sqlx::query(
        "SELECT mandate_json, revoked_at FROM mandates WHERE org_id = $1 AND mandate_id = $2",
    )
    .bind(&org)
    .bind(&id)
    .fetch_optional(&s.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("permission not found".into()))?;
    let m: Mandate = serde_json::from_str(&row.try_get::<String, _>(0)?)?;
    let revoked_at: Option<i64> = row.try_get(1)?;

    let info = engine::org_info(&s.db, &org).await?;
    let cap = plans::get(&info.plan).evidence_max_proofs;
    let rows = sqlx::query(&format!(
        "SELECT {PROOF_COLS} FROM proofs WHERE org_id = $1 AND stream_id = $2 AND actor = $3 AND created_at >= $4 AND created_at <= $5 ORDER BY seq ASC LIMIT $6"
    ))
    .bind(&org)
    .bind(&q.stream_id)
    .bind(&m.body.agent)
    .bind(q.from_ms.unwrap_or(0).max(0))
    .bind(q.to_ms.unwrap_or_else(now_ms))
    .bind(cap + 1)
    .fetch_all(&s.db)
    .await?;
    if rows.len() as i64 > cap {
        return Err(ApiError::PaymentRequired(format!(
            "more than {cap} records match, the {} plan's limit; narrow the range or upgrade",
            plans::get(&info.plan).name
        )));
    }
    let mut cache = AnchorCache::default();
    let mut proofs = Vec::with_capacity(rows.len());
    let mut created: Vec<i64> = Vec::new();
    for r in &rows {
        let pr = ProofRow::from_row(r)?;
        created.push(pr.created_at);
        proofs.push(engine::bundle_for(&s.db, &pr, &info, &mut cache).await?);
    }
    let mut rep = hp::evaluate_mandate(&m, &proofs);
    if let Some(rv) = revoked_at {
        for (b, c) in proofs.iter().zip(&created) {
            if *c > rv {
                rep.violations.push(hp::Violation {
                    proof_id: b.envelope.proof_id.clone(),
                    seq: b.envelope.seq.clone(),
                    event_hash: b.event_hash.clone(),
                    rule: "after_revocation".into(),
                    detail: "recorded after the permission was revoked on this platform".into(),
                });
            }
        }
        rep.within_permission = rep.mandate_genuine && rep.violations.is_empty();
    }
    Ok(Json(json!({
        "mandate_id": id,
        "revoked_at": revoked_at,
        "report": rep,
        "note": "Re-run this offline: `hsip-proof mandate-check mandate.json pack.json` on the mandate and an evidence pack.",
    })))
}
