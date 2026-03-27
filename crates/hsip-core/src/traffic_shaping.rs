//! Traffic shaping for metadata protection
//!
//! Mitigates traffic analysis attacks by normalizing packet sizes and timing

use rand::Rng;

/// Target packet sizes for padding (in bytes)
///
/// These are MTU-safe sizes that prevent fragmentation while
/// making it harder to infer content length from packet size
const PAD_TARGETS: &[usize] = &[512, 1024, 1200];

/// Add padding to plaintext to reach nearest target size
///
/// This prevents traffic analysis based on packet size patterns.
/// Padding format: [original data][0x80][random bytes][padding length as u16 BE]
///
/// Returns: padded plaintext
pub fn add_padding(plaintext: &[u8]) -> Vec<u8> {
    let original_len = plaintext.len();

    // Find next target size
    let target = PAD_TARGETS
        .iter()
        .find(|&&t| t > original_len + 3) // +3 for padding marker + length
        .copied()
        .unwrap_or(1200); // Default to MAX_SESSION_PACKET_SIZE if too large

    let pad_len = target - original_len - 3; // -3 for marker (1) + length (2)

    let mut result = Vec::with_capacity(target);
    result.extend_from_slice(plaintext);
    result.push(0x80); // Padding marker (ISO 7816-4 padding)

    // Random padding bytes (makes padding non-deterministic)
    let mut rng = rand::thread_rng();
    for _ in 0..pad_len {
        result.push(rng.gen::<u8>());
    }

    // Append padding length as u16 BE
    result.extend_from_slice(&(pad_len as u16).to_be_bytes());

    result
}

/// Remove padding from decrypted plaintext
///
/// Returns: original plaintext without padding
pub fn remove_padding(padded: &[u8]) -> Result<Vec<u8>, &'static str> {
    if padded.len() < 3 {
        return Err("packet too short for padding");
    }

    // Read padding length from last 2 bytes
    let len_bytes = &padded[padded.len() - 2..];
    let pad_len = u16::from_be_bytes([len_bytes[0], len_bytes[1]]) as usize;

    if pad_len + 3 > padded.len() {
        return Err("invalid padding length");
    }

    let data_end = padded.len() - pad_len - 3;

    // Verify padding marker
    if padded[data_end] != 0x80 {
        return Err("invalid padding marker");
    }

    Ok(padded[..data_end].to_vec())
}

/// Add timing jitter to prevent traffic analysis
///
/// Introduces random delay (±50-200ms) before sending packets
/// to obscure timing patterns and prevent correlation attacks
pub fn apply_timing_jitter() {
    let mut rng = rand::thread_rng();
    let jitter_ms = rng.gen_range(50..=200);
    std::thread::sleep(std::time::Duration::from_millis(jitter_ms));
}

/// Configuration for traffic shaping
#[derive(Clone, Debug)]
pub struct TrafficShapingConfig {
    /// Enable packet padding to constant sizes
    pub enable_padding: bool,
    /// Enable timing jitter (random delays)
    pub enable_timing_jitter: bool,
    /// Send periodic cover traffic (dummy packets)
    pub enable_cover_traffic: bool,
    /// Cover traffic interval in milliseconds
    pub cover_traffic_interval_ms: u64,
}

impl Default for TrafficShapingConfig {
    fn default() -> Self {
        Self {
            enable_padding: true, // Enabled by default for privacy
            enable_timing_jitter: true, // Enabled by default
            enable_cover_traffic: false, // Opt-in (bandwidth overhead)
            cover_traffic_interval_ms: 5000, // 5 seconds
        }
    }
}

impl TrafficShapingConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let enable_padding = std::env::var("HSIP_DISABLE_PADDING").is_err();
        let enable_timing_jitter = std::env::var("HSIP_DISABLE_TIMING_JITTER").is_err();
        let enable_cover_traffic = std::env::var("HSIP_ENABLE_COVER_TRAFFIC").is_ok();

        let cover_traffic_interval_ms = std::env::var("HSIP_COVER_TRAFFIC_INTERVAL_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(5000);

        Self {
            enable_padding,
            enable_timing_jitter,
            enable_cover_traffic,
            cover_traffic_interval_ms,
        }
    }

    /// Print configuration banner
    pub fn print_banner(&self) {
        println!("[traffic-shaping] Configuration:");
        println!("  Padding: {}", if self.enable_padding { "ENABLED" } else { "DISABLED" });
        println!("  Timing jitter: {}", if self.enable_timing_jitter { "ENABLED" } else { "DISABLED" });
        println!("  Cover traffic: {}", if self.enable_cover_traffic { "ENABLED" } else { "DISABLED" });
        if self.enable_cover_traffic {
            println!("    Interval: {}ms", self.cover_traffic_interval_ms);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_padding_roundtrip() {
        let original = b"Hello, HSIP!";
        let padded = add_padding(original);

        // Should be padded to at least 512 bytes
        assert!(padded.len() >= 512);

        let recovered = remove_padding(&padded).unwrap();
        assert_eq!(recovered, original);
    }

    #[test]
    fn test_padding_sizes() {
        // Small message should pad to 512
        let small = b"x";
        let padded_small = add_padding(small);
        assert_eq!(padded_small.len(), 512);

        // Medium message should pad to 1024
        let medium = vec![0u8; 600];
        let padded_medium = add_padding(&medium);
        assert_eq!(padded_medium.len(), 1024);

        // Large message should pad to 1200
        let large = vec![0u8; 1100];
        let padded_large = add_padding(&large);
        assert_eq!(padded_large.len(), 1200);
    }

    #[test]
    fn test_invalid_padding() {
        // Too short
        assert!(remove_padding(b"xx").is_err());

        // Invalid marker
        let mut bad = vec![0u8; 512];
        bad[509] = 0xFF; // Wrong marker
        assert!(remove_padding(&bad).is_err());
    }
}
