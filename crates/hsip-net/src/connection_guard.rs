//! Connection guards with timeouts and resource limits
//!
//! Prevents resource exhaustion and slowloris attacks
//!
//! **STATUS: NOT CURRENTLY INTEGRATED**
//!
//! This module is **not actively used** in the CLI. The `guard::Guard` module
//! (in `guard.rs`) provides active protection through per-IP rate limiting,
//! bad signature tracking, and frame size limits. While this module offers
//! complementary features (connection slot limits, bandwidth tracking, idle detection),
//! it is not currently integrated into the protocol handlers.
//!
//! This module remains for potential future use in Phase 2 when more sophisticated
//! connection management is needed. See `guard.rs` for the **active** protection layer.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use parking_lot::Mutex;

/// Connection limits and timeouts
#[derive(Debug, Clone)]
pub struct ConnectionLimits {
    /// Maximum total concurrent connections
    pub max_total_connections: usize,
    /// Connection idle timeout
    pub idle_timeout: Duration,
    /// Handshake timeout
    pub handshake_timeout: Duration,
    /// Read/write timeout
    pub io_timeout: Duration,
    /// Maximum bandwidth per connection (bytes/sec)
    pub max_bandwidth_per_conn: u64,
}

impl Default for ConnectionLimits {
    fn default() -> Self {
        Self {
            max_total_connections: 1000,
            idle_timeout: Duration::from_secs(300), // 5 minutes
            handshake_timeout: Duration::from_secs(10),
            io_timeout: Duration::from_secs(30),
            max_bandwidth_per_conn: 10 * 1024 * 1024, // 10 MB/s
        }
    }
}

/// Global connection tracker
#[derive(Debug)]
pub struct ConnectionTracker {
    active_connections: Arc<AtomicUsize>,
    total_bytes_sent: Arc<AtomicU64>,
    total_bytes_received: Arc<AtomicU64>,
    limits: ConnectionLimits,
}

impl ConnectionTracker {
    pub fn new(limits: ConnectionLimits) -> Self {
        Self {
            active_connections: Arc::new(AtomicUsize::new(0)),
            total_bytes_sent: Arc::new(AtomicU64::new(0)),
            total_bytes_received: Arc::new(AtomicU64::new(0)),
            limits,
        }
    }

    /// Try to acquire a connection slot
    pub fn try_acquire(&self) -> Result<ConnectionGuard, ConnectionError> {
        let current = self.active_connections.load(Ordering::Relaxed);

        if current >= self.limits.max_total_connections {
            return Err(ConnectionError::TooManyConnections);
        }

        self.active_connections.fetch_add(1, Ordering::Relaxed);

        Ok(ConnectionGuard {
            tracker: Arc::new(Mutex::new(self.clone())),
            created: Instant::now(),
            last_activity: Arc::new(Mutex::new(Instant::now())),
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Get current statistics
    pub fn stats(&self) -> ConnectionStats {
        ConnectionStats {
            active_connections: self.active_connections.load(Ordering::Relaxed),
            total_bytes_sent: self.total_bytes_sent.load(Ordering::Relaxed),
            total_bytes_received: self.total_bytes_received.load(Ordering::Relaxed),
        }
    }

    fn release(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Clone for ConnectionTracker {
    fn clone(&self) -> Self {
        Self {
            active_connections: Arc::clone(&self.active_connections),
            total_bytes_sent: Arc::clone(&self.total_bytes_sent),
            total_bytes_received: Arc::clone(&self.total_bytes_received),
            limits: self.limits.clone(),
        }
    }
}

/// RAII guard for a single connection
#[derive(Debug)]
pub struct ConnectionGuard {
    tracker: Arc<Mutex<ConnectionTracker>>,
    created: Instant,
    last_activity: Arc<Mutex<Instant>>,
    bytes_sent: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
}

impl ConnectionGuard {
    /// Check if connection is idle
    pub fn is_idle(&self, timeout: Duration) -> bool {
        let last = *self.last_activity.lock();
        Instant::now().duration_since(last) > timeout
    }

    /// Update activity timestamp
    pub fn touch(&self) {
        *self.last_activity.lock() = Instant::now();
    }

    /// Record sent bytes
    pub fn record_sent(&self, bytes: u64) {
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
        self.tracker
            .lock()
            .total_bytes_sent
            .fetch_add(bytes, Ordering::Relaxed);
        self.touch();
    }

    /// Record received bytes
    pub fn record_received(&self, bytes: u64) {
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
        self.tracker
            .lock()
            .total_bytes_received
            .fetch_add(bytes, Ordering::Relaxed);
        self.touch();
    }

    /// Check bandwidth limit
    pub fn check_bandwidth(&self, limits: &ConnectionLimits) -> Result<(), ConnectionError> {
        let elapsed = Instant::now().duration_since(self.created).as_secs_f64();

        if elapsed < 1.0 {
            // Don't check in first second
            return Ok(());
        }

        let bytes_sent = self.bytes_sent.load(Ordering::Relaxed);
        let bytes_received = self.bytes_received.load(Ordering::Relaxed);
        let total_bytes = bytes_sent + bytes_received;

        let bandwidth = total_bytes as f64 / elapsed;

        if bandwidth > limits.max_bandwidth_per_conn as f64 {
            return Err(ConnectionError::BandwidthExceeded);
        }

        Ok(())
    }

    /// Get connection age
    pub fn age(&self) -> Duration {
        Instant::now().duration_since(self.created)
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.tracker.lock().release();
    }
}

/// Connection statistics
#[derive(Debug, Clone, Copy)]
pub struct ConnectionStats {
    pub active_connections: usize,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
}

/// Connection errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionError {
    TooManyConnections,
    BandwidthExceeded,
    Timeout,
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyConnections => write!(f, "Too many concurrent connections"),
            Self::BandwidthExceeded => write!(f, "Bandwidth limit exceeded"),
            Self::Timeout => write!(f, "Connection timeout"),
        }
    }
}

impl std::error::Error for ConnectionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_limits() {
        let limits = ConnectionLimits {
            max_total_connections: 2,
            ..Default::default()
        };

        let tracker = ConnectionTracker::new(limits);

        // Should allow 2 connections
        let conn1 = tracker.try_acquire().unwrap();
        let conn2 = tracker.try_acquire().unwrap();

        // Should deny 3rd
        assert_eq!(
            tracker.try_acquire().unwrap_err(),
            ConnectionError::TooManyConnections
        );

        // Drop one
        drop(conn1);

        // Should allow again
        let _conn3 = tracker.try_acquire().unwrap();

        drop(conn2);
    }

    #[test]
    fn test_idle_detection() {
        let limits = ConnectionLimits::default();
        let tracker = ConnectionTracker::new(limits);
        let conn = tracker.try_acquire().unwrap();

        // Not idle initially
        assert!(!conn.is_idle(Duration::from_secs(1)));

        // Touch it
        conn.touch();
        assert!(!conn.is_idle(Duration::from_secs(1)));
    }
}
