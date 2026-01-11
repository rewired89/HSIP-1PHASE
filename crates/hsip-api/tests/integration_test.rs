//! Integration tests for HSIP API message verification
//!
//! Tests the complete flow: Message signing → API verification

use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;

#[test]
fn test_ed25519_sign_and_verify() {
    // Generate a key pair (simulating what Python SDK does)
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = VerifyingKey::from(&signing_key);

    // Build canonical message format (matching Python SDK)
    let message_data = build_canonical_format(
        "msg_test123",
        "chase.com",
        "user@example.com",
        "transaction_alert",
        "Test transaction $50.00",
        "2024-01-11T10:00:00Z",
    );

    // Sign the message (Python SDK does this)
    let signature = signing_key.sign(&message_data);

    // Verify the signature (Rust API does this)
    use ed25519_dalek::Verifier;
    assert!(verifying_key.verify(&message_data, &signature).is_ok());
}

#[test]
fn test_message_format_consistency() {
    // Ensure the canonical format matches Python SDK exactly
    let canonical = build_canonical_format(
        "msg_123",
        "bank.com",
        "customer@example.com",
        "security_alert",
        "Suspicious login detected",
        "2024-01-11T12:00:00Z",
    );

    let expected = b"HSIP_V1|msg_123|bank.com|customer@example.com|security_alert|Suspicious login detected|2024-01-11T12:00:00Z";
    assert_eq!(canonical, expected);
}

#[test]
fn test_hex_encoding_round_trip() {
    // Test hex encoding/decoding for signature and public key
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = VerifyingKey::from(&signing_key);

    // Encode public key to hex (Python SDK does this)
    let pubkey_bytes = verifying_key.to_bytes();
    let pubkey_hex = hex::encode(pubkey_bytes);

    // Decode public key from hex (Rust API does this)
    let decoded_bytes = hex::decode(&pubkey_hex).unwrap();
    let decoded_key = VerifyingKey::from_bytes(decoded_bytes.as_slice().try_into().unwrap()).unwrap();

    assert_eq!(verifying_key.to_bytes(), decoded_key.to_bytes());
}

#[test]
fn test_invalid_signature_fails() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = VerifyingKey::from(&signing_key);

    let message = b"Test message";
    let signature = signing_key.sign(message);

    // Tamper with the message
    let tampered_message = b"Tampered message";

    // Verification should fail
    use ed25519_dalek::Verifier;
    assert!(verifying_key.verify(tampered_message, &signature).is_err());
}

#[test]
fn test_wrong_public_key_fails() {
    let signing_key1 = SigningKey::generate(&mut OsRng);
    let signing_key2 = SigningKey::generate(&mut OsRng);
    let verifying_key2 = VerifyingKey::from(&signing_key2);

    let message = b"Test message";
    let signature = signing_key1.sign(message);

    // Verification with wrong public key should fail
    use ed25519_dalek::Verifier;
    assert!(verifying_key2.verify(message, &signature).is_err());
}

// Helper function matching Python SDK's _build_sign_data
fn build_canonical_format(
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
