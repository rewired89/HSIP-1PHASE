//! Verification Engine
//!
//! Core verification logic for enterprise messages

use crate::{MessageMetadata, RejectionReason, Result, VerifyError};
use hsip_enterprise::{CertChain, TrustStore};
use serde::{Deserialize, Serialize};

/// Verification result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationResult {
    /// ✅ VERIFIED - Valid enterprise message
    Verified,

    /// 🔴 UNVERIFIED - Invalid or missing signature
    Unverified,

    /// ❌ REJECTED - Explicitly rejected (policy, revoked, etc.)
    Rejected(RejectionReason),
}

impl VerificationResult {
    /// Check if verified
    pub fn is_verified(&self) -> bool {
        matches!(self, Self::Verified)
    }

    /// Get user-friendly status
    pub fn status_text(&self) -> &'static str {
        match self {
            Self::Verified => "VERIFIED",
            Self::Unverified => "UNVERIFIED",
            Self::Rejected(_) => "REJECTED",
        }
    }

    /// Get UI indicator
    pub fn ui_indicator(&self) -> &'static str {
        match self {
            Self::Verified => "✅",
            Self::Unverified => "🔴",
            Self::Rejected(_) => "❌",
        }
    }
}

/// Signed enterprise message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedMessage {
    pub metadata: MessageMetadata,
    pub cert_chain: CertChain,
    #[serde(with = "serde_big_array::BigArray")]
    pub signature: [u8; 64],
}

impl SignedMessage {
    /// Serialize message for signing
    pub fn serialize_for_signing(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"HSIP_ENTERPRISE_MSG_V1|");

        // Add message metadata
        let metadata_json = serde_json::to_string(&self.metadata).unwrap();
        bytes.extend_from_slice(metadata_json.as_bytes());
        bytes.push(b'|');

        // Add cert chain
        let chain_json = serde_json::to_string(&self.cert_chain).unwrap();
        bytes.extend_from_slice(chain_json.as_bytes());

        bytes
    }

    /// Get message hash
    pub fn message_hash(&self) -> [u8; 32] {
        let bytes = self.serialize_for_signing();
        *blake3::hash(&bytes).as_bytes()
    }
}

/// Verification engine
pub struct VerificationEngine {
    trust_store: TrustStore,
}

impl VerificationEngine {
    pub fn new(trust_store: TrustStore) -> Self {
        Self { trust_store }
    }

    /// Verify a signed message
    pub fn verify(&self, signed_msg: &SignedMessage) -> VerificationResult {
        // Step 1: Check message expiration
        if signed_msg.metadata.is_expired() {
            return VerificationResult::Rejected(RejectionReason::MessageExpired);
        }

        // Step 2: Verify cert chain against trust store
        if let Err(e) = self.trust_store.verify_cert_chain(&signed_msg.cert_chain) {
            return match e {
                hsip_enterprise::EnterpriseError::KeyRevoked(_) => {
                    VerificationResult::Rejected(RejectionReason::RevokedKey)
                }
                hsip_enterprise::EnterpriseError::CertChainInvalid(_) => {
                    VerificationResult::Unverified
                }
                hsip_enterprise::EnterpriseError::IdentityNotFound(_) => {
                    VerificationResult::Unverified
                }
                _ => VerificationResult::Unverified,
            };
        }

        // Step 3: Verify signature
        let data = signed_msg.serialize_for_signing();
        if let Err(_) = self.verify_signature(&signed_msg.cert_chain, &data, &signed_msg.signature) {
            return VerificationResult::Unverified;
        }

        // Step 4: Verify sender domain matches cert chain
        if signed_msg.metadata.sender_domain != signed_msg.cert_chain.root_domain {
            return VerificationResult::Unverified;
        }

        VerificationResult::Verified
    }

    fn verify_signature(
        &self,
        chain: &CertChain,
        data: &[u8],
        signature: &[u8; 64],
    ) -> Result<()> {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        let leaf_key = chain.leaf_public_key()
            .ok_or_else(|| VerifyError::VerificationFailed("No leaf cert".to_string()))?;

        let verifying_key = VerifyingKey::from_bytes(&leaf_key)
            .map_err(|e| VerifyError::VerificationFailed(e.to_string()))?;

        let sig = Signature::from_bytes(signature);

        verifying_key.verify(data, &sig)
            .map_err(|e| VerifyError::VerificationFailed(e.to_string()))?;

        Ok(())
    }

    /// Get trust store (for management)
    pub fn trust_store(&self) -> &TrustStore {
        &self.trust_store
    }

    /// Get mutable trust store
    pub fn trust_store_mut(&mut self) -> &mut TrustStore {
        &mut self.trust_store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message_types::{MessageType, Money};
    use hsip_enterprise::{EnterpriseIdentity, IdentityCapabilities, IdentityMetadata, TrustAnchor, TrustAnchorMetadata};
    use chrono::Utc;

    #[test]
    fn test_verification_flow() {
        // Create enterprise identity
        let metadata = IdentityMetadata {
            organization_name: "Example Bank".to_string(),
            contact_email: "security@bank.example.com".to_string(),
            industry: "Banking".to_string(),
            country_code: "US".to_string(),
        };

        let root = EnterpriseIdentity::create("bank.example.com".to_string(), metadata.clone());

        let sub = root.create_sub_identity(
            "fraud-alerts",
            IdentityCapabilities::SEND_TRANSACTION_ALERTS,
            chrono::Duration::days(90),
        ).unwrap();

        // Create message
        let msg_metadata = MessageMetadata {
            message_id: "msg_001".to_string(),
            sender_domain: "bank.example.com".to_string(),
            recipient: "user@example.com".to_string(),
            message_type: MessageType::TransactionAlert {
                amount: Money::usd(1234, 56),
                merchant: "Amazon".to_string(),
                transaction_id: "txn_001".to_string(),
                timestamp: Utc::now(),
                card_last_four: "1234".to_string(),
            },
            sent_at: Utc::now(),
            expires_at: None,
            nonce: [0u8; 16],
        };

        // Create cert chain
        let mut chain = CertChain::new(root.domain.clone(), root.public_key);
        chain.add_cert(sub.delegation_cert.clone());

        // Sign message
        let mut signed_msg = SignedMessage {
            metadata: msg_metadata,
            cert_chain: chain,
            signature: [0u8; 64],
        };

        let data = signed_msg.serialize_for_signing();
        let signature = sub.sign(&data).unwrap();
        signed_msg.signature = signature;

        // Create trust store
        let mut trust_store = TrustStore::new();
        trust_store.add_anchor(TrustAnchor {
            domain: root.domain.clone(),
            public_key: root.public_key,
            added_at: Utc::now(),
            metadata: TrustAnchorMetadata {
                organization_name: metadata.organization_name,
                industry: metadata.industry,
                country_code: metadata.country_code,
                verified: true,
                notes: "Test".to_string(),
            },
        });

        // Verify
        let engine = VerificationEngine::new(trust_store);
        let result = engine.verify(&signed_msg);

        assert_eq!(result, VerificationResult::Verified);
        assert_eq!(result.status_text(), "VERIFIED");
        assert_eq!(result.ui_indicator(), "✅");
    }
}
