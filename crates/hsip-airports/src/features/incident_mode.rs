// Incident Mode Policy - extends hsip-policy
// Tightened authorization rules during airport emergencies
// Requires multi-party approval for critical SCADA/ACS commands

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Incident mode policy for airport emergency scenarios
///
/// During cybersecurity incidents or physical emergencies, this policy
/// enforces stricter authorization requirements for critical systems.
///
/// TODO: Full implementation in next session
/// - Integrate with hsip-policy::Policy trait
/// - Multi-party approval workflow (operations manager + security officer)
/// - Time-limited emergency authorizations
/// - Audit trail for all incident-mode actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentModePolicy {
    /// Whether incident mode is currently active
    pub active: bool,

    /// When incident mode was activated
    pub activated_at: Option<DateTime<Utc>>,

    /// Incident identifier
    pub incident_id: Option<String>,

    /// Required approvers for critical actions
    pub required_approvers: Vec<String>,

    /// Minimum number of approvals needed
    pub approval_threshold: u8,
}

impl IncidentModePolicy {
    pub fn new() -> Self {
        Self {
            active: false,
            activated_at: None,
            incident_id: None,
            required_approvers: Vec::new(),
            approval_threshold: 2,
        }
    }

    /// Activate incident mode
    /// TODO: Implement multi-party activation
    pub fn activate(&mut self, incident_id: String, approvers: Vec<String>) {
        self.active = true;
        self.activated_at = Some(Utc::now());
        self.incident_id = Some(incident_id);
        self.required_approvers = approvers;
    }

    /// Deactivate incident mode
    /// TODO: Require sign-off from all approvers
    pub fn deactivate(&mut self) {
        self.active = false;
        self.activated_at = None;
        self.incident_id = None;
    }

    /// Check if action requires elevated approval
    /// TODO: Implement policy rules for SCADA, ACS, emergency systems
    pub fn requires_elevated_approval(&self, action: &str) -> bool {
        if !self.active {
            return false;
        }

        // TODO: Define critical action patterns
        action.contains("scada") || action.contains("emergency") || action.contains("shutdown")
    }
}

impl Default for IncidentModePolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incident_mode_activation() {
        let mut policy = IncidentModePolicy::new();
        assert!(!policy.active);

        policy.activate(
            "INC-2026-001".to_string(),
            vec!["ops_manager".to_string(), "security_officer".to_string()],
        );

        assert!(policy.active);
        assert_eq!(policy.incident_id, Some("INC-2026-001".to_string()));
        assert_eq!(policy.required_approvers.len(), 2);
    }

    #[test]
    fn test_elevated_approval_required() {
        let mut policy = IncidentModePolicy::new();
        policy.activate("INC-001".to_string(), vec!["user1".to_string()]);

        assert!(policy.requires_elevated_approval("scada_command"));
        assert!(policy.requires_elevated_approval("emergency_override"));
        assert!(!policy.requires_elevated_approval("read_logs"));
    }
}
