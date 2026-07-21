use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Too many requests: {0}")]
    TooManyRequests(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

// These two conversions previously embedded the raw error's `Display` text
// directly in `ApiError::Internal`, which `IntoResponse` below sends
// verbatim to the caller — real database/internal error detail (schema
// names, query fragments) leaking into an HTTP response with no debug-only
// gate. Neither `sqlx::Error` nor `anyhow::Error` here ever carry caller
// input (both come from genuine internal failures — a bad query, a broken
// invariant), so the detail is safe to log server-side but not to return.
// Deliberately hand-written `ApiError::Internal("...")` call sites
// elsewhere in this codebase are untouched — those messages were already
// authored to be safe to show a caller.

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        tracing::error!(error = %e, "internal error");
        ApiError::Internal("internal server error".into())
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => ApiError::NotFound("Record not found".into()),
            _ => {
                tracing::error!(error = %e, "database error");
                ApiError::Internal("internal server error".into())
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m.clone()),
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            ApiError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            ApiError::TooManyRequests(m) => (StatusCode::TOO_MANY_REQUESTS, m.clone()),
            ApiError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    async fn response_body_string(res: Response) -> String {
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn sqlx_error_detail_never_reaches_the_client() {
        let raw = sqlx::Error::Protocol("leaked-column-name-or-query-fragment".into());
        let api_err: ApiError = raw.into();
        let res = api_err.into_response();
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = response_body_string(res).await;
        assert!(
            !body.contains("leaked-column-name-or-query-fragment"),
            "raw sqlx error detail must not appear in the client-facing body, got: {body}"
        );
        assert!(body.contains("internal server error"));
    }

    #[tokio::test]
    async fn anyhow_error_detail_never_reaches_the_client() {
        let raw = anyhow::anyhow!("leaked-internal-invariant-detail");
        let api_err: ApiError = raw.into();
        let res = api_err.into_response();
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = response_body_string(res).await;
        assert!(
            !body.contains("leaked-internal-invariant-detail"),
            "raw anyhow error detail must not appear in the client-facing body, got: {body}"
        );
        assert!(body.contains("internal server error"));
    }

    #[tokio::test]
    async fn row_not_found_still_maps_to_a_clean_404() {
        let api_err: ApiError = sqlx::Error::RowNotFound.into();
        let res = api_err.into_response();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        let body = response_body_string(res).await;
        assert!(body.contains("Record not found"));
    }

    #[tokio::test]
    async fn hand_written_internal_messages_are_unaffected() {
        // Deliberately authored ApiError::Internal(...) call sites elsewhere
        // in this codebase are untouched by this fix — only the two `From`
        // conversions above changed.
        let api_err = ApiError::Internal("authenticated key vanished mid-request".into());
        let res = api_err.into_response();
        let body = response_body_string(res).await;
        assert!(body.contains("authenticated key vanished mid-request"));
    }
}
