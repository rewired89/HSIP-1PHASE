# HSIP: proof format

This is the wire format behind every proof, share link and evidence pack. It is deliberately small so that a third party can reimplement the verifier in an afternoon. The reference implementation is `crates/hsip-proof` (`hsip-proof verify <file>`); checked-in fixtures are in `docs/examples/prove/`, pinned by `crates/hsip-proof/tests/samples.rs`.

Primitives: SHA-256, Ed25519, RFC 8785 JSON Canonicalization (JCS), RFC 6962 Merkle trees. Nothing else.

## 1. Envelope (the signed statement)

```json
{
  "format": "hsip-proof/1",
  "proof_id": "uuid",
  "org_id": "uuid",
  "stream_id": "uuid",
  "seq": "2",
  "actor": "underwriting-model-v3",
  "action": "loan.decision",
  "subject": "application-1002",
  "payload_hash": "64 hex chars, SHA-256 of the customer's record",
  "metadata": { "region": "eu" },
  "prev_hash": "event_hash of seq-1, empty string for seq 1",
  "timestamp_iso": "2026-10-17T09:59:50.761Z",
  "timestamp_ms": "1792231190761"
}
```

`seq` and `timestamp_ms` are strings so no verifier loses precision on large integers. `metadata` is a flat string map.

## 2. Hashing and signature

```
event_hash = SHA256( JCS(envelope) )            // 32 bytes, hex in bundles
signature  = Ed25519_sign(org_key, event_hash)  // over the 32 raw bytes, base64
```

The organization's public key (`org_verify_key`, base64 of the 32-byte Ed25519 key) travels in the bundle. A verifier recomputes `event_hash` from the envelope, checks it equals the stated one, and checks the signature. Editing any envelope field changes the hash; re-hashing an edited envelope invalidates the signature.

## 3. Hash chain

Each stream is a chain: `envelope.prev_hash` of record *n* equals `event_hash` of record *n-1*, and `seq` increases by exactly one. A deleted record leaves a sequence gap; a reordered or substituted record breaks a link. The server enforces this with unique constraints on `(stream_id, seq)` and `(stream_id, prev_hash)`.

## 4. Batch anchor

Unanchored records from all organizations are batched into an RFC 6962 Merkle tree.

```
leaf_hash(d)    = SHA256(0x00 || d)           // d = the 32 raw bytes of event_hash
node_hash(l, r) = SHA256(0x01 || l || r)
root            = MTH(leaves)                  // RFC 6962 section 2.1
anchor_signature = Ed25519_sign(anchor_key, root)   // over the 32 raw root bytes
```

The bundle's `anchor` block carries `merkle_root`, `merkle_index`, `leaf_count`, the `inclusion_proof` (list of `{hash, position}` where `position` says which side the sibling sits on), `anchor_signature` and `anchor_verify_key`. Verification folds the leaf up the path and compares with the root, then checks the signature.

`ots_status` and `ots_proof` carry an optional OpenTimestamps commitment of `root` (`pending`, `confirmed`, `calendar_unreachable`, `disabled`). `hsip-proof` does not validate it; use the OpenTimestamps client on `ots_proof` for the Bitcoin-backed time guarantee.

## 5. Bundle

```json
{
  "format": "hsip-proof-bundle/1",
  "envelope": { },
  "event_hash": "hex",
  "signature": "base64",
  "sign_algo": "ED25519",
  "org_verify_key": "base64",
  "org_name": "Acme Lending",
  "anchor": null
}
```

`anchor` is `null` until the record has been batched (normally within a minute). Authorship and integrity are verifiable immediately; batch inclusion is reported as "not applicable yet", never as a failure.

## 6. Evidence pack

```json
{
  "format": "hsip-evidence-pack/1",
  "org_name": "", "org_verify_key": "", "stream_name": "", "generated_at_iso": "",
  "manifest": { "org_id": "", "stream_id": "", "from_ms": 0, "to_ms": 0, "event_hashes": ["..."], "issuer_keys": ["base64 keys, only when proofs were signed by more than the pack key"] },
  "manifest_hash": "hex",
  "manifest_signature": "base64",
  "proofs": [ "<bundle>", "..." ]
}
```

```
manifest_hash      = SHA256( JCS(manifest) )
manifest_signature = Ed25519_sign(org_key, manifest_hash)
```

A pack verifies when: the manifest hash matches, the manifest signature verifies, `manifest.event_hashes` equals the included proofs' hashes in order (so removing or adding a record fails), every bundle verifies, all bundles share `org_verify_key` or, when the manifest carries `issuer_keys` (organizations that switched to customer-held keys sign some records themselves), every bundle's key is listed there, and the records form a contiguous chain across the range.

## 7. What a valid result proves, and what it does not

Proves: each record existed in exactly this form, was signed by the holder of `org_verify_key`, and has not been changed, removed or reordered within the chain; and, once anchored, that it was part of a batch whose root the anchor key signed (and, with a confirmed OpenTimestamps proof, that the root existed before a given Bitcoin block).

Does not prove: that the decision was correct; that `org_verify_key` belongs to the organization you think (confirm the fingerprint through a second channel); that the customer recorded everything they should have; or the content behind `payload_hash` (that is why you keep the original and compare hashes).

## 8. Stability

`hsip-proof/1` is frozen once published: fields are never repurposed. New capabilities arrive as new optional fields or a new format string.
