# HSIP

**A notary for AI decisions.** HSIP gives companies tamper-evident, independently verifiable proof of what their automated systems decided and when.

Your systems send a SHA-256 fingerprint of each decision. HSIP signs it, chains it to the previous one, batches it into a Merkle tree and anchors the root. Anyone, including an auditor, a regulator or a court expert, can then verify the record offline with open code, without trusting us. The content of your decisions never reaches HSIP.

> A proof shows that a record existed, unaltered, at a given time. It does not show the decision was right.

## How it works

1. **Record.** `POST /v1/ingest` with the hash of a decision. Back comes a signed, chained proof.
2. **Anchor.** Records are batched about every minute into an RFC 6962 Merkle tree. The signed root can also be timestamped on Bitcoin through OpenTimestamps.
3. **Verify.** `hsip-proof verify proof.json` checks the signature, the hash chain and the Merkle path, offline.
4. **Export.** Evidence packs bundle a range of records under a signed manifest, with a printable report.

### Who holds the signing key

By default the platform holds each organization's key. An organization can switch to **customer-held keys**: you generate the key on your own machine, register only the public key, and sign every record yourself. A breach of the platform then cannot forge your records. See [Honest limits](docs/PROVE_PLATFORM.md) for what this does and does not cover.

## Quick start

```powershell
cargo run -p hsip-platform            # http://127.0.0.1:8080, console included once built
cd console; npm install; npm run build   # build the console first
```

Sign up in the console, create a stream and an API key, then:

```python
from hsip_prove import Prove
prove = Prove("http://127.0.0.1:8080", api_key)
prove.record({"application": "48211", "decision": "approve"},
             stream="loan-decisions", actor="underwriting-model-v3", action="loan.decision")
```

Verify independently:

```powershell
cargo install --path crates/hsip-proof
hsip-proof verify proof.json
```

For your own signing key: `hsip-proof keygen`, then register it in Settings, Key custody.

## What's in the repo

| Path | What it is |
|---|---|
| `crates/hsip-proof` | Open proof format and offline verifier (`hsip-proof`). RFC 6962 Merkle, RFC 8785 JCS, Ed25519. |
| `crates/hsip-platform` | The multi-tenant platform server: accounts, organizations, streams, API keys, anchoring, evidence packs, plans. SQLite or PostgreSQL. |
| `console/` | Business console and public pages (verify, shared proofs). |
| `sdk/prove-python`, `sdk/prove-node` | Dependency-free client SDKs, with customer-held key support. |
| `docs/` | Platform guide, proof format, audit notes, and the protocol documents below. |
| `crates/hsip-core` and related | The HSIP consent and secure-session protocol (Ed25519, X25519, ChaCha20-Poly1305). Separate from the platform. |

## Documentation

- [Platform guide, configuration and honest limits](docs/PROVE_PLATFORM.md)
- [Proof format specification](docs/PROVE_PROOF_FORMAT.md)
- [Security self-scan and audit guide](docs/PROVE_AUDIT.md)
- [Sample proof and evidence pack](docs/examples/prove/)
- [Consent protocol specification](docs/PROTOCOL_SPEC.md) and [architecture](docs/PHASE2_ARCHITECTURE.md)
- [Security policy](SECURITY.md)

## Development

```powershell
cargo test -p hsip-proof -p hsip-platform
cargo clippy -p hsip-proof -p hsip-platform --all-targets -- -D warnings
```

Scope Cargo commands with `-p`; the full workspace also builds unrelated crates. Contributions: see [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md).

## Security

Cryptography: Ed25519 signatures, SHA-256, RFC 6962 Merkle trees, ChaCha20-Poly1305 for keys at rest. Report vulnerabilities privately to nyxsystemsllc@gmail.com. No independent third-party audit has been completed yet.

## License

The platform (`crates/hsip-platform`, `console/`) is free under the GNU AGPL-3.0, or available under a paid commercial license if you want to build it into your own product without the AGPL's source-sharing terms. The open verifier, proof format and SDKs are Apache-2.0. The older protocol crates remain under the HSIP Community License (non-commercial). See [LICENSE-NOTICE.md](LICENSE-NOTICE.md) and [COMMERCIAL-LICENSE.md](COMMERCIAL-LICENSE.md). Commercial inquiries: nyxsystemsllc@gmail.com.
