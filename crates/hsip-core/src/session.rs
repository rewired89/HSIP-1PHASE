//! Minimal HSIP session helpers (nonce + AEAD)
//!
//! This module does two things:
//! 1. Low-level counter-based AEAD helpers (`seal_with_counter` / `open_with_counter`)
//! 2. A higher-level `ManagedSession` with nonce + rekey policy:
//!    - Monotonic nonces with a per-session salt
//!    - Age-based + packet-count-based rekey triggers

use std::time::{Duration, Instant};

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305, Key, Nonce,
};

/// How long a single session is allowed to live before it force a rekey.
const MAX_SESSION_AGE: Duration = Duration::from_secs(60 * 60);

/// How many packets is allow under a single key before forcing rekey.
const MAX_PACKETS_BEFORE_REKEY: u64 = 100_000;

const MAX_NONCE_COUNTER: u64 = u64::MAX - 1;

/// Errors that can occur when sealing/opening HSIP session data.
#[derive(Debug)]
pub enum SessionError {
    /// Nonce counter in AAD didn't match what the caller expected.
    NonceMismatch { expected: u64, got: u64 },

    /// Underlying AEAD failure (encrypt/decrypt).
    Crypto(&'static str),

    /// If it ran out of safe nonce space under a single key.
    NonceExhausted,

    /// Policy says we must rekey (age or packet-count limit).
    RekeyRequired,

    /// Consent was revoked during active session.
    ConsentRevoked,
}

/// Tiny AAD carrier for nonce integrity.
/// Caller supplies a monotonic counter and both sides must agree.
#[derive(Debug, Clone, Copy)]
pub struct AeadMeta {
    /// Monotonic counter supplied by the caller.
    pub nonce_counter: u64,
}

impl AeadMeta {
    pub fn new(nonce_counter: u64) -> Self {
        Self { nonce_counter }
    }
}

/// Internal helper: derive a 96-bit nonce from a u64 counter.
///
/// Layout: [ 0 0 0 0 | counter_be(8 bytes) ]
///
/// NOTE: This is low-level and does **not** enforce monotonicity on its own.
/// The caller must ensure 'counter' never repeats under a given key.
fn nonce_from_counter(counter: u64) -> Nonce {
    let mut n = [0u8; 12];
    // put the counter in the last 8 bytes (big-endian)
    n[4..].copy_from_slice(&counter.to_be_bytes());
    n.into()
}

/// Seal 'plaintext' with ChaCha20-Poly1305 using a 32-byte key and a
/// counter-based nonce.
///
/// The caller is responsible for incrementing 'meta.nonce_counter' per packet
/// and never reusing the same (key, counter) pair.
pub fn seal_with_counter(
    key_bytes: &[u8; 32],
    meta: &AeadMeta,
    plaintext: &[u8],
) -> Result<Vec<u8>, SessionError> {
    let key: Key = (*key_bytes).into();
    let cipher = ChaCha20Poly1305::new(&key);
    let nonce = nonce_from_counter(meta.nonce_counter);

    cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| SessionError::Crypto("encrypt"))
}

/// Open `ciphertext` with ChaCha20-Poly1305 using a 32-byte key and a
/// counter-based nonce, **verifying the counter matches** what the caller
/// expected.
///
/// Returns `(tag, plaintext)` where `tag` is currently always `0` and kept
/// for forward-compatibility with future framing.
pub fn open_with_counter(
    key_bytes: &[u8; 32],
    expected_counter: u64,
    meta: &AeadMeta,
    ciphertext: &[u8],
) -> Result<(u8, Vec<u8>), SessionError> {
    // First enforce nonce-counter integrity
    if meta.nonce_counter != expected_counter {
        return Err(SessionError::NonceMismatch {
            expected: expected_counter,
            got: meta.nonce_counter,
        });
    }

    let key: Key = (*key_bytes).into();
    let cipher = ChaCha20Poly1305::new(&key);
    let nonce = nonce_from_counter(meta.nonce_counter);

    let pt = cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|_| SessionError::Crypto("decrypt"))?;

    // `tag` is a placeholder for future framing versions
    Ok((0u8, pt))
}

//
// ──────────────────────────────────────────────
//   Managed session: nonce + rekey policy
// ──────────────────────────────────────────────
//

/// Internal salt used to derive nonces as:
/// [ salt(4 bytes) | counter_be(8 bytes) ]
#[derive(Clone, Copy)]
struct SessionNonceSalt {
    salt: [u8; 4],
}

impl SessionNonceSalt {
    fn new(salt: [u8; 4]) -> Self {
        Self { salt }
    }

    fn derive(&self, counter: u64) -> Result<Nonce, SessionError> {
        if counter > MAX_NONCE_COUNTER {
            return Err(SessionError::NonceExhausted);
        }

        let mut bytes = [0u8; 12];
        bytes[0..4].copy_from_slice(&self.salt);
        bytes[4..12].copy_from_slice(&counter.to_be_bytes());

        Ok(bytes.into())
    }
}

/// Higher-level session wrapper that:
/// - Enforces nonce uniqueness (monotonic counter per key)
/// - Enforces rekey after MAX_SESSION_AGE or MAX_PACKETS_BEFORE_REKEY
/// - Checks consent revocation during active sessions
///
/// This does **not** do the handshake; it just manages AEAD usage safely.
pub struct ManagedSession {
    cipher: ChaCha20Poly1305,
    nonce_salt: SessionNonceSalt,
    started_at: Instant,
    packets_sent: u64,
    /// Optional consent check function called before encrypt/decrypt.
    /// Returns true if consent is still valid, false if revoked.
    consent_check: Option<Box<dyn Fn() -> bool + Send + Sync>>,
}

impl ManagedSession {
    /// Create a new managed session.
    ///
    /// - `key_bytes`: 32-byte AEAD key derived from the handshake
    /// - `nonce_salt`: 4 random bytes from the handshake (per-session)
    pub fn new(key_bytes: &[u8; 32], nonce_salt: [u8; 4]) -> Self {
        let key: Key = (*key_bytes).into();
        let cipher = ChaCha20Poly1305::new(&key);
        let nonce_salt = SessionNonceSalt::new(nonce_salt);

        ManagedSession {
            cipher,
            nonce_salt,
            started_at: Instant::now(),
            packets_sent: 0,
            consent_check: None,
        }
    }

    /// Set a consent check callback that will be called before each encrypt/decrypt.
    /// The callback should return true if consent is still valid, false if revoked.
    pub fn with_consent_check<F>(mut self, check: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.consent_check = Some(Box::new(check));
        self
    }

    /// Attach a consent check callback to an existing session.
    /// This enables instant session termination when consent is revoked.
    ///
    /// The callback will be invoked on every encrypt/decrypt operation.
    /// If it returns false, the operation fails with SessionError::ConsentRevoked.
    pub fn attach_consent_check<F>(&mut self, check: F)
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.consent_check = Some(Box::new(check));
    }

    /// Internal: enforce age/packet limits and consent status for this key.
    fn check_limits(&self) -> Result<(), SessionError> {
        // Check consent first (cheapest check)
        if let Some(ref check) = self.consent_check {
            if !check() {
                return Err(SessionError::ConsentRevoked);
            }
        }

        if self.started_at.elapsed() >= MAX_SESSION_AGE {
            return Err(SessionError::RekeyRequired);
        }

        if self.packets_sent >= MAX_PACKETS_BEFORE_REKEY {
            return Err(SessionError::RekeyRequired);
        }

        Ok(())
    }

    /// Encrypt a payload with AAD.
    ///
    /// Returns `(counter, ciphertext)`. The caller is expected to:
    /// - Track the packet counter on the wire
    /// - Use that same counter when calling `decrypt`.
    pub fn encrypt(
        &mut self,
        plaintext: &[u8],
        aad: &[u8],
    ) -> Result<(u64, Vec<u8>), SessionError> {
        // Enforce policy before sending anything new.
        self.check_limits()?;

        let counter = self.packets_sent;
        let nonce = self.nonce_salt.derive(counter)?;

        let payload = Payload {
            msg: plaintext,
            aad,
        };

        let ct = self
            .cipher
            .encrypt(&nonce, payload)
            .map_err(|_| SessionError::Crypto("encrypt"))?;

        // Only bump the counter if we actually succeeded.
        self.packets_sent = self
            .packets_sent
            .checked_add(1)
            .ok_or(SessionError::NonceExhausted)?;

        Ok((counter, ct))
    }

    /// Encrypt with automatic traffic shaping (padding + timing jitter)
    ///
    /// This is the recommended method for privacy-sensitive applications.
    /// Adds constant-size padding and timing jitter to prevent metadata analysis.
    pub fn encrypt_with_shaping(
        &mut self,
        plaintext: &[u8],
        aad: &[u8],
    ) -> Result<(u64, Vec<u8>), SessionError> {
        // Add padding to normalize packet sizes
        let padded = crate::traffic_shaping::add_padding(plaintext);

        // Apply timing jitter before sending
        crate::traffic_shaping::apply_timing_jitter();

        // Encrypt the padded plaintext
        self.encrypt(&padded, aad)
    }

    /// Decrypt a payload given the packet `counter` and AAD.
    ///
    /// The caller must pass the same `counter` that was used by the sender.
    pub fn decrypt(
        &self,
        counter: u64,
        ciphertext: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, SessionError> {
        // Check consent before allowing decrypt
        self.check_limits()?;

        let nonce = self.nonce_salt.derive(counter)?;

        let payload = Payload {
            msg: ciphertext,
            aad,
        };

        let pt = self
            .cipher
            .decrypt(&nonce, payload)
            .map_err(|_| SessionError::Crypto("decrypt"))?;

        Ok(pt)
    }

    /// Decrypt with automatic padding removal
    ///
    /// Use this when receiving packets encrypted with `encrypt_with_shaping`.
    pub fn decrypt_with_shaping(
        &self,
        counter: u64,
        ciphertext: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, SessionError> {
        // Decrypt first
        let padded = self.decrypt(counter, ciphertext, aad)?;

        // Remove padding
        crate::traffic_shaping::remove_padding(&padded)
            .map_err(|e| SessionError::Crypto(e))
    }

    /// Expose basic stats for monitoring / logging.
    pub fn stats(&self) -> (Duration, u64) {
        (self.started_at.elapsed(), self.packets_sent)
    }
}
