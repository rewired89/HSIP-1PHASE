//! Signed checkpoints: the head of a stream at a moment, signed by the platform.
//!
//! A hash chain shows that records link to each other. It does not stop whoever
//! runs the database from truncating the newest records, rolling back to an
//! earlier state, or showing two people two different histories. A checkpoint
//! makes the platform commit to "this stream was at position N with this head"
//! at a time. A customer who keeps the checkpoints themselves can later catch
//! any history that disagrees with one. Everything here is pure: no I/O.
//!
//! What it detects, given checkpoints the customer kept:
//! - a presented history whose record at a checkpointed position differs;
//! - an export that claims to reach past a checkpoint but ends before it;
//! - two signed checkpoints for the same position with different heads;
//! - a later checkpoint with a lower position than an earlier one (rollback).
//!
//! What it does not do: it cannot detect a rewrite of positions the customer
//! holds no checkpoint for, and a platform that never signs a checkpoint leaves
//! nothing to compare. Checkpoints are only as good as the copies customers keep.

use base64::Engine as _;
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{verify_hash_signature, verify_key_b64, EvidencePack};

pub const CHECKPOINT_FORMAT: &str = "hsip-checkpoint/1";

/// The signed part. `seq` and the times are strings, as in the proof envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointBody {
    pub format: String,
    pub org_id: String,
    pub stream_id: String,
    /// Position of the head record in the stream.
    pub seq: String,
    /// `event_hash` of the head record.
    pub head_hash: String,
    pub timestamp_ms: String,
    pub timestamp_iso: String,
    /// `checkpoint_hash` of the stream's previous checkpoint, empty for the first.
    #[serde(default)]
    pub prev_checkpoint_hash: String,
    /// Public key (base64) that signed this checkpoint.
    pub signer_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Checkpoint {
    #[serde(flatten)]
    pub body: CheckpointBody,
    /// SHA-256 over the RFC 8785 canonical form of the body, hex.
    pub checkpoint_hash: String,
    /// Ed25519 over the 32 raw bytes of `checkpoint_hash`, base64.
    pub signature: String,
}

pub fn checkpoint_hash(body: &CheckpointBody) -> Result<[u8; 32], serde_json::Error> {
    Ok(Sha256::digest(serde_jcs::to_vec(body)?).into())
}

pub fn sign_checkpoint(body: CheckpointBody, key: &SigningKey) -> Result<Checkpoint, String> {
    if body.format != CHECKPOINT_FORMAT {
        return Err("unexpected checkpoint format".into());
    }
    if body.signer_key != verify_key_b64(key) {
        return Err("signer_key does not match the signing key".into());
    }
    let h = checkpoint_hash(&body).map_err(|e| e.to_string())?;
    let signature = crate::B64.encode(key.sign(&h).to_bytes());
    Ok(Checkpoint {
        body,
        checkpoint_hash: hex::encode(h),
        signature,
    })
}

/// Checks a checkpoint's own format, hash and signature (strict, weak keys refused).
pub fn verify_checkpoint(cp: &Checkpoint) -> Result<(), String> {
    if cp.body.format != CHECKPOINT_FORMAT {
        return Err("unknown checkpoint format".into());
    }
    let h = checkpoint_hash(&cp.body).map_err(|e| e.to_string())?;
    if !hex::encode(h).eq_ignore_ascii_case(&cp.checkpoint_hash) {
        return Err("the checkpoint was altered: its hash does not match its contents".into());
    }
    if !verify_hash_signature(&cp.body.signer_key, &cp.checkpoint_hash, &cp.signature) {
        return Err("the signature does not verify under the key written in the checkpoint".into());
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointResult {
    pub seq: String,
    pub timestamp_iso: String,
    /// `consistent`, `beyond_pack`, `before_pack`, `other_stream` (all fine) or
    /// `different_history`, `tail_missing`, `bad_signature`, `wrong_key`, `malformed` (problems).
    pub state: String,
    pub detail: String,
}

impl CheckpointResult {
    fn is_problem(&self) -> bool {
        matches!(
            self.state.as_str(),
            "different_history" | "tail_missing" | "bad_signature" | "wrong_key" | "malformed"
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointReport {
    /// No problem found. Read `checked` too: zero means nothing was actually compared.
    pub consistent: bool,
    /// Checkpoints whose position the pack covered and whose head matched.
    pub checked: usize,
    pub results: Vec<CheckpointResult>,
    /// Problems found across the checkpoints themselves (conflicts, rollback).
    pub problems: Vec<String>,
}

fn num(s: &str) -> Option<i64> {
    s.trim().parse().ok()
}

/// Problems visible from the checkpoints alone: two heads for one position, or
/// a head that moved backwards in time. Only checkpoints for one stream, already
/// signature-checked, belong here.
#[must_use]
pub fn check_checkpoint_set(cps: &[&Checkpoint]) -> Vec<String> {
    let mut problems = Vec::new();
    for (i, a) in cps.iter().enumerate() {
        for b in cps.iter().skip(i + 1) {
            let (Some(sa), Some(sb)) = (num(&a.body.seq), num(&b.body.seq)) else {
                continue;
            };
            if sa == sb && !a.body.head_hash.eq_ignore_ascii_case(&b.body.head_hash) {
                problems.push(format!(
                    "two different histories were signed for position {sa}: the same stream shows two different heads"
                ));
            }
            let (Some(ta), Some(tb)) = (num(&a.body.timestamp_ms), num(&b.body.timestamp_ms))
            else {
                continue;
            };
            let (early, late, se, sl) = if ta <= tb {
                (a, b, sa, sb)
            } else {
                (b, a, sb, sa)
            };
            if late.body.timestamp_ms != early.body.timestamp_ms && sl < se {
                problems.push(format!(
                    "the stream moved backwards: a checkpoint at {} had reached position {se}, but a later one at {} is at position {sl} (rollback)",
                    early.body.timestamp_iso, late.body.timestamp_iso
                ));
            }
        }
    }
    problems.sort();
    problems.dedup();
    problems
}

/// Compares checkpoints the customer kept with an evidence pack presented later.
/// `expected_key` is the platform key you trust for this organization; when absent,
/// the key written in the pack is used (consistency only, not identity).
#[must_use]
pub fn check_checkpoints(
    pack: &EvidencePack,
    cps: &[Checkpoint],
    expected_key: Option<&str>,
) -> CheckpointReport {
    let trusted = expected_key.unwrap_or(&pack.org_verify_key);
    let seqs: Vec<i64> = pack
        .proofs
        .iter()
        .filter_map(|b| num(&b.envelope.seq))
        .collect();
    let (first, last) = (seqs.iter().min().copied(), seqs.iter().max().copied());
    let to_ms = pack.manifest.to_ms;

    let mut results = Vec::new();
    let mut usable: Vec<&Checkpoint> = Vec::new();
    for cp in cps {
        let mut r = CheckpointResult {
            seq: cp.body.seq.clone(),
            timestamp_iso: cp.body.timestamp_iso.clone(),
            state: String::new(),
            detail: String::new(),
        };
        let mut set = |state: &str, detail: &str| {
            r.state = state.into();
            r.detail = detail.into();
        };
        if cp.body.org_id != pack.manifest.org_id || cp.body.stream_id != pack.manifest.stream_id {
            set(
                "other_stream",
                "This checkpoint is for a different stream. Ignored.",
            );
        } else if let Err(e) = verify_checkpoint(cp) {
            set("bad_signature", &e);
        } else if cp.body.signer_key != trusted {
            set(
                "wrong_key",
                "Signed by a different key than the one you trust for this organization.",
            );
        } else if let (Some(seq), Some(ts)) = (num(&cp.body.seq), num(&cp.body.timestamp_ms)) {
            usable.push(cp);
            match (first, last) {
                (Some(f), Some(l)) if seq >= f && seq <= l => {
                    let head = pack
                        .proofs
                        .iter()
                        .find(|b| num(&b.envelope.seq) == Some(seq))
                        .map(|b| b.event_hash.as_str());
                    if head.is_some_and(|h| h.eq_ignore_ascii_case(&cp.body.head_hash)) {
                        set(
                            "consistent",
                            "The pack's record at this position is the one the platform signed.",
                        );
                    } else {
                        set(
                            "different_history",
                            "The pack has a different record at this position than the platform signed. This is not the same history.",
                        );
                    }
                }
                (Some(_), Some(l)) if seq > l => {
                    if to_ms >= ts {
                        set(
                            "tail_missing",
                            "The pack claims to cover this moment but stops before this checkpoint's position. Newer records are missing.",
                        );
                    } else {
                        set("beyond_pack", "This checkpoint is later than the range the pack covers. Not compared.");
                    }
                }
                (Some(_), Some(_)) => {
                    set(
                        "before_pack",
                        "This checkpoint is earlier than the pack's first record. Not compared.",
                    );
                }
                _ => set("beyond_pack", "The pack has no records to compare."),
            }
        } else {
            set(
                "malformed",
                "The position or time in this checkpoint is not a number.",
            );
        }
        results.push(r);
    }
    let problems = check_checkpoint_set(&usable);
    let checked = results.iter().filter(|r| r.state == "consistent").count();
    let consistent = problems.is_empty() && !results.iter().any(CheckpointResult::is_problem);
    CheckpointReport {
        consistent,
        checked,
        results,
        problems,
    }
}

/// Reads checkpoints from what a person has: one checkpoint, a list, or the
/// platform's `{ "checkpoints": [...] }` response.
pub fn checkpoints_from_value(v: serde_json::Value) -> Result<Vec<Checkpoint>, String> {
    let list = if v.is_array() {
        v
    } else if let Some(inner) = v.get("checkpoints") {
        inner.clone()
    } else {
        serde_json::Value::Array(vec![v])
    };
    serde_json::from_value(list)
        .map_err(|e| format!("not a checkpoint or a list of checkpoints: {e}"))
}

/// Text in, report out: used by the command-line tool and the browser verifier.
/// `expected_key` is the platform key you trust for the organization, or empty.
pub fn check_checkpoints_text(
    pack: &str,
    checkpoints: &str,
    expected_key: &str,
) -> Result<CheckpointReport, String> {
    let pack: EvidencePack = serde_json::from_str(pack)
        .map_err(|_| "The pack is not a valid HSIP evidence pack.".to_string())?;
    let value: serde_json::Value = serde_json::from_str(checkpoints)
        .map_err(|_| "The checkpoints are not valid JSON.".to_string())?;
    let cps = checkpoints_from_value(value)?;
    let key = expected_key.trim();
    Ok(check_checkpoints(
        &pack,
        &cps,
        if key.is_empty() { None } else { Some(key) },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        manifest_hash, sign_envelope, sign_manifest, Envelope, PackManifest, ProofBundle,
        BUNDLE_FORMAT, ENVELOPE_FORMAT, PACK_FORMAT,
    };
    use std::collections::BTreeMap;

    const NOW: i64 = 1_800_000_000_000;

    fn key(n: u8) -> SigningKey {
        SigningKey::from_bytes(&[n; 32])
    }

    /// A stream of `n` records whose action names carry `tag`, so two tags are two histories.
    fn history(n: u64, tag: &str, signer: &SigningKey) -> Vec<ProofBundle> {
        let mut out: Vec<ProofBundle> = Vec::new();
        for seq in 1..=n {
            let prev = out.last().map(|b| b.event_hash.clone()).unwrap_or_default();
            let env = Envelope {
                format: ENVELOPE_FORMAT.into(),
                proof_id: format!("p{seq}"),
                org_id: "org1".into(),
                stream_id: "s1".into(),
                seq: seq.to_string(),
                actor: "bot".into(),
                action: format!("act.{tag}.{seq}"),
                subject: String::new(),
                payload_hash: "0".repeat(64),
                metadata: BTreeMap::new(),
                prev_hash: prev,
                timestamp_iso: String::new(),
                timestamp_ms: (NOW + seq as i64 * 1000).to_string(),
            };
            let (event_hash, signature) = sign_envelope(&env, signer).unwrap();
            out.push(ProofBundle {
                format: BUNDLE_FORMAT.into(),
                envelope: env,
                event_hash,
                signature,
                sign_algo: "ED25519".into(),
                org_verify_key: verify_key_b64(signer),
                org_name: "Acme".into(),
                anchor: None,
            });
        }
        out
    }

    fn pack_of(proofs: Vec<ProofBundle>, platform: &SigningKey, to_ms: i64) -> EvidencePack {
        let manifest = PackManifest {
            org_id: "org1".into(),
            stream_id: "s1".into(),
            from_ms: 0,
            to_ms,
            event_hashes: proofs.iter().map(|b| b.event_hash.clone()).collect(),
            issuer_keys: vec![],
        };
        let (mh, ms) = sign_manifest(&manifest, platform).unwrap();
        assert_eq!(hex::encode(manifest_hash(&manifest).unwrap()), mh);
        EvidencePack {
            format: PACK_FORMAT.into(),
            org_name: "Acme".into(),
            org_verify_key: verify_key_b64(platform),
            stream_name: "S".into(),
            generated_at_iso: String::new(),
            manifest,
            manifest_hash: mh,
            manifest_signature: ms,
            proofs,
        }
    }

    fn cp_at(platform: &SigningKey, seq: u64, head: &str, at_ms: i64, prev: &str) -> Checkpoint {
        sign_checkpoint(
            CheckpointBody {
                format: CHECKPOINT_FORMAT.into(),
                org_id: "org1".into(),
                stream_id: "s1".into(),
                seq: seq.to_string(),
                head_hash: head.into(),
                timestamp_ms: at_ms.to_string(),
                timestamp_iso: format!("t{at_ms}"),
                prev_checkpoint_hash: prev.into(),
                signer_key: verify_key_b64(platform),
            },
            platform,
        )
        .unwrap()
    }

    #[test]
    fn a_checkpoint_verifies_and_any_change_breaks_it() {
        let p = key(1);
        let cp = cp_at(&p, 3, &"a".repeat(64), NOW, "");
        assert!(verify_checkpoint(&cp).is_ok());
        let mut t = cp.clone();
        t.body.seq = "2".into();
        assert!(verify_checkpoint(&t).is_err());
        let mut t = cp.clone();
        t.body.signer_key = verify_key_b64(&key(2));
        assert!(verify_checkpoint(&t).is_err());
        // A weak key cannot sign a checkpoint for anyone.
        let mut identity = [0u8; 32];
        identity[0] = 1;
        let mut t = cp.clone();
        t.body.signer_key = crate::B64.encode(identity);
        t.checkpoint_hash = hex::encode(checkpoint_hash(&t.body).unwrap());
        let mut sig = [0u8; 64];
        sig[0] = 1;
        t.signature = crate::B64.encode(sig);
        assert!(verify_checkpoint(&t).is_err());
    }

    #[test]
    fn the_same_history_is_consistent() {
        let p = key(1);
        let h = history(5, "a", &p);
        let cp = cp_at(&p, 3, &h[2].event_hash, NOW + 3_500, "");
        let r = check_checkpoints(&pack_of(h, &p, NOW + 60_000), &[cp], None);
        assert!(r.consistent && r.checked == 1, "{r:?}");
        assert_eq!(r.results[0].state, "consistent");
    }

    #[test]
    fn a_rewritten_history_is_caught_even_though_the_pack_is_valid_on_its_own() {
        let p = key(1);
        let honest = history(5, "a", &p);
        let cp = cp_at(&p, 3, &honest[2].event_hash, NOW + 3_500, "");
        // The platform presents a different, internally valid history from position 2 on.
        let forked = history(5, "b", &p);
        assert!(crate::verify_pack(&pack_of(forked.clone(), &p, NOW + 60_000)).valid);
        let r = check_checkpoints(&pack_of(forked, &p, NOW + 60_000), &[cp], None);
        assert!(!r.consistent);
        assert_eq!(r.results[0].state, "different_history");
    }

    #[test]
    fn a_truncated_tail_is_caught_only_when_the_pack_claims_to_reach_it() {
        let p = key(1);
        let full = history(5, "a", &p);
        let cp = cp_at(&p, 5, &full[4].event_hash, NOW + 5_500, "");
        let short = full[..3].to_vec();
        // The pack says it covers everything up to a time after the checkpoint.
        let r = check_checkpoints(
            &pack_of(short.clone(), &p, NOW + 60_000),
            std::slice::from_ref(&cp),
            None,
        );
        assert!(!r.consistent);
        assert_eq!(r.results[0].state, "tail_missing");
        // A pack that only claims to cover an earlier range is not accused.
        let r = check_checkpoints(&pack_of(short, &p, NOW + 3_000), &[cp], None);
        assert!(r.consistent && r.checked == 0, "{r:?}");
        assert_eq!(r.results[0].state, "beyond_pack");
    }

    #[test]
    fn two_heads_for_one_position_and_a_backwards_head_are_flagged() {
        let p = key(1);
        let a = history(5, "a", &p);
        let b = history(5, "b", &p);
        let c1 = cp_at(&p, 4, &a[3].event_hash, NOW + 4_500, "");
        let c2 = cp_at(&p, 4, &b[3].event_hash, NOW + 4_600, "");
        let problems = check_checkpoint_set(&[&c1, &c2]);
        assert!(
            problems
                .iter()
                .any(|x| x.contains("two different histories")),
            "{problems:?}"
        );
        // Position 5 at one time, position 3 later: the stream went backwards.
        let hi = cp_at(&p, 5, &a[4].event_hash, NOW + 5_000, "");
        let lo = cp_at(&p, 3, &a[2].event_hash, NOW + 9_000, "");
        let problems = check_checkpoint_set(&[&hi, &lo]);
        assert!(
            problems.iter().any(|x| x.contains("rollback")),
            "{problems:?}"
        );
        // Normal growth is fine.
        let early = cp_at(&p, 3, &a[2].event_hash, NOW + 3_000, "");
        assert!(check_checkpoint_set(&[&early, &hi]).is_empty());
    }

    #[test]
    fn wrong_key_bad_signature_and_other_streams_are_reported() {
        let p = key(1);
        let h = history(3, "a", &p);
        let pack = pack_of(h.clone(), &p, NOW + 60_000);
        let by_other = cp_at(&key(9), 2, &h[1].event_hash, NOW + 2_500, "");
        let r = check_checkpoints(&pack, &[by_other], None);
        assert_eq!(r.results[0].state, "wrong_key");
        let mut forged = cp_at(&p, 2, &h[1].event_hash, NOW + 2_500, "");
        forged.body.head_hash = "f".repeat(64);
        let r = check_checkpoints(&pack, &[forged], None);
        assert_eq!(r.results[0].state, "bad_signature");
        let mut elsewhere = cp_at(&p, 2, &h[1].event_hash, NOW + 2_500, "");
        elsewhere.body.stream_id = "other".into();
        let r = check_checkpoints(&pack, &[elsewhere], None);
        assert_eq!(r.results[0].state, "other_stream");
        // Pinning a key you trust beats the key written in the pack.
        let good = cp_at(&p, 2, &h[1].event_hash, NOW + 2_500, "");
        let r = check_checkpoints(&pack, &[good], Some(&verify_key_b64(&key(7))));
        assert_eq!(r.results[0].state, "wrong_key");
    }

    #[test]
    fn checkpoints_are_read_from_a_single_object_a_list_or_the_api_response() {
        let p = key(1);
        let cp = cp_at(&p, 1, &"a".repeat(64), NOW, "");
        let one = serde_json::to_value(&cp).unwrap();
        assert_eq!(checkpoints_from_value(one.clone()).unwrap().len(), 1);
        assert_eq!(
            checkpoints_from_value(serde_json::json!([one.clone(), one.clone()]))
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            checkpoints_from_value(serde_json::json!({ "checkpoints": [one] }))
                .unwrap()
                .len(),
            1
        );
        assert!(checkpoints_from_value(serde_json::json!({ "nope": 1 })).is_err());
    }
}
