//! Enterprise Message Types
//!
//! Defines structured message types for banking, healthcare, and other industries

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Enterprise message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MessageType {
    /// Bank transaction alert
    TransactionAlert {
        amount: Money,
        merchant: String,
        transaction_id: String,
        timestamp: DateTime<Utc>,
        card_last_four: String,
    },

    /// Login notification
    LoginNotice {
        ip_address: String,
        location: String,
        device: String,
        timestamp: DateTime<Utc>,
        suspicious: bool,
    },

    /// Security alert
    SecurityAlert {
        severity: Severity,
        description: String,
        action_required: bool,
        deadline: Option<DateTime<Utc>>,
    },

    /// Compliance notice
    ComplianceNotice {
        regulation: String,
        description: String,
        deadline: DateTime<Utc>,
        document_url: Option<String>,
    },

    /// Marketing message (requires explicit consent)
    Marketing {
        campaign_id: String,
        content: String,
        opt_out_url: String,
    },

    /// Healthcare appointment reminder
    HealthcareAppointment {
        provider: String,
        appointment_time: DateTime<Utc>,
        location: String,
        appointment_type: String,
    },

    /// Payment confirmation
    PaymentConfirmation {
        amount: Money,
        recipient: String,
        payment_id: String,
        timestamp: DateTime<Utc>,
    },

    /// Account update notification
    AccountUpdate {
        update_type: String,
        description: String,
        effective_date: DateTime<Utc>,
    },

    /// Custom message (extensible)
    Custom {
        type_name: String,
        data: serde_json::Value,
    },
}

/// Money amount
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Money {
    pub amount: i64,  // in cents
    pub currency: String,  // ISO 4217
}

impl Money {
    pub fn usd(dollars: i64, cents: i64) -> Self {
        Self {
            amount: dollars * 100 + cents,
            currency: "USD".to_string(),
        }
    }

    pub fn format(&self) -> String {
        let dollars = self.amount / 100;
        let cents = self.amount.abs() % 100;
        format!("{}.{:02} {}", dollars, cents, self.currency)
    }
}

/// Severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Message metadata (wraps the typed message with envelope data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub message_id: String,
    pub sender_domain: String,
    pub recipient: String,
    pub message_type: MessageType,
    pub sent_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub nonce: [u8; 16],
}

impl MessageMetadata {
    /// Get the type name as string
    pub fn type_name(&self) -> &'static str {
        match &self.message_type {
            MessageType::TransactionAlert { .. } => "transaction_alert",
            MessageType::LoginNotice { .. } => "login_notice",
            MessageType::SecurityAlert { .. } => "security_alert",
            MessageType::ComplianceNotice { .. } => "compliance_notice",
            MessageType::Marketing { .. } => "marketing",
            MessageType::HealthcareAppointment { .. } => "healthcare_appointment",
            MessageType::PaymentConfirmation { .. } => "payment_confirmation",
            MessageType::AccountUpdate { .. } => "account_update",
            MessageType::Custom { .. } => "custom",
        }
    }

    /// Check if message is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_money_formatting() {
        let money = Money::usd(1234, 56);
        assert_eq!(money.format(), "1234.56 USD");
    }

    #[test]
    fn test_message_type_name() {
        let msg = MessageMetadata {
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

        assert_eq!(msg.type_name(), "transaction_alert");
    }
}
