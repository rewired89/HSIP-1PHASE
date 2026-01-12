//! Authentication and Authorization Module
//!
//! Provides JWT-based authentication and role-based access control

use axum::{
    extract::{Request, FromRequestParts},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID or client ID)
    pub sub: String,
    /// Issuer
    pub iss: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Audience (API identifier)
    pub aud: String,
    /// Roles for RBAC
    pub roles: Vec<String>,
    /// Scopes for fine-grained permissions
    pub scopes: Vec<String>,
}

/// User roles for RBAC
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// System administrator (full access)
    Admin,
    /// Bank administrator (manage bank's messages)
    BankAdmin,
    /// API user (send/verify messages)
    ApiUser,
    /// Read-only auditor
    Auditor,
    /// End user (consent management only)
    EndUser,
}

impl Role {
    pub fn as_str(&self) -> &str {
        match self {
            Role::Admin => "admin",
            Role::BankAdmin => "bank_admin",
            Role::ApiUser => "api_user",
            Role::Auditor => "auditor",
            Role::EndUser => "end_user",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(Role::Admin),
            "bank_admin" => Some(Role::BankAdmin),
            "api_user" => Some(Role::ApiUser),
            "auditor" => Some(Role::Auditor),
            "end_user" => Some(Role::EndUser),
            _ => None,
        }
    }
}

/// API scopes for fine-grained permissions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    /// Send messages
    MessageSend,
    /// Verify messages
    MessageVerify,
    /// Grant consent
    ConsentGrant,
    /// Revoke consent
    ConsentRevoke,
    /// Export audit logs
    AuditExport,
    /// Read audit logs
    AuditRead,
}

impl Scope {
    pub fn as_str(&self) -> &str {
        match self {
            Scope::MessageSend => "message:send",
            Scope::MessageVerify => "message:verify",
            Scope::ConsentGrant => "consent:grant",
            Scope::ConsentRevoke => "consent:revoke",
            Scope::AuditExport => "audit:export",
            Scope::AuditRead => "audit:read",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "message:send" => Some(Scope::MessageSend),
            "message:verify" => Some(Scope::MessageVerify),
            "consent:grant" => Some(Scope::ConsentGrant),
            "consent:revoke" => Some(Scope::ConsentRevoke),
            "audit:export" => Some(Scope::AuditExport),
            "audit:read" => Some(Scope::AuditRead),
            _ => None,
        }
    }
}

/// JWT manager for token generation and validation
#[derive(Clone)]
pub struct JwtManager {
    encoding_key: Arc<EncodingKey>,
    decoding_key: Arc<DecodingKey>,
    issuer: String,
    audience: String,
}

impl JwtManager {
    /// Create a new JWT manager with a secret key
    pub fn new(secret: &str, issuer: String, audience: String) -> Self {
        Self {
            encoding_key: Arc::new(EncodingKey::from_secret(secret.as_bytes())),
            decoding_key: Arc::new(DecodingKey::from_secret(secret.as_bytes())),
            issuer,
            audience,
        }
    }

    /// Create a new JWT manager from environment
    pub fn from_env() -> Self {
        let secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "hsip-dev-secret-change-in-production".to_string());
        let issuer = std::env::var("JWT_ISSUER")
            .unwrap_or_else(|_| "hsip-api".to_string());
        let audience = std::env::var("JWT_AUDIENCE")
            .unwrap_or_else(|_| "hsip-clients".to_string());

        Self::new(&secret, issuer, audience)
    }

    /// Generate a new JWT token
    pub fn generate_token(
        &self,
        subject: &str,
        roles: Vec<String>,
        scopes: Vec<String>,
        expires_in_hours: i64,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let now = Utc::now();
        let exp = now + Duration::hours(expires_in_hours);

        let claims = Claims {
            sub: subject.to_string(),
            iss: self.issuer.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            aud: self.audience.clone(),
            roles,
            scopes,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
    }

    /// Validate and decode a JWT token
    pub fn validate_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let mut validation = Validation::default();
        validation.set_audience(&[&self.audience]);
        validation.set_issuer(&[&self.issuer]);

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }

    /// Check if token has required role
    pub fn has_role(&self, claims: &Claims, required_role: &Role) -> bool {
        claims.roles.iter().any(|r| {
            Role::from_str(r).map(|role| {
                // Admin has access to everything
                if role == Role::Admin {
                    return true;
                }
                // Check exact role match
                role == *required_role
            }).unwrap_or(false)
        })
    }

    /// Check if token has required scope
    pub fn has_scope(&self, claims: &Claims, required_scope: &Scope) -> bool {
        claims.scopes.iter().any(|s| {
            Scope::from_str(s).map(|scope| scope == *required_scope).unwrap_or(false)
        })
    }
}

/// Authenticated user context extracted from JWT
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub claims: Claims,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        // Check Bearer token format
        if !auth_header.starts_with("Bearer ") {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let token = &auth_header[7..]; // Skip "Bearer "

        // Get JWT manager from extensions (set by middleware)
        let jwt_manager = parts
            .extensions
            .get::<Arc<JwtManager>>()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

        // Validate token
        let claims = jwt_manager
            .validate_token(token)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        Ok(AuthContext { claims })
    }
}

/// Middleware to inject JWT manager into request extensions
pub async fn auth_middleware(
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get JWT manager (should be set by application state)
    let jwt_manager = Arc::new(JwtManager::from_env());

    // Insert into request extensions
    req.extensions_mut().insert(jwt_manager);

    Ok(next.run(req).await)
}

/// Require specific role (middleware)
pub fn require_role(role: Role) -> impl Fn(AuthContext) -> Result<(), StatusCode> + Clone {
    move |auth: AuthContext| {
        let jwt_manager = JwtManager::from_env();
        if jwt_manager.has_role(&auth.claims, &role) {
            Ok(())
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    }
}

/// Require specific scope (middleware)
pub fn require_scope(scope: Scope) -> impl Fn(AuthContext) -> Result<(), StatusCode> + Clone {
    move |auth: AuthContext| {
        let jwt_manager = JwtManager::from_env();
        if jwt_manager.has_scope(&auth.claims, &scope) {
            Ok(())
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_generation_and_validation() {
        let manager = JwtManager::new(
            "test-secret",
            "test-issuer".to_string(),
            "test-audience".to_string(),
        );

        let token = manager
            .generate_token(
                "user123",
                vec!["api_user".to_string()],
                vec!["message:send".to_string()],
                24,
            )
            .unwrap();

        let claims = manager.validate_token(&token).unwrap();

        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.iss, "test-issuer");
        assert_eq!(claims.aud, "test-audience");
        assert!(claims.roles.contains(&"api_user".to_string()));
        assert!(claims.scopes.contains(&"message:send".to_string()));
    }

    #[test]
    fn test_role_checking() {
        let manager = JwtManager::new(
            "test-secret",
            "test-issuer".to_string(),
            "test-audience".to_string(),
        );

        let token = manager
            .generate_token(
                "admin123",
                vec!["admin".to_string()],
                vec![],
                24,
            )
            .unwrap();

        let claims = manager.validate_token(&token).unwrap();

        // Admin should have access to everything
        assert!(manager.has_role(&claims, &Role::ApiUser));
        assert!(manager.has_role(&claims, &Role::BankAdmin));
        assert!(manager.has_role(&claims, &Role::Auditor));
    }

    #[test]
    fn test_scope_checking() {
        let manager = JwtManager::new(
            "test-secret",
            "test-issuer".to_string(),
            "test-audience".to_string(),
        );

        let token = manager
            .generate_token(
                "user123",
                vec!["api_user".to_string()],
                vec!["message:send".to_string(), "message:verify".to_string()],
                24,
            )
            .unwrap();

        let claims = manager.validate_token(&token).unwrap();

        assert!(manager.has_scope(&claims, &Scope::MessageSend));
        assert!(manager.has_scope(&claims, &Scope::MessageVerify));
        assert!(!manager.has_scope(&claims, &Scope::AuditExport));
    }

    #[test]
    fn test_expired_token() {
        let manager = JwtManager::new(
            "test-secret",
            "test-issuer".to_string(),
            "test-audience".to_string(),
        );

        // Create token that expires immediately
        let token = manager
            .generate_token(
                "user123",
                vec!["api_user".to_string()],
                vec![],
                -1, // Expired 1 hour ago
            )
            .unwrap();

        // Validation should fail
        assert!(manager.validate_token(&token).is_err());
    }
}
