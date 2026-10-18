//! Unauthenticated endpoints: health, metadata, independent verification,
//! share-link viewing, published organization keys and the sales form.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use hsip_proof::{self as hp, EvidencePack, ProofBundle};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth, crypto,
    db::now_ms,
    engine::{self, AnchorCache},
    errors::{ApiError, ApiResult},
    plans,
    state::{base64_std, AppState},
};

pub async fn health(State(s): State<AppState>) -> ApiResult<Json<Value>> {
    sqlx::query("SELECT 1").fetch_one(&s.db).await?;
    Ok(Json(
        json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }),
    ))
}

pub async fn meta(State(s): State<AppState>) -> Json<Value> {
    Json(json!({
        "name": "HSIP",
        "version": env!("CARGO_PKG_VERSION"),
        "signup_open": s.cfg.signup_open,
        "ots_enabled": s.cfg.ots_enabled,
        "anchor_verify_key": base64_std(&s.anchor_key.verifying_key().to_bytes()),
        "proof_format": hp::BUNDLE_FORMAT,
        "plans": plans::PLANS,
        "services": plans::SERVICES,
    }))
}

/// `POST /v1/verify`: a pure function of the request body. Accepts a proof
/// bundle or an evidence pack. No account, no database.
pub async fn verify(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    if !s.allow(
        &format!("verify:{}", auth::client_ip(&headers)),
        120,
        60_000,
    ) {
        return Err(ApiError::TooManyRequests(
            "too many verification requests, slow down".into(),
        ));
    }
    // Accept either the bundle itself or the console's {"bundle": ...} wrapper.
    let body = match body.get("bundle") {
        Some(inner) if body.get("format").is_none() => inner.clone(),
        _ => body,
    };
    let format = body.get("format").and_then(Value::as_str).unwrap_or("");
    if format == hp::PACK_FORMAT {
        let pack: EvidencePack = serde_json::from_value(body)
            .map_err(|e| ApiError::BadRequest(format!("malformed evidence pack: {e}")))?;
        let v = hp::verify_pack(&pack);
        return Ok(Json(
            json!({ "kind": "evidence_pack", "valid": v.valid, "result": v }),
        ));
    }
    let bundle: ProofBundle = serde_json::from_value(body)
        .map_err(|e| ApiError::BadRequest(format!("malformed proof bundle: {e}")))?;
    let v = hp::verify_bundle(&bundle);
    Ok(Json(
        json!({ "kind": "proof", "valid": v.valid, "result": v, "org_name": bundle.org_name }),
    ))
}

#[derive(Deserialize)]
pub struct MatchReq {
    pub bundle: ProofBundle,
    /// The original record, as text. Hashed here only to compare; not stored.
    pub content: String,
}

/// Checks that a bundle's `payload_hash` is the SHA-256 of supplied content.
pub async fn match_content(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(r): Json<MatchReq>,
) -> ApiResult<Json<Value>> {
    if !s.allow(
        &format!("verify:{}", auth::client_ip(&headers)),
        120,
        60_000,
    ) {
        return Err(ApiError::TooManyRequests(
            "too many verification requests, slow down".into(),
        ));
    }
    Ok(Json(
        json!({ "matches": hp::payload_matches(&r.bundle, r.content.as_bytes()) }),
    ))
}

/// Public share link: the proof bundle, freshly verified.
pub async fn view_share(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> ApiResult<Json<Value>> {
    if !s.allow(&format!("share:{}", auth::client_ip(&headers)), 120, 60_000) {
        return Err(ApiError::TooManyRequests(
            "too many requests, slow down".into(),
        ));
    }
    let gone = || ApiError::NotFound("this link is invalid, expired or has been revoked".into());
    let sh = sqlx::query(
        "SELECT id, org_id, proof_id, expires_at, revoked_at FROM shares WHERE token_hash = $1",
    )
    .bind(crypto::sha256_hex(&token))
    .fetch_optional(&s.db)
    .await?
    .ok_or_else(gone)?;
    let revoked: Option<i64> = sh.try_get(4)?;
    let exp: Option<i64> = sh.try_get(3)?;
    if revoked.is_some() || exp.is_some_and(|e| e < now_ms()) {
        return Err(gone());
    }
    let (sid, org, pid): (String, String, String) =
        (sh.try_get(0)?, sh.try_get(1)?, sh.try_get(2)?);
    sqlx::query("UPDATE shares SET views = views + 1 WHERE id = $1")
        .bind(&sid)
        .execute(&s.db)
        .await?;
    let row = engine::fetch_proof(&s.db, &org, &pid).await?;
    let info = engine::org_info(&s.db, &org).await?;
    let bundle = engine::bundle_for(&s.db, &row, &info, &mut AnchorCache::default()).await?;
    let v = hp::verify_bundle(&bundle);
    Ok(Json(json!({
        "bundle": bundle,
        "verification": v,
        "org_name": info.name,
        "org_fingerprint": super::orgs::fingerprint(&info.verify_key),
    })))
}

/// Published organization key, so a recipient can compare it with the key
/// inside a bundle through a second channel.
pub async fn org_key(
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> ApiResult<Json<Value>> {
    let r =
        sqlx::query("SELECT name, verify_key_b64, created_at FROM organizations WHERE slug = $1")
            .bind(&slug)
            .fetch_optional(&s.db)
            .await?
            .ok_or_else(|| ApiError::NotFound("organization not found".into()))?;
    let vk: String = r.try_get(1)?;
    Ok(Json(
        json!({ "name": r.try_get::<String,_>(0)?, "verify_key": vk, "fingerprint": super::orgs::fingerprint(&vk), "created_at": r.try_get::<i64,_>(2)? }),
    ))
}

#[derive(Deserialize)]
pub struct LeadReq {
    pub email: String,
    pub company: String,
    #[serde(default)]
    pub message: String,
}

pub async fn lead(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(r): Json<LeadReq>,
) -> ApiResult<Json<Value>> {
    if !s.allow(&format!("lead:{}", auth::client_ip(&headers)), 5, 3_600_000) {
        return Err(ApiError::TooManyRequests(
            "too many requests, try later".into(),
        ));
    }
    let email = r.email.trim();
    if email.len() < 5 || email.len() > 254 || !email.contains('@') {
        return Err(ApiError::BadRequest("enter a valid email address".into()));
    }
    if r.company.trim().is_empty() || r.company.len() > 120 || r.message.len() > 2000 {
        return Err(ApiError::BadRequest(
            "company is required (max 120 characters); message max 2000".into(),
        ));
    }
    sqlx::query("INSERT INTO leads (id, org_id, email, company, message, kind, created_at) VALUES ($1,NULL,$2,$3,$4,'contact',$5)")
        .bind(Uuid::new_v4().to_string()).bind(email).bind(r.company.trim()).bind(r.message.trim()).bind(now_ms())
        .execute(&s.db).await?;
    Ok(Json(json!({ "ok": true })))
}
