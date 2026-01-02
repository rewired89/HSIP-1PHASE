//! Cryptographic Consent Proofs
//!
//! Non-repudiable proof that user granted/denied consent for a specific message

use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, SigningKey, Signer, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::{PolicyError, Result};

/// Consent decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsentDecision {
    Allow,
    Deny,
}

/// Cryptographic proof of consent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentProof {
    pub proof_id: [u8; 32],
    pub user_public_key: [u8; 32],
    pub policy_id: String,
    pub decision: ConsentDecision,
    pub message_hash: [u8; 32],
    pub timestamp: DateTime<Utc>,
    #[serde(with = "serde_big_array::BigArray")]
    pub signature: [u8; 64],
}

impl ConsentProof {
    /// Create a new consent proof
    pub fn create(
        user_signing_key: &SigningKey,
        policy_id: String,
        decision: ConsentDecision,
        message_hash: [u8; 32],
    ) -> Result<Self> {
        let user_verifying_key: VerifyingKey = user_signing_key.into();
        let user_public_key = user_verifying_key.to_bytes();

        let timestamp = Utc::now();

        // Generate proof ID
        let mut hasher = blake3::Hasher::new();
        hasher.update(&user_public_key);
        hasher.update(policy_id.as_bytes());
        hasher.update(&message_hash);
        hasher.update(&timestamp.timestamp().to_be_bytes());
        let proof_id = *hasher.finalize().as_bytes();

        let mut proof = Self {
            proof_id,
            user_public_key,
            policy_id,
            decision,
            message_hash,
            timestamp,
            signature: [0u8; 64],
        };

        // Sign the proof
        let data = proof.serialize_for_signing();
        let signature = user_signing_key.sign(&data);
        proof.signature = signature.to_bytes();

        Ok(proof)
    }

    /// Serialize for signing
    fn serialize_for_signing(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"HSIP_CONSENT_PROOF_V1|");
        bytes.extend_from_slice(&self.proof_id);
        bytes.push(b'|');
        bytes.extend_from_slice(&self.user_public_key);
        bytes.push(b'|');
        bytes.extend_from_slice(self.policy_id.as_bytes());
        bytes.push(b'|');
        bytes.extend_from_slice(match self.decision {
            ConsentDecision::Allow => b"allow",
            ConsentDecision::Deny => b"deny",
        });
        bytes.push(b'|');
        bytes.extend_from_slice(&self.message_hash);
        bytes.push(b'|');
        bytes.extend_from_slice(self.timestamp.to_rfc3339().as_bytes());
        bytes
    }

    /// Verify proof signature
    pub fn verify(&self) -> Result<()> {
        let verifying_key = VerifyingKey::from_bytes(&self.user_public_key)
            .map_err(|e| PolicyError::SignatureError(e.to_string()))?;

        let data = self.serialize_for_signing();
        let signature = Signature::from_bytes(&self.signature);

        verifying_key.verify(&data, &signature)
            .map_err(|e| PolicyError::SignatureError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    #[test]
    fn test_consent_proof_creation_and_verification() {
        let user_key = SigningKey::generate(&mut OsRng);
        let message_hash = [1u8; 32];

        let proof = ConsentProof::create(
            &user_key,
            "policy_001".to_string(),
            ConsentDecision::Allow,
            message_hash,
        ).unwrap();

        assert_eq!(proof.decision, ConsentDecision::Allow);
        assert!(proof.verify().is_ok());
    }

    #[test]
    fn test_consent_proof_tamper_detection() {
        let user_key = SigningKey::generate(&mut OsRng);
        let message_hash = [1u8; 32];

        let mut proof = ConsentProof::create(
            &user_key,
            "policy_001".to_string(),
            ConsentDecision::Allow,
            message_hash,
        ).unwrap();

        // Tamper with the decision
        proof.decision = ConsentDecision::Deny;

        // Verification should fail
        assert!(proof.verify().is_err());
    }
}
