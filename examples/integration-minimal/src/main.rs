//! Minimal HSIP Integration Example
//!
//! This example shows how to implement custom policy hooks, audit sinks, and capability
//! providers using the HSIP Integration SDK.
//!
//! **Copy this template to your private repo and customize for your use case.**
//!
//! ## What This Example Shows
//!
//! 1. **StrictPolicyHook**: Auto-deny unknown peers and enforce attempt limits
//! 2. **FileAuditSink**: Log audit events to JSON files with hash chain verification
//! 3. **SimpleCapabilityProvider**: Issue file transfer capabilities to trusted peers
//!
//! ## What This Example Does NOT Do
//!
//! - ❌ Platform-specific logic (Roblox, Discord, etc.)
//! - ❌ Demographics or age verification
//! - ❌ Unverifiable claims or attributes
//! - ❌ Wire format or crypto changes
//!
//! ## Usage
//!
//! Copy this entire directory to your private repo, then customize the implementations
//! to match your specific needs while preserving protocol-level behavior.

use hsip_integration_sdk::{
    AuditEvent, AuditExport, AuditSink, Capability, CapabilityProvider, ConsentRequestContext,
    PolicyDecision, PolicyHook, PolicyReason,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ═══════════════════════════════════════════════════════════════════════════════════
// 1. POLICY HOOK EXAMPLE: Strict Mode
// ═══════════════════════════════════════════════════════════════════════════════════

/// Strict policy hook: deny unknown peers and enforce low attempt limits
pub struct StrictPolicyHook {
    max_attempts: u32,
}

impl StrictPolicyHook {
    pub fn new(max_attempts: u32) -> Self {
        Self { max_attempts }
    }
}

impl PolicyHook for StrictPolicyHook {
    fn evaluate(&self, ctx: &ConsentRequestContext) -> Option<(PolicyDecision, PolicyReason)> {
        // Check for rate limiting (protocol-observable)
        if ctx.rate_limited {
            return Some((
                PolicyDecision::AutoDeny,
                PolicyReason::RateLimitExceeded,
            ));
        }

        // Check for suspicious requests (protocol-observable)
        if ctx.suspicious {
            return Some((
                PolicyDecision::SilentReject,
                PolicyReason::SuspiciousRequest,
            ));
        }

        // Check attempt limit (protocol-observable)
        if ctx.failed_attempts >= self.max_attempts {
            return Some((
                PolicyDecision::AutoDeny,
                PolicyReason::TooManyAttempts {
                    count: ctx.failed_attempts,
                },
            ));
        }

        // Strict mode: deny all unknown peers (protocol-observable)
        if ctx.unknown_peer {
            return Some((
                PolicyDecision::AutoDeny,
                PolicyReason::CustomPolicyRule {
                    rule_id: "strict_mode".into(),
                    reason: "Unknown peer denied in strict mode".into(),
                },
            ));
        }

        // Fall through to default HSIP policy
        None
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 2. AUDIT SINK EXAMPLE: JSON File Logger
// ═══════════════════════════════════════════════════════════════════════════════════

/// File-based audit sink with hash chain verification
pub struct FileAuditSink {
    events: Arc<Mutex<Vec<AuditEvent>>>,
    export_counter: Arc<Mutex<u64>>,
}

impl FileAuditSink {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            export_counter: Arc::new(Mutex::new(0)),
        }
    }

    fn hash_event(event: &AuditEvent) -> String {
        // In production, use BLAKE3 or SHA256
        // This is a placeholder
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        event.event_id.hash(&mut hasher);
        event.timestamp.hash(&mut hasher);
        event.actor_peer_id.hash(&mut hasher);
        event.subject_peer_id.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

impl AuditSink for FileAuditSink {
    fn log_event(&self, event: &AuditEvent) -> Result<(), String> {
        let mut events = self.events.lock().map_err(|e| format!("Lock error: {}", e))?;

        // Verify prev_hash matches if not first event
        if !events.is_empty() {
            let expected_prev_hash = Self::hash_event(events.last().unwrap());
            if event.prev_hash != expected_prev_hash {
                return Err(format!(
                    "Hash chain broken: expected {}, got {}",
                    expected_prev_hash, event.prev_hash
                ));
            }
        }

        events.push(event.clone());
        Ok(())
    }

    fn verify_chain(&self, events: &[AuditEvent]) -> Result<bool, String> {
        if events.is_empty() {
            return Ok(true);
        }

        // Verify hash chain
        for i in 1..events.len() {
            let prev = &events[i - 1];
            let curr = &events[i];
            let expected_hash = Self::hash_event(prev);
            if curr.prev_hash != expected_hash {
                return Ok(false); // Chain broken
            }
        }

        Ok(true)
    }

    fn export(&self) -> Result<AuditExport, String> {
        let events = self.events.lock().map_err(|e| format!("Lock error: {}", e))?;
        let mut counter = self
            .export_counter
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        let genesis_hash = if events.is_empty() {
            "0".repeat(64)
        } else {
            Self::hash_event(&events[0])
        };

        let head_hash = if events.is_empty() {
            "0".repeat(64)
        } else {
            Self::hash_event(events.last().unwrap())
        };

        *counter += 1;

        let verification_hash = format!(
            "{}{}{}",
            genesis_hash,
            head_hash,
            format!("{:016x}", *counter)
        );

        Ok(AuditExport {
            events: events.clone(),
            genesis_hash,
            head_hash,
            export_counter: *counter,
            verification_hash,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 3. CAPABILITY PROVIDER EXAMPLE: File Transfer Permissions
// ═══════════════════════════════════════════════════════════════════════════════════

/// Simple capability provider for file transfer permissions
pub struct SimpleCapabilityProvider {
    trusted_peers: Arc<Mutex<HashMap<String, u64>>>, // peer_id -> expiration_ms
}

impl SimpleCapabilityProvider {
    pub fn new() -> Self {
        Self {
            trusted_peers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_trusted_peer(&self, peer_id: String, expires_ms: u64) {
        if let Ok(mut peers) = self.trusted_peers.lock() {
            peers.insert(peer_id, expires_ms);
        }
    }

    fn is_trusted(&self, peer_id: &str) -> bool {
        if let Ok(peers) = self.trusted_peers.lock() {
            if let Some(&expires_ms) = peers.get(peer_id) {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                return now_ms < expires_ms;
            }
        }
        false
    }
}

impl CapabilityProvider for SimpleCapabilityProvider {
    fn capabilities_for_peer(&self, peer_id: &str) -> Vec<Capability> {
        if !self.is_trusted(peer_id) {
            return vec![];
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        vec![Capability {
            capability_id: "file_transfer".into(),
            token: format!("token:{}", peer_id).into_bytes(),
            expires_ms: now_ms + 3600_000, // 1 hour
        }]
    }

    fn verify_capability(
        &self,
        peer_id: &str,
        capability_id: &str,
        token: &[u8],
    ) -> Result<bool, String> {
        if capability_id != "file_transfer" {
            return Ok(false);
        }

        let expected_token = format!("token:{}", peer_id).into_bytes();
        Ok(token == expected_token && self.is_trusted(peer_id))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// MAIN: Demo Usage
// ═══════════════════════════════════════════════════════════════════════════════════

fn main() {
    println!("HSIP Integration SDK - Minimal Example\n");

    // 1. Policy Hook Demo
    println!("1. Policy Hook Demo:");
    let policy = StrictPolicyHook::new(3);
    let ctx = ConsentRequestContext {
        peer_id: "hsip:ed25519:abc123".into(),
        purpose: "file transfer".into(),
        timestamp_ms: 1234567890,
        unknown_peer: true,
        denied_before: false,
        failed_attempts: 0,
        rate_limited: false,
        suspicious: false,
    };

    match policy.evaluate(&ctx) {
        Some((decision, reason)) => {
            println!("   Decision: {:?}", decision);
            println!("   Reason: {:?}\n", reason);
        }
        None => println!("   Fell through to default policy\n"),
    }

    // 2. Audit Sink Demo
    println!("2. Audit Sink Demo:");
    let audit_sink = FileAuditSink::new();
    let event = AuditEvent {
        event_id: "event-001".into(),
        timestamp: "2026-01-17T00:00:00Z".into(),
        actor_peer_id: "hsip:ed25519:alice".into(),
        subject_peer_id: "hsip:ed25519:bob".into(),
        decision_type: "ALLOW".into(),
        severity: 0,
        reason_code: "PRIOR_CONSENT".into(),
        reason_text: "Prior consent exists".into(),
        evidence: vec![],
        ttl_ms: Some(3600000),
        prev_hash: "0".repeat(64),
        signature: "0".repeat(128),
    };

    audit_sink.log_event(&event).unwrap();
    println!("   Logged audit event: {}", event.event_id);

    let export = audit_sink.export().unwrap();
    println!("   Export counter: {}", export.export_counter);
    println!("   Genesis hash: {}", &export.genesis_hash[..16]);
    println!("   Head hash: {}\n", &export.head_hash[..16]);

    // 3. Capability Provider Demo
    println!("3. Capability Provider Demo:");
    let cap_provider = SimpleCapabilityProvider::new();
    cap_provider.add_trusted_peer(
        "hsip:ed25519:alice".into(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
            + 3600_000,
    );

    let caps = cap_provider.capabilities_for_peer("hsip:ed25519:alice");
    println!("   Capabilities for alice: {} granted", caps.len());
    for cap in caps {
        println!("      - {}", cap.capability_id);
    }

    println!("\nIntegration example complete!");
    println!("Copy this template to your private repo and customize as needed.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strict_policy_denies_unknown() {
        let policy = StrictPolicyHook::new(3);
        let ctx = ConsentRequestContext {
            peer_id: "test".into(),
            purpose: "test".into(),
            timestamp_ms: 1000,
            unknown_peer: true,
            denied_before: false,
            failed_attempts: 0,
            rate_limited: false,
            suspicious: false,
        };

        let result = policy.evaluate(&ctx);
        assert!(result.is_some());
        let (decision, _) = result.unwrap();
        assert_eq!(decision, PolicyDecision::AutoDeny);
    }

    #[test]
    fn test_file_audit_sink_chain() {
        let sink = FileAuditSink::new();
        let event1 = AuditEvent {
            event_id: "e1".into(),
            timestamp: "t1".into(),
            actor_peer_id: "a1".into(),
            subject_peer_id: "s1".into(),
            decision_type: "ALLOW".into(),
            severity: 0,
            reason_code: "test".into(),
            reason_text: "test".into(),
            evidence: vec![],
            ttl_ms: None,
            prev_hash: "0".repeat(64),
            signature: "sig1".into(),
        };

        sink.log_event(&event1).unwrap();
        let export = sink.export().unwrap();
        assert_eq!(export.events.len(), 1);
        assert_eq!(export.export_counter, 1);
    }

    #[test]
    fn test_capability_provider() {
        let provider = SimpleCapabilityProvider::new();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        provider.add_trusted_peer("peer1".into(), now_ms + 10000);
        let caps = provider.capabilities_for_peer("peer1");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].capability_id, "file_transfer");
    }
}
