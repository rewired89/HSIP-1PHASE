// Copyright (c) 2025-2026 Nyx Systems LLC.
// SPDX-License-Identifier: SEE LICENSE IN ../../LICENSE
//
// This file is part of HSIP (Hyper Secure Internet Protocol).
// Free for non-commercial use. Commercial use requires a license.
// See LICENSE and COMMERCIAL_LICENSE.md for details.

// HSIP authentication subsystem
// Provides peer identity management, secure key storage, and token-based authentication

pub mod identity;
pub mod keystore;

#[doc(inline)]
pub use keystore as key_storage;

pub mod tokens;

// Internal authentication utilities
mod auth_internal {
    #[allow(unused)]
    pub(crate) fn _reserved_for_auth_expansion() {}
}
