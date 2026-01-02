//! Consent Rules
//!
//! Advanced rule-based consent conditions

use chrono::Timelike;
use serde::{Deserialize, Serialize};

/// A consent rule with conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentRule {
    pub rule_id: String,
    pub name: String,
    pub message_type: String,
    pub conditions: Vec<RuleCondition>,
    pub enabled: bool,
}

/// Rule condition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RuleCondition {
    /// Maximum transaction amount (for financial messages)
    MaxAmount { currency: String, amount: i64 },

    /// Geographic restriction
    GeoRestriction { allowed_countries: Vec<String> },

    /// Time-of-day restriction (UTC hours)
    TimeOfDay { start_hour: u8, end_hour: u8 },

    /// IP address whitelist/blacklist
    IpFilter { whitelist: Vec<String> },

    /// Sender sub-identity must match
    RequireSubIdentity { sub_identity_name: String },

    /// Custom key-value condition
    Custom { key: String, value: serde_json::Value },
}

impl ConsentRule {
    /// Create a new rule
    pub fn new(rule_id: String, name: String, message_type: String) -> Self {
        Self {
            rule_id,
            name,
            message_type,
            conditions: Vec::new(),
            enabled: true,
        }
    }

    /// Add a condition to this rule
    pub fn add_condition(mut self, condition: RuleCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Evaluate rule against message context
    pub fn evaluate(&self, context: &MessageContext) -> bool {
        if !self.enabled {
            return false;
        }

        // All conditions must be satisfied
        self.conditions.iter().all(|cond| self.evaluate_condition(cond, context))
    }

    fn evaluate_condition(&self, condition: &RuleCondition, context: &MessageContext) -> bool {
        match condition {
            RuleCondition::MaxAmount { currency, amount } => {
                if let Some(msg_amount) = &context.amount {
                    msg_amount.currency == *currency && msg_amount.amount <= *amount
                } else {
                    false
                }
            }

            RuleCondition::GeoRestriction { allowed_countries } => {
                if let Some(country) = &context.origin_country {
                    allowed_countries.contains(country)
                } else {
                    false
                }
            }

            RuleCondition::TimeOfDay { start_hour, end_hour } => {
                let current_hour = chrono::Utc::now().hour() as u8;
                if start_hour < end_hour {
                    current_hour >= *start_hour && current_hour < *end_hour
                } else {
                    current_hour >= *start_hour || current_hour < *end_hour
                }
            }

            RuleCondition::IpFilter { whitelist } => {
                if let Some(ip) = &context.origin_ip {
                    whitelist.contains(ip)
                } else {
                    false
                }
            }

            RuleCondition::RequireSubIdentity { sub_identity_name } => {
                if let Some(sender) = &context.sender_full_name {
                    sender.ends_with(sub_identity_name)
                } else {
                    false
                }
            }

            RuleCondition::Custom { .. } => {
                // Custom conditions require application-specific logic
                true // Delegate to higher layer
            }
        }
    }
}

/// Message context for rule evaluation
#[derive(Debug, Clone)]
pub struct MessageContext {
    pub message_type: String,
    pub amount: Option<MoneyAmount>,
    pub origin_country: Option<String>,
    pub origin_ip: Option<String>,
    pub sender_full_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MoneyAmount {
    pub amount: i64,
    pub currency: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_amount_rule() {
        let rule = ConsentRule::new(
            "rule_001".to_string(),
            "Max transaction amount".to_string(),
            "transaction_alert".to_string(),
        ).add_condition(RuleCondition::MaxAmount {
            currency: "USD".to_string(),
            amount: 1000_00, // $1000.00
        });

        let context = MessageContext {
            message_type: "transaction_alert".to_string(),
            amount: Some(MoneyAmount {
                amount: 500_00,
                currency: "USD".to_string(),
            }),
            origin_country: None,
            origin_ip: None,
            sender_full_name: None,
        };

        assert!(rule.evaluate(&context));

        let context_too_high = MessageContext {
            amount: Some(MoneyAmount {
                amount: 2000_00,
                currency: "USD".to_string(),
            }),
            ..context
        };

        assert!(!rule.evaluate(&context_too_high));
    }

    #[test]
    fn test_geo_restriction() {
        let rule = ConsentRule::new(
            "rule_002".to_string(),
            "Geo restriction".to_string(),
            "transaction_alert".to_string(),
        ).add_condition(RuleCondition::GeoRestriction {
            allowed_countries: vec!["US".to_string(), "CA".to_string()],
        });

        let context_us = MessageContext {
            message_type: "transaction_alert".to_string(),
            amount: None,
            origin_country: Some("US".to_string()),
            origin_ip: None,
            sender_full_name: None,
        };

        assert!(rule.evaluate(&context_us));

        let context_ru = MessageContext {
            origin_country: Some("RU".to_string()),
            ..context_us
        };

        assert!(!rule.evaluate(&context_ru));
    }
}
