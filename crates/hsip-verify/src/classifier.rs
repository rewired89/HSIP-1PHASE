//! Message Classifier
//!
//! Classifies messages and determines verification status for display

use crate::{MessageMetadata, VerificationResult};
use serde::{Deserialize, Serialize};

/// Classification result with display information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub verification: VerificationResult,
    pub display_info: DisplayInfo,
}

/// Display information for UI rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayInfo {
    pub indicator: String,  // "✅", "🔴", "❌"
    pub badge_color: BadgeColor,
    pub status_text: String,
    pub sender_verified: bool,
    pub show_warning: bool,
    pub warning_text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BadgeColor {
    Green,   // Verified
    Red,     // Unverified/Rejected
    Yellow,  // Warning
}

/// Message classifier
pub struct MessageClassifier;

impl MessageClassifier {
    /// Classify a message based on verification result
    pub fn classify(
        metadata: &MessageMetadata,
        verification: VerificationResult,
    ) -> ClassificationResult {
        let display_info = match verification {
            VerificationResult::Verified => DisplayInfo {
                indicator: "✅".to_string(),
                badge_color: BadgeColor::Green,
                status_text: "VERIFIED MESSAGE".to_string(),
                sender_verified: true,
                show_warning: false,
                warning_text: None,
            },

            VerificationResult::Unverified => {
                let warning = format!(
                    "This message from {} could not be verified. It may not be legitimate.",
                    metadata.sender_domain
                );

                DisplayInfo {
                    indicator: "🔴".to_string(),
                    badge_color: BadgeColor::Red,
                    status_text: "UNVERIFIED MESSAGE".to_string(),
                    sender_verified: false,
                    show_warning: true,
                    warning_text: Some(warning),
                }
            }

            VerificationResult::Rejected(reason) => {
                let warning = format!(
                    "This message was rejected: {:?}",
                    reason
                );

                DisplayInfo {
                    indicator: "❌".to_string(),
                    badge_color: BadgeColor::Red,
                    status_text: "REJECTED MESSAGE".to_string(),
                    sender_verified: false,
                    show_warning: true,
                    warning_text: Some(warning),
                }
            }
        };

        ClassificationResult {
            verification,
            display_info,
        }
    }

    /// Determine if message should be shown to user
    pub fn should_display(result: &ClassificationResult) -> bool {
        match result.verification {
            VerificationResult::Verified => true,
            VerificationResult::Unverified => true, // Show with warning
            VerificationResult::Rejected(_) => false, // Silent drop
        }
    }

    /// Get urgency level for notification
    pub fn urgency_level(metadata: &MessageMetadata, result: &ClassificationResult) -> UrgencyLevel {
        use crate::message_types::{MessageType, Severity};

        if !result.verification.is_verified() {
            return UrgencyLevel::Low;
        }

        match &metadata.message_type {
            MessageType::SecurityAlert { severity, .. } => match severity {
                Severity::Critical => UrgencyLevel::Critical,
                Severity::High => UrgencyLevel::High,
                Severity::Medium => UrgencyLevel::Medium,
                Severity::Low => UrgencyLevel::Low,
            },
            MessageType::TransactionAlert { .. } => UrgencyLevel::High,
            MessageType::LoginNotice { suspicious, .. } => {
                if *suspicious {
                    UrgencyLevel::High
                } else {
                    UrgencyLevel::Medium
                }
            }
            MessageType::ComplianceNotice { .. } => UrgencyLevel::Medium,
            MessageType::Marketing { .. } => UrgencyLevel::Low,
            _ => UrgencyLevel::Medium,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UrgencyLevel {
    Critical,  // Immediate action required
    High,      // Action required soon
    Medium,    // Informational, important
    Low,       // Informational, not urgent
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message_types::{MessageType, Money};
    use chrono::Utc;

    #[test]
    fn test_classifier_verified() {
        let metadata = MessageMetadata {
            message_id: "msg_001".to_string(),
            sender_domain: "bank.example.com".to_string(),
            recipient: "user@example.com".to_string(),
            message_type: MessageType::TransactionAlert {
                amount: Money::usd(100, 0),
                merchant: "Amazon".to_string(),
                transaction_id: "txn_001".to_string(),
                timestamp: Utc::now(),
                card_last_four: "1234".to_string(),
            },
            sent_at: Utc::now(),
            expires_at: None,
            nonce: [0u8; 16],
        };

        let result = MessageClassifier::classify(&metadata, VerificationResult::Verified);

        assert!(result.display_info.sender_verified);
        assert_eq!(result.display_info.badge_color, BadgeColor::Green);
        assert!(!result.display_info.show_warning);
        assert!(MessageClassifier::should_display(&result));
    }

    #[test]
    fn test_classifier_unverified() {
        let metadata = MessageMetadata {
            message_id: "msg_001".to_string(),
            sender_domain: "suspicious.com".to_string(),
            recipient: "user@example.com".to_string(),
            message_type: MessageType::TransactionAlert {
                amount: Money::usd(100, 0),
                merchant: "Amazon".to_string(),
                transaction_id: "txn_001".to_string(),
                timestamp: Utc::now(),
                card_last_four: "1234".to_string(),
            },
            sent_at: Utc::now(),
            expires_at: None,
            nonce: [0u8; 16],
        };

        let result = MessageClassifier::classify(&metadata, VerificationResult::Unverified);

        assert!(!result.display_info.sender_verified);
        assert_eq!(result.display_info.badge_color, BadgeColor::Red);
        assert!(result.display_info.show_warning);
        assert!(MessageClassifier::should_display(&result));
    }
}
