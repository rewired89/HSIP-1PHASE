/// Consent policy evaluation for user-facing prompts and automated blocking
///
/// This module implements policy-based consent evaluation, supporting:
/// - Auto-deny based on protocol-level observable behavior
/// - Queueing for user review
/// - Auto-accept for previously granted consent

use crate::consent::ConsentRequestMetadata;
use serde::{Deserialize, Serialize};

/// Policy decision outcome
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

/// Reason code for policy decisions (logged for audit)
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
    /// Custom policy rule matched
    CustomPolicyRule { rule_id: String },
}

/// User-configurable consent policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentPolicy {
    /// Auto-deny all unknown peers (strict mode)
    pub deny_unknown_peers: bool,
    /// Maximum failed attempts before auto-deny
    pub max_failed_attempts: u32,
    /// Auto-deny peers previously denied (no retry)
    pub deny_previously_denied: bool,
}

impl Default for ConsentPolicy {
    fn default() -> Self {
        Self {
            // Default: queue unknown peers for review (not auto-deny)
            deny_unknown_peers: false,
            // Default: deny after 5 failed attempts
            max_failed_attempts: 5,
            // Default: allow retry after denial
            deny_previously_denied: false,
        }
    }
}

impl ConsentPolicy {
    /// Evaluate consent request against policy
    ///
    /// Returns (PolicyDecision, PolicyReason) tuple.
    /// Decision determines how system handles request.
    /// Reason is logged for audit trail.
    pub fn evaluate(&self, metadata: &ConsentRequestMetadata) -> (PolicyDecision, PolicyReason) {
        // Check for suspicious or malformed requests (silent reject)
        if metadata.flags.suspicious {
            return (PolicyDecision::SilentReject, PolicyReason::SuspiciousRequest);
        }

        // Check rate limiting (auto-deny)
        if metadata.flags.rate_limited {
            return (
                PolicyDecision::AutoDeny,
                PolicyReason::RateLimitExceeded,
            );
        }

        // Check attempt count (possible harassment)
        if metadata.flags.failed_attempts >= self.max_failed_attempts {
            return (
                PolicyDecision::AutoDeny,
                PolicyReason::TooManyAttempts {
                    count: metadata.flags.failed_attempts,
                },
            );
        }

        // Check previous denial policy
        if self.deny_previously_denied && metadata.flags.denied_before {
            return (
                PolicyDecision::AutoDeny,
                PolicyReason::PreviouslyDenied,
            );
        }

        // Check unknown peer policy
        if self.deny_unknown_peers && metadata.flags.unknown_peer {
            return (PolicyDecision::AutoDeny, PolicyReason::UnknownPeer);
        }

        // If peer is known and previously granted consent (checked elsewhere),
        // decision would be AutoAccept. That check happens in consent cache layer.
        // Here, we only evaluate policy rules.

        // Default: Queue for user review (unknown peer, no policy violation)
        if metadata.flags.unknown_peer {
            return (PolicyDecision::QueueForReview, PolicyReason::UnknownPeer);
        }

        // Should not reach here in normal flow, but default to review
        (PolicyDecision::QueueForReview, PolicyReason::UnknownPeer)
    }

    /// Create strict policy (deny all unknown peers)
    pub fn strict() -> Self {
        Self {
            deny_unknown_peers: true,
            max_failed_attempts: 3,
            deny_previously_denied: true,
        }
    }

    /// Create permissive policy (queue all unknown peers for review)
    pub fn permissive() -> Self {
        Self {
            deny_unknown_peers: false,
            max_failed_attempts: 10,
            deny_previously_denied: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consent::ConsentRequestFlags;

    #[test]
    fn test_default_policy_queues_unknown() {
        let policy = ConsentPolicy::default();
        let metadata = ConsentRequestMetadata {
            peer_id: "test_peer".into(),
            purpose: "test".into(),
            timestamp_ms: 1000,
            flags: ConsentRequestFlags {
                unknown_peer: true,
                ..Default::default()
            },
        };

        let (decision, reason) = policy.evaluate(&metadata);
        assert_eq!(decision, PolicyDecision::QueueForReview);
        matches!(reason, PolicyReason::UnknownPeer);
    }

    #[test]
    fn test_rate_limit_auto_denies() {
        let policy = ConsentPolicy::default();
        let metadata = ConsentRequestMetadata {
            peer_id: "test_peer".into(),
            purpose: "test".into(),
            timestamp_ms: 1000,
            flags: ConsentRequestFlags {
                rate_limited: true,
                ..Default::default()
            },
        };

        let (decision, reason) = policy.evaluate(&metadata);
        assert_eq!(decision, PolicyDecision::AutoDeny);
        matches!(reason, PolicyReason::RateLimitExceeded);
    }

    #[test]
    fn test_too_many_attempts_auto_denies() {
        let policy = ConsentPolicy::default();
        let metadata = ConsentRequestMetadata {
            peer_id: "test_peer".into(),
            purpose: "test".into(),
            timestamp_ms: 1000,
            flags: ConsentRequestFlags {
                failed_attempts: 10,
                ..Default::default()
            },
        };

        let (decision, reason) = policy.evaluate(&metadata);
        assert_eq!(decision, PolicyDecision::AutoDeny);
        match reason {
            PolicyReason::TooManyAttempts { count } => assert_eq!(count, 10),
            _ => panic!("Expected TooManyAttempts reason"),
        }
    }

    #[test]
    fn test_strict_policy_denies_unknown() {
        let policy = ConsentPolicy::strict();
        let metadata = ConsentRequestMetadata {
            peer_id: "test_peer".into(),
            purpose: "test".into(),
            timestamp_ms: 1000,
            flags: ConsentRequestFlags {
                unknown_peer: true,
                ..Default::default()
            },
        };

        let (decision, reason) = policy.evaluate(&metadata);
        assert_eq!(decision, PolicyDecision::AutoDeny);
        matches!(reason, PolicyReason::UnknownPeer);
    }

    #[test]
    fn test_suspicious_request_silent_reject() {
        let policy = ConsentPolicy::default();
        let metadata = ConsentRequestMetadata {
            peer_id: "test_peer".into(),
            purpose: "test".into(),
            timestamp_ms: 1000,
            flags: ConsentRequestFlags {
                suspicious: true,
                ..Default::default()
            },
        };

        let (decision, reason) = policy.evaluate(&metadata);
        assert_eq!(decision, PolicyDecision::SilentReject);
        matches!(reason, PolicyReason::SuspiciousRequest);
    }
}
