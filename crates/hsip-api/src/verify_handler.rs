//! Message Verification Handler with Ed25519 signature verification

use crate::{error::{ApiError, ApiResult}, models::*};
use axum::{extract::State, Json};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use std::sync::Arc;

use super::handlers::ApiState;

/// Verify a signed message from the Python SDK
pub async fn verify_signed_message(
    State(_state): State<Arc<ApiState>>,
    Json(req): Json<VerifyMessageRequest>,
) -> ApiResult<Json<VerifyMessageResponse>> {
    // Parse the message data
    let message_data: SignedMessageData = serde_json::from_str(&req.message_data)
        .map_err(|e| ApiError::ValidationError(format!("Invalid message format: {}", e)))?;

    // Verify the signature
    match verify_ed25519_signature(&message_data) {
        Ok(()) => Ok(Json(VerifyMessageResponse {
            verified: true,
            sender_domain: message_data.sender_domain.clone(),
            message_type: message_data.message_type.clone(),
            verification_details: format!(
                "Valid Ed25519 signature from {} at {}",
                message_data.sender_domain, message_data.sent_at
            ),
        })),
        Err(e) => Ok(Json(VerifyMessageResponse {
            verified: false,
            sender_domain: message_data.sender_domain.clone(),
            message_type: message_data.message_type.clone(),
            verification_details: format!("Signature verification failed: {}", e),
        })),
    }
}

/// Data format from Python SDK's SignedMessage
#[derive(serde::Deserialize)]
struct SignedMessageData {
    message_id: String,
    sender_domain: String,
    recipient: String,
    message_type: String,
    message_text: String,
    sent_at: String,
    signature: String,      // hex-encoded
    public_key: String,     // hex-encoded
}

/// Verify Ed25519 signature on a message
fn verify_ed25519_signature(msg: &SignedMessageData) -> Result<(), String> {
    // Decode public key from hex
    let public_key_bytes = hex::decode(&msg.public_key)
        .map_err(|e| format!("Invalid public key hex: {}", e))?;

    let verifying_key = VerifyingKey::from_bytes(
        public_key_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid public key length")?,
    )
    .map_err(|e| format!("Invalid public key: {}", e))?;

    // Decode signature from hex
    let signature_bytes = hex::decode(&msg.signature)
        .map_err(|e| format!("Invalid signature hex: {}", e))?;

    let signature = Signature::from_bytes(
        signature_bytes
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid signature length")?,
    );

    // Build the same canonical format that was signed
    let sign_data = build_canonical_sign_data(
        &msg.message_id,
        &msg.sender_domain,
        &msg.recipient,
        &msg.message_type,
        &msg.message_text,
        &msg.sent_at,
    );

    // Verify the signature
    verifying_key
        .verify(&sign_data, &signature)
        .map_err(|e| format!("Signature verification failed: {}", e))?;

    Ok(())
}

/// Build canonical signing data (must match Python SDK's _build_sign_data)
fn build_canonical_sign_data(
    message_id: &str,
    sender_domain: &str,
    recipient: &str,
    message_type: &str,
    message_text: &str,
    sent_at: &str,
) -> Vec<u8> {
    let parts = [
        "HSIP_V1",
        message_id,
        sender_domain,
        recipient,
        message_type,
        message_text,
        sent_at,
    ];
    parts.join("|").into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_format() {
        let data = build_canonical_sign_data(
            "msg_123",
            "chase.com",
            "user@example.com",
            "transaction_alert",
            "Test message",
            "2024-01-11T10:00:00Z",
        );

        let expected =
            "HSIP_V1|msg_123|chase.com|user@example.com|transaction_alert|Test message|2024-01-11T10:00:00Z";
        assert_eq!(String::from_utf8(data).unwrap(), expected);
    }
}
