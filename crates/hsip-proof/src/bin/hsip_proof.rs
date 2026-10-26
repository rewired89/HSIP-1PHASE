//! `hsip-proof verify <file.json>`: verify a proof bundle or evidence pack
//! offline, with no account, no network and no HSIP server.
//!
//! Customer-held keys:
//!   `hsip-proof keygen > my-key.json`            create your signing key (keep it private)
//!   `hsip-proof register-proof <org-id> my-key.json`   prove you hold it, for the console
//!
//! Permissions:
//!   `hsip-proof mandate-sign my-key.json mandate-body.json > mandate.json`
//!   `hsip-proof mandate-check mandate.json proofs-or-pack.json`   which records fall outside it

use hsip_proof::{verify_bundle, verify_pack, Check, EvidencePack, ProofBundle};
use std::process::ExitCode;

fn print_checks(checks: &[Check]) {
    for c in checks {
        let mark = match c.ok {
            Some(true) => "PASS",
            Some(false) => "FAIL",
            None => "N/A ",
        };
        println!("  [{mark}] {:<24} {}", c.name, c.detail);
    }
}

fn read_json(path: &str) -> Result<serde_json::Value, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("{path} is not valid JSON: {e}"))
}

fn mandate_sign(key_path: &str, body_path: &str) -> ExitCode {
    let run = || -> Result<String, String> {
        let k = read_json(key_path)?;
        let key = k
            .get("private_key_b64")
            .and_then(|s| s.as_str())
            .and_then(hsip_proof::signing_key_from_b64)
            .ok_or("key file must be the JSON printed by `hsip-proof keygen`")?;
        let mut body: hsip_proof::MandateBody = serde_json::from_value(read_json(body_path)?)
            .map_err(|e| format!("bad mandate body: {e}"))?;
        body.format = hsip_proof::MANDATE_FORMAT.into();
        body.grantor_key = hsip_proof::verify_key_b64(&key);
        let m = hsip_proof::sign_mandate(body, &key)?;
        serde_json::to_string_pretty(&m).map_err(|e| e.to_string())
    };
    match run() {
        Ok(s) => {
            println!("{s}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}

fn mandate_check(mandate_path: &str, proofs_path: &str) -> ExitCode {
    let run = || -> Result<hsip_proof::MandateReport, String> {
        let m: hsip_proof::Mandate = serde_json::from_value(read_json(mandate_path)?)
            .map_err(|e| format!("not a mandate: {e}"))?;
        let proofs = hsip_proof::proofs_from_value(read_json(proofs_path)?)?;
        Ok(hsip_proof::evaluate_mandate(&m, &proofs))
    };
    let r = match run() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    if !r.mandate_genuine {
        println!("RESULT: the permission itself is not genuine (bad signature or format)");
        return ExitCode::from(1);
    }
    println!(
        "Checked {} records for this agent ({} for others ignored)",
        r.checked, r.ignored
    );
    for x in &r.violations {
        println!("  [OUTSIDE] seq {} {:<20} {}", x.seq, x.rule, x.detail);
    }
    if r.checked == 0 {
        println!("RESULT: no records for this agent were found, so nothing was checked");
        ExitCode::from(3)
    } else if r.within_permission {
        println!("RESULT: every checked record is inside the permission");
        ExitCode::SUCCESS
    } else {
        println!(
            "RESULT: {} record(s) fall outside the permission",
            r.violations.len()
        );
        ExitCode::from(1)
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && args[1] == "keygen" {
        let k = hsip_proof::generate_signing_key();
        println!(
            "{}",
            serde_json::json!({
                "private_key_b64": hsip_proof::seed_b64(&k),
                "public_key_b64": hsip_proof::verify_key_b64(&k),
                "note": "Keep private_key_b64 secret. Register only public_key_b64 with HSIP."
            })
        );
        return ExitCode::SUCCESS;
    }
    if args.len() == 4 && args[1] == "register-proof" {
        let raw = match std::fs::read_to_string(&args[3]) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("cannot read {}: {e}", args[3]);
                return ExitCode::from(2);
            }
        };
        let seed = serde_json::from_str::<serde_json::Value>(&raw)
            .ok()
            .and_then(|v| {
                v.get("private_key_b64")
                    .and_then(|s| s.as_str().map(String::from))
            });
        let Some(key) = seed.and_then(|s| hsip_proof::signing_key_from_b64(&s)) else {
            eprintln!("key file must be the JSON printed by `hsip-proof keygen`");
            return ExitCode::from(2);
        };
        println!(
            "{}",
            serde_json::json!({
                "verify_key": hsip_proof::verify_key_b64(&key),
                "proof": hsip_proof::sign_registration(&args[2], &key)
            })
        );
        return ExitCode::SUCCESS;
    }
    if args.len() == 4 && args[1] == "mandate-sign" {
        return mandate_sign(&args[2], &args[3]);
    }
    if args.len() == 4 && args[1] == "mandate-check" {
        return mandate_check(&args[2], &args[3]);
    }
    if args.len() != 3 || args[1] != "verify" {
        eprintln!("usage: hsip-proof verify <bundle-or-pack.json>\n       hsip-proof keygen\n       hsip-proof register-proof <org-id> <key.json>\n       hsip-proof mandate-sign <key.json> <mandate-body.json>\n       hsip-proof mandate-check <mandate.json> <proofs-or-pack.json>");
        return ExitCode::from(2);
    }
    let raw = match std::fs::read_to_string(&args[2]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {}: {e}", args[2]);
            return ExitCode::from(2);
        }
    };
    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("not valid JSON: {e}");
            return ExitCode::from(2);
        }
    };
    let format = value.get("format").and_then(|f| f.as_str()).unwrap_or("");
    let ok = if format == hsip_proof::PACK_FORMAT {
        match serde_json::from_value::<EvidencePack>(value) {
            Ok(p) => {
                let v = verify_pack(&p);
                println!(
                    "Evidence pack: {} proofs, {} anchored",
                    v.proof_count, v.anchored_count
                );
                print_checks(&v.checks);
                v.valid
            }
            Err(e) => {
                eprintln!("malformed evidence pack: {e}");
                return ExitCode::from(2);
            }
        }
    } else {
        match serde_json::from_value::<ProofBundle>(value) {
            Ok(b) => {
                let v = verify_bundle(&b);
                println!(
                    "Proof {} ({})",
                    b.envelope.proof_id,
                    if v.anchored {
                        "anchored"
                    } else {
                        "not anchored yet"
                    }
                );
                print_checks(&v.checks);
                v.valid
            }
            Err(e) => {
                eprintln!("malformed proof bundle: {e}");
                return ExitCode::from(2);
            }
        }
    };
    println!(
        "{}",
        if ok {
            "RESULT: VALID"
        } else {
            "RESULT: INVALID"
        }
    );
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
