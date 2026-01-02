//! API Request/Response Models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// API response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: Utc::now(),
        }
    }
}

/// Send message request
#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub recipient: String,
    pub message_type: String,
    pub content: serde_json::Value,
    pub expires_in_seconds: Option<u64>,
}

/// Send message response
#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageResponse {
    pub message_id: String,
    pub verification_status: String,
    pub audit_entry_id: u64,
}

/// Verify message request
#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyMessageRequest {
    pub message_data: String, // Base64-encoded signed message
}

/// Verify message response
#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyMessageResponse {
    pub verified: bool,
    pub sender_domain: String,
    pub message_type: String,
    pub verification_details: String,
}

/// Grant consent request
#[derive(Debug, Serialize, Deserialize)]
pub struct GrantConsentRequest {
    pub user_id: String,
    pub sender_domain: String,
    pub allowed_message_types: Vec<String>,
    pub duration_days: Option<u32>,
    pub max_messages_per_day: Option<u32>,
}

/// Grant consent response
#[derive(Debug, Serialize, Deserialize)]
pub struct GrantConsentResponse {
    pub policy_id: String,
    pub created_at: DateTime<Utc>,
}

/// Revoke consent request
#[derive(Debug, Serialize, Deserialize)]
pub struct RevokeConsentRequest {
    pub user_id: String,
    pub sender_domain: String,
}

/// Export audit request
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportAuditRequest {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub format: String, // "json", "csv"
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}
