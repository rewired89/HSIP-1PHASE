# HSIP Threat Model

**Version:** 0.21-draft  
**Date:** 2026-07-21  
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

**Velocity counters survive a restart.** `AppState.agent_tracker` is in-memory (atomics in a `DashMap`, for hot-path speed), but is periodically snapshotted to the `rate_limit_state` table and restored at startup — see §4.6's persistence note below, which applies identically here since both trackers share the same mechanism.

*Source: `crates/hsip-api/src/auth.rs` — `check_agent_velocity()`*

### 4.6 Per-Key Rate Limiting

Every API key (not only agent keys) is subject to a sliding-window rate limit of 300 requests/minute by default (configurable via `RATE_LIMIT_RPM` environment variable). Enforcement uses an in-memory `DashMap<key_id, RateWindow>` — the limit is on the key identity, not the client IP, so it cannot be bypassed by IP rotation or multiplexed connections.

**Persisted across restarts.** Previously entirely in-memory: a restart (crash, deploy, container reschedule) silently reset every key's count to zero, and the same was true of `agent_tracker`'s velocity/anomaly counters (§4.5) and the sandbox provisioning IP limiter (§4.2-adjacent, `routes::sandbox`). `rate_limit_persistence::snapshot` now upserts the current contents of all three trackers to a `rate_limit_state` table every 30s; `rate_limit_persistence::load` restores live (non-expired) windows at startup, before the server accepts traffic. This is a periodic snapshot, not a write-through on every request — adding a DB write to the hot auth path on every single request was rejected as a disproportionate cost for what this closes. **Residual risk:** state since the last snapshot (up to 30s) is still lost on a crash or unclean restart; bounded, not eliminated, the same tradeoff already accepted elsewhere in this document (master-key rotation's staging-file window, decision attestations' signing-to-anchoring gap).

*Source: `crates/hsip-api/src/auth.rs` — `check_rate_limit()`; `crates/hsip-api/src/rate_limit_persistence.rs`*

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
- **This proves tamper-*evidence*, not tamper-*prevention*.** An attacker with DB write access can still delete the *entire* chain (not just alter it undetected). As of this revision that gap is closed the same way as decision attestations — see §4.8.1 below — but only going forward from when a batch is actually anchored; a chain deleted *before* its next anchor cycle runs is still undetectable (bounded by the anchor cadence, `INTERVAL_TRIGGER_MS` = 5 min, same residual window decisions already accept).
- For environments requiring protection against total chain deletion even within that window, the mitigation is still filesystem-level immutability (append-only S3 bucket, WORM storage, or periodic signed export to an external log sink) — the hash chain detects alteration of what remains, anchoring bounds how long full deletion goes undetected, neither makes deletion impossible.

#### 4.8.1 External Anchoring (`GET /v1/audit/:id/proof`, `POST /v1/audit/verify-proof`)

Closes the gap §4.8 documented above: the BLAKE3 chain alone can't prove the whole chain wasn't deleted and silently recreated by whoever controls this database, only that what remains is internally consistent. `anchor_job::run_audit_anchor_cycle` batches `audit_entries` (by `entry_hash`, excluding pre-chain rows that have none) into an RFC 6962 Merkle tree, signs the root with the same node-level `anchor_identity` key used for decision attestations (one node identity vouches for everything this node anchors — anchoring was never decision-specific), and submits it to OpenTimestamps — the identical mechanism as § Decision Attestations below, applied to `audit_entries` instead of `decisions`. Same cadence (`BATCH_SIZE_TRIGGER` = 50 / `INTERVAL_TRIGGER_MS` = 5 min), same `calendar_unreachable` retry-next-cycle behavior, stored in a twin `audit_anchors` table.

`GET /v1/audit/:id/proof` returns the same self-contained bundle shape as `GET /v1/decisions/:id/proof`; `POST /v1/audit/verify-proof` is the same "no `TenantId`, no `State`, no DB call" pure verification function as `POST /v1/decisions/verify` — a third party recomputes `entry_hash` from the disclosed fields, checks Merkle inclusion, checks the anchor signature, with zero trust required in this server's account of its own database. Verified end-to-end against a real running server (not just the mocked-calendar unit tests): entry recorded → proof shows `anchored: false` → anchor cycle run against local mock (real public calendars are still sandbox-blocked, see §7) → proof shows `anchored: true` with a verifying Merkle proof and anchor signature → tampering any disclosed field flips `verify-proof`'s `valid` to `false`.

*Source: `crates/hsip-api/src/anchor_job.rs::run_audit_anchor_cycle`, `crates/hsip-api/src/routes/audit.rs::{proof, verify_proof}`, `crates/hsip-api/src/db.rs` (`audit_anchors` table, `audit_entries.anchor_id`/`merkle_index`)*

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

### 4.15 Mutual TLS (`[server.tls] client_ca_path`)

HSIP has no dedicated node-to-node network protocol as of this writing — federated trust (§ Federated Trust in `CLAUDE.md`) is offline Ed25519 verify-key registration plus local signature verification, not a live channel between HSIP instances. The gap previously documented here (§7's residual-risk table: "no automatic peer auth at the transport layer") was nonetheless real and generic: HSIP's own HTTPS server, when `[server.tls]` is configured, only ever authenticated itself to connecting clients, never the reverse. Any TLS client holding a valid bearer token could connect with nothing enforced at the transport layer.

`client_ca_path`, when set to a CA certificate file, closes that: the server now requires and verifies a client certificate chaining to that CA before the TLS handshake completes, on top of (not instead of) the bearer-token auth every request still goes through afterward. This is what an operator running multiple HSIP nodes — or a partner/regulator's system — that connect to each other's APIs over HTTPS would configure identically on both ends to authenticate each other at the transport layer. It is not restricted to HSIP-to-HSIP traffic specifically, since no such specific protocol exists to scope it to; it is the direct, generic fix for the gap as documented.

**Fully opt-in, backward compatible:** `client_ca_path: None` (the default, and the only option before this revision) takes the exact same `RustlsConfig::from_pem_file` code path as before — byte-for-byte unchanged. No existing TLS-enabled deployment is affected until an operator deliberately sets the new option.

**A pre-existing latent panic, found and fixed along the way:** `reqwest` (this process's HTTP client, used for OpenTimestamps submission) and `axum-server` (this process's TLS server) enable *different* rustls crypto-provider features — `ring` and `aws-lc-rs` respectively. Both end up compiled into the same binary, so rustls cannot auto-select a default provider, and the first `ServerConfig`/`ClientConfig` builder call anywhere in the process would panic with "Could not automatically determine the process-level CryptoProvider." This was not introduced by this work — the pre-existing plain-TLS code path already called the same underlying builder internally, and would have panicked the moment any deployment actually enabled `[server.tls]` in production. It had simply never been exercised by any existing test or by this project's dev/desktop-mode-only environment. Fixed by explicitly installing the `aws-lc-rs` provider as the process-wide default at the top of `main()`, before any TLS code runs.

**Operational gotcha, discovered during verification:** client certificates must carry the `clientAuth` Extended Key Usage extension (OID 1.3.6.1.5.5.7.3.2) or the handshake fails with a "certificate unknown" TLS alert, even when the certificate correctly chains to the configured trusted CA — `rustls-webpki` enforces EKU for client certificates. A first attempt at end-to-end verification, using a quick `openssl req -x509 -newkey ...` test certificate with no extensions, reproduced exactly this failure; documented in `config.example.toml` so operators don't have to rediscover it.

**Verified:** unit tests (`mtls.rs`) build real X.509 certificates via the system `openssl` CLI (not mocked bytes) and confirm the CA-loading/verifier-building logic accepts valid input and rejects garbage. Full end-to-end verification against a real running server in server mode (not just unit tests): a client certificate signed by the configured CA, with the `clientAuth` EKU, connects successfully and receives a genuine `200 {"status":"ok",...}` response; a certificate signed by a *different*, untrusted CA, and a connection attempt presenting no client certificate at all, are both rejected — confirmed via `curl`'s verbose TLS trace that rejection happens during the TLS handshake itself (the client never reaches the point of successfully sending an HTTP request), not as an application-layer 401/403.

*Source: `crates/hsip-api/src/mtls.rs`, `crates/hsip-api/src/config.rs` (`TlsConfig::client_ca_path`), `crates/hsip-api/src/main.rs` (crypto provider install + TLS branch)*

---

### 4.16 SQLite → PostgreSQL Migration Tooling (`hsip-migrate`), and Two Bugs That Meant PostgreSQL Had Never Actually Worked

§7's residual-risk table previously described this gap as "no migration tooling exists yet, but `DATABASE_URL` pointing at PostgreSQL is supported for a fresh install." Investigating that claim while building the migration tool disproved it: **no HSIP deployment, fresh or migrated, had ever actually worked against real PostgreSQL.** Two independent bugs, both invisible in this project's development environment because it had only ever run against SQLite:

**Bug 1 — integer overflow on every timestamp.** `db.rs`'s schema declared every millisecond-epoch column (`created_at`, `timestamp`, `expires_at`, etc.) as `INTEGER`. SQLite's only integer keyword is dynamically typed up to 8 bytes, so this was never a problem there — but PostgreSQL's `INTEGER` is a real 4-byte `int4` (max ~2.1×10⁹), and a real epoch-ms timestamp (~1.7×10¹² as of this writing) overflows it by three orders of magnitude. Confirmed directly against a live `psql` session before touching any code: `CREATE TABLE t (created_at INTEGER); INSERT INTO t VALUES (1774038000000);` → `ERROR: integer out of range`. Every `CREATE TABLE` in `db.rs` succeeded regardless (DDL doesn't touch the value), so a Postgres-backed HSIP looked like it started up fine right up until the first write of a real timestamp — which is every single state-changing request. Fixed by widening every millisecond-epoch or similarly-wide column to `BIGINT` (identical storage on SQLite, correct 8-byte `int8` on Postgres); small bounded values (0/1 flags, in-batch Merkle indices) were left as `INTEGER`. A companion bug: `uploads.data` and `*_anchors.ots_proof` used the SQLite/MySQL-only `BLOB` keyword, which doesn't exist in PostgreSQL at all (`ERROR: type "blob" does not exist`) — fixed by using `BYTEA`, which both backends accept (SQLite assigns it NUMERIC type affinity, but affinity conversion never touches an actual BLOB-typed bound value, so storage is unaffected).

**Bug 2 — every parameterized query was a Postgres syntax error.** HSIP uses `sqlx::Any` specifically so the same SQL runs against either backend, and every one of the ~150 parameterized queries in this codebase used `?` placeholders (SQLite/MySQL style). `sqlx::Any` does not rewrite placeholder syntax per backend — that turned out to be a hard requirement discovered only by testing against a live Postgres server, not documented anywhere prominent in `sqlx`'s own docs for the `Any` driver. `?` is not valid PostgreSQL syntax outside a string literal; every single `INSERT`/`UPDATE`/`SELECT ... WHERE x = ?` in this codebase would have failed with `ERROR: syntax error at or near ","` (Postgres's error points at the first bind-value separator after the rejected `?` token) the moment it reached a real Postgres backend — meaning the bootstrap-admin-key creation on first boot would have been the very first failure, before any actual API traffic. Fixed by rewriting every `?` in every `sqlx::query(...)` call to PostgreSQL-style numbered placeholders (`$1, $2, ...`) — confirmed empirically that SQLite accepts `$N` placeholders identically to `?` (same positional bind-order semantics), so this single rewrite works unchanged on both backends with no runtime backend detection required. Mechanical, scripted rewrite across 19 files / 100 statements, verified by full workspace test suite pass (no `.bind()` count or order was affected — only the placeholder token) plus the live end-to-end tests below.

**`hsip-migrate` (new binary, `crates/hsip-api/src/bin/hsip_migrate.rs`):** `hsip-migrate --from sqlite:<path> --to postgresql://...` connects to both, creates the target schema by calling the exact same `db::run_migrations` the server itself runs at startup (so the target can never drift from what the running server expects), refuses to proceed into a non-empty target without `--force`, copies every table's rows inside one target-side transaction, then verifies row counts match on both sides post-copy. Never writes to the source database.

**Verified end-to-end, not just unit-tested:** a real PostgreSQL 16 instance was installed and run in this development sandbox specifically to test this (previously untested in any environment this project has been developed in, per the residual-risk table this section replaces). Full round trip: started a real `hsip-api` server against SQLite, created an identity, signed messages, granted consent, added a contact, recorded a decision attestation, created a second API key (7 audit entries, hash chain valid) → ran `hsip-migrate` against the populated database → started a **fresh** `hsip-api` process pointed at the migrated PostgreSQL database, using the *original* master key and admin key files → confirmed: the same tenant ID and Ed25519 verify key were returned (proving the encrypted signing key round-tripped through migration and still decrypts under the same master key), the original admin bearer token still authenticated, `GET /v1/audit/verify` reported all 9 entries (7 migrated + 2 new) as a valid unbroken hash chain, and the background anchor job successfully ran against Postgres — writing to the `anchor_identity` singleton table (`INTEGER PRIMARY KEY`) and a `decision_anchors` row (`BYTEA ots_proof`) — anchoring the migrated decision attestation into a real Merkle batch.

A lightweight, `#[ignore]`-by-default regression test (`crates/hsip-api/tests/postgres_compat.rs`, run explicitly against `HSIP_TEST_POSTGRES_URL`) locks in both fixes: it confirms `run_migrations` succeeds and that a real epoch-ms timestamp and a `BYTEA` blob both round-trip correctly. Not part of the default `cargo test --workspace` run, matching the rest of this codebase's tests, which are all SQLite-only with no live external dependency.

*Source: `crates/hsip-api/src/db.rs` (schema types, placeholder rewrite), `crates/hsip-api/src/bin/hsip_migrate.rs`, `crates/hsip-api/tests/postgres_compat.rs`*

---

### 4.17 Formal Verification Now Runs by Default (`hsip-verify` in the Workspace)

`hsip-verify` — Z3 SMT-solver proofs of consent non-forgery, temporal consistency, and identity-binding soundness (see its own `README.md` for the formal specifications) — was previously excluded from the root `Cargo.toml`'s workspace `members`, because `z3-sys` builds the actual Z3 C++ solver from source on first compile. That meant its guarantees never ran as part of `cargo build --workspace` / `cargo test --workspace`, including in CI, unless someone remembered to run `cargo build -p hsip-verify` separately — which, per §8's audit table before this revision, nothing did.

It's now a normal workspace member. No code inside `hsip-verify` changed (only whitespace, via `cargo fmt`, applied now that it's subject to this repo's formatting check for the first time) — the only actual blocker was environmental, not architectural: `cargo build -p hsip-verify` already worked standalone before this. This session's sandbox ran out of disk space partway through compiling Z3 the first time this was attempted, traced to a stale, redundant `crates/hsip-verify/target/` directory left over from building it outside the workspace — deleting it freed enough space for a clean `cargo build --workspace` to succeed with no dependency-resolution conflicts against the rest of the workspace.

**Tradeoff accepted, not eliminated:** a from-scratch `cargo build --workspace` now costs roughly 8 extra minutes compiling Z3 from source via `cmake`. One-time per clean `target/` directory — Cargo caches the result like any other dependency, so this doesn't recur on incremental builds or CI cache hits. Requires `cmake` and a C++ toolchain wherever this workspace is built, which this workspace's other native dependencies (e.g. `curve25519-dalek`'s optional asm backends) already implicitly required.

**Verified:** `cargo build --workspace` and `cargo test --workspace` both succeed with `hsip-verify` included — its 9 unit tests plus 10 integration tests (`tests/verification_tests.rs`) run real Z3 SMT queries, not mocked, alongside every other crate's tests.

**Found and corrected while reviewing this change, unrelated to it:** this document previously misattributed HSIP's post-quantum crypto (ML-KEM-768/ML-DSA-65) to `hsip-verify` — the real implementation is `hsip-core::pqc` (a Cargo feature, on by default), and `hsip-core` has always been a normal workspace member. Corrected in §4 and §7's residual-risk table above.

*Source: root `Cargo.toml` (`members`), `crates/hsip-verify/`*

---

### 4.18 Dashboard UI Gaps Closed, and a Federated-Trust Table That Was Never Created

Five dashboard gaps tracked in CLAUDE.md's roadmap are closed: a Trust page (add/list/remove a federated-trust peer, verify a signature by peer label), a Discover page (one-click registration for detected local AI agents), an Admin page (master-key fingerprint/rotate, root-admin list/grant/revoke), a `role` column and selector on the Keys page, and a hash-chain-intact/broken indicator on the Audit page. All Expert-mode-only, matching where comparably sensitive existing pages (Credentials, Keys, Audit) already live — none of this is exposed to the consumer-facing Simple mode.

Building the Trust page surfaced a real, previously-undiscovered bug, not a UI gap: **`trusted_peers` — the table every handler in `routes/trust.rs` queries — was never created by `db::run_migrations()`.** Federated trust (§ Federated Trust in `CLAUDE.md`) has been documented, routed, and reachable via both the CLI (`hsip trust add/list/remove/verify`) and the API since it shipped, but every one of those calls has 500'd with "no such table" on any database created since — nothing had ever exercised the full add → list → verify → remove lifecycle against a real, freshly-migrated database until the new dashboard page did. This means the "Trusted peer (federated trust)" row in §5's trust-boundaries table below, and the "no live channel between HSIP instances, federated trust is offline key registration" framing in §4.15, described a mechanism that could not actually be used — not a working mitigation with caveats, a non-functional one.

**Fixed:** added the missing table (plus its tenant index) to `db::run_migrations`, and added `trusted_peers` to `bin/hsip_migrate.rs`'s `TABLES` list per the invariant that governs it (a table present in the schema but missing from that list silently isn't migrated — this bug is exactly why that invariant exists). Zero existing databases had usable `trusted_peers` data to lose, since no write to the table could ever have succeeded.

**Verified, not just patched:** a new integration test, `tests/integration.rs::test_trust_add_list_verify_remove`, exercises the full lifecycle over the real HTTP stack — add a peer, confirm it lists, verify a real Ed25519 signature (both the valid and deliberately-tampered case), then remove it — where zero trust-route tests existed before. Also verified end-to-end against a real running server: signed into the dashboard, added a real trust peer through the fixed backend, confirmed it persisted and listed correctly, and confirmed the Discover/Audit/Keys/Admin pages all render live data with zero failed `/v1/*` requests (checked via full browser automation, not just visual inspection).

*Source: `crates/hsip-api/src/db.rs`, `crates/hsip-api/src/bin/hsip_migrate.rs`, `crates/hsip-api/tests/integration.rs`, `dashboard/src/App.jsx`, `dashboard/src/pages/{Trust,Discover,Admin,Keys,Audit}.jsx`*

---

### 4.19 Security Self-Review: Three Confirmed Findings, All Fixed

A full-codebase security self-review — not a third-party audit (the author built the features under review; §7/§8 still list an independent audit as not yet done) — covering every route handler, the auth extractor, key-encryption code, admin/rotation code, the CLI, and the MCP server. Combined manual reading of the highest-risk files (`auth.rs`, `key_encryption.rs`, `admin.rs`, `nonce.rs`) with a parallel sweep of the remaining route/CLI surface, then independently re-verified every candidate against actual running-server or running-process behavior before treating it as confirmed — the same "exploit, don't just read" bar this document's other §4.x entries hold themselves to. Three findings met that bar; all three are fixed.

**Finding 1 — HIGH — stored XSS via SVG upload (`routes/uploads.rs`).** `POST /v1/uploads` accepted any `content_type` starting with `"image/"`, including `image/svg+xml`. SVG is XML and can carry a `<script>` element or an event-handler attribute (`onload=`, etc.) that a browser executes — and `GET /v1/uploads/:id` (deliberately public, no auth, by design so a recipient can open the URL directly — see its own doc comment) echoes the stored `content_type` back verbatim as the response header, so navigating to that URL runs the attacker's script in the visiting browser's origin. Confirmed exploitable directly: crafted an SVG with an `onload` payload, uploaded it with a forged `Content-Type: image/svg+xml`, and confirmed the server accepted and served it back unchanged before the fix. Because the dashboard SPA is served same-origin as the API in `embed-dashboard` builds and stores its bearer token in `localStorage` (`dashboard/src/App.jsx`), this was a path to session-token theft, not just a defaced page. **Fixed:** `upload()` now rejects `image/svg+xml` explicitly even though it matches the `image/*` prefix (every other accepted image format is not script-capable, so this one exclusion closes the class without narrowing "image" further); `serve()` additionally now sends `x-content-type-options: nosniff` as defense-in-depth, in case a stored file's declared type ever doesn't match its actual bytes. Re-verified after the fix: the same SVG payload is now rejected with a clear error, and a legitimate PNG upload still succeeds and now carries the `nosniff` header.

**Finding 2 — HIGH — missing authentication on all `/v1/proxy/*` endpoints (`routes/proxy.rs`).** All five handlers (`status`, `enable`, `disable`, `log`, `setup`) took only `State(state): State<AppState>` — no `TenantId` extractor — and this codebase has no global auth middleware layer (`main.rs` only applies CORS, a body-size limit, and request-ID layers; confirmed via `grep -n "\.layer("`). Every protected route in HSIP enforces auth individually by including the `TenantId` extractor in its handler signature (see `dns.rs`'s equivalent handlers for the correct pattern); `proxy.rs` simply never had it. Confirmed exploitable directly: unauthenticated `curl` (no `Authorization` header) against `GET /v1/proxy/log`, `GET /v1/proxy/status`, and `POST /v1/proxy/enable` all returned `200` before the fix — meaning anyone who could reach the HSIP port could enable/disable the local MITM traffic proxy and read its full captured-traffic log with zero credentials. **Fixed:** all five handlers now take `_tenant: TenantId` as their first parameter, same as every other authenticated route (not tenant-scoped beyond authentication, matching `dns.rs`'s reasoning — the proxy is a single node-level resource, not per-tenant data, so any valid key suffices). Re-verified: the same three endpoints now return `401` without a bearer token and `200` with a valid one.

**Finding 3 — MEDIUM — hardcoded fallback signing key in the local identity broker (`hsip-cli/src/identity.rs`).** `HSIP_KEY`, the HMAC-SHA256 key `hsip identity-serve`'s `/token` endpoint signs its JWTs with, fell back to a fixed hex string (`00112233...`) — checked into this public repository — whenever `HSIP_LOCAL_JWT_KEY_HEX` wasn't set. `/token` itself requires no authentication and accepts any caller-supplied `aud`, so anyone who had ever read this source could forge a valid token for any relying party still trusting an unconfigured local broker, without ever calling the broker at all. Confirmed the underlying mechanism directly: ran two separate broker processes with no key configured and confirmed both signed byte-identical tokens for identical claims (i.e., a key known in advance from source, not derived per-deployment). **Fixed:** unset `HSIP_LOCAL_JWT_KEY_HEX` now falls back to a fresh 32-byte key generated via `rand::rngs::OsRng` per process, rather than a fixed known value — an unconfigured broker is now merely unpredictable (acceptable for the local single-user demo flow this file's own `/demo` page describes), not a publicly known secret. Re-verified: two fresh broker processes with no key configured now produce different signatures for identical claims; an explicit `HSIP_LOCAL_JWT_KEY_HEX` override still produces a stable, deterministic signature across restarts as intended.

**Not flagged, considered and ruled out:** `credentials.rs::verify`'s revocation lookup by credential ID alone (without a `tenant_id` filter) is an intentional, already-documented design choice — the caller already possesses the credential being verified, so there's no cross-tenant data disclosure, only a shared revocation-status check against a value the caller supplied themselves. `routes/decisions.rs::verify`'s lack of anchor-key pinning is inherent to its documented purpose as a pure, third-party-runnable verification function — the caller is expected to independently know which key they trust, same as any raw signature-verification API. `hsip-mcp`'s `urlenc` percent-encodes a Unicode code point rather than UTF-8 bytes (a real correctness bug for non-ASCII input) but its only call site is always base64 (ASCII-only), so it isn't reachable — a correctness note, not a security finding.

*Source: `crates/hsip-api/src/routes/uploads.rs`, `crates/hsip-api/src/routes/proxy.rs`, `crates/hsip-cli/src/identity.rs`*

---

### 4.20 OpenTimestamps Calendar Submission Verified Against Real Calendars — Previously Blocked in Every Sandbox

Every prior attempt to verify `anchor.rs` against real OpenTimestamps calendars (§4.16's residual-risk table entry, §7) had run inside a sandboxed development environment whose egress proxy blocked outbound HTTPS to arbitrary hosts by policy — confirmed each time via an explicit `403` on the CONNECT tunnel to `alice.btc.calendar.opentimestamps.org` and `bob.btc.calendar.opentimestamps.org`, logged by the sandbox's own proxy as a policy denial, not a transient failure. That meant `anchor.rs`'s HTTP client logic had only ever been exercised against a `wiremock`-mocked calendar (`tests/anchor.rs`'s unit tests) — real submission had never been observed to complete, end to end, in this project's history.

Verified for the first time from an actual unrestricted network: the repo owner ran a real `hsip-api` server (`cargo run -p hsip-api --bin hsip-api`, desktop mode, Windows 11) outside any sandbox, from their own machine.

**Step 1 — raw connectivity, independent of any HSIP code:** a direct `Invoke-WebRequest POST <calendar>/digest` (the exact OpenTimestamps calendar protocol `anchor.rs` uses — `Content-Type: application/x-www-form-urlencoded`, raw 32-byte digest body) against all three of `DEFAULT_CALENDARS` returned real `HTTP 200` responses with real, nonzero-length bodies:

| Calendar | Result |
|---|---|
| `alice.btc.calendar.opentimestamps.org` | `HTTP 200`, 137 bytes |
| `bob.btc.calendar.opentimestamps.org` | `HTTP 200`, 135 bytes |
| `finney.calendar.eternitywall.com` | `HTTP 200`, 121 bytes |

**Step 2 — the full HSIP pipeline, not just raw connectivity:** recorded a real decision via `POST /v1/decisions` against the running server, then let `anchor_job.rs`'s background loop pick it up (fires on its very first cycle when no prior anchor exists yet in the DB, rather than waiting the full `INTERVAL_TRIGGER_MS`). The server's own log confirmed a real, successful anchor:

```
INFO hsip_api: anchored decision batch anchor_id=a66ff9bf-01ad-4961-9a82-7f8558a0b729 leaf_count=1 ots_status=pending
```

`GET /v1/decisions/:id/proof` returned `"anchored": true`, `"ots_status": "pending"` (the correct success state for this MVP — `anchor.rs` doesn't parse the `.ots` format or poll for Bitcoin confirmation yet, so "pending" is what a working submission looks like, not a failure), and a real `ots_proof` blob. Decoding those base64 bytes independently confirmed it is a genuine calendar response, not a placeholder: the raw bytes literally contain `https://alice.btc.calendar.opentimestamps.org` (the calendar's own URL, embedded in its response) and the byte count — 137 — is the exact same figure Step 1's raw request against that same calendar returned. This rules out a stub, a cached mock, or a same-length coincidence.

**What this closes, and what's still open:** this confirms the submission half of §4.16's residual-risk table entry — calendar submission genuinely works, end to end, over a real network, through the real code path (not just the mocked unit tests). What §4.16/§7 still correctly flag as open: the "upgrade" step (polling calendars later for a Bitcoin-confirmed proof, flipping `ots_status` to `confirmed`) remains unimplemented — this verification only exercised the initial `pending`-commitment submission, which is all `anchor.rs` currently does.

*Source: `crates/hsip-api/src/anchor.rs`, `crates/hsip-api/src/anchor_job.rs`, verified against a real running `hsip-api` server from an unrestricted Windows 11 network, 2026-07-20.*

---

### 4.21 OpenTimestamps "Upgrade" Polling — Bitcoin Confirmation, Not Just Submission

§4.20 confirmed submission works: HSIP can hand a digest to a calendar and get back a `pending` commitment. That commitment alone still requires trusting the calendar's own honesty about *when* it received the digest. The stronger guarantee — proof anchored to Bitcoin's own proof-of-work, verifiable without trusting the calendar at all — only exists once the calendar's batch has actually been mined into a Bitcoin block, something that happens sometime after submission, not at submission time. Getting from the weaker "pending" proof to the stronger Bitcoin-backed one is what the OpenTimestamps protocol calls "upgrading" a timestamp, and until now `anchor.rs` never asked calendars for it — a batch's `ots_status` sat at `pending` forever, even if the calendar had long since confirmed it.

**What's now implemented:**
- `anchor.rs::check_for_upgrade()` — `GET <calendar>/timestamp/<hex-digest>`, the real OpenTimestamps calendar upgrade-check endpoint. Returns `Ok(None)` (not an error) for "nothing new yet," `Ok(Some(bytes))` when the calendar has an update, `Err` only on genuine unreachability.
- `anchor.rs::contains_bitcoin_attestation()` — detects confirmation via presence of OpenTimestamps' `BitcoinBlockHeaderAttestation` byte tag (`05 88 96 0d 73 d7 19 01`, matching the reference Python implementation's `notary.py`). This is a tag-presence check, not a full parse of the `.ots` binary format's Merkle-path operations or independent verification against a real Bitcoin block header — consistent with this module's already-documented "store the opaque blob, trust the calendar's response" MVP scope for the initial submission. It doesn't introduce a new, weaker trust assumption than what already existed.
- `anchor.rs::extract_pending_calendar_uri()` — reads the originating calendar's URL back out of the *stored* `PendingAttestation` proof bytes, rather than adding a new `calendar_url` column to `decision_anchors`/`audit_anchors`. The calendar's URI is already embedded in what gets stored at submission time (confirmed empirically — see below); a schema change wasn't needed.
- `anchor_job::run_upgrade_cycle()` — checks every `decision_anchors`/`audit_anchors` row still at `ots_status = 'pending'` against its originating calendar, flips matching rows to `'confirmed'` with the fuller proof bytes swapped in. Spawned in `main.rs` on its own 15-minute timer, deliberately separate from the 10-second anchor-submission poll — Bitcoin blocks land roughly every 10 minutes on average, so checking as often as submission does would just hammer calendar servers for no benefit.
- `metrics::ANCHOR_UPGRADED_TO_CONFIRMED` — new counter, incremented on each successful upgrade.

**Two bounds added after a self-review QA pass ("what happens at maximum / at infinity"), before this was ever committed:**
- `anchor_job::MAX_UPGRADE_CHECKS_PER_CYCLE` (25) — a first version of this feature queried *every* `pending` row with no `LIMIT`, so a large backlog would mean one 15-minute cycle sequentially checking an unbounded number of calendars, each with its own 15s timeout — in the worst case, a cycle could take longer than the 15-minute gap before the next one. Fixed: `ORDER BY created_at ASC LIMIT 25` — oldest-first, so a backlog drains gradually across cycles instead of one cycle trying to do everything. Verified by `tests/integration.rs::test_upgrade_cycle_caps_checks_per_run`, which seeds 30 pending rows against one mock calendar and asserts exactly 25 requests are made in a single cycle — a request-count assertion, not an inferred one.
- `anchor_job::MAX_PENDING_UPGRADE_AGE_MS` (7 days) — without this, a batch whose calendar never confirms (permanently offline, or some edge case that never resolves) would be checked every 15 minutes forever, for the entire operational life of the server. Fixed: rows older than 7 days (generous relative to the hours-scale confirmation time real calendars normally take) stop being auto-checked; the anchor data itself remains fully intact and independently verifiable either way, it just stops being auto-upgraded. New `metrics::ANCHOR_UPGRADE_STALE` counter tracks how many rows have crossed that threshold, so an operator isn't relying on log-grepping to notice. Verified by `tests/integration.rs::test_stale_pending_anchor_is_not_auto_polled`, which seeds a mock calendar that *would* confirm immediately if asked, inserts an 8-day-old pending row, runs the upgrade cycle, and asserts the calendar received **zero** requests — proving the row was actually skipped, not just that it happened to still read `pending` (which could be true even with a real bug, if the check ran but the response were misread).

**The byte-tag constants are confirmed correct against a real calendar, not just recalled from memory or copied from documentation:** decoding the actual `ots_proof` bytes captured during §4.20's live verification (the real `alice.btc.calendar.opentimestamps.org` response) shows the `PendingAttestation` tag appearing exactly where expected, immediately followed by a length-prefixed payload whose length fields and content exactly match that response's embedded calendar URI (`https://alice.btc.calendar.opentimestamps.org`, 45 bytes, matching both the inner length-prefix byte and the URI's actual character count). This real fixture is preserved as `anchor.rs`'s `REAL_PENDING_PROOF` unit test constant, so the parsing logic is checked against genuine calendar output on every test run, not a synthetic approximation of one.

**What's still open — the upgrade path itself hasn't been live-tested against a real calendar.** §4.20's live verification only exercised the initial submission; confirming that a real calendar genuinely returns a Bitcoin-confirmed proof after enough time has passed would require waiting out a real submission's confirmation window (potentially hours), which wasn't practical to verify in this round. Instead, `tests/integration.rs::test_decision_anchor_upgrades_to_bitcoin_confirmed` covers the full pipeline against a mocked calendar: submit → anchor at `pending` (with a mock response shaped exactly like the real captured fixture, including a genuine calendar URI a real upgrade check would need to parse) → mock the calendar reporting confirmation → run the upgrade cycle → confirm `ots_status` becomes `confirmed`, both in the raw DB row and through `GET /v1/decisions/:id/proof`. This proves the mechanism is wired correctly end-to-end; it does not prove a real calendar's actual confirmed-proof response parses cleanly, since no real one has been observed yet.

*Source: `crates/hsip-api/src/anchor.rs`, `crates/hsip-api/src/anchor_job.rs`, `crates/hsip-api/src/main.rs`, `crates/hsip-api/src/metrics.rs`, `crates/hsip-api/tests/integration.rs::test_decision_anchor_upgrades_to_bitcoin_confirmed`.*

---

### 4.22 A Real Silent-Failure Bug, and a New System-Health Surface for "Can This Recover Automatically?"

A QA review asking "can [each recovery mechanism] recover automatically?" surfaced two things worth documenting: one confirmed bug in already-shipped code, and a real gap in how HSIP answers "something needs a human — how would that human ever find out?"

**The bug — three sites that logged success without checking it happened.** `anchor_job.rs`'s OTS retry/upgrade paths (`retry_pending_ots_submissions`, `retry_pending_audit_ots_submissions`, and `upgrade_one_anchor`, added in §4.21) all discarded their corrective `UPDATE`'s result via `let _ = sqlx::query(...).execute(db).await;`, then unconditionally logged success (and, in `upgrade_one_anchor`'s case, incremented `metrics::ANCHOR_UPGRADED_TO_CONFIRMED`) regardless of whether that write actually landed. Had the `UPDATE` genuinely failed — a momentarily locked database, a row deleted concurrently — the row would silently stay in its old state while logs and metrics falsely reported it resolved. **Fixed:** all three sites now match on the `UPDATE`'s actual result and check `rows_affected() > 0` before logging success or touching a metric; a zero-rows or `Err` result now logs a `warn!` instead, and the row is naturally retried on the next cycle since its DB state never changed. Verified by `anchor_job.rs`'s own `zero_rows_affected_is_not_counted_as_a_successful_upgrade` unit test, which fetches a real row, deletes it out from under the code (a clean, realistic way to force a genuine zero-rows-affected `UPDATE`), and asserts the success metric does not move — proving the fix against the exact failure mode, not just re-reading the diff.

**The gap — "can it recover automatically?" has real "no" answers, and nothing surfaced them.** This codebase has several conditions that genuinely cannot self-heal: an incomplete master key rotation (§4.13's `.rotating` staging file, deliberately left for manual recovery), a node that somehow reaches zero active root-admin keys (guarded against via the API, but not against direct database tampering), and — as of §4.21 — an anchor batch that's given up being auto-upgraded past `MAX_PENDING_UPGRADE_AGE_MS`. HSIP has no push-based alerting of its own (no email, no webhook, no PagerDuty integration — deliberately, matching the same "HSIP never holds a new class of credentials on the operator's behalf" reasoning as `HSIP_ROTATION_HOOK`, §4.13). Without something surfacing these, an operator — whether a single person running HSIP on their desktop or a business running it behind real infrastructure — would only discover any of these by reading the database directly.

**Fixed with a new `system_health.rs` module and three surfaces built on it:**
- `system_health::check()` runs three checks — incomplete master key rotation (a filesystem stat for `{master_key_path}.rotating`), zero root admins (`SELECT COUNT(*) FROM api_keys WHERE is_root_admin = 1`), and abandoned OTS anchors (`COUNT(*)` of `pending` rows older than `MAX_PENDING_UPGRADE_AGE_MS`) — and returns a structured `{healthy, issues: [{code, severity, summary, detail}]}` result. Pure and side-effect-free, so it's directly unit-testable (5 tests covering each check firing and not-firing).
- `GET /v1/admin/system-health` — root-admin gated like every other node-level `/v1/admin/*` route, read-only. The on-demand path for an operator (or their own tooling) to ask "is anything wrong right now."
- `metrics::SYSTEM_HEALTH_ISSUES` — a `GaugeVec` (by `severity`), not a counter, specifically so a *resolved* issue correctly drops the count back to zero rather than staying "triggered" forever the way a monotonic counter would. Refreshed live on every `GET /v1/admin/system-health` call and by a new periodic background task (`main.rs`, 5-minute interval — cheap enough to check often, since it's a filesystem stat plus two tiny `COUNT(*)` queries, no network I/O) so `/metrics` stays current even if nobody polls the JSON endpoint. This is the actual answer for a business running real Prometheus/Grafana alerting: fire on `hsip_system_health_issues{severity="critical"} > 0` without ever touching HSIP's own API.
- `hsip status` (CLI) now calls this endpoint first and prints any issues loudly at the very top of its output, before identity/agent/audit sections — the answer for the individual-desktop-user audience who isn't running Prometheus. A key without root-admin privilege gets a clear "unavailable, here's why" line instead of `hsip status` failing outright.

**Verified end-to-end against a real running server, not just unit tests:** ran a real `hsip-api` instance, confirmed `hsip status` prints "✓ OK" on a clean node, forced a real `zero_root_admins` condition by clearing the flag directly in the database and confirmed the CLI correctly showed "unavailable" once the calling key itself lost root-admin (an honest side effect of that exact test), restored it, inserted a real 8-day-old pending anchor row, and confirmed all three surfaces agreed: `GET /v1/admin/system-health` returned the issue, `hsip status` printed it prominently, and `curl /metrics` showed `hsip_system_health_issues{severity="warning"} 1`.

*Source: `crates/hsip-api/src/anchor_job.rs`, `crates/hsip-api/src/system_health.rs`, `crates/hsip-api/src/routes/admin.rs`, `crates/hsip-api/src/metrics.rs`, `crates/hsip-api/src/main.rs`, `crates/hsip-cli/src/commands/agent.rs`, `crates/hsip-api/tests/integration.rs::test_system_health_route`.*

---

### 4.23 Information Disclosure via Error Messages, and Per-Key mTLS Client-Certificate Binding

A QA pass asking "what is exposed during debugging?" and "what becomes dangerous after compromise?" surfaced two independent, unrelated issues, both closed the same session.

**Information disclosure — raw library/database error text reaching ordinary callers.** `ApiError::Internal` is meant to carry a message safe to hand back over HTTP; two independent code paths defeated that. First, `errors.rs`'s `From<sqlx::Error>`/`From<anyhow::Error>` impls embedded the error's own `Display` text directly into the response body — any route using `?` on a `sqlx`/`anyhow` result (the normal, idiomatic way to write a route handler in this codebase) leaked whatever detail that library's error type happened to include: schema/column names, file paths, occasionally fragments of query text. Second, and found only by grepping for the whole pattern class rather than trusting the first fix covered every site, several routes constructed `ApiError::Internal` manually via `.map_err(|e| ApiError::Internal(e.to_string()))` or `format!("...: {e}")`, bypassing the `From` impls entirely — the fix above did nothing for these. **Fixed:** both `From` impls now log the real error server-side via `tracing::error!` and return a fixed `"internal server error"` message (`sqlx::Error::RowNotFound` still maps to a clean `404`, unchanged); the same treatment was applied to four manual `.map_err` sites in `auth.rs`'s `TenantId` extractor (the single highest-traffic code path in the codebase — every authenticated request passes through it) via a new shared `internal_db_error()` helper, plus one site each in `routes/credentials.rs`, `routes/messages.rs`, `routes/identity.rs`, and `routes/decisions.rs`. Deliberately **not** touched: `routes/dns.rs`'s local-service-status message (not sensitive) and most of `routes/admin.rs`'s diagnostic messages (root-admin-gated only — that caller is already trusted with node-level operations, and several of those messages, like `HSIP_ROTATION_HOOK` stderr, are documented elsewhere in this file as deliberately informative for exactly that audience). Verified by a new `errors.rs` test module (4 tests: DB/anyhow detail never reaches the client, `RowNotFound` still maps cleanly, hand-written safe messages are unaffected) plus the full existing 41-test integration suite passing with zero regressions.

**"What becomes dangerous after compromise?" — a stolen bearer token still worked from anywhere, even over mTLS.** §4.15's mutual TLS authenticates the *connection*: once a client presents a certificate chaining to `client_ca_path`, every bearer token is usable over that connection, with no link between a specific key and a specific certificate. A stolen bearer token was therefore still fully usable from any client the CA would issue a certificate to — mTLS narrowed "any TLS client" down to "any client with a CA-issued cert," but didn't close the "stolen token = full access" problem §6's "API key theft" entry already flags as out-of-scope-but-worth-narrowing. This adds an *opt-in, per-key* binding on top: `POST /v1/keys/:id/bind-client-cert` lets an owner-role key tie `api_keys.bound_client_cert_fingerprint` (SHA-256 of the DER certificate) to a specific key in its tenant. Once bound, `auth.rs`'s `TenantId` extractor requires that exact certificate to have been presented on the connection carrying that key's bearer token — a copied/stolen bearer token alone is no longer sufficient for a key an operator has chosen to bind, on top of (not instead of) whatever `client_ca_path` already required at the transport layer.

**Implementation, and the deliberate constraint that keeps this from being self-defeating:** `mtls::ClientCertAcceptor` wraps the existing `RustlsAcceptor`, implementing `axum_server::accept::Accept` — it changes nothing about the TLS handshake itself (that's still fully delegated to the inner acceptor and still verified against `client_ca_path` exactly as before), it only reads back the certificate rustls already accepted and makes its fingerprint available to the request via `tower_http::add_extension::AddExtension`. The binding endpoint only ever binds the fingerprint from the *calling connection's own* presented certificate — there is no field to supply an arbitrary fingerprint string. This matters: without that constraint, an owner-role key could "bind" another key in its tenant to a certificate fingerprint nobody actually possesses, permanently locking that key out with no recovery path (the same failure shape as reaching zero root admins, §4.14, or losing the master key, §6 — a self-inflicted, unrecoverable lockout). Because binding can only ever use a fingerprint the binding request's own connection just proved possession of during its handshake, that failure mode isn't reachable through this endpoint. Fully opt-in — `bound_client_cert_fingerprint IS NULL` (the default, and the only possible state before this existed) is zero behavior change, same backward-compatibility guarantee every other opt-in feature in this codebase (replay protection, §4.3; mTLS itself, §4.15) already makes.

**Verified:** 4 new integration tests exercise the full lifecycle against the real router (via `tower::ServiceExt::oneshot`, injecting the same `ClientCertFingerprint` extension `ClientCertAcceptor` would insert on a real connection, since the test harness doesn't open a real TLS socket) — owner-role gating, refusal to bind without a presented certificate, and the complete bind → reject-without-cert → reject-wrong-cert → accept-right-cert → clear → access-restored sequence — alongside `mtls.rs`'s pre-existing real-X.509 unit tests (via the system `openssl` CLI) for the handshake-level certificate loading this builds on. Full workspace suite (41 integration + 28 unit tests) passes with zero regressions; `cargo clippy --all-targets` and `cargo fmt --check` both clean on every file this touched.

*Source: `crates/hsip-api/src/errors.rs`, `crates/hsip-api/src/auth.rs`, `crates/hsip-api/src/routes/{credentials,messages,identity,decisions,keys}.rs`, `crates/hsip-api/src/mtls.rs`, `crates/hsip-api/src/db.rs`, `crates/hsip-api/src/bin/hsip_migrate.rs`, `crates/hsip-api/src/routes/mod.rs`, `crates/hsip-api/tests/integration.rs`.*

---

## 5. Trust Boundaries

| Boundary | Trust level | Notes |
|---|---|---|
| HTTPS client → HSIP API | Authenticated | Valid bearer key required for all `/v1/*` endpoints. If `[server.tls] client_ca_path` is configured (§4.15), a client certificate signed by that CA is also required before the TLS handshake completes — opt-in, off by default. A key with `bound_client_cert_fingerprint` set (§4.23) additionally requires that exact certificate on the connection, not just any CA-signed one — opt-in per key, on top of the connection-level requirement. |
| HSIP server → SQLite | Trusted | Database must not be publicly accessible |
| Master key storage → HSIP server | Critical | Compromise = all signing keys exposed |
| Tenant A ↔ Tenant B | Untrusted | Isolated by `tenant_id` on every query |
| Root-admin key holder (`is_root_admin=1`, any tenant) | Full node-level — master key rotation + granting/revoking root-admin on other keys | As of §4.13/§4.14, can rotate the node's master key (a system-wide operation touching every tenant's identity) and grant/revoke the flag itself. No longer tied to a single hardcoded key — the bootstrap admin key starts with it, but any root admin can add more. Still one flat capability, not a real RBAC system — protect every root-admin key like the master key itself. |
| Owner-role key holder (`role='owner'`, own tenant) | Full within its own tenant's key management | Can create and revoke keys — including granting `owner` to others — in its own tenant. Cannot see or touch another tenant's keys, and cannot reach node-level operations without also holding `is_root_admin`. |
| AI agent key holder | Scoped | Velocity-limited, auto-revoked at 1000 req/min |
| Trusted peer (federated trust) | Explicit | Verify key manually registered; messages verified locally. §4.18 fixed a bug (`trusted_peers` table never created) that meant this mechanism could not actually be used before — treat any deployment older than that fix as never having had working federated trust. |

---

## 6. What HSIP Does Not Protect Against

The following are **explicitly out of scope**. They must be addressed at the infrastructure or application layer.

| Attack | Why out of scope | Recommended mitigation |
|---|---|---|
| **OS or host compromise** | An attacker with root access can read the master key from the filesystem or memory | OS hardening, container isolation, secrets manager for master key |
| **Physical server access** | An attacker with physical access can read the filesystem | Full-disk encryption at the OS level |
| **API key theft** | A stolen key grants full tenant access until rotated | Short-lived keys, audit log monitoring, key rotation policy. Per-key mTLS client-certificate binding (§4.23, `POST /v1/keys/:id/bind-client-cert`) narrows this for any key an operator opts in: a stolen bearer token alone stops being sufficient once a specific client certificate is bound, since the request must also arrive over a connection presenting that exact certificate. Still opt-in, not a default — an unbound key (the default) is unaffected. |
| **Network-layer DDoS** | HSIP has per-key rate limiting but no IP-level flood protection | Reverse proxy or CDN in front for public deployments |
| **Side-channel attacks** | No constant-time guarantees outside what `ed25519-dalek` and `chacha20poly1305` provide | Not a realistic concern for network-connected deployments |
| **Consent coercion** | HSIP enforces cryptographic consent, not voluntary human consent | Application-layer UX and legal controls |
| **Master key loss** | If `master.key` is lost, all encrypted signing keys are permanently unrecoverable | Back up the master key (a startup warning now reminds you to); `GET /v1/admin/master-key/fingerprint` / `hsip keys master-fingerprint` lets you *verify* that backup actually matches production without exposing or rotating the key; `POST /v1/admin/master-key/rotate` (§4.13) lets you *replace* a still-available key on a schedule, which reduces how long any one key's loss would matter, but does not help if the current key is already gone — that's still unrecoverable |
| **HSM-backed key storage** | Master key lives on the filesystem by default | Point `HSIP_MASTER_KEY` at a secrets manager (Vault, AWS KMS) — **this now actually works**; previously the only code path that read `HSIP_MASTER_KEY` was dead code nothing called, so this documented mitigation was not functional. Fixed — see `main.rs::load_master_key`. |
| **Post-quantum adversaries (current Ed25519)** | Ed25519 is not quantum-safe | ML-KEM-768 + ML-DSA-65 available via `hsip-core::pqc` (the `pqc` Cargo feature, on by default) for environments requiring it |
| **Social engineering** | If an admin is phished, HSIP cannot detect it | Operational security, 2FA on the server, key rotation |

---

## 7. Residual Risks and Known Gaps

Documented openly. Tracked for the v1.0 audit milestone.

| Gap | Risk | Mitigation path |
|---|---|---|
| No third-party security audit | Medium | Planned before v1.0. Codebase is open source and auditable now. §4.19's self-review found and fixed three concrete vulnerabilities (unauthenticated proxy endpoints, stored XSS via SVG upload, a hardcoded fallback JWT key) — a useful interim pass, but self-review by the code's own author cannot substitute for independent review; this gap stays open until a third party actually does one. |
| **Single maintainer, no succession plan** | Medium | Relevant to any team evaluating HSIP for compliance-grade audit trails: patch timelines and long-term availability are best-effort, not contractual (see §9). No code fix for this — flagged here for buyer due diligence, not left implicit in the disclosure section alone. |
| Master key on filesystem (no HSM) | Medium | Use `HSIP_MASTER_KEY` env var + external secrets manager for production — now functional, see §6. Rotation of an env-var-sourced key can now be automated via `HSIP_ROTATION_HOOK` (§4.13) instead of being purely manual. |
| **Rotation hook execution is a new trust boundary** | Low | `HSIP_ROTATION_HOOK` runs an operator-configured executable with the new master key on stdin. The path is only ever read from server-side environment configuration, never from an HTTP request — no caller can influence which script runs or with what arguments. The operator who sets this env var is implicitly trusting that script already, the same way `config.toml` or `master_key_path` are already-trusted operator-supplied paths; HSIP does not sandbox or validate the hook's contents. |
| **Flat RBAC model, not scoped permissions** | Low | §4.14 closed the "single hardcoded root-admin credential" gap (multiple root admins can now exist via grant/revoke) and the "any key can manage any other key in its tenant" gap (now `role='owner'`-gated). What remains by design: `is_root_admin` is one flat capability covering every node-level operation (no "can rotate but not grant"), and `role` is a two-tier split, not per-action scoped permissions. Revisit only when an operation shows up that genuinely needs finer scoping than that. |
| **`hsip-migrate` is a straightforward row-copy tool, not a zero-downtime replication system** | Low | §4.16 closed the "no migration tooling exists at all" gap, and along the way found and fixed that PostgreSQL had never actually worked for *any* HSIP deployment, fresh or migrated (see §4.16 for both bugs). What remains by design: `hsip-migrate` requires stopping writes to the source SQLite database before running (it's a one-shot `SELECT` + transactional `INSERT` copy, not continuous replication) and is meant for a planned maintenance-window cutover, not a live migration. |
| ~~OpenTimestamps calendar submission unverified end-to-end~~ — **closed, §4.20** | Was: Medium | §4.20 verified real submission end-to-end from an unrestricted network: raw connectivity to all three `DEFAULT_CALENDARS`, plus a real decision recorded and anchored through the actual `hsip-api` server with a genuine, independently-decoded calendar receipt (`ots_status: "pending"`) — not a mock. |
| ~~"Upgrade" polling for Bitcoin confirmation unimplemented~~ — **implemented, §4.21** | Was: Low | `anchor_job::run_upgrade_cycle` now polls calendars on its own 15-minute timer and flips `ots_status` to `confirmed` on a detected Bitcoin attestation. Byte-tag detection logic confirmed correct against a real captured calendar response. Still open: the confirmation path itself (a real calendar actually returning a Bitcoin-confirmed proof) hasn't been live-tested — only proven against a mocked calendar, since verifying against a real one requires waiting out a real confirmation window. |
| Rate-limit/velocity snapshot interval (was: full reset on restart) | Low | §4.6 closed the "in-memory only, full reset on every restart" gap — `rate_limit_state` now persists rate-limit, AI-agent-velocity, and sandbox-provisioning counters across restarts. Residual window: up to `SNAPSHOT_INTERVAL_SECS` (30s) of state can still be lost on a crash or unclean restart, since this is a periodic snapshot rather than a write-through on every request. |
| SQLite without WAL under write contention | Low | Low risk for single-tenant deployments; use `DATABASE_URL` pointing at PostgreSQL for high-concurrency — this recommendation is now actually backed by a working, tested PostgreSQL path (§4.16); before this it was untested advice pointing at a backend that, per §4.16, could not actually complete a single write. |
| Mutual TLS is opt-in, not enforced by default (was: no peer auth at the transport layer at all) | Low | §4.15 closed the "no automatic peer auth at the transport layer" gap — `[server.tls] client_ca_path` lets an operator require and verify client certificates. What remains by design: it's off by default (as TLS itself always has been for HSIP, a self-hosted/desktop-first product), and HSIP does not run its own CA or issue certificates — the operator brings their own PKI. |
| Audit log anchor cadence window | Low | §4.8.1 closed the "chain not anchored outside this database" gap — an attacker who deletes the whole chain *after* its last anchor is now detectable by comparing the anchored root against what remains. The residual window is the same shape as decisions' own: a chain deleted *before* its next anchor cycle runs (bounded by `INTERVAL_TRIGGER_MS` = 5 min) is still undetectable by this mechanism alone. |
| Clock skew affects consent, credential, *and* decision-chain ordering | Low | All three subsystems trust the server's wall clock for expiry/ordering, not just consent. Use NTP synchronization in production. |
| Post-quantum crypto (ML-KEM-768/ML-DSA-65) is not wired into any live protocol path | Informational, not a gap | Corrects a previous version of this table, which misattributed this to `hsip-verify` (excluded from the workspace at the time) — the actual implementation is `hsip-core::pqc`, gated by the `pqc` Cargo feature (on by default), and `hsip-core` has always been a normal workspace member, always built. It compiles and its own unit tests run, but nothing in `hsip-api`'s request path or `hsip-session`'s handshake code currently calls it — `grep -rn "pqc::"` outside `pqc.rs` itself returns nothing. For HSIP's actual threat model — API-key theft, filesystem/DB compromise — this matters far less than the items above; don't let its presence in dependency lists (§4.11) imply it's closer to wired-in-and-live than it actually is. |
| ~~No way to discover conditions requiring manual intervention~~ — **closed, §4.22** | Was: Low | §4.22 added `system_health.rs` (incomplete master key rotation, zero root admins, abandoned OTS anchors), a root-admin-gated `GET /v1/admin/system-health`, a `hsip_system_health_issues` Prometheus gauge refreshed every 5 minutes independent of anyone polling the API, and prominent surfacing in `hsip status`. Still by design, not a gap: HSIP still has no push-based alerting of its own (no email/webhook/PagerDuty) — an operator or their monitoring stack has to actually look, whether that's a human running `hsip status` or a Prometheus rule watching the gauge. |
| ~~Raw DB/library error text reaching ordinary API callers~~ — **closed, §4.23** | Was: Low | §4.23 found and fixed two independent leak paths: `errors.rs`'s `From<sqlx::Error>`/`From<anyhow::Error>` impls, and several routes' own manual `.map_err(\|e\| ApiError::Internal(e.to_string()))` sites that bypassed those impls. Both now log server-side and return a generic message to the caller. Root-admin-gated diagnostics in `routes/admin.rs` deliberately left untouched — that audience is already trusted with node-level operations. |
| Per-key mTLS certificate binding is opt-in, and only narrows (doesn't eliminate) stolen-token risk | Low | §4.23 added `POST /v1/keys/:id/bind-client-cert` — a stolen bearer token for a *bound* key additionally requires the exact bound certificate, but an unbound key (the default) is exactly as exposed to token theft as before this existed. Binding is per-key, opt-in, and requires `client_ca_path` (§4.15) to already be configured — there's no way to bind a key on a deployment that hasn't already opted into mTLS at the connection level. |
| Silent-failure logging bug in OTS retry/upgrade paths (found and fixed, §4.22) | Was: Low, now closed | Three sites in `anchor_job.rs` discarded a corrective `UPDATE`'s result and logged/counted success unconditionally — found during the same QA review that produced §4.22's health checks, fixed the same session, and verified by a unit test that forces a genuine zero-rows-affected `UPDATE` rather than just re-reading the diff. Documented here rather than silently folded into the changelog, since it's exactly the kind of bug this document exists to surface. |

---

## 8. Audit and Review Status

| Item | Status |
|---|---|
| Third-party security audit | Not yet completed — planned for v1.0 |
| Full-codebase security self-review | §4.19 — swept every route handler, the auth extractor, key-encryption, admin/rotation code, the CLI, and the MCP server. Found and fixed three confirmed vulnerabilities: unauthenticated `/v1/proxy/*` endpoints, stored XSS via SVG upload in `/v1/uploads`, and a hardcoded fallback JWT signing key in `hsip identity-serve`. Each re-verified fixed against real running-server/process behavior, not just re-read code. Self-review only — does not substitute for the third-party audit above. |
| Formal verification of protocol properties | `hsip-verify` crate uses the Z3 SMT solver to prove consent non-forgery, temporal consistency, and identity-binding soundness. **Now a normal workspace member** (§4.17) — `cargo build --workspace`/`cargo test --workspace` build and run it, so its guarantees run in CI wherever those commands do. |
| RFC compliance test vectors | RFC 8439 (ChaCha20-Poly1305), RFC 8032 (Ed25519) vectors pass in CI |
| Audit log hash chain integrity | Covered by `hsip-api/tests/integration.rs::test_audit_chain_verify_detects_valid_and_tampered_chains` — writes a chain, verifies it, then directly tampers with a row via SQL (simulating OS-level DB compromise) and confirms `GET /v1/audit/verify` detects it. |
| Master key rotation | Covered by `hsip-api/tests/integration.rs::test_master_key_rotation_reencrypts_and_swaps_live_key` — proves actual re-encryption (old key stops decrypting, the key now on disk decrypts), live in-memory key swap on the *same running process* (not just DB/file state), and rejection of a non-root-admin key. Also manually verified end-to-end against a running server: real `hsip keys rotate-master`/`master-fingerprint` CLI invocations, confirming fingerprints change, the key file on disk is rewritten, and `POST /v1/messages/sign` keeps working transparently across the rotation. |
| `HSIP_ROTATION_HOOK` rotation | Covered by `test_master_key_rotation_hook_for_env_sourced_key` (Unix-only) — proves refusal with no hook configured, a succeeding hook receives exactly the new key and the DB genuinely re-encrypts, and — the safety-critical case — a *failing* hook (non-zero exit) leaves the database completely untouched rather than partially rotated. Also manually verified end-to-end against a running server with `HSIP_MASTER_KEY` and `HSIP_ROTATION_HOOK` both set, with the hook's output independently re-hashed and confirmed to match the reported fingerprint. |
| Master key fingerprint endpoint | Covered by `test_master_key_fingerprint_is_read_only_and_admin_gated` — proves it's idempotent (repeated calls return the identical fingerprint, i.e. no mutation) and rejects a non-root-admin key. |
| Mutual TLS (`client_ca_path`) | Unit-tested in `mtls.rs` against real X.509 certificates generated via the system `openssl` CLI (not mocked). Also manually verified end-to-end against a real running server in server mode: a client certificate signed by the configured CA (with the required `clientAuth` EKU) connects and receives a genuine response; a certificate from an untrusted CA, and a request with no client certificate at all, are both rejected at the TLS handshake itself, confirmed via `curl`'s verbose TLS trace. |
| PostgreSQL schema/query compatibility | `crates/hsip-api/tests/postgres_compat.rs` (`#[ignore]`-by-default, run against `HSIP_TEST_POSTGRES_URL`) — proves `run_migrations` succeeds and a real epoch-ms timestamp / `BYTEA` blob round-trip correctly. Also manually verified end-to-end against a real PostgreSQL 16 instance: full server lifecycle (identity, messages, consent, decisions, audit chain) against a Postgres-backed server, `hsip-migrate` run against a populated SQLite database, and a fresh server started against the migrated Postgres database confirming the same tenant/identity, the original admin key, an intact audit hash chain, and a successful anchor-job run — see §4.16. |
| Federated trust (`/v1/trust/*`) | Previously **zero** integration coverage — the missing-table bug in §4.18 went undetected precisely because nothing exercised these routes end-to-end. `tests/integration.rs::test_trust_add_list_verify_remove` now covers add/list/verify (valid and tampered signature)/remove over the real HTTP stack. Also manually verified through the dashboard's new Trust page against a real running server. |
| OpenTimestamps calendar submission | **Now verified end-to-end from a real unrestricted network** (§4.20) — previously only unit-tested against a `wiremock` mock, since every sandboxed environment this project had been developed in blocked outbound HTTPS to the calendar hosts by policy. Raw connectivity confirmed to all three `DEFAULT_CALENDARS`; a real decision recorded and anchored through the live `hsip-api` server produced a genuine calendar receipt (`ots_status: "pending"`), independently confirmed by decoding the response bytes and finding the calendar's own URL embedded in them. |
| OpenTimestamps "upgrade" polling (Bitcoin confirmation) | §4.21 — `anchor_job::run_upgrade_cycle` and its full submit→pending→confirmed pipeline covered by `tests/integration.rs::test_decision_anchor_upgrades_to_bitcoin_confirmed` against a mocked calendar, plus `anchor.rs`'s unit tests for the tag-detection/URI-extraction logic against a real captured calendar response fixture (`REAL_PENDING_PROOF`). Not yet verified against a real calendar actually reporting Bitcoin confirmation — that requires waiting out a real submission's confirmation window, which hasn't been done. |
| System health checks (`system_health.rs`) | §4.22 — 6 unit tests cover each of the three checks firing and not firing, plus the Prometheus gauge correctly dropping back to zero when an issue resolves (proving it behaves as a gauge, not an accumulating counter). `tests/integration.rs::test_system_health_route` covers the actual HTTP route: root-admin gate, a healthy fresh node, and a real triggered issue. Also manually verified end-to-end against a real running server: forced a genuine `zero_root_admins` state via direct DB edit, confirmed `hsip status` degraded gracefully once the calling key itself lost root-admin as a result, restored it, inserted a real stale pending anchor, and confirmed `GET /v1/admin/system-health`, `hsip status`, and `GET /metrics` all agreed on the same issue. |
| Silent-failure fix in OTS retry/upgrade logging | §4.22 — `anchor_job::tests::zero_rows_affected_is_not_counted_as_a_successful_upgrade` forces a genuine zero-rows-affected `UPDATE` (fetches a row, deletes it, then calls the upgrade logic with the already-fetched row) and asserts the success metric doesn't move — proving the fix against the exact failure mode that motivated it, not just re-reading the diff. |
| Error-message information disclosure fix | §4.23 — new `errors.rs` test module (4 tests): a raw `sqlx::Error`/`anyhow::Error` never reaches the client body, `RowNotFound` still cleanly maps to `404`, and existing hand-written `ApiError::Internal` messages are unaffected by the change. |
| Per-key mTLS client-certificate binding (`POST /v1/keys/:id/bind-client-cert`) | §4.23 — 4 new integration tests: owner-role gating, refusal to bind without a presented certificate, and the full bind → reject-without-cert → reject-wrong-cert → accept-right-cert → clear → access-restored lifecycle, plus an explicit test that an unbound key authenticates identically whether or not a certificate is presented. Builds on `mtls.rs`'s pre-existing unit tests, which use real X.509 certificates generated via the system `openssl` CLI, not mocked crypto. |
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
