use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{Response, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{auth::TenantId, db::now_ms, state::AppState};

const MAX_BYTES: usize = 8 * 1024 * 1024; // 8 MB

#[derive(Serialize)]
pub struct UploadResponse {
    pub id: String,
    pub url: String,
    pub filename: String,
    pub content_type: String,
    pub size: usize,
}

/// POST /v1/uploads — authenticated, multipart/form-data, images only, max 8 MB.
/// Returns a public URL the recipient can open directly in a browser.
pub async fn upload(
    State(state): State<AppState>,
    TenantId(tenant_id): TenantId,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, Json<serde_json::Value>)> {
    let bad = |msg: &str| -> (StatusCode, Json<serde_json::Value>) {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": msg })),
        )
    };

    let field = multipart
        .next_field()
        .await
        .map_err(|_| bad("failed to read upload"))?
        .ok_or_else(|| bad("no file field in request"))?;

    let filename = field.file_name().unwrap_or("upload").to_string();
    let content_type = field
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    if !content_type.starts_with("image/") {
        return Err(bad("only image files are accepted (jpeg, png, gif, webp, …)"));
    }

    let data = field
        .bytes()
        .await
        .map_err(|_| bad("failed to read file data"))?;

    if data.len() > MAX_BYTES {
        return Err(bad("file too large — maximum is 8 MB"));
    }

    let id = Uuid::new_v4().to_string();
    let size = data.len();
    let now = now_ms();

    sqlx::query(
        "INSERT INTO uploads (id, tenant_id, filename, content_type, data, size, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&tenant_id)
    .bind(&filename)
    .bind(&content_type)
    .bind(data.as_ref())
    .bind(size as i64)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("upload db insert error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "database error" })),
        )
    })?;

    Ok(Json(UploadResponse {
        url: format!("/v1/uploads/{}", id),
        id,
        filename,
        content_type,
        size,
    }))
}

/// GET /v1/uploads/:id — public (no auth), serves the raw image bytes.
/// The URL returned by `upload` is intentionally shareable without a token
/// so the recipient can paste it directly into a browser.
pub async fn serve(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let row = sqlx::query("SELECT content_type, data FROM uploads WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await;

    match row {
        Ok(Some(r)) => {
            let content_type: String = r.try_get("content_type").unwrap_or_default();
            let data: Vec<u8> = r.try_get("data").unwrap_or_default();
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", content_type)
                .header("cache-control", "public, max-age=31536000, immutable")
                .body(Body::from(data))
                .unwrap_or_else(|_| {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::empty())
                        .unwrap()
                })
        }
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("content-type", "text/plain")
            .body(Body::from("not found"))
            .unwrap(),
    }
}
