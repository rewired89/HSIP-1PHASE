pub mod prefix;

/// Maximum size for HELLO messages (bytes)
pub const MAX_HELLO_SIZE: usize = 1024;

/// Maximum size for consent request messages (bytes)
pub const MAX_CONSENT_REQUEST_SIZE: usize = 2048;

/// Maximum size for consent response messages (bytes)
pub const MAX_CONSENT_RESPONSE_SIZE: usize = 2048;

/// Maximum size for control frames (bytes)
pub const MAX_CONTROL_FRAME_SIZE: usize = 4096;
