//! The same verifier the `hsip-proof` command-line tool and the platform use,
//! compiled to WebAssembly so a browser can check a proof with no server.
//! Pure function of its input: no network, no storage.

use hsip_proof::{verify_bundle, verify_pack, EvidencePack, ProofBundle, PACK_FORMAT};
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

/// Verifies a proof bundle or an evidence pack given as JSON text.
/// Always returns JSON: `{kind, valid, result, org_name?}` or `{error}`.
#[wasm_bindgen]
pub fn verify(text: &str) -> String {
    verify_text(text).to_string()
}

/// Checks recorded proofs (a proof, a list, or an evidence pack, as JSON text)
/// against a signed permission (JSON text). Pure function, no network.
#[wasm_bindgen]
pub fn check_permission(mandate: &str, proofs: &str) -> String {
    check_permission_text(mandate, proofs).to_string()
}

pub fn check_permission_text(mandate: &str, proofs: &str) -> Value {
    let m: hsip_proof::Mandate = match serde_json::from_str(mandate) {
        Ok(m) => m,
        Err(_) => return json!({ "error": "The permission is not a valid HSIP mandate." }),
    };
    let parsed = serde_json::from_str::<Value>(proofs)
        .map_err(|_| "The proofs are not valid JSON.".to_string())
        .and_then(hsip_proof::proofs_from_value);
    match parsed {
        Ok(p) => json!({ "report": hsip_proof::evaluate_mandate(&m, &p) }),
        Err(e) => json!({ "error": e }),
    }
}

/// Public key (base64) for a 32-byte seed (base64) made by the browser's
/// secure random generator. Returns an empty string for a bad seed.
#[wasm_bindgen]
pub fn public_key(seed_b64: &str) -> String {
    hsip_proof::signing_key_from_b64(seed_b64)
        .map(|k| hsip_proof::verify_key_b64(&k))
        .unwrap_or_default()
}

/// Signs a permission body (JSON text) with the seed. The key never leaves
/// the page. Returns the signed permission, or `{error}`.
#[wasm_bindgen]
pub fn sign_permission(seed_b64: &str, body: &str) -> String {
    sign_permission_text(seed_b64, body).to_string()
}

pub fn sign_permission_text(seed_b64: &str, body: &str) -> Value {
    let Some(key) = hsip_proof::signing_key_from_b64(seed_b64) else {
        return json!({ "error": "That is not a valid private key." });
    };
    let mut b: hsip_proof::MandateBody = match serde_json::from_str(body) {
        Ok(b) => b,
        Err(e) => return json!({ "error": format!("The rules are not valid: {e}") }),
    };
    b.format = hsip_proof::MANDATE_FORMAT.into();
    b.grantor_key = hsip_proof::verify_key_b64(&key);
    if b.agent.trim().is_empty() || b.mandate_id.trim().is_empty() {
        return json!({ "error": "An agent name and a permission id are required." });
    }
    match hsip_proof::sign_mandate(b, &key) {
        Ok(m) => json!({ "mandate": m }),
        Err(e) => json!({ "error": e }),
    }
}

pub fn verify_text(text: &str) -> Value {
    let body: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return json!({ "error": "That is not valid JSON." }),
    };
    // Accept the console's {"bundle": ...} wrapper as well as a bare bundle.
    let body = match body.get("bundle") {
        Some(inner) if body.get("format").is_none() => inner.clone(),
        _ => body,
    };
    let format = body.get("format").and_then(Value::as_str).unwrap_or("");
    if format == PACK_FORMAT {
        return match serde_json::from_value::<EvidencePack>(body) {
            Ok(p) => {
                let v = verify_pack(&p);
                json!({ "kind": "evidence_pack", "valid": v.valid, "result": v })
            }
            Err(_) => json!({ "error": "This looks like an evidence pack but is malformed." }),
        };
    }
    match serde_json::from_value::<ProofBundle>(body) {
        Ok(b) => {
            let v = verify_bundle(&b);
            json!({ "kind": "proof", "valid": v.valid, "result": v, "org_name": b.org_name })
        }
        Err(_) => json!({ "error": "This is not an HSIP proof bundle or evidence pack." }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_proof_verifies_and_tampering_is_caught() {
        let sample = include_str!("../../../docs/examples/prove/sample-proof.json");
        assert_eq!(verify_text(sample)["valid"], true);
        let mut v: Value = serde_json::from_str(sample).unwrap();
        v["envelope"]["action"] = json!("something.else");
        assert_eq!(verify_text(&v.to_string())["valid"], false);
    }

    #[test]
    fn sample_pack_verifies() {
        let pack = include_str!("../../../docs/examples/prove/sample-evidence-pack.json");
        let r = verify_text(pack);
        assert_eq!(r["kind"], "evidence_pack");
        assert_eq!(r["valid"], true);
    }

    #[test]
    fn browser_signed_permission_verifies_and_rejects_bad_input() {
        let seed = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=";
        assert!(!public_key(seed).is_empty());
        assert!(public_key("nope").is_empty());
        let body = r#"{"format":"","mandate_id":"m","grantor_key":"","agent":"bot","valid_from_ms":0,"valid_until_ms":10,"rules":{}}"#;
        let r = sign_permission_text(seed, body);
        let m: hsip_proof::Mandate = serde_json::from_value(r["mandate"].clone()).unwrap();
        assert!(hsip_proof::verify_mandate(&m));
        assert!(sign_permission_text("nope", body)["error"].is_string());
        assert!(sign_permission_text(seed, "{}")["error"].is_string());
    }

    #[test]
    fn permission_check_rejects_junk_without_panicking() {
        assert!(check_permission_text("{}", "[]")["error"].is_string());
        assert!(check_permission_text("nope", "[]")["error"].is_string());
    }

    #[test]
    fn garbage_is_reported_not_panicked() {
        assert!(verify_text("nope")["error"].is_string());
        assert!(verify_text("{}")["error"].is_string());
    }
}
