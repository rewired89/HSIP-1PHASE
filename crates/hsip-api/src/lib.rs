//! HSIP Phase 2: Enterprise API
//!
//! REST API for enterprise integration
//!
//! # License
//! This is a COMMERCIAL component of HSIP Phase 2.

#![allow(clippy::doc_markdown)]

pub mod server;
pub mod handlers;
pub mod models;
pub mod error;
pub mod verify_handler;

pub use server::ApiServer;
pub use error::{ApiError, ApiResult};
pub use models::*;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Enterprise error: {0}")]
    EnterpriseError(#[from] hsip_enterprise::EnterpriseError),

    #[error("Verify error: {0}")]
    VerifyError(#[from] hsip_verify::VerifyError),

    #[error("Policy error: {0}")]
    PolicyError(#[from] hsip_policy::PolicyError),

    #[error("Audit error: {0}")]
    AuditError(#[from] hsip_audit::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
