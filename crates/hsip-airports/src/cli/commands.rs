use crate::domain::{AirportSystem, GapCategory, Severity};

/// Airport cybersecurity CLI commands
#[derive(Debug, Clone)]
pub enum AirportsCommand {
    /// Run gap analysis
    Gap(GapCommand),

    /// Manage evidence sources
    Evidence(EvidenceCommand),

    /// Use airport-specific features
    Features(FeaturesCommand),
}

/// Gap analysis commands
#[derive(Debug, Clone)]
pub enum GapCommand {
    /// Analyze specific airport systems
    Analyze {
        /// Systems to analyze (empty = all)
        systems: Vec<AirportSystem>,
        /// Output format (json, markdown, both)
        output: OutputFormat,
    },

    /// List existing gaps
    List {
        /// Filter by severity
        severity: Option<Severity>,
        /// Filter by category
        category: Option<GapCategory>,
    },

    /// Export gap report
    Export {
        /// Gap ID to export
        gap_id: String,
        /// Export format
        format: OutputFormat,
    },
}

/// Evidence collection commands
#[derive(Debug, Clone)]
pub enum EvidenceCommand {
    /// Fetch evidence from sources
    Fetch {
        /// Source IDs to fetch (empty = all known sources)
        sources: Vec<String>,
    },

    /// List collected evidence
    List,

    /// Verify evidence integrity
    Verify {
        /// Source ID to verify
        source_id: String,
    },
}

/// Airport-specific features
#[derive(Debug, Clone)]
pub enum FeaturesCommand {
    /// Incident mode management
    IncidentMode {
        action: IncidentAction,
    },

    /// Audit bundle operations
    AuditBundle {
        action: AuditAction,
    },
}

#[derive(Debug, Clone)]
pub enum IncidentAction {
    /// Activate incident mode
    Activate {
        incident_id: String,
        approvers: Vec<String>,
    },

    /// Deactivate incident mode
    Deactivate,

    /// Show current status
    Status,
}

#[derive(Debug, Clone)]
pub enum AuditAction {
    /// Create new audit bundle
    Create {
        bundle_id: String,
        compliance_tags: Vec<String>,
    },

    /// Seal bundle with signature
    Seal {
        bundle_id: String,
    },

    /// Verify bundle integrity
    Verify {
        bundle_id: String,
        public_key: String,
    },

    /// Export to SIEM format
    Export {
        bundle_id: String,
        format: SiemFormat,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Json,
    Markdown,
    Both,
}

#[derive(Debug, Clone, Copy)]
pub enum SiemFormat {
    CEF,  // Common Event Format
    LEEF, // Log Event Extended Format
}

// TODO: Implement clap derive macros when integrating with hsip-cli
// For now, these are plain enums that can be constructed programmatically

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_construction() {
        let cmd = AirportsCommand::Gap(GapCommand::Analyze {
            systems: vec![AirportSystem::SCADA],
            output: OutputFormat::Json,
        });

        match cmd {
            AirportsCommand::Gap(GapCommand::Analyze { systems, .. }) => {
                assert_eq!(systems.len(), 1);
            }
            _ => panic!("Wrong command variant"),
        }
    }

    #[test]
    fn test_evidence_command() {
        let cmd = AirportsCommand::Evidence(EvidenceCommand::Fetch {
            sources: vec!["NIST-CSF-2.0".to_string()],
        });

        match cmd {
            AirportsCommand::Evidence(EvidenceCommand::Fetch { sources }) => {
                assert_eq!(sources[0], "NIST-CSF-2.0");
            }
            _ => panic!("Wrong command variant"),
        }
    }

    #[test]
    fn test_incident_mode_command() {
        let cmd = AirportsCommand::Features(FeaturesCommand::IncidentMode {
            action: IncidentAction::Activate {
                incident_id: "INC-001".to_string(),
                approvers: vec!["ops".to_string(), "security".to_string()],
            },
        });

        match cmd {
            AirportsCommand::Features(FeaturesCommand::IncidentMode { action }) => {
                match action {
                    IncidentAction::Activate { approvers, .. } => {
                        assert_eq!(approvers.len(), 2);
                    }
                    _ => panic!("Wrong action variant"),
                }
            }
            _ => panic!("Wrong command variant"),
        }
    }
}
