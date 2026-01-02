//! Enterprise Root Identity
//!
//! Represents a root organizational identity (e.g., "bank.example.com")
//! with Ed25519 signing capabilities and sub-identity delegation.

use crate::{EnterpriseError, Result, SubIdentity};
use chrono::{DateTime, Duration, Utc};
use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Verifier, Signature};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

/// Capabilities bitmask for enterprise identities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityCapabilities(pub u32);

impl IdentityCapabilities {
    pub const SEND_TRANSACTION_ALERTS: Self = Self(1 << 0);
    pub const SEND_LOGIN_NOTICES: Self = Self(1 << 1);
    pub const SEND_SECURITY_ALERTS: Self = Self(1 << 2);
    pub const SEND_COMPLIANCE_NOTICES: Self = Self(1 << 3);
    pub const SEND_MARKETING: Self = Self(1 << 4);
    pub const CREATE_SUB_IDENTITIES: Self = Self(1 << 5);
    pub const REVOKE_KEYS: Self = Self(1 << 6);
    pub const AUDIT_EXPORT: Self = Self(1 << 7);
    pub const ALL: Self = Self(0xFFFF_FFFF);

    pub fn has(&self, cap: Self) -> bool {
        (self.0 & cap.0) == cap.0
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Root enterprise identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseIdentity {
    pub domain: String,
    pub public_key: [u8; 32],
    #[serde(skip)]
    signing_key: Option<SigningKey>,
    pub capabilities: IdentityCapabilities,
    pub created_at: DateTime<Utc>,
    pub metadata: IdentityMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityMetadata {
    pub organization_name: String,
    pub contact_email: String,
    pub industry: String,
    pub country_code: String,
}

impl EnterpriseIdentity {
    /// Create a new root enterprise identity
    ///
    /// This should be done in a secure key ceremony (HSM, air-gapped machine)
    pub fn create(domain: String, metadata: IdentityMetadata) -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: VerifyingKey = (&signing_key).into();

        Self {
            domain,
            public_key: verifying_key.to_bytes(),
            signing_key: Some(signing_key),
            capabilities: IdentityCapabilities::ALL,
            created_at: Utc::now(),
            metadata,
        }
    }

    /// Load from secure storage (reads signing key from protected keystore)
    pub fn load(domain: &str, signing_key_bytes: [u8; 32], metadata: IdentityMetadata) -> Result<Self> {
        let signing_key = SigningKey::from_bytes(&signing_key_bytes);
        let verifying_key: VerifyingKey = (&signing_key).into();

        Ok(Self {
            domain: domain.to_string(),
            public_key: verifying_key.to_bytes(),
            signing_key: Some(signing_key),
            capabilities: IdentityCapabilities::ALL,
            created_at: Utc::now(),
            metadata,
        })
    }

    /// Create a sub-identity with delegation
    pub fn create_sub_identity(
        &self,
        name: &str,
        capabilities: IdentityCapabilities,
        validity_duration: Duration,
    ) -> Result<SubIdentity> {
        if !self.capabilities.has(IdentityCapabilities::CREATE_SUB_IDENTITIES) {
            return Err(EnterpriseError::InsufficientCapabilities {
                required: "CREATE_SUB_IDENTITIES".to_string(),
                actual: format!("{:?}", self.capabilities),
            });
        }

        let signing_key = self.signing_key.as_ref()
            .ok_or_else(|| EnterpriseError::CryptoError("No signing key available".to_string()))?;

        SubIdentity::new(
            &self.domain,
            name,
            signing_key,
            capabilities,
            validity_duration,
        )
    }

    /// Sign arbitrary data
    pub fn sign(&self, data: &[u8]) -> Result<[u8; 64]> {
        let signing_key = self.signing_key.as_ref()
            .ok_or_else(|| EnterpriseError::CryptoError("No signing key available".to_string()))?;

        let signature = signing_key.sign(data);
        Ok(signature.to_bytes())
    }

    /// Verify signature against this identity's public key
    pub fn verify(&self, data: &[u8], signature: &[u8; 64]) -> Result<()> {
        let verifying_key = VerifyingKey::from_bytes(&self.public_key)
            .map_err(|e| EnterpriseError::CryptoError(e.to_string()))?;

        let sig = Signature::from_bytes(signature);

        verifying_key.verify(data, &sig)
            .map_err(|e| EnterpriseError::InvalidSignature(e.to_string()))
    }

    /// Get identity ID (hash of domain + public key)
    pub fn identity_id(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.domain.as_bytes());
        hasher.update(&self.public_key);
        hex::encode(hasher.finalize().as_bytes())
    }

    /// Export public identity (safe to share, no private key)
    pub fn to_public(&self) -> PublicEnterpriseIdentity {
        PublicEnterpriseIdentity {
            domain: self.domain.clone(),
            public_key: self.public_key,
            capabilities: self.capabilities,
            created_at: self.created_at,
            metadata: self.metadata.clone(),
        }
    }
}

/// Public-facing enterprise identity (no private key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicEnterpriseIdentity {
    pub domain: String,
    pub public_key: [u8; 32],
    pub capabilities: IdentityCapabilities,
    pub created_at: DateTime<Utc>,
    pub metadata: IdentityMetadata,
}

impl PublicEnterpriseIdentity {
    pub fn verify(&self, data: &[u8], signature: &[u8; 64]) -> Result<()> {
        let verifying_key = VerifyingKey::from_bytes(&self.public_key)
            .map_err(|e| EnterpriseError::CryptoError(e.to_string()))?;

        let sig = Signature::from_bytes(signature);

        verifying_key.verify(data, &sig)
            .map_err(|e| EnterpriseError::InvalidSignature(e.to_string()))
    }

    pub fn identity_id(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.domain.as_bytes());
        hasher.update(&self.public_key);
        hex::encode(hasher.finalize().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_enterprise_identity() {
        let metadata = IdentityMetadata {
            organization_name: "Example Bank".to_string(),
            contact_email: "security@bank.example.com".to_string(),
            industry: "Banking".to_string(),
            country_code: "US".to_string(),
        };

        let identity = EnterpriseIdentity::create(
            "bank.example.com".to_string(),
            metadata,
        );

        assert_eq!(identity.domain, "bank.example.com");
        assert!(identity.capabilities.has(IdentityCapabilities::ALL));
    }

    #[test]
    fn test_sign_and_verify() {
        let metadata = IdentityMetadata {
            organization_name: "Example Bank".to_string(),
            contact_email: "security@bank.example.com".to_string(),
            industry: "Banking".to_string(),
            country_code: "US".to_string(),
        };

        let identity = EnterpriseIdentity::create(
            "bank.example.com".to_string(),
            metadata,
        );

        let data = b"test message";
        let signature = identity.sign(data).unwrap();

        assert!(identity.verify(data, &signature).is_ok());

        let wrong_data = b"wrong message";
        assert!(identity.verify(wrong_data, &signature).is_err());
    }

    #[test]
    fn test_capabilities() {
        let caps = IdentityCapabilities::SEND_TRANSACTION_ALERTS
            .union(IdentityCapabilities::SEND_LOGIN_NOTICES);

        assert!(caps.has(IdentityCapabilities::SEND_TRANSACTION_ALERTS));
        assert!(caps.has(IdentityCapabilities::SEND_LOGIN_NOTICES));
        assert!(!caps.has(IdentityCapabilities::SEND_MARKETING));
    }
}
