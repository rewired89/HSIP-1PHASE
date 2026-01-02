//! Message Rejection Logic
//!
//! Handles rejection of invalid, suspicious, or policy-violating messages

use serde::{Deserialize, Serialize};

/// Reasons for message rejection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejectionReason {
    NoSignature,
    InvalidSignature,
    CertChainBroken,
    RevokedKey,
    ConsentDenied,
    PolicyViolation,
    RateLimitExceeded,
    SuspiciousBehavior,
    MessageExpired,
    UntrustedSender,
}

/// Action to take on rejection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejectionAction {
    SilentDrop,           // Drop without notification
    ShowWarning,          // Show "UNVERIFIED" warning
    QuarantineForReview,  // Hold for manual review
}

/// Rejection handler configuration
pub struct RejectionHandler {
    config: RejectionConfig,
}

#[derive(Debug, Clone)]
pub struct RejectionConfig {
    pub log_rejections: bool,
    pub notify_security_team: bool,
    pub default_action: RejectionAction,
}

impl Default for RejectionConfig {
    fn default() -> Self {
        Self {
            log_rejections: true,
            notify_security_team: false,
            default_action: RejectionAction::SilentDrop,
        }
    }
}

impl RejectionHandler {
    pub fn new(config: RejectionConfig) -> Self {
        Self { config }
    }

    /// Determine action for a rejection reason
    pub fn action_for_reason(&self, reason: RejectionReason) -> RejectionAction {
        match reason {
            // Cryptographic failures: silent drop
            RejectionReason::NoSignature => RejectionAction::SilentDrop,
            RejectionReason::InvalidSignature => RejectionAction::SilentDrop,
            RejectionReason::CertChainBroken => RejectionAction::SilentDrop,
            RejectionReason::UntrustedSender => RejectionAction::SilentDrop,

            // Revoked/expired: silent drop but log
            RejectionReason::RevokedKey => RejectionAction::SilentDrop,
            RejectionReason::MessageExpired => RejectionAction::SilentDrop,

            // Policy/consent: show warning (user might want to know)
            RejectionReason::ConsentDenied => RejectionAction::ShowWarning,
            RejectionReason::PolicyViolation => RejectionAction::ShowWarning,

            // Rate limit: show warning
            RejectionReason::RateLimitExceeded => RejectionAction::ShowWarning,

            // Suspicious: quarantine for review
            RejectionReason::SuspiciousBehavior => RejectionAction::QuarantineForReview,
        }
    }

    /// Check if rejection should be logged
    pub fn should_log(&self, reason: RejectionReason) -> bool {
        self.config.log_rejections || matches!(
            reason,
            RejectionReason::SuspiciousBehavior
                | RejectionReason::InvalidSignature
                | RejectionReason::RevokedKey
        )
    }

    /// Check if security team should be notified
    pub fn should_notify_security(&self, reason: RejectionReason) -> bool {
        self.config.notify_security_team && matches!(
            reason,
            RejectionReason::SuspiciousBehavior | RejectionReason::RevokedKey
        )
    }

    /// Get user-friendly rejection message
    pub fn user_message(&self, reason: RejectionReason) -> Option<&'static str> {
        match self.action_for_reason(reason) {
            RejectionAction::SilentDrop => None,
            RejectionAction::ShowWarning => Some(match reason {
                RejectionReason::ConsentDenied => {
                    "You have not granted permission for this type of message."
                }
                RejectionReason::PolicyViolation => {
                    "This message violates your security policy."
                }
                RejectionReason::RateLimitExceeded => {
                    "Too many messages from this sender. Some messages were blocked."
                }
                _ => "This message could not be verified.",
            }),
            RejectionAction::QuarantineForReview => Some(
                "This message has been flagged for security review."
            ),
        }
    }
}

impl Default for RejectionHandler {
    fn default() -> Self {
        Self::new(RejectionConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejection_actions() {
        let handler = RejectionHandler::default();

        assert_eq!(
            handler.action_for_reason(RejectionReason::NoSignature),
            RejectionAction::SilentDrop
        );

        assert_eq!(
            handler.action_for_reason(RejectionReason::ConsentDenied),
            RejectionAction::ShowWarning
        );

        assert_eq!(
            handler.action_for_reason(RejectionReason::SuspiciousBehavior),
            RejectionAction::QuarantineForReview
        );
    }

    #[test]
    fn test_user_messages() {
        let handler = RejectionHandler::default();

        assert!(handler.user_message(RejectionReason::NoSignature).is_none());
        assert!(handler.user_message(RejectionReason::ConsentDenied).is_some());
    }

    #[test]
    fn test_logging_decisions() {
        let mut config = RejectionConfig::default();
        config.log_rejections = true;

        let handler = RejectionHandler::new(config);

        assert!(handler.should_log(RejectionReason::InvalidSignature));
        assert!(handler.should_log(RejectionReason::SuspiciousBehavior));
    }
}
