//! HSIP Phase 2: Enterprise Identity & Key Management
//!
//! This crate provides hierarchical enterprise identities with delegation,
//! certificate chains, key rotation, and revocation for commercial deployments.
//!
//! # License
//! This is a COMMERCIAL component of HSIP Phase 2.
//! Licensed under HSIP Enterprise License.
//! See LICENSE-ENTERPRISE for terms.

#![allow(clippy::doc_markdown)]

pub mod identity;
pub mod delegation;
pub mod cert_chain;
pub mod revocation;
pub mod trust_store;

pub use identity::{EnterpriseIdentity, IdentityCapabilities, IdentityMetadata, PublicEnterpriseIdentity};
pub use delegation::{DelegationCert, SubIdentity};
pub use cert_chain::{CertChain, CertChainValidator};
pub use revocation::{RevocationList, RevocationReason};
pub use trust_store::{TrustStore, TrustAnchor, TrustAnchorMetadata};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum EnterpriseError {
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Certificate chain validation failed: {0}")]
    CertChainInvalid(String),

    #[error("Key has been revoked: {0}")]
    KeyRevoked(String),

    #[error("Delegation expired at {0}")]
    DelegationExpired(String),

    #[error("Insufficient capabilities: required {required}, has {actual}")]
    InsufficientCapabilities { required: String, actual: String },

    #[error("Identity not found: {0}")]
    IdentityNotFound(String),

    #[error("Cryptographic error: {0}")]
    CryptoError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, EnterpriseError>;
