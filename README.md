# HSIP — Local Identity Server

**One binary. No cloud. No subscription. Cryptographic identity and tamper-proof audit trail for individuals, AI agents, and financial institutions.**

[![License: Proprietary](https://img.shields.io/badge/License-Commercial-red.svg)](LICENSE)
[![Build](https://github.com/rewired89/HSIP-1PHASE/actions/workflows/release.yml/badge.svg)](https://github.com/rewired89/HSIP-1PHASE/actions)
[![Security Audit](https://github.com/rewired89/HSIP-1PHASE/actions/workflows/security-audit.yml/badge.svg)](https://github.com/rewired89/HSIP-1PHASE/actions/workflows/security-audit.yml)

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
| **Financial services infrastructure** — audit-trail and key-management patterns for regulated environments | [Financial Services →](#financial-services) |

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

Every operation in HSIP — message signed, consent granted, key created, AI action logged — writes to an append-only audit log. There is no delete or update endpoint for audit records.

Export the log at any time for legal proceedings, compliance audits, or personal records.

---

## Financial Services

HSIP is cryptographic infrastructure for banks, trading desks, fintechs, and any regulated institution that needs a tamper-proof audit trail, AI agent governance, and cross-institution identity verification — without a central cloud vendor in the middle.

**The client is the institution, not the retail investor.** HSIP runs inside your data center (or on-premise), signs every action with your Ed25519 keypair, and produces legally defensible evidence that your systems, analysts, and AI agents did exactly what the audit trail says they did.

---

### Why financial institutions need this now

**1. AI agents act on behalf of your institution** — and regulators are going to ask who authorized each action. Without a cryptographic identity attached to each agent and an append-only log of every request, you cannot answer that question. HSIP assigns every AI agent its own Ed25519 keypair, logs every action it takes, and lets you revoke its access in milliseconds.

**2. Regulated environments require you to prove** what your systems did, when, and on whose authority. HSIP writes every action to an append-only audit log — no delete or update endpoints exist. Every entry is signed with the tenant's Ed25519 key, so the authorship of each record is cryptographically verifiable.

**3. Time-bounded consent is a design requirement in many financial workflows.** HSIP's Consent Wallet generates exactly that: a cryptographically signed grant scoped to a specific action, automatically expiring, revocable in real time.

**4. Inter-institution trust is broken.** When a message arrives from a counterparty, how do you verify it wasn't altered in transit? HSIP's Federated Trust layer lets institutions exchange Ed25519 verify keys out-of-band (email, secure channel) and then verify any future message cryptographically — no central registry, no PKI vendor, no single point of failure.

**5. Anomalous AI and automated system behavior requires detection and response.** HSIP's velocity monitoring flags agents exceeding 100 requests/minute and auto-revokes access at 1,000 requests/minute — with a signed audit entry at every step.

---

### What HSIP provides for regulated environments

HSIP is not certified against any regulatory framework. It provides cryptographic infrastructure — the building blocks that compliance architectures are built on.

| Capability | What it does |
|---|---|
| **Append-only, hash-chained audit log** | No delete or update endpoint exists. Every entry extends a per-tenant BLAKE3 hash chain (`GET /v1/audit/verify` recomputes and checks it), so altering or deleting a row breaks every link after it. Exportable for auditors. |
| **Ed25519-signed actions** | Every control action signed with the institution's key. Timestamp + signature = non-repudiable record. |
| **Time-bounded consent grants** | Machine-readable grants with scope, expiry, and revocation via `POST /v1/consent/grant`. |
| **Right-to-erasure** | `DELETE /v1/tenant/erase` permanently removes all tenant data and logs the erasure event. |
| **AI agent identity and revocation** | Every agent gets a unique keypair. Velocity monitoring, anomaly detection, auto-revocation. |
| **Inter-institution message verification** | Ed25519 verify keys exchanged out-of-band. Messages verified locally, no central authority. |
| **Non-repudiation** | Signed messages with public verify key. Any counterparty can verify without calling your server. |

Whether these capabilities satisfy a specific regulatory requirement in your jurisdiction is a legal question, not a product claim. Talk to your compliance team and legal counsel before deploying HSIP in a regulated context. Contact [sanchezleal1989@gmail.com](mailto:sanchezleal1989@gmail.com) to discuss your architecture.

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

### Master key rotation and backups

The master key encrypts every tenant's private signing key at rest. It's generated automatically on first boot — back it up, and rotate it on whatever schedule your compliance policy requires:

```bash
# Confirm your backup file actually matches what's running — no key exposure, no rotation
hsip keys master-fingerprint

# Rotate on a schedule: re-encrypts every identity, swaps live, no restart, no downtime
hsip keys rotate-master
#   → asks for confirmation before doing anything; pass --yes for scripted/scheduled runs
```

Admin key only. Requires the master key to be file-backed (the default) rather than sourced from `HSIP_MASTER_KEY` — if you're pointing that at your own secrets manager, rotate it there instead, then restart.

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
| Audit log integrity | BLAKE3 hash chain | — | Every `audit_entries` row links to the previous one via `entry_hash = BLAKE3(prev_hash \|\| ...)`. `GET /v1/audit/verify` recomputes the chain server-side; altering or deleting a row breaks every link after it. Append-only by policy (no delete/update endpoint) *and* now tamper-evident by hash chain. |
| Session key exchange | X25519 ephemeral | RFC 7748 | Elliptic-curve Diffie-Hellman on Curve25519. New session key per connection = perfect forward secrecy. Past sessions cannot be decrypted if long-term keys are compromised. |
| **Post-quantum identity** † | **ML-DSA-65 (Dilithium)** | **NIST FIPS 204** | **"Harvest now, decrypt later" resistant. A quantum computer cannot forge signatures even with the public key.** |
| **Post-quantum key exchange** † | **ML-KEM-768 (Kyber)** | **NIST FIPS 203** | **Encapsulation mechanism secure against Shor's algorithm. Enable for long-lived key material that must survive 2030+.** |

† Lives in the `hsip-verify` crate, which is **excluded from the default workspace build** (it requires the Z3 SMT solver built from source — see `cargo build -p hsip-verify` in Build Commands). Everything else in this table ships in the default `hsip-api` binary; post-quantum support does not, yet.

### Why these choices matter for financial institutions

**Ed25519 vs RSA-2048:** RSA requires randomness — a flawed RNG produces a forgeable signature. Ed25519 is deterministic: same message + same key = same signature, always. No randomness failure mode. Hardware security modules (HSMs) used in banking already support Ed25519 natively (PKCS#11, AWS CloudHSM, Azure Dedicated HSM).

**ChaCha20-Poly1305 vs AES-GCM:** AES-GCM is vulnerable to nonce reuse. ChaCha20-Poly1305 degrades gracefully. More importantly, ChaCha20 has no timing side-channel — AES on CPUs without hardware acceleration leaks key material through cache timing. HSIP uses constant-time implementations throughout.

**Append-only, hash-chained audit log:** The HTTP API has no delete or update endpoint for audit records — omission or alteration requires direct database access, which is an OS-level compromise. Every audit entry now also links to the previous one via a BLAKE3 hash chain (`entry_hash = BLAKE3(prev_hash || id || tenant_id || action || peer_verify_key || details || timestamp)`), verifiable on demand via `GET /v1/audit/verify` — so even an attacker with direct database write access cannot alter or delete a row without breaking the chain for everything after it. This does not (yet) prevent deleting the *entire* chain undetected — see the anchoring note below and `THREAT_MODEL.md` §4.8 for the remaining scope.

**Post-quantum timeline:** NIST finalized ML-KEM and ML-DSA in 2024. The NSA's CNSA 2.0 suite requires post-quantum algorithms for TOP SECRET material by 2030 and recommends migration now. HSIP builds in both algorithms today, disabled by default, enabled with one config flag — so institutions can begin PQ migration on their own timeline without a software upgrade.

### Key storage architecture

```
Master key (file, or HSIP_MASTER_KEY from a secrets manager)
    ↓ HKDF-SHA-256 derivation
Wrapping key
    ↓ ChaCha20-Poly1305 encryption
Encrypted Ed25519 private key → stored in SQLite
```

By default the master key lives in a file with restrictive filesystem permissions (`~/.hsip/master.key`, or a configured path); set `HSIP_MASTER_KEY` to source it from a secrets manager instead and keep it off this machine's disk entirely. Either way, compromise of the database file alone does not expose private keys — an attacker also needs the master key. `POST /v1/admin/master-key/rotate` can replace it live without a restart. API keys are stored as SHA-256 hashes only; the raw key is shown once at creation and never stored.

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

**Built for regulated environments:**
- **Append-only, hash-chained audit log** — no delete or update endpoint. Every entry extends a BLAKE3 hash chain, verifiable via `GET /v1/audit/verify`. Bulk export via `GET /v1/audit`.
- **Non-repudiable signed actions** — Ed25519 signature proves authorization, identity, and timestamp for every instruction or consent action.
- **Time-bounded consent grants** — scoped, machine-readable, revocable. `POST /v1/consent/grant` with `expires_in_seconds`.
- **Right-to-erasure** — `DELETE /v1/tenant/erase` permanently removes all tenant data and writes a signed erasure event to the audit log.
- **AI agent governance** — velocity monitoring, anomaly detection, auto-revocation. All events in the signed audit trail.
- **Inter-institution message auth** — Ed25519 verify keys, no shared secrets. Federated trust key exchange, verified locally.
- **No telemetry, no phone-home, no licensing server** — your keys and your audit trail never leave your infrastructure.

> HSIP is not certified against any regulatory framework. Whether these capabilities satisfy a specific requirement in your jurisdiction is a legal and compliance question. Contact [sanchezleal1989@gmail.com](mailto:sanchezleal1989@gmail.com) to discuss your architecture.

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

16 specialized crates. 261 tests (`cargo test --workspace`; `hsip-verify` excluded, see above, has its own suite). RFC 8032 (Ed25519) + RFC 8439 (ChaCha20-Poly1305) + RFC 5869 (HKDF) + RFC 7748 (X25519) compliance verified. NIST FIPS 203 + 204 post-quantum algorithms available via `hsip-verify` (not part of the default build). Audited RustCrypto primitives throughout — no custom cryptography.

---

## Security

- **Private keys encrypted at rest** — ChaCha20-Poly1305 + HKDF-SHA-256. Compromise of the database file alone does not expose private keys — the master key is also required. By default the master key lives in a file on disk (`~/.hsip/master.key` or a configured path); point `HSIP_MASTER_KEY` at a secrets manager (Vault, AWS KMS, etc.) instead if you don't want that.
- **Master key rotation** — `hsip keys rotate-master` (or `POST /v1/admin/master-key/rotate` directly; bootstrap admin key only) re-encrypts every identity under a fresh key and swaps it live, no restart. Previously there was no rotation path at all. `hsip keys master-fingerprint` lets you verify a backup file matches the running key without exposing or rotating anything.
- **API keys stored as SHA-256 hashes only** — raw key shown once at creation, never stored. Compromise of the database does not expose API credentials.
- **Rate limiting on all endpoints** — 300 req/min default per key, configurable via `RATE_LIMIT_RPM`.
- **AI agent velocity monitoring** — anomaly logged at >100 req/min; key auto-revoked at >1,000 req/min with immediate in-memory block before DB write, and the DB write that makes revocation durable past a restart now retries with backoff and logs loudly if it still fails.
- **Append-only, hash-chained audit trail** — no delete or update endpoint exists. Every entry extends a BLAKE3 hash chain (`GET /v1/audit/verify` recomputes and checks it). Tamper requires OS-level DB access, not just API access, and is detectable if attempted.
- **Consent records who authorized it** — every grant/revoke stores which kind of key (human / service / ai_agent) did it, so "consent" can't quietly mean "an AI agent approved its own action" without that being visible.
- **Replay attack prevention on the UDP session/consent protocol** — monotonic nonce counters (`hsip-core`) reject zero, previously-seen, and stale nonces. **This does not currently cover the HTTP REST API** — a captured HTTP request with a valid API key can be replayed until the key expires or is revoked; the mitigations there are the rate limiter and key expiry, not per-request nonces. See `THREAT_MODEL.md` §4.3.
- **Instant revocation** — `pending_revocation` DashSet blocks in-flight requests in memory before the async DB write completes. No race window.
- **No telemetry, no analytics, no phone-home** — ever. Verified by code review: no outbound connections except DNS forwarding (1.1.1.1:53) when the DNS blocker is enabled.
- **`HSIP_SANDBOX=true` is loudly flagged** — it opens the one endpoint in the API requiring no bearer key at all (a rate-limited, capped, 24h trial provisioner meant for a public demo deployment). Enabling it logs an unmissable startup warning and increments a dedicated metric.
- **Formal verification available for the protocol layer, not yet part of CI** — optional Z3 SMT solver module (`hsip-verify`) provides machine-checked proofs of security properties, not just tests. Excluded from the default workspace build (`cargo build`/`cargo test --workspace` don't touch it) — build and test it explicitly with `cargo build -p hsip-verify`.

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
