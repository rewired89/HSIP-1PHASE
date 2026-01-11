//! API Error Handling

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// API error type
#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    NotFound(String),
    InternalError(String),
    ValidationError(String),
}

pub type ApiResult<T> = Result<T, ApiError>;

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::ValidationError(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
        };

        let body = Json(json!({
            "success": false,
            "error": message,
        }));

        (status, body).into_response()
    }
}

impl From<hsip_enterprise::EnterpriseError> for ApiError {
    fn from(err: hsip_enterprise::EnterpriseError) -> Self {
        ApiError::InternalError(err.to_string())
    }
}

impl From<hsip_verify::VerifyError> for ApiError {
    fn from(err: hsip_verify::VerifyError) -> Self {
        ApiError::InternalError(err.to_string())
    }
}

impl From<hsip_policy::PolicyError> for ApiError {
    fn from(err: hsip_policy::PolicyError) -> Self {
        ApiError::InternalError(err.to_string())
    }
}
