//! Trust Store
//!
//! Manages trusted root identities (trust anchors) for verification

use crate::{EnterpriseError, Result, RevocationManager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A trusted root identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustAnchor {
    pub domain: String,
    pub public_key: [u8; 32],
    pub added_at: chrono::DateTime<chrono::Utc>,
    pub metadata: TrustAnchorMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustAnchorMetadata {
    pub organization_name: String,
    pub industry: String,
    pub country_code: String,
    pub verified: bool, // manually verified by user/admin
    pub notes: String,
}

/// Trust store managing all trusted roots
pub struct TrustStore {
    anchors: HashMap<String, TrustAnchor>,
    revocation_manager: RevocationManager,
}

impl TrustStore {
    pub fn new() -> Self {
        Self {
            anchors: HashMap::new(),
            revocation_manager: RevocationManager::new(),
        }
    }

    /// Add a trust anchor
    pub fn add_anchor(&mut self, anchor: TrustAnchor) {
        self.anchors.insert(anchor.domain.clone(), anchor);
    }

    /// Remove a trust anchor
    pub fn remove_anchor(&mut self, domain: &str) -> Option<TrustAnchor> {
        self.anchors.remove(domain)
    }

    /// Get a trust anchor by domain
    pub fn get_anchor(&self, domain: &str) -> Option<&TrustAnchor> {
        self.anchors.get(domain)
    }

    /// Check if a domain is trusted
    pub fn is_trusted(&self, domain: &str) -> bool {
        self.anchors.contains_key(domain)
    }

    /// Verify a cert chain against trust store
    pub fn verify_cert_chain(&self, chain: &crate::CertChain) -> Result<()> {
        // Check if root is trusted
        let anchor = self.anchors.get(&chain.root_domain)
            .ok_or_else(|| EnterpriseError::IdentityNotFound(
                format!("Domain {} not in trust store", chain.root_domain)
            ))?;

        // Verify root public key matches
        if anchor.public_key != chain.root_public_key {
            return Err(EnterpriseError::CertChainInvalid(
                "Root public key mismatch".to_string()
            ));
        }

        // Check if root key is revoked
        if self.revocation_manager.is_revoked(&chain.root_public_key) {
            return Err(EnterpriseError::KeyRevoked(
                format!("Root key for {} is revoked", chain.root_domain)
            ));
        }

        // Validate the chain itself
        chain.validate()?;

        // Check if leaf key is revoked
        if let Some(leaf_key) = chain.leaf_public_key() {
            if self.revocation_manager.is_revoked(&leaf_key) {
                return Err(EnterpriseError::KeyRevoked(
                    "Leaf key is revoked".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Get revocation manager
    pub fn revocation_manager_mut(&mut self) -> &mut RevocationManager {
        &mut self.revocation_manager
    }

    /// List all trusted domains
    pub fn list_domains(&self) -> Vec<String> {
        self.anchors.keys().cloned().collect()
    }

    /// Count of trust anchors
    pub fn anchor_count(&self) -> usize {
        self.anchors.len()
    }
}

impl Default for TrustStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-configured trust stores for common use cases
pub mod presets {
    use super::*;

    /// Create a trust store with common banks (example)
    pub fn common_banks() -> TrustStore {
        let mut store = TrustStore::new();

        // NOTE: These are EXAMPLES ONLY - real deployments would use actual verified keys
        // and would be distributed via secure channels

        // Example: Major US Bank
        store.add_anchor(TrustAnchor {
            domain: "example-bank.com".to_string(),
            public_key: [0u8; 32], // PLACEHOLDER - use real key
            added_at: chrono::Utc::now(),
            metadata: TrustAnchorMetadata {
                organization_name: "Example Bank".to_string(),
                industry: "Banking".to_string(),
                country_code: "US".to_string(),
                verified: false,
                notes: "Example only - replace with real key".to_string(),
            },
        });

        store
    }

    /// Create an empty trust store for manual configuration
    pub fn empty() -> TrustStore {
        TrustStore::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{EnterpriseIdentity, IdentityCapabilities, IdentityMetadata};
    use chrono::Duration;

    #[test]
    fn test_trust_store_basic() {
        let mut store = TrustStore::new();

        let anchor = TrustAnchor {
            domain: "bank.example.com".to_string(),
            public_key: [1u8; 32],
            added_at: chrono::Utc::now(),
            metadata: TrustAnchorMetadata {
                organization_name: "Example Bank".to_string(),
                industry: "Banking".to_string(),
                country_code: "US".to_string(),
                verified: true,
                notes: "Test anchor".to_string(),
            },
        };

        store.add_anchor(anchor);

        assert!(store.is_trusted("bank.example.com"));
        assert!(!store.is_trusted("evil-bank.com"));
        assert_eq!(store.anchor_count(), 1);
    }

    #[test]
    fn test_verify_cert_chain_with_trust_store() {
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
            Duration::days(90),
        ).unwrap();

        let mut chain = crate::CertChain::new(root.domain.clone(), root.public_key);
        chain.add_cert(sub.delegation_cert.clone());

        let mut store = TrustStore::new();
        store.add_anchor(TrustAnchor {
            domain: root.domain.clone(),
            public_key: root.public_key,
            added_at: chrono::Utc::now(),
            metadata: TrustAnchorMetadata {
                organization_name: metadata.organization_name,
                industry: metadata.industry,
                country_code: metadata.country_code,
                verified: true,
                notes: "Test".to_string(),
            },
        });

        assert!(store.verify_cert_chain(&chain).is_ok());
    }
}
