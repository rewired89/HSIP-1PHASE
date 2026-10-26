//! Permissions ("mandates"): a signed statement of what an agent may do, and
//! a pure check of recorded proofs against it.
//!
//! A person or organization signs a mandate with their own Ed25519 key. Later,
//! anyone holding the mandate and the recorded proofs can run `evaluate` and
//! see which records fall outside it, with no account and no network. This
//! proves what was *recorded* against what was *allowed*. It does not stop an
//! agent, and it cannot see actions that were never recorded.

use std::collections::BTreeMap;

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{verify_bundle, ProofBundle};

pub const MANDATE_FORMAT: &str = "hsip-mandate/1";

/// Lets a record's own metadata carry the numbers the rules check. The
/// sender declares them, so a mandate is only as honest as the recorder.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rules {
    /// Actions the agent may take. Empty means any action.
    #[serde(default)]
    pub allowed_actions: Vec<String>,
    /// At most `count` records in any `period_secs` window.
    #[serde(default)]
    pub max_per_period: Option<RateRule>,
    /// `metadata[field]` is a whole number that must not exceed `limit`.
    #[serde(default)]
    pub max_amount: Option<AmountRule>,
    /// `metadata[field]` must be one of the listed values.
    #[serde(default)]
    pub allowed_values: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RateRule {
    pub count: u32,
    pub period_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AmountRule {
    pub field: String,
    pub limit: u64,
}

/// The signed part. Times are Unix milliseconds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MandateBody {
    pub format: String,
    pub mandate_id: String,
    /// Public key (base64) of whoever grants the permission.
    pub grantor_key: String,
    /// Matches `actor` in the recorded envelopes.
    pub agent: String,
    pub valid_from_ms: i64,
    pub valid_until_ms: i64,
    pub rules: Rules,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Mandate {
    #[serde(flatten)]
    pub body: MandateBody,
    pub signature: String,
}

pub fn mandate_hash(body: &MandateBody) -> Result<[u8; 32], serde_json::Error> {
    Ok(Sha256::digest(serde_jcs::to_vec(body)?).into())
}

pub fn sign_mandate(body: MandateBody, key: &SigningKey) -> Result<Mandate, String> {
    if body.format != MANDATE_FORMAT {
        return Err("unexpected mandate format".into());
    }
    if body.grantor_key != B64.encode(key.verifying_key().to_bytes()) {
        return Err("grantor_key does not match the signing key".into());
    }
    let h = mandate_hash(&body).map_err(|e| e.to_string())?;
    let signature = B64.encode(key.sign(&h).to_bytes());
    Ok(Mandate { body, signature })
}

/// True when the signature is valid for the grantor key named in the body.
#[must_use]
pub fn verify_mandate(m: &Mandate) -> bool {
    (|| {
        if m.body.format != MANDATE_FORMAT || m.body.valid_until_ms < m.body.valid_from_ms {
            return None;
        }
        let vk: [u8; 32] = B64
            .decode(m.body.grantor_key.trim())
            .ok()?
            .try_into()
            .ok()?;
        let vk = VerifyingKey::from_bytes(&vk).ok()?;
        let sig: [u8; 64] = B64.decode(m.signature.trim()).ok()?.try_into().ok()?;
        let h = mandate_hash(&m.body).ok()?;
        Some(vk.verify(&h, &Signature::from_bytes(&sig)).is_ok())
    })()
    .unwrap_or(false)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Violation {
    pub proof_id: String,
    pub seq: String,
    pub event_hash: String,
    /// `not_genuine`, `outside_window`, `action_not_allowed`, `amount_over_limit`,
    /// `value_not_allowed`, `field_missing`, `rate_exceeded`.
    pub rule: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MandateReport {
    /// The mandate's own signature verifies. When false nothing else counts.
    pub mandate_genuine: bool,
    /// Records checked: those whose `actor` is the mandate's agent.
    pub checked: usize,
    /// Records for other actors, ignored.
    pub ignored: usize,
    pub violations: Vec<Violation>,
    /// Mandate genuine and no violations.
    pub within_permission: bool,
}

fn v(b: &ProofBundle, rule: &str, detail: String) -> Violation {
    Violation {
        proof_id: b.envelope.proof_id.clone(),
        seq: b.envelope.seq.clone(),
        event_hash: b.event_hash.clone(),
        rule: rule.into(),
        detail,
    }
}

/// Reads proofs from whatever a person has on hand: one bundle, a list of
/// bundles, or an evidence pack.
pub fn proofs_from_value(v: serde_json::Value) -> Result<Vec<ProofBundle>, String> {
    if v.is_array() {
        return serde_json::from_value(v).map_err(|e| format!("malformed proof list: {e}"));
    }
    if v.get("format").and_then(serde_json::Value::as_str) == Some(crate::PACK_FORMAT) {
        let p: crate::EvidencePack =
            serde_json::from_value(v).map_err(|e| format!("malformed evidence pack: {e}"))?;
        return Ok(p.proofs);
    }
    serde_json::from_value::<ProofBundle>(v)
        .map(|b| vec![b])
        .map_err(|e| format!("not a proof, a list of proofs or an evidence pack: {e}"))
}

/// Checks recorded proofs against a mandate. Pure: no I/O.
#[must_use]
pub fn evaluate(m: &Mandate, proofs: &[ProofBundle]) -> MandateReport {
    let genuine = verify_mandate(m);
    if !genuine {
        return MandateReport {
            mandate_genuine: false,
            checked: 0,
            ignored: proofs.len(),
            violations: vec![],
            within_permission: false,
        };
    }
    let r = &m.body.rules;
    let mut violations = Vec::new();
    let mut covered: Vec<(&ProofBundle, i64)> = Vec::new();
    let mut ignored = 0;

    for b in proofs {
        if b.envelope.actor != m.body.agent {
            ignored += 1;
            continue;
        }
        let ts: i64 = b.envelope.timestamp_ms.parse().unwrap_or(i64::MIN);
        covered.push((b, ts));

        if !verify_bundle(b).valid {
            violations.push(v(
                b,
                "not_genuine",
                "this record does not verify (altered or invalid)".into(),
            ));
        }
        if ts < m.body.valid_from_ms || ts > m.body.valid_until_ms {
            violations.push(v(
                b,
                "outside_window",
                "recorded outside the time the permission was valid".into(),
            ));
        }
        if !r.allowed_actions.is_empty() && !r.allowed_actions.contains(&b.envelope.action) {
            violations.push(v(
                b,
                "action_not_allowed",
                format!("action `{}` is not permitted", b.envelope.action),
            ));
        }
        if let Some(a) = &r.max_amount {
            match b
                .envelope
                .metadata
                .get(&a.field)
                .and_then(|x| x.parse::<u64>().ok())
            {
                None => violations.push(v(
                    b,
                    "field_missing",
                    format!("`{}` is missing or not a whole number", a.field),
                )),
                Some(n) if n > a.limit => violations.push(v(
                    b,
                    "amount_over_limit",
                    format!("`{}` is {n}, above the limit {}", a.field, a.limit),
                )),
                Some(_) => {}
            }
        }
        for (field, allowed) in &r.allowed_values {
            match b.envelope.metadata.get(field) {
                None => violations.push(v(b, "field_missing", format!("`{field}` is missing"))),
                Some(x) if !allowed.contains(x) => violations.push(v(
                    b,
                    "value_not_allowed",
                    format!("`{field}` = `{x}` is not permitted"),
                )),
                Some(_) => {}
            }
        }
    }

    if let Some(rate) = &r.max_per_period {
        let window = i64::try_from(rate.period_secs)
            .unwrap_or(i64::MAX / 1000)
            .saturating_mul(1000);
        let mut times: Vec<i64> = covered.iter().map(|(_, t)| *t).collect();
        times.sort_unstable();
        for (b, t) in &covered {
            let in_window = times
                .iter()
                .filter(|x| **x <= *t && **x > t.saturating_sub(window))
                .count();
            if in_window > rate.count as usize {
                violations.push(v(
                    b,
                    "rate_exceeded",
                    format!(
                        "{in_window} records within {}s, above the limit {}",
                        rate.period_secs, rate.count
                    ),
                ));
            }
        }
    }

    let within = violations.is_empty();
    MandateReport {
        mandate_genuine: true,
        checked: covered.len(),
        ignored,
        violations,
        within_permission: within,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{sign_envelope, Envelope, BUNDLE_FORMAT, ENVELOPE_FORMAT};

    fn key(n: u8) -> SigningKey {
        SigningKey::from_bytes(&[n; 32])
    }

    fn body(agent: &str, rules: Rules) -> MandateBody {
        MandateBody {
            format: MANDATE_FORMAT.into(),
            mandate_id: "m1".into(),
            grantor_key: B64.encode(key(1).verifying_key().to_bytes()),
            agent: agent.into(),
            valid_from_ms: 1_000,
            valid_until_ms: 100_000,
            rules,
        }
    }

    fn bundle(seq: u32, actor: &str, action: &str, ts: i64, amount: &str) -> ProofBundle {
        let mut md = BTreeMap::new();
        md.insert("amount".to_string(), amount.to_string());
        md.insert("payee".to_string(), "acme".to_string());
        let env = Envelope {
            format: ENVELOPE_FORMAT.into(),
            proof_id: format!("p{seq}"),
            org_id: "o".into(),
            stream_id: "s".into(),
            seq: seq.to_string(),
            actor: actor.into(),
            action: action.into(),
            subject: String::new(),
            payload_hash: "0".repeat(64),
            metadata: md,
            prev_hash: String::new(),
            timestamp_iso: String::new(),
            timestamp_ms: ts.to_string(),
        };
        let k = key(2);
        let (event_hash, signature) = sign_envelope(&env, &k).unwrap();
        ProofBundle {
            format: BUNDLE_FORMAT.into(),
            envelope: env,
            event_hash,
            signature,
            sign_algo: "ED25519".into(),
            org_verify_key: B64.encode(k.verifying_key().to_bytes()),
            org_name: String::new(),
            anchor: None,
        }
    }

    fn rules() -> Rules {
        let mut allowed = BTreeMap::new();
        allowed.insert("payee".to_string(), vec!["acme".to_string()]);
        Rules {
            allowed_actions: vec!["pay".into()],
            max_per_period: Some(RateRule {
                count: 2,
                period_secs: 60,
            }),
            max_amount: Some(AmountRule {
                field: "amount".into(),
                limit: 100,
            }),
            allowed_values: allowed,
        }
    }

    #[test]
    fn signed_mandate_verifies_and_tampering_breaks_it() {
        let m = sign_mandate(body("bot", rules()), &key(1)).unwrap();
        assert!(verify_mandate(&m));
        let mut t = m.clone();
        t.body.rules.max_amount.as_mut().unwrap().limit = 1_000_000;
        assert!(!verify_mandate(&t));
        // Signed by someone other than the named grantor.
        assert!(sign_mandate(body("bot", rules()), &key(9)).is_err());
    }

    #[test]
    fn records_inside_the_permission_pass() {
        let m = sign_mandate(body("bot", rules()), &key(1)).unwrap();
        let r = evaluate(
            &m,
            &[
                bundle(1, "bot", "pay", 10_000, "50"),
                bundle(2, "bot", "pay", 20_000, "100"),
            ],
        );
        assert!(r.within_permission, "{:?}", r.violations);
        assert_eq!(r.checked, 2);
    }

    #[test]
    fn every_rule_catches_its_violation() {
        let m = sign_mandate(body("bot", rules()), &key(1)).unwrap();
        let rule_of = |b: ProofBundle| -> Vec<String> {
            evaluate(&m, &[b])
                .violations
                .into_iter()
                .map(|x| x.rule)
                .collect()
        };
        assert_eq!(
            rule_of(bundle(1, "bot", "transfer", 10_000, "5")),
            vec!["action_not_allowed"]
        );
        assert_eq!(
            rule_of(bundle(1, "bot", "pay", 10_000, "101")),
            vec!["amount_over_limit"]
        );
        assert_eq!(
            rule_of(bundle(1, "bot", "pay", 500_000, "5")),
            vec!["outside_window"]
        );
        let mut wrong_payee = bundle(1, "bot", "pay", 10_000, "5");
        wrong_payee
            .envelope
            .metadata
            .insert("payee".into(), "evil".into());
        // Changing metadata also breaks the record's own hash.
        let got = rule_of(wrong_payee);
        assert!(got.contains(&"value_not_allowed".to_string()));
        assert!(got.contains(&"not_genuine".to_string()));
    }

    #[test]
    fn rate_limit_and_other_actors() {
        let m = sign_mandate(body("bot", rules()), &key(1)).unwrap();
        let r = evaluate(
            &m,
            &[
                bundle(1, "bot", "pay", 10_000, "5"),
                bundle(2, "bot", "pay", 20_000, "5"),
                bundle(3, "bot", "pay", 30_000, "5"),
                bundle(4, "someone-else", "pay", 40_000, "999999"),
            ],
        );
        assert_eq!(r.checked, 3);
        assert_eq!(r.ignored, 1);
        assert_eq!(r.violations.len(), 1);
        assert_eq!(r.violations[0].rule, "rate_exceeded");
        assert_eq!(r.violations[0].seq, "3");
        assert!(!r.within_permission);
    }

    #[test]
    fn a_forged_mandate_is_not_trusted() {
        let mut m = sign_mandate(body("bot", rules()), &key(1)).unwrap();
        m.body.agent = "other".into();
        let r = evaluate(&m, &[bundle(1, "other", "pay", 10_000, "5")]);
        assert!(!r.mandate_genuine);
        assert!(!r.within_permission);
    }
}
