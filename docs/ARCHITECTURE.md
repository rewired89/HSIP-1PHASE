# HSIP Architecture Documentation

**Product:** Hardened Secure Identity Protocol (HSIP) API  
**Version:** 0.2.0  
**Date:** February 2026  

---

## 1. Overview

HSIP is a self-hosted, multi-tenant REST API that provides:

- **Cryptographic identity** — Ed25519 keypair generation and management per tenant
- **Consent management** — time-bound, revocable consent grants between parties
- **Message signing and verification** — tamper-evident message authentication
- **Privacy-preserving credentials** — signed claims (age, KYC, certifications) without storing underlying documents
- **AI agent governance** — velocity tracking, anomaly detection, and automatic revocation for machine identities
- **Full audit trail** — immutable per-tenant audit log for every action
- **GDPR compliance** — right-to-erasure endpoint that deletes all tenant data

HSIP is designed to be deployed on-premises or in the deploying organization's own cloud — it does not phone home, has no licensing server, and all data stays within the organization's infrastructure.

---

## 2. Technology Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Runtime | Rust (stable) | Memory safety, zero-cost abstractions, no GC pauses |
| Web framework | Axum 0.7 (Tokio) | Type-safe async HTTP with compile-time route safety |
| Database | SQLite (default) or PostgreSQL | Single-file local-first (SQLite) or production HA (PostgreSQL) |
| DB driver | sqlx 0.8 (AnyPool) | Async, supports both SQLite and PostgreSQL at runtime |
| Cryptography | ed25519-dalek v2 | Ed25519 signing (same algorithm as Signal, Tor, SSH) |
| Key hashing | sha2 (SHA-256) | One-way hash of API keys — raw keys never stored |
| Concurrency | DashMap + AtomicU64 | Lock-free in-memory velocity tracking |
| Observability | Prometheus 0.13 | Industry-standard metrics scraping |
| Deployment | Helm 3 + Kubernetes | Production deployment with PVC, ingress, security context |
| API documentation | OpenAPI 3.0 + Swagger UI | Interactive API browser at `/docs` |

### Core Crates (11 total)

| Crate | Purpose |
|-------|---------|
| `hsip-api` | REST API server — the primary integration point |
| `hsip-core` | Cryptographic primitives, consent protocol implementation |
| `hsip-session` | Session management, handshake retry logic |
| `hsip-net` | UDP transport layer for peer-to-peer communication |
| `hsip-auth` | Authentication primitives |
| `hsip-reputation` | Reputation scoring for P2P network nodes |
| `hsip-cli` | Command-line interface for administration |
| `hsip-gateway` | Gateway/proxy for protocol bridging |
| `hsip-common` | Shared types and utilities |
| `hsip-regenerative` | Key recovery and regeneration |
| `hsip-telemetry-guard` | Telemetry anonymization |
| `hsip-integration-sdk` | Rust SDK for application integration |

---

## 3. API Reference

All protected endpoints require `Authorization: Bearer <api_key>`.

### Identity
| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/v1/identity` | Create or retrieve Ed25519 keypair for this tenant |
| `GET` | `/v1/identity` | Get existing tenant identity (verify key + created_at) |

### Consent
| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/v1/consent/grant` | Grant time-bound consent to a peer (by their verify key) |
| `POST` | `/v1/consent/revoke` | Instantly revoke consent from a peer |
| `GET` | `/v1/consent` | List all consents for this tenant |
| `GET` | `/v1/consent/:peer_key` | Get consent status for a specific peer |

### Messages
| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/v1/messages/sign` | Sign a message with this tenant's private key |
| `POST` | `/v1/messages/verify` | Verify a peer's message signature |
| `GET` | `/v1/messages` | List message records (last 100) |

### Credentials
| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/v1/credentials/issue` | Issue a signed credential (age_over_18, kyc_verified, etc.) |
| `POST` | `/v1/credentials/verify` | Verify signature, expiry, and revocation status |
| `DELETE` | `/v1/credentials/:id/revoke` | Instantly revoke a credential |
| `GET` | `/v1/credentials` | List issued credentials |

### API Keys
| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/v1/keys` | Create a new API key (human / service / ai_agent) with optional expiry |
| `GET` | `/v1/keys` | List all keys for this tenant |
| `DELETE` | `/v1/keys/:id` | Revoke a key immediately |

### AI Agents
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/v1/agents` | List AI agent keys with live velocity stats |

### Audit
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/v1/audit` | Retrieve audit log (filterable by action, limit up to 500) |

### Tenant
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/v1/tenant` | Get tenant information |
| `POST` | `/v1/tenant/erase` | GDPR Article 17 — permanently delete all tenant data |

### System (no auth required)
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Health check (`{"status":"ok","version":"0.2.0"}`) |
| `GET` | `/metrics` | Prometheus metrics in text format |
| `GET` | `/openapi.json` | OpenAPI 3.0 specification |
| `GET` | `/docs` | Interactive Swagger UI |

---

## 4. Data Flow

### 4.1 API Key Authentication

```
Client Request (Bearer token)
    │
    ▼
SHA-256 hash of token
    │
    ▼
Lookup key_hash in api_keys table
    ├── Not found / inactive → HTTP 401 Unauthorized
    ├── expires_at in the past → HTTP 401 Expired
    │
    ▼
Check rate_limiter (in-memory DashMap)
    ├── Count > RATE_LIMIT_RPM in current 60s window → HTTP 429
    │
    ▼
If agent_type = 'ai_agent': check velocity tracker
    ├── > 100 req/min → log anomaly to audit + Prometheus
    └── > 1000 req/min → auto-revoke key + log to audit
    │
    ▼
Extract tenant_id → route handler
```

### 4.2 Credential Issuance

```
POST /v1/credentials/issue
    │
    ▼
Load tenant signing key (Ed25519 private key, stored in DB)
    │
    ▼
Build CredentialPayload { id, claim, user_token, issuer_verify_key, issued_at, expires_at }
    │
    ▼
Sign canonical JSON of payload with Ed25519
    │
    ▼
Store { payload, signature } in credentials table
    │
    ▼
Write audit entry: 'credential.issued'
    │
    ▼
Return { credential, signature } to caller
```

### 4.3 Credential Verification

```
POST /v1/credentials/verify  { credential, signature }
    │
    ▼
Check: expires_at > now?  → expired = true/false
    │
    ▼
Lookup credential.id in DB → revoked = true/false
    │
    ▼
Decode issuer_verify_key (Base64 → 32 bytes)
    │
    ▼
Decode signature (Base64 → 64 bytes)
    │
    ▼
Ed25519 verify(canonical_json, signature, issuer_verify_key)
    │
    ▼
valid = (sig_valid AND NOT expired AND NOT revoked)
    │
    ▼
Return { valid, claim, expired, revoked, expires_at }
```

### 4.4 Data at Rest — What is Stored

| Table | Contents | Sensitive? | Notes |
|-------|----------|-----------|-------|
| `tenants` | id, name, created_at | Low | No PII |
| `api_keys` | id, tenant_id, **key_hash**, name, agent_type, expires_at | Medium | Raw key never stored |
| `identities` | tenant_id, signing_key_b64, verify_key_b64, created_at | High | Private signing key in Base64 |
| `consents` | id, peer_verify_key, status, timestamps | Medium | No user identity data |
| `messages` | content, signature, peer_verify_key, timestamps | Medium | Content is application-defined |
| `credentials` | claim, **user_token** (opaque), issuer_verify_key, signature | Low | No underlying identity document |
| `audit_entries` | action, details, timestamps | Low | No PII by design |

**Key privacy properties:**
- API keys: only SHA-256 hash stored. The raw key is shown once at creation and never retrievable.
- Credentials: the `user_token` is an opaque value chosen by the integrating application. HSIP never sees or stores the user's actual identity document.
- Identities: the Ed25519 private signing key is stored in the tenant's own database (which they control). HSIP Engineering never has access to this key.

---

## 5. Configuration Reference

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `sqlite:hsip_api.db` | Database connection string. Use `postgresql://user:pass@host/db` for PostgreSQL |
| `HSIP_DB_PATH` | `hsip_api.db` | Alternative: SQLite file path (used if DATABASE_URL is not set) |
| `PORT` | `3000` | HTTP listen port |
| `RATE_LIMIT_RPM` | `300` | Per-key request rate limit (requests per 60-second window) |
| `RUST_LOG` | `hsip_api=info` | Log level (trace, debug, info, warn, error) |

---

## 6. Dependencies — Security Audit Summary

Run `cargo audit` from the repository root to check all dependencies against the RustSec Advisory Database.

**Direct cryptographic dependencies:**

| Crate | Version | Role | Audit Status |
|-------|---------|------|-------------|
| `ed25519-dalek` | 2.x | Ed25519 signing/verification | No known advisories |
| `sha2` | 0.10.x | SHA-256 key hashing | No known advisories |
| `rand` | 0.8.x | Cryptographically secure random key generation | No known advisories |

**All dependencies are pinned in `Cargo.lock`.** To generate a Software Bill of Materials:

```bash
cargo metadata --format-version 1 > sbom.json
# Or with CycloneDX format:
cargo install cargo-cyclonedx
cargo cyclonedx
```

---

## 7. Deployment

### Minimum Requirements

| Resource | Minimum | Recommended |
|----------|---------|-------------|
| CPU | 0.1 core | 0.5 core |
| RAM | 64 MB | 256 MB |
| Disk | 100 MB | 1 GB (for database) |
| OS | Linux (x86_64, ARM64) or Windows | Linux |

### Kubernetes (Helm)

```bash
# Deploy with default settings (SQLite)
helm install hsip ./helm/hsip-api

# Deploy with PostgreSQL
helm install hsip ./helm/hsip-api \
  --set env.DATABASE_URL="postgresql://user:pass@postgres:5432/hsip" \
  --set persistence.enabled=false
```

### Docker (single container)

```bash
docker build -t hsip-api .
docker run -p 3000:3000 \
  -e DATABASE_URL=sqlite:/data/hsip_api.db \
  -v hsip_data:/data \
  hsip-api
```

### Direct binary

```bash
DATABASE_URL=sqlite:hsip_api.db cargo run -p hsip-api --release
```

The admin API key is printed on first startup and saved to `hsip_admin_key.txt`.

---

## 8. Security Boundaries

```
┌─────────────────────────────────────────────────────┐
│  Client (browser, app, service, AI agent)           │
│  Authenticates with: Bearer <api_key>               │
└───────────────────┬─────────────────────────────────┘
                    │ HTTPS (TLS at reverse proxy)
┌───────────────────▼─────────────────────────────────┐
│  Reverse Proxy / Kubernetes Ingress                 │
│  (nginx, Caddy, or cloud load balancer)             │
│  Responsibility: TLS termination, IP filtering      │
└───────────────────┬─────────────────────────────────┘
                    │ HTTP
┌───────────────────▼─────────────────────────────────┐
│  HSIP API (hsip-api binary)                         │
│  Port 3000, non-root user                           │
│  Rate limiting, auth, audit logging                 │
│  Request body limit: 2 MB                           │
└───────────────────┬─────────────────────────────────┘
                    │
┌───────────────────▼─────────────────────────────────┐
│  Database                                           │
│  SQLite: single file (hsip_api.db)                 │
│  PostgreSQL: external managed database              │
│  Multi-tenant row-level isolation                   │
└─────────────────────────────────────────────────────┘
```

**HSIP is responsible for:** Authentication, authorization, cryptographic operations, audit logging, rate limiting, data isolation between tenants.

**The deploying organization is responsible for:** TLS/HTTPS, IP allowlisting, database backups, OS hardening, network segmentation, log aggregation.

---

## 9. Compliance Roadmap

| Item | Priority | Status |
|------|----------|--------|
| Penetration test (independent) | High | Planned Q3 2026 |
| SOC 2 Type I | High | Planned Q4 2026 |
| Formal incident response plan | High | In progress |
| Automated SBOM generation in CI | Medium | In progress |
| Native TLS / mTLS support | Medium | Planned |
| Hash-chained tamper-evident audit log | Medium | Planned |
| SOC 2 Type II | High (post-revenue) | Planned 2027 |
| ISO 27001 | Medium | Planned 2027 |
