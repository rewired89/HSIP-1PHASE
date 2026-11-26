//! HSIP platform: multi-tenant proof service for businesses.
//!
//! The cryptography lives in `hsip-proof` (open format, pure verifier).
//! This crate adds accounts, organizations, streams, API keys, anchoring,
//! evidence exports, plans and the console.

pub mod anchor;
pub mod audit;
pub mod auth;
pub mod checkpoints;
pub mod config;
pub mod crypto;
pub mod db;
pub mod email;
pub mod engine;
pub mod errors;
pub mod plans;
pub mod qb;
pub mod report;
pub mod routes;
pub mod state;
pub mod ui;

use std::net::{IpAddr, SocketAddr};

use axum::{
    extract::{ConnectInfo, Request, State},
    http::HeaderValue,
    middleware::{from_fn_with_state, Next},
    response::Response,
    Router,
};
use tower_http::set_header::SetResponseHeaderLayer;

/// Resolves the caller's IP once, from a source the caller cannot forge, and
/// stores it under `auth::CLIENT_IP_HEADER` (replacing anything sent there).
async fn resolve_client_ip(
    State(s): State<state::AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let from_proxy = s
        .cfg
        .client_ip_header
        .as_deref()
        .and_then(|h| req.headers().get(h))
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<IpAddr>().ok());
    let ip = from_proxy
        .or_else(|| {
            req.extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|c| c.0.ip())
        })
        .map_or_else(|| "local".to_string(), |ip| ip.to_string());
    if let Ok(v) = HeaderValue::from_str(&ip) {
        req.headers_mut().insert(auth::CLIENT_IP_HEADER, v);
    }
    next.run(req).await
}

/// Complete application: API routes, security headers, console fallback.
pub fn app(state: state::AppState) -> Router {
    let console_dir = state.cfg.console_dir.clone();
    let ip_state = state.clone();
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
        .layer(from_fn_with_state(ip_state, resolve_client_ip))
}
