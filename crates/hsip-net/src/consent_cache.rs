use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct ConsentCache {
    allow_until: HashMap<String, Instant>,
    ttl: Duration,
}

impl ConsentCache {
    /// ttl_ms: cache lifetime for an "allow" (e.g., 300_000 ms = 5m)
    pub fn new(ttl_ms: u64) -> Self {
        Self {
            allow_until: HashMap::new(),
            ttl: Duration::from_millis(ttl_ms),
        }
    }

    /// If requester is cached and not expired, return true.
    pub fn is_allowed(&mut self, requester: &str) -> bool {
        if requester.is_empty() {
            return false;
        }
        let now = Instant::now();
        if let Some(exp) = self.allow_until.get(requester).cloned() {
            if now < exp {
                return true;
            }
            self.allow_until.remove(requester);
        }
        false
    }

    /// Insert/refresh an allow entry.
    pub fn insert_allow(&mut self, requester: &str) {
        if requester.is_empty() {
            return;
        }
        let exp = Instant::now() + self.ttl;
        self.allow_until.insert(requester.to_string(), exp);
    }

    /// Remove an entry (e.g., after tamper).
    /// This triggers instant session termination for active sessions with this peer.
    pub fn revoke(&mut self, requester: &str) {
        if requester.is_empty() {
            return;
        }
        self.allow_until.remove(requester);
        // Sessions check consent on every encrypt/decrypt via with_consent_check()
        // Next operation will fail with SessionError::ConsentRevoked
    }
}

/// Thread-safe consent cache for sharing across sessions
#[derive(Clone)]
pub struct SharedConsentCache {
    inner: Arc<RwLock<ConsentCache>>,
}

impl SharedConsentCache {
    pub fn new(ttl_ms: u64) -> Self {
        Self {
            inner: Arc::new(RwLock::new(ConsentCache::new(ttl_ms))),
        }
    }

    pub fn is_allowed(&self, peer_id: &str) -> bool {
        self.inner.write().is_allowed(peer_id)
    }

    pub fn insert_allow(&self, peer_id: &str) {
        self.inner.write().insert_allow(peer_id);
    }

    pub fn revoke(&self, peer_id: &str) {
        self.inner.write().revoke(peer_id);
    }

    /// Create a consent check callback for use with sessions
    /// Sessions will check consent on every encrypt/decrypt operation
    pub fn create_check_callback(&self, peer_id: String) -> impl Fn() -> bool + Send + Sync + 'static {
        let cache = self.clone();
        move || cache.is_allowed(&peer_id)
    }
}
