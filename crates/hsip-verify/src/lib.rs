//! HSIP Phase 2: Verified Message Classification
//!
//! Provides message type classification, verification, and the core
//! "✅ VERIFIED" / "🔴 UNVERIFIED" system for enterprise messages.
//!
//! # License
//! This is a COMMERCIAL component of HSIP Phase 2.
//! Licensed under HSIP Enterprise License.

#![allow(clippy::doc_markdown)]

pub mod message_types;
pub mod verification;
pub mod classifier;
pub mod rejection;

pub use message_types::{MessageType, MessageMetadata};
pub use verification::{VerificationEngine, VerificationResult};
pub use classifier::{MessageClassifier, ClassificationResult};
pub use rejection::{RejectionReason, RejectionAction, RejectionHandler};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum VerifyError {
    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("Message rejected: {reason:?}")]
    MessageRejected { reason: RejectionReason },

    #[error("Invalid message format: {0}")]
    InvalidFormat(String),

    #[error("Unsupported message type: {0}")]
    UnsupportedType(String),

    #[error("Enterprise error: {0}")]
    EnterpriseError(#[from] hsip_enterprise::EnterpriseError),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, VerifyError>;
