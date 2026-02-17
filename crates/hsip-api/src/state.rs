use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64};
use dashmap::DashMap;
use crate::db::Db;

/// Per-key velocity record stored in memory (no DB overhead for hot path)
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

/// Shared in-memory tracker for all AI agent keys
pub type AgentTracker = Arc<DashMap<String, VelocityRecord>>;

#[derive(Clone)]
pub struct AppState {
    pub db:            Db,
    pub agent_tracker: AgentTracker,
}

impl AppState {
    pub fn new(db: Db) -> Self {
        Self {
            db,
            agent_tracker: Arc::new(DashMap::new()),
        }
    }
}
