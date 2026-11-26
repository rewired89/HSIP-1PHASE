//! Writes the synthetic checkpoint samples in docs/examples/prove. The key is a
//! public, fixed test key: these files demonstrate the check, they prove nothing.
//!   cargo run -p hsip-proof --example gen_checkpoint_samples

use std::collections::BTreeMap;

use ed25519_dalek::SigningKey;
use hsip_proof::*;

const T0: i64 = 1_800_000_000_000;

fn history(n: u64, tag: &str, key: &SigningKey) -> Vec<ProofBundle> {
    let mut out: Vec<ProofBundle> = Vec::new();
    for seq in 1..=n {
        let prev = out.last().map(|b| b.event_hash.clone()).unwrap_or_default();
        let env = Envelope {
            format: ENVELOPE_FORMAT.into(),
            proof_id: format!("00000000-0000-4000-8000-{seq:012}"),
            org_id: "sample-org".into(),
            stream_id: "sample-stream".into(),
            seq: seq.to_string(),
            actor: "underwriting-model-v3".into(),
            action: "loan.decision".into(),
            subject: format!("application-{tag}-{seq}"),
            payload_hash: sha256_hex(format!("{tag} record {seq}").as_bytes()),
            metadata: BTreeMap::new(),
            prev_hash: prev,
            timestamp_iso: format!("2027-01-15T08:00:{seq:02}.000Z"),
            timestamp_ms: (T0 + seq as i64 * 1000).to_string(),
        };
        let (event_hash, signature) = sign_envelope(&env, key).unwrap();
        out.push(ProofBundle {
            format: BUNDLE_FORMAT.into(),
            envelope: env,
            event_hash,
            signature,
            sign_algo: "ED25519".into(),
            org_verify_key: verify_key_b64(key),
            org_name: "Acme Lending (sample)".into(),
            anchor: None,
        });
    }
    out
}

fn pack(proofs: Vec<ProofBundle>, key: &SigningKey, to_ms: i64) -> EvidencePack {
    let manifest = PackManifest {
        org_id: "sample-org".into(),
        stream_id: "sample-stream".into(),
        from_ms: 0,
        to_ms,
        event_hashes: proofs.iter().map(|b| b.event_hash.clone()).collect(),
        issuer_keys: vec![],
    };
    let (manifest_hash, manifest_signature) = sign_manifest(&manifest, key).unwrap();
    EvidencePack {
        format: PACK_FORMAT.into(),
        org_name: "Acme Lending (sample)".into(),
        org_verify_key: verify_key_b64(key),
        stream_name: "Loan decisions".into(),
        generated_at_iso: "2027-01-15T09:00:00.000Z".into(),
        manifest,
        manifest_hash,
        manifest_signature,
        proofs,
    }
}

fn checkpoint(key: &SigningKey, seq: u64, head: &str, at: i64, prev: &str) -> Checkpoint {
    sign_checkpoint(
        CheckpointBody {
            format: CHECKPOINT_FORMAT.into(),
            org_id: "sample-org".into(),
            stream_id: "sample-stream".into(),
            seq: seq.to_string(),
            head_hash: head.into(),
            timestamp_ms: at.to_string(),
            timestamp_iso: format!("2027-01-15T08:01:{:02}.000Z", at % 60),
            prev_checkpoint_hash: prev.into(),
            signer_key: verify_key_b64(key),
        },
        key,
    )
    .unwrap()
}

fn write(name: &str, v: &impl serde::Serialize) {
    let path = format!(
        "{}/../../docs/examples/prove/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::write(&path, serde_json::to_string_pretty(v).unwrap() + "\n").unwrap();
    println!("wrote {path}");
}

fn main() {
    let key = SigningKey::from_bytes(&[42u8; 32]);
    let honest = history(5, "a", &key);
    // Checkpoints the customer kept: the platform signed positions 2 and 5.
    let c2 = checkpoint(&key, 2, &honest[1].event_hash, T0 + 60_000, "");
    let c5 = checkpoint(
        &key,
        5,
        &honest[4].event_hash,
        T0 + 120_000,
        &c2.checkpoint_hash,
    );
    write(
        "sample-checkpoint-pack.json",
        &pack(honest.clone(), &key, T0 + 600_000),
    );
    write(
        "sample-checkpoints.json",
        &serde_json::json!({ "checkpoints": [c2, c5] }),
    );
    // The platform later presents a different history that is valid on its own.
    let forked = history(5, "b", &key);
    write(
        "sample-checkpoint-pack-rewritten.json",
        &pack(forked, &key, T0 + 600_000),
    );
    // Or an export that claims to reach past the checkpoints but is missing the newest records.
    write(
        "sample-checkpoint-pack-truncated.json",
        &pack(honest[..3].to_vec(), &key, T0 + 600_000),
    );
}
