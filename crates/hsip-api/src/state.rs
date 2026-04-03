use crate::db::Db;
use dashmap::{DashMap, DashSet};
use std::sync::atomic::{AtomicI64, AtomicU64};
use std::sync::Arc;
use tokio::sync::Mutex;

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
        }
    }
}
