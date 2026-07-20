# HSIP Threat Model

**Version:** 0.9-draft  
**Date:** 2026-07-20  
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

### 4.3 Replay Prevention: Protocol Layer (UDP Sessions) and HTTP (Opt-In)

The `hsip-core` crate implements a 64-packet sliding window nonce tracker for the UDP consent and session protocol:

- Zero nonces are rejected unconditionally (`NonceError::ZeroNonce`)
- Any previously seen nonce is rejected (`NonceError::Replay`)
- Nonces more than 64 positions behind the current maximum are rejected (`NonceError::TooOld`)

*Source: `crates/hsip-core/src/nonce.rs`*

**HTTP REST API:** as of this revision, a caller can opt a request into replay protection by sending two headers:

- `x-hsip-timestamp` — Unix timestamp in seconds
- `x-hsip-nonce` — an opaque string, 1–128 characters, unique per request

If both are present, `auth.rs::check_replay_protection` rejects the request (401) unless the timestamp is within 5 minutes of server time and the `(key_id, nonce)` pair has not been seen before within that window. Deduplication is scoped per `key_id`, not global, so two different tenants can safely reuse the same nonce value. Nonces are tracked in an in-memory `DashMap` (`AppState.replay_nonces`), swept every 60s by a background task in `main.rs` to bound memory growth — entries live for 10 minutes (twice the tolerance window) before being eligible for sweep.

**This is opt-in, not mandatory** — a request with neither header behaves exactly as before this feature existed, and every existing SDK/CLI caller is unaffected until it's updated to send them. Sending only one of the two headers is a 400 (malformed), not silently ignored, so a caller can't accidentally think it's protected when it isn't.

**What this does and doesn't defend against:** a signature (or a bearer token) proves who sent a request; it says nothing about whether that exact request has been sent before. Without these headers, an attacker who captures a valid HTTP request (e.g. via a compromised proxy, logging pipeline, or MITM before TLS termination) can resend it verbatim until the key is revoked or the rate limiter kicks in (Section 4.6) — replaying it is otherwise indistinguishable from the original call. With these headers, that captured request stops working the moment its nonce is reused or its timestamp ages out of the window, without requiring a key rotation. This does **not** protect against a stolen API key being used for *new*, non-replayed requests — that's a key-theft problem, not a replay problem, and is out of scope for this mechanism (see the rate limiter and key expiry as the relevant mitigations there).

**Not yet adopted by HSIP's own SDKs/CLI/dashboard** — this pass wires the server-side check only; none of `sdks/python`, `sdks/node`, `sdks/go`, `hsip-cli`, or the dashboard send these headers yet. A caller who wants replay protection today constructs them directly.

*Source: `crates/hsip-api/src/auth.rs::check_replay_protection`, `crates/hsip-api/src/state.rs::ReplayNonceTracker`, `crates/hsip-api/src/main.rs` (sweep task)*

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

**Authorization:** gated to whichever key(s) hold the `is_root_admin` flag (see §4.14) — this is a system-wide operation touching every tenant's identity, not a tenant-scoped one.

**Residual risk:** a `state.master_key.write().await` lock is held for the entire rotation, serializing it against every concurrent signing/encryption operation — a brief, deliberate stop-the-world rather than a narrow, hard-to-reproduce corruption window. Separately, if the process crashes in the exact gap between the DB transaction committing and the key-file rename completing, the DB holds ciphertext under the new key while the on-disk file still has the old one; the staging file is deliberately left in place (not cleaned up) specifically so an operator can complete that rename manually rather than face silent, undetected data loss. This is the same category of residual risk this document already documents for decision-attestation anchoring (§ Decision Attestations trust model in `CLAUDE.md`) — narrow, acknowledged, and recoverable, not eliminated.

**`HSIP_MASTER_KEY`-sourced keys — `HSIP_ROTATION_HOOK`:** rotation no longer unconditionally refuses when the key has no file. If `HSIP_ROTATION_HOOK` names an executable, rotation invokes it with the new key hex-encoded on stdin (never as a process argument — those are visible via `ps` on some systems) and old/new fingerprints as env vars for context; the hook's exit code is the only signal trusted. **HSIP never holds Vault/AWS/etc. credentials itself for this** — deliberately: the alternative (embedding a specific vendor's SDK and letting the HSIP process authenticate directly to a secrets manager) was considered and rejected, because it would give this process a new secret to protect on top of the master key itself, and would lock the feature to one vendor. The hook is the operator's own trusted tooling, using whatever credentials it already has. If the hook fails or times out (30s), the DB transaction is still uncommitted at that point and rolls back — no partial rotation. If no `master_key_path` *and* no `HSIP_ROTATION_HOOK`, rotation still refuses with an actionable error. Proven by `test_master_key_rotation_hook_for_env_sourced_key`, which specifically exercises the failure path: a hook that exits non-zero must leave the database completely untouched, not partially re-encrypted — and was additionally verified against a real running server with both `HSIP_MASTER_KEY` and `HSIP_ROTATION_HOOK` set.

**Verifying a backup without rotating:** `GET /v1/admin/master-key/fingerprint` (same admin gate, no mutation) returns the current key's SHA-256 fingerprint on demand. Before this existed, confirming "does my backup file actually match what's running in production" required either grepping the startup log or triggering an actual rotation — not something you want as the only way to audit a backup.

**Reachable by non-technical operators, not just API calls:** `hsip keys master-fingerprint` and `hsip keys rotate-master` (the latter requires typing `yes` at an interactive prompt, or `--yes` for scripted use) — HSIP's original design point was non-technical users, and a security control only reachable via hand-rolled `curl` with bearer-token auth is a control most of that audience would never actually use.

*Source: `crates/hsip-api/src/routes/admin.rs`, `crates/hsip-cli/src/commands/keys.rs`*

### 4.14 RBAC: Tenant-Scoped Roles and Root-Admin Grants

Auditing the model above surfaced two gaps at once, both closed this revision: (1) within a tenant, *any* active key — including a low-privilege `ai_agent` key — could mint new `human` keys or revoke *any* other key in the same tenant, including the tenant's own admin key, with zero check; (2) node-level "root admin" (§4.13's gate) was a single hardcoded credential — `name == "admin"` in the first tenant ever created — with no way to add a second one short of editing the database by hand.

**Tenant-scoped (`api_keys.role`, `'owner' | 'member'`):** `POST`/`DELETE /v1/keys*` now requires the caller's own key to be `role='owner'`; a `member` key (the default for anything created via `POST /v1/keys` unless the caller explicitly requests `role: "owner"`) can neither create nor revoke keys, including itself. `GET /v1/keys` (list) stays open to any active tenant key — informational only. `revoke` additionally refuses (`409`) to remove a tenant's *last* remaining active `owner` — otherwise the tenant becomes permanently unable to manage its own keys, including recovering from that exact mistake.

**Node-level (`api_keys.is_root_admin`, `0 | 1`):** replaces the `name == "admin"` heuristic with an explicit, grantable flag, not tied to any tenant. `POST /v1/admin/root-admins/grant` / `.../revoke` (root-admin-only) let an existing root admin add or remove the flag on any active key, by id, in any tenant — the mechanism that makes more than one root admin possible at all. `revoke` refuses (`409`) if it would leave zero root admins on the node — the equivalent lockout guard to the tenant-level one above, since there is no recovery path from zero root admins except editing the database directly. `GET /v1/admin/root-admins` (root-admin-gated) lists who currently holds the flag.

**Migration / upgrade path:** fresh installs get `role='owner'`/`is_root_admin=1` set explicitly on the bootstrap key's `INSERT` (`main.rs::bootstrap_admin`) and `role='owner'` on each sandbox trial tenant's sole key (`routes::sandbox::provision`) — not left to the backfill below, since a row created *after* migrations run in the same boot was never touched by them. Upgraded (pre-existing) databases get a one-time backfill in `db.rs`: the earliest-created key in each tenant becomes `'owner'`, every other still-unset key becomes `'member'`; the key named `admin` in the very first tenant ever created — the exact key §4.13's old gate already trusted — becomes a root admin, so nobody loses admin access across the upgrade.

**Audit trail:** `POST`/`DELETE /v1/keys*` now write `key.created`/`key.revoked` audit entries — both were previously state-changing operations with no audit trail at all, a gap independent of but found alongside this work. Grant/revoke write `admin.root_admin_granted`/`admin.root_admin_revoked` on the target key's own tenant. `metrics::ROOT_ADMIN_CHANGES{action}` covers both.

**Still not a full RBAC system, by design:** `is_root_admin` is one flat capability covering every node-level operation (no "can rotate but not grant," no per-operation scoping); `role` is a two-tier owner/member split, not fine-grained per-action permissions ("can create `ai_agent` keys but not `human` keys" isn't expressible). Both are sized to what HSIP's current operations actually need — two node-level operations sharing one gate, one tenant-level capability (key management) sharing one gate — not built out speculatively ahead of a second *kind* of operation that would need finer scoping. Revisit with real scoped grants only when one shows up.

*Source: `crates/hsip-api/src/routes/keys.rs`, `crates/hsip-api/src/routes/admin.rs`, `crates/hsip-api/src/db.rs`, `crates/hsip-cli/src/commands/keys.rs`*

---

## 5. Trust Boundaries

| Boundary | Trust level | Notes |
|---|---|---|
| HTTPS client → HSIP API | Authenticated | Valid bearer key required for all `/v1/*` endpoints |
| HSIP server → SQLite | Trusted | Database must not be publicly accessible |
| Master key storage → HSIP server | Critical | Compromise = all signing keys exposed |
| Tenant A ↔ Tenant B | Untrusted | Isolated by `tenant_id` on every query |
| Root-admin key holder (`is_root_admin=1`, any tenant) | Full node-level — master key rotation + granting/revoking root-admin on other keys | As of §4.13/§4.14, can rotate the node's master key (a system-wide operation touching every tenant's identity) and grant/revoke the flag itself. No longer tied to a single hardcoded key — the bootstrap admin key starts with it, but any root admin can add more. Still one flat capability, not a real RBAC system — protect every root-admin key like the master key itself. |
| Owner-role key holder (`role='owner'`, own tenant) | Full within its own tenant's key management | Can create and revoke keys — including granting `owner` to others — in its own tenant. Cannot see or touch another tenant's keys, and cannot reach node-level operations without also holding `is_root_admin`. |
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
| **Master key loss** | If `master.key` is lost, all encrypted signing keys are permanently unrecoverable | Back up the master key (a startup warning now reminds you to); `GET /v1/admin/master-key/fingerprint` / `hsip keys master-fingerprint` lets you *verify* that backup actually matches production without exposing or rotating the key; `POST /v1/admin/master-key/rotate` (§4.13) lets you *replace* a still-available key on a schedule, which reduces how long any one key's loss would matter, but does not help if the current key is already gone — that's still unrecoverable |
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
| Master key on filesystem (no HSM) | Medium | Use `HSIP_MASTER_KEY` env var + external secrets manager for production — now functional, see §6. Rotation of an env-var-sourced key can now be automated via `HSIP_ROTATION_HOOK` (§4.13) instead of being purely manual. |
| **Rotation hook execution is a new trust boundary** | Low | `HSIP_ROTATION_HOOK` runs an operator-configured executable with the new master key on stdin. The path is only ever read from server-side environment configuration, never from an HTTP request — no caller can influence which script runs or with what arguments. The operator who sets this env var is implicitly trusting that script already, the same way `config.toml` or `master_key_path` are already-trusted operator-supplied paths; HSIP does not sandbox or validate the hook's contents. |
| **Flat RBAC model, not scoped permissions** | Low | §4.14 closed the "single hardcoded root-admin credential" gap (multiple root admins can now exist via grant/revoke) and the "any key can manage any other key in its tenant" gap (now `role='owner'`-gated). What remains by design: `is_root_admin` is one flat capability covering every node-level operation (no "can rotate but not grant"), and `role` is a two-tier split, not per-action scoped permissions. Revisit only when an operation shows up that genuinely needs finer scoping than that. |
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
| Master key rotation | Covered by `hsip-api/tests/integration.rs::test_master_key_rotation_reencrypts_and_swaps_live_key` — proves actual re-encryption (old key stops decrypting, the key now on disk decrypts), live in-memory key swap on the *same running process* (not just DB/file state), and rejection of a non-root-admin key. Also manually verified end-to-end against a running server: real `hsip keys rotate-master`/`master-fingerprint` CLI invocations, confirming fingerprints change, the key file on disk is rewritten, and `POST /v1/messages/sign` keeps working transparently across the rotation. |
| `HSIP_ROTATION_HOOK` rotation | Covered by `test_master_key_rotation_hook_for_env_sourced_key` (Unix-only) — proves refusal with no hook configured, a succeeding hook receives exactly the new key and the DB genuinely re-encrypts, and — the safety-critical case — a *failing* hook (non-zero exit) leaves the database completely untouched rather than partially rotated. Also manually verified end-to-end against a running server with `HSIP_MASTER_KEY` and `HSIP_ROTATION_HOOK` both set, with the hook's output independently re-hashed and confirmed to match the reported fingerprint. |
| Master key fingerprint endpoint | Covered by `test_master_key_fingerprint_is_read_only_and_admin_gated` — proves it's idempotent (repeated calls return the identical fingerprint, i.e. no mutation) and rejects a non-root-admin key. |
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
