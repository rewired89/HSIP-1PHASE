//! Audit Log (Hash-Chained, Append-Only)

use crate::{AuditEntry, Error, Result};
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};

/// Tamper-evident audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub log_id: String,
    pub entries: Vec<SignedEntry>,
    pub signing_key_public: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedEntry {
    pub entry: AuditEntry,
    #[serde(with = "serde_big_array::BigArray")]
    pub signature: [u8; 64],
}

/// Audit error type (re-exported)
pub use crate::Error as AuditError;

impl AuditLog {
    /// Create a new audit log
    pub fn new(log_id: String, signing_key: &SigningKey) -> Self {
        let verifying_key: ed25519_dalek::VerifyingKey = signing_key.into();

        Self {
            log_id,
            entries: Vec::new(),
            signing_key_public: verifying_key.to_bytes(),
        }
    }

    /// Append an entry to the log
    pub fn append(&mut self, entry: AuditEntry, signing_key: &SigningKey) -> Result<()> {
        // Verify sequence number
        let expected_seq = self.entries.len() as u64 + 1;
        if entry.sequence_number != expected_seq {
            return Err(Error::InvalidEntry(format!(
                "Sequence mismatch: expected {}, got {}",
                expected_seq, entry.sequence_number
            )));
        }

        // Verify previous hash
        if let Some(last_entry) = self.entries.last() {
            let expected_prev_hash = last_entry.entry.compute_hash();
            if entry.previous_hash != expected_prev_hash {
                return Err(Error::ChainBroken(entry.sequence_number));
            }
        } else {
            // First entry: previous hash should be all zeros
            if entry.previous_hash != [0u8; 32] {
                return Err(Error::InvalidEntry(
                    "First entry must have zero previous_hash".to_string()
                ));
            }
        }

        // Sign the entry
        let signature = entry.sign(signing_key);

        self.entries.push(SignedEntry { entry, signature });

        Ok(())
    }

    /// Verify integrity of the entire log
    pub fn verify_integrity(&self) -> Result<()> {
        for (i, signed_entry) in self.entries.iter().enumerate() {
            let expected_seq = i as u64 + 1;

            // Check sequence number
            if signed_entry.entry.sequence_number != expected_seq {
                return Err(Error::ChainBroken(expected_seq));
            }

            // Check previous hash
            if i > 0 {
                let prev_hash = self.entries[i - 1].entry.compute_hash();
                if signed_entry.entry.previous_hash != prev_hash {
                    return Err(Error::ChainBroken(expected_seq));
                }
            }

            // Verify signature
            signed_entry.entry.verify_signature(
                &self.signing_key_public,
                &signed_entry.signature,
            )?;
        }

        Ok(())
    }

    /// Get the current head hash
    pub fn head_hash(&self) -> Option<[u8; 32]> {
        self.entries.last().map(|e| e.entry.compute_hash())
    }

    /// Get entry count
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Get entry by sequence number (1-indexed)
    pub fn get_entry(&self, sequence_number: u64) -> Option<&AuditEntry> {
        if sequence_number < 1 || sequence_number as usize > self.entries.len() {
            return None;
        }

        Some(&self.entries[sequence_number as usize - 1].entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuditEventType;
    use rand::rngs::OsRng;

    #[test]
    fn test_audit_log_append() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let mut log = AuditLog::new("test_log".to_string(), &signing_key);

        let entry1 = AuditEntry::new(
            1,
            [0u8; 32],
            AuditEventType::MessageSent,
            [1u8; 32],
            "sender".to_string(),
            "recipient".to_string(),
        );

        log.append(entry1, &signing_key).unwrap();

        assert_eq!(log.entry_count(), 1);
    }

    #[test]
    fn test_audit_log_chain() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let mut log = AuditLog::new("test_log".to_string(), &signing_key);

        let entry1 = AuditEntry::new(
            1,
            [0u8; 32],
            AuditEventType::MessageSent,
            [1u8; 32],
            "sender".to_string(),
            "recipient".to_string(),
        );

        log.append(entry1, &signing_key).unwrap();

        let prev_hash = log.head_hash().unwrap();

        let entry2 = AuditEntry::new(
            2,
            prev_hash,
            AuditEventType::MessageReceived,
            [2u8; 32],
            "sender2".to_string(),
            "recipient2".to_string(),
        );

        log.append(entry2, &signing_key).unwrap();

        assert_eq!(log.entry_count(), 2);
        assert!(log.verify_integrity().is_ok());
    }

    #[test]
    fn test_tamper_detection() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let mut log = AuditLog::new("test_log".to_string(), &signing_key);

        let entry1 = AuditEntry::new(
            1,
            [0u8; 32],
            AuditEventType::MessageSent,
            [1u8; 32],
            "sender".to_string(),
            "recipient".to_string(),
        );

        log.append(entry1, &signing_key).unwrap();

        // Tamper with the entry
        log.entries[0].entry.sender_identity = "attacker".to_string();

        // Verification should fail
        assert!(log.verify_integrity().is_err());
    }
}
