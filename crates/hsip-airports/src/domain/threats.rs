use serde::{Deserialize, Serialize};
use crate::domain::AirportSystem;

/// Threat vector specific to airport infrastructure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatVector {
    /// Unique threat ID
    pub id: String,

    /// Threat name
    pub name: String,

    /// Detailed description
    pub description: String,

    /// MITRE ATT&CK for ICS technique IDs (if applicable)
    pub mitre_attack_ids: Vec<String>,

    /// Affected airport systems
    pub affected_systems: Vec<AirportSystem>,

    /// Known attack paths
    pub attack_paths: Vec<AttackPath>,

    /// Likelihood of exploitation
    pub likelihood: Likelihood,

    /// Potential impact if exploited
    pub impact: Impact,

    /// Known CVEs associated with this threat
    pub cves: Vec<String>,

    /// Real-world incidents (if any)
    pub incidents: Vec<String>,
}

/// Attack path from entry point to target system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackPath {
    /// Entry point (e.g., "Public WiFi", "Phishing email")
    pub entry_point: String,

    /// Lateral movement mechanism
    pub pivot_mechanism: String,

    /// Target system
    pub target: AirportSystem,

    /// Step-by-step attack sequence
    pub steps: Vec<String>,

    /// Preconditions required for this attack
    pub preconditions: Vec<String>,

    /// Detection difficulty (low/medium/high)
    pub detection_difficulty: String,
}

/// Likelihood of threat exploitation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Likelihood {
    /// Active exploitation in the wild
    Critical,

    /// Known vulnerabilities, public exploits exist
    High,

    /// Theoretical but plausible, no public exploits yet
    Medium,

    /// Requires significant resources/insider access
    Low,
}

/// Impact if threat is successfully exploited
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Impact {
    /// Loss of life, major safety incident, complete operational shutdown
    Critical,

    /// Significant operational disruption, major financial loss
    High,

    /// Limited operational impact, data breach
    Medium,

    /// Nuisance only, minimal impact
    Low,
}

impl ThreatVector {
    /// Creates a new threat vector
    pub fn new(id: String, name: String, description: String) -> Self {
        Self {
            id,
            name,
            description,
            mitre_attack_ids: Vec::new(),
            affected_systems: Vec::new(),
            attack_paths: Vec::new(),
            likelihood: Likelihood::Medium,
            impact: Impact::Medium,
            cves: Vec::new(),
            incidents: Vec::new(),
        }
    }

    /// Calculates risk score (likelihood × impact)
    pub fn risk_score(&self) -> u8 {
        let likelihood_score = match self.likelihood {
            Likelihood::Critical => 4,
            Likelihood::High => 3,
            Likelihood::Medium => 2,
            Likelihood::Low => 1,
        };

        let impact_score = match self.impact {
            Impact::Critical => 4,
            Impact::High => 3,
            Impact::Medium => 2,
            Impact::Low => 1,
        };

        likelihood_score * impact_score
    }

    /// Returns true if this threat has real-world precedent
    pub fn has_precedent(&self) -> bool {
        !self.incidents.is_empty() || !self.cves.is_empty()
    }
}

impl AttackPath {
    /// Creates a new attack path
    pub fn new(
        entry_point: String,
        pivot_mechanism: String,
        target: AirportSystem,
    ) -> Self {
        Self {
            entry_point,
            pivot_mechanism,
            target,
            steps: Vec::new(),
            preconditions: Vec::new(),
            detection_difficulty: "medium".to_string(),
        }
    }

    /// Adds a step to the attack path
    pub fn add_step(mut self, step: String) -> Self {
        self.steps.push(step);
        self
    }

    /// Adds a precondition
    pub fn add_precondition(mut self, precondition: String) -> Self {
        self.preconditions.push(precondition);
        self
    }

    /// Sets detection difficulty
    pub fn set_detection_difficulty(mut self, difficulty: &str) -> Self {
        self.detection_difficulty = difficulty.to_string();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_score_calculation() {
        let threat = ThreatVector {
            id: "T001".to_string(),
            name: "Test Threat".to_string(),
            description: "Test".to_string(),
            mitre_attack_ids: vec![],
            affected_systems: vec![],
            attack_paths: vec![],
            likelihood: Likelihood::High, // 3
            impact: Impact::Critical,     // 4
            cves: vec![],
            incidents: vec![],
        };

        assert_eq!(threat.risk_score(), 12); // 3 × 4
    }

    #[test]
    fn test_attack_path_builder() {
        let path = AttackPath::new(
            "Public WiFi".to_string(),
            "VPN credential theft".to_string(),
            AirportSystem::SCADA,
        )
        .add_step("Connect to passenger WiFi".to_string())
        .add_step("Perform ARP spoofing to intercept VPN credentials".to_string())
        .add_step("Use stolen credentials to access SCADA network".to_string())
        .add_precondition("Weak WiFi network segmentation".to_string())
        .set_detection_difficulty("high");

        assert_eq!(path.steps.len(), 3);
        assert_eq!(path.preconditions.len(), 1);
        assert_eq!(path.detection_difficulty, "high");
    }

    #[test]
    fn test_has_precedent() {
        let mut threat = ThreatVector::new(
            "T001".to_string(),
            "Ransomware".to_string(),
            "BHS ransomware attack".to_string(),
        );

        assert!(!threat.has_precedent());

        threat.incidents.push("Atlanta Airport 2022".to_string());
        assert!(threat.has_precedent());
    }
}
