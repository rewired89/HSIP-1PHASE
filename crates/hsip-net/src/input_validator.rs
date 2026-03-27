//! Input validation and sanitization to prevent injection attacks
//!
//! Validates all external inputs before processing

use std::net::IpAddr;

/// Maximum sizes for various inputs (prevents fragmentation and memory exhaustion)
pub const MAX_MESSAGE_SIZE: usize = 1100;
pub const MAX_CONSENT_PURPOSE_LENGTH: usize = 512;
pub const MAX_DESTINATION_LENGTH: usize = 253;
pub const MAX_PEER_ID_LENGTH: usize = 64;
pub const MAX_SIGNATURE_LENGTH: usize = 128;
pub const MAX_PUBLIC_KEY_LENGTH: usize = 64;
pub const MAX_NONCE_LENGTH: usize = 32;

/// Input validation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    TooLarge(String),
    InvalidFormat(String),
    InvalidCharacters(String),
    Empty(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge(field) => write!(f, "Field '{}' exceeds maximum size", field),
            Self::InvalidFormat(field) => write!(f, "Field '{}' has invalid format", field),
            Self::InvalidCharacters(field) => {
                write!(f, "Field '{}' contains invalid characters", field)
            }
            Self::Empty(field) => write!(f, "Field '{}' cannot be empty", field),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validates a destination (domain or IP)
pub fn validate_destination(dest: &str) -> Result<(), ValidationError> {
    if dest.is_empty() {
        return Err(ValidationError::Empty("destination".to_string()));
    }

    if dest.len() > MAX_DESTINATION_LENGTH {
        return Err(ValidationError::TooLarge("destination".to_string()));
    }

    // Try parsing as IP first
    if dest.parse::<IpAddr>().is_ok() {
        return Ok(());
    }

    // Validate as domain name
    // Must contain only alphanumeric, dots, hyphens
    if !dest
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == ':')
    {
        return Err(ValidationError::InvalidCharacters(
            "destination".to_string(),
        ));
    }

    // Must not start or end with dot or hyphen
    if dest.starts_with('.') || dest.ends_with('.') || dest.starts_with('-') || dest.ends_with('-')
    {
        return Err(ValidationError::InvalidFormat("destination".to_string()));
    }

    Ok(())
}

/// Validates a peer ID (base32)
pub fn validate_peer_id(peer_id: &str) -> Result<(), ValidationError> {
    if peer_id.is_empty() {
        return Err(ValidationError::Empty("peer_id".to_string()));
    }

    if peer_id.len() > MAX_PEER_ID_LENGTH {
        return Err(ValidationError::TooLarge("peer_id".to_string()));
    }

    // Base32 characters only: A-Z, 2-7
    if !peer_id
        .chars()
        .all(|c| c.is_ascii_uppercase() || ('2'..='7').contains(&c))
    {
        return Err(ValidationError::InvalidCharacters("peer_id".to_string()));
    }

    Ok(())
}

/// Validates a hex string (signature, public key, etc.)
pub fn validate_hex_string(hex: &str, max_len: usize, field_name: &str) -> Result<(), ValidationError> {
    if hex.is_empty() {
        return Err(ValidationError::Empty(field_name.to_string()));
    }

    if hex.len() > max_len {
        return Err(ValidationError::TooLarge(field_name.to_string()));
    }

    // Must be valid hex
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ValidationError::InvalidCharacters(field_name.to_string()));
    }

    // Must be even length (pairs of hex digits)
    if hex.len() % 2 != 0 {
        return Err(ValidationError::InvalidFormat(field_name.to_string()));
    }

    Ok(())
}

/// Validates message size
pub fn validate_message_size(size: usize) -> Result<(), ValidationError> {
    if size == 0 {
        return Err(ValidationError::Empty("message".to_string()));
    }

    if size > MAX_MESSAGE_SIZE {
        return Err(ValidationError::TooLarge("message".to_string()));
    }

    Ok(())
}

/// Sanitizes a string for logging (removes control characters)
pub fn sanitize_for_log(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .take(256) // Limit log length
        .collect()
}

/// Validates a nonce (must be random-looking)
pub fn validate_nonce(nonce: &str) -> Result<(), ValidationError> {
    if nonce.is_empty() {
        return Err(ValidationError::Empty("nonce".to_string()));
    }

    if nonce.len() > MAX_NONCE_LENGTH {
        return Err(ValidationError::TooLarge("nonce".to_string()));
    }

    // Must be alphanumeric or base64 characters
    if !nonce
        .chars()
        .all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '=')
    {
        return Err(ValidationError::InvalidCharacters("nonce".to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_destination() {
        // Valid domains
        assert!(validate_destination("example.com").is_ok());
        assert!(validate_destination("sub.example.com").is_ok());

        // Valid IPs
        assert!(validate_destination("127.0.0.1").is_ok());
        assert!(validate_destination("::1").is_ok());

        // Invalid
        assert!(validate_destination("").is_err());
        assert!(validate_destination(&"a".repeat(300)).is_err());
        assert!(validate_destination(".example.com").is_err());
        assert!(validate_destination("example.com.").is_err());
        assert!(validate_destination("ex ample.com").is_err()); // space
    }

    #[test]
    fn test_validate_peer_id() {
        // Valid base32
        assert!(validate_peer_id("ABCDEFGH234567").is_ok());

        // Invalid
        assert!(validate_peer_id("").is_err());
        assert!(validate_peer_id("abcdefgh").is_err()); // lowercase
        assert!(validate_peer_id("ABCDEFGH18").is_err()); // 0,1,8,9 not in base32
        assert!(validate_peer_id(&"A".repeat(100)).is_err()); // too long
    }

    #[test]
    fn test_validate_hex_string() {
        // Valid
        assert!(validate_hex_string("deadbeef", 16, "test").is_ok());
        assert!(validate_hex_string("DEADBEEF", 16, "test").is_ok());

        // Invalid
        assert!(validate_hex_string("", 16, "test").is_err()); // empty
        assert!(validate_hex_string("zzz", 16, "test").is_err()); // not hex
        assert!(validate_hex_string("abc", 16, "test").is_err()); // odd length
        assert!(validate_hex_string(&"aa".repeat(100), 16, "test").is_err()); // too long
    }

    #[test]
    fn test_sanitize_for_log() {
        assert_eq!(sanitize_for_log("hello"), "hello");
        assert_eq!(sanitize_for_log("hello\nworld"), "hello\nworld");
        assert_eq!(sanitize_for_log("hello\x00world"), "helloworld"); // null byte removed
    }
}
