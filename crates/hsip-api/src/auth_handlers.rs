//! Authentication API Handlers

use axum::{
    extract::{Json, State},
};
use crate::{
    auth::{JwtManager, Role, Scope, AuthContext},
    error::ApiResult,
    database::Database,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// API state with JWT manager
#[derive(Clone)]
pub struct AuthState {
    pub jwt_manager: JwtManager,
    pub db: Database,
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response with JWT token
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub roles: Vec<String>,
    pub scopes: Vec<String>,
}

/// Create API key request
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub description: Option<String>,
    pub roles: Vec<String>,
    pub scopes: Vec<String>,
    pub expires_in_days: Option<i64>,
}

/// Create API key response
#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub api_key: String,
    pub key_id: String,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Token info response
#[derive(Debug, Serialize)]
pub struct TokenInfoResponse {
    pub subject: String,
    pub issuer: String,
    pub audience: String,
    pub issued_at: i64,
    pub expires_at: i64,
    pub roles: Vec<String>,
    pub scopes: Vec<String>,
}

/// Login endpoint (simplified - real implementation would check database)
pub async fn login(
    State(state): State<Arc<AuthState>>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<Json<LoginResponse>> {
    // TODO: Implement real password verification against database
    // For now, this is a placeholder with hardcoded credentials

    // Check hardcoded credentials (INSECURE - replace with database check)
    let (roles, scopes) = if req.username == "admin" && req.password == "admin" {
        (
            vec![Role::Admin.as_str().to_string()],
            vec![
                Scope::MessageSend.as_str().to_string(),
                Scope::MessageVerify.as_str().to_string(),
                Scope::ConsentGrant.as_str().to_string(),
                Scope::ConsentRevoke.as_str().to_string(),
                Scope::AuditExport.as_str().to_string(),
                Scope::AuditRead.as_str().to_string(),
            ],
        )
    } else if req.username == "bank" && req.password == "bank" {
        (
            vec![Role::BankAdmin.as_str().to_string()],
            vec![
                Scope::MessageSend.as_str().to_string(),
                Scope::MessageVerify.as_str().to_string(),
            ],
        )
    } else {
        return Err(crate::error::ApiError::Unauthorized(
            "Invalid credentials".to_string(),
        ));
    };

    // Generate JWT token (24 hour expiry)
    let token = state
        .jwt_manager
        .generate_token(&req.username, roles.clone(), scopes.clone(), 24)
        .map_err(|e| crate::error::ApiError::InternalError(e.to_string()))?;

    // Write audit log
    let metadata = serde_json::json!({
        "username": req.username,
        "roles": roles,
        "scopes": scopes,
    });

    state.db.audit_log(
        "user_login",
        Some(&req.username),
        None, None, None, None, None, None, None,
        Some(metadata),
    ).await?;

    Ok(Json(LoginResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in: 24 * 3600, // 24 hours in seconds
        roles,
        scopes,
    }))
}

/// Create API key endpoint (requires admin role)
pub async fn create_api_key(
    auth: AuthContext,
    State(state): State<Arc<AuthState>>,
    Json(req): Json<CreateApiKeyRequest>,
) -> ApiResult<Json<CreateApiKeyResponse>> {
    // Check if user has admin role
    if !state.jwt_manager.has_role(&auth.claims, &Role::Admin) {
        return Err(crate::error::ApiError::Unauthorized(
            "Admin role required".to_string(),
        ));
    }

    // Generate API key as JWT token with long expiry
    let expires_in_hours = req.expires_in_days.unwrap_or(365) * 24;
    let api_key = state
        .jwt_manager
        .generate_token(
            &format!("apikey_{}", uuid::Uuid::new_v4().simple()),
            req.roles.clone(),
            req.scopes.clone(),
            expires_in_hours,
        )
        .map_err(|e| crate::error::ApiError::InternalError(e.to_string()))?;

    let key_id = uuid::Uuid::new_v4().to_string();
    let expires_at = if let Some(days) = req.expires_in_days {
        Some(chrono::Utc::now() + chrono::Duration::days(days))
    } else {
        None
    };

    // Write audit log
    let metadata = serde_json::json!({
        "key_id": key_id,
        "name": req.name,
        "description": req.description,
        "roles": req.roles,
        "scopes": req.scopes,
        "created_by": auth.claims.sub,
    });

    state.db.audit_log(
        "api_key_created",
        Some(&auth.claims.sub),
        None, None, None, None, None, None, None,
        Some(metadata),
    ).await?;

    Ok(Json(CreateApiKeyResponse {
        api_key,
        key_id,
        expires_at,
    }))
}

/// Get token info (validates current token and returns claims)
pub async fn token_info(
    auth: AuthContext,
) -> ApiResult<Json<TokenInfoResponse>> {
    Ok(Json(TokenInfoResponse {
        subject: auth.claims.sub,
        issuer: auth.claims.iss,
        audience: auth.claims.aud,
        issued_at: auth.claims.iat,
        expires_at: auth.claims.exp,
        roles: auth.claims.roles,
        scopes: auth.claims.scopes,
    }))
}

/// Refresh token endpoint
pub async fn refresh_token(
    auth: AuthContext,
    State(state): State<Arc<AuthState>>,
) -> ApiResult<Json<LoginResponse>> {
    // Generate new token with same claims but new expiry
    let token = state
        .jwt_manager
        .generate_token(
            &auth.claims.sub,
            auth.claims.roles.clone(),
            auth.claims.scopes.clone(),
            24, // 24 hours
        )
        .map_err(|e| crate::error::ApiError::InternalError(e.to_string()))?;

    Ok(Json(LoginResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in: 24 * 3600,
        roles: auth.claims.roles,
        scopes: auth.claims.scopes,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_login_success() {
        // Would require database setup
    }

    #[tokio::test]
    async fn test_login_failure() {
        // Would require database setup
    }
}
