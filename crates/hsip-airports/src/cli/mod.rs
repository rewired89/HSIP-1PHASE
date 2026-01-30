pub mod commands;

pub use commands::{AirportsCommand, GapCommand, EvidenceCommand, FeaturesCommand};

use crate::Result;

/// CLI entry point for airports module
pub async fn execute(command: AirportsCommand) -> Result<()> {
    match command {
        AirportsCommand::Gap(gap_cmd) => execute_gap_command(gap_cmd).await,
        AirportsCommand::Evidence(evidence_cmd) => execute_evidence_command(evidence_cmd).await,
        AirportsCommand::Features(features_cmd) => execute_features_command(features_cmd).await,
    }
}

async fn execute_gap_command(command: GapCommand) -> Result<()> {
    match command {
        GapCommand::Analyze { systems, output } => {
            println!("Running gap analysis for systems: {:?}", systems);
            println!("Output format: {:?}", output);
            // TODO: Implement gap analysis pipeline execution
            Ok(())
        }
        GapCommand::List { severity, category } => {
            println!("Listing gaps - severity: {:?}, category: {:?}", severity, category);
            // TODO: Query and display existing gaps
            Ok(())
        }
        GapCommand::Export { gap_id, format } => {
            println!("Exporting gap {} as {:?}", gap_id, format);
            // TODO: Export specific gap to file
            Ok(())
        }
    }
}

async fn execute_evidence_command(command: EvidenceCommand) -> Result<()> {
    match command {
        EvidenceCommand::Fetch { sources } => {
            println!("Fetching evidence from: {:?}", sources);
            // TODO: Use Collector to fetch documents
            Ok(())
        }
        EvidenceCommand::List => {
            println!("Listing collected evidence sources");
            // TODO: Display provenance tracker contents
            Ok(())
        }
        EvidenceCommand::Verify { source_id } => {
            println!("Verifying evidence source: {}", source_id);
            // TODO: Verify SHA-256 hash and accessibility
            Ok(())
        }
    }
}

async fn execute_features_command(command: FeaturesCommand) -> Result<()> {
    match command {
        FeaturesCommand::IncidentMode { action } => {
            println!("Incident mode action: {:?}", action);
            // TODO: Activate/deactivate incident mode
            Ok(())
        }
        FeaturesCommand::AuditBundle { action } => {
            println!("Audit bundle action: {:?}", action);
            // TODO: Create/seal/verify audit bundles
            Ok(())
        }
    }
}
