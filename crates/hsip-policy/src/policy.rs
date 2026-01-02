//! Consent Policy Definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A consent policy defining what messages a user allows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentPolicy {
    pub policy_id: String,
    pub user_id: String,
    pub sender_domain: String,
    pub scope: PolicyScope,
    pub time_window: TimeWindow,
    pub max_messages_per_day: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Policy scope (what message types are allowed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyScope {
    /// Allow all message types
    AllTypes,

    /// Allow specific message types only
    AllowedTypes(Vec<String>),

    /// Allow all except these types
    DeniedTypes(Vec<String>),

    /// Custom rules (more complex)
    CustomRules(Vec<String>), // Rule IDs
}

/// Time window for auto-accept
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeWindow {
    /// No time restriction (permanent until revoked)
    Permanent,

    /// Auto-accept until this date
    Until(DateTime<Utc>),

    /// Auto-accept for N days from creation
    Duration { days: u32 },

    /// Only during specific time-of-day (UTC hours)
    TimeOfDay { start_hour: u8, end_hour: u8 },
}

impl ConsentPolicy {
    /// Check if policy is currently valid
    pub fn is_valid(&self) -> bool {
        // Check if revoked
        if self.revoked_at.is_some() {
            return false;
        }

        // Check expiration
        if let Some(expires_at) = self.expires_at {
            if Utc::now() > expires_at {
                return false;
            }
        }

        true
    }

    /// Check if message type is allowed by this policy
    pub fn allows_type(&self, message_type: &str) -> bool {
        match &self.scope {
            PolicyScope::AllTypes => true,
            PolicyScope::AllowedTypes(allowed) => allowed.contains(&message_type.to_string()),
            PolicyScope::DeniedTypes(denied) => !denied.contains(&message_type.to_string()),
            PolicyScope::CustomRules(_) => {
                // Custom rules require engine evaluation
                true // Delegate to engine
            }
        }
    }

    /// Check if currently within time window
    pub fn in_time_window(&self) -> bool {
        let now = Utc::now();

        match &self.time_window {
            TimeWindow::Permanent => true,
            TimeWindow::Until(until) => now <= *until,
            TimeWindow::Duration { days } => {
                let expiry = self.created_at + chrono::Duration::days(*days as i64);
                now <= expiry
            }
            TimeWindow::TimeOfDay { start_hour, end_hour } => {
                let current_hour = now.hour() as u8;
                if start_hour < end_hour {
                    current_hour >= *start_hour && current_hour < *end_hour
                } else {
                    // Wraps around midnight
                    current_hour >= *start_hour || current_hour < *end_hour
                }
            }
        }
    }

    /// Revoke this policy
    pub fn revoke(&mut self) {
        self.revoked_at = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_scope() {
        let policy = ConsentPolicy {
            policy_id: "policy_001".to_string(),
            user_id: "user@example.com".to_string(),
            sender_domain: "bank.example.com".to_string(),
            scope: PolicyScope::AllowedTypes(vec!["transaction_alert".to_string()]),
            time_window: TimeWindow::Permanent,
            max_messages_per_day: Some(10),
            created_at: Utc::now(),
            expires_at: None,
            revoked_at: None,
        };

        assert!(policy.allows_type("transaction_alert"));
        assert!(!policy.allows_type("marketing"));
    }

    #[test]
    fn test_time_window() {
        let policy = ConsentPolicy {
            policy_id: "policy_001".to_string(),
            user_id: "user@example.com".to_string(),
            sender_domain: "bank.example.com".to_string(),
            scope: PolicyScope::AllTypes,
            time_window: TimeWindow::TimeOfDay {
                start_hour: 8,
                end_hour: 22,
            },
            max_messages_per_day: None,
            created_at: Utc::now(),
            expires_at: None,
            revoked_at: None,
        };

        // Time-of-day check depends on current time
        // Just verify it doesn't panic
        let _ = policy.in_time_window();
    }

    #[test]
    fn test_revocation() {
        let mut policy = ConsentPolicy {
            policy_id: "policy_001".to_string(),
            user_id: "user@example.com".to_string(),
            sender_domain: "bank.example.com".to_string(),
            scope: PolicyScope::AllTypes,
            time_window: TimeWindow::Permanent,
            max_messages_per_day: None,
            created_at: Utc::now(),
            expires_at: None,
            revoked_at: None,
        };

        assert!(policy.is_valid());

        policy.revoke();

        assert!(!policy.is_valid());
    }
}
