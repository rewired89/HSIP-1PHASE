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
