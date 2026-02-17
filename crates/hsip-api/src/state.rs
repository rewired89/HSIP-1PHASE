use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64};
use dashmap::DashMap;
use crate::db::Db;

/// Per-key velocity record for AI agent anomaly detection
pub struct VelocityRecord {
    pub request_count:   AtomicU64,
    pub anomaly_count:   AtomicU64,
    pub window_start_ms: AtomicI64,
}

impl VelocityRecord {
    pub fn new(now_ms: i64) -> Self {
        Self {
            request_count:   AtomicU64::new(1),
            anomaly_count:   AtomicU64::new(0),
            window_start_ms: AtomicI64::new(now_ms),
        }
    }
}

/// Per-key rate limit window for all key types
pub struct RateWindow {
    pub count:           AtomicU64,
    pub window_start_ms: AtomicI64,
}

impl RateWindow {
    pub fn new(now_ms: i64) -> Self {
        Self {
            count:           AtomicU64::new(1),
            window_start_ms: AtomicI64::new(now_ms),
        }
    }
}

pub type AgentTracker = Arc<DashMap<String, VelocityRecord>>;
pub type RateLimiter  = Arc<DashMap<String, RateWindow>>;

#[derive(Clone)]
pub struct AppState {
    pub db:            Db,
    pub agent_tracker: AgentTracker,
    pub rate_limiter:  RateLimiter,
}

impl AppState {
    pub fn new(db: Db) -> Self {
        Self {
            db,
            agent_tracker: Arc::new(DashMap::new()),
            rate_limiter:  Arc::new(DashMap::new()),
        }
    }
}
