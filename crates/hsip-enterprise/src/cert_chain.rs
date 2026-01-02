//! Certificate Chain Validation
//!
//! Validates delegation chains from root to sub-identity

use crate::{DelegationCert, EnterpriseError, Result};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// Certificate chain from root to leaf
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertChain {
    pub root_domain: String,
    pub root_public_key: [u8; 32],
    pub chain: Vec<DelegationCert>,
}

impl CertChain {
    /// Create a new cert chain
    pub fn new(root_domain: String, root_public_key: [u8; 32]) -> Self {
        Self {
            root_domain,
            root_public_key,
            chain: Vec::new(),
        }
    }

    /// Add a delegation cert to the chain
    pub fn add_cert(&mut self, cert: DelegationCert) {
        self.chain.push(cert);
    }

    /// Get the leaf (final) public key in the chain
    pub fn leaf_public_key(&self) -> Option<[u8; 32]> {
        self.chain.last().map(|cert| cert.sub_identity_public_key)
    }

    /// Validate the entire chain
    pub fn validate(&self) -> Result<()> {
        if self.chain.is_empty() {
            return Err(EnterpriseError::CertChainInvalid(
                "Empty cert chain".to_string()
            ));
        }

        let mut current_public_key = self.root_public_key;

        for cert in &self.chain {
            // Verify cert is signed by current public key
            if cert.parent_public_key != current_public_key {
                return Err(EnterpriseError::CertChainInvalid(
                    "Parent public key mismatch".to_string()
                ));
            }

            // Verify cert signature
            cert.verify()?;

            // Move to next level
            current_public_key = cert.sub_identity_public_key;
        }

        Ok(())
    }

    /// Get the full identity name (e.g., "bank.example.com/fraud-alerts")
    pub fn full_name(&self) -> String {
        if self.chain.is_empty() {
            return self.root_domain.clone();
        }

        let mut name = self.root_domain.clone();
        for cert in &self.chain {
            name.push('/');
            name.push_str(&cert.sub_identity_name);
        }
        name
    }
}

/// Certificate chain validator
pub struct CertChainValidator {
    trusted_roots: Vec<([u8; 32], String)>, // (public_key, domain)
}

impl CertChainValidator {
    pub fn new() -> Self {
        Self {
            trusted_roots: Vec::new(),
        }
    }

    /// Add a trusted root public key
    pub fn add_trusted_root(&mut self, domain: String, public_key: [u8; 32]) {
        self.trusted_roots.push((public_key, domain));
    }

    /// Validate a cert chain against trusted roots
    pub fn validate_chain(&self, chain: &CertChain) -> Result<()> {
        // Check if root is trusted
        let is_trusted = self.trusted_roots.iter()
            .any(|(pk, domain)| {
                pk == &chain.root_public_key && domain == &chain.root_domain
            });

        if !is_trusted {
            return Err(EnterpriseError::CertChainInvalid(
                format!("Root {} is not trusted", chain.root_domain)
            ));
        }

        // Validate the chain itself
        chain.validate()
    }

    /// Verify a message signature with cert chain
    pub fn verify_with_chain(
        &self,
        chain: &CertChain,
        data: &[u8],
        signature: &[u8; 64],
    ) -> Result<()> {
        // Validate chain first
        self.validate_chain(chain)?;

        // Get leaf public key
        let leaf_key = chain.leaf_public_key()
            .ok_or_else(|| EnterpriseError::CertChainInvalid("No leaf cert".to_string()))?;

        // Verify signature
        let verifying_key = VerifyingKey::from_bytes(&leaf_key)
            .map_err(|e| EnterpriseError::CryptoError(e.to_string()))?;

        let sig = Signature::from_bytes(signature);

        verifying_key.verify(data, &sig)
            .map_err(|e| EnterpriseError::InvalidSignature(e.to_string()))
    }
}

impl Default for CertChainValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{EnterpriseIdentity, IdentityCapabilities, IdentityMetadata};
    use chrono::Duration;

    #[test]
    fn test_cert_chain_validation() {
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

        let mut chain = CertChain::new(root.domain.clone(), root.public_key);
        chain.add_cert(sub.delegation_cert.clone());

        assert!(chain.validate().is_ok());
        assert_eq!(chain.full_name(), "bank.example.com/fraud-alerts");
    }

    #[test]
    fn test_cert_chain_validator() {
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

        let mut chain = CertChain::new(root.domain.clone(), root.public_key);
        chain.add_cert(sub.delegation_cert.clone());

        let mut validator = CertChainValidator::new();
        validator.add_trusted_root(root.domain.clone(), root.public_key);

        assert!(validator.validate_chain(&chain).is_ok());

        // Test message verification
        let data = b"transaction alert";
        let signature = sub.sign(data).unwrap();

        assert!(validator.verify_with_chain(&chain, data, &signature).is_ok());
    }
}
