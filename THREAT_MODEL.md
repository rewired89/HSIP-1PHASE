# HSIP Threat Model

**Version:** 0.5-draft  
**Date:** 2026-07-19  
**Author:** Dayana Sanchez (rewired89)  
**Review status:** Self-reviewed draft. This document was written from code inspection and requires the author's line-by-line verification before being treated as a published attack surface claim. Third-party audit planned before v1.0 commercial release. Codebase is fully open source for independent review.

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

### 4.3 Replay Prevention: Protocol Layer (UDP Sessions)

The `hsip-core` crate implements a 64-packet sliding window nonce tracker for the UDP consent and session protocol:

- Zero nonces are rejected unconditionally (`NonceError::ZeroNonce`)
- Any previously seen nonce is rejected (`NonceError::Replay`)
- Nonces more than 64 positions behind the current maximum are rejected (`NonceError::TooOld`)

**Scope limitation:** This nonce window is used in the UDP session layer (`hsip-core`). The HTTP REST API does not enforce per-request nonce replay prevention. An attacker who captures a valid HTTP request with a stolen API key can replay it until the key is revoked. The defense against this at the HTTP layer is the rate limiter (Section 4.6) and short-lived key expiry.

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

### 4.8 Append-Only Audit Log, Now BLAKE3 Hash-Chained

The `audit_entries` table in the HTTP API is append-only by design: there is no `UPDATE` or `DELETE` endpoint for audit records, and no application code path removes entries. An attacker using only the API cannot erase or alter the audit history.

As of this revision, every write also extends a per-tenant BLAKE3 hash chain: `entry_hash = BLAKE3(prev_hash || id || tenant_id || action || peer_verify_key || details || timestamp)`. `GET /v1/audit/verify` recomputes the chain server-side and reports whether it's intact — an attacker with direct database write access (OS-level compromise) can no longer alter or delete a row without breaking every link after it, detectably. This closes the gap previously documented here: BLAKE3 hash chaining existed in the `hsip-telemetry-guard` crate but was not wired into the HTTP API's own audit table.

*Source: `crates/hsip-api/src/audit_log.rs`, `crates/hsip-api/src/routes/audit.rs::verify_chain`*

**Scope limitations that remain:**
- **The chain starts at upgrade time, not at tenant creation.** Rows written before this migration have NULL `prev_hash`/`entry_hash` and are excluded from verification (`unchained` count in the `/v1/audit/verify` response) — there is no retroactive integrity proof for history older than the chain itself.
- **This proves tamper-*evidence*, not tamper-*prevention*.** An attacker with DB write access can still delete the *entire* chain (not just alter it undetected) and there is nothing here that stops that, or that anchors the chain outside this database. For that, see the anchoring approach already used for decision attestations (§ Decision Attestations in `CLAUDE.md`) — the audit log is not yet anchored to an external timestamping service the way decisions are.
- For environments requiring protection against total chain deletion, the mitigation is still filesystem-level immutability (append-only S3 bucket, WORM storage, or periodic signed export to an external log sink) — the hash chain detects alteration of what remains, it doesn't prevent removal.

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

### 4.12 Consent Records Which Kind of Key Authorized It

HSIP's audit and consent APIs previously tracked *that* an action happened, but not *what kind of principal* authorized it. In particular, `POST /v1/consent/grant` — the one endpoint whose entire purpose is representing authorization — recorded a consent grant identically whether a human operator called it or an `ai_agent` key called it to approve its own action. For a product whose stated purpose includes AI agent governance, that's a meaningful gap: "consent" that can't distinguish a person from the agent it's supposed to be governing is a weaker claim than it sounds.

Every consent grant/revoke now resolves the authenticated key's `agent_type` (`human` | `service` | `ai_agent`) and stores it on the row (`granted_by_key_type`) and in the audit entry (`granted_by=`/`revoked_by=` in `details`). This does not stop an `ai_agent` key from granting consent on its own behalf — that may be a legitimate use case — but it makes that fact visible and queryable instead of indistinguishable from a human-authorized grant.

**Scope limitation:** this is provenance, not a policy engine. HSIP does not (yet) let a tenant require that certain consent scopes can only be granted by a `human` key — it only records which kind of key did it. Enforcing that distinction, if wanted, is an application-layer decision on top of this field.

*Source: `crates/hsip-api/src/routes/consent.rs::resolve_granting_key_type`*

### 4.13 Master Key Rotation

Until this revision, the master key that encrypts every tenant's Ed25519 signing key (§4.1) had no rotation mechanism at all — it was loaded once at process startup and lived for the lifetime of the deployment. That's a real gap for any operator with a key-rotation compliance requirement, and it meant the master key was a security asset that, unlike tenant signing keys (`POST /v1/identity/rotate` has existed since the decision-attestation work), could only ever be replaced by re-provisioning the entire deployment from scratch.

`POST /v1/admin/master-key/rotate` re-encrypts every `identities.signing_key_b64` row and the singleton `anchor_identity` row under a freshly generated key inside one database transaction, then durably persists the new key to disk (staging file + `fsync` + atomic rename) and swaps it into the running process's memory — no restart, no downtime for other tenants' non-identity operations.

**Authorization:** gated to the bootstrap admin key specifically (see §5's note on the admin trust boundary below) — this is a system-wide operation touching every tenant's identity, not a tenant-scoped one.

**Residual risk:** a `state.master_key.write().await` lock is held for the entire rotation, serializing it against every concurrent signing/encryption operation — a brief, deliberate stop-the-world rather than a narrow, hard-to-reproduce corruption window. Separately, if the process crashes in the exact gap between the DB transaction committing and the key-file rename completing, the DB holds ciphertext under the new key while the on-disk file still has the old one; the staging file is deliberately left in place (not cleaned up) specifically so an operator can complete that rename manually rather than face silent, undetected data loss. This is the same category of residual risk this document already documents for decision-attestation anchoring (§ Decision Attestations trust model in `CLAUDE.md`) — narrow, acknowledged, and recoverable, not eliminated.

**Not yet covered:** rotation via the API is unavailable when the master key is sourced from `HSIP_MASTER_KEY` (no file this process can rewrite) — that path must be rotated wherever the env var's value is managed (e.g. the secrets manager), followed by a restart.

*Source: `crates/hsip-api/src/routes/admin.rs`*

---

## 5. Trust Boundaries

| Boundary | Trust level | Notes |
|---|---|---|
| HTTPS client → HSIP API | Authenticated | Valid bearer key required for all `/v1/*` endpoints |
| HSIP server → SQLite | Trusted | Database must not be publicly accessible |
| Master key storage → HSIP server | Critical | Compromise = all signing keys exposed |
| Tenant A ↔ Tenant B | Untrusted | Isolated by `tenant_id` on every query |
| Admin key holder (bootstrap tenant, key named `admin`) | Full — now including master key rotation | Can provision all tenant keys and, as of §4.13, rotate the node's master key (a system-wide operation touching every tenant's identity). This is a single global admin tied to the first tenant ever created, not a real RBAC system — protect this key like the master key itself. |
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
| **Master key loss** | If `master.key` is lost, all encrypted signing keys are permanently unrecoverable | Back up the master key (a startup warning now reminds you to); `POST /v1/admin/master-key/rotate` (§4.13) lets you *replace* a still-available key on a schedule, which reduces how long any one key's loss would matter, but does not help if the current key is already gone — that's still unrecoverable |
| **HSM-backed key storage** | Master key lives on the filesystem by default | Point `HSIP_MASTER_KEY` at a secrets manager (Vault, AWS KMS) — **this now actually works**; previously the only code path that read `HSIP_MASTER_KEY` was dead code nothing called, so this documented mitigation was not functional. Fixed — see `main.rs::load_master_key`. |
| **Post-quantum adversaries (current Ed25519)** | Ed25519 is not quantum-safe | ML-KEM-768 + ML-DSA-65 available via `hsip-verify` for environments requiring it |
| **Social engineering** | If an admin is phished, HSIP cannot detect it | Operational security, 2FA on the server, key rotation |

---

## 7. Residual Risks and Known Gaps

Documented openly. Tracked for the v1.0 audit milestone.

| Gap | Risk | Mitigation path |
|---|---|---|
| No third-party security audit | Medium | Planned before v1.0. Codebase is open source and auditable now. |
| **Single maintainer, no succession plan** | Medium | Relevant to any team evaluating HSIP for compliance-grade audit trails: patch timelines and long-term availability are best-effort, not contractual (see §9). No code fix for this — flagged here for buyer due diligence, not left implicit in the disclosure section alone. |
| Master key on filesystem (no HSM) | Medium | Use `HSIP_MASTER_KEY` env var + external secrets manager for production — now functional, see §6 |
| **Single global admin model** | Low-Medium | `routes::admin::require_root_admin` (§4.13) authorizes master key rotation to exactly one key: the one named `admin` in the first tenant ever created. There is no RBAC, no multiple admins, no scoped permissions — the bootstrap admin key is a de facto root credential for one system-wide operation today, and any future node-level operation would need the same treatment or a real permissions model. |
| **SQLite → PostgreSQL migration path is undocumented and untested** | Medium | `DATABASE_URL` pointing at PostgreSQL is supported by `db.rs`'s `AnyPool`, but no migration tooling or tested procedure exists for moving an existing SQLite deployment's data over. Treat a PostgreSQL deployment as a fresh install, not an upgrade, until this is tested. |
| **OpenTimestamps calendar submission unverified end-to-end** | Medium | `anchor.rs`'s HTTP client logic is unit-tested against a mock server only. Outbound HTTPS to `*.calendar.opentimestamps.org` has been confirmed blocked in every sandboxed environment this project has been developed in to date (including this session's, via a 403 on the CONNECT tunnel) — real-network submission has never been observed to complete. Verify from an unrestricted network before relying on it for compliance purposes. |
| In-memory rate limiter resets on restart | Low | A burst attack timed around a restart or deploy can temporarily exceed rate limits |
| SQLite without WAL under write contention | Low | Low risk for single-tenant deployments; use `DATABASE_URL` pointing at PostgreSQL for high-concurrency |
| No mutual TLS between federated HSIP nodes | Low | Federated trust uses explicit Ed25519 verify key registration; no automatic peer auth at the transport layer |
| Audit log hash chain not externally anchored | Low | The BLAKE3 chain (§4.8) is self-verifiable and now wired into the HTTP audit table, but — like decisions before anchoring — an attacker who deletes the whole chain leaves no internal trace; no blockchain or transparency log integration for the audit log yet (decisions already have one, see `CLAUDE.md`). |
| Clock skew affects consent, credential, *and* decision-chain ordering | Low | All three subsystems trust the server's wall clock for expiry/ordering, not just consent. Use NTP synchronization in production. |
| Post-quantum crypto (ML-KEM-768/ML-DSA-65) is not part of the default build | Informational, not a gap | These live in `hsip-verify`, excluded from the workspace (requires Z3 built from source — see `CLAUDE.md`). For HSIP's actual threat model — API-key theft, filesystem/DB compromise — this matters far less than the items above; don't let its prominence in dependency lists (§4.11) imply it's closer to production-ready than the excluded-crate status indicates. |

---

## 8. Audit and Review Status

| Item | Status |
|---|---|
| Third-party security audit | Not yet completed — planned for v1.0 |
| Formal verification of protocol properties | `hsip-verify` crate uses Z3 SMT solver for cryptographic protocol proofs. **Excluded from the workspace build and from `cargo test --workspace`** — its guarantees do not currently run in CI. |
| RFC compliance test vectors | RFC 8439 (ChaCha20-Poly1305), RFC 8032 (Ed25519) vectors pass in CI |
| Audit log hash chain integrity | Covered by `hsip-api/tests/integration.rs::test_audit_chain_verify_detects_valid_and_tampered_chains` — writes a chain, verifies it, then directly tampers with a row via SQL (simulating OS-level DB compromise) and confirms `GET /v1/audit/verify` detects it. |
| Master key rotation | Covered by `hsip-api/tests/integration.rs::test_master_key_rotation_reencrypts_and_swaps_live_key` — proves actual re-encryption (old key stops decrypting, the key now on disk decrypts), live in-memory key swap on the *same running process* (not just DB/file state), and rejection of a non-root-admin key. |
| Dependency vulnerability scanning | `cargo audit` runs on every build |
| Minimum supported Rust version | 1.88.0 |

---

## 9. Responsible Disclosure

If you find a vulnerability, please disclose it responsibly:

**Email:** sanchezleal1989@gmail.com  
**Subject:** `[HSIP SECURITY]`

**Response commitments:**
- Acknowledgement within 48 hours
- For critical issues: status update and documented mitigation steps within 7 days, or a message explaining why the timeline is delayed
- Researchers credited by name in release notes (or anonymously on request)

> Note: HSIP is currently maintained by a single developer. Patch timelines reflect best-effort availability, not an SLA. If you need a contractual response commitment, contact us before deploying in production.

HSIP does not currently have a bug bounty program.
