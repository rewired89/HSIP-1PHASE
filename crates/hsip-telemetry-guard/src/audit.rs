//! Audit Trail - Cryptographic logging of all decisions
//!
//! Integrates with Observer Effect for tamper-evident audit logs.

use crate::{Decision, DecisionType, TelemetryIntent};
use blake3::Hasher;
use chrono::{DateTime, Utc};
use hex;
use hsip_common::quantum_physics::observer_effect::{ObservationLog, ObservationType};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

/// Maximum audit entries to keep in memory
const MAX_AUDIT_ENTRIES: usize = 50000;

/// An audit entry for a telemetry decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry ID
    pub entry_id: [u8; 32],
    /// When the decision was made
    pub timestamp: DateTime<Utc>,
    /// Decision type
    pub decision: DecisionType,
    /// Destination
    pub destination: String,
    /// Intent
    pub intent: TelemetryIntent,
    /// Reason summary
    pub reason: String,
    /// Flow ID prefix (for correlation)
    pub flow_id_prefix: String,
    /// Chain hash (links to previous entry)
    pub prev_hash: [u8; 32],
    /// This entry's hash
    pub entry_hash: [u8; 32],
}

impl AuditEntry {
    /// Create from a decision
    pub fn from_decision(decision: &Decision, prev_hash: [u8; 32]) -> Self {
        let timestamp = Utc::now();

        // Generate entry ID
        let mut id_hasher = Hasher::new();
        id_hasher.update(&timestamp.timestamp_nanos_opt().unwrap_or(0).to_le_bytes());
        id_hasher.update(decision.flow_summary.flow_id_prefix.as_bytes());
        let mut entry_id = [0u8; 32];
        entry_id.copy_from_slice(id_hasher.finalize().as_bytes());

        // Compute entry hash
        let mut hash_hasher = Hasher::new();
        hash_hasher.update(&entry_id);
        hash_hasher.update(&[decision.decision_type as u8]);
        hash_hasher.update(decision.flow_summary.destination.as_bytes());
        hash_hasher.update(&prev_hash);
        let mut entry_hash = [0u8; 32];
        entry_hash.copy_from_slice(hash_hasher.finalize().as_bytes());

        Self {
            entry_id,
            timestamp,
            decision: decision.decision_type,
            destination: decision.flow_summary.destination.clone(),
            intent: decision.flow_summary.intent,
            reason: decision.primary_reason.description(),
            flow_id_prefix: decision.flow_summary.flow_id_prefix.clone(),
            prev_hash,
            entry_hash,
        }
    }

    /// Verify this entry's integrity
    pub fn verify(&self) -> bool {
        let mut hash_hasher = Hasher::new();
        hash_hasher.update(&self.entry_id);
        hash_hasher.update(&[self.decision as u8]);
        hash_hasher.update(self.destination.as_bytes());
        hash_hasher.update(&self.prev_hash);
        let expected = hash_hasher.finalize();

        self.entry_hash == expected.as_bytes()[..32]
    }
}

/// Audit log for telemetry decisions
#[derive(Debug)]
pub struct AuditLog {
    /// Audit entries (append-only in memory)
    entries: RwLock<VecDeque<AuditEntry>>,
    /// Maximum entries
    max_entries: usize,
    /// Observer effect log for cryptographic receipts
    observation_log: Arc<ObservationLog>,
    /// Statistics
    stats: RwLock<AuditStats>,
    /// Genesis hash (hash of first entry, or zero if empty)
    genesis_hash: RwLock<Option<[u8; 32]>>,
    /// Export counter (increments on each export, detects selective export)
    export_counter: RwLock<u64>,
}

/// Audit statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AuditStats {
    /// Total decisions logged
    pub total_logged: u64,
    /// Allowed
    pub allowed: u64,
    /// Blocked
    pub blocked: u64,
    /// Quarantined
    pub quarantined: u64,
    /// Chain integrity verified
    pub chain_valid: bool,
    /// Last verification time
    pub last_verified: Option<DateTime<Utc>>,
    /// Genesis hash (for tamper detection)
    pub genesis_hash: Option<String>,
    /// Current chain head hash
    pub head_hash: Option<String>,
    /// Export counter
    pub export_count: u64,
}

impl AuditLog {
    /// Create a new audit log
    pub fn new() -> Self {
        let binding_secret = {
            let mut secret = [0u8; 32];
            secret[0] = 0xAu8;
            secret[1] = 0xD1u8;
            secret[2] = 0x17u8;
            secret
        };

        Self {
            entries: RwLock::new(VecDeque::with_capacity(MAX_AUDIT_ENTRIES)),
            max_entries: MAX_AUDIT_ENTRIES,
            observation_log: Arc::new(ObservationLog::new(binding_secret)),
            stats: RwLock::new(AuditStats::default()),
            genesis_hash: RwLock::new(None),
            export_counter: RwLock::new(0),
        }
    }

    /// Log a decision
    pub fn log(&self, decision: &Decision) -> [u8; 32] {
        let mut entries = self.entries.write();

        // Get previous hash
        let prev_hash = entries.back()
            .map(|e| e.entry_hash)
            .unwrap_or([0u8; 32]);

        // Create entry
        let entry = AuditEntry::from_decision(decision, prev_hash);
        let entry_id = entry.entry_id;
        let entry_hash = entry.entry_hash;

        // Set genesis hash if this is the first entry
        if entries.is_empty() {
            let mut genesis = self.genesis_hash.write();
            if genesis.is_none() {
                *genesis = Some(entry_hash);
            }
        }

        // Evict oldest if at capacity
        if entries.len() >= self.max_entries {
            entries.pop_front();
        }

        entries.push_back(entry);

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.total_logged += 1;
            match decision.decision_type {
                DecisionType::Allow | DecisionType::AllowOnce => stats.allowed += 1,
                DecisionType::Block => stats.blocked += 1,
                DecisionType::Quarantine => stats.quarantined += 1,
                _ => {}
            }
            stats.head_hash = Some(hex::encode(entry_hash));
            if let Some(gen) = *self.genesis_hash.read() {
                stats.genesis_hash = Some(hex::encode(gen));
            }
        }

        // Also log to observation log for cryptographic receipts
        let resource_id = {
            let mut id = [0u8; 32];
            let hash = blake3::hash(decision.flow_summary.destination.as_bytes());
            id.copy_from_slice(hash.as_bytes());
            id
        };
        let observer_id = [0u8; 32]; // System observer

        self.observation_log.record_observation(
            resource_id,
            observer_id,
            match decision.decision_type {
                DecisionType::Allow | DecisionType::AllowOnce => ObservationType::Read,
                DecisionType::Block => ObservationType::ConsentCheck,
                DecisionType::Quarantine => ObservationType::Export,
                _ => ObservationType::MetadataAccess,
            },
            None,
        );

        entry_id
    }

    /// Verify chain integrity
    pub fn verify_chain(&self) -> bool {
        let entries = self.entries.read();

        let mut expected_prev = [0u8; 32];

        for entry in entries.iter() {
            // Verify entry integrity
            if !entry.verify() {
                return false;
            }

            // Verify chain link
            if entry.prev_hash != expected_prev {
                return false;
            }

            expected_prev = entry.entry_hash;
        }

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.chain_valid = true;
            stats.last_verified = Some(Utc::now());
        }

        true
    }

    /// Get recent entries
    pub fn recent(&self, limit: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read();
        entries.iter().rev().take(limit).cloned().collect()
    }

    /// Get entries for a destination
    pub fn for_destination(&self, destination: &str) -> Vec<AuditEntry> {
        let dest_lower = destination.to_lowercase();
        self.entries
            .read()
            .iter()
            .filter(|e| e.destination.to_lowercase().contains(&dest_lower))
            .cloned()
            .collect()
    }

    /// Get entries by decision type
    pub fn by_decision(&self, decision: DecisionType) -> Vec<AuditEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.decision == decision)
            .cloned()
            .collect()
    }

    /// Get entry count
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    /// Get statistics
    pub fn stats(&self) -> AuditStats {
        self.stats.read().clone()
    }

    /// Get the latest chain hash
    pub fn latest_hash(&self) -> Option<[u8; 32]> {
        self.entries.read().back().map(|e| e.entry_hash)
    }

    /// Clear all entries (for testing)
    pub fn clear(&self) {
        self.entries.write().clear();
        *self.stats.write() = AuditStats::default();
        *self.genesis_hash.write() = None;
        *self.export_counter.write() = 0;
    }

    /// Export as JSON with tamper-evident metadata
    pub fn export_json(&self) -> String {
        let entries: Vec<_> = self.entries.read().iter().cloned().collect();
        let stats = self.stats();

        // Increment export counter
        let export_count = {
            let mut counter = self.export_counter.write();
            *counter += 1;
            *counter
        };

        // Update stats with export count
        {
            let mut s = self.stats.write();
            s.export_count = export_count;
        }

        #[derive(Serialize)]
        struct ExportManifest {
            export_count: u64,
            export_timestamp: DateTime<Utc>,
            genesis_hash: Option<String>,
            head_hash: Option<String>,
            entry_count: usize,
            total_logged: u64,
            chain_valid: bool,
            entries: Vec<AuditEntry>,
        }

        let manifest = ExportManifest {
            export_count,
            export_timestamp: Utc::now(),
            genesis_hash: stats.genesis_hash,
            head_hash: stats.head_hash,
            entry_count: entries.len(),
            total_logged: stats.total_logged,
            chain_valid: stats.chain_valid,
            entries,
        };

        serde_json::to_string_pretty(&manifest).unwrap_or_default()
    }

    /// Export verification hash (hash of all entry hashes in order).
    /// This allows external verification that exported logs are complete and unmodified.
    pub fn export_verification_hash(&self) -> [u8; 32] {
        let entries = self.entries.read();
        let mut hasher = Hasher::new();

        for entry in entries.iter() {
            hasher.update(&entry.entry_hash);
        }

        let mut result = [0u8; 32];
        result.copy_from_slice(hasher.finalize().as_bytes());
        result
    }

    /// Get observation count from underlying log
    pub fn observation_count(&self) -> usize {
        self.observation_log.len()
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared audit log
pub type SharedAuditLog = Arc<AuditLog>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DecisionReason, DecisionFlowSummary, RiskLevel};

    fn test_decision(dest: &str, decision_type: DecisionType) -> Decision {
        Decision {
            decision_type,
            primary_reason: DecisionReason::NoConsent,
            contributing_reasons: Vec::new(),
            timestamp: Utc::now(),
            ttl: None,
            flow_summary: DecisionFlowSummary {
                flow_id_prefix: "12345678".to_string(),
                destination: dest.to_string(),
                intent: TelemetryIntent::UsageAnalytics,
                risk_level: RiskLevel::Medium,
            },
            confidence: 1.0,
        }
    }

    #[test]
    fn test_audit_log_creation() {
        let log = AuditLog::new();
        assert!(log.is_empty());
    }

    #[test]
    fn test_log_decision() {
        let log = AuditLog::new();
        let decision = test_decision("analytics.example.com", DecisionType::Block);

        let entry_id = log.log(&decision);
        assert_eq!(log.len(), 1);

        let recent = log.recent(1);
        assert_eq!(recent[0].entry_id, entry_id);
    }

    #[test]
    fn test_chain_integrity() {
        let log = AuditLog::new();

        // Log multiple decisions
        for i in 0..10 {
            let decision = test_decision(
                &format!("tracker{}.example.com", i),
                DecisionType::Block,
            );
            log.log(&decision);
        }

        assert!(log.verify_chain());
    }

    #[test]
    fn test_filter_by_destination() {
        let log = AuditLog::new();

        log.log(&test_decision("google.com", DecisionType::Block));
        log.log(&test_decision("facebook.com", DecisionType::Block));
        log.log(&test_decision("google-analytics.com", DecisionType::Block));

        let google_entries = log.for_destination("google");
        assert_eq!(google_entries.len(), 2);
    }

    #[test]
    fn test_statistics() {
        let log = AuditLog::new();

        log.log(&test_decision("a.com", DecisionType::Block));
        log.log(&test_decision("b.com", DecisionType::Block));
        log.log(&test_decision("c.com", DecisionType::Allow));

        let stats = log.stats();
        assert_eq!(stats.total_logged, 3);
        assert_eq!(stats.blocked, 2);
        assert_eq!(stats.allowed, 1);
    }

    #[test]
    fn test_observation_integration() {
        let log = AuditLog::new();

        log.log(&test_decision("tracker.com", DecisionType::Block));

        // Should have logged to observation log too
        assert_eq!(log.observation_count(), 1);
    }
}
