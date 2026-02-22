//! At-rest encryption for Ed25519 private signing keys.
//!
//! Uses ChaCha20-Poly1305 with a key derived from HSIP_MASTER_KEY env var.
//! If HSIP_MASTER_KEY is not set the server will refuse to start.
//!
//! Encrypted format (Base64-encoded):
//!   [ nonce(12 bytes) | ciphertext+tag(32+16 bytes) ]

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chacha20poly1305::{ChaCha20Poly1305, Key, KeyInit, Nonce, aead::Aead};
use hkdf::Hkdf;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::Sha256;

const HKDF_INFO: &[u8] = b"hsip-key-encryption-v1";

/// Derive a 32-byte encryption key from the master key using HKDF-SHA256.
fn derive_encryption_key(master_key: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(None, master_key);
    let mut okm = [0u8; 32];
    hk.expand(HKDF_INFO, &mut okm).expect("HKDF expand failed");
    okm
}

/// Load the master key from the HSIP_MASTER_KEY environment variable.
/// Panics at startup if not set — do not call after startup.
pub fn load_master_key() -> Vec<u8> {
    let val = std::env::var("HSIP_MASTER_KEY").unwrap_or_default();
    if val.is_empty() {
        eprintln!();
        eprintln!("FATAL: HSIP_MASTER_KEY environment variable is not set.");
        eprintln!("Generate one with:  openssl rand -hex 32");
        eprintln!("Then set it:        export HSIP_MASTER_KEY=<value>");
        eprintln!();
        std::process::exit(1);
    }
    hex::decode(&val).unwrap_or_else(|_| {
        eprintln!("FATAL: HSIP_MASTER_KEY must be a hex-encoded 32-byte value.");
        eprintln!("Generate one with:  openssl rand -hex 32");
        std::process::exit(1);
    })
}

/// Encrypt a 32-byte Ed25519 signing key.
/// Returns a Base64 string: nonce(12) || ciphertext+tag(48).
pub fn encrypt_signing_key(key_bytes: &[u8; 32], master_key: &[u8]) -> String {
    let enc_key = derive_encryption_key(master_key);
    let cipher  = ChaCha20Poly1305::new(Key::from_slice(&enc_key));

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, key_bytes.as_slice())
        .expect("key encryption failed");

    let mut payload = Vec::with_capacity(12 + ciphertext.len());
    payload.extend_from_slice(&nonce_bytes);
    payload.extend_from_slice(&ciphertext);

    BASE64.encode(payload)
}

/// Decrypt a previously encrypted signing key.
pub fn decrypt_signing_key(encrypted_b64: &str, master_key: &[u8]) -> anyhow::Result<[u8; 32]> {
    let raw = BASE64.decode(encrypted_b64)
        .map_err(|e| anyhow::anyhow!("base64 decode failed: {e}"))?;

    if raw.len() < 12 {
        anyhow::bail!("encrypted key too short");
    }

    let (nonce_bytes, ciphertext) = raw.split_at(12);
    let enc_key = derive_encryption_key(master_key);
    let cipher  = ChaCha20Poly1305::new(Key::from_slice(&enc_key));
    let nonce   = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("key decryption failed — wrong HSIP_MASTER_KEY?"))?;

    plaintext.try_into()
        .map_err(|_| anyhow::anyhow!("decrypted key has wrong length"))
}
