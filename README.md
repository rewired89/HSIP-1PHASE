# HSIP — Local Identity Server

**One binary. No cloud. No subscription. Your data never leaves your machine.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Build](https://github.com/rewired89/HSIP-1PHASE/actions/workflows/release.yml/badge.svg)](https://github.com/rewired89/HSIP-1PHASE/actions)

### [🌐 hsip.rewired89.github.io/HSIP-1PHASE](https://rewired89.github.io/HSIP-1PHASE/) — Landing page with one-click downloads

> HSIP is open source because *closed-source privacy is an oxymoron.* Every key is yours. Every byte is auditable. [Read the threat model →](THREAT_MODEL.md)

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

## Cryptography — what's under the hood

HSIP uses audited [RustCrypto](https://github.com/RustCrypto) libraries throughout. No custom cryptography.

| What | Algorithm | Why |
|------|-----------|-----|
| Identity & signatures | Ed25519 | Same as Signal, Tor, SSH. Battle-tested. |
| Key encryption at rest | ChaCha20-Poly1305 | RFC 8439 compliant. Constant-time. |
| Key derivation | HKDF-SHA-256 | Standard, audited. |
| Audit chain integrity | BLAKE3 | Fast, secure, tamper-evident. |
| Session exchange | X25519 ephemeral | Perfect forward secrecy per session. |
| **Post-quantum (optional)** | **ML-KEM-768 + ML-DSA-65** | **NIST-standardized. "Harvest now, decrypt later" resistant.** |

Post-quantum support is built in today, not a future promise. Enable it with a config flag when you need it.

**Formal verification:** HSIP includes an optional Z3 SMT solver module for machine-checked security proofs — not just tests, but mathematical guarantees of security properties.

---

## For Developers

HSIP exposes a REST API at `http://127.0.0.1:7777`. SDKs available for Python, Node.js, and Go.

```bash
export KEY="hsip_your_key_here"

# Sign a message — creates a cryptographic, timestamped proof
curl -X POST http://127.0.0.1:7777/v1/messages/sign \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{"content": "I authorize this transaction."}'

# Get AI agent capability spec — inject into any AI system prompt
curl http://127.0.0.1:7777/v1/agent/capabilities \
  -H "Authorization: Bearer $KEY"

# Grant time-bounded consent to a peer
curl -X POST http://127.0.0.1:7777/v1/consent/grant \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{"peer_verify_key": "...", "scope": "contact", "expires_in_seconds": 86400}'

# Enable DNS tracker blocker
curl -X POST http://127.0.0.1:7777/v1/dns/enable \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{"port": 5300}'
```

Full interactive API docs at `http://127.0.0.1:7777/docs` when HSIP is running (OpenAPI 3.0).

### Python SDK

```python
from hsip import HSIPClient

client = HSIPClient(api_key="hsip_...", base_url="http://localhost:7777")
identity = client.get_or_create_identity()
signed = client.sign_message("I authorized this action.")
client.grant_consent(peer_verify_key="...", scope="contact")
```

### Connecting an AI agent

Point any AI at the capabilities endpoint and it knows exactly what HSIP can do:

```
GET http://127.0.0.1:7777/v1/agent/capabilities
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
- GDPR Article 17 right-to-erasure endpoint (`DELETE /v1/tenant/erase`)
- Tamper-evident audit trail exportable for legal proceedings
- Rate limiting per API key
- No telemetry, no phone-home, no licensing server

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
# Browser opens automatically at http://127.0.0.1:7777
```

Development mode (dashboard hot-reloads):

```bash
cargo run -p hsip-api          # API on :7777
cd dashboard && npm run dev    # UI on :5173 with hot reload
```

Run the full test suite (238 tests):

```bash
cargo test --workspace
```

---

## Architecture

```
┌────────────────────────────────────────────────────┐
│  hsip-api     Rust / Axum / Tokio — REST API        │
│  hsip-dns     UDP :5300 — DNS tracker blocker       │
│  hsip-core    Ed25519, X25519, ChaCha20, ML-KEM     │
│  hsip-session Ephemeral sessions, forward secrecy   │
│  SQLite        Local storage — no cloud required    │
│  React         Embedded dashboard — single binary   │
└────────────────────────────────────────────────────┘
```

Everything runs in a single binary. No Docker required to get started. No external services. The entire system fits in your pocket.

16 specialized crates. 238 tests. RFC 8439 compliance verified. Audited RustCrypto primitives throughout.

---

## Security

- Private keys encrypted with ChaCha20-Poly1305 + HKDF before storage — master key never touches disk
- API keys stored as SHA-256 hashes, never plaintext
- Rate limiting on all endpoints
- Append-only BLAKE3 hash-chained audit trail — tampering is detectable
- Replay attack prevention via monotonic nonce counters
- No telemetry, no analytics, no phone-home — ever

See [THREAT_MODEL.md](THREAT_MODEL.md) for a full breakdown of what HSIP protects against and what it does not.

To report a vulnerability: **sanchezleal1989@gmail.com**

---

## License

MIT © 2025–2026 Dayana Sanchez — use it freely. See [LICENSE](LICENSE).

HSIP is open source because trust requires auditability. You should be able to verify that your identity key never leaves your machine. Now you can.

---

**Your data. Your keys. Your machine.**
