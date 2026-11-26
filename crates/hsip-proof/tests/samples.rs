//! The checked-in samples under docs/examples/prove were produced by a real
//! platform run. They pin the wire format: if canonicalization, hashing or
//! signing changes, these stop verifying, which would also invalidate every
//! proof already issued. Treat a failure here as a breaking format change.

use hsip_proof::{verify_bundle, verify_pack, EvidencePack, ProofBundle};

fn sample(name: &str) -> String {
    let p = format!(
        "{}/../../docs/examples/prove/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"))
}

#[test]
fn sample_proof_bundle_verifies_and_is_anchored() {
    let b: ProofBundle = serde_json::from_str(&sample("sample-proof.json")).unwrap();
    let v = verify_bundle(&b);
    assert!(v.valid, "{:?}", v.checks);
    assert!(v.anchored);
}

#[test]
fn sample_evidence_pack_verifies() {
    let p: EvidencePack = serde_json::from_str(&sample("sample-evidence-pack.json")).unwrap();
    let v = verify_pack(&p);
    assert!(v.valid, "{:?}", v.checks);
    assert_eq!(v.proof_count, 3);
}

#[test]
fn any_single_byte_change_to_the_sample_is_detected() {
    let mut b: ProofBundle = serde_json::from_str(&sample("sample-proof.json")).unwrap();
    b.envelope.subject.push('x');
    assert!(!verify_bundle(&b).valid);
}

#[test]
fn sample_pack_with_a_record_removed_is_caught() {
    // Same signed manifest, but the middle record has been taken out.
    let p: EvidencePack =
        serde_json::from_str(&sample("sample-evidence-pack-removed-record.json")).unwrap();
    let v = verify_pack(&p);
    assert!(!v.valid);
    let failed = |name: &str| {
        v.checks
            .iter()
            .any(|c| c.name == name && c.ok == Some(false))
    };
    let passed = |name: &str| {
        v.checks
            .iter()
            .any(|c| c.name == name && c.ok == Some(true))
    };
    assert!(failed("manifest_matches_proofs"), "{:?}", v.checks);
    assert!(failed("hash_chain"), "{:?}", v.checks);
    // The signed manifest itself was not touched, which is why the removal shows.
    assert!(passed("manifest_signature"), "{:?}", v.checks);
}

#[test]
fn sample_reports_its_time_evidence_state() {
    let b: ProofBundle = serde_json::from_str(&sample("sample-proof.json")).unwrap();
    let t = verify_bundle(&b).time_evidence.unwrap();
    // The sample was made with Bitcoin timestamps off: batch signature only.
    assert_eq!(t.state, "batch_only");
}

fn checkpoint_report(pack_file: &str) -> hsip_proof::CheckpointReport {
    hsip_proof::check_checkpoints_text(&sample(pack_file), &sample("sample-checkpoints.json"), "")
        .unwrap()
}

#[test]
fn checkpoint_samples_pin_the_checkpoint_format_and_the_three_outcomes() {
    // The honest pack is valid and matches both checkpoints the customer kept.
    let honest: EvidencePack =
        serde_json::from_str(&sample("sample-checkpoint-pack.json")).unwrap();
    assert!(verify_pack(&honest).valid);
    let r = checkpoint_report("sample-checkpoint-pack.json");
    assert!(r.consistent && r.checked == 2, "{r:?}");
    // A rewritten history is valid on its own, and still caught.
    let rewritten: EvidencePack =
        serde_json::from_str(&sample("sample-checkpoint-pack-rewritten.json")).unwrap();
    assert!(verify_pack(&rewritten).valid);
    let r = checkpoint_report("sample-checkpoint-pack-rewritten.json");
    assert!(!r.consistent);
    assert!(r.results.iter().any(|x| x.state == "different_history"));
    // An export with the newest records cut off.
    let r = checkpoint_report("sample-checkpoint-pack-truncated.json");
    assert!(!r.consistent);
    assert!(r.results.iter().any(|x| x.state == "tail_missing"));
}
