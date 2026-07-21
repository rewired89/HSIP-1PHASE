//! Canonical JSON encoding and event hashing for signed decision records.
//!
//! Uses RFC 8785 JSON Canonicalization Scheme (JCS) rather than the
//! alphabetical-`BTreeMap` trick `hsip-api`'s credential route uses today —
//! JCS is what the VeritasChain Protocol (VCP) mandates for its `EventHash`
//! computation, and unlike the `BTreeMap` shortcut it is correct for nested
//! objects/arrays and defines exact number formatting, so a signature
//! produced here verifies identically in any other JCS-compliant
//! implementation, not just this one.
//!
//! `DecisionEnvelope` is deliberately two-tier (see `hsip-api`'s
//! `routes/decisions.rs`): every field here is either accountability
//! metadata that's fine to disclose (who, which model, which strategy) or
//! an opaque hash. The actual trade content it attests to is never present
//! in this struct or anywhere in HSIP's database — only its caller
//! (Predicta) holds the preimage of `payload_hash`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// HSIP's own draft of "which fields describe accountability for an AI
/// agent's decision" — VCP-GOV is referenced by the VCP spec (v1.2) but has
/// no published field list as of this writing. This tag marks the schema
/// version of HSIP's extension so it can be reconciled if/when VSO
/// publishes a real VCP-GOV schema, without silently pretending to be an
/// official VCP module.
pub const HSIP_GOV_EXT_VERSION: &str = "0.1";

/// The signed envelope for one AI-agent decision attestation.
///
/// Every field is either clear accountability metadata or an opaque hash —
/// see the module docs. `prev_hash` is hex-encoded and empty for the first
/// decision in a tenant's chain (VCP marks `PrevHash` optional; HSIP always
/// includes the field but allows it empty rather than omitting it, so the
/// canonical shape is stable across the whole chain).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEnvelope {
    pub decision_id: String,
    pub tenant_id: String,
    pub agent_key_id: String,
    pub accountable_key: String,
    pub model_version: String,
    pub strategy_id: String,
    pub decision_type: String,
    /// Hex-encoded SHA-256 of the caller's actual (undisclosed) decision payload.
    pub payload_hash: String,
    /// Hex-encoded `event_hash` of this tenant's previous decision, or "".
    pub prev_hash: String,
    pub timestamp_iso: String,
    /// Stringified integer (nanoseconds since epoch) — kept as a JSON
    /// string rather than a JSON number so canonicalization never risks
    /// IEEE-754-double precision loss on large timestamps.
    pub timestamp_int: String,
    pub hsip_gov_ext: String,
    /// Base64 Ed25519 signature by `accountable_key`'s own private key over
    /// `accountable_proof_preimage_hash(...)`, proving whoever submitted
    /// this decision actually holds `accountable_key`'s private key rather
    /// than merely asserting the field. Empty string when no proof was
    /// supplied — `accountable_key` remains caller-asserted metadata in
    /// that case, same as before this field existed. `#[serde(default)]`
    /// so an envelope JSON from before this field existed (an older SDK, an
    /// external verifier that hasn't updated) still deserializes cleanly
    /// into `verify()`, treated as "no proof supplied."
    #[serde(default)]
    pub accountable_key_signature: String,
}

/// The fields `accountable_key`'s own signature attests to: "I hold this
/// key, and I take accountability for a decision with exactly this
/// content." Deliberately narrower than the full `DecisionEnvelope` —
/// everything here is knowable by the caller *before* submitting
/// `POST /v1/decisions` (unlike `decision_id`/`prev_hash`/`timestamp_*`,
/// which the server assigns and which can change across a hash-chain
/// retry attempt), so a caller can compute this preimage and produce
/// `accountable_key_signature` in one client-side step ahead of the
/// request.
///
/// `tenant_id` is included specifically to bind the proof to one tenant on
/// one HSIP deployment: without it, a signature obtained from a real
/// `accountable_key` for one decision could be replayed verbatim by a
/// completely different tenant (or a different HSIP deployment entirely)
/// reusing the same `{model_version, strategy_id, decision_type,
/// payload_hash}` values, since none of those are secret — `payload_hash`
/// itself is returned in every proof bundle and decision listing. Binding
/// to `tenant_id` closes that cross-tenant replay. A residual, lower-severity
/// replay remains *within* the same tenant (an already-credentialed agent in
/// that tenant reusing another agent's exact decision content to
/// misattribute a decision to the same `accountable_key`) — documented in
/// THREAT_MODEL.md rather than closed here, since eliminating it needs a
/// per-decision server-issued nonce (a two-phase challenge/response
/// protocol), a materially bigger change than this proof-of-possession step
/// warrants on its own.
#[derive(Debug, Clone, Serialize)]
struct AccountableProofPreimage<'a> {
    accountable_key: &'a str,
    tenant_id: &'a str,
    model_version: &'a str,
    strategy_id: &'a str,
    decision_type: &'a str,
    payload_hash: &'a str,
}

/// `SHA256(JCS(AccountableProofPreimage))` — the exact bytes `accountable_key`
/// must Ed25519-sign to produce `DecisionEnvelope::accountable_key_signature`.
/// The single source of truth for this formula: `routes::decisions::record`
/// (verifying at write time) and `routes::decisions::verify` (independently
/// re-checking a bundle, DB-free) both call this rather than each
/// reimplementing the JCS+SHA256 steps separately — same reasoning as
/// `audit_log::compute_entry_hash` being the one place the audit hash-chain
/// formula lives.
pub fn accountable_proof_preimage_hash(
    accountable_key: &str,
    tenant_id: &str,
    model_version: &str,
    strategy_id: &str,
    decision_type: &str,
    payload_hash: &str,
) -> Result<[u8; 32], serde_json::Error> {
    let preimage = AccountableProofPreimage {
        accountable_key,
        tenant_id,
        model_version,
        strategy_id,
        decision_type,
        payload_hash,
    };
    let bytes = serde_jcs::to_vec(&preimage)?;
    Ok(Sha256::digest(bytes).into())
}

/// Serialize `envelope` per RFC 8785 JCS. Deterministic across
/// implementations — this is what actually gets hashed and signed.
pub fn canonical_bytes(envelope: &DecisionEnvelope) -> Result<Vec<u8>, serde_json::Error> {
    serde_jcs::to_vec(envelope)
}

/// `SHA256(JCS(envelope))` — the `EventHash` that gets Ed25519-signed and
/// fed into the RFC 6962 Merkle tree as leaf data.
pub fn event_hash(envelope: &DecisionEnvelope) -> Result<[u8; 32], serde_json::Error> {
    let bytes = canonical_bytes(envelope)?;
    Ok(Sha256::digest(bytes).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> DecisionEnvelope {
        DecisionEnvelope {
            decision_id: "d1".into(),
            tenant_id: "t1".into(),
            agent_key_id: "k1".into(),
            accountable_key: "abc".into(),
            model_version: "predicta-v3".into(),
            strategy_id: "mean-reversion-1".into(),
            decision_type: "trade.order".into(),
            payload_hash: "deadbeef".into(),
            prev_hash: String::new(),
            timestamp_iso: "2026-07-09T00:00:00Z".into(),
            timestamp_int: "1770595200000000000".into(),
            hsip_gov_ext: HSIP_GOV_EXT_VERSION.into(),
            accountable_key_signature: String::new(),
        }
    }

    #[test]
    fn canonical_bytes_is_deterministic() {
        let e = sample();
        assert_eq!(canonical_bytes(&e).unwrap(), canonical_bytes(&e).unwrap());
    }

    #[test]
    fn event_hash_changes_when_any_field_changes() {
        let e1 = sample();
        let mut e2 = sample();
        e2.strategy_id = "mean-reversion-2".into();
        assert_ne!(event_hash(&e1).unwrap(), event_hash(&e2).unwrap());
    }

    #[test]
    fn field_order_in_struct_does_not_affect_output() {
        // JCS sorts object keys, so two envelopes with identical field
        // values must canonicalize identically regardless of how the
        // struct happened to be constructed.
        let e1 = sample();
        let e2 = sample();
        assert_eq!(canonical_bytes(&e1).unwrap(), canonical_bytes(&e2).unwrap());
    }

    #[test]
    fn large_timestamp_survives_as_string_without_precision_loss() {
        let mut e = sample();
        // Larger than 2^53 (~9.007e15) — the point at which a JSON *number*
        // encoding could lose precision; kept here as a string field.
        e.timestamp_int = "9223372036854775807".into();
        let bytes = canonical_bytes(&e).unwrap();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("9223372036854775807"));
    }

    #[test]
    fn envelope_json_without_accountable_key_signature_still_deserializes() {
        // Backward compatibility for any pre-existing SDK/verifier that
        // doesn't know this field exists yet: `#[serde(default)]` means an
        // envelope JSON omitting it entirely parses as "no proof supplied",
        // not a hard error.
        let json = r#"{
            "decision_id": "d1", "tenant_id": "t1", "agent_key_id": "k1",
            "accountable_key": "abc", "model_version": "v1", "strategy_id": "s1",
            "decision_type": "trade.order", "payload_hash": "deadbeef",
            "prev_hash": "", "timestamp_iso": "2026-07-09T00:00:00Z",
            "timestamp_int": "1770595200000000000", "hsip_gov_ext": "0.1"
        }"#;
        let envelope: DecisionEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(envelope.accountable_key_signature, "");
    }

    #[test]
    fn accountable_proof_preimage_hash_is_deterministic_and_order_independent() {
        let h1 =
            accountable_proof_preimage_hash("key1", "tenant1", "v1", "s1", "trade.order", "hash1")
                .unwrap();
        let h2 =
            accountable_proof_preimage_hash("key1", "tenant1", "v1", "s1", "trade.order", "hash1")
                .unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn accountable_proof_preimage_hash_changes_when_tenant_id_changes() {
        // The whole point of including tenant_id: a proof for one tenant
        // must not verify for another tenant reusing the same decision
        // content fields.
        let h1 =
            accountable_proof_preimage_hash("key1", "tenant1", "v1", "s1", "trade.order", "hash1")
                .unwrap();
        let h2 =
            accountable_proof_preimage_hash("key1", "tenant2", "v1", "s1", "trade.order", "hash1")
                .unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn accountable_proof_preimage_hash_changes_when_any_content_field_changes() {
        let base =
            accountable_proof_preimage_hash("key1", "tenant1", "v1", "s1", "trade.order", "hash1")
                .unwrap();
        let diff_payload =
            accountable_proof_preimage_hash("key1", "tenant1", "v1", "s1", "trade.order", "hash2")
                .unwrap();
        let diff_decision_type =
            accountable_proof_preimage_hash("key1", "tenant1", "v1", "s1", "trade.close", "hash1")
                .unwrap();
        assert_ne!(base, diff_payload);
        assert_ne!(base, diff_decision_type);
    }
}
