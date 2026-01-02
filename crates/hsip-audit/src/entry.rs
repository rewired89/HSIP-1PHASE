//! Audit Entry Definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Audit event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditEventType {
    MessageSent,
    MessageReceived,
    MessageVerified,
    MessageRejected,
    ConsentGranted,
    ConsentRevoked,
    KeyRotated,
    IdentityDelegated,
}

/// Verification result (re-exported from verify crate)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationResult {
    Verified,
    Unverified,
    Rejected,
}

/// Single audit log entry (hash-chained)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub sequence_number: u64,
    pub previous_hash: [u8; 32],
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub message_hash: [u8; 32],
    pub sender_identity: String,
    pub recipient_identity: String,
    pub consent_proof_id: Option<[u8; 32]>,
    pub verification_result: Option<VerificationResult>,
    pub metadata: serde_json::Value,
}

impl AuditEntry {
    /// Create a new audit entry
    pub fn new(
        sequence_number: u64,
        previous_hash: [u8; 32],
        event_type: AuditEventType,
        message_hash: [u8; 32],
        sender_identity: String,
        recipient_identity: String,
    ) -> Self {
        Self {
            sequence_number,
            previous_hash,
            timestamp: Utc::now(),
            event_type,
            message_hash,
            sender_identity,
            recipient_identity,
            consent_proof_id: None,
            verification_result: None,
            metadata: serde_json::Value::Null,
        }
    }

    /// Compute hash of this entry (for chain linking)
    pub fn compute_hash(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();

        hasher.update(&self.sequence_number.to_be_bytes());
        hasher.update(&self.previous_hash);
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(&(self.event_type as u8).to_be_bytes());
        hasher.update(&self.message_hash);
        hasher.update(self.sender_identity.as_bytes());
        hasher.update(self.recipient_identity.as_bytes());

        if let Some(proof_id) = &self.consent_proof_id {
            hasher.update(proof_id);
        }

        *hasher.finalize().as_bytes()
    }

    /// Sign this entry
    pub fn sign(&self, signing_key: &ed25519_dalek::SigningKey) -> [u8; 64] {
        use ed25519_dalek::Signer;

        let hash = self.compute_hash();
        let signature = signing_key.sign(&hash);
        signature.to_bytes()
    }

    /// Verify entry signature
    pub fn verify_signature(
        &self,
        public_key: &[u8; 32],
        signature: &[u8; 64],
    ) -> Result<(), crate::Error> {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        let verifying_key = VerifyingKey::from_bytes(public_key)
            .map_err(|e| crate::Error::InvalidEntry(e.to_string()))?;

        let hash = self.compute_hash();
        let sig = Signature::from_bytes(signature);

        verifying_key.verify(&hash, &sig)
            .map_err(|e| crate::Error::InvalidEntry(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_hashing() {
        let entry = AuditEntry::new(
            1,
            [0u8; 32],
            AuditEventType::MessageSent,
            [1u8; 32],
            "bank.example.com".to_string(),
            "user@example.com".to_string(),
        );

        let hash1 = entry.compute_hash();
        let hash2 = entry.compute_hash();

        // Hash should be deterministic
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_entry_signing() {
        use ed25519_dalek::SigningKey;
        use rand::rngs::OsRng;

        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: ed25519_dalek::VerifyingKey = (&signing_key).into();

        let entry = AuditEntry::new(
            1,
            [0u8; 32],
            AuditEventType::MessageSent,
            [1u8; 32],
            "bank.example.com".to_string(),
            "user@example.com".to_string(),
        );

        let signature = entry.sign(&signing_key);
        let result = entry.verify_signature(&verifying_key.to_bytes(), &signature);

        assert!(result.is_ok());
    }
}
