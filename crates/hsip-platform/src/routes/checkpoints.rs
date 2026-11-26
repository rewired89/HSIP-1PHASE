//! Signed stream checkpoints for customers to keep. See `crate::checkpoints`.

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    auth, checkpoints,
    errors::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Deserialize, Default)]
pub struct Page {
    /// Only checkpoints after this stream position, for incremental sync.
    pub after_seq: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Deserialize, Default)]
pub struct ApiPage {
    /// Stream id or slug. Optional when the API key is bound to a stream.
    pub stream: Option<String>,
    pub after_seq: Option<i64>,
    pub limit: Option<i64>,
}

fn page_json(list: Vec<hsip_proof::Checkpoint>) -> Json<Value> {
    let next = list.last().map(|c| c.body.seq.clone());
    Json(json!({ "checkpoints": list, "next_after_seq": next }))
}

/// `GET /v1/orgs/:org/streams/:sid/checkpoints` (any member).
pub async fn list_stream(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((org, sid)): Path<(String, String)>,
    Query(q): Query<Page>,
) -> ApiResult<Json<Value>> {
    auth::member(&s, &headers, &org).await?;
    sqlx::query("SELECT 1 FROM streams WHERE id = $1 AND org_id = $2")
        .bind(&sid)
        .bind(&org)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("stream not found".into()))?;
    let list = checkpoints::list(
        &s,
        &org,
        &sid,
        q.after_seq.unwrap_or(0).max(0),
        q.limit.unwrap_or(100),
    )
    .await?;
    Ok(page_json(list))
}

/// `GET /v1/checkpoints` (API key, `read` scope).
pub async fn list_api(
    State(s): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ApiPage>,
) -> ApiResult<Json<Value>> {
    let k = auth::api_key(&s, &headers).await?;
    if k.scope != "read" {
        return Err(ApiError::Forbidden(format!(
            "this API key has the `{}` scope; `read` is required",
            k.scope
        )));
    }
    let stream =
        super::proofs::resolve_stream(&s, &k.org_id, k.stream_id.as_deref(), q.stream.as_deref())
            .await?;
    let list = checkpoints::list(
        &s,
        &k.org_id,
        &stream,
        q.after_seq.unwrap_or(0).max(0),
        q.limit.unwrap_or(100),
    )
    .await?;
    Ok(page_json(list))
}
