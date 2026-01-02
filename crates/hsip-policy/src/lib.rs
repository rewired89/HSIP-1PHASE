//! HSIP Phase 2: Consent Policy Engine
//!
//! Enterprise-grade consent management with time-bounds, message-type scoping,
//! revocation, and cryptographic consent proofs.
//!
//! # License
//! This is a COMMERCIAL component of HSIP Phase 2.

#![allow(clippy::doc_markdown)]

pub mod policy;
pub mod consent_proof;
pub mod rules;
pub mod engine;

pub use policy::{ConsentPolicy, PolicyScope, TimeWindow};
pub use consent_proof::{ConsentProof, ConsentDecision};
pub use rules::{ConsentRule, RuleCondition};
pub use engine::{ConsentPolicyEngine, PolicyCheckResult};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PolicyError {
    #[error("Policy not found: {0}")]
    PolicyNotFound(String),

    #[error("Consent denied: {0}")]
    ConsentDenied(String),

    #[error("Policy expired: {0}")]
    PolicyExpired(String),

    #[error("Invalid policy: {0}")]
    InvalidPolicy(String),

    #[error("Signature error: {0}")]
    SignatureError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, PolicyError>;
