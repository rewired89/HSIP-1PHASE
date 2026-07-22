//! At-rest encryption for Ed25519 private signing keys, and for sensitive
//! free-text fields stored in the database (message content, credential
//! claims — see `encrypt_field`/`decrypt_field` below).
//!
//! Uses ChaCha20-Poly1305 with a key derived from the master key (see
//! `main.rs::load_master_key` for where that comes from — file or
//! `HSIP_MASTER_KEY` env var).
//!
//! Encrypted format (Base64-encoded):
//!   [ nonce(12 bytes) | ciphertext+tag ]

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, Key, KeyInit, Nonce};
use hkdf::Hkdf;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::Sha256;

const HKDF_INFO: &[u8] = b"hsip-key-encryption-v1";
const HKDF_INFO_FIELD: &[u8] = b"hsip-field-encryption-v1";

/// Derive a 32-byte encryption key from the master key using HKDF-SHA256.
fn derive_encryption_key(master_key: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(None, master_key);
    let mut okm = [0u8; 32];
    hk.expand(HKDF_INFO, &mut okm).expect("HKDF expand failed");
    okm
}

/// Derive the field-encryption key — same master key, but a *different*
/// HKDF `info` string than `derive_encryption_key`, so a compromise of one
/// derived key's usage pattern (e.g. a nonce-reuse bug specific to one call
/// site) can't be leveraged against the other. Both still ultimately
/// depend on the same master key, so this is domain separation, not an
/// independent secret.
fn derive_field_encryption_key(master_key: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(None, master_key);
    let mut okm = [0u8; 32];
    hk.expand(HKDF_INFO_FIELD, &mut okm)
        .expect("HKDF expand failed");
    okm
}

/// Encrypt a 32-byte Ed25519 signing key.
/// Returns a Base64 string: nonce(12) || ciphertext+tag(48).
pub fn encrypt_signing_key(key_bytes: &[u8; 32], master_key: &[u8]) -> String {
    let enc_key = derive_encryption_key(master_key);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&enc_key));

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
    let raw = BASE64
        .decode(encrypted_b64)
        .map_err(|e| anyhow::anyhow!("base64 decode failed: {e}"))?;

    if raw.len() < 12 {
        anyhow::bail!("encrypted key too short");
    }

    let (nonce_bytes, ciphertext) = raw.split_at(12);
    let enc_key = derive_encryption_key(master_key);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&enc_key));
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("key decryption failed — wrong HSIP_MASTER_KEY?"))?;

    plaintext
        .try_into()
        .map_err(|_| anyhow::anyhow!("decrypted key has wrong length"))
}

/// Encrypt an arbitrary UTF-8 string field for at-rest storage — e.g.
/// `messages.content`, `credentials.claim`/`user_token`. Same
/// ChaCha20-Poly1305 primitive as `encrypt_signing_key`, but a
/// domain-separated derived key (`derive_field_encryption_key`) and no
/// fixed output length, since these fields are variable-length text
/// rather than a 32-byte key.
pub fn encrypt_field(plaintext: &str, master_key: &[u8]) -> String {
    let enc_key = derive_field_encryption_key(master_key);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&enc_key));

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .expect("field encryption failed");

    let mut payload = Vec::with_capacity(12 + ciphertext.len());
    payload.extend_from_slice(&nonce_bytes);
    payload.extend_from_slice(&ciphertext);

    BASE64.encode(payload)
}

/// Decrypt a field previously encrypted with `encrypt_field`.
pub fn decrypt_field(encrypted_b64: &str, master_key: &[u8]) -> anyhow::Result<String> {
    let raw = BASE64
        .decode(encrypted_b64)
        .map_err(|e| anyhow::anyhow!("base64 decode failed: {e}"))?;

    if raw.len() < 12 {
        anyhow::bail!("encrypted field too short");
    }

    let (nonce_bytes, ciphertext) = raw.split_at(12);
    let enc_key = derive_field_encryption_key(master_key);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&enc_key));
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("field decryption failed — wrong HSIP_MASTER_KEY?"))?;

    String::from_utf8(plaintext)
        .map_err(|e| anyhow::anyhow!("decrypted field is not valid UTF-8: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_roundtrips_through_encrypt_decrypt() {
        let master_key = [1u8; 32];
        let plaintext = "BUY 100 AAPL @ 191.20 — sensitive trade content";
        let encrypted = encrypt_field(plaintext, &master_key);
        assert_ne!(encrypted, plaintext, "ciphertext must not equal plaintext");
        assert!(
            !encrypted.contains("AAPL"),
            "ciphertext must not leak plaintext substrings"
        );
        let decrypted = decrypt_field(&encrypted, &master_key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn field_decryption_fails_under_the_wrong_master_key() {
        let master_key = [1u8; 32];
        let wrong_key = [2u8; 32];
        let encrypted = encrypt_field("secret content", &master_key);
        assert!(decrypt_field(&encrypted, &wrong_key).is_err());
    }

    #[test]
    fn field_decryption_fails_on_tampered_ciphertext() {
        let master_key = [1u8; 32];
        let encrypted = encrypt_field("secret content", &master_key);
        let mut raw = BASE64.decode(&encrypted).unwrap();
        let last = raw.len() - 1;
        raw[last] ^= 0xFF;
        let tampered = BASE64.encode(raw);
        assert!(decrypt_field(&tampered, &master_key).is_err());
    }

    #[test]
    fn field_and_signing_key_encryption_use_different_derived_keys() {
        // Domain separation must actually hold: the same 32 bytes,
        // encrypted as a "field" vs as a "signing key" under the same
        // master key, must not be interchangeable ciphertext — decrypting
        // a field-encrypted value with the signing-key derived key must
        // fail on AEAD tag verification, not merely on length.
        let master_key = [3u8; 32];
        let exactly_32_bytes = "01234567890123456789012345678901";
        assert_eq!(exactly_32_bytes.len(), 32);
        let as_field = encrypt_field(exactly_32_bytes, &master_key);
        assert!(decrypt_signing_key(&as_field, &master_key).is_err());
    }
}
