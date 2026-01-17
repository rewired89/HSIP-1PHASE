#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_const_for_fn)]

pub mod aad;
pub mod consent;
pub mod consent_policy;
pub mod error;
pub mod hello;
pub mod liveness;
pub mod nonce;
pub mod session;
pub mod session_resumption;

pub mod crypto {
    pub mod aead;
    pub mod labels;
    pub mod nonce;
}
pub mod identity;
pub mod keystore;
pub mod wire;

/// Security hardening modules
pub mod constant_time;
pub mod secure_memory;

/// Post-quantum cryptography module (requires 'pqc' feature)
/// Provides hybrid X25519+ML-KEM key exchange and Ed25519+ML-DSA signatures
#[cfg(feature = "pqc")]
pub mod pqc;
