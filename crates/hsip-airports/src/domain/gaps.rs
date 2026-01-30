use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::domain::{AirportSystem, ThreatVector, Mitigation};
use crate::evidence::EvidenceRef;

/// Identified cybersecurity gap in airport infrastructure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceGap {
    /// Unique identifier (e.g., "GAP-AODB-001")
    pub gap_id: String,

    /// Short descriptive title
    pub title: String,

    /// Detailed description of the gap
    pub description: String,

    /// Category of gap
    pub category: GapCategory,

    /// Affected airport systems
    pub affected_systems: Vec<AirportSystem>,

    /// Threat vectors that exploit this gap
    pub threat_vectors: Vec<ThreatVector>,

    /// Current framework coverage (if any)
    pub current_coverage: Vec<FrameworkCoverage>,

    /// What specifically is NOT addressed by existing frameworks
    pub coverage_deficit: String,

    /// Severity assessment
    pub severity: Severity,

    /// Evidence supporting this gap identification
    pub evidence: Vec<EvidenceRef>,

    /// Recommended mitigation
    pub recommended_mitigation: Mitigation,

    /// Confidence score (0-100) based on evidence quality
    pub confidence: u8,

    /// When this gap was discovered/added to database
    pub discovered_at: DateTime<Utc>,

    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Category of cybersecurity gap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GapCategory {
    /// Regulatory gap - TSA/FAA/CISA mandates don't cover this
    Regulatory,

    /// Technical gap - known vulnerability, no standard addresses it
    Technical,

    /// Operational gap - standard exists but adoption is low
    Operational,

    /// Emerging threat - AI/supply chain/new attack vectors
    EmergingThreat,
}

/// Severity level of a gap
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Critical - immediate exploitation risk, safety impact possible
    Critical,

    /// High - significant operational risk, compliance violation
    High,

    /// Medium - limited impact, should be addressed in next cycle
    Medium,

    /// Low - informational, low priority
    Low,
}

impl Severity {
    /// Returns numeric score for sorting (higher = more severe)
    pub fn score(&self) -> u8 {
        match self {
            Self::Critical => 4,
            Self::High => 3,
            Self::Medium => 2,
            Self::Low => 1,
        }
    }

    /// Returns color code for display
    pub fn color(&self) -> &'static str {
        match self {
            Self::Critical => "red",
            Self::High => "orange",
            Self::Medium => "yellow",
            Self::Low => "blue",
        }
    }
}

/// Framework coverage for a specific gap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkCoverage {
    /// Framework name (e.g., "NIST CSF 2.0", "TSA SD 1580/82-2022-01")
    pub framework: String,

    /// Specific control/requirement ID (if applicable)
    pub control_id: Option<String>,

    /// Level of coverage this framework provides
    pub coverage_level: CoverageLevel,

    /// Notes on how this framework addresses (or doesn't address) the gap
    pub notes: String,
}

/// Level of coverage a framework provides
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoverageLevel {
    /// Framework fully addresses this gap
    Full,

    /// Framework mentions this area but lacks specific guidance
    Partial,

    /// Framework is tangentially related but not directly applicable
    Tangential,

    /// No coverage at all
    None,
}

impl CoverageLevel {
    /// Returns numeric score (higher = better coverage)
    pub fn score(&self) -> u8 {
        match self {
            Self::Full => 3,
            Self::Partial => 2,
            Self::Tangential => 1,
            Self::None => 0,
        }
    }
}

impl ComplianceGap {
    /// Creates a new gap with auto-generated ID
    pub fn new(
        title: String,
        description: String,
        category: GapCategory,
        affected_systems: Vec<AirportSystem>,
    ) -> Self {
        let gap_id = Self::generate_gap_id(&affected_systems, category);

        Self {
            gap_id,
            title,
            description,
            category,
            affected_systems,
            threat_vectors: Vec::new(),
            current_coverage: Vec::new(),
            coverage_deficit: String::new(),
            severity: Severity::Medium,
            evidence: Vec::new(),
            recommended_mitigation: Mitigation::default(),
            confidence: 50,
            discovered_at: Utc::now(),
            last_updated: Utc::now(),
        }
    }

    /// Generates gap ID based on affected systems and category
    fn generate_gap_id(systems: &[AirportSystem], category: GapCategory) -> String {
        use uuid::Uuid;

        let system_prefix = if systems.is_empty() {
            "GENERAL"
        } else {
            match systems[0] {
                AirportSystem::AODB => "AODB",
                AirportSystem::BHS => "BHS",
                AirportSystem::FIDS => "FIDS",
                AirportSystem::ACS => "ACS",
                AirportSystem::SCADA => "SCADA",
                AirportSystem::AirlineDataNetworks => "AIRLINE",
                AirportSystem::DCS_PSS => "DCS",
                AirportSystem::PublicWiFi => "WIFI",
                AirportSystem::CDM => "CDM",
                AirportSystem::FAA_SWIM => "FAA",
                AirportSystem::Surveillance => "SURV",
                AirportSystem::EmergencyComms => "EMRG",
                AirportSystem::Network => "NET",
                AirportSystem::VendorAccess => "VENDOR",
                AirportSystem::Monitoring => "MON",
            }
        };

        let category_prefix = match category {
            GapCategory::Regulatory => "REG",
            GapCategory::Technical => "TECH",
            GapCategory::Operational => "OPS",
            GapCategory::EmergingThreat => "EMRG",
        };

        format!("GAP-{}-{}-{}", system_prefix, category_prefix, Uuid::new_v4().simple().to_string()[..6].to_uppercase())
    }

    /// Calculates severity based on affected systems and coverage
    pub fn calculate_severity(&mut self) {
        let mut score = 0;

        // Factor 1: Affected systems
        for system in &self.affected_systems {
            if system.is_safety_critical() {
                score += 30;
            } else if system.is_operational_critical() {
                score += 20;
            } else if system.is_ot() {
                score += 15;
            } else {
                score += 10;
            }
        }

        // Factor 2: Coverage level (worse coverage = higher severity)
        let worst_coverage = self.current_coverage.iter()
            .map(|c| c.coverage_level.score())
            .min()
            .unwrap_or(0);

        score += match worst_coverage {
            0 => 25, // No coverage
            1 => 15, // Tangential
            2 => 5,  // Partial
            _ => 0,  // Full
        };

        // Factor 3: Evidence quality
        if !self.evidence.is_empty() {
            let avg_grade = self.evidence.iter()
                .map(|e| e.evidence_grade.score())
                .sum::<u8>() as f32 / self.evidence.len() as f32;

            if avg_grade >= 3.5 {
                score += 10; // High-quality evidence
            }
        }

        // Map score to severity
        self.severity = match score {
            80.. => Severity::Critical,
            50..=79 => Severity::High,
            25..=49 => Severity::Medium,
            _ => Severity::Low,
        };
    }

    /// Returns true if this gap is ready for export (has evidence and mitigation)
    pub fn is_complete(&self) -> bool {
        !self.evidence.is_empty() && !self.recommended_mitigation.actions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gap_creation() {
        let gap = ComplianceGap::new(
            "SCADA Network Segmentation".to_string(),
            "Lack of network isolation between SCADA and corporate IT".to_string(),
            GapCategory::Technical,
            vec![AirportSystem::SCADA],
        );

        assert!(gap.gap_id.starts_with("GAP-SCADA-TECH-"));
        assert_eq!(gap.severity, Severity::Medium);
    }

    #[test]
    fn test_severity_calculation() {
        let mut gap = ComplianceGap::new(
            "Test Gap".to_string(),
            "Test".to_string(),
            GapCategory::Technical,
            vec![AirportSystem::SCADA, AirportSystem::ACS],
        );

        gap.current_coverage.push(FrameworkCoverage {
            framework: "NIST CSF".to_string(),
            control_id: None,
            coverage_level: CoverageLevel::None,
            notes: "No coverage".to_string(),
        });

        gap.calculate_severity();

        // SCADA (30) + ACS (30) + No coverage (25) = 85 = Critical
        assert_eq!(gap.severity, Severity::Critical);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
    }
}
