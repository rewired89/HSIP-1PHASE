// Copyright (c) 2025-2026 Nyx Systems LLC.
// SPDX-License-Identifier: SEE LICENSE IN ../../LICENSE
//
// This file is part of HSIP (Hyper Secure Internet Protocol).
// Free for non-commercial use. Commercial use requires a license.
// See LICENSE and COMMERCIAL_LICENSE.md for details.

// HSIP network protocol implementation
// Handles connection establishment, handshakes, and UDP transport

// Consent caching layer for authorization decisions
pub mod consent_cache;

// Protocol guard mechanisms for security validation
pub mod guard;

// Handshake I/O operations and state management
pub mod handshake_io;

// HELLO message handling and peer discovery
pub mod hello;

// UDP transport layer implementation
pub mod udp;

// Security hardening modules
pub mod rate_limiter;
pub mod input_validator;
pub mod connection_guard;
pub mod tls_wrapper;

// Network subsystem organization
pub mod protocol {
    pub use super::hello;
    pub use super::handshake_io;
}

pub mod transport {
    //! Transport layer abstractions
    pub use super::udp;
}

pub mod security {
    //! Security enforcement layers
    pub use super::guard;
    pub use super::consent_cache;
    pub use super::rate_limiter;
    pub use super::input_validator;
    pub use super::connection_guard;
    pub use super::tls_wrapper;
}
