//! DNS resolver control endpoints
//!
//! GET  /v1/dns/status  — resolver state + counters
//! POST /v1/dns/enable  — start the local DNS resolver
//! POST /v1/dns/disable — stop  the local DNS resolver
//! GET  /v1/dns/log     — last 50 queries (blocked first)

use crate::{auth::TenantId, errors::ApiError, state::AppState};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

const DEFAULT_DNS_PORT: u16 = 5300;

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct DnsStatusResponse {
    pub running: bool,
    pub port: Option<u16>,
    pub queries_total: u64,
    pub blocked_total: u64,
    pub blocklist_size: usize,
}

#[derive(Deserialize)]
pub struct EnableRequest {
    /// UDP port to listen on. Defaults to 5300.
    pub port: Option<u16>,
}

#[derive(Serialize)]
pub struct DnsLogEntry {
    pub domain: String,
    pub blocked: bool,
    pub vendor: Option<String>,
    pub category: Option<String>,
    pub timestamp_ms: i64,
}

#[derive(Serialize)]
pub struct DnsLogResponse {
    pub entries: Vec<DnsLogEntry>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /v1/dns/status
pub async fn status(
    _tenant: TenantId,
    State(state): State<AppState>,
) -> Result<Json<DnsStatusResponse>, ApiError> {
    let guard = state.dns.lock().await;
    match &*guard {
        Some(h) => Ok(Json(DnsStatusResponse {
            running: true,
            port: Some(h.port),
            queries_total: h.stats.queries_total.load(Ordering::Relaxed),
            blocked_total: h.stats.blocked_total.load(Ordering::Relaxed),
            blocklist_size: hsip_dns::DnsHandle::blocklist_size(),
        })),
        None => Ok(Json(DnsStatusResponse {
            running: false,
            port: None,
            queries_total: 0,
            blocked_total: 0,
            blocklist_size: hsip_dns::DnsHandle::blocklist_size(),
        })),
    }
}

/// POST /v1/dns/enable
pub async fn enable(
    _tenant: TenantId,
    State(state): State<AppState>,
    body: Option<Json<EnableRequest>>,
) -> Result<Json<DnsStatusResponse>, ApiError> {
    let port = body
        .as_ref()
        .and_then(|b| b.port)
        .unwrap_or(DEFAULT_DNS_PORT);

    let mut guard = state.dns.lock().await;

    // Already running on the requested port — return current stats
    if let Some(ref h) = *guard {
        if h.port == port {
            return Ok(Json(DnsStatusResponse {
                running: true,
                port: Some(h.port),
                queries_total: h.stats.queries_total.load(Ordering::Relaxed),
                blocked_total: h.stats.blocked_total.load(Ordering::Relaxed),
                blocklist_size: hsip_dns::DnsHandle::blocklist_size(),
            }));
        }
        // Different port requested — shut down old instance first
        h.shutdown();
    }

    let handle = hsip_dns::start(port).await.map_err(|e| {
        // Port already in use means another HSIP instance is running the DNS resolver.
        let msg = e.to_string();
        if msg.contains("10048")
            || msg.contains("already in use")
            || msg.contains("Address already in use")
        {
            ApiError::Internal(
                "DNS resolver is already active (started by another HSIP window). \
                 Close all HSIP windows, then reopen HSIP and enable DNS from here."
                    .to_string(),
            )
        } else {
            ApiError::Internal(format!("Failed to start DNS resolver: {}", e))
        }
    })?;

    let resp = DnsStatusResponse {
        running: true,
        port: Some(handle.port),
        queries_total: 0,
        blocked_total: 0,
        blocklist_size: hsip_dns::DnsHandle::blocklist_size(),
    };
    *guard = Some(handle);

    tracing::info!("DNS resolver started on port {}", port);
    Ok(Json(resp))
}

/// POST /v1/dns/disable
pub async fn disable(
    _tenant: TenantId,
    State(state): State<AppState>,
) -> Result<Json<DnsStatusResponse>, ApiError> {
    let mut guard = state.dns.lock().await;
    if let Some(h) = guard.take() {
        h.shutdown();
        tracing::info!("DNS resolver stopped");
    }

    Ok(Json(DnsStatusResponse {
        running: false,
        port: None,
        queries_total: 0,
        blocked_total: 0,
        blocklist_size: hsip_dns::DnsHandle::blocklist_size(),
    }))
}

/// GET /v1/dns/log
pub async fn log(
    _tenant: TenantId,
    State(state): State<AppState>,
) -> Result<Json<DnsLogResponse>, ApiError> {
    let guard = state.dns.lock().await;
    let entries = match &*guard {
        None => vec![],
        Some(h) => {
            let q = h.log.entries.read().await;
            q.iter()
                .rev()
                .take(50)
                .map(|e| DnsLogEntry {
                    domain: e.domain.clone(),
                    blocked: e.blocked,
                    vendor: e.vendor.clone(),
                    category: e.category.clone(),
                    timestamp_ms: e.timestamp_ms,
                })
                .collect()
        }
    };

    Ok(Json(DnsLogResponse { entries }))
}
