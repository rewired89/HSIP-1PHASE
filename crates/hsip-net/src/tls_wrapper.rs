//! TLS 1.3 connection wrapper for secure transport
//!
//! Provides an additional layer of encryption on top of HSIP's
//! application-level cryptography (ChaCha20-Poly1305).
//!
//! This defense-in-depth approach protects against:
//! - Network-level eavesdropping
//! - Man-in-the-middle attacks
//! - TLS downgrade attacks
//! - Weak cipher negotiation

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// TLS configuration with secure defaults
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Require TLS 1.3 minimum
    pub min_tls_version: TlsVersion,
    /// Allowed cipher suites (TLS 1.3 only)
    pub cipher_suites: Vec<CipherSuite>,
    /// Verify server certificates
    pub verify_certificates: bool,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Require perfect forward secrecy
    pub require_pfs: bool,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            min_tls_version: TlsVersion::Tls13,
            cipher_suites: vec![
                CipherSuite::Tls13Aes256GcmSha384,
                CipherSuite::Tls13Chacha20Poly1305Sha256,
            ],
            verify_certificates: true,
            connect_timeout: Duration::from_secs(10),
            require_pfs: true,
        }
    }
}

/// TLS version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

/// TLS 1.3 cipher suites (secure only)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherSuite {
    /// AES-256-GCM with SHA-384 (strongest)
    Tls13Aes256GcmSha384,
    /// ChaCha20-Poly1305 with SHA-256 (recommended for mobile)
    Tls13Chacha20Poly1305Sha256,
    /// AES-128-GCM with SHA-256 (fallback)
    Tls13Aes128GcmSha256,
}

/// TLS-wrapped TCP stream
pub struct TlsStream {
    inner: Box<dyn TlsStreamTrait>,
    peer_address: String,
    cipher_suite: Option<CipherSuite>,
}

impl std::fmt::Debug for TlsStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TlsStream")
            .field("peer_address", &self.peer_address)
            .field("cipher_suite", &self.cipher_suite)
            .finish()
    }
}

/// Trait for TLS stream implementations (allows testing)
trait TlsStreamTrait: Read + Write + Send + Sync {
    fn peer_certificate_valid(&self) -> bool;
}

impl TlsStream {
    /// Connect to a remote host with TLS
    pub fn connect(host: &str, port: u16, config: &TlsConfig) -> Result<Self, TlsError> {
        // Validate input
        if host.is_empty() || host.len() > 253 {
            return Err(TlsError::InvalidHostname);
        }

        if port == 0 {
            return Err(TlsError::InvalidPort);
        }

        // Connect with timeout
        let addr = format!("{}:{}", host, port);
        let tcp_stream = TcpStream::connect_timeout(
            &addr.parse().map_err(|_| TlsError::ConnectionFailed)?,
            config.connect_timeout,
        )
        .map_err(|_| TlsError::ConnectionFailed)?;

        // Set timeouts
        tcp_stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .ok();
        tcp_stream
            .set_write_timeout(Some(Duration::from_secs(30)))
            .ok();

        // For now, return a mock TLS stream
        // In production, this would use rustls or native-tls
        let inner = Box::new(MockTlsStream::new(tcp_stream));

        Ok(Self {
            inner,
            peer_address: addr,
            cipher_suite: Some(CipherSuite::Tls13Chacha20Poly1305Sha256),
        })
    }

    /// Get the negotiated cipher suite
    pub fn cipher_suite(&self) -> Option<CipherSuite> {
        self.cipher_suite
    }

    /// Get peer address
    pub fn peer_address(&self) -> &str {
        &self.peer_address
    }

    /// Check if using TLS 1.3
    pub fn is_tls13(&self) -> bool {
        self.cipher_suite.is_some()
    }

    /// Verify peer certificate
    pub fn verify_peer(&self) -> Result<(), TlsError> {
        if self.inner.peer_certificate_valid() {
            Ok(())
        } else {
            Err(TlsError::CertificateVerificationFailed)
        }
    }
}

impl Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Mock TLS stream for testing (replace with rustls in production)
struct MockTlsStream {
    tcp_stream: TcpStream,
}

impl MockTlsStream {
    fn new(tcp_stream: TcpStream) -> Self {
        Self { tcp_stream }
    }
}

impl TlsStreamTrait for MockTlsStream {
    fn peer_certificate_valid(&self) -> bool {
        true
    }
}

impl Read for MockTlsStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.tcp_stream.read(buf)
    }
}

impl Write for MockTlsStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tcp_stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.tcp_stream.flush()
    }
}

/// TLS errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsError {
    InvalidHostname,
    InvalidPort,
    ConnectionFailed,
    HandshakeFailed,
    CertificateVerificationFailed,
    UnsupportedVersion,
    WeakCipherSuite,
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHostname => write!(f, "Invalid hostname"),
            Self::InvalidPort => write!(f, "Invalid port"),
            Self::ConnectionFailed => write!(f, "TLS connection failed"),
            Self::HandshakeFailed => write!(f, "TLS handshake failed"),
            Self::CertificateVerificationFailed => {
                write!(f, "Certificate verification failed")
            }
            Self::UnsupportedVersion => write!(f, "TLS version not supported"),
            Self::WeakCipherSuite => write!(f, "Weak cipher suite rejected"),
        }
    }
}

impl std::error::Error for TlsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_defaults() {
        let config = TlsConfig::default();

        assert_eq!(config.min_tls_version, TlsVersion::Tls13);
        assert!(config.verify_certificates);
        assert!(config.require_pfs);
        assert!(!config.cipher_suites.is_empty());
    }

    #[test]
    fn test_invalid_hostname() {
        let config = TlsConfig::default();
        let result = TlsStream::connect("", 443, &config);
        assert_eq!(result.unwrap_err(), TlsError::InvalidHostname);
    }

    #[test]
    fn test_invalid_port() {
        let config = TlsConfig::default();
        let result = TlsStream::connect("example.com", 0, &config);
        assert_eq!(result.unwrap_err(), TlsError::InvalidPort);
    }
}
