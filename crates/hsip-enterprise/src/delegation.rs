//! Sub-Identity Delegation
//!
//! Allows root identities to create sub-identities with limited capabilities
//! and time-bound validity (e.g., "bank.example.com/fraud-alerts")

use crate::{EnterpriseError, IdentityCapabilities, Result};
use chrono::{DateTime, Duration, Utc};
use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Signature, Verifier};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

/// A delegated sub-identity with limited capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubIdentity {
    pub parent_domain: String,
    pub name: String,
    pub full_name: String, // e.g., "bank.example.com/fraud-alerts"
    pub public_key: [u8; 32],
    #[serde(skip)]
    signing_key: Option<SigningKey>,
    pub capabilities: IdentityCapabilities,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub delegation_cert: DelegationCert,
}

/// Delegation certificate proving authority chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationCert {
    pub parent_domain: String,
    pub parent_public_key: [u8; 32],
    pub sub_identity_name: String,
    pub sub_identity_public_key: [u8; 32],
    pub capabilities: IdentityCapabilities,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub nonce: [u8; 16],
    #[serde(with = "serde_big_array::BigArray")]
    pub parent_signature: [u8; 64],
}

impl SubIdentity {
    /// Create a new sub-identity
    pub(crate) fn new(
        parent_domain: &str,
        name: &str,
        parent_signing_key: &SigningKey,
        capabilities: IdentityCapabilities,
        validity_duration: Duration,
    ) -> Result<Self> {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: VerifyingKey = (&signing_key).into();

        let full_name = format!("{}/{}", parent_domain, name);
        let created_at = Utc::now();
        let expires_at = created_at + validity_duration;

        let parent_verifying_key: VerifyingKey = parent_signing_key.into();

        let delegation_cert = DelegationCert::create(
            parent_domain,
            &parent_verifying_key.to_bytes(),
            name,
            &verifying_key.to_bytes(),
            capabilities,
            created_at,
            expires_at,
            parent_signing_key,
        )?;

        Ok(Self {
            parent_domain: parent_domain.to_string(),
            name: name.to_string(),
            full_name,
            public_key: verifying_key.to_bytes(),
            signing_key: Some(signing_key),
            capabilities,
            created_at,
            expires_at,
            delegation_cert,
        })
    }

    /// Sign data with this sub-identity
    pub fn sign(&self, data: &[u8]) -> Result<[u8; 64]> {
        if Utc::now() > self.expires_at {
            return Err(EnterpriseError::DelegationExpired(
                self.expires_at.to_rfc3339()
            ));
        }

        let signing_key = self.signing_key.as_ref()
            .ok_or_else(|| EnterpriseError::CryptoError("No signing key available".to_string()))?;

        let signature = signing_key.sign(data);
        Ok(signature.to_bytes())
    }

    /// Check if this sub-identity is still valid
    pub fn is_valid(&self) -> bool {
        Utc::now() <= self.expires_at
    }

    /// Check if has capability
    pub fn has_capability(&self, cap: IdentityCapabilities) -> bool {
        self.capabilities.has(cap)
    }
}

impl DelegationCert {
    pub(crate) fn create(
        parent_domain: &str,
        parent_public_key: &[u8; 32],
        sub_identity_name: &str,
        sub_identity_public_key: &[u8; 32],
        capabilities: IdentityCapabilities,
        issued_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        parent_signing_key: &SigningKey,
    ) -> Result<Self> {
        let mut nonce = [0u8; 16];
        rand::RngCore::fill_bytes(&mut OsRng, &mut nonce);

        let mut cert = Self {
            parent_domain: parent_domain.to_string(),
            parent_public_key: *parent_public_key,
            sub_identity_name: sub_identity_name.to_string(),
            sub_identity_public_key: *sub_identity_public_key,
            capabilities,
            issued_at,
            expires_at,
            nonce,
            parent_signature: [0u8; 64],
        };

        let cert_bytes = cert.serialize_for_signing();
        let signature = parent_signing_key.sign(&cert_bytes);
        cert.parent_signature = signature.to_bytes();

        Ok(cert)
    }

    /// Serialize cert for signing (everything except signature)
    fn serialize_for_signing(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"HSIP_DELEGATION_CERT_V1|");
        bytes.extend_from_slice(self.parent_domain.as_bytes());
        bytes.push(b'|');
        bytes.extend_from_slice(&self.parent_public_key);
        bytes.push(b'|');
        bytes.extend_from_slice(self.sub_identity_name.as_bytes());
        bytes.push(b'|');
        bytes.extend_from_slice(&self.sub_identity_public_key);
        bytes.push(b'|');
        bytes.extend_from_slice(&self.capabilities.0.to_be_bytes());
        bytes.push(b'|');
        bytes.extend_from_slice(self.issued_at.to_rfc3339().as_bytes());
        bytes.push(b'|');
        bytes.extend_from_slice(self.expires_at.to_rfc3339().as_bytes());
        bytes.push(b'|');
        bytes.extend_from_slice(&self.nonce);
        bytes
    }

    /// Verify delegation certificate signature
    pub fn verify(&self) -> Result<()> {
        let parent_verifying_key = VerifyingKey::from_bytes(&self.parent_public_key)
            .map_err(|e| EnterpriseError::CryptoError(e.to_string()))?;

        let cert_bytes = self.serialize_for_signing();
        let signature = Signature::from_bytes(&self.parent_signature);

        parent_verifying_key.verify(&cert_bytes, &signature)
            .map_err(|e| EnterpriseError::InvalidSignature(e.to_string()))?;

        // Check expiration
        if Utc::now() > self.expires_at {
            return Err(EnterpriseError::DelegationExpired(
                self.expires_at.to_rfc3339()
            ));
        }

        Ok(())
    }

    /// Check if cert is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::EnterpriseIdentity;
    use crate::identity::IdentityMetadata;

    #[test]
    fn test_create_sub_identity() {
        let metadata = IdentityMetadata {
            organization_name: "Example Bank".to_string(),
            contact_email: "security@bank.example.com".to_string(),
            industry: "Banking".to_string(),
            country_code: "US".to_string(),
        };

        let root = EnterpriseIdentity::create("bank.example.com".to_string(), metadata);

        let sub = root.create_sub_identity(
            "fraud-alerts",
            IdentityCapabilities::SEND_TRANSACTION_ALERTS,
            Duration::days(90),
        ).unwrap();

        assert_eq!(sub.full_name, "bank.example.com/fraud-alerts");
        assert!(sub.is_valid());
        assert!(sub.has_capability(IdentityCapabilities::SEND_TRANSACTION_ALERTS));
    }

    #[test]
    fn test_delegation_cert_verification() {
        let metadata = IdentityMetadata {
            organization_name: "Example Bank".to_string(),
            contact_email: "security@bank.example.com".to_string(),
            industry: "Banking".to_string(),
            country_code: "US".to_string(),
        };

        let root = EnterpriseIdentity::create("bank.example.com".to_string(), metadata);

        let sub = root.create_sub_identity(
            "fraud-alerts",
            IdentityCapabilities::SEND_TRANSACTION_ALERTS,
            Duration::days(90),
        ).unwrap();

        assert!(sub.delegation_cert.verify().is_ok());
    }

    #[test]
    fn test_sub_identity_sign() {
        let metadata = IdentityMetadata {
            organization_name: "Example Bank".to_string(),
            contact_email: "security@bank.example.com".to_string(),
            industry: "Banking".to_string(),
            country_code: "US".to_string(),
        };

        let root = EnterpriseIdentity::create("bank.example.com".to_string(), metadata);

        let sub = root.create_sub_identity(
            "fraud-alerts",
            IdentityCapabilities::SEND_TRANSACTION_ALERTS,
            Duration::days(90),
        ).unwrap();

        let data = b"transaction alert";
        let signature = sub.sign(data).unwrap();

        let verifying_key = VerifyingKey::from_bytes(&sub.public_key).unwrap();
        let sig = Signature::from_bytes(&signature);
        assert!(verifying_key.verify(data, &sig).is_ok());
    }
}
