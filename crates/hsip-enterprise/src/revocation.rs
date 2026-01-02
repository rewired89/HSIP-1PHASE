//! Key Revocation Lists (CRL)
//!
//! Manages revoked keys and delegation certificates

use crate::{EnterpriseError, Result};
use chrono::{DateTime, Utc};
use ed25519_dalek::{SigningKey, Signer, VerifyingKey, Verifier, Signature};
use serde::{Deserialize, Serialize};

/// Reason for key revocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RevocationReason {
    KeyCompromise,
    EmployeeTermination,
    SupersededByNewKey,
    PolicyViolation,
    Unspecified,
}

/// Single revocation entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationEntry {
    pub revoked_public_key: [u8; 32],
    pub reason: RevocationReason,
    pub revoked_at: DateTime<Utc>,
    pub revoked_by: String, // domain of revoking authority
    pub notes: String,
}

/// Revocation List (CRL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationList {
    pub issuer_domain: String,
    pub issuer_public_key: [u8; 32],
    pub sequence_number: u64,
    pub issued_at: DateTime<Utc>,
    pub next_update: DateTime<Utc>,
    pub entries: Vec<RevocationEntry>,
    pub signature: [u8; 64],
}

impl RevocationList {
    /// Create a new revocation list
    pub fn create(
        issuer_domain: String,
        issuer_public_key: [u8; 32],
        issuer_signing_key: &SigningKey,
        sequence_number: u64,
        next_update: DateTime<Utc>,
        entries: Vec<RevocationEntry>,
    ) -> Result<Self> {
        let issued_at = Utc::now();

        let mut crl = Self {
            issuer_domain,
            issuer_public_key,
            sequence_number,
            issued_at,
            next_update,
            entries,
            signature: [0u8; 64],
        };

        let crl_bytes = crl.serialize_for_signing();
        let signature = issuer_signing_key.sign(&crl_bytes);
        crl.signature = signature.to_bytes();

        Ok(crl)
    }

    /// Serialize CRL for signing
    fn serialize_for_signing(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"HSIP_CRL_V1|");
        bytes.extend_from_slice(self.issuer_domain.as_bytes());
        bytes.push(b'|');
        bytes.extend_from_slice(&self.issuer_public_key);
        bytes.push(b'|');
        bytes.extend_from_slice(&self.sequence_number.to_be_bytes());
        bytes.push(b'|');
        bytes.extend_from_slice(self.issued_at.to_rfc3339().as_bytes());
        bytes.push(b'|');
        bytes.extend_from_slice(self.next_update.to_rfc3339().as_bytes());
        bytes.push(b'|');

        for entry in &self.entries {
            bytes.extend_from_slice(&entry.revoked_public_key);
            bytes.push(b'|');
        }

        bytes
    }

    /// Verify CRL signature
    pub fn verify(&self) -> Result<()> {
        let verifying_key = VerifyingKey::from_bytes(&self.issuer_public_key)
            .map_err(|e| EnterpriseError::CryptoError(e.to_string()))?;

        let crl_bytes = self.serialize_for_signing();
        let signature = Signature::from_bytes(&self.signature);

        verifying_key.verify(&crl_bytes, &signature)
            .map_err(|e| EnterpriseError::InvalidSignature(e.to_string()))?;

        Ok(())
    }

    /// Check if a public key is revoked
    pub fn is_revoked(&self, public_key: &[u8; 32]) -> bool {
        self.entries.iter().any(|entry| &entry.revoked_public_key == public_key)
    }

    /// Check if CRL is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.next_update
    }
}

/// Revocation list manager
pub struct RevocationManager {
    current_crl: Option<RevocationList>,
    cached_crls: Vec<RevocationList>,
}

impl RevocationManager {
    pub fn new() -> Self {
        Self {
            current_crl: None,
            cached_crls: Vec::new(),
        }
    }

    /// Load a CRL
    pub fn load_crl(&mut self, crl: RevocationList) -> Result<()> {
        crl.verify()?;

        if let Some(current) = &self.current_crl {
            if crl.sequence_number > current.sequence_number {
                self.cached_crls.push(current.clone());
                self.current_crl = Some(crl);
            }
        } else {
            self.current_crl = Some(crl);
        }

        Ok(())
    }

    /// Check if a key is revoked
    pub fn is_revoked(&self, public_key: &[u8; 32]) -> bool {
        if let Some(crl) = &self.current_crl {
            if !crl.is_expired() && crl.is_revoked(public_key) {
                return true;
            }
        }

        // Check cached CRLs too
        self.cached_crls.iter()
            .any(|crl| !crl.is_expired() && crl.is_revoked(public_key))
    }

    /// Get current CRL
    pub fn current_crl(&self) -> Option<&RevocationList> {
        self.current_crl.as_ref()
    }
}

impl Default for RevocationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{EnterpriseIdentity, IdentityMetadata};
    use chrono::Duration;

    #[test]
    fn test_create_and_verify_crl() {
        let metadata = IdentityMetadata {
            organization_name: "Example Bank".to_string(),
            contact_email: "security@bank.example.com".to_string(),
            industry: "Banking".to_string(),
            country_code: "US".to_string(),
        };

        let root = EnterpriseIdentity::create("bank.example.com".to_string(), metadata);

        let revoked_key = [1u8; 32];
        let entry = RevocationEntry {
            revoked_public_key: revoked_key,
            reason: RevocationReason::KeyCompromise,
            revoked_at: Utc::now(),
            revoked_by: "bank.example.com".to_string(),
            notes: "Suspected compromise".to_string(),
        };

        let crl = RevocationList::create(
            root.domain.clone(),
            root.public_key,
            root.signing_key.as_ref().unwrap(),
            1,
            Utc::now() + Duration::days(7),
            vec![entry],
        ).unwrap();

        assert!(crl.verify().is_ok());
        assert!(crl.is_revoked(&revoked_key));
        assert!(!crl.is_revoked(&[2u8; 32]));
    }

    #[test]
    fn test_revocation_manager() {
        let metadata = IdentityMetadata {
            organization_name: "Example Bank".to_string(),
            contact_email: "security@bank.example.com".to_string(),
            industry: "Banking".to_string(),
            country_code: "US".to_string(),
        };

        let root = EnterpriseIdentity::create("bank.example.com".to_string(), metadata);

        let revoked_key = [1u8; 32];
        let entry = RevocationEntry {
            revoked_public_key: revoked_key,
            reason: RevocationReason::KeyCompromise,
            revoked_at: Utc::now(),
            revoked_by: "bank.example.com".to_string(),
            notes: "Suspected compromise".to_string(),
        };

        let crl = RevocationList::create(
            root.domain.clone(),
            root.public_key,
            root.signing_key.as_ref().unwrap(),
            1,
            Utc::now() + Duration::days(7),
            vec![entry],
        ).unwrap();

        let mut manager = RevocationManager::new();
        manager.load_crl(crl).unwrap();

        assert!(manager.is_revoked(&revoked_key));
        assert!(!manager.is_revoked(&[2u8; 32]));
    }
}
