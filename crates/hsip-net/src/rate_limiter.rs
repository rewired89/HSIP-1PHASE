//! Rate limiting to prevent DoS attacks
//!
//! Implements token bucket algorithm for connection and message rate limiting
//!
//! **STATUS: NOT CURRENTLY INTEGRATED**
//!
//! This module is **not actively used** in the CLI. The `guard::Guard` module
//! (in `guard.rs`) provides equivalent protection with sliding window rate limiting
//! and is actively integrated into `udp.rs` and used by all control-plane listeners.
//!
//! This module remains for potential future use or alternative rate limiting strategies.
//! See `guard.rs` for the **active** rate limiting implementation.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per second per IP
    pub requests_per_second: u32,
    /// Burst capacity (tokens)
    pub burst_capacity: u32,
    /// Ban duration for violators
    pub ban_duration: Duration,
    /// Maximum connections per IP
    pub max_connections_per_ip: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 100,
            burst_capacity: 200,
            ban_duration: Duration::from_secs(300), // 5 minutes
            max_connections_per_ip: 10,
        }
    }
}

/// Token bucket for rate limiting
#[derive(Debug)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    violations: u32,
    banned_until: Option<Instant>,
}

impl TokenBucket {
    fn new(capacity: u32) -> Self {
        Self {
            tokens: capacity as f64,
            last_refill: Instant::now(),
            violations: 0,
            banned_until: None,
        }
    }

    fn refill(&mut self, rate: u32, capacity: u32) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        // Add tokens based on time elapsed
        self.tokens = (self.tokens + elapsed * rate as f64).min(capacity as f64);
        self.last_refill = now;
    }

    fn is_banned(&self) -> bool {
        if let Some(until) = self.banned_until {
            Instant::now() < until
        } else {
            false
        }
    }

    fn try_consume(&mut self, config: &RateLimitConfig) -> bool {
        // Check if banned
        if self.is_banned() {
            return false;
        }

        // Refill tokens
        self.refill(config.requests_per_second, config.burst_capacity);

        // Try to consume a token
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            // Rate limit exceeded
            self.violations += 1;

            // Ban after 3 violations
            if self.violations >= 3 {
                self.banned_until = Some(Instant::now() + config.ban_duration);
                eprintln!("[RATE_LIMIT] IP banned for {} seconds due to violations",
                         config.ban_duration.as_secs());
            }

            false
        }
    }
}

/// Rate limiter for network connections
pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<IpAddr, TokenBucket>>>,
    connections: Arc<RwLock<HashMap<IpAddr, u32>>>,
    config: RateLimitConfig,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Check if request is allowed
    pub fn check_request(&self, ip: IpAddr) -> Result<(), RateLimitError> {
        let mut buckets = self.buckets.write();

        let bucket = buckets
            .entry(ip)
            .or_insert_with(|| TokenBucket::new(self.config.burst_capacity));

        if bucket.is_banned() {
            return Err(RateLimitError::Banned);
        }

        if bucket.try_consume(&self.config) {
            Ok(())
        } else {
            Err(RateLimitError::RateExceeded)
        }
    }

    /// Check if connection is allowed
    pub fn check_connection(&self, ip: IpAddr) -> Result<(), RateLimitError> {
        let mut connections = self.connections.write();

        let count = connections.entry(ip).or_insert(0);

        if *count >= self.config.max_connections_per_ip {
            return Err(RateLimitError::TooManyConnections);
        }

        *count += 1;
        Ok(())
    }

    /// Release a connection
    pub fn release_connection(&self, ip: IpAddr) {
        let mut connections = self.connections.write();

        if let Some(count) = connections.get_mut(&ip) {
            if *count > 0 {
                *count -= 1;
            }

            if *count == 0 {
                connections.remove(&ip);
            }
        }
    }

    /// Clean up old entries (call periodically)
    pub fn cleanup(&self) {
        let now = Instant::now();

        // Clean up buckets
        let mut buckets = self.buckets.write();
        buckets.retain(|_, bucket| {
            // Keep if recently used or banned
            now.duration_since(bucket.last_refill) < Duration::from_secs(300)
                || bucket.is_banned()
        });

        // Clean up zero connections
        let mut connections = self.connections.write();
        connections.retain(|_, count| *count > 0);
    }
}

/// Rate limit errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitError {
    /// Rate limit exceeded
    RateExceeded,
    /// IP is banned
    Banned,
    /// Too many concurrent connections
    TooManyConnections,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RateExceeded => write!(f, "Rate limit exceeded"),
            Self::Banned => write!(f, "IP banned due to violations"),
            Self::TooManyConnections => write!(f, "Too many concurrent connections"),
        }
    }
}

impl std::error::Error for RateLimitError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiting() {
        let config = RateLimitConfig {
            requests_per_second: 10,
            burst_capacity: 20,
            ban_duration: Duration::from_secs(1),
            max_connections_per_ip: 5,
        };

        let limiter = RateLimiter::new(config);
        let ip = "127.0.0.1".parse().unwrap();

        // Should allow burst
        for _ in 0..20 {
            assert!(limiter.check_request(ip).is_ok());
        }

        // Should deny after burst exhausted
        assert!(limiter.check_request(ip).is_err());
    }

    #[test]
    fn test_connection_limiting() {
        let config = RateLimitConfig {
            max_connections_per_ip: 2,
            ..Default::default()
        };

        let limiter = RateLimiter::new(config);
        let ip = "127.0.0.1".parse().unwrap();

        // Allow 2 connections
        assert!(limiter.check_connection(ip).is_ok());
        assert!(limiter.check_connection(ip).is_ok());

        // Deny 3rd connection
        assert_eq!(
            limiter.check_connection(ip).unwrap_err(),
            RateLimitError::TooManyConnections
        );

        // Release one
        limiter.release_connection(ip);

        // Should allow again
        assert!(limiter.check_connection(ip).is_ok());
    }
}
