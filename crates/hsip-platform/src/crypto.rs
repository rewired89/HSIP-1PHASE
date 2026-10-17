//! Secrets handling: signing-key encryption at rest, password hashing,
//! bearer-token generation and hashing.

use anyhow::{anyhow, Result};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, Key, KeyInit, Nonce};
use hkdf::Hkdf;
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};

const KEY_INFO: &[u8] = b"hsip-prove-key-encryption-v1";

fn derive(master: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(None, master);
    let mut out = [0u8; 32];
    hk.expand(KEY_INFO, &mut out).expect("32-byte HKDF output");
    out
}

/// nonce(12) || ciphertext+tag, base64.
pub fn encrypt_secret(master: &[u8], plaintext: &[u8]) -> Result<String> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&derive(master)));
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let ct = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|_| anyhow!("encryption failed"))?;
    let mut out = nonce.to_vec();
    out.extend(ct);
    Ok(B64.encode(out))
}

pub fn decrypt_secret(master: &[u8], encoded: &str) -> Result<Vec<u8>> {
    let raw = B64
        .decode(encoded)
        .map_err(|_| anyhow!("bad ciphertext encoding"))?;
    if raw.len() < 13 {
        return Err(anyhow!("ciphertext too short"));
    }
    let (nonce, ct) = raw.split_at(12);
    ChaCha20Poly1305::new(Key::from_slice(&derive(master)))
        .decrypt(Nonce::from_slice(nonce), ct)
        .map_err(|_| anyhow!("decryption failed (wrong master key or corrupted data)"))
}

pub fn sha256_hex(s: &str) -> String {
    hex::encode(Sha256::digest(s.as_bytes()))
}

/// `prefix` + 43 URL-safe random chars (256 bits).
pub fn random_token(prefix: &str) -> String {
    let mut b = [0u8; 32];
    OsRng.fill_bytes(&mut b);
    format!("{prefix}{}", base64_url(&b))
}

fn base64_url(b: &[u8]) -> String {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    URL_SAFE_NO_PAD.encode(b)
}

fn argon(fast: bool) -> Argon2<'static> {
    if fast {
        Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(8, 1, 1, None).expect("params"),
        )
    } else {
        Argon2::default()
    }
}

pub fn hash_password(password: &str, fast: bool) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    argon(fast)
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| anyhow!("password hashing failed: {e}"))
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|h| {
            Argon2::default()
                .verify_password(password.as_bytes(), &h)
                .is_ok()
        })
        .unwrap_or(false)
}

/// Constant-time-ish comparison for the operator key.
pub fn ct_eq(a: &str, b: &str) -> bool {
    let (a, b) = (Sha256::digest(a.as_bytes()), Sha256::digest(b.as_bytes()));
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_roundtrip_and_wrong_key() {
        let enc = encrypt_secret(b"master-1", b"seed-bytes").unwrap();
        assert_eq!(decrypt_secret(b"master-1", &enc).unwrap(), b"seed-bytes");
        assert!(decrypt_secret(b"master-2", &enc).is_err());
    }

    #[test]
    fn password_hash_verifies_regardless_of_cost_params() {
        let h = hash_password("correct horse", true).unwrap();
        assert!(verify_password("correct horse", &h));
        assert!(!verify_password("wrong", &h));
    }

    #[test]
    fn tokens_are_unique_and_prefixed() {
        let (a, b) = (random_token("hsp_"), random_token("hsp_"));
        assert!(a.starts_with("hsp_") && a != b && a.len() > 40);
    }
}
