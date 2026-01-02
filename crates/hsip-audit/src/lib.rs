//! HSIP Phase 2: Audit & Non-Repudiation Layer
//!
//! Tamper-evident, append-only, hash-chained audit logging for compliance.
//!
//! # License
//! This is a COMMERCIAL component of HSIP Phase 2.

#![allow(clippy::doc_markdown)]

pub mod entry;
pub mod log;
pub mod export;
pub mod merkle;

pub use entry::{AuditEntry, AuditEventType, VerificationResult};
pub use log::{AuditLog, AuditError};
pub use export::{ExportFormat, AuditExporter};
pub use merkle::{MerkleTree, MerkleProof};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Audit chain broken at entry {0}")]
    ChainBroken(u64),

    #[error("Invalid entry: {0}")]
    InvalidEntry(String),

    #[error("Export error: {0}")]
    ExportError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("CSV error: {0}")]
    CsvError(#[from] csv::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
