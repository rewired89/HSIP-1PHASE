//! HSIP Airports - Commercial Cybersecurity Module for Airport Infrastructure
//!
//! This module provides:
//! - Gap analysis for airport cybersecurity vulnerabilities
//! - HSIP feature extensions tailored for airport operations
//! - Compliance mapping against TSA/NIST/FAA/CISA frameworks
//! - Court-defensible audit trails for airport critical systems

pub mod domain;
pub mod evidence;
pub mod analysis;
pub mod features;
pub mod export;

#[cfg(feature = "cli")]
pub mod cli;

// Re-export key types
pub use domain::{
    AirportSystem, ThreatVector, ComplianceGap, GapCategory, Severity,
    FrameworkCoverage, Mitigation
};

pub use analysis::{
    GapAnalysisPipeline, AnalysisResult
};

pub use features::{
    IncidentModePolicy, CourtGradeAuditBundle
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AirportError {
    #[error("Evidence collection failed: {0}")]
    EvidenceError(String),

    #[error("Analysis pipeline error: {0}")]
    AnalysisError(String),

    #[error("Export error: {0}")]
    ExportError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, AirportError>;
