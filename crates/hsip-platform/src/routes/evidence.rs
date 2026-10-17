//! Evidence packs: a signed, independently verifiable export of one
//! stream's records over a time range. This is what gets handed to an
//! auditor, regulator or court.

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    Json,
};
use hsip_proof::{self as hp, EvidencePack, PackManifest, PACK_FORMAT};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    audit, auth,
    db::{ms_to_iso, now_ms},
    engine::{self, AnchorCache, ProofRow, PROOF_COLS},
    errors::{ApiError, ApiResult},
    plans, report,
    state::AppState,
};

#[derive(Deserialize)]
pub struct CreateReq {
    pub stream_id: String,
    pub from_ms: Option<i64>,
    pub to_ms: Option<i64>,
    pub title: Option<String>,
}

pub async fn create(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
    Json(r): Json<CreateReq>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    let stream = sqlx::query("SELECT name FROM streams WHERE id = $1 AND org_id = $2")
        .bind(&r.stream_id)
        .bind(&org)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("stream not found".into()))?;
    let stream_name: String = stream.try_get(0)?;
    let info = engine::org_info(&s.db, &org).await?;
    let cap = plans::get(&info.plan).evidence_max_proofs;
    let from = r.from_ms.unwrap_or(0).max(0);
    let to = r.to_ms.unwrap_or_else(now_ms);
    if to < from {
        return Err(ApiError::BadRequest(
            "the end of the range is before its start".into(),
        ));
    }
    let title = r.title.clone().unwrap_or_default();
    let title = title.trim();
    let title = if title.is_empty() {
        format!("{stream_name} evidence")
    } else {
        title.chars().take(120).collect()
    };

    let rows = sqlx::query(&format!(
        "SELECT {PROOF_COLS} FROM proofs WHERE stream_id = $1 AND org_id = $2 AND created_at >= $3 AND created_at <= $4 ORDER BY seq ASC LIMIT $5"
    ))
    .bind(&r.stream_id).bind(&org).bind(from).bind(to).bind(cap + 1)
    .fetch_all(&s.db).await?;
    if rows.is_empty() {
        return Err(ApiError::BadRequest("no records in that range".into()));
    }
    if rows.len() as i64 > cap {
        return Err(ApiError::PaymentRequired(format!(
            "that range holds more than {cap} records, the {} plan's evidence limit; narrow the range or upgrade",
            plans::get(&info.plan).name
        )));
    }

    let mut cache = AnchorCache::default();
    let mut proofs = Vec::with_capacity(rows.len());
    for row in &rows {
        proofs.push(engine::bundle_for(&s.db, &ProofRow::from_row(row)?, &info, &mut cache).await?);
    }
    let manifest = PackManifest {
        org_id: org.clone(),
        stream_id: r.stream_id.clone(),
        from_ms: from,
        to_ms: to,
        event_hashes: proofs.iter().map(|b| b.event_hash.clone()).collect(),
    };
    let key = s
        .org_signing_key(&org)
        .await
        .map_err(|_| ApiError::internal())?;
    let (manifest_hash, manifest_signature) = hp::sign_manifest(&manifest, &key)?;
    let generated = now_ms();
    let pack = EvidencePack {
        format: PACK_FORMAT.into(),
        org_name: info.name.clone(),
        org_verify_key: info.verify_key.clone(),
        stream_name,
        generated_at_iso: ms_to_iso(generated),
        manifest,
        manifest_hash: manifest_hash.clone(),
        manifest_signature,
        proofs,
    };
    // Never hand out a pack that does not verify against itself.
    let v = hp::verify_pack(&pack);
    if !v.valid {
        tracing::error!(org = %org, stream = %r.stream_id, checks = ?v.checks, "evidence pack failed self-verification");
        return Err(ApiError::Conflict(
            "integrity check failed while preparing this export: the stored records do not verify. Run the stream integrity check and contact support".into(),
        ));
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO evidence_exports (id, org_id, stream_id, title, from_ms, to_ms, proof_count, manifest_hash, pack_json, created_by, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)")
        .bind(&id).bind(&org).bind(&r.stream_id).bind(&title).bind(from).bind(to).bind(pack.proofs.len() as i64)
        .bind(&manifest_hash).bind(serde_json::to_string(&pack)?).bind(&m.user.id).bind(generated)
        .execute(&s.db).await?;
    audit::log(
        &s,
        &org,
        &m.actor(),
        "evidence.exported",
        &id,
        &format!("{} records", pack.proofs.len()),
    )
    .await;
    Ok(Json(
        json!({ "id": id, "title": title, "proof_count": pack.proofs.len(), "anchored_count": v.anchored_count, "manifest_hash": manifest_hash, "verification": v }),
    ))
}

pub async fn list(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
) -> ApiResult<Json<Value>> {
    auth::member(&s, &headers, &org).await?;
    let rows = sqlx::query(
        "SELECT e.id, e.title, e.stream_id, st.name, e.from_ms, e.to_ms, e.proof_count, e.manifest_hash, e.created_at, u.email
         FROM evidence_exports e LEFT JOIN streams st ON st.id = e.stream_id LEFT JOIN users u ON u.id = e.created_by
         WHERE e.org_id = $1 ORDER BY e.created_at DESC LIMIT 100",
    )
    .bind(&org).fetch_all(&s.db).await?;
    let mut out = Vec::new();
    for r in &rows {
        out.push(json!({
            "id": r.try_get::<String,_>(0)?, "title": r.try_get::<String,_>(1)?, "stream_id": r.try_get::<String,_>(2)?,
            "stream_name": r.try_get::<Option<String>,_>(3)?, "from_ms": r.try_get::<i64,_>(4)?, "to_ms": r.try_get::<i64,_>(5)?,
            "proof_count": r.try_get::<i64,_>(6)?, "manifest_hash": r.try_get::<String,_>(7)?,
            "created_at": r.try_get::<i64,_>(8)?, "created_by": r.try_get::<Option<String>,_>(9)?,
        }));
    }
    Ok(Json(json!({ "exports": out })))
}

async fn load_pack(
    s: &AppState,
    org: &str,
    id: &str,
) -> ApiResult<(String, String, i64, i64, String)> {
    let r = sqlx::query("SELECT pack_json, title, from_ms, to_ms, manifest_hash FROM evidence_exports WHERE id = $1 AND org_id = $2")
        .bind(id).bind(org).fetch_optional(&s.db).await?
        .ok_or_else(|| ApiError::NotFound("export not found".into()))?;
    Ok((
        r.try_get(0)?,
        r.try_get(1)?,
        r.try_get(2)?,
        r.try_get(3)?,
        r.try_get(4)?,
    ))
}

pub async fn pack(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((org, id)): Path<(String, String)>,
) -> ApiResult<Response> {
    let m = auth::member(&s, &headers, &org).await?;
    let (json_text, ..) = load_pack(&s, &org, &id).await?;
    audit::log(&s, &org, &m.actor(), "evidence.downloaded", &id, "").await;
    let mut resp = json_text.into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    resp.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"evidence-{id}.json\""))
            .map_err(|_| ApiError::internal())?,
    );
    Ok(resp)
}

pub async fn report_html(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((org, id)): Path<(String, String)>,
) -> ApiResult<Response> {
    auth::member(&s, &headers, &org).await?;
    let (json_text, title, from, to, _) = load_pack(&s, &org, &id).await?;
    let pack: EvidencePack = serde_json::from_str(&json_text)?;
    let v = hp::verify_pack(&pack);
    let fp = super::orgs::fingerprint(&pack.org_verify_key);
    let html = report::render(&title, &pack, &v, &fp, &ms_to_iso(from), &ms_to_iso(to));
    let mut resp = html.into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    Ok(resp)
}
