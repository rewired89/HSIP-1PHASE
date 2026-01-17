//! HSIP Integration SDK
//!
//! **Stable extension points for third-party HSIP integrations.**
//!
//! This crate provides traits and types that allow external systems to integrate with HSIP
//! without modifying core protocol behavior. All interfaces are generic and protocol-observable
//! only - no platform-specific logic, demographics, or unverifiable claims.
//!
//! ## Stability Guarantee
//!
//! This SDK follows semantic versioning. Breaking changes will increment the major version.
//! Integrations should pin to specific HSIP tags (e.g., `hsip-integration-sdk = "0.1.0"`).
//!
//! ## Core Traits
//!
//! - [`PolicyHook`]: Extend consent policy evaluation with custom logic
//! - [`AuditSink`]: Export audit events to external storage or systems
//! - [`CapabilityProvider`]: Provide opaque capability tokens for peers/sessions
//!
//! ## Usage
//!
//! See `examples/integration-minimal/` for a complete working example.
//!
//! ## What NOT to do
//!
//! - ❌ Do not encode platform-specific identity (Roblox UserID, Discord snowflake, etc.)
//! - ❌ Do not make unverifiable claims (age, demographics, roles beyond protocol-observable)
//! - ❌ Do not modify wire format or cryptographic primitives
//! - ❌ Do not break HSIP Phase 1 compatibility
//!
//! ## Litigation and Court Readiness
//!
//! **CRITICAL:** This SDK preserves HSIP's tamper-evident audit trail and evidence export
//! capabilities. Any integration MUST maintain:
//!
//! - Hash-chained append-only logs
//! - Genesis hash, head hash, export counter integrity
//! - Cryptographic receipts (Observer Effect)
//! - Consent decision logging with timestamps and peer binding
//!
//! Do not weaken or remove these features. DFF eligibility depends on them.

use serde::{Deserialize, Serialize};

/// Policy decision outcome from hook evaluation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyDecision {
    /// Automatically deny without user interaction (abusive pattern detected)
    AutoDeny,
    /// Queue for user review (legitimate but unknown peer)
    QueueForReview,
    /// Automatically accept (prior consent exists and still valid)
    AutoAccept,
    /// Silently reject (malformed or suspicious traffic, no logging)
    SilentReject,
}

/// Reason code for policy decisions (logged for audit trail)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyReason {
    /// Request failed cryptographic validation
    InvalidSignature,
    /// Request exceeded rate limit
    RateLimitExceeded,
    /// Peer was denied consent previously
    PreviouslyDenied,
    /// Repeated failed attempts (possible harassment)
    TooManyAttempts { count: u32 },
    /// Unknown peer, no prior history
    UnknownPeer,
    /// Prior consent exists and is still valid
    PriorConsentValid,
    /// Request has suspicious or malformed fields
    SuspiciousRequest,
    /// Custom policy rule matched (hook-specific)
    CustomPolicyRule { rule_id: String, reason: String },
}

/// Protocol-observable flags about a consent request
///
/// All fields are derived from cryptographic verification, protocol state,
/// or observable behavior - NOT from unverified claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentRequestContext {
    /// Cryptographically verified peer ID (from Ed25519 signature)
    pub peer_id: String,
    /// Purpose string from request (NOT VERIFIED - claimed by requester)
    pub purpose: String,
    /// Request timestamp in milliseconds (signature-verified, within clock skew)
    pub timestamp_ms: u64,
    /// Request is from previously unknown peer (protocol-observable)
    pub unknown_peer: bool,
    /// Peer was denied consent before (protocol-observable from history)
    pub denied_before: bool,
    /// Number of failed attempts from this peer (protocol-observable)
    pub failed_attempts: u32,
    /// Request triggered rate limiting (protocol-observable)
    pub rate_limited: bool,
    /// Request has invalid or suspicious fields (protocol-observable)
    pub suspicious: bool,
}

/// Extension point for custom policy evaluation
///
/// Implementations can inject additional decision logic based on protocol-observable
/// state. MUST NOT make decisions based on unverifiable claims or platform-specific identity.
///
/// # Example
///
/// ```ignore
/// struct StrictPolicy;
///
/// impl PolicyHook for StrictPolicy {
///     fn evaluate(&self, ctx: &ConsentRequestContext) -> Option<(PolicyDecision, PolicyReason)> {
///         // Deny all unknown peers in strict mode
///         if ctx.unknown_peer {
///             return Some((
///                 PolicyDecision::AutoDeny,
///                 PolicyReason::CustomPolicyRule {
///                     rule_id: "strict_mode".into(),
///                     reason: "Unknown peer denied in strict mode".into(),
///                 },
///             ));
///         }
///         None // Fall through to default HSIP policy
///     }
/// }
/// ```
pub trait PolicyHook: Send + Sync {
    /// Evaluate a consent request and optionally override default policy
    ///
    /// Returns `Some((decision, reason))` to override HSIP's default policy,
    /// or `None` to fall through to default behavior.
    ///
    /// # Contract
    ///
    /// - MUST base decisions only on protocol-observable state in `ConsentRequestContext`
    /// - MUST NOT make assumptions about platform identity
    /// - MUST NOT introduce unverifiable claims
    /// - SHOULD be fast (sub-millisecond) to avoid DoS amplification
    fn evaluate(&self, ctx: &ConsentRequestContext) -> Option<(PolicyDecision, PolicyReason)>;
}

/// Audit event for consent decisions
///
/// Logged to tamper-evident audit trail for litigation/evidence purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Event ID (UUID)
    pub event_id: String,
    /// Timestamp (RFC3339 or Unix epoch)
    pub timestamp: String,
    /// Actor peer ID (who made the decision)
    pub actor_peer_id: String,
    /// Subject peer ID (who was affected)
    pub subject_peer_id: String,
    /// Decision type (ALLOW, DENY, REVOKE, etc.)
    pub decision_type: String,
    /// Severity level (0-3)
    pub severity: u8,
    /// Machine-readable reason code
    pub reason_code: String,
    /// Human-readable reason text
    pub reason_text: String,
    /// Cryptographic evidence (e.g., signature hashes, packet hashes)
    pub evidence: Vec<Evidence>,
    /// TTL for this decision (if applicable)
    pub ttl_ms: Option<u64>,
    /// Previous event hash (for hash chaining)
    pub prev_hash: String,
    /// Ed25519 signature of this event (hex)
    pub signature: String,
}

/// Evidence attached to audit events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Evidence type (e.g., "signature_hash", "packet_hash", "request_cid")
    pub kind: String,
    /// Evidence value (e.g., "sha256:abc123...")
    pub value: String,
}

/// Extension point for custom audit sinks
///
/// Allows integrations to export audit events to external storage, databases,
/// or monitoring systems while preserving HSIP's tamper-evident log integrity.
///
/// # Example
///
/// ```ignore
/// struct DatabaseAuditSink {
///     db: Database,
/// }
///
/// impl AuditSink for DatabaseAuditSink {
///     fn log_event(&self, event: &AuditEvent) -> Result<(), String> {
///         // Write to database
///         self.db.insert("audit_log", event)
///             .map_err(|e| format!("DB error: {}", e))
///     }
///
///     fn verify_chain(&self, events: &[AuditEvent]) -> Result<bool, String> {
///         // Verify hash chain integrity
///         for i in 1..events.len() {
///             let prev = &events[i - 1];
///             let curr = &events[i];
///             let expected_hash = hash_event(prev);
///             if curr.prev_hash != expected_hash {
///                 return Ok(false); // Chain broken
///             }
///         }
///         Ok(true)
///     }
/// }
/// ```
pub trait AuditSink: Send + Sync {
    /// Log an audit event
    ///
    /// MUST preserve event integrity (hash chain, signatures).
    /// SHOULD be idempotent (duplicate event_id should not cause errors).
    ///
    /// # Errors
    ///
    /// Returns error if event cannot be logged (e.g., I/O failure, storage full).
    fn log_event(&self, event: &AuditEvent) -> Result<(), String>;

    /// Verify hash chain integrity of audit events
    ///
    /// MUST verify:
    /// - Each event's prev_hash matches hash(previous_event)
    /// - Signatures are valid for each event
    ///
    /// # Errors
    ///
    /// Returns error if verification cannot be performed.
    /// Returns Ok(false) if chain is broken or invalid.
    fn verify_chain(&self, events: &[AuditEvent]) -> Result<bool, String>;

    /// Export audit events with tamper-detection metadata
    ///
    /// MUST include:
    /// - Genesis hash (hash of first event)
    /// - Head hash (hash of last event)
    /// - Export counter (monotonic, detects selective exports)
    ///
    /// # Errors
    ///
    /// Returns error if export cannot be performed.
    fn export(&self) -> Result<AuditExport, String>;
}

/// Audit export with tamper-detection metadata
///
/// Includes genesis hash, head hash, and export counter to detect:
/// - Selective exports (missing events)
/// - Modified events (hash mismatch)
/// - Rolled-back logs (export counter decreases)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditExport {
    /// All audit events in chronological order
    pub events: Vec<AuditEvent>,
    /// Hash of first event (genesis)
    pub genesis_hash: String,
    /// Hash of last event (head)
    pub head_hash: String,
    /// Monotonic export counter (increments on each export)
    pub export_counter: u64,
    /// Verification hash: HMAC(genesis_hash || head_hash || export_counter, secret_key)
    pub verification_hash: String,
}

/// Opaque capability token for peer/session
///
/// Generic capability system - does NOT encode platform-specific identity.
/// Use for protocol-level capabilities only (e.g., "can send files", "can initiate video").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    /// Capability identifier (e.g., "file_transfer", "video_call")
    pub capability_id: String,
    /// Opaque token bytes (implementation-defined, typically signed JWT or similar)
    pub token: Vec<u8>,
    /// Expiration timestamp (Unix milliseconds)
    pub expires_ms: u64,
}

/// Extension point for capability provisioning
///
/// Allows integrations to issue generic capability tokens for peers/sessions.
/// MUST NOT encode platform-specific identity or unverifiable claims.
///
/// # Example
///
/// ```ignore
/// struct FileTransferCapabilities;
///
/// impl CapabilityProvider for FileTransferCapabilities {
///     fn capabilities_for_peer(&self, peer_id: &str) -> Vec<Capability> {
///         // Issue file transfer capability if peer is trusted
///         if is_trusted(peer_id) {
///             vec![Capability {
///                 capability_id: "file_transfer".into(),
///                 token: sign_token(peer_id, "file_transfer"),
///                 expires_ms: now() + 3600_000, // 1 hour
///             }]
///         } else {
///             vec![]
///         }
///     }
/// }
/// ```
pub trait CapabilityProvider: Send + Sync {
    /// Get capabilities for a peer
    ///
    /// Returns list of opaque capability tokens based on peer_id.
    /// MUST base decisions only on protocol-observable state.
    ///
    /// # Contract
    ///
    /// - MUST NOT encode platform-specific identity in tokens
    /// - MUST NOT make unverifiable claims
    /// - SHOULD return empty vec if no capabilities granted
    fn capabilities_for_peer(&self, peer_id: &str) -> Vec<Capability>;

    /// Verify a capability token
    ///
    /// Returns true if token is valid for the given peer and capability.
    ///
    /// # Errors
    ///
    /// Returns error if verification cannot be performed (e.g., malformed token).
    fn verify_capability(
        &self,
        peer_id: &str,
        capability_id: &str,
        token: &[u8],
    ) -> Result<bool, String>;
}

/// Default no-op policy hook (always falls through to HSIP defaults)
pub struct NoOpPolicyHook;

impl PolicyHook for NoOpPolicyHook {
    fn evaluate(&self, _ctx: &ConsentRequestContext) -> Option<(PolicyDecision, PolicyReason)> {
        None // Always fall through to default HSIP policy
    }
}

/// Default no-op audit sink (discards events)
pub struct NoOpAuditSink;

impl AuditSink for NoOpAuditSink {
    fn log_event(&self, _event: &AuditEvent) -> Result<(), String> {
        Ok(()) // Discard
    }

    fn verify_chain(&self, _events: &[AuditEvent]) -> Result<bool, String> {
        Ok(true) // No events to verify
    }

    fn export(&self) -> Result<AuditExport, String> {
        Ok(AuditExport {
            events: vec![],
            genesis_hash: "0".repeat(64),
            head_hash: "0".repeat(64),
            export_counter: 0,
            verification_hash: "0".repeat(64),
        })
    }
}

/// Default no-op capability provider (grants no capabilities)
pub struct NoOpCapabilityProvider;

impl CapabilityProvider for NoOpCapabilityProvider {
    fn capabilities_for_peer(&self, _peer_id: &str) -> Vec<Capability> {
        vec![] // No capabilities granted
    }

    fn verify_capability(
        &self,
        _peer_id: &str,
        _capability_id: &str,
        _token: &[u8],
    ) -> Result<bool, String> {
        Ok(false) // No capabilities granted, so all verifications fail
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_policy_hook() {
        let hook = NoOpPolicyHook;
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
        assert!(hook.evaluate(&ctx).is_none());
    }

    #[test]
    fn test_noop_audit_sink() {
        let sink = NoOpAuditSink;
        let event = AuditEvent {
            event_id: "test".into(),
            timestamp: "2026-01-17T00:00:00Z".into(),
            actor_peer_id: "actor".into(),
            subject_peer_id: "subject".into(),
            decision_type: "ALLOW".into(),
            severity: 0,
            reason_code: "test".into(),
            reason_text: "test".into(),
            evidence: vec![],
            ttl_ms: None,
            prev_hash: "0".repeat(64),
            signature: "0".repeat(128),
        };
        assert!(sink.log_event(&event).is_ok());
        assert!(sink.verify_chain(&[event]).unwrap());
    }

    #[test]
    fn test_noop_capability_provider() {
        let provider = NoOpCapabilityProvider;
        assert_eq!(provider.capabilities_for_peer("test").len(), 0);
        assert!(!provider
            .verify_capability("test", "file_transfer", &[])
            .unwrap());
    }
}
