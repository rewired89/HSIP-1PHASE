//! HSIP Prove platform: multi-tenant proof service for businesses.
//!
//! The cryptography lives in `hsip-proof` (open format, pure verifier).
//! This crate adds accounts, organizations, streams, API keys, anchoring,
//! evidence exports, plans and the console.

pub mod anchor;
pub mod audit;
pub mod auth;
pub mod config;
pub mod crypto;
pub mod db;
pub mod engine;
pub mod errors;
pub mod plans;
pub mod qb;
pub mod report;
pub mod routes;
pub mod state;
pub mod ui;

use axum::{http::HeaderValue, Router};
use tower_http::set_header::SetResponseHeaderLayer;

/// Complete application: API routes, security headers, console fallback.
pub fn app(state: state::AppState) -> Router {
    let console_dir = state.cfg.console_dir.clone();
    let h = |name: &'static str, v: &'static str| {
        SetResponseHeaderLayer::if_not_present(
            axum::http::HeaderName::from_static(name),
            HeaderValue::from_static(v),
        )
    };
    routes::api(state)
        .fallback(move |uri| ui::serve(uri, console_dir.clone()))
        .layer(h("x-content-type-options", "nosniff"))
        .layer(h("x-frame-options", "DENY"))
        .layer(h("referrer-policy", "no-referrer"))
        .layer(h(
            "content-security-policy",
            "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'",
        ))
}
