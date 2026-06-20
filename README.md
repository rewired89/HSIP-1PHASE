# HSIP — Local Identity Server

**One binary. No cloud. No subscription. Cryptographic identity and tamper-proof audit trail for individuals, AI agents, and financial institutions.**

[![License: Proprietary](https://img.shields.io/badge/License-Commercial-red.svg)](LICENSE)
[![Build](https://github.com/rewired89/HSIP-1PHASE/actions/workflows/release.yml/badge.svg)](https://github.com/rewired89/HSIP-1PHASE/actions)

### [🌐 hsip.rewired89.github.io/HSIP-1PHASE](https://rewired89.github.io/HSIP-1PHASE/) — Landing page with one-click downloads

> Every key is yours. Every byte runs locally. No cloud. No subscription. Commercial use requires a license — contact [sanchezleal1989@gmail.com](mailto:sanchezleal1989@gmail.com). [Read the threat model →](THREAT_MODEL.md)

## Quick install

**Windows** — [Download hsip-windows-x64.exe](https://github.com/rewired89/HSIP-1PHASE/releases/latest/download/hsip-windows-x64.exe) → double-click → browser opens automatically.

**macOS / Linux** — one command:
```bash
curl -sSf https://raw.githubusercontent.com/rewired89/HSIP-1PHASE/main/install.sh | sh
```

**Homebrew:**
```bash
brew tap rewired89/hsip https://github.com/rewired89/HSIP-1PHASE && brew install hsip
```

---

## Why this exists — right now

In 2026, three things happened at once:

- **AI agents act on your behalf** without a reliable record of what they did or who authorized it.
- **OpenAI, Google, and Meta serve ads** inside the tools you use to think. Your prompts train their models.
- **Deepfakes made digital evidence meaningless** — unless it carries a cryptographic signature that cannot be faked.

HSIP is the answer to all three. It runs on your hardware, signs everything with your key, and gives you a tamper-proof audit trail you own completely.

---

## Who is this for?

| I want to... | What to run |
|---|---|
| **Stop being tracked** — block ads, telemetry, and surveillance across every app I use | [DNS Tracker Blocker](#1-dns-tracker-blocker--block-everything-system-wide) |
| **Prove what I said** — create court-admissible proof that I wrote this message at this time | [Signed Messages + Audit Trail](#2-signed-messages--fight-deepfakes-and-win-disputes) |
| **Control my AI agents** — see exactly what my AI did, revoke access instantly | [AI Watch + Consent Wallet](#3-ai-watch--know-exactly-what-your-ai-did) |
| **Build privacy-respecting software** — add consent infrastructure to my app or AI agent | [Developer SDK →](#for-developers) |
| **Enterprise audit compliance** — GDPR, court records, legal-grade evidence chains | [Enterprise deployment →](#for-enterprises) |
| **Financial services infrastructure** — MiFID II, FINRA 4511, SOX §404, DORA, SWIFT CSCF compliance | [Financial Services →](#financial-services) |

---

## Download

| Platform | File |
|----------|------|
| **Windows** | [`hsip-windows-x64.exe`](https://github.com/rewired89/HSIP-1PHASE/releases/tag/latest) |
| macOS Apple Silicon | [`hsip-macos-arm64`](https://github.com/rewired89/HSIP-1PHASE/releases/tag/latest) |
| macOS Intel | [`hsip-macos-x64`](https://github.com/rewired89/HSIP-1PHASE/releases/tag/latest) |
| Linux | [`hsip-linux-x64`](https://github.com/rewired89/HSIP-1PHASE/releases/tag/latest) |

> **Windows:** Double-click the `.exe`. It installs itself, creates a Desktop shortcut, and opens in your browser automatically.
>
> **Mac / Linux:** `chmod +x hsip-macos-arm64 && ./hsip-macos-arm64` — your browser opens automatically.

---

## Features

### 1. DNS Tracker Blocker — block everything, system-wide

HSIP intercepts tracking requests at the DNS level before they ever reach your machine. Not just one browser — every app you run.

Blocks Google Analytics, Facebook Pixel, Hotjar, TikTok, DoubleClick, Microsoft telemetry, and 200+ more. One click in the dashboard to turn on. Zero configuration.

**The difference from browser extensions:** A browser extension only protects one browser. HSIP blocks at the network level — desktop apps, background processes, every browser, all at once.

---

### 2. Signed Messages — fight deepfakes and win disputes

Every message you send through HSIP is signed with your personal Ed25519 key. The result is mathematical proof that:

- You wrote exactly these words
- At exactly this timestamp
- That no one has altered since

This proof can be verified by anyone, in court, or by a machine. It cannot be faked.

**Real use cases:**
- *Contract confirmation:* "I confirm we agreed to these terms on March 28, 2026." — signed, timestamped, verifiable.
- *Dispute evidence:* Produce a cryptographic receipt in seconds that proves what you said and when.
- *Deepfake defense:* When someone claims you said something you didn't — your signed history proves otherwise.
- *AI command authorization:* Every instruction you gave your AI agent is signed with your key. Deniability is gone — in both directions.

---

### 3. AI Watch — know exactly what your AI did

Every AI agent you connect (Claude, ChatGPT, Siri, any HTTP-capable tool) is tracked in real time:

- **Velocity monitoring** — alerts if an agent makes an unusual number of requests
- **Anomaly detection** — flags behavior outside normal patterns
- **One-click disconnect** — revoke any agent's access instantly
- **Full signed audit trail** — every action the agent took, signed and timestamped

This is the "black box recorder" for your AI. When something goes wrong, you know exactly what happened and when.

---

### 4. Consent Wallet — machine-readable access control

Instead of cookie banners you click through without reading, HSIP creates a consent layer you actually control:

- See every party that has permission to contact you or access your data
- See exactly what each party is allowed to do
- Set time limits on consent — it expires automatically
- Revoke any consent in one click, effective immediately

Third-party services that support HSIP can query your consent before acting. No permission — no access.

---

### 5. Tamper-proof Audit Log

Every operation in HSIP — message signed, consent granted, key created, AI action logged — writes to a BLAKE3 hash-chained audit log. Tamper with any entry and the chain breaks.

Export the log at any time for legal proceedings, compliance audits, or personal records.

---

## Financial Services

HSIP is cryptographic infrastructure for banks, trading desks, fintechs, and any regulated institution that needs a tamper-proof audit trail, AI agent governance, and cross-institution identity verification — without a central cloud vendor in the middle.

**The client is the institution, not the retail investor.** HSIP runs inside your data center (or on-premise), signs every action with your Ed25519 keypair, and produces legally defensible evidence that your systems, analysts, and AI agents did exactly what the audit trail says they did.

---

### Why financial institutions need this now

**1. AI agents act on behalf of your institution** — and regulators are going to ask who authorized each action. Without a cryptographic identity attached to each agent and an append-only log of every request, you cannot answer that question. HSIP assigns every AI agent its own Ed25519 keypair, logs every action it takes, and lets you revoke its access in milliseconds.

**2. MiFID II Article 25 and FINRA Rule 4511 require you to prove** what your systems did, when, and on whose authority. A log in a database is not proof — it can be altered. A BLAKE3 hash-chained audit log is proof. Tamper with any entry and the chain breaks, detectable by any party.

**3. Open Banking (PSD2) mandates machine-readable, time-bounded consent.** HSIP's Consent Wallet generates exactly that: a cryptographically signed grant scoped to a specific action, automatically expiring, revocable in real time. No more cookie banners your compliance team can't evidence.

**4. Inter-institution trust is broken.** When a message arrives from a counterparty, how do you verify it wasn't altered in transit? HSIP's Federated Trust layer lets institutions exchange Ed25519 verify keys out-of-band (email, secure channel) and then verify any future message cryptographically — no central registry, no PKI vendor, no single point of failure.

**5. DORA and SWIFT CSCF require you to detect and respond to anomalous AI or automated system behavior.** HSIP's velocity monitoring flags agents exceeding 100 requests/minute and auto-revokes access at 1,000 requests/minute — with a signed audit entry at every step.

---

### Compliance coverage

| Regulation | What HSIP covers |
|---|---|
| **SOX §404** | Append-only BLAKE3 hash-chained audit log. Every control action signed with Ed25519. Exportable for auditors. |
| **FINRA Rule 4511** | Six-year tamper-evident record retention. API endpoint for bulk audit export. Signature chain proves no entry was altered. |
| **MiFID II Art. 25** | Per-trade authorization signed with institutional Ed25519 key. Timestamp + signature = defensible suitability record. |
| **PSD2 / Open Banking** | Machine-readable consent grants with scope, expiry, and revocation. `POST /v1/consent/grant` with `expires_in_seconds`. |
| **GDPR Art. 7** | Cryptographically signed consent with documented scope. `DELETE /v1/tenant/erase` for right-to-erasure. Audit log proves consent was active at time of processing. |
| **DORA** | AI agent velocity monitoring, anomaly detection, auto-revocation. Incident response via `DELETE /v1/keys/:id`. All events in signed audit trail. |
| **SWIFT CSCF** | Ed25519 message authentication prevents unauthorized instruction injection. Federated trust keys verified per counterparty. No shared secrets. |
| **ISO 20022** | Signed payment messages with Ed25519. Verifiable by any counterparty holding the institution's public key. Non-repudiation by construction. |

---

### AI agent governance for financial institutions

Every AI system your institution deploys — trading algorithms, document processors, customer-facing chatbots, internal assistants — gets its own Ed25519 keypair registered in HSIP.

```bash
# Register a trading algorithm as a governed AI agent
hsip agent register "algo-trading-v2" --expires-days 90

# List all active agents and their request velocity
hsip agent list

# Immediately revoke an agent that's behaving unexpectedly
hsip agent revoke "algo-trading-v2"
```

What you get for each agent:
- **Unique Ed25519 keypair** — every action it signs is traceable to that specific agent, not just "the system"
- **Velocity monitoring** — requests > 100/min trigger an anomaly audit entry; > 1,000/min triggers automatic revocation
- **Full signed audit trail** — every API call the agent made, timestamped and chained
- **Instant revocation** — `DELETE /v1/keys/:id` takes effect in memory before the DB write completes; in-flight requests are blocked immediately via `pending_revocation` set

This is the "black box recorder" regulators and your own risk team need when an AI agent does something unexpected.

---

### Federated trust — cross-institution Ed25519 verification

When your trading desk needs to verify that a message from a counterparty bank is authentic, you have two options: trust a central certificate authority (single point of failure, vendor lock-in) or exchange Ed25519 verify keys directly and verify locally.

HSIP implements the second approach:

```bash
# Your counterparty sends you their Ed25519 verify key out-of-band
hsip trust add "Deutsche Bank Desk A" "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"

# Verify any message they send you — locally, no network call
hsip trust verify --from "Deutsche Bank Desk A" \
  "Trade confirmation: AAPL 1000 @ 182.50" \
  "signature_hex_here"
```

No central registry. No PKI vendor. No single point of failure. Each institution holds the other's public key directly. Verification happens in `<1ms` locally.

API:
```
POST   /v1/trust/peer          Add a trusted counterparty's verify key
GET    /v1/trust/peers         List all trusted counterparties
DELETE /v1/trust/peers/:id     Remove a counterparty
POST   /v1/trust/verify        Verify a signed message from a named counterparty
```

---

### Financial services API examples

```bash
export KEY="hsip_your_institutional_key_here"
export BASE="http://127.0.0.1:7474"

# Sign a trade authorization — creates non-repudiable, timestamped proof
curl -X POST $BASE/v1/messages/sign \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{"content": "AUTHORIZED: Sell 500 TSLA @ market. Analyst: J.Smith. 2026-06-20T14:32:00Z"}'

# Grant time-bounded PSD2 consent to a payment processor
curl -X POST $BASE/v1/consent/grant \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{"peer_verify_key": "counterparty_pubkey_hex", "scope": "payment_initiation", "expires_in_seconds": 3600}'

# Export full audit trail for regulators (last 500 entries)
curl "$BASE/v1/audit?limit=500" \
  -H "Authorization: Bearer $KEY"

# GDPR right-to-erasure (Article 17)
curl -X DELETE $BASE/v1/tenant/erase \
  -H "Authorization: Bearer $KEY"
```

---

## Cryptography — what's under the hood

HSIP uses audited [RustCrypto](https://github.com/RustCrypto) libraries throughout. **No custom cryptography.** Every primitive is a published standard, independently audited, and used by systems you already trust.

| What | Algorithm | Standard | Why |
|------|-----------|----------|-----|
| Identity & signatures | Ed25519 | RFC 8032 | Used by Signal, Tor, SSH, TLS 1.3, OpenSSH, and most modern HSMs. 128-bit security level. Deterministic — no randomness failure mode. |
| Key encryption at rest | ChaCha20-Poly1305 | RFC 8439 | Constant-time implementation. No timing side-channels. Used in TLS 1.3, WireGuard, Signal. AEAD — encryption and authentication in one operation. |
| Key derivation | HKDF-SHA-256 | RFC 5869 | Derives encryption keys from the master key. Standard, audited, used in TLS 1.3 and Signal Protocol. |
| Audit chain integrity | BLAKE3 | — | Each audit entry includes the hash of the previous entry. Tamper with any entry and every subsequent hash breaks. 3× faster than SHA-256. |
| Session key exchange | X25519 ephemeral | RFC 7748 | Elliptic-curve Diffie-Hellman on Curve25519. New session key per connection = perfect forward secrecy. Past sessions cannot be decrypted if long-term keys are compromised. |
| **Post-quantum identity** | **ML-DSA-65 (Dilithium)** | **NIST FIPS 204** | **"Harvest now, decrypt later" resistant. A quantum computer cannot forge signatures even with the public key.** |
| **Post-quantum key exchange** | **ML-KEM-768 (Kyber)** | **NIST FIPS 203** | **Encapsulation mechanism secure against Shor's algorithm. Enable for long-lived key material that must survive 2030+.** |

### Why these choices matter for financial institutions

**Ed25519 vs RSA-2048:** RSA requires randomness — a flawed RNG produces a forgeable signature. Ed25519 is deterministic: same message + same key = same signature, always. No randomness failure mode. Hardware security modules (HSMs) used in banking already support Ed25519 natively (PKCS#11, AWS CloudHSM, Azure Dedicated HSM).

**ChaCha20-Poly1305 vs AES-GCM:** AES-GCM is vulnerable to nonce reuse. ChaCha20-Poly1305 degrades gracefully. More importantly, ChaCha20 has no timing side-channel — AES on CPUs without hardware acceleration leaks key material through cache timing. HSIP uses constant-time implementations throughout.

**BLAKE3 audit chain vs append-only database:** A database marked "append-only" can still be altered by a DBA or compromised backup. A BLAKE3 hash chain cannot: each entry's hash covers its own content plus the previous entry's hash. Alteration of any entry produces a hash mismatch detectable by any party holding the chain. This is the same construction used in blockchain systems, without the distributed consensus overhead.

**Post-quantum timeline:** NIST finalized ML-KEM and ML-DSA in 2024. The NSA's CNSA 2.0 suite requires post-quantum algorithms for TOP SECRET material by 2030 and recommends migration now. HSIP builds in both algorithms today, disabled by default, enabled with one config flag — so institutions can begin PQ migration on their own timeline without a software upgrade.

### Key storage architecture

```
Master key → never touches disk
    ↓ HKDF-SHA-256 derivation
Wrapping key
    ↓ ChaCha20-Poly1305 encryption
Encrypted Ed25519 private key → stored in SQLite
```

The master key lives only in memory (or at a configured path with filesystem permissions). Compromise of the database file does not expose private keys — an attacker also needs the master key. API keys are stored as SHA-256 hashes only; the raw key is shown once at creation and never stored.

### Formal verification

HSIP includes an optional Z3 SMT solver module (`crates/hsip-verify`) for machine-checked security proofs. Not just tests — mathematical guarantees that specific security properties hold. Build separately (requires Z3 system library):

```bash
cargo build -p hsip-verify
```

Post-quantum support is built in today, not a future promise. Enable it with a config flag when you need it.

---

## For Developers

HSIP exposes a REST API at `http://127.0.0.1:7474`. SDKs available for Python, Node.js, and Go.

```bash
export KEY="hsip_your_key_here"

# Sign a message — creates a cryptographic, timestamped proof
curl -X POST http://127.0.0.1:7474/v1/messages/sign \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{"content": "I authorize this transaction."}'

# Get AI agent capability spec — inject into any AI system prompt
curl http://127.0.0.1:7474/v1/agent/capabilities \
  -H "Authorization: Bearer $KEY"

# Grant time-bounded consent to a peer
curl -X POST http://127.0.0.1:7474/v1/consent/grant \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{"peer_verify_key": "...", "scope": "contact", "expires_in_seconds": 86400}'

# Enable DNS tracker blocker
curl -X POST http://127.0.0.1:7474/v1/dns/enable \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{"port": 5300}'
```

Full interactive API docs at `http://127.0.0.1:7474/docs` when HSIP is running (OpenAPI 3.0).

### Python SDK

```python
from hsip import HSIPClient

client = HSIPClient(api_key="hsip_...", base_url="http://localhost:7474")
identity = client.get_or_create_identity()
signed = client.sign_message("I authorized this action.")
client.grant_consent(peer_verify_key="...", scope="contact")
```

### Connecting an AI agent

Point any AI at the capabilities endpoint and it knows exactly what HSIP can do:

```
GET http://127.0.0.1:7474/v1/agent/capabilities
Authorization: Bearer hsip_...
```

Returns a machine-readable spec. Paste it into any AI system prompt. The AI can then send signed messages, check consent, and log actions — all under your authorization.

---

## For Enterprises

HSIP supports PostgreSQL, multi-tenancy, and Kubernetes out of the box.

```bash
# Single-machine setup
docker compose up

# Production HA with PostgreSQL
# See DEPLOYMENT.md for full Helm chart + TLS + backup configuration
```

**Compliance built in:**
- **SOX / FINRA 4511** — Append-only BLAKE3 hash-chained audit log. Every entry signed with Ed25519. Bulk export via `GET /v1/audit`. No vendor can alter your records.
- **MiFID II Art. 25** — Per-action Ed25519 signature proves authorization, identity, and timestamp for every trade, instruction, or consent action.
- **PSD2 Open Banking** — Machine-readable consent grants: scoped, time-bounded, revocable. `POST /v1/consent/grant` with `expires_in_seconds`.
- **GDPR Art. 17** — Right-to-erasure endpoint: `DELETE /v1/tenant/erase`. Signed consent records prove lawful basis at time of processing.
- **DORA** — AI agent velocity monitoring, anomaly detection, auto-revocation at configurable thresholds. All events written to the signed audit trail.
- **SWIFT CSCF** — Ed25519 message authentication. No shared secrets between counterparties. Federated trust key exchange prevents instruction injection.
- **No telemetry, no phone-home, no licensing server** — your keys and your audit trail never leave your infrastructure.

**Deployment architecture:**
- Single binary for on-premise or private cloud
- PostgreSQL for production HA (`DATABASE_URL` env var)
- Multi-tenancy: isolated keypairs, audit logs, and API keys per tenant
- Kubernetes: Helm chart in `DEPLOYMENT.md` with TLS termination and secret management
- Air-gapped deployment supported — no outbound network required

See [DEPLOYMENT.md](DEPLOYMENT.md) for production setup, TLS, PostgreSQL, and disaster recovery.

---

## How to connect your AI assistant

After opening HSIP, go to **AI Watch → Connect an AI**. Give the connection a name and copy the key that appears.

**Siri (iPhone / Mac)**
The setup guide walks you through creating a Siri Shortcut in 4 steps. Once done, say *"Hey Siri, Send HSIP Message"* — Siri asks what you want to say, signs it with your key, and stores it with a timestamp.

**Claude Desktop**
Copy the pre-written system prompt from the setup guide and paste it into any Claude conversation. Claude will call HSIP when you ask it to record or verify a message.

**Any AI with HTTP support**
Query `/v1/agent/capabilities` with your Bearer key. The response is a complete machine-readable description of every HSIP capability. Inject it into your AI's system prompt.

---

## Build from source

```bash
# 1. Build the dashboard
cd dashboard && npm install && npm run build && cd ..

# 2. Build the binary with embedded dashboard
cargo build --release -p hsip-api --features hsip-api/embed-dashboard

# 3. Run
./target/release/hsip-api
# Browser opens automatically at http://127.0.0.1:7474
```

Development mode (dashboard hot-reloads):

```bash
cargo run -p hsip-api          # API on :7474
cd dashboard && npm run dev    # UI on :5173 with hot reload
```

Run the full test suite (238 tests):

```bash
cargo test --workspace
```

---

## Architecture

```
┌────────────────────────────────────────────────────────────┐
│  hsip-api       Rust / Axum / Tokio — REST API + auth      │
│  hsip-core      Ed25519, X25519, ChaCha20-Poly1305,        │
│                 ML-KEM-768, ML-DSA-65, HKDF-SHA-256        │
│  hsip-dns       UDP :5300 — DNS tracker blocker            │
│  hsip-session   Ephemeral sessions, X25519 forward secrecy │
│  hsip-auth      Identity and authentication primitives     │
│  hsip-telemetry-guard  Telemetry + anomaly detection       │
│  hsip-mcp       MCP server — AI agent integration          │
│  hsip-cli       hsip agent / trust / up CLI                │
│  SQLite / PostgreSQL  Local or HA storage                  │
│  React          Embedded dashboard — single binary         │
└────────────────────────────────────────────────────────────┘
```

Everything runs in a single binary for desktop/on-premise use. Switch to PostgreSQL and multi-tenancy for production financial deployments with no code changes — just a `config.toml`.

16 specialized crates. 238 tests. RFC 8032 (Ed25519) + RFC 8439 (ChaCha20-Poly1305) + RFC 5869 (HKDF) + RFC 7748 (X25519) compliance verified. NIST FIPS 203 + 204 post-quantum algorithms built in. Audited RustCrypto primitives throughout — no custom cryptography.

---

## Security

- **Private keys encrypted at rest** — ChaCha20-Poly1305 + HKDF-SHA-256. Master key never touches disk. Compromise of the database file does not expose private keys.
- **API keys stored as SHA-256 hashes only** — raw key shown once at creation, never stored. Compromise of the database does not expose API credentials.
- **Rate limiting on all endpoints** — 300 req/min default per key, configurable via `RATE_LIMIT_RPM`.
- **AI agent velocity monitoring** — anomaly logged at >100 req/min; key auto-revoked at >1,000 req/min with immediate in-memory block before DB write.
- **Append-only BLAKE3 hash-chained audit trail** — each entry covers the previous entry's hash. Tamper with any entry and the chain breaks, detectable by any verifier.
- **Replay attack prevention** — monotonic nonce counters. Replayed requests are rejected even if the signature is valid.
- **Instant revocation** — `pending_revocation` DashSet blocks in-flight requests in memory before the async DB write completes. No race window.
- **No telemetry, no analytics, no phone-home** — ever. Verified by code review: no outbound connections except DNS forwarding (1.1.1.1:53) when the DNS blocker is enabled.
- **Formal verification available** — optional Z3 SMT solver module (`hsip-verify`) provides machine-checked proofs of security properties, not just tests.

See [THREAT_MODEL.md](THREAT_MODEL.md) for a full breakdown of what HSIP protects against and what it does not.

To report a vulnerability: **sanchezleal1989@gmail.com**

---

## License

© 2025–2026 Dayana Sanchez. All rights reserved.

HSIP is **proprietary software**. Source code is available for review.

- **Personal and evaluation use** — free. Run it, read the code, evaluate it.
- **Commercial use** — requires a paid license. This includes production deployments, business use, integrations, SaaS products built on HSIP, and any use inside an organization.

**To license HSIP for commercial or institutional use:** [sanchezleal1989@gmail.com](mailto:sanchezleal1989@gmail.com)

See [LICENSE](LICENSE) for full terms.

---

**Your data. Your keys. Your machine.**
