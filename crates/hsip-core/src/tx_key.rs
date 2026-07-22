//! Per-transaction signing key derivation (HKDF-SHA256).
//!
//! Rather than signing every decision with the same long-lived tenant root
//! key directly, a fresh single-use signing key can be derived per
//! transaction from that root. Two properties this buys together:
//!
//! - **Unlinkable to an outside observer.** A derived key is statistically
//!   indistinguishable from random to anyone without `root_seed` — two
//!   transactions from the same tenant produce completely unrelated-looking
//!   public keys on the wire.
//! - **Re-derivable and verifiable by the root holder.** HKDF is
//!   deterministic: the same `(root_seed, tenant_id, transaction_id)`
//!   always produces the same key. Anyone who holds `root_seed` (the
//!   tenant itself, or an auditor given temporary access to it) can
//!   independently recompute a transaction's signing key and confirm it
//!   really descends from that tenant's root, without HSIP storing any
//!   extra secret to prove it.
//!
//! This does **not** protect against someone with direct access to HSIP's
//! own database — `tenant_id` already links every record there for
//! ordinary multi-tenant operation (auth, rate limiting, queries). The
//! unlinkability property applies to an outside network observer or anyone
//! who only ever sees individually-published signed artifacts.

use ed25519_dalek::SigningKey;
use hkdf::Hkdf;
use sha2::Sha256;

const TX_KEY_INFO_PREFIX: &[u8] = b"hsip-tx-key-v1|";

/// Derive the Ed25519 signing key for one transaction from a tenant's root
/// signing-key seed. `root_seed` is the tenant's own root Ed25519 seed (the
/// same 32 bytes stored, encrypted, as `identities.signing_key_b64`) —
/// never a shared or global secret. `tenant_id` provides domain separation
/// between tenants; `transaction_id` (a decision ID, message ID, etc.)
/// binds the derived key to exactly one transaction so it never repeats.
pub fn derive_transaction_signing_key(
    root_seed: &[u8; 32],
    tenant_id: &str,
    transaction_id: &str,
) -> SigningKey {
    let hk = Hkdf::<Sha256>::new(Some(tenant_id.as_bytes()), root_seed);
    let mut okm = [0u8; 32];
    let mut info = Vec::with_capacity(TX_KEY_INFO_PREFIX.len() + transaction_id.len());
    info.extend_from_slice(TX_KEY_INFO_PREFIX);
    info.extend_from_slice(transaction_id.as_bytes());
    hk.expand(&info, &mut okm)
        .expect("HKDF-SHA256 expand with a 32-byte output cannot fail");
    SigningKey::from_bytes(&okm)
}

/// Recompute a transaction's signing key from `root_seed` and compare its
/// public key against `claimed_verify_key`. This is the audit-side half of
/// the scheme: given the root (never transmitted, never stored by HSIP
/// itself beyond the tenant's own encrypted identity row) and a
/// transaction's recorded `issuer_verify_key`, anyone can confirm that key
/// really was derived from this tenant's root — without needing HSIP's
/// database, and without the derivation ever having revealed the root.
pub fn verify_transaction_key_derivation(
    root_seed: &[u8; 32],
    tenant_id: &str,
    transaction_id: &str,
    claimed_verify_key: &[u8; 32],
) -> bool {
    let derived = derive_transaction_signing_key(root_seed, tenant_id, transaction_id);
    derived.verifying_key().to_bytes() == *claimed_verify_key
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, Verifier};

    #[test]
    fn deterministic_same_inputs_produce_same_key() {
        let root = [7u8; 32];
        let a = derive_transaction_signing_key(&root, "tenant-1", "tx-1");
        let b = derive_transaction_signing_key(&root, "tenant-1", "tx-1");
        assert_eq!(a.to_bytes(), b.to_bytes());
    }

    #[test]
    fn different_transaction_ids_produce_different_keys() {
        let root = [7u8; 32];
        let a = derive_transaction_signing_key(&root, "tenant-1", "tx-1");
        let b = derive_transaction_signing_key(&root, "tenant-1", "tx-2");
        assert_ne!(a.to_bytes(), b.to_bytes());
    }

    #[test]
    fn different_tenants_produce_different_keys_for_the_same_transaction_id() {
        // Domain separation: a derived key must never collide between
        // tenants even if the same transaction_id string is reused, so a
        // leaked key from tenant A can never be confused for tenant B's.
        let root = [7u8; 32];
        let a = derive_transaction_signing_key(&root, "tenant-1", "tx-1");
        let b = derive_transaction_signing_key(&root, "tenant-2", "tx-1");
        assert_ne!(a.to_bytes(), b.to_bytes());
    }

    #[test]
    fn different_roots_produce_unrelated_keys() {
        let root_a = [7u8; 32];
        let root_b = [9u8; 32];
        let a = derive_transaction_signing_key(&root_a, "tenant-1", "tx-1");
        let b = derive_transaction_signing_key(&root_b, "tenant-1", "tx-1");
        assert_ne!(a.to_bytes(), b.to_bytes());
    }

    #[test]
    fn output_is_a_valid_ed25519_signing_key_that_signs_and_verifies() {
        let root = [7u8; 32];
        let key = derive_transaction_signing_key(&root, "tenant-1", "tx-1");
        let msg = b"hello";
        let sig = key.sign(msg);
        assert!(key.verifying_key().verify(msg, &sig).is_ok());
    }

    #[test]
    fn audit_verification_accepts_the_real_derivation_and_rejects_everything_else() {
        let root = [7u8; 32];
        let real_key = derive_transaction_signing_key(&root, "tenant-1", "tx-1");
        let real_vk = real_key.verifying_key().to_bytes();
        assert!(verify_transaction_key_derivation(
            &root, "tenant-1", "tx-1", &real_vk
        ));

        // Wrong transaction id, wrong tenant, wrong root, and a totally
        // unrelated key must all fail.
        assert!(!verify_transaction_key_derivation(
            &root, "tenant-1", "tx-2", &real_vk
        ));
        assert!(!verify_transaction_key_derivation(
            &root, "tenant-2", "tx-1", &real_vk
        ));
        assert!(!verify_transaction_key_derivation(
            &[9u8; 32], "tenant-1", "tx-1", &real_vk
        ));
        let unrelated_vk = derive_transaction_signing_key(&root, "tenant-1", "tx-2")
            .verifying_key()
            .to_bytes();
        assert!(!verify_transaction_key_derivation(
            &root,
            "tenant-1",
            "tx-1",
            &unrelated_vk
        ));
    }
}
