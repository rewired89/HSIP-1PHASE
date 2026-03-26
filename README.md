# HSIP — High Security Internet Protocol

**Cryptographic consent management for everyone: individuals, developers, and organizations**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust: 1.82+](https://img.shields.io/badge/Rust-1.82%2B-orange.svg)](https://www.rust-lang.org/)
[![Build](https://github.com/rewired89/hsip-1phase/actions/workflows/ci.yml/badge.svg)](https://github.com/rewired89/hsip-1phase/actions)

HSIP started as a tool to protect non-technical people from trackers, surveillance, and silent data collection — and grew into a full cryptographic consent system. It gives you **mathematical proof** of consent, not just a checkbox a company can quietly ignore.

---

## What HSIP does

- **Signs anything** — messages, agreements, creative work — with your Ed25519 key. The signature is a certificate that holds up even in a dispute.
- **Manages consent cryptographically** — every permission you grant is locked with your key and can be revoked instantly.
- **Blocks trackers at the DNS level** — a built-in DNS resolver intercepts traffic before trackers load, system-wide across every app and browser.
- **Keeps a tamper-proof audit log** — complete history of every consent operation.
- **Runs entirely on your machine** — no cloud, no third party, no phoning home.

---

## Quick start

### Download a release binary (no setup required)

Go to [Releases](https://github.com/rewired89/hsip-1phase/releases) and download the binary for your platform:

| Platform | File |
|----------|------|
| Windows  | `hsip-windows-x64.exe` |
| macOS (Apple Silicon) | `hsip-macos-arm64` |
| macOS (Intel) | `hsip-macos-x64` |
| Linux | `hsip-linux-x64` |

Double-click (Windows) or run from terminal — your browser opens automatically at `http://localhost:3000`.

Your access key is printed in the terminal on first run and saved to:
- **Windows:** `%APPDATA%\HSIP\admin.key`
- **Mac / Linux:** `~/.hsip/admin.key`

### Build from source

```bash
# 1. Build the dashboard
cd dashboard && npm install && npm run build && cd ..

# 2. Build the binary with embedded dashboard
cargo build --release -p hsip-api --features hsip-api/embed-dashboard

# 3. Run it
./target/release/hsip-api
```

Or for development (dashboard hot-reloads separately):
```bash
cargo run -p hsip-api          # API on :3000
cd dashboard && npm run dev    # UI on :5173
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  REST API (Axum / Tokio)    Multi-tenant, async         │
│  SQLite / PostgreSQL        Persistent storage           │
│  Ed25519 Signatures         Identity & consent proofs    │
│  ChaCha20-Poly1305          Key encryption at rest       │
│  hsip-dns                   Local DNS tracker blocker    │
└─────────────────────────────────────────────────────────┘
```

### Cryptographic stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| Identity | Ed25519 | Signatures for consent records |
| Encryption | ChaCha20-Poly1305 + HKDF | Private key encryption at rest |
| Key Exchange | X25519 | Optional session encryption |
| TLS | rustls (TLS 1.3) | HTTPS in production |

All cryptography uses audited [RustCrypto](https://github.com/RustCrypto) libraries.

### Crates

| Crate | Description |
|-------|-------------|
| `hsip-core` | Ed25519 identity, consent records, message signing |
| `hsip-api` | REST API server (Axum), embeds the dashboard |
| `hsip-dns` | Local UDP DNS resolver — blocks tracker domains |
| `hsip-session` | Encrypted session layer |
| `hsip-net` | Network utilities |
| `hsip-telemetry-guard` | Tracker database (40+ known endpoints) |
| `hsip-intercept` | OS-level event detection (Windows/macOS/Linux) |
| `hsip-auth` | Authentication and key management |
| `hsip-gateway` | API gateway layer |
| `hsip-reputation` | Domain reputation scoring |
| `hsip-regenerative` | Key rotation and recovery |
| `hsip-integration-sdk` | SDK for third-party integrations |

---

## API

```bash
export KEY="hsip_your_key_here"

# Create an identity
curl -X POST http://localhost:3000/v1/identity \
  -H "Authorization: Bearer $KEY"

# Sign a message
curl -X POST http://localhost:3000/v1/messages/sign \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{"message": "I agree to these terms on 2025-01-01"}'

# Grant consent to another party
curl -X POST http://localhost:3000/v1/consent/grant \
  -H "Authorization: Bearer $KEY" \
  -d '{"peer_key": "...", "scope": "read_data", "expires_in_days": 30}'

# Enable the DNS tracker blocker
curl -X POST http://localhost:3000/v1/dns/enable \
  -H "Authorization: Bearer $KEY" \
  -d '{"port": 5300}'

# Check what's been blocked
curl http://localhost:3000/v1/dns/log \
  -H "Authorization: Bearer $KEY"
```

Full API reference: [docs/API_REFERENCE.md](docs/API_REFERENCE.md)

---

## Security

HSIP underwent a self-conducted red-team review with **20 vulnerabilities identified and fixed**:

- **3 Critical** — Private key encryption, admin key permissions, cross-tenant isolation
- **7 High** — Rate limiting, CORS, HTTPS enforcement, revocation gaps
- **5 Medium** — JSON canonicalization, key validation, CDN integrity
- **4 Low** — Cryptographic RNG, pagination, database indexes

A third-party audit is recommended before deploying in sensitive environments.

---

## Performance

Tested on 4-core CPU, 8 GB RAM, PostgreSQL on the same host:

| Endpoint | Throughput | p95 Latency |
|----------|------------|-------------|
| `POST /v1/identity` | 500 req/s | 150 ms |
| `POST /v1/credentials/issue` | 200 req/s | 250 ms |
| `GET /v1/consent/:id` | 2,000 req/s | 50 ms |
| `/health` | 5,000 req/s | 15 ms |

---

## Why HSIP vs. alternatives

| Feature | HSIP | Auth0/Okta | AWS IAM |
|---------|------|------------|---------|
| Cryptographic consent proofs | ✅ Ed25519 | ❌ Policy-based | ❌ Policy-based |
| Revocable credentials | ✅ Instant | ⚠️ Token-based | ⚠️ Token-based |
| Immutable audit trail | ✅ Yes | ⚠️ Log-based | ⚠️ CloudTrail |
| Runs fully local | ✅ Yes | ❌ Cloud only | ❌ Cloud only |
| Open source | ✅ Yes | ❌ No | ❌ No |
| DNS tracker blocking | ✅ Built-in | ❌ No | ❌ No |

---

## Documentation

- [DEPLOYMENT.md](DEPLOYMENT.md) — Production setup, HA, PostgreSQL, monitoring
- [WINDOWS_SETUP.md](WINDOWS_SETUP.md) — Windows development guide
- [LINUX_SETUP.md](LINUX_SETUP.md) — Linux setup guide
- [docs/API_REFERENCE.md](docs/API_REFERENCE.md) — Full API reference
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — Deep architecture overview
- [docs/PROTOCOL_SPEC.md](docs/PROTOCOL_SPEC.md) — Wire protocol specification
- [docs/SDK_INTEGRATION.md](docs/SDK_INTEGRATION.md) — Integration guide

---

## Contributing

Issues and pull requests are welcome. If you're building something with HSIP or have ideas, open an issue — happy to discuss.

---

## Contact

If you're interested in the project, want to collaborate, or want to hire me:

**Email:** sanchezleal1989@gmail.com

---

## License

MIT — use it freely. See [LICENSE](LICENSE) for the full terms.

---

**HSIP: Where consent is code, not policy.**
