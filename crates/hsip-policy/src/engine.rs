//! Consent Policy Engine
//!
//! Main engine for evaluating consent policies

use crate::{ConsentPolicy, ConsentProof, ConsentRule, MessageContext, PolicyError, Result};
use std::collections::HashMap;

/// Policy check result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyCheckResult {
    Allowed,
    Denied,
    RequiresUserPrompt,
}

/// Consent Policy Engine
pub struct ConsentPolicyEngine {
    policies: HashMap<String, ConsentPolicy>, // user_id + sender -> policy
    rules: HashMap<String, ConsentRule>,
    consent_proofs: Vec<ConsentProof>,
}

impl ConsentPolicyEngine {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            rules: HashMap::new(),
            consent_proofs: Vec::new(),
        }
    }

    /// Add a consent policy
    pub fn add_policy(&mut self, policy: ConsentPolicy) {
        let key = Self::policy_key(&policy.user_id, &policy.sender_domain);
        self.policies.insert(key, policy);
    }

    /// Remove a consent policy
    pub fn remove_policy(&mut self, user_id: &str, sender_domain: &str) {
        let key = Self::policy_key(user_id, sender_domain);
        self.policies.remove(&key);
    }

    /// Get a policy
    pub fn get_policy(&self, user_id: &str, sender_domain: &str) -> Option<&ConsentPolicy> {
        let key = Self::policy_key(user_id, sender_domain);
        self.policies.get(&key)
    }

    /// Add a consent rule
    pub fn add_rule(&mut self, rule: ConsentRule) {
        self.rules.insert(rule.rule_id.clone(), rule);
    }

    /// Check if a message is allowed by consent policy
    pub fn check_consent(
        &self,
        user_id: &str,
        sender_domain: &str,
        message_type: &str,
        context: &MessageContext,
    ) -> PolicyCheckResult {
        // Get policy for this user+sender combination
        let policy = match self.get_policy(user_id, sender_domain) {
            Some(p) => p,
            None => return PolicyCheckResult::RequiresUserPrompt,
        };

        // Check if policy is valid
        if !policy.is_valid() {
            return PolicyCheckResult::RequiresUserPrompt;
        }

        // Check if within time window
        if !policy.in_time_window() {
            return PolicyCheckResult::Denied;
        }

        // Check if message type is allowed
        if !policy.allows_type(message_type) {
            return PolicyCheckResult::Denied;
        }

        // Evaluate custom rules if any
        if let crate::PolicyScope::CustomRules(rule_ids) = &policy.scope {
            for rule_id in rule_ids {
                if let Some(rule) = self.rules.get(rule_id) {
                    if !rule.evaluate(context) {
                        return PolicyCheckResult::Denied;
                    }
                }
            }
        }

        PolicyCheckResult::Allowed
    }

    /// Store a consent proof
    pub fn store_proof(&mut self, proof: ConsentProof) -> Result<()> {
        proof.verify()?;
        self.consent_proofs.push(proof);
        Ok(())
    }

    /// Get all consent proofs for a user
    pub fn get_proofs_for_user(&self, user_id: &str) -> Vec<&ConsentProof> {
        self.consent_proofs
            .iter()
            .filter(|p| {
                // Match by public key (we'd need to map user_id -> public_key in production)
                // For now, just return all proofs
                true
            })
            .collect()
    }

    /// Revoke a policy
    pub fn revoke_policy(&mut self, user_id: &str, sender_domain: &str) -> Result<()> {
        let key = Self::policy_key(user_id, sender_domain);
        let policy = self.policies.get_mut(&key)
            .ok_or_else(|| PolicyError::PolicyNotFound(key.clone()))?;

        policy.revoke();
        Ok(())
    }

    fn policy_key(user_id: &str, sender_domain: &str) -> String {
        format!("{}::{}", user_id, sender_domain)
    }

    /// Count active policies
    pub fn active_policy_count(&self) -> usize {
        self.policies.values().filter(|p| p.is_valid()).count()
    }

    /// Count total consent proofs
    pub fn proof_count(&self) -> usize {
        self.consent_proofs.len()
    }
}

impl Default for ConsentPolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PolicyScope, TimeWindow};
    use chrono::Utc;

    #[test]
    fn test_policy_engine() {
        let mut engine = ConsentPolicyEngine::new();

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

        engine.add_policy(policy);

        let context = MessageContext {
            message_type: "transaction_alert".to_string(),
            amount: None,
            origin_country: None,
            origin_ip: None,
            sender_full_name: None,
        };

        let result = engine.check_consent(
            "user@example.com",
            "bank.example.com",
            "transaction_alert",
            &context,
        );

        assert_eq!(result, PolicyCheckResult::Allowed);
    }

    #[test]
    fn test_policy_revocation() {
        let mut engine = ConsentPolicyEngine::new();

        let policy = ConsentPolicy {
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

        engine.add_policy(policy);
        assert_eq!(engine.active_policy_count(), 1);

        engine.revoke_policy("user@example.com", "bank.example.com").unwrap();
        assert_eq!(engine.active_policy_count(), 0);
    }
}
