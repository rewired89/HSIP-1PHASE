//! HSIP: the open proof format and its independent verifier.
//!
//! Everything here is a pure function of its inputs. A customer, auditor or
//! regulator can run it with no account and no network, which is the whole
//! point: a proof must not depend on trusting the vendor's database.
//!
//! Layers, weakest to strongest:
//! 1. **Authorship**: Ed25519 signature by the organization's key over the
//!    envelope hash (RFC 8785 JCS canonical JSON, SHA-256).
//! 2. **Batch inclusion**: RFC 6962 Merkle inclusion of the envelope hash in
//!    an anchor batch, with the batch root signed by the anchor key.
//! 3. **External time** (optional): OpenTimestamps commitment of that root.
//!    Carried in the bundle, not checked here.

pub mod mandate;
pub mod merkle;

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub use mandate::{
    evaluate as evaluate_mandate, proofs_from_value, sign_mandate, verify_mandate, Mandate,
    MandateBody, MandateReport, Violation, MANDATE_FORMAT,
};
pub use merkle::{MerkleTree, ProofStep, Side};

pub const ENVELOPE_FORMAT: &str = "hsip-proof/1";
pub const BUNDLE_FORMAT: &str = "hsip-proof-bundle/1";
pub const PACK_FORMAT: &str = "hsip-evidence-pack/1";

/// The signed statement. `payload_hash` is the only link to the real
/// content, which never leaves the customer's systems.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Envelope {
    pub format: String,
    pub proof_id: String,
    pub org_id: String,
    pub stream_id: String,
    /// Position in the stream's hash chain, as a string (JSON numbers lose
    /// precision above 2^53 in many verifiers).
    pub seq: String,
    pub actor: String,
    pub action: String,
    #[serde(default)]
    pub subject: String,
    /// Hex SHA-256 of the customer's undisclosed record.
    pub payload_hash: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// `event_hash` of the previous proof in this stream, empty at genesis.
    pub prev_hash: String,
    pub timestamp_iso: String,
    pub timestamp_ms: String,
}

pub fn canonical_bytes(env: &Envelope) -> Result<Vec<u8>, serde_json::Error> {
    serde_jcs::to_vec(env)
}

pub fn event_hash(env: &Envelope) -> Result<[u8; 32], serde_json::Error> {
    Ok(Sha256::digest(canonical_bytes(env)?).into())
}

#[must_use]
pub fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

/// Signs an envelope. Returns `(event_hash_hex, signature_b64)`.
pub fn sign_envelope(
    env: &Envelope,
    key: &SigningKey,
) -> Result<(String, String), serde_json::Error> {
    let h = event_hash(env)?;
    Ok((hex::encode(h), B64.encode(key.sign(&h).to_bytes())))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofStepDto {
    pub hash: String,
    pub position: String,
}

impl From<&ProofStep> for ProofStepDto {
    fn from(s: &ProofStep) -> Self {
        Self {
            hash: hex::encode(s.hash),
            position: match s.side {
                Side::Left => "left".into(),
                Side::Right => "right".into(),
            },
        }
    }
}

impl TryFrom<&ProofStepDto> for ProofStep {
    type Error = ();
    fn try_from(d: &ProofStepDto) -> Result<Self, ()> {
        let hash: [u8; 32] = hex::decode(&d.hash)
            .map_err(|_| ())?
            .try_into()
            .map_err(|_| ())?;
        let side = match d.position.as_str() {
            "left" => Side::Left,
            "right" => Side::Right,
            _ => return Err(()),
        };
        Ok(Self { hash, side })
    }
}

/// Batch anchoring evidence for one proof.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnchorBlock {
    pub anchor_id: String,
    pub merkle_root: String,
    pub merkle_index: i64,
    pub leaf_count: i64,
    pub inclusion_proof: Vec<ProofStepDto>,
    /// Ed25519 over the 32 raw root bytes.
    pub anchor_signature: String,
    pub anchor_verify_key: String,
    pub anchored_at_iso: String,
    /// `pending` | `confirmed` | `calendar_unreachable` | `disabled`.
    #[serde(default)]
    pub ots_status: String,
    /// Base64 OpenTimestamps commitment, if any.
    #[serde(default)]
    pub ots_proof: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofBundle {
    pub format: String,
    pub envelope: Envelope,
    pub event_hash: String,
    pub signature: String,
    pub sign_algo: String,
    /// The organization's published verify key (base64).
    pub org_verify_key: String,
    #[serde(default)]
    pub org_name: String,
    #[serde(default)]
    pub anchor: Option<AnchorBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Check {
    pub name: String,
    /// `None` = not applicable (e.g. not anchored yet), never a failure.
    pub ok: Option<bool>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Verification {
    /// Every applicable check passed.
    pub valid: bool,
    pub anchored: bool,
    pub checks: Vec<Check>,
}

impl Verification {
    fn finish(checks: Vec<Check>, anchored: bool) -> Self {
        let valid = checks.iter().all(|c| c.ok != Some(false));
        Self {
            valid,
            anchored,
            checks,
        }
    }
}

fn check(name: &str, ok: Option<bool>, detail: impl Into<String>) -> Check {
    Check {
        name: name.into(),
        ok,
        detail: detail.into(),
    }
}

fn vk_from_b64(s: &str) -> Option<VerifyingKey> {
    let b: [u8; 32] = B64.decode(s).ok()?.try_into().ok()?;
    VerifyingKey::from_bytes(&b).ok()
}

fn sig_from_b64(s: &str) -> Option<Signature> {
    let b: [u8; 64] = B64.decode(s).ok()?.try_into().ok()?;
    Some(Signature::from_bytes(&b))
}

/// Independent verification of one proof bundle. No I/O.
#[must_use]
pub fn verify_bundle(b: &ProofBundle) -> Verification {
    let mut checks = Vec::new();

    let recomputed = event_hash(&b.envelope).map(hex::encode).unwrap_or_default();
    let hash_ok = !recomputed.is_empty() && recomputed.eq_ignore_ascii_case(&b.event_hash);
    checks.push(check(
        "content_hash",
        Some(hash_ok),
        if hash_ok {
            "Record hash recomputed from the envelope matches."
        } else {
            "Envelope was altered: recomputed hash differs from the recorded one."
        },
    ));

    let sig_ok = (|| {
        let vk = vk_from_b64(&b.org_verify_key)?;
        let sig = sig_from_b64(&b.signature)?;
        let h: [u8; 32] = hex::decode(&b.event_hash).ok()?.try_into().ok()?;
        Some(vk.verify(&h, &sig).is_ok())
    })()
    .unwrap_or(false);
    checks.push(check(
        "signature",
        Some(sig_ok),
        if sig_ok {
            "Signature verifies against the organization's public key."
        } else {
            "Signature does not verify against the organization's public key."
        },
    ));

    let format_ok = b.envelope.format == ENVELOPE_FORMAT && b.format == BUNDLE_FORMAT;
    checks.push(check(
        "format",
        Some(format_ok),
        if format_ok {
            "Known proof format."
        } else {
            "Unknown or unsupported format version."
        },
    ));

    let mut anchored = false;
    match &b.anchor {
        None => {
            checks.push(check(
                "batch_inclusion",
                None,
                "Not anchored yet; authorship is proven, batch inclusion is pending.",
            ));
            checks.push(check("anchor_signature", None, "Not anchored yet."));
        }
        Some(a) => {
            anchored = true;
            let incl = (|| {
                let root: [u8; 32] = hex::decode(&a.merkle_root).ok()?.try_into().ok()?;
                let leaf: [u8; 32] = hex::decode(&b.event_hash).ok()?.try_into().ok()?;
                let steps: Vec<ProofStep> = a
                    .inclusion_proof
                    .iter()
                    .map(ProofStep::try_from)
                    .collect::<Result<_, _>>()
                    .ok()?;
                Some(merkle::verify_inclusion(&leaf, &steps, &root))
            })()
            .unwrap_or(false);
            checks.push(check(
                "batch_inclusion",
                Some(incl),
                if incl {
                    "Record is included in the anchored Merkle batch."
                } else {
                    "Record is NOT included in the claimed Merkle batch."
                },
            ));
            let asig = (|| {
                let vk = vk_from_b64(&a.anchor_verify_key)?;
                let sig = sig_from_b64(&a.anchor_signature)?;
                let root: [u8; 32] = hex::decode(&a.merkle_root).ok()?.try_into().ok()?;
                Some(vk.verify(&root, &sig).is_ok())
            })()
            .unwrap_or(false);
            checks.push(check(
                "anchor_signature",
                Some(asig),
                if asig {
                    "Batch root is signed by the anchor key."
                } else {
                    "Batch root signature is invalid."
                },
            ));
        }
    }
    Verification::finish(checks, anchored)
}

/// A bundle that proves a record against a *specific* expected payload.
/// Returns true when `sha256(content)` equals the envelope's `payload_hash`.
#[must_use]
pub fn payload_matches(b: &ProofBundle, content: &[u8]) -> bool {
    sha256_hex(content).eq_ignore_ascii_case(&b.envelope.payload_hash)
}

// ---------------------------------------------------------------- evidence packs

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackManifest {
    pub org_id: String,
    pub stream_id: String,
    pub from_ms: i64,
    pub to_ms: i64,
    /// Event hashes in sequence order. Hashing this list is what the org
    /// signs, so an omitted or inserted record changes the signature.
    pub event_hashes: Vec<String>,
    /// Verify keys that signed the included proofs, when they are not all
    /// the pack's `org_verify_key` (customer-held keys, or a stream that
    /// switched custody). Empty for packs where one key signed everything,
    /// and then omitted from the JSON so those packs stay byte-compatible.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub issuer_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidencePack {
    pub format: String,
    pub org_name: String,
    pub org_verify_key: String,
    pub stream_name: String,
    pub generated_at_iso: String,
    pub manifest: PackManifest,
    pub manifest_hash: String,
    pub manifest_signature: String,
    pub proofs: Vec<ProofBundle>,
}

pub fn manifest_hash(m: &PackManifest) -> Result<[u8; 32], serde_json::Error> {
    Ok(Sha256::digest(serde_jcs::to_vec(m)?).into())
}

pub fn sign_manifest(
    m: &PackManifest,
    key: &SigningKey,
) -> Result<(String, String), serde_json::Error> {
    let h = manifest_hash(m)?;
    Ok((hex::encode(h), B64.encode(key.sign(&h).to_bytes())))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackVerification {
    pub valid: bool,
    pub proof_count: usize,
    pub invalid_proofs: Vec<String>,
    pub anchored_count: usize,
    pub checks: Vec<Check>,
}

/// Verifies a whole pack: manifest signature, manifest/proof agreement,
/// per-proof validity, and the stream's hash chain across the exported range.
#[must_use]
pub fn verify_pack(p: &EvidencePack) -> PackVerification {
    let mut checks = Vec::new();

    let mh = manifest_hash(&p.manifest)
        .map(hex::encode)
        .unwrap_or_default();
    let mh_ok = !mh.is_empty() && mh == p.manifest_hash;
    checks.push(check(
        "manifest_hash",
        Some(mh_ok),
        if mh_ok {
            "Manifest hash matches its contents."
        } else {
            "Manifest was altered."
        },
    ));

    let msig = (|| {
        let vk = vk_from_b64(&p.org_verify_key)?;
        let sig = sig_from_b64(&p.manifest_signature)?;
        let h: [u8; 32] = hex::decode(&p.manifest_hash).ok()?.try_into().ok()?;
        Some(vk.verify(&h, &sig).is_ok())
    })()
    .unwrap_or(false);
    checks.push(check(
        "manifest_signature",
        Some(msig),
        if msig {
            "Manifest is signed by the organization."
        } else {
            "Manifest signature is invalid."
        },
    ));

    let listed: Vec<&str> = p.manifest.event_hashes.iter().map(String::as_str).collect();
    let actual: Vec<&str> = p.proofs.iter().map(|b| b.event_hash.as_str()).collect();
    let agree = listed == actual;
    checks.push(check(
        "manifest_matches_proofs",
        Some(agree),
        if agree {
            "Signed manifest lists exactly the proofs included."
        } else {
            "Included proofs differ from the signed manifest (record removed, added or reordered)."
        },
    ));

    let mut invalid = Vec::new();
    let mut anchored_count = 0;
    let mut issuers_ok = true;
    for b in &p.proofs {
        let v = verify_bundle(b);
        if !v.valid {
            invalid.push(b.envelope.proof_id.clone());
        }
        if v.anchored {
            anchored_count += 1;
        }
        let allowed = if p.manifest.issuer_keys.is_empty() {
            b.org_verify_key == p.org_verify_key
        } else {
            p.manifest.issuer_keys.contains(&b.org_verify_key)
        };
        if !allowed {
            issuers_ok = false;
        }
    }
    checks.push(check(
        "proofs_valid",
        Some(invalid.is_empty()),
        if invalid.is_empty() {
            "Every included proof verifies.".to_string()
        } else {
            format!("{} proof(s) failed verification.", invalid.len())
        },
    ));
    checks.push(check(
        "single_issuer",
        Some(issuers_ok),
        if issuers_ok {
            if p.manifest.issuer_keys.is_empty() {
                "All proofs share the pack's organization key.".to_string()
            } else {
                format!(
                    "Every proof is signed by one of the {} declared issuer keys.",
                    p.manifest.issuer_keys.len()
                )
            }
        } else {
            "A proof was signed under a key the signed manifest does not declare.".to_string()
        },
    ));

    // Chain: consecutive seq must link prev_hash -> previous event_hash.
    let mut chain_ok = true;
    let mut gap = false;
    for w in p.proofs.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        let (sa, sb): (i64, i64) = (
            a.envelope.seq.parse().unwrap_or(-1),
            b.envelope.seq.parse().unwrap_or(-2),
        );
        if sb != sa + 1 {
            gap = true;
        }
        if b.envelope.prev_hash != a.event_hash {
            chain_ok = false;
        }
    }
    checks.push(check(
        "hash_chain",
        Some(chain_ok && !gap),
        if chain_ok && !gap {
            "Records are contiguous and each links to the one before it."
        } else if gap {
            "A gap in sequence numbers: one or more records are missing."
        } else {
            "A record does not link to its predecessor: chain is broken."
        },
    ));

    let valid = checks.iter().all(|c| c.ok != Some(false));
    PackVerification {
        valid,
        proof_count: p.proofs.len(),
        invalid_proofs: invalid,
        anchored_count,
        checks,
    }
}

// ------------------------------------------------------------ customer-held keys
//
// A customer generates their own Ed25519 key, registers the public half with
// the platform, and signs every record themselves. The platform prepares the
// envelope (sequence, previous hash, time), the customer checks it matches
// what they asked for, signs `event_hash`, and the platform verifies, chains
// and anchors it. The platform never sees the private key.

pub const CUSTODY_DOMAIN: &str = "hsip-custody-v1";

/// A fresh Ed25519 signing key from the operating system's randomness.
#[cfg(feature = "keygen")]
#[must_use]
pub fn generate_signing_key() -> SigningKey {
    SigningKey::generate(&mut rand::rngs::OsRng)
}

/// Rebuilds a signing key from its base64 32-byte seed.
#[must_use]
pub fn signing_key_from_b64(seed_b64: &str) -> Option<SigningKey> {
    let seed: [u8; 32] = B64.decode(seed_b64.trim()).ok()?.try_into().ok()?;
    Some(SigningKey::from_bytes(&seed))
}

#[must_use]
pub fn seed_b64(key: &SigningKey) -> String {
    B64.encode(key.to_bytes())
}

#[must_use]
pub fn verify_key_b64(key: &SigningKey) -> String {
    B64.encode(key.verifying_key().to_bytes())
}

/// True when `vk_b64` is a well-formed Ed25519 public key.
#[must_use]
pub fn is_valid_verify_key(vk_b64: &str) -> bool {
    vk_from_b64(vk_b64).is_some()
}

/// Verifies an Ed25519 signature (base64) over a 32-byte hash (hex) with a
/// public key (base64).
#[must_use]
pub fn verify_hash_signature(vk_b64: &str, hash_hex: &str, sig_b64: &str) -> bool {
    (|| {
        let vk = vk_from_b64(vk_b64)?;
        let sig = sig_from_b64(sig_b64)?;
        let h: [u8; 32] = hex::decode(hash_hex).ok()?.try_into().ok()?;
        Some(vk.verify(&h, &sig).is_ok())
    })()
    .unwrap_or(false)
}

/// What a customer signs to register a key: binds the key to one
/// organization, so a signature for one org cannot register the key in
/// another, and proves the registrant holds the private key.
#[must_use]
pub fn registration_digest(org_id: &str, verify_key_b64: &str) -> [u8; 32] {
    Sha256::digest(format!("{CUSTODY_DOMAIN}|{org_id}|{verify_key_b64}").as_bytes()).into()
}

#[must_use]
pub fn sign_registration(org_id: &str, key: &SigningKey) -> String {
    let d = registration_digest(org_id, &verify_key_b64(key));
    B64.encode(key.sign(&d).to_bytes())
}

#[must_use]
pub fn verify_registration(org_id: &str, verify_key_b64: &str, sig_b64: &str) -> bool {
    verify_hash_signature(
        verify_key_b64,
        &hex::encode(registration_digest(org_id, verify_key_b64)),
        sig_b64,
    )
}

/// The client-side safety check before signing a prepared envelope. A
/// customer-held key must never sign a hash the platform hands over blindly:
/// recompute it from the envelope and confirm the envelope says what the
/// customer asked to record.
pub fn check_prepared(
    env: &Envelope,
    event_hash_hex: &str,
    actor: &str,
    action: &str,
    subject: &str,
    payload_hash: &str,
    metadata: &BTreeMap<String, String>,
) -> Result<(), String> {
    let recomputed = event_hash(env)
        .map(hex::encode)
        .map_err(|e| e.to_string())?;
    if !recomputed.eq_ignore_ascii_case(event_hash_hex) {
        return Err("event_hash does not match the envelope".into());
    }
    if env.format != ENVELOPE_FORMAT {
        return Err("unexpected envelope format".into());
    }
    if env.actor != actor || env.action != action || env.subject != subject {
        return Err("envelope actor, action or subject differs from the request".into());
    }
    if !env.payload_hash.eq_ignore_ascii_case(payload_hash) {
        return Err("envelope payload_hash differs from the request".into());
    }
    if &env.metadata != metadata {
        return Err("envelope metadata differs from the request".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn key(n: u8) -> SigningKey {
        SigningKey::from_bytes(&[n; 32])
    }

    fn env(seq: u64, prev: &str) -> Envelope {
        Envelope {
            format: ENVELOPE_FORMAT.into(),
            proof_id: format!("p{seq}"),
            org_id: "org1".into(),
            stream_id: "s1".into(),
            seq: seq.to_string(),
            actor: "loan-model-v3".into(),
            action: "loan.decision".into(),
            subject: "application-77".into(),
            payload_hash: sha256_hex(format!("record {seq}").as_bytes()),
            metadata: BTreeMap::from([("region".to_string(), "eu".to_string())]),
            prev_hash: prev.into(),
            timestamp_iso: "2026-10-17T00:00:00Z".into(),
            timestamp_ms: "1792195200000".into(),
        }
    }

    fn bundle(e: Envelope, k: &SigningKey) -> ProofBundle {
        let (h, s) = sign_envelope(&e, k).unwrap();
        ProofBundle {
            format: BUNDLE_FORMAT.into(),
            envelope: e,
            event_hash: h,
            signature: s,
            sign_algo: "ED25519".into(),
            org_verify_key: B64.encode(k.verifying_key().to_bytes()),
            org_name: "Acme".into(),
            anchor: None,
        }
    }

    fn anchor_all(bs: &mut [ProofBundle], ak: &SigningKey) {
        let leaves: Vec<[u8; 32]> = bs
            .iter()
            .map(|b| hex::decode(&b.event_hash).unwrap().try_into().unwrap())
            .collect();
        let tree = MerkleTree::from_leaves(&leaves);
        let root = tree.root();
        let sig = B64.encode(ak.sign(&root).to_bytes());
        for (i, b) in bs.iter_mut().enumerate() {
            b.anchor = Some(AnchorBlock {
                anchor_id: "a1".into(),
                merkle_root: hex::encode(root),
                merkle_index: i as i64,
                leaf_count: leaves.len() as i64,
                inclusion_proof: tree
                    .inclusion_proof(i)
                    .iter()
                    .map(ProofStepDto::from)
                    .collect(),
                anchor_signature: sig.clone(),
                anchor_verify_key: B64.encode(ak.verifying_key().to_bytes()),
                anchored_at_iso: "2026-10-17T00:05:00Z".into(),
                ots_status: "disabled".into(),
                ots_proof: None,
            });
        }
    }

    #[test]
    fn unanchored_bundle_is_valid_but_not_anchored() {
        let v = verify_bundle(&bundle(env(1, ""), &key(1)));
        assert!(v.valid && !v.anchored);
    }

    #[test]
    fn altered_envelope_fails_hash_and_signature_still_checks_old_hash() {
        let mut b = bundle(env(1, ""), &key(1));
        b.envelope.action = "loan.approve".into();
        let v = verify_bundle(&b);
        assert!(!v.valid);
        assert_eq!(v.checks[0].ok, Some(false));
    }

    #[test]
    fn rehashed_forgery_fails_signature() {
        let mut b = bundle(env(1, ""), &key(1));
        b.envelope.actor = "someone-else".into();
        b.event_hash = hex::encode(event_hash(&b.envelope).unwrap());
        let v = verify_bundle(&b);
        assert!(!v.valid);
        assert_eq!(v.checks[0].ok, Some(true));
        assert_eq!(v.checks[1].ok, Some(false));
    }

    #[test]
    fn wrong_org_key_is_rejected() {
        let mut b = bundle(env(1, ""), &key(1));
        b.org_verify_key = B64.encode(key(2).verifying_key().to_bytes());
        assert!(!verify_bundle(&b).valid);
    }

    #[test]
    fn anchored_bundles_verify_and_tampering_is_caught() {
        let k = key(1);
        let e1 = env(1, "");
        let b1 = bundle(e1, &k);
        let e2 = env(2, &b1.event_hash);
        let b2 = bundle(e2, &k);
        let mut bs = vec![b1, b2];
        anchor_all(&mut bs, &key(9));
        assert!(bs
            .iter()
            .all(|b| verify_bundle(b).valid && verify_bundle(b).anchored));
        let mut t = bs[1].clone();
        t.anchor.as_mut().unwrap().merkle_index = 0;
        t.anchor.as_mut().unwrap().inclusion_proof[0].position = "right".into();
        assert!(!verify_bundle(&t).valid);
        let mut t2 = bs[0].clone();
        t2.anchor.as_mut().unwrap().anchor_verify_key =
            B64.encode(key(8).verifying_key().to_bytes());
        assert!(!verify_bundle(&t2).valid);
    }

    #[test]
    fn payload_matching() {
        let b = bundle(env(1, ""), &key(1));
        assert!(payload_matches(&b, b"record 1"));
        assert!(!payload_matches(&b, b"record 2"));
    }

    fn pack(k: &SigningKey) -> EvidencePack {
        let b1 = bundle(env(1, ""), k);
        let b2 = bundle(env(2, &b1.event_hash), k);
        let b3 = bundle(env(3, &b2.event_hash), k);
        let proofs = vec![b1, b2, b3];
        let manifest = PackManifest {
            org_id: "org1".into(),
            stream_id: "s1".into(),
            from_ms: 0,
            to_ms: i64::MAX,
            event_hashes: proofs.iter().map(|b| b.event_hash.clone()).collect(),
            issuer_keys: vec![],
        };
        let (mh, ms) = sign_manifest(&manifest, k).unwrap();
        EvidencePack {
            format: PACK_FORMAT.into(),
            org_name: "Acme".into(),
            org_verify_key: B64.encode(k.verifying_key().to_bytes()),
            stream_name: "Loans".into(),
            generated_at_iso: "2026-10-17T01:00:00Z".into(),
            manifest,
            manifest_hash: mh,
            manifest_signature: ms,
            proofs,
        }
    }

    #[test]
    fn pack_verifies() {
        let v = verify_pack(&pack(&key(1)));
        assert!(v.valid, "{:?}", v.checks);
        assert_eq!(v.proof_count, 3);
    }

    #[test]
    fn pack_with_removed_record_is_detected() {
        let mut p = pack(&key(1));
        p.proofs.remove(1);
        let v = verify_pack(&p);
        assert!(!v.valid);
        assert!(v
            .checks
            .iter()
            .any(|c| c.name == "manifest_matches_proofs" && c.ok == Some(false)));
        assert!(v
            .checks
            .iter()
            .any(|c| c.name == "hash_chain" && c.ok == Some(false)));
    }

    #[test]
    fn pack_with_removed_record_and_rewritten_manifest_still_fails_chain_and_sig() {
        let k = key(1);
        let mut p = pack(&k);
        p.proofs.remove(1);
        p.manifest.event_hashes.remove(1);
        p.manifest_hash = hex::encode(manifest_hash(&p.manifest).unwrap());
        let v = verify_pack(&p);
        assert!(!v.valid);
        assert!(v
            .checks
            .iter()
            .any(|c| c.name == "manifest_signature" && c.ok == Some(false)));
        assert!(v
            .checks
            .iter()
            .any(|c| c.name == "hash_chain" && c.ok == Some(false)));
    }

    #[test]
    fn customer_key_registration_binds_key_to_org() {
        let k = generate_signing_key();
        let sig = sign_registration("org-1", &k);
        let vk = verify_key_b64(&k);
        assert!(verify_registration("org-1", &vk, &sig));
        assert!(
            !verify_registration("org-2", &vk, &sig),
            "signature must not transfer to another org"
        );
        let other = verify_key_b64(&generate_signing_key());
        assert!(
            !verify_registration("org-1", &other, &sig),
            "signature must not register someone else's key"
        );
        assert!(is_valid_verify_key(&vk) && !is_valid_verify_key("not-a-key"));
        assert_eq!(
            signing_key_from_b64(&seed_b64(&k)).unwrap().to_bytes(),
            k.to_bytes()
        );
    }

    #[test]
    fn check_prepared_refuses_to_sign_a_doctored_envelope() {
        let k = key(1);
        let e = env(1, "");
        let (h, _) = sign_envelope(&e, &k).unwrap();
        let md = e.metadata.clone();
        assert!(check_prepared(
            &e,
            &h,
            &e.actor,
            &e.action,
            &e.subject,
            &e.payload_hash,
            &md
        )
        .is_ok());
        // the platform swaps the action but keeps a hash that matches the swapped envelope
        let mut evil = e.clone();
        evil.action = "loan.approve".into();
        let evil_hash = hex::encode(event_hash(&evil).unwrap());
        assert!(check_prepared(
            &evil,
            &evil_hash,
            &e.actor,
            &e.action,
            &e.subject,
            &e.payload_hash,
            &md
        )
        .is_err());
        // the platform sends a hash that does not match the envelope
        assert!(check_prepared(
            &e,
            &evil_hash,
            &e.actor,
            &e.action,
            &e.subject,
            &e.payload_hash,
            &md
        )
        .is_err());
        // payload swapped
        let mut p = e.clone();
        p.payload_hash = sha256_hex(b"other");
        let ph = hex::encode(event_hash(&p).unwrap());
        assert!(check_prepared(
            &p,
            &ph,
            &e.actor,
            &e.action,
            &e.subject,
            &e.payload_hash,
            &md
        )
        .is_err());
    }

    #[test]
    fn pack_with_mixed_issuers_needs_them_declared() {
        let (platform, customer) = (key(1), key(5));
        let b1 = bundle(env(1, ""), &platform);
        let b2 = bundle(env(2, &b1.event_hash), &customer);
        let proofs = vec![b1, b2];
        let pk = B64.encode(platform.verifying_key().to_bytes());
        let ck = B64.encode(customer.verifying_key().to_bytes());
        let manifest = |issuers: Vec<String>| PackManifest {
            org_id: "org1".into(),
            stream_id: "s1".into(),
            from_ms: 0,
            to_ms: i64::MAX,
            event_hashes: proofs.iter().map(|b| b.event_hash.clone()).collect(),
            issuer_keys: issuers,
        };
        let build = |m: PackManifest| {
            let (mh, ms) = sign_manifest(&m, &platform).unwrap();
            EvidencePack {
                format: PACK_FORMAT.into(),
                org_name: "Acme".into(),
                org_verify_key: pk.clone(),
                stream_name: "Loans".into(),
                generated_at_iso: "2026-10-18T00:00:00Z".into(),
                manifest: m,
                manifest_hash: mh,
                manifest_signature: ms,
                proofs: proofs.clone(),
            }
        };
        assert!(verify_pack(&build(manifest(vec![pk.clone(), ck.clone()]))).valid);
        // undeclared issuer: the customer's records are not covered by the manifest's list
        assert!(!verify_pack(&build(manifest(vec![pk.clone()]))).valid);
        assert!(!verify_pack(&build(manifest(vec![]))).valid);
    }
}
