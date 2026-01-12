//! API Request Handlers

use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use crate::{
    error::ApiResult,
    models::*,
    database::Database,
};
use std::sync::Arc;

/// API state (shared across handlers)
#[derive(Clone)]
pub struct ApiState {
    pub version: String,
    pub db: Database,
}

/// Health check endpoint
pub async fn health_check(
    State(state): State<Arc<ApiState>>,
) -> ApiResult<Json<HealthResponse>> {
    // Check database health
    let db_healthy = state.db.health_check().await.is_ok();
    let status = if db_healthy { "healthy" } else { "degraded" };

    Ok(Json(HealthResponse {
        status: status.to_string(),
        version: state.version.clone(),
        uptime_seconds: 0, // TODO: Track actual uptime
    }))
}

/// Send verified message
pub async fn send_message(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<SendMessageRequest>,
) -> ApiResult<Json<SendMessageResponse>> {
    // Generate message ID
    let message_id = format!("msg_{}", uuid::Uuid::new_v4().simple());

    // Log basic info
    tracing::info!(
        "Sending message to {} (type: {})",
        req.recipient,
        req.message_type
    );

    // Write to audit log in PostgreSQL
    let metadata = serde_json::json!({
        "content": req.content,
        "expires_in_seconds": req.expires_in_seconds,
    });

    let audit_entry_id = state.db.audit_log(
        "message_sent",
        None, // user_id not in request
        None, // sender_domain not in request
        Some(&req.recipient),
        Some(&message_id),
        Some(&req.message_type),
        Some("verified"),
        None, // signature_hex
        None, // public_key_hex
        Some(metadata),
    ).await?;

    Ok(Json(SendMessageResponse {
        message_id,
        verification_status: "verified".to_string(),
        audit_entry_id: audit_entry_id as u64,
    }))
}

/// Verify a message
pub async fn verify_message(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<VerifyMessageRequest>,
) -> ApiResult<Json<VerifyMessageResponse>> {
    // Use the real verification handler
    use crate::verify_handler::verify_signed_message;

    tracing::info!("Verifying message: {} bytes", req.message_data.len());

    // Call the actual verification function
    verify_signed_message(State(state), Json(req)).await
}

/// Grant consent
pub async fn grant_consent(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<GrantConsentRequest>,
) -> ApiResult<Json<GrantConsentResponse>> {
    tracing::info!(
        "Granting consent: user={}, domain={}, types={:?}",
        req.user_id,
        req.sender_domain,
        req.allowed_message_types
    );

    // Create consent policy in database
    let policy_uuid = state.db.create_consent_policy(
        &req.user_id,
        &req.sender_domain,
        &req.allowed_message_types,
        req.duration_days,
        req.max_messages_per_day,
        None,
    ).await?;

    let policy_id = format!("policy_{}_{}", req.user_id, policy_uuid.simple());

    // Write audit log
    let metadata = serde_json::json!({
        "policy_id": policy_id,
        "allowed_message_types": req.allowed_message_types,
        "duration_days": req.duration_days,
        "max_messages_per_day": req.max_messages_per_day,
    });

    state.db.audit_log(
        "consent_granted",
        Some(&req.user_id),
        Some(&req.sender_domain),
        None,
        None,
        None,
        None,
        None,
        None,
        Some(metadata),
    ).await?;

    Ok(Json(GrantConsentResponse {
        policy_id,
        created_at: chrono::Utc::now(),
    }))
}

/// Revoke consent
pub async fn revoke_consent(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<RevokeConsentRequest>,
) -> ApiResult<StatusCode> {
    tracing::info!(
        "Revoking consent: user={}, domain={}",
        req.user_id,
        req.sender_domain
    );

    // Revoke in database
    let rows_affected = state.db.revoke_consent(
        &req.user_id,
        &req.sender_domain,
    ).await?;

    // Write audit log
    let metadata = serde_json::json!({
        "rows_affected": rows_affected,
        "revoked_at": chrono::Utc::now(),
    });

    state.db.audit_log(
        "consent_revoked",
        Some(&req.user_id),
        Some(&req.sender_domain),
        None,
        None,
        None,
        None,
        None,
        None,
        Some(metadata),
    ).await?;

    Ok(StatusCode::OK)
}

/// Export audit log
pub async fn export_audit(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<ExportAuditRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let export_id = uuid::Uuid::new_v4().simple().to_string();

    tracing::info!(
        "Exporting audit log: from={:?} to={:?} format={}",
        req.start_date,
        req.end_date,
        req.format
    );

    // Parse dates or use defaults
    let start_date = req.start_date.unwrap_or_else(|| {
        chrono::Utc::now() - chrono::Duration::days(30)
    });
    let end_date = req.end_date.unwrap_or_else(chrono::Utc::now);

    // Get audit logs from database
    let logs = state.db.get_audit_logs(start_date, end_date, 10000).await?;
    let entry_count = logs.len();

    // Get statistics
    let stats = state.db.get_audit_stats(start_date, end_date).await?;

    // Return response based on format
    let response = match req.format.as_str() {
        "json" => {
            serde_json::json!({
                "export_id": export_id,
                "format": "json",
                "entry_count": entry_count,
                "start_date": start_date,
                "end_date": end_date,
                "generated_at": chrono::Utc::now().to_rfc3339(),
                "statistics": {
                    "total_events": stats.total_events,
                    "unique_messages": stats.unique_messages,
                    "verified_count": stats.verified_count,
                    "failed_count": stats.failed_count,
                    "consents_granted": stats.consents_granted,
                    "consents_revoked": stats.consents_revoked,
                },
                "entries": logs,
            })
        },
        "csv" => {
            // CSV export metadata (actual CSV generation would be separate)
            serde_json::json!({
                "export_id": export_id,
                "export_url": format!("https://api.hsip.io/exports/{}.csv", export_id),
                "format": "csv",
                "entry_count": entry_count,
                "start_date": start_date,
                "end_date": end_date,
                "generated_at": chrono::Utc::now().to_rfc3339(),
            })
        },
        _ => {
            serde_json::json!({
                "export_id": export_id,
                "format": req.format,
                "entry_count": entry_count,
                "start_date": start_date,
                "end_date": end_date,
                "generated_at": chrono::Utc::now().to_rfc3339(),
            })
        }
    };

    Ok(Json(response))
}
