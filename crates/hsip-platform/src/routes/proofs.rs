//! Recording and reading proofs: the machine ingest API, the console
//! listing/detail views, share links and stream integrity checks.

use std::collections::BTreeMap;

use axum::{
    extract::{Path, Query, State},
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
    engine::{self, AnchorCache, NewProof, ProofRow, ProofSummary, PROOF_COLS},
    errors::{ApiError, ApiResult},
    qb::{bind_args, Where},
    state::AppState,
};

#[derive(Deserialize)]
pub struct IngestItem {
    pub actor: String,
    pub action: String,
    #[serde(default)]
    pub subject: String,
    pub payload_hash: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Deserialize)]
pub struct IngestReq {
    /// Stream id or slug. Optional when the API key is bound to a stream.
    pub stream: Option<String>,
    #[serde(flatten)]
    pub item: IngestItem,
}

#[derive(Deserialize)]
pub struct BatchReq {
    pub stream: Option<String>,
    pub items: Vec<IngestItem>,
}

async fn resolve_stream(
    s: &AppState,
    org: &str,
    bound: Option<&str>,
    requested: Option<&str>,
) -> ApiResult<String> {
    if let Some(b) = bound {
        if let Some(r) = requested {
            if r != b {
                // Allow naming the bound stream by slug as well.
                let ok = sqlx::query(
                    "SELECT 1 FROM streams WHERE id = $1 AND org_id = $2 AND slug = $3",
                )
                .bind(b)
                .bind(org)
                .bind(r)
                .fetch_optional(&s.db)
                .await?
                .is_some();
                if !ok {
                    return Err(ApiError::Forbidden(
                        "this API key is bound to a different stream".into(),
                    ));
                }
            }
        }
        return Ok(b.to_string());
    }
    let r = requested
        .ok_or_else(|| ApiError::BadRequest("`stream` (id or slug) is required".into()))?;
    let row =
        sqlx::query("SELECT id, kind FROM streams WHERE org_id = $1 AND (id = $2 OR slug = $2)")
            .bind(org)
            .bind(r)
            .fetch_optional(&s.db)
            .await?
            .ok_or_else(|| ApiError::NotFound("stream not found".into()))?;
    if row.try_get::<String, _>(1)? == "system" {
        return Err(ApiError::Forbidden(
            "the platform activity stream is maintained by the platform".into(),
        ));
    }
    Ok(row.try_get(0)?)
}

fn require_scope(k: &auth::ApiKeyAuth, want: &str) -> ApiResult<()> {
    if k.scope == want {
        Ok(())
    } else {
        Err(ApiError::Forbidden(format!(
            "this API key has the `{}` scope; `{want}` is required",
            k.scope
        )))
    }
}

fn ingest_json(b: &hsip_proof::ProofBundle) -> Value {
    json!({
        "proof_id": b.envelope.proof_id,
        "seq": b.envelope.seq,
        "event_hash": b.event_hash,
        "signature": b.signature,
        "timestamp_iso": b.envelope.timestamp_iso,
        "anchored": false,
        "bundle": b,
    })
}

/// `POST /v1/ingest` (API key, `ingest` scope).
pub async fn ingest(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(r): Json<IngestReq>,
) -> ApiResult<Json<Value>> {
    let k = auth::api_key(&s, &headers).await?;
    require_scope(&k, "ingest")?;
    let stream = resolve_stream(&s, &k.org_id, k.stream_id.as_deref(), r.stream.as_deref()).await?;
    let b = engine::record(
        &s,
        &NewProof {
            org_id: &k.org_id,
            stream_id: &stream,
            key_id: Some(&k.key_id),
            actor: &r.item.actor,
            action: &r.item.action,
            subject: &r.item.subject,
            payload_hash: &r.item.payload_hash,
            metadata: r.item.metadata.clone(),
        },
    )
    .await?;
    Ok(Json(ingest_json(&b)))
}

/// `POST /v1/ingest/batch`: up to 100 items, recorded in order. A failure
/// stops the batch so later items never land ahead of an earlier one.
pub async fn ingest_batch(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(r): Json<BatchReq>,
) -> ApiResult<Json<Value>> {
    let k = auth::api_key(&s, &headers).await?;
    require_scope(&k, "ingest")?;
    if r.items.is_empty() || r.items.len() > 100 {
        return Err(ApiError::BadRequest(
            "items must contain 1-100 entries".into(),
        ));
    }
    let stream = resolve_stream(&s, &k.org_id, k.stream_id.as_deref(), r.stream.as_deref()).await?;
    let mut results = Vec::new();
    let mut failed = false;
    for it in &r.items {
        if failed {
            results.push(json!({ "ok": false, "error": "skipped after an earlier failure" }));
            continue;
        }
        match engine::record(&s, &NewProof {
            org_id: &k.org_id, stream_id: &stream, key_id: Some(&k.key_id),
            actor: &it.actor, action: &it.action, subject: &it.subject,
            payload_hash: &it.payload_hash, metadata: it.metadata.clone(),
        }).await {
            Ok(b) => results.push(json!({ "ok": true, "proof_id": b.envelope.proof_id, "seq": b.envelope.seq, "event_hash": b.event_hash })),
            Err(e) => {
                failed = true;
                let msg = match e {
                    ApiError::BadRequest(m) | ApiError::PaymentRequired(m) | ApiError::Conflict(m) | ApiError::Forbidden(m) | ApiError::NotFound(m) => m,
                    _ => "internal error".into(),
                };
                results.push(json!({ "ok": false, "error": msg }));
            }
        }
    }
    Ok(Json(json!({ "results": results, "all_ok": !failed })))
}

/// `GET /v1/proofs/:id` (API key, `read` scope): the full verification bundle.
pub async fn api_get(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(pid): Path<String>,
) -> ApiResult<Json<Value>> {
    let k = auth::api_key(&s, &headers).await?;
    require_scope(&k, "read")?;
    let row = engine::fetch_proof(&s.db, &k.org_id, &pid).await?;
    if let Some(bound) = &k.stream_id {
        if &row.stream_id != bound {
            return Err(ApiError::NotFound("proof not found".into()));
        }
    }
    let org = engine::org_info(&s.db, &k.org_id).await?;
    let bundle = engine::bundle_for(&s.db, &row, &org, &mut AnchorCache::default()).await?;
    let v = hsip_proof::verify_bundle(&bundle);
    Ok(Json(json!({ "bundle": bundle, "verification": v })))
}

// ------------------------------------------------------------------ console

#[derive(Deserialize)]
pub struct TestProofReq {
    pub action: String,
    #[serde(default)]
    pub subject: String,
    /// Convenience for the console's "send a test proof": the content is
    /// hashed in the browser, never sent.
    pub payload_hash: String,
    pub actor: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

pub async fn console_record(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((org, sid)): Path<(String, String)>,
    Json(r): Json<TestProofReq>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    m.require_write()?;
    let kind: String = sqlx::query("SELECT kind FROM streams WHERE id = $1 AND org_id = $2")
        .bind(&sid)
        .bind(&org)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("stream not found".into()))?
        .try_get(0)?;
    if kind == "system" {
        return Err(ApiError::Forbidden(
            "the platform activity stream is maintained by the platform".into(),
        ));
    }
    let actor = r.actor.clone().unwrap_or_else(|| m.actor());
    let b = engine::record(
        &s,
        &NewProof {
            org_id: &org,
            stream_id: &sid,
            key_id: None,
            actor: &actor,
            action: &r.action,
            subject: &r.subject,
            payload_hash: &r.payload_hash,
            metadata: r.metadata.clone(),
        },
    )
    .await?;
    Ok(Json(ingest_json(&b)))
}

#[derive(Deserialize, Default)]
pub struct ListQuery {
    pub stream_id: Option<String>,
    pub actor: Option<String>,
    pub action: Option<String>,
    pub q: Option<String>,
    pub from_ms: Option<i64>,
    pub to_ms: Option<i64>,
    pub anchored: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Value>> {
    auth::member(&s, &headers, &org).await?;
    let mut w = Where::new();
    w.eq_s("p.org_id", &org);
    match &q.stream_id {
        Some(sid) => {
            w.eq_s("p.stream_id", sid);
        }
        None => {
            w.raw("p.stream_id IN (SELECT id FROM streams WHERE kind = 'user')");
        }
    }
    if let Some(a) = q.actor.as_deref().filter(|v| !v.is_empty()) {
        w.eq_s("p.actor", a);
    }
    if let Some(a) = q.action.as_deref().filter(|v| !v.is_empty()) {
        w.eq_s("p.action", a);
    }
    if let Some(t) = q.q.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        w.contains_any(
            &["p.actor", "p.action", "p.subject", "p.payload_hash", "p.id"],
            t,
        );
    }
    if let Some(f) = q.from_ms {
        w.cmp_i("p.created_at", ">=", f);
    }
    if let Some(t) = q.to_ms {
        w.cmp_i("p.created_at", "<=", t);
    }
    match q.anchored {
        Some(true) => {
            w.raw("p.anchor_id IS NOT NULL");
        }
        Some(false) => {
            w.raw("p.anchor_id IS NULL");
        }
        None => {}
    }
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = q.offset.unwrap_or(0).max(0);

    let where_sql = w.sql();
    let total: i64 = bind_args(
        &format!("SELECT COUNT(*) FROM proofs p{where_sql}"),
        &w.args,
        &[],
    )
    .fetch_one(&s.db)
    .await?
    .try_get(0)?;
    let n = w.next_index();
    let cols = PROOF_COLS
        .split(", ")
        .map(|c| format!("p.{}", c.trim()))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("SELECT {cols} FROM proofs p{where_sql} ORDER BY p.created_at DESC, p.seq DESC LIMIT ${n} OFFSET ${}", n + 1);
    let rows = bind_args(&sql, &w.args, &[limit, offset])
        .fetch_all(&s.db)
        .await?;
    let items: Vec<ProofSummary> = rows
        .iter()
        .map(|r| ProofRow::from_row(r).map(|p| ProofSummary::from(&p)))
        .collect::<Result<_, _>>()?;
    Ok(Json(
        json!({ "items": items, "total": total, "limit": limit, "offset": offset }),
    ))
}

pub async fn get_one(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((org, pid)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    auth::member(&s, &headers, &org).await?;
    let row = engine::fetch_proof(&s.db, &org, &pid).await?;
    let info = engine::org_info(&s.db, &org).await?;
    let bundle = engine::bundle_for(&s.db, &row, &info, &mut AnchorCache::default()).await?;
    let verification = hsip_proof::verify_bundle(&bundle);
    let stream = sqlx::query("SELECT name FROM streams WHERE id = $1")
        .bind(&row.stream_id)
        .fetch_one(&s.db)
        .await?;
    Ok(Json(json!({
        "bundle": bundle,
        "verification": verification,
        "stream_name": stream.try_get::<String, _>(0)?,
        "signed_by_key_id": row.key_id,
    })))
}

#[derive(Deserialize)]
pub struct ShareReq {
    /// Days until the link stops working; omit for no expiry.
    pub expires_days: Option<i64>,
}

pub async fn create_share(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((org, pid)): Path<(String, String)>,
    Json(r): Json<ShareReq>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    let row = engine::fetch_proof(&s.db, &org, &pid).await?;
    let token = crypto::random_token("hsv_");
    let id = Uuid::new_v4().to_string();
    let now = now_ms();
    let exp = match r.expires_days {
        Some(d) if (1..=3650).contains(&d) => Some(now + d * 86_400_000),
        Some(_) => return Err(ApiError::BadRequest("expires_days must be 1-3650".into())),
        None => None,
    };
    sqlx::query("INSERT INTO shares (id, org_id, proof_id, token_hash, created_by, created_at, expires_at, revoked_at, views) VALUES ($1,$2,$3,$4,$5,$6,$7,NULL,0)")
        .bind(&id).bind(&org).bind(&row.id).bind(crypto::sha256_hex(&token)).bind(&m.user.id).bind(now).bind(exp)
        .execute(&s.db).await?;
    audit::log(&s, &org, &m.actor(), "proof.shared", &row.id, "").await;
    Ok(Json(
        json!({ "id": id, "token": token, "url": format!("{}/p/{}", s.cfg.public_url, token), "expires_at": exp }),
    ))
}

pub async fn list_shares(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
) -> ApiResult<Json<Value>> {
    auth::member(&s, &headers, &org).await?;
    let rows = sqlx::query(
        "SELECT sh.id, sh.proof_id, sh.created_at, sh.expires_at, sh.revoked_at, sh.views, p.action, p.subject
         FROM shares sh JOIN proofs p ON p.id = sh.proof_id WHERE sh.org_id = $1 ORDER BY sh.created_at DESC LIMIT 200",
    )
    .bind(&org).fetch_all(&s.db).await?;
    let mut out = Vec::new();
    for r in &rows {
        out.push(json!({
            "id": r.try_get::<String,_>(0)?, "proof_id": r.try_get::<String,_>(1)?,
            "created_at": r.try_get::<i64,_>(2)?, "expires_at": r.try_get::<Option<i64>,_>(3)?,
            "revoked_at": r.try_get::<Option<i64>,_>(4)?, "views": r.try_get::<i64,_>(5)?,
            "action": r.try_get::<String,_>(6)?, "subject": r.try_get::<String,_>(7)?,
        }));
    }
    Ok(Json(json!({ "shares": out })))
}

pub async fn revoke_share(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((org, id)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    m.require_write()?;
    let res = sqlx::query(
        "UPDATE shares SET revoked_at = $1 WHERE id = $2 AND org_id = $3 AND revoked_at IS NULL",
    )
    .bind(now_ms())
    .bind(&id)
    .bind(&org)
    .execute(&s.db)
    .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound("share link not found".into()));
    }
    audit::log(&s, &org, &m.actor(), "share.revoked", &id, "").await;
    Ok(Json(json!({ "ok": true })))
}

/// Recomputes a stream's hash chain from the database and reports the first break.
pub async fn integrity(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((org, sid)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    auth::member(&s, &headers, &org).await?;
    sqlx::query("SELECT 1 FROM streams WHERE id = $1 AND org_id = $2")
        .bind(&sid)
        .bind(&org)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("stream not found".into()))?;
    let info = engine::org_info(&s.db, &org).await?;
    let mut cache = AnchorCache::default();
    let mut after: i64 = 0;
    let mut checked: i64 = 0;
    let mut prev_hash = String::new();
    let mut expected_seq: i64 = 1;
    let mut break_at: Option<(i64, &'static str)> = None;
    'outer: loop {
        let rows = sqlx::query(&format!("SELECT {PROOF_COLS} FROM proofs WHERE stream_id = $1 AND seq > $2 ORDER BY seq ASC LIMIT 500"))
            .bind(&sid).bind(after).fetch_all(&s.db).await?;
        if rows.is_empty() {
            break;
        }
        for r in &rows {
            let p = ProofRow::from_row(r)?;
            after = p.seq;
            checked += 1;
            if p.seq != expected_seq {
                break_at = Some((p.seq, "a record is missing before this one (sequence gap)"));
                break 'outer;
            }
            if p.prev_hash != prev_hash {
                break_at = Some((p.seq, "record does not link to its predecessor"));
                break 'outer;
            }
            let bundle = engine::bundle_for(&s.db, &p, &info, &mut cache).await?;
            if !hsip_proof::verify_bundle(&bundle).valid {
                break_at = Some((p.seq, "record content, signature or anchor does not verify"));
                break 'outer;
            }
            prev_hash = p.event_hash.clone();
            expected_seq += 1;
        }
    }
    Ok(Json(json!({
        "checked": checked,
        "intact": break_at.is_none(),
        "first_break_seq": break_at.map(|b| b.0),
        "reason": break_at.map(|b| b.1),
    })))
}
