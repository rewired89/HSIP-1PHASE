//! Flow Metadata - Information about network connections
//!
//! Captures all relevant metadata about a network flow to enable
//! policy decisions without inspecting actual payload content.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};

#[cfg(feature = "geolocation")]
use crate::geolocation::GeoLocation;

/// Protocol type for the flow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FlowProtocol {
    /// HTTP/1.x
    Http,
    /// HTTPS (TLS)
    Https,
    /// HTTP/2
    Http2,
    /// HTTP/3 (QUIC)
    Http3,
    /// WebSocket
    WebSocket,
    /// WebSocket Secure
    WebSocketSecure,
    /// gRPC
    Grpc,
    /// Raw TCP
    Tcp,
    /// UDP
    Udp,
    /// DNS
    Dns,
    /// Unknown protocol
    Unknown,
}

impl FlowProtocol {
    /// Check if this protocol is encrypted
    pub fn is_encrypted(&self) -> bool {
        matches!(
            self,
            FlowProtocol::Https
                | FlowProtocol::Http2
                | FlowProtocol::Http3
                | FlowProtocol::WebSocketSecure
                | FlowProtocol::Grpc
        )
    }
}

/// Inferred intent of the telemetry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TelemetryIntent {
    /// Crash/error reporting (often legitimate)
    CrashReport,
    /// Usage analytics and metrics
    UsageAnalytics,
    /// Vendor diagnostics and debugging
    Diagnostics,
    /// Advertising and tracking
    Advertising,
    /// Feature flags and A/B testing
    FeatureFlags,
    /// License verification
    LicenseCheck,
    /// Heartbeat/keepalive
    Heartbeat,
    /// User behavior tracking
    BehaviorTracking,
    /// Performance monitoring
    Performance,
    /// Security/fraud detection
    Security,
    /// Unknown/unclassified
    Unknown,
}

impl TelemetryIntent {
    /// Returns whether this intent is typically privacy-invasive
    pub fn is_invasive(&self) -> bool {
        matches!(
            self,
            TelemetryIntent::Advertising
                | TelemetryIntent::BehaviorTracking
                | TelemetryIntent::UsageAnalytics
        )
    }

    /// Returns a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            TelemetryIntent::CrashReport => "Crash and error reporting",
            TelemetryIntent::UsageAnalytics => "Usage analytics and metrics collection",
            TelemetryIntent::Diagnostics => "Vendor diagnostics and debugging",
            TelemetryIntent::Advertising => "Advertising and ad tracking",
            TelemetryIntent::FeatureFlags => "Feature flags and A/B testing",
            TelemetryIntent::LicenseCheck => "License and activation verification",
            TelemetryIntent::Heartbeat => "Connection keepalive",
            TelemetryIntent::BehaviorTracking => "User behavior tracking",
            TelemetryIntent::Performance => "Performance monitoring",
            TelemetryIntent::Security => "Security and fraud detection",
            TelemetryIntent::Unknown => "Unknown telemetry purpose",
        }
    }
}

/// Risk level of the telemetry
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RiskLevel {
    /// No privacy risk (e.g., local-only)
    None = 0,
    /// Low risk (e.g., anonymous crash reports)
    Low = 1,
    /// Medium risk (e.g., usage analytics)
    Medium = 2,
    /// High risk (e.g., behavior tracking)
    High = 3,
    /// Critical risk (e.g., PII exfiltration)
    Critical = 4,
}

/// Device fingerprint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceFingerprint {
    /// User-Agent string
    pub user_agent: Option<String>,
    /// Accept-Language header
    pub accept_language: Option<String>,
    /// Accept-Encoding header
    pub accept_encoding: Option<String>,
    /// Screen resolution (if available from headers)
    pub screen_resolution: Option<String>,
    /// Color depth
    pub color_depth: Option<u8>,
    /// Timezone offset in minutes
    pub timezone_offset: Option<i32>,
    /// Platform (from User-Agent parsing)
    pub platform: Option<String>,
    /// Browser name and version
    pub browser: Option<String>,
    /// Operating system
    pub os: Option<String>,
    /// Hardware concurrency (CPU cores)
    pub hardware_concurrency: Option<u8>,
    /// Device memory in GB
    pub device_memory: Option<u8>,
}

impl Default for DeviceFingerprint {
    fn default() -> Self {
        Self {
            user_agent: None,
            accept_language: None,
            accept_encoding: None,
            screen_resolution: None,
            color_depth: None,
            timezone_offset: None,
            platform: None,
            browser: None,
            os: None,
            hardware_concurrency: None,
            device_memory: None,
        }
    }
}

impl DeviceFingerprint {
    /// Parse User-Agent to extract platform and browser info
    pub fn parse_user_agent(&mut self, ua: &str) {
        self.user_agent = Some(ua.to_string());

        // Simple parsing (production would use user-agent parser crate)
        if ua.contains("Windows") {
            self.os = Some("Windows".to_string());
        } else if ua.contains("Mac OS X") || ua.contains("Macintosh") {
            self.os = Some("macOS".to_string());
        } else if ua.contains("Linux") {
            self.os = Some("Linux".to_string());
        } else if ua.contains("Android") {
            self.os = Some("Android".to_string());
        } else if ua.contains("iPhone") || ua.contains("iPad") {
            self.os = Some("iOS".to_string());
        }

        if ua.contains("Chrome") && !ua.contains("Edg") {
            self.browser = Some("Chrome".to_string());
        } else if ua.contains("Firefox") {
            self.browser = Some("Firefox".to_string());
        } else if ua.contains("Safari") && !ua.contains("Chrome") {
            self.browser = Some("Safari".to_string());
        } else if ua.contains("Edg") {
            self.browser = Some("Edge".to_string());
        }
    }

    /// Generate fingerprint hash
    pub fn fingerprint_hash(&self) -> String {
        let mut hasher = blake3::Hasher::new();

        if let Some(ua) = &self.user_agent {
            hasher.update(ua.as_bytes());
        }
        if let Some(lang) = &self.accept_language {
            hasher.update(lang.as_bytes());
        }
        if let Some(enc) = &self.accept_encoding {
            hasher.update(enc.as_bytes());
        }
        if let Some(tz) = self.timezone_offset {
            hasher.update(&tz.to_le_bytes());
        }

        hex::encode(hasher.finalize().as_bytes())
    }
}

/// Metadata about a network flow for policy evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowMeta {
    /// Unique flow identifier
    pub flow_id: [u8; 32],
    /// When the flow was initiated
    pub timestamp: DateTime<Utc>,
    /// Source address (local)
    pub source: SocketAddr,
    /// Destination address
    pub destination: SocketAddr,
    /// Destination hostname (from DNS/SNI)
    pub hostname: Option<String>,
    /// Protocol
    pub protocol: FlowProtocol,
    /// HTTP method (if applicable)
    pub http_method: Option<String>,
    /// Request path (if applicable)
    pub request_path: Option<String>,
    /// Content-Type header (if available)
    pub content_type: Option<String>,
    /// User-Agent header (if available)
    pub user_agent: Option<String>,
    /// Request size in bytes
    pub request_size: u64,
    /// TLS SNI (Server Name Indication)
    pub sni: Option<String>,
    /// TLS certificate fingerprint (SHA-256)
    pub cert_fingerprint: Option<[u8; 32]>,
    /// Process ID that initiated the connection (if available)
    pub process_id: Option<u32>,
    /// Process name (if available)
    pub process_name: Option<String>,
    /// Inferred telemetry intent
    pub inferred_intent: TelemetryIntent,
    /// Risk level
    pub risk_level: RiskLevel,
    /// Geolocation of destination IP
    #[cfg(feature = "geolocation")]
    pub geolocation: Option<GeoLocation>,
    #[cfg(not(feature = "geolocation"))]
    pub geolocation: Option<String>, // Placeholder
    /// Device fingerprint information
    pub device_fingerprint: DeviceFingerprint,
}

impl FlowMeta {
    /// Create a new FlowMeta with minimal information
    pub fn new(source: SocketAddr, destination: SocketAddr) -> Self {
        let mut flow_id = [0u8; 32];
        let mut hasher = blake3::Hasher::new();
        hasher.update(&source.ip().to_string().as_bytes());
        hasher.update(&source.port().to_le_bytes());
        hasher.update(&destination.ip().to_string().as_bytes());
        hasher.update(&destination.port().to_le_bytes());
        hasher.update(&chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0).to_le_bytes());
        flow_id.copy_from_slice(hasher.finalize().as_bytes());

        Self {
            flow_id,
            timestamp: Utc::now(),
            source,
            destination,
            hostname: None,
            protocol: FlowProtocol::Unknown,
            http_method: None,
            request_path: None,
            content_type: None,
            user_agent: None,
            request_size: 0,
            sni: None,
            cert_fingerprint: None,
            process_id: None,
            process_name: None,
            inferred_intent: TelemetryIntent::Unknown,
            risk_level: RiskLevel::Medium,
            geolocation: None,
            device_fingerprint: DeviceFingerprint::default(),
        }
    }

    /// Create from HTTP request metadata
    pub fn from_http(
        source: SocketAddr,
        destination: SocketAddr,
        hostname: &str,
        method: &str,
        path: &str,
    ) -> Self {
        let mut meta = Self::new(source, destination);
        meta.hostname = Some(hostname.to_string());
        meta.protocol = if destination.port() == 443 {
            FlowProtocol::Https
        } else {
            FlowProtocol::Http
        };
        meta.http_method = Some(method.to_string());
        meta.request_path = Some(path.to_string());
        meta.sni = Some(hostname.to_string());
        meta
    }

    /// Set the inferred intent and recalculate risk
    pub fn with_intent(mut self, intent: TelemetryIntent) -> Self {
        self.inferred_intent = intent;
        self.risk_level = match intent {
            TelemetryIntent::Advertising | TelemetryIntent::BehaviorTracking => RiskLevel::Critical,
            TelemetryIntent::UsageAnalytics => RiskLevel::High,
            TelemetryIntent::Diagnostics | TelemetryIntent::Performance => RiskLevel::Medium,
            TelemetryIntent::CrashReport | TelemetryIntent::Heartbeat => RiskLevel::Low,
            TelemetryIntent::LicenseCheck | TelemetryIntent::Security => RiskLevel::Low,
            TelemetryIntent::FeatureFlags => RiskLevel::Medium,
            TelemetryIntent::Unknown => RiskLevel::Medium,
        };
        self
    }

    /// Get the effective hostname (SNI > hostname > IP)
    pub fn effective_hostname(&self) -> String {
        self.sni
            .clone()
            .or_else(|| self.hostname.clone())
            .unwrap_or_else(|| self.destination.ip().to_string())
    }

    /// Get destination IP
    pub fn destination_ip(&self) -> IpAddr {
        self.destination.ip()
    }

    /// Get destination port
    pub fn destination_port(&self) -> u16 {
        self.destination.port()
    }

    /// Check if this looks like telemetry based on path patterns
    pub fn path_suggests_telemetry(&self) -> bool {
        if let Some(path) = &self.request_path {
            let path_lower = path.to_lowercase();
            let telemetry_patterns = [
                "/telemetry",
                "/analytics",
                "/collect",
                "/events",
                "/metrics",
                "/beacon",
                "/pixel",
                "/track",
                "/log",
                "/report",
                "/crash",
                "/diagnostic",
                "/usage",
                "/stats",
                "/ping",
                "/heartbeat",
                "/_/",
                "/v1/t",
                "/v2/t",
                "/batch",
            ];

            telemetry_patterns.iter().any(|p| path_lower.contains(p))
        } else {
            false
        }
    }

    /// Generate a privacy-safe summary (no PII)
    pub fn privacy_summary(&self) -> FlowSummary {
        FlowSummary {
            flow_id_prefix: hex::encode(&self.flow_id[..8]),
            timestamp: self.timestamp,
            destination_domain: self.effective_hostname(),
            protocol: self.protocol,
            intent: self.inferred_intent,
            risk_level: self.risk_level,
            size_bytes: self.request_size,
        }
    }
}

/// Privacy-safe summary of a flow (no PII, suitable for logging)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowSummary {
    /// Truncated flow ID (first 8 bytes as hex)
    pub flow_id_prefix: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Destination domain
    pub destination_domain: String,
    /// Protocol
    pub protocol: FlowProtocol,
    /// Inferred intent
    pub intent: TelemetryIntent,
    /// Risk level
    pub risk_level: RiskLevel,
    /// Size in bytes
    pub size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddrV4};

    fn test_socket() -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080))
    }

    fn test_dest() -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(1, 2, 3, 4), 443))
    }

    #[test]
    fn test_flow_meta_creation() {
        let meta = FlowMeta::new(test_socket(), test_dest());
        assert_eq!(meta.protocol, FlowProtocol::Unknown);
        assert_eq!(meta.inferred_intent, TelemetryIntent::Unknown);
    }

    #[test]
    fn test_http_flow_meta() {
        let meta = FlowMeta::from_http(
            test_socket(),
            test_dest(),
            "analytics.example.com",
            "POST",
            "/v1/collect",
        );

        assert_eq!(meta.protocol, FlowProtocol::Https);
        assert!(meta.path_suggests_telemetry());
    }

    #[test]
    fn test_intent_risk_mapping() {
        let meta = FlowMeta::new(test_socket(), test_dest())
            .with_intent(TelemetryIntent::Advertising);

        assert_eq!(meta.risk_level, RiskLevel::Critical);
        assert!(meta.inferred_intent.is_invasive());
    }

    #[test]
    fn test_telemetry_path_detection() {
        let paths = vec![
            ("/api/telemetry", true),
            ("/v1/collect", true),
            ("/events/batch", true),
            ("/api/users", false),
            ("/index.html", false),
            ("/analytics/data", true),
        ];

        for (path, expected) in paths {
            let mut meta = FlowMeta::new(test_socket(), test_dest());
            meta.request_path = Some(path.to_string());
            assert_eq!(
                meta.path_suggests_telemetry(),
                expected,
                "Path {} should be telemetry: {}",
                path,
                expected
            );
        }
    }

    #[test]
    fn test_privacy_summary() {
        let meta = FlowMeta::from_http(
            test_socket(),
            test_dest(),
            "tracking.example.com",
            "POST",
            "/collect",
        );

        let summary = meta.privacy_summary();
        assert_eq!(summary.destination_domain, "tracking.example.com");
        assert_eq!(summary.flow_id_prefix.len(), 16); // 8 bytes = 16 hex chars
    }
}
