use crate::db::Db;
use dashmap::{DashMap, DashSet};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

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

    /// Reconstructs a record from persisted values — see
    /// `rate_limit_persistence::load`.
    pub fn from_parts(request_count: u64, anomaly_count: u64, window_start_ms: i64) -> Self {
        Self {
            request_count: AtomicU64::new(request_count),
            anomaly_count: AtomicU64::new(anomaly_count),
            window_start_ms: AtomicI64::new(window_start_ms),
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

    /// Reconstructs a window from persisted values — see
    /// `rate_limit_persistence::load`.
    pub fn from_parts(count: u64, window_start_ms: i64) -> Self {
        Self {
            count: AtomicU64::new(count),
            window_start_ms: AtomicI64::new(window_start_ms),
        }
    }
}

/// Velocity/anomaly tracking for `ai_agent` keys. In-memory for hot-path
/// speed; periodically snapshotted to the `rate_limit_state` table and
/// restored at startup by `rate_limit_persistence`, so a restart doesn't
/// silently reset how close a key is to auto-revocation.
pub type AgentTracker = Arc<DashMap<String, VelocityRecord>>;
/// Per-key rate limit windows for all key types. Same persistence story as
/// `AgentTracker` — see `rate_limit_persistence`.
pub type RateLimiter = Arc<DashMap<String, RateWindow>>;
/// Keys that have been flagged for revocation but DB write may be in-flight.
/// Requests are rejected immediately once a key_id appears here.
pub type PendingRevocation = Arc<DashSet<String>>;

/// Shared DNS resolver handle — None when the resolver is stopped.
pub type DnsState = Arc<Mutex<Option<hsip_dns::DnsHandle>>>;

/// IP-keyed provision rate limiter for the sandbox endpoint (5/hour per IP).
/// Same persistence story as `RateLimiter` — see `rate_limit_persistence`.
pub type SandboxRate = Arc<DashMap<String, RateWindow>>;

/// Replay-protection nonce tracker, keyed by `"{key_id}:{nonce}"`, value is
/// the ms timestamp after which the entry may be swept. Opt-in — only
/// populated for requests that send both `x-hsip-timestamp` and
/// `x-hsip-nonce` (see `auth.rs::check_replay_protection`). A background
/// sweep in `main.rs` removes expired entries so this can't grow unbounded.
pub type ReplayNonceTracker = Arc<DashMap<String, i64>>;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub agent_tracker: AgentTracker,
    pub rate_limiter: RateLimiter,
    pub pending_revocation: PendingRevocation,
    pub replay_nonces: ReplayNonceTracker,
    /// Loaded at startup from `HSIP_MASTER_KEY` env var or the master key
    /// file (see `main.rs::load_master_key`). Behind a lock, not a plain
    /// `Arc<Vec<u8>>`, so `POST /v1/admin/master-key/rotate` can swap it for
    /// every subsequent request without a restart.
    pub master_key: Arc<RwLock<Vec<u8>>>,
    /// File path the master key can be durably rewritten to, if any.
    /// `None` when the key came from `HSIP_MASTER_KEY` — there's no file
    /// this process owns to rewrite, so rotation via the API is refused in
    /// that case (rotate the env var's source, e.g. the secrets manager,
    /// instead).
    pub master_key_path: Option<Arc<String>>,
    /// Optional running DNS resolver handle.
    pub dns: DnsState,
    /// HTTP/HTTPS proxy traffic state.
    pub proxy: ProxyState,
    /// Sandbox provision rate limiter — keyed by source IP.
    pub sandbox_rate: SandboxRate,
}

impl AppState {
    pub fn new(db: Db, master_key: Vec<u8>) -> Self {
        Self::new_with_master_key_path(db, master_key, None)
    }

    pub fn new_with_master_key_path(
        db: Db,
        master_key: Vec<u8>,
        master_key_path: Option<String>,
    ) -> Self {
        Self {
            db,
            agent_tracker: Arc::new(DashMap::new()),
            rate_limiter: Arc::new(DashMap::new()),
            pending_revocation: Arc::new(DashSet::new()),
            replay_nonces: Arc::new(DashMap::new()),
            master_key: Arc::new(RwLock::new(master_key)),
            master_key_path: master_key_path.map(Arc::new),
            dns: Arc::new(Mutex::new(None)),
            proxy: ProxyShared::new(),
            sandbox_rate: Arc::new(DashMap::new()),
        }
    }
}
