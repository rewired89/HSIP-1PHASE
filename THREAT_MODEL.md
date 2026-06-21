# HSIP Threat Model

**Version:** 0.3  
**Date:** 2026-06-21  
**Author:** Dayana Sanchez (rewired89)  
**Review status:** Self-reviewed. Third-party audit planned before v1.0 commercial release. Codebase is fully open source for independent review.

---

## 1. What HSIP Is and Is Not

HSIP is a self-hosted, single-binary identity and consent server. It gives AI agents, services, and people a cryptographic identity (Ed25519), a consent layer, and a tamper-evident audit trail.

**HSIP is not:**
- A TLS terminator or VPN
- A hardware security module (HSM)
- A firewall or intrusion detection system
- A replacement for OS-level access controls

HSIP assumes it runs on a server you already trust at the OS level. Its job is to protect identity state, signing keys, and audit integrity *above* that layer.

---

## 2. Protected Assets

| Asset | Sensitivity | Where stored |
|---|---|---|
| Ed25519 private signing keys | Critical | Encrypted at rest in SQLite (ChaCha20-Poly1305 + HKDF-SHA-256) |
| Master key (encrypts signing keys) | Critical | Filesystem (`~/.hsip/master.key`) or `HSIP_MASTER_KEY` env var |
| API keys (admin, service, agent) | High | SHA-256 hashes only — raw key never written to DB or disk |
| Audit log integrity | High | SQLite; BLAKE3 hash-chained entries |
| Consent records | Medium | SQLite, scoped by `tenant_id` |
| Message signatures | Medium | SQLite, scoped by `tenant_id` |
| Verifiable credentials | Medium | SQLite, scoped by `tenant_id` |
| Trusted peer keys | Low | SQLite, scoped by `tenant_id` |

---

## 3. Threat Actors

| Actor | Capability | Goal |
|---|---|---|
| Remote unauthenticated attacker | HTTP access to the API | Enumerate tenants, forge signatures, bypass rate limits |
| Compromised AI agent | Valid `ai_agent` API key | Make unlimited calls, exfiltrate data, impersonate tenant |
| Attacker with DB read access | Read-only SQLite file | Extract private keys, forge audit entries |
| Attacker with filesystem access | Read `~/.hsip/` directory | Steal master key, derive private keys from encrypted DB blobs |
| Rogue tenant (multi-tenant mode) | Valid API key for their tenant | Access another tenant's keys, consents, or audit log |
| Replay attacker | Captured signed messages or nonces | Resubmit a previously authorized action |

---

## 4. Defenses — What the Code Actually Does

### 4.1 Private Keys Never Leave the Server Unencrypted

Every Ed25519 private key is encrypted before writing to the database:

- **ChaCha20-Poly1305** authenticated encryption (`chacha20poly1305` crate — RustCrypto, publicly audited)
- **HKDF-SHA-256** key derivation with fixed info string `hsip-key-encryption-v1` (`hkdf` crate — RustCrypto, RFC 5869)
- **12-byte random nonce per encryption** drawn from `OsRng` (OS CSPRNG)
- Wire format stored in DB: `nonce(12 bytes) || ciphertext+tag(48 bytes)`, base64-encoded

An attacker with read-only access to the SQLite file cannot decrypt private keys without also having the master key.

*Source: `crates/hsip-api/src/key_encryption.rs`*

### 4.2 API Keys Stored as SHA-256 Hashes Only

Raw API tokens are never written to disk or the database. On every authenticated request:

1. The `Bearer <token>` is extracted from the `Authorization` header
2. `SHA-256(token)` is computed in memory
3. The hash is looked up in the `api_keys` table
4. The raw token is discarded

A stolen database contains only hashes — not usable API keys.

*Source: `crates/hsip-api/src/auth.rs` — `hash_key()`*

### 4.3 Replay Attack Prevention: 64-Packet Sliding Window

The `hsip-core` crate implements a 64-packet sliding window nonce tracker:

- Zero nonces are rejected unconditionally (`NonceError::ZeroNonce`)
- Any previously seen nonce is rejected (`NonceError::Replay`)
- Nonces more than 64 positions behind the current maximum are rejected (`NonceError::TooOld`)
- Out-of-order delivery within a 64-packet window is supported (handles UDP reordering)

An attacker who captures a signed message and resubmits it will be rejected.

*Source: `crates/hsip-core/src/nonce.rs`*

### 4.4 Tenant Isolation at the Query Level

Every SQL query that touches tenant data binds `tenant_id` as a required parameter. There is no shared table scan that could return another tenant's rows:

```sql
-- consent routes
SELECT status, expires_ms FROM consents
  WHERE tenant_id = ? AND peer_verify_key = ?

-- audit routes
SELECT * FROM audit_entries WHERE tenant_id = ? AND action LIKE ?

-- key revocation
UPDATE api_keys SET active = 0 WHERE id = ? AND tenant_id = ?
```

A valid API key from Tenant A cannot be used to read or modify Tenant B's data.

*Source: `crates/hsip-api/src/routes/`*

### 4.5 AI Agent Velocity Limits and Auto-Revocation

Keys with `agent_type = 'ai_agent'` are subject to two automatic thresholds enforced per 60-second window:

| Threshold | Action |
|---|---|
| > 100 requests/min | Anomaly logged to audit trail as `agent.anomaly_detected` |
| > 1000 requests/min | Key immediately added to `pending_revocation` DashSet (in-memory) + async DB revocation + `agent.auto_revoked` audit entry |

**The `pending_revocation` DashSet is checked on every incoming request before the DB lookup.** This means a runaway agent's key is blocked within the same request cycle that triggers the hard limit — there is no window where subsequent requests slip through while the async database write is in flight.

*Source: `crates/hsip-api/src/auth.rs` — `check_agent_velocity()`*

### 4.6 Per-Key Rate Limiting

Every API key (not only agent keys) is subject to a sliding-window rate limit of 300 requests/minute by default (configurable via `RATE_LIMIT_RPM` environment variable). Enforcement uses an in-memory `DashMap<key_id, RateWindow>` — the limit is on the key identity, not the client IP, so it cannot be bypassed by IP rotation or multiplexed connections.

*Source: `crates/hsip-api/src/auth.rs` — `check_rate_limit()`*

### 4.7 Consent Expiry Enforced Server-Side on Every Check

Consent records include `expires_ms`. Expiry is evaluated at query time, not cached:

```sql
SELECT status, expires_ms FROM consents
WHERE tenant_id = ? AND peer_verify_key = ?
  AND status = 'active'
  AND (expires_ms IS NULL OR expires_ms > ?)
```

An expired consent returns the same response as a revoked consent. There is no grace period and no client-side caching that could serve stale authorization.

*Source: `crates/hsip-api/src/routes/consent.rs`*

### 4.8 BLAKE3 Hash-Chained Audit Log

Audit entries are append-only and BLAKE3 hash-chained. Each entry's stored hash includes the content of the previous entry. Modifying any historical entry breaks the chain from that point forward, which is detectable by any verifier with access to the log.

An attacker with write access to the SQLite database cannot silently alter history — they would need to recompute the entire chain from the modified entry forward, producing a different terminal hash that any external verifier will reject.

### 4.9 Credential Integrity: Ed25519 Signatures

Verifiable credentials carry Ed25519 signatures over a canonical payload including `claim`, `user_token`, `issuer_verify_key`, `issued_at`, and `expires_at`. Modifying any field after issuance invalidates the signature. The `/v1/credentials/verify` endpoint rejects any credential with an invalid signature.

An intercepted credential cannot be modified without the tenant's private signing key.

### 4.10 Immediate Revocation

When a credential or API key is revoked:

- **Credentials**: `revoked = 1` is set in the database; the `/v1/credentials/verify` endpoint checks this flag on every call — there is no cache
- **API keys (normal)**: `active = 0` set in the database; checked on every request via the auth middleware
- **AI agent keys (auto-revoked)**: inserted into the `pending_revocation` DashSet *before* the async DB write — in-flight requests are blocked immediately

Revocation takes effect on the next request, with no propagation delay.

### 4.11 No Hand-Rolled Cryptography

Every cryptographic operation uses a published, independently reviewed library:

| Operation | Crate | Standard |
|---|---|---|
| Ed25519 sign / verify | `ed25519-dalek` (Ristretto team) | RFC 8032 |
| ChaCha20-Poly1305 AEAD | `chacha20poly1305` (RustCrypto) | RFC 8439 |
| HKDF key derivation | `hkdf` (RustCrypto) | RFC 5869 |
| BLAKE3 hash chaining | `blake3` | BLAKE3 spec |
| X25519 key exchange | `x25519-dalek` | RFC 7748 |
| SHA-256 (API key hashing) | `sha2` (RustCrypto) | FIPS 180-4 |
| ML-KEM-768 (post-quantum KEM) | `pqcrypto-kyber` | NIST FIPS 203 |
| ML-DSA-65 (post-quantum signatures) | `pqcrypto-dilithium` | NIST FIPS 204 |
| Random key / nonce generation | `rand` with `OsRng` | Delegates to OS CSPRNG |

HSIP does not implement any cryptographic algorithm from scratch. The only glue code is key derivation setup, nonce generation, and encode/decode — approximately 90 lines in `key_encryption.rs`, auditable in a single file.

---

## 5. Trust Boundaries

| Boundary | Trust level | Notes |
|---|---|---|
| HTTPS client → HSIP API | Authenticated | Valid bearer key required for all `/v1/*` endpoints |
| HSIP server → SQLite | Trusted | Database must not be publicly accessible |
| Master key storage → HSIP server | Critical | Compromise = all signing keys exposed |
| Tenant A ↔ Tenant B | Untrusted | Isolated by `tenant_id` on every query |
| Admin key holder | Full | Can provision all tenant keys — protect carefully |
| AI agent key holder | Scoped | Velocity-limited, auto-revoked at 1000 req/min |
| Trusted peer (federated trust) | Explicit | Verify key manually registered; messages verified locally |

---

## 6. What HSIP Does Not Protect Against

The following are **explicitly out of scope**. They must be addressed at the infrastructure or application layer.

| Attack | Why out of scope | Recommended mitigation |
|---|---|---|
| **OS or host compromise** | An attacker with root access can read the master key from the filesystem or memory | OS hardening, container isolation, secrets manager for master key |
| **Physical server access** | An attacker with physical access can read the filesystem | Full-disk encryption at the OS level |
| **API key theft** | A stolen key grants full tenant access until rotated | Short-lived keys, audit log monitoring, key rotation policy |
| **Network-layer DDoS** | HSIP has per-key rate limiting but no IP-level flood protection | Reverse proxy or CDN in front for public deployments |
| **Side-channel attacks** | No constant-time guarantees outside what `ed25519-dalek` and `chacha20poly1305` provide | Not a realistic concern for network-connected deployments |
| **Consent coercion** | HSIP enforces cryptographic consent, not voluntary human consent | Application-layer UX and legal controls |
| **Master key loss** | If `master.key` is lost, all encrypted signing keys are permanently unrecoverable | Back up the master key; use secrets manager |
| **HSM-backed key storage** | Master key lives on the filesystem | Point `HSIP_MASTER_KEY` at a secrets manager (Vault, AWS KMS) |
| **Post-quantum adversaries (current Ed25519)** | Ed25519 is not quantum-safe | ML-KEM-768 + ML-DSA-65 available via `hsip-verify` for environments requiring it |
| **Social engineering** | If an admin is phished, HSIP cannot detect it | Operational security, 2FA on the server, key rotation |

---

## 7. Residual Risks and Known Gaps

Documented openly. Tracked for the v1.0 audit milestone.

| Gap | Risk | Mitigation path |
|---|---|---|
| No third-party security audit | Medium | Planned before v1.0. Codebase is open source and auditable now. |
| Master key on filesystem (no HSM) | Medium | Use `HSIP_MASTER_KEY` env var + external secrets manager for production |
| In-memory rate limiter resets on restart | Low | A burst attack timed around a restart or deploy can temporarily exceed rate limits |
| SQLite without WAL under write contention | Low | Low risk for single-tenant deployments; use `DATABASE_URL` pointing at PostgreSQL for high-concurrency |
| No mutual TLS between federated HSIP nodes | Low | Federated trust uses explicit Ed25519 verify key registration; no automatic peer auth at the transport layer |
| Audit log not externally anchored | Low | BLAKE3 chain is self-verifiable; no blockchain or transparency log integration yet |
| Clock skew affects consent/credential expiry | Low | Use NTP synchronization in production |

---

## 8. Audit and Review Status

| Item | Status |
|---|---|
| Third-party security audit | Not yet completed — planned for v1.0 |
| Formal verification of protocol properties | `hsip-verify` crate uses Z3 SMT solver for cryptographic protocol proofs |
| RFC compliance test vectors | RFC 8439 (ChaCha20-Poly1305), RFC 8032 (Ed25519) vectors pass in CI |
| Dependency vulnerability scanning | `cargo audit` runs on every build |
| Minimum supported Rust version | 1.88.0 |

---

## 9. Responsible Disclosure

If you find a vulnerability, please disclose it responsibly:

**Email:** sanchezleal1989@gmail.com  
**Subject:** `[HSIP SECURITY]`

**Response commitments:**
- Acknowledgement within 48 hours
- Status update within 7 days
- Critical issues patched within 7 days of confirmed reproduction
- Researchers credited by name in release notes (or anonymously on request)

HSIP does not currently have a bug bounty program.
