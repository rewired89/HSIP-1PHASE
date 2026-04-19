use crate::db::Db;
use dashmap::{DashMap, DashSet};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64};
use std::sync::Arc;
use tokio::sync::Mutex;

// ── Proxy traffic monitoring ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ProxyEvent {
    pub id: String,
    pub ts_ms: i64,
    pub host: String,
    pub method: String,
    pub path: String,
    pub verdict: String,          // "blocked" | "allowed"
    pub category: Option<String>, // "advertising", "analytics", etc.
    pub reason: Option<String>,
}

/// Shared state owned by AppState; written by proxy thread, read by API handler.
pub struct ProxyShared {
    pub enabled: AtomicBool,
    pub port: AtomicU64,
    /// Ring buffer of last 500 events (newest at back).
    pub events: std::sync::Mutex<VecDeque<ProxyEvent>>,
    /// Signal the proxy thread to stop.
    pub shutdown: std::sync::Mutex<Option<std::sync::mpsc::SyncSender<()>>>,
}

impl ProxyShared {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            enabled: AtomicBool::new(false),
            port: AtomicU64::new(8877),
            events: std::sync::Mutex::new(VecDeque::with_capacity(500)),
            shutdown: std::sync::Mutex::new(None),
        })
    }
}

pub type ProxyState = Arc<ProxyShared>;

/// Per-key velocity record for AI agent anomaly detection
pub struct VelocityRecord {
    pub request_count: AtomicU64,
    pub anomaly_count: AtomicU64,
    pub window_start_ms: AtomicI64,
}

impl VelocityRecord {
    pub fn new(now_ms: i64) -> Self {
        Self {
            request_count: AtomicU64::new(1),
            anomaly_count: AtomicU64::new(0),
            window_start_ms: AtomicI64::new(now_ms),
        }
    }
}

/// Per-key rate limit window for all key types
pub struct RateWindow {
    pub count: AtomicU64,
    pub window_start_ms: AtomicI64,
}

impl RateWindow {
    pub fn new(now_ms: i64) -> Self {
        Self {
            count: AtomicU64::new(1),
            window_start_ms: AtomicI64::new(now_ms),
        }
    }
}

pub type AgentTracker = Arc<DashMap<String, VelocityRecord>>;
pub type RateLimiter = Arc<DashMap<String, RateWindow>>;
/// Keys that have been flagged for revocation but DB write may be in-flight.
/// Requests are rejected immediately once a key_id appears here.
pub type PendingRevocation = Arc<DashSet<String>>;

/// Shared DNS resolver handle — None when the resolver is stopped.
pub type DnsState = Arc<Mutex<Option<hsip_dns::DnsHandle>>>;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub agent_tracker: AgentTracker,
    pub rate_limiter: RateLimiter,
    pub pending_revocation: PendingRevocation,
    /// Loaded once at startup from HSIP_MASTER_KEY env var.
    pub master_key: Arc<Vec<u8>>,
    /// Optional running DNS resolver handle.
    pub dns: DnsState,
    /// HTTP/HTTPS proxy traffic state.
    pub proxy: ProxyState,
}

impl AppState {
    pub fn new(db: Db, master_key: Vec<u8>) -> Self {
        Self {
            db,
            agent_tracker: Arc::new(DashMap::new()),
            rate_limiter: Arc::new(DashMap::new()),
            pending_revocation: Arc::new(DashSet::new()),
            master_key: Arc::new(master_key),
            dns: Arc::new(Mutex::new(None)),
            proxy: ProxyShared::new(),
        }
    }
}
