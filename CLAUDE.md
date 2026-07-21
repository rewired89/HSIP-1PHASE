# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Project Is

HSIP (High Security Internet Protocol) is a self-hosted, single-binary local identity server. It provides cryptographic identity (Ed25519), consent management, message signing, verifiable credentials, AI agent governance, DNS tracker blocking, and a tamper-proof audit trail.

**Strategic direction:** Own the "AI agent identity" niche — local-first, zero-config, the tool that gives every AI agent a cryptographic identity and consent-gated audit trail. Think Tailscale for AI agent security.

**The server runs at:**
- `http://127.0.0.1:7474` — desktop mode (no `config.toml` present)
- `http://127.0.0.1:3000` — server mode (with `config.toml`)

**IMPORTANT:** `config.toml` is NOT committed (it's `.gitignore`d). `config.example.toml` shows the format. Without `config.toml`, the server always starts in desktop mode on port 7474. Do not re-add `config.toml` to the repo.

**Active development branch:** `claude/create-claude-md-pBtap`

---

## Build Commands

```bash
# Dev API only (no embedded dashboard) → port 7474
cargo run -p hsip-api

# Dev with hot-reload dashboard (API on :3000 via config.toml, Vite on :3001)
cargo run -p hsip-api
cd dashboard && npm install && npm run dev   # proxies /v1 → localhost:3000

# Production binary with embedded dashboard
cd dashboard && npm install && npm run build && cd ..
cargo build --release -p hsip-api --features hsip-api/embed-dashboard

# Build individual crates
cargo build -p hsip-core
cargo build -p hsip-cli
cargo build -p hsip-mcp

# hsip-verify is a normal workspace member — cargo build --workspace builds it too.
# First build compiles Z3 from source via z3-sys (~8 min, one-time — cached
# by Cargo afterward like any other dependency). Requires cmake + a C++
# toolchain (both already needed to build this workspace's other native deps).
cargo build -p hsip-verify
```

## Test Commands

```bash
# Full suite
cargo test --workspace

# Single crate
cargo test -p hsip-api
cargo test -p hsip-core

# Single test (exact match)
cargo test -p hsip-api test_credential_issue_verify_revoke -- --nocapture --exact
cargo test -p hsip-api test_decision_attestation_sign_anchor_verify_end_to_end -- --nocapture --exact

# Crypto compliance
cargo test -p hsip-core rfc8439 -- --nocapture   # RFC 8439 ChaCha20-Poly1305 vectors
cargo test -p hsip-core nonce -- --nocapture      # replay attack prevention

# Quality
cargo clippy --workspace -- -D warnings
cargo fmt --check
cargo audit
```

---

## Crate Map (17 total in workspace)

| Crate | Binary | Role | Status |
|---|---|---|---|
| `hsip-api` | `hsip-api` | REST API server. Axum + Tokio. All HTTP routes, auth, rate limiting, DB. The only deployable binary. | Core — do not break |
| `hsip-core` | — | Crypto primitives: Ed25519, X25519, ChaCha20-Poly1305, consent protocol, nonce tracking, RFC 6962 Merkle trees (`merkle.rs`), RFC 8785 JCS canonicalization (`canonical.rs`). No I/O. | Core — do not break |
| `hsip-dns` | — | UDP DNS server (:5300). Hardcoded tracker blocklist; forwards to 1.1.1.1:53. | Working |
| `hsip-cli` | `hsip-cli` | CLI tool. Includes `agent`, `trust`, `up`, `status` subcommands. | Active development |
| `hsip-mcp` | `hsip-mcp` | MCP server (JSON-RPC over stdio). Exposes HSIP tools to AI clients. | Active development |
| `hsip-common` | — | Shared types. | Stable |
| `hsip-intercept` | — | Cross-platform network interception (Windows/Android/Linux/macOS). | Stable |
| `hsip-verify` | — | Formal verification of core HSIP security properties (consent non-forgery, temporal consistency, identity binding) via the Z3 SMT solver, invoked as a Rust library (`z3` crate, `static-link-z3` feature — no system Z3 install needed). A normal workspace member — see "Including hsip-verify in the Build" below for why it wasn't always one. | Working |
| `hsip-session`, `hsip-net`, `hsip-auth`, `hsip-reputation`, `hsip-gateway`, `hsip-regenerative`, `hsip-telemetry-guard`, `hsip-integration-sdk` | — | Supporting crates in workspace but not actively integrated. | Stable — don't touch unless needed |

---

## `hsip-api` Internals

```
main.rs           Startup: config → master key → DB → bootstrap admin → Axum router → spawns anchor loop
config.rs         Two modes: Config::load("config.toml") or Config::desktop_defaults()
db.rs             AnyPool (sqlx). ALL migrations are inline SQL in run_migrations() (pub — reused by bin/hsip_migrate.rs) — no migration files. Every SQL bind placeholder in this codebase must be $1/$2/... (never ?) and every wide/timestamp column must be BIGINT (never INTEGER) / BYTEA (never BLOB) — see SQLite → PostgreSQL Migration below for why.
bin/hsip_migrate.rs  hsip-migrate binary: copies an existing SQLite deployment's data into PostgreSQL — see SQLite → PostgreSQL Migration below.
auth.rs           TenantId extractor: SHA-256(Bearer) → DB lookup → rate limit → AI velocity check
state.rs          AppState: DB + DashMap rate limiter + agent tracker + DNS handle + proxy ring buffer
key_encryption.rs ChaCha20-Poly1305 + HKDF-SHA256 encryption for Ed25519 private keys at rest
anchor.rs         OpenTimestamps calendar HTTP client (network I/O only, no DB) — see Decision Attestations below
anchor_job.rs     Batches unanchored decisions into a Merkle tree on a timer, submits root to OpenTimestamps
audit_log.rs      BLAKE3 hash-chained writes to audit_entries — see Audit Log Hash Chain below
rate_limit_persistence.rs  Periodic snapshot/restore of rate-limit + AI-agent-velocity DashMaps — see Rate Limiter Persistence below
mtls.rs           Optional mutual TLS for [server.tls] — see Mutual TLS below
routes/           One file per domain (see route table below)
static_files.rs   Serves dashboard/dist/ via rust-embed (only active with embed-dashboard feature)
```

### API Routes

```
POST/GET  /v1/identity              Ed25519 keypair create/get
POST      /v1/identity/rotate       Rotate signing key
POST/GET  /v1/consent/*             Grant, revoke, list, check consent
POST/GET  /v1/messages/*            Sign and verify messages
POST/GET/DELETE /v1/credentials/*   Issue, verify, revoke verifiable credentials
POST/GET/DELETE /v1/keys/*          API key management (human / service / ai_agent types)
POST      /v1/keys/:id/bind-client-cert  Bind (or, with {"clear":true}, remove) an mTLS client-certificate requirement on a key — owner-role only, see Mutual TLS below
GET       /v1/agents                List ai_agent keys with live velocity stats
GET       /v1/agents/discover       Probe localhost ports for running AI agents / MCP servers
GET       /v1/agent/capabilities    Machine-readable HSIP capability spec for AI system prompts
GET       /v1/audit                 Audit log (filterable, max 500 entries)
GET       /v1/audit/verify           Recompute this tenant's BLAKE3 audit hash chain and report whether it's intact
GET       /v1/audit/:id/proof       Full self-contained verification bundle for one entry (BLAKE3 hash + Merkle proof + anchor)
POST      /v1/audit/verify-proof    Pure verification of a bundle — no auth, no DB call, runnable by anyone
POST/GET  /v1/tenant/*              Tenant info + GDPR erase
POST/GET  /v1/dns/*                 DNS blocker start/stop/status/log
POST/GET  /v1/proxy/*               Proxy traffic monitor start/stop/status/log
POST/GET/DELETE /v1/contacts/*      Contact management
POST      /v1/trust/peer            Add a trusted peer (label + Ed25519 verify key)
GET       /v1/trust/peers           List trusted peers
DELETE    /v1/trust/peers/:id       Remove a trusted peer
POST      /v1/trust/verify          Verify a message signature from a trusted peer by label
POST/GET  /v1/decisions             Sign + chain an AI-agent decision attestation / list this tenant's decisions
GET       /v1/decisions/:id/proof   Full self-contained verification bundle (signature + Merkle proof + anchor)
POST      /v1/decisions/verify      Pure verification of a bundle — no auth, no DB call, runnable by anyone
POST      /v1/admin/master-key/rotate  Rotate the master key (root-admin key only) — see Master Key Rotation below
GET       /v1/admin/master-key/fingerprint  SHA-256 fingerprint of the running master key — read-only, no mutation, root-admin key only
GET       /v1/admin/root-admins      List every active key holding root-admin privilege — root-admin key only, see RBAC below
POST      /v1/admin/root-admins/grant   Grant root-admin to another active key by id — root-admin key only
POST      /v1/admin/root-admins/revoke  Revoke root-admin from a key by id — refused if it's the last one — root-admin key only
GET       /health                   {"status":"ok","version":"0.2.0"}
GET       /metrics                  Prometheus metrics
GET       /openapi.json             OpenAPI 3.0 spec
GET       /docs                     Swagger UI
```

### Request Auth Flow

Every protected endpoint flows through `TenantId` extractor in `auth.rs`:
1. Extract `Bearer <token>` from `Authorization` header
2. SHA-256 hash it — raw tokens never stored
3. DB lookup `key_hash` → check `active=1` and `expires_at`
4. Check `pending_revocation` DashSet (immediate in-memory block before DB write)
5. Opt-in HTTP replay protection (see below) — only runs if the caller sent both replay headers
6. Per-key rate limit (300 req/min default, `RATE_LIMIT_RPM` env overrides)
7. `agent_type='ai_agent'` only: velocity check (>100/min logs anomaly; >1000/min auto-revokes)

### HTTP Replay Protection (opt-in)

A bearer token proves *who* sent a request; it doesn't prove a captured copy of that exact request hasn't been resent. `auth.rs::check_replay_protection` closes that gap, but only for callers who opt in — sending neither header is a complete no-op, so no existing SDK/CLI/dashboard caller is affected until updated to use it.

- Send both `x-hsip-timestamp` (Unix seconds) and `x-hsip-nonce` (opaque string, 1-128 chars) to opt in.
- Sending only one of the two is `400 Bad Request` — never silently ignored, so a caller can't mistakenly believe it's protected.
- Timestamp must be within 5 minutes of server time (`REPLAY_TOLERANCE_SECS` in `auth.rs`) or `401`.
- The `(key_id, nonce)` pair must not have been seen before within that window, or `401` (`"Duplicate x-hsip-nonce"`). Dedup is scoped per `key_id` — two different tenants can reuse the same nonce value without colliding.
- Seen nonces live in `AppState.replay_nonces` (`Arc<DashMap<String, i64>>`, expiry timestamp as the value), swept every 60s by a background task in `main.rs` so it can't grow unbounded. Entries are retained for 10 minutes (2x the tolerance window).
- `metrics::REPLAY_REJECTED` (labels: `malformed_headers`, `timestamp_out_of_window`, `duplicate_nonce`) — near-zero unless something is actually opted in and being replayed or misconfigured.
- **Not yet sent by any of HSIP's own SDKs, CLI, or dashboard** — this pass is the server-side check only. A caller who wants replay protection today constructs the headers itself.

### Rate Limiter Persistence (`rate_limit_persistence.rs`)

`AppState.rate_limiter`, `.agent_tracker`, and `.sandbox_rate` are in-memory `DashMap`s (atomics inside, for hot-path speed — no DB round-trip on every authenticated request). Previously that meant a restart (crash, deploy, container reschedule) silently reset every key's rate-limit count and every `ai_agent` key's velocity/anomaly counters back to zero — a key mid-way toward the 1000 req/min auto-revoke threshold got a clean slate for free.

- `rate_limit_persistence::snapshot()` upserts the current contents of all three trackers into a `rate_limit_state` table (`kind`, `state_key`, `count`, `anomaly_count`, `window_start_ms`, `updated_at`; `PRIMARY KEY (kind, state_key)`) every `SNAPSHOT_INTERVAL_SECS` (30s) — spawned in `main.rs` on its own timer, alongside the anchor and replay-nonce-sweep loops.
- `rate_limit_persistence::load()` runs once at startup, before the server accepts traffic, restoring only *live* windows (`now - window_start_ms < WINDOW_MS`) — an expired window is skipped, since it would reset to fresh on first use anyway regardless of whether it's restored.
- Deliberately a periodic snapshot, not a write-through on every request — that would add a synchronous DB write to the hot auth path on every single authenticated call, exactly what these DashMaps exist to avoid.
- **Residual risk, by design**: up to `SNAPSHOT_INTERVAL_SECS` of state is still lost on a crash or unclean restart. Bounded, not eliminated — same tradeoff already accepted for the master-key-rotation staging-file window and the decision-attestation signing-to-anchoring gap.
- Verified end-to-end against a real running server (not just the unit tests in `rate_limit_persistence.rs`): sent a burst of requests, waited for a snapshot tick, restarted the process, confirmed the startup log (`restored rate-limit/velocity state from last snapshot`) picked the live window back up instead of resetting to zero.

### Database Schema

All tables created at startup in `db::run_migrations()` — **no separate migration files**.

| Table | Key columns | Notes |
|---|---|---|
| `tenants` | id, name, created_at | — |
| `api_keys` | id, tenant_id, key_hash, name, agent_type, role, is_root_admin, expires_at, active | Raw key never stored. `role` ('owner'\|'member') gates tenant-scoped key management (create/revoke other keys in the same tenant) — see `routes/keys.rs`. `is_root_admin` (0/1) gates node-level operations spanning every tenant (master key rotation) — see `routes/admin.rs` and RBAC below. |
| `identities` | tenant_id, signing_key_b64, verify_key_b64 | Private key encrypted with ChaCha20-Poly1305 |
| `consents` | id, tenant_id, peer_verify_key, status, expires_ms, granted_by_key_type | UNIQUE(tenant_id, peer_verify_key). `granted_by_key_type` (human/service/ai_agent) records which kind of key authorized the grant — nullable, pre-migration rows are NULL. |
| `messages` | id, tenant_id, content, signature, timestamp | — |
| `audit_entries` | id, tenant_id, action, peer_verify_key, details, timestamp, prev_hash, entry_hash, anchor_id, merkle_index | Append-only — never delete. `prev_hash`/`entry_hash` form a per-tenant BLAKE3 hash chain (nullable — pre-migration rows unchained); write only via `audit_log::record()`, never a raw INSERT. `anchor_id`/`merkle_index` (nullable) point at the `audit_anchors` batch this entry's `entry_hash` was folded into, once anchored. |
| `contacts` | id, tenant_id, nickname, verify_key | — |
| `credentials` | id, tenant_id, claim, user_token, issuer_verify_key, signature, revoked | — |
| `trusted_peers` | id, tenant_id, label, verify_key, added_at | UNIQUE(tenant_id, verify_key). Federated trust store. |
| `decisions` | id, tenant_id, agent_key_id, accountable_key, model_version, strategy_id, decision_type, payload_hash, prev_hash, event_hash, signature, anchor_id, merkle_index | UNIQUE(tenant_id, prev_hash) — hash-chains each tenant's decisions, prevents forks under concurrent inserts. `payload_hash` only — actual decision content never stored. |
| `decision_anchors` | id, merkle_root, leaf_count, anchor_signature, anchor_verify_key, ots_proof, ots_status | One row per RFC 6962 Merkle batch. `ots_status`: `pending` \| `calendar_unreachable`. |
| `audit_anchors` | id, merkle_root, leaf_count, anchor_signature, anchor_verify_key, ots_proof, ots_status | Same shape as `decision_anchors`, one row per RFC 6962 Merkle batch of `audit_entries` — see External Anchoring below. |
| `rate_limit_state` | kind, state_key, count, anomaly_count, window_start_ms, updated_at | `PRIMARY KEY (kind, state_key)`. Periodic snapshot of the in-memory rate-limit/velocity DashMaps — see Rate Limiter Persistence below. `kind` is `rate_limit`\|`agent_velocity`\|`sandbox_rate`. |
| `anchor_identity` | id (singleton, always 1), signing_key_b64, verify_key_b64 | Node-level Ed25519 key that signs anchor roots — distinct from any tenant identity. Created on first anchor cycle. |

### Master Key Sources (in priority order)

1. `HSIP_MASTER_KEY` env var (hex string) — takes precedence when set; point this at a secrets manager (Vault, AWS KMS, etc.) for production. **This is actually read now** — for a while `main.rs::load_master_key` only ever read the file, and the only code that read `HSIP_MASTER_KEY` was a `#[allow(dead_code)]` function in `key_encryption.rs` that nothing called. Removed; `main.rs::load_master_key` is the one implementation.
2. Key file at path from `config.toml` → `[security] master_key_path` (server mode)
3. `~/.hsip/master.key` (Linux/macOS) or `%APPDATA%\HSIP\master.key` (Windows) — desktop mode

A SHA-256 fingerprint (first 8 bytes, hex) of the loaded key is logged at startup either way — never the key itself — so an operator can confirm a backup matches what's actually running. When loaded from a file, a "back this up now, it's unrecoverable if lost" warning logs alongside it.

### Master Key Rotation (`POST /v1/admin/master-key/rotate`)

Was previously impossible — the master key was loaded once into an `Arc<Vec<u8>>` at startup and never changed. Now `AppState.master_key` is `Arc<RwLock<Vec<u8>>>`, and the bootstrap admin key can rotate it live, no restart required.

**What it does, in order:** generates a new 32-byte key → re-encrypts every tenant's `identities.signing_key_b64` and the singleton `anchor_identity` row inside one DB transaction → writes the new key to a staging file next to the real master key path and `fsync`s it → commits the transaction → atomically renames the staging file onto the real path → swaps the in-memory key. A `state.master_key.write().await` guard is held for the *entire* operation, not just the final swap — without that, a concurrent identity creation could encrypt a new row under the old key after this function's read but before the in-memory swap, permanently orphaning that one row. Writes a `master_key.rotated` audit entry per tenant touched and returns SHA-256 fingerprints (never the raw key) of the old and new key.

**Gating:** `require_root_admin()` in `routes/admin.rs` — requires the calling key's `is_root_admin` column to be `1`. See RBAC below for how that flag is granted/revoked; it replaced an earlier "`name == "admin"` and tenant is the first ever created" heuristic that only ever supported exactly one root admin.

**`HSIP_MASTER_KEY`-sourced keys:** resolved via `routes::admin::resolve_persistence` / `KeyPersistence` (`File(path)` | `Hook(command)`). When the key comes from `HSIP_MASTER_KEY` (`state.master_key_path` is `None`), rotation isn't automatically refused anymore — if `HSIP_ROTATION_HOOK` names an executable, that script is invoked with the new key hex-encoded on stdin (never as a CLI arg — visible in `ps`) and `HSIP_ROTATION_OLD_FINGERPRINT`/`HSIP_ROTATION_NEW_FINGERPRINT` env vars for context. Its exit code is the only signal trusted: non-zero or a 30s timeout aborts rotation with the DB transaction still uncommitted — nothing changes. **HSIP never holds Vault/AWS/etc. credentials itself** — the hook is the operator's own trusted tooling, run with whatever auth it already has; this was a deliberate design choice over embedding a specific vendor's SDK (see the QA-driven design discussion this originated from) precisely to avoid giving the HSIP process a new secret to protect. If no `master_key_path` *and* no `HSIP_ROTATION_HOOK`, rotation still refuses with an actionable error, same as before. Covered by `test_master_key_rotation_hook_for_env_sourced_key` (Unix-only — builds/chmods shell scripts as test hooks), which proves: refusal with no hook, a succeeding hook receives exactly the new key and the DB genuinely re-encrypts, and — the safety-critical case — a *failing* hook leaves the database completely untouched, not partially rotated. Also verified end-to-end against a real running server with `HSIP_MASTER_KEY` + `HSIP_ROTATION_HOOK` both set.

**Residual risk, by design, documented rather than hidden:** if the process crashes in the narrow window between the DB transaction committing and the staging-file rename completing, the DB holds ciphertext under the new key but the real key file still has the old one. The staging file (`{path}.rotating`) is deliberately left in place rather than cleaned up, specifically so that window is recoverable — an operator moves it into place manually. Covered by `crates/hsip-api/tests/integration.rs::test_master_key_rotation_reencrypts_and_swaps_live_key`, which proves actual re-encryption (old key stops decrypting, disk key decrypts), live in-memory swap (signing keeps working on the same running process), and rejection of a non-root-admin key.

**Read-only companion:** `GET /v1/admin/master-key/fingerprint` — same `require_root_admin()` gate, no mutation, returns the current key's fingerprint, `master_key_path`, and `rotation_available` (whether rotation currently has anywhere to persist a new key — `true` for file-backed keys or env-var-sourced keys with a hook configured). Exists because before it did, the *only* way to see a fingerprint was the startup log or a rotation response — there was no way to check "does my backup file actually match what's running right now" without either grepping server logs or triggering a real rotation. Covered by `test_master_key_fingerprint_is_read_only_and_admin_gated` — proves it's idempotent (same fingerprint on repeated calls) and admin-gated the same way rotation is.

**CLI:** `hsip keys master-fingerprint` and `hsip keys rotate-master` in `crates/hsip-cli/src/commands/keys.rs`. `rotate-master` prints what it's about to do and requires typing `yes` at an interactive prompt before calling the API — `--yes` skips that for scripts/automation. This was deliberately built as a CLI command and not left as "call the HTTP endpoint yourself" — HSIP's original design point was non-technical users, and requiring hand-rolled `curl` with bearer auth for a rotation operation would have made it unreachable for exactly that audience, while `--yes` keeps it scriptable for the enterprise/ops use case the CLI also needs to serve.

## RBAC: Tenant-Scoped Roles and Root-Admin Grants

Two independent, deliberately simple layers — not a general permissions system. Added because auditing the original model surfaced two real gaps at once: (1) within a tenant, *any* active key — including a low-privilege `ai_agent` key — could mint new `human` keys or revoke *any* other key in the same tenant, including the tenant's own admin key, with zero check; (2) node-level "root admin" was a single hardcoded credential (`name == "admin"` in the first tenant ever created) with no way to add a second one short of editing the database by hand.

### Tenant-scoped: `api_keys.role` ('owner' | 'member')

- `owner` can create and revoke keys in its own tenant via `POST`/`DELETE /v1/keys*`. `member` (the default for every key created via `POST /v1/keys` unless the caller explicitly requests `"role":"owner"`) can do neither — same 401 shape as every other privilege check in this codebase.
- `GET /v1/keys` (list) stays open to any active key in the tenant — informational only, not a mutation, and the dashboard/CLI need it to show key metadata regardless of the caller's role.
- `routes::keys::revoke` refuses to revoke a tenant's *last* remaining active `owner` key (`409 Conflict`) — otherwise the tenant becomes permanently unable to manage its own keys, including recovering from that exact mistake.
- Fresh installs: `main.rs::bootstrap_admin` sets `role='owner'` explicitly on the bootstrap key's INSERT. `routes::sandbox::provision` does the same for each trial tenant's sole key. Upgraded (pre-existing) databases: `db.rs`'s migration backfill makes the earliest-created key in each tenant `'owner'`, every other still-NULL key `'member'`.

### Node-level: `api_keys.is_root_admin` (0 | 1)

- Gates `POST /v1/admin/master-key/rotate` and `GET /v1/admin/master-key/fingerprint` via `require_root_admin()` — a straight `SELECT is_root_admin FROM api_keys WHERE key_hash=? AND active=1` check, no tenant scoping (the flag is node-wide by design; an anchor batch and a master key both span every tenant).
- `POST /v1/admin/root-admins/grant` / `.../revoke` (root-admin-only, `key_id` in the body) are how a second, third, etc. root admin gets created — the mechanism that didn't exist before this. `revoke` refuses (`409 Conflict`) if it would leave zero root admins on the node, mirroring the last-owner guard above; there'd be no way to recover except editing the database directly.
- `GET /v1/admin/root-admins` lists every active root-admin key (id, tenant_id, name, created_at) — root-admin-gated so it doesn't leak "who holds node-level authority" to anyone but the people who already have it.
- Every grant/revoke writes an `admin.root_admin_granted`/`admin.root_admin_revoked` audit entry (on the *target* key's own tenant) and increments `metrics::ROOT_ADMIN_CHANGES{action}`. `routes::keys::create`/`revoke` similarly write `key.created`/`key.revoked` now — both were previously state-changing operations with no audit trail at all, a gap independent of but found alongside this work.
- Fresh installs: `bootstrap_admin` sets `is_root_admin=1` explicitly. Upgraded databases: `db.rs`'s migration backfill sets it for the key named `admin` in the very first tenant ever created — the exact key the old heuristic already trusted, so nobody loses admin access on upgrade.

**CLI:** `hsip keys list-root-admins`, `hsip keys grant-root-admin <key_id>`, `hsip keys revoke-root-admin <key_id>` (interactive `yes` confirm, `--yes` for scripts) in `crates/hsip-cli/src/commands/keys.rs` — same non-technical-audience reasoning as `rotate-master` above.

**Still not a full RBAC system, by design:** `is_root_admin` is one flat capability covering every node-level operation, not scoped grants (no "can rotate but not grant," no per-operation permissions). `role` is a two-tier owner/member split, not fine-grained per-action permissions. Both are deliberately as simple as the current two node-level operations (rotate + fingerprint) and the one tenant-level capability (key management) actually need — see THREAT_MODEL.md for the residual tradeoff. Revisit with real scoped grants only when an operation shows up that the flat model can't express.

### Admin Key Location (platform-aware)

The server writes the admin key on first boot:
- **Linux/macOS:** `~/.hsip/admin.key`
- **Windows:** `%APPDATA%\HSIP\admin.key` (e.g. `C:\Users\<name>\AppData\Roaming\HSIP\admin.key`)

The CLI resolves this automatically via `commands/util.rs::admin_key_path()`. Never hardcode `~/.hsip/admin.key` in CLI code — always call `crate::commands::util::load_admin_key()`.

### Configuration Modes

| Mode | Trigger | Port | Data dir |
|---|---|---|---|
| Desktop defaults | No `config.toml` present | 7474 | `~/.hsip/` or `%APPDATA%\HSIP\` (auto-created) |
| Server mode | `config.toml` present or `HSIP_CONFIG` env set | 3000 | Configured in toml |

`DATABASE_URL` env var overrides the database URL in either mode.

---

## `hsip-cli` — CLI Tool

Binary: `hsip-cli`. Main file: `crates/hsip-cli/src/main.rs` (large — uses clap derive).

### Commands

```bash
# Onboarding — check server is up, start it if found, print welcome box
hsip up [--api-url URL] [--no-browser]

# AI agent governance
hsip agent register <name> [--expires-days N] [--api-url URL] [--key K]
hsip agent list [--api-url URL] [--key K]
hsip agent revoke <name-or-id> [--api-url URL] [--key K]
hsip agent discover [--api-url URL] [--key K]

# Overall status
hsip status [--api-url URL] [--key K]

# Federated trust (peer-to-peer Ed25519 verification)
hsip trust add <label> <verify-key> [--api-url URL] [--key K]
hsip trust list [--api-url URL] [--key K]
hsip trust remove <id> [--api-url URL] [--key K]
hsip trust verify --from <label> <content> <signature> [--api-url URL] [--key K]

# Master key inspection/rotation (root-admin key only)
hsip keys master-fingerprint [--api-url URL] [--key K]
hsip keys rotate-master [--yes] [--api-url URL] [--key K]

# Root-admin management (root-admin key only)
hsip keys list-root-admins [--api-url URL] [--key K]
hsip keys grant-root-admin <target-key-id> [--api-url URL] [--key K]
hsip keys revoke-root-admin <target-key-id> [--yes] [--api-url URL] [--key K]
```

The existing commands in `main.rs` handle: keygen, init, key import/export (plain + encrypted), consent over UDP, session management, token issue/verify, discovery, reputation, daemon, audit export/verify/query. Do not delete these — they are separate functionality.

### Key / URL resolution for all agent/trust/status/up commands

1. `--key` flag
2. `HSIP_API_KEY` env var
3. Platform-aware key file via `commands::util::load_admin_key()`

URL priority:
1. `--api-url` flag
2. `HSIP_API_URL` env var
3. `http://127.0.0.1:7474`

### Command files layout

```
crates/hsip-cli/src/commands/
  mod.rs        Declares all submodules
  util.rs       admin_key_path() + load_admin_key() — platform-aware, shared by all commands
  agent.rs      AgentCmd enum: Register, List, Revoke, Discover + status() pub fn
  trust.rs      TrustCmd enum: Add, List, Remove, Verify
  keys.rs       KeysCmd enum: MasterFingerprint, RotateMaster, ListRootAdmins, GrantRootAdmin, RevokeRootAdmin (interactive y/N confirm, --yes to skip)
  up.rs         UpArgs + run() — onboarding wizard
  diag.rs       Diagnostics (pre-existing)
  handshake.rs  UDP handshake (pre-existing)
```

### Before adding new CLI commands

1. Add the variant to `Commands` enum in `crates/hsip-cli/src/main.rs`
2. Create `crates/hsip-cli/src/commands/<name>.rs`
3. Add `pub mod <name>;` to `commands/mod.rs`
4. Use `use super::util::load_admin_key;` — do NOT write a local `load_admin_key()` function
5. Reuse the `ApiClient` pattern from `agent.rs` for HTTP calls
6. Wire the match arm in `fn main()`

---

## `hsip-mcp` — MCP Server

**Crate:** `crates/hsip-mcp/`. Binary: `hsip-mcp`.

Speaks Model Context Protocol (JSON-RPC 2.0 over stdio). AI clients (Claude Desktop, Cursor, Continue) add it to their MCP config once.

### Tools exposed

| Tool | API call | Purpose |
|---|---|---|
| `sign_message` | `POST /v1/messages/sign` | Tamper-proof signed record with timestamp |
| `verify_message` | `POST /v1/messages/verify` | Verify a peer's Ed25519 signature |
| `get_identity` | `POST /v1/identity` | Agent's public key (auto-creates if missing) |
| `grant_consent` | `POST /v1/consent/grant` | Record time-bounded consent |
| `check_consent` | `GET /v1/consent/:peer` | Gate actions behind consent check |
| `revoke_consent` | `POST /v1/consent/revoke` | Instant revocation |
| `log_action` | `POST /v1/messages/sign` | Explicit audit trail entry (signed message prefixed `[ACTION:...]`) |
| `get_recent_actions` | `GET /v1/audit?limit=N` | Read the audit trail |

MCP protocol handled: `initialize`, `ping`, `tools/list`, `tools/call`. Notifications (no `id` field) silently ignored per spec.

### Claude Desktop setup

```json
{
  "mcpServers": {
    "hsip": {
      "command": "/path/to/hsip-mcp",
      "env": {
        "HSIP_API_KEY": "hsip_...",
        "HSIP_API_URL": "http://127.0.0.1:7474"
      }
    }
  }
}
```

Create a dedicated agent key first: `hsip agent register claude`

### Before adding new MCP tools

1. Add the tool definition to `tool_list()` in `crates/hsip-mcp/src/main.rs`
2. Add a match arm in `call_tool()`
3. Use `api.post()` or `api.get()` — the `ApiClient` handles auth and error formatting

---

## Federated Trust (`/v1/trust/*`)

Allows an HSIP node to store trusted peers' Ed25519 verify keys by human-readable label. Any message the peer signed can then be verified locally without sharing raw key bytes.

Implementation: `crates/hsip-api/src/routes/trust.rs`

- `add()` — validates key bytes, upserts to `trusted_peers`, writes `trust.peer_added` audit entry
- `list()` — SELECT from `trusted_peers` ordered by `added_at DESC`
- `remove()` — DELETE by id + tenant, writes `trust.peer_removed` audit entry
- `verify()` — looks up peer by label, decodes key + signature via `ed25519_dalek::VerifyingKey::verify()`, writes `trust.verify_ok` or `trust.verify_failed` audit entry

CLI: `hsip trust add/list/remove/verify` in `crates/hsip-cli/src/commands/trust.rs`

Dashboard: `dashboard/src/pages/Trust.jsx` (Expert mode) — see "Dashboard" below.

---

## Mutual TLS (`[server.tls] client_ca_path`)

HSIP has no dedicated node-to-node network protocol — federated trust above is offline key registration plus local signature verification, not a live channel between HSIP instances. THREAT_MODEL.md nonetheless flagged a real, generic gap: HSIP's HTTPS server (`[server.tls]`) only ever authenticated itself to clients, never the reverse — any TLS client holding a bearer token could connect, with nothing enforced at the transport layer. `client_ca_path` closes that: when set to a CA certificate file, the server requires and verifies a client certificate signed by that CA before completing the TLS handshake, on top of (not instead of) the existing bearer-token auth every request still goes through. This is what an operator running multiple HSIP nodes (or a partner/regulator's system) that connect to each other's APIs over HTTPS would configure identically on both ends — it isn't HSIP-to-HSIP-specific under the hood, since no such specific protocol exists to restrict it to, but it's the direct fix for the federated-nodes gap.

**Implementation:** `crates/hsip-api/src/mtls.rs::build_rustls_config` — `client_ca_path: None` (default) takes the exact same `RustlsConfig::from_pem_file` code path as before this existed, byte-for-byte unchanged. `client_ca_path: Some(path)` instead hand-builds a `rustls::ServerConfig` with `rustls::server::WebPkiClientVerifier` requiring every client to chain to a CA in that file, via `axum_server::tls_rustls::RustlsConfig::from_config` (the constructor that accepts a pre-built `Arc<rustls::ServerConfig>`).

**A required fix surfaced along the way:** `reqwest` (HTTP client, used for OpenTimestamps submission) enables rustls's `ring` crypto-provider feature while `axum-server` enables `aws-lc-rs` — both end up compiled into the same binary, so rustls can't auto-select a default and the *first* `ServerConfig::builder()`/`ClientConfig::builder()` call anywhere in the process panics with "Could not automatically determine the process-level CryptoProvider." This was a **pre-existing latent bug**: the plain (non-mTLS) TLS-enabled code path already called `ServerConfig::builder()` internally and would have hit this the moment anyone actually enabled `[server.tls]` in production — it just had never been exercised by any test or by this dev/desktop-mode-only environment. Fixed by explicitly installing the `aws-lc-rs` provider as process default at the very top of `main()`, before any TLS code runs.

**Operational gotcha, easy to hit:** client certificates must carry the `clientAuth` Extended Key Usage extension (OID 1.3.6.1.5.5.7.3.2) or `rustls-webpki` rejects them with a "certificate unknown" TLS alert — discovered during E2E verification when a quick `openssl req -x509 -newkey ...` test cert with no extensions failed even though it chained to the correct, trusted CA. Documented in `config.example.toml`'s commented-out example.

**Verified:** unit tests in `mtls.rs` (`load_client_verifier_*`) build a real CA via the system `openssl` CLI and check accept/reject on valid vs. garbage input — no mocked crypto. Full end-to-end verification against a real running server in server mode: a client cert signed by the configured CA (with `clientAuth` EKU) connects and gets a real `200 {"status":"ok",...}`; a cert signed by a *different* CA, and a request with no client cert at all, both fail at the TLS handshake — `curl` never even reaches the point of sending an HTTP request, confirming rejection happens at the transport layer, not the application layer.

**Not required, always opt-in:** omitting `client_ca_path` preserves today's server-only-TLS behavior exactly — no existing TLS-enabled deployment is affected until an operator deliberately sets it.

### Per-key client-certificate binding (`POST /v1/keys/:id/bind-client-cert`)

mTLS above authenticates the *connection* — every request over that connection, regardless of which bearer token it carries, must present a certificate chaining to `client_ca_path`. It doesn't tie a specific *key* to a specific *certificate*: any bearer token is still usable from any client cert the CA will sign, and a stolen bearer token still works unmodified from a completely different, otherwise-legitimate mTLS client. This closes that: an owner-role key can bind `api_keys.bound_client_cert_fingerprint` (SHA-256 of the DER-encoded certificate, hex) on any key in its own tenant, after which `auth.rs`'s `TenantId` extractor requires that *exact* certificate to have been presented on the connection carrying that key's bearer token — a captured/stolen bearer token alone is no longer sufficient for a key an operator has opted into binding.

**Implementation:**
- `crates/hsip-api/src/mtls.rs` — `ClientCertFingerprint(pub Option<String>)`, a per-connection value inserted into every request's `http::Extensions` by a new `ClientCertAcceptor`. `ClientCertAcceptor` wraps `axum_server::tls_rustls::RustlsAcceptor`, implementing `axum_server::accept::Accept` itself: it delegates the actual TLS handshake to the inner acceptor unchanged, then reads back the client certificate rustls already verified against `client_ca_path` during that handshake (`tokio_rustls::server::TlsStream::get_ref().1.peer_certificates()`), hashes its DER bytes (SHA-256, hex), and wraps the resulting service with `tower_http::add_extension::AddExtension` so every request on that connection carries the fingerprint. Only constructed when `client_ca_path` is configured — `main.rs`'s plain `axum_server::bind_rustls(...)` path for server-only TLS is untouched, preserving the same byte-for-byte-unchanged guarantee `build_rustls_config`'s `None` branch already made.
- `db.rs` — `api_keys.bound_client_cert_fingerprint TEXT`, nullable. `NULL` (the default, and the only state possible before this existed) means zero behavior change — a bearer token alone is sufficient, exactly as before. Added to `bin/hsip_migrate.rs`'s `TABLES` list per the standing invariant (a column absent from that list silently isn't migrated).
- `auth.rs::TenantId::from_request_parts` — the `SELECT` now also fetches `bound_client_cert_fingerprint`. When set, the request's `ClientCertFingerprint` extension (absent entirely on a non-TLS or non-mTLS-configured connection) must carry that exact value or the request is rejected `401` with `metrics::AUTH_FAILURES{reason="client_cert_mismatch"}` — checked right after the `pending_revocation` check, before replay protection and rate limiting.
- `routes/keys.rs::bind_client_cert` — `POST /v1/keys/:id/bind-client-cert`, owner-role gated (same privilege boundary as `create`/`revoke` in the same file — key management, including binding, is an owner-only action in a tenant). Binds the fingerprint from the *caller's own* current connection (`Option<Extension<ClientCertFingerprint>>` — `Option` because a non-mTLS caller has no such extension at all), never an arbitrary caller-supplied string — an owner can't brick another key by binding it to a fingerprint nobody can ever present, since the only fingerprint ever bound is one the binding request's own connection just proved possession of. `{"clear":true}` in the body removes an existing binding instead (restoring bearer-token-only auth for that key) and, unlike binding, needs no presented certificate. Writes `key.cert_bound`/`key.cert_unbound` audit entries.

**Verified:** 4 new integration tests in `tests/integration.rs` (`test_bind_client_cert_requires_owner_role`, `test_bind_client_cert_requires_a_presented_certificate`, `test_bind_client_cert_enforced_then_cleared`, `test_unbound_key_unaffected_by_absent_or_present_client_cert`) — since the test harness drives the router directly via `tower::ServiceExt::oneshot` rather than a real socket, these simulate `ClientCertAcceptor`'s effect by inserting the same `ClientCertFingerprint` extension onto the request directly (exactly what `AddExtension` does on a real connection), while `mtls.rs`'s own existing unit tests (real X.509 certs via the system `openssl` CLI) cover the handshake-level cert loading/verification this builds on. `test_bind_client_cert_enforced_then_cleared` proves the full lifecycle end-to-end: bind → no-cert rejected → wrong-cert rejected → exact-cert accepted → clear → unbound access restored.

---

## Information Disclosure via Error Messages

A full sweep of `ApiError::Internal(...)` construction sites (prompted by "what is exposed during debugging" during a QA pass on the whole project) found raw library/database error text reaching ordinary, non-privileged API callers in two independent ways — both closed, and now a standing invariant (see Key Invariants below).

1. **`errors.rs`'s `From<sqlx::Error>`/`From<anyhow::Error>` impls** embedded the error's `Display` text directly into the `ApiError::Internal` response body via the `?` operator's automatic conversion — any route using `?` on a `sqlx`/`anyhow` result leaked whatever that library chose to put in its error message (schema details, file paths, occasionally partial query text) to the HTTP caller. Fixed: both impls now log the real error server-side via `tracing::error!` and return a fixed `"internal server error"` message. `sqlx::Error::RowNotFound` still maps to a clean `404`, unchanged.
2. **A second, independent instance of the same bug**, found by grepping for the whole pattern rather than trusting fix #1 covered every site: several routes bypass the `From` impl entirely with a manual `.map_err(|e| ApiError::Internal(e.to_string()))` (or `format!("...: {e}")`), which fix #1 can't touch since no `?`-triggered `From` conversion is involved. Found and fixed in `auth.rs` (4 sites in the `TenantId` extractor — the highest-traffic code path in the codebase, now via a shared `internal_db_error()` helper), `routes/credentials.rs` (a JSON-serialization error), `routes/messages.rs` and `routes/identity.rs` (both wrapping `key_encryption::decrypt_signing_key`'s error on ordinary tenant-callable routes), and `routes/decisions.rs` (JCS canonicalization error on `POST /v1/decisions`).

**Deliberately left unchanged** — judged as legitimate operational/diagnostic output for their audience, not leaks: `routes/dns.rs`'s "Failed to start DNS resolver: {}" (local service status, not sensitive), and most of `routes/admin.rs`'s `ApiError::Internal(format!(...))` sites (root-admin-gated only — some, like `HSIP_ROTATION_HOOK` stderr, are already documented above as deliberately informative for that trusted, privileged caller).

**Verified:** `errors.rs` gained a `#[cfg(test)] mod tests` (previously had none) — `sqlx_error_detail_never_reaches_the_client`, `anyhow_error_detail_never_reaches_the_client`, `row_not_found_still_maps_to_a_clean_404`, `hand_written_internal_messages_are_unaffected` — all passing, alongside the full existing 41-test integration suite and 28-test unit suite with zero regressions.

---

## SQLite → PostgreSQL Migration (`hsip-migrate`)

`DATABASE_URL`/`config.toml`'s `[database] url` pointing at PostgreSQL had never actually worked — for a fresh install or a migration — until this. Two independent bugs, both invisible in this project's dev environment because it had only ever run against SQLite, found while building the migration tool and confirmed against a real, locally-installed PostgreSQL 16 instance:

1. **Integer overflow on every timestamp.** `db.rs` declared every millisecond-epoch column (`created_at`, `timestamp`, `expires_at`, etc.) as `INTEGER`. SQLite's only integer keyword is dynamically 8-byte, so this was never a problem there — PostgreSQL's `INTEGER` is a real 4-byte `int4` (max ~2.1e9), and a real epoch-ms timestamp (~1.7e12) overflows it. Confirmed directly: `CREATE TABLE t (x INTEGER); INSERT INTO t VALUES (1774038000000);` → `ERROR: integer out of range`. Fixed by widening every ms-epoch/wide column to `BIGINT` (identical storage on SQLite, correct on Postgres); small bounded values (0/1 flags, in-batch Merkle indices) stayed `INTEGER`. Companion bug: `uploads.data`/`*_anchors.ots_proof` used the SQLite/MySQL-only `BLOB` keyword — doesn't exist in Postgres (`ERROR: type "blob" does not exist`) — fixed with `BYTEA`, which both backends accept.
2. **Every parameterized query was a Postgres syntax error.** HSIP uses `sqlx::Any` specifically so the same SQL runs against either backend, and every one of ~150 parameterized queries used `?` placeholders. `sqlx::Any` does **not** rewrite placeholder syntax per backend — `?` is a syntax error on Postgres outside a string literal. Fixed by rewriting every `?` to PostgreSQL-style numbered placeholders (`$1, $2, ...`) across 19 files / 100 statements (mechanical, scripted rewrite — one instance per query, in `.bind()` call order). Confirmed empirically that SQLite accepts `$N` placeholders identically to `?` (same positional bind semantics), so this single rewrite works unchanged on both backends — no runtime backend detection needed.

**Non-negotiable going forward — see Key Invariants below:** every new `sqlx::query(...)` call must use `$1, $2, ...` placeholders, never `?`; every new `db.rs` column storing a millisecond-epoch or similarly wide value must be `BIGINT`, never `INTEGER`; every new binary/blob column must be `BYTEA`, never `BLOB`.

**`hsip-migrate` (new binary, `crates/hsip-api/src/bin/hsip_migrate.rs`, registered as a second `[[bin]]` in `hsip-api/Cargo.toml`):**

```bash
cargo build -p hsip-api --bin hsip-migrate
./target/release/hsip-migrate --from sqlite:hsip.db --to postgresql://user:pass@host/db [--yes] [--force]
```

Connects to both, creates the target schema by calling the exact same `db::run_migrations` the server itself runs at startup (target schema can never drift from what the server expects), refuses to proceed into a non-empty target without `--force`, copies every table's rows inside one target-side transaction, verifies row counts match on both sides post-copy. Never writes to the source database. `TABLES` (a `Table { name, columns }` list mirroring `db.rs`'s schema) drives the copy — **adding a new table to `db.rs` also requires adding it to this list**, or it silently won't be migrated.

**Verified end-to-end, not just unit-tested:** real `hsip-api` server run against SQLite (identity, messages, consent, contact, decision attestation, second API key — 7 audit entries, valid hash chain) → `hsip-migrate` run against the populated database → fresh `hsip-api` process started against the migrated PostgreSQL database with the *original* master key and admin key files → same tenant ID and verify key returned (encrypted signing key round-trips through migration under the same master key), original admin bearer token still authenticates, `GET /v1/audit/verify` reports all entries as a valid unbroken chain, and the background anchor job successfully wrote to the `anchor_identity` singleton table and a `decision_anchors` row (`BYTEA ots_proof`) against Postgres.

A `#[ignore]`-by-default regression test, `crates/hsip-api/tests/postgres_compat.rs` (run explicitly with `HSIP_TEST_POSTGRES_URL` set), locks in both fixes without adding a live-Postgres dependency to the normal `cargo test --workspace` run — every other test in this repo is SQLite-only.

---

## Including hsip-verify in the Build

`crates/hsip-verify` — formal verification of HSIP's core security properties (consent non-forgery, temporal consistency, identity-binding soundness) via the Z3 SMT solver — was excluded from the root `Cargo.toml`'s `members` list since it depends on `z3-sys`, which builds the Z3 SMT solver from source (via `cmake`) the first time it's compiled. It's now a normal workspace member.

**What changed, and what didn't:** the crate itself needed no code changes — `cargo build -p hsip-verify` (the "build separately" workaround this repo used to document) already worked standalone. The only actual blocker was environmental: this session's sandbox ran out of disk space (`error: failed to build archive... No space left on device`) partway through compiling `z3-sys`, because a redundant, separately-resolved `target/` directory for `hsip-verify`'s standalone build (from being built outside the workspace) had accumulated alongside the main workspace `target/`. Deleting that stale standalone build directory freed enough space for a clean `cargo build --workspace` to succeed, building `hsip-verify` in the same pass as everything else with no dependency-resolution conflicts against the rest of the workspace (`z3`, `serde`, `chrono`, `ed25519-dalek`, etc. all resolved to versions compatible with what the rest of the workspace already used).

**The real, ongoing tradeoff — not eliminated, just accepted:** a from-scratch `cargo build --workspace` now takes noticeably longer (~8 extra minutes) because `z3-sys` compiles the actual Z3 C++ source tree via `cmake` on first build. This is a one-time cost per clean `target/` directory — Cargo caches the compiled artifacts exactly like any other dependency, so incremental builds (the overwhelming majority of local dev iterations and CI cache-hit runs) are unaffected. Requires `cmake` and a C++ toolchain (`g++`/`gcc`) to be present wherever this workspace is built — both were already implicitly required by other native dependencies in this workspace (e.g. `curve25519-dalek`'s optional asm backends), so this isn't a new category of build requirement, just a heavier instance of one that already existed.

**Verified:** `cargo build --workspace` and `cargo test --workspace` both succeed with `hsip-verify` included — its 9 unit tests (`src/models.rs`, `src/counterexample.rs`) plus 10 integration tests (`tests/verification_tests.rs`) all run real Z3 SMT queries (not mocked) and pass, alongside every other crate's existing test suite.

---

## Auto-Discovery (`GET /v1/agents/discover`)

Probes 12 well-known localhost ports with `tokio::spawn` + 150ms TCP timeout. Returns `DiscoveredAgent` list with `port`, `url`, `hint`, `description`, `already_registered`, `suggested_name`.

Implementation: `crates/hsip-api/src/routes/agents.rs` — `discover()` async handler. Registered on the router in `routes/mod.rs` as `.route("/v1/agents/discover", get(agents::discover))`.

Ports probed: Ollama :11434, Vite :5173, wrangler :8787, generic HTTP :8080, uvicorn :8000, Flask :5000, generic agent :4000/:9000, dashboard :3001, dev-api :3000, LM Studio :1234, Jupyter :8888.

CLI: `hsip agent discover` — `AgentCmd::Discover` in `crates/hsip-cli/src/commands/agent.rs`, calls `GET /v1/agents/discover`.

---

## Audit Log Hash Chain (`GET /v1/audit/verify`)

Every write to `audit_entries` extends a per-tenant BLAKE3 hash chain, so tampering with or deleting a row after the fact breaks every link after it — detectable without trusting the database's own account of what happened. Same threat this closes as Decision Attestations below, applied to the general audit trail instead of just decisions.

**Implementation:**
- `crates/hsip-api/src/audit_log.rs` — `record()` is the *only* way to write to `audit_entries`. Computes `entry_hash = BLAKE3(prev_hash || id || tenant_id || action || peer_verify_key || details || timestamp)`, chained per tenant. Retries on `UNIQUE(tenant_id, prev_hash)` conflict up to `MAX_ATTEMPTS`, same optimistic-concurrency pattern as `routes::decisions::record`. `verify_chain()` recomputes and checks a chain given its rows.
- `crates/hsip-api/src/routes/audit.rs::verify_chain` — `GET /v1/audit/verify` handler. Fetches the tenant's rows in chain order, recomputes the chain server-side, returns `{ valid, checked, unchained, first_break_id }`.
- `db.rs` — `audit_entries` has `prev_hash`/`entry_hash` columns (nullable — pre-migration rows have neither) and a `UNIQUE(tenant_id, prev_hash)` index enforcing the chain can't fork under concurrent writes.
- Chain starts at upgrade time, not tenant creation — rows from before this feature existed are counted in `unchained`, not treated as breaks.

**Do not** insert into `audit_entries` directly from a route handler — always call `audit_log::record()`, or the row won't be chained and `verify_chain` will report it as `unchained` (harmless but defeats the point of adding it).

### External Anchoring (`GET /v1/audit/:id/proof`, `POST /v1/audit/verify-proof`)

The BLAKE3 chain above is self-verifiable but, on its own, only proves internal consistency — an attacker with DB write access could still delete the whole chain and nothing left behind would detect it. This closes that gap the same way Decision Attestations (below) are anchored, applied to `audit_entries` instead of `decisions`.

**Implementation:**
- `crates/hsip-api/src/anchor_job.rs::run_audit_anchor_cycle`/`run_audit_anchor_cycle_with_calendars` — twin of `run_anchor_cycle` for decisions. Batches `audit_entries` where `anchor_id IS NULL AND entry_hash IS NOT NULL` (rows predating the hash chain have nothing to anchor and are excluded, same as `verify_chain` counts them `unchained`) into an RFC 6962 Merkle tree keyed on `entry_hash`, signs the root with the same node-level `anchor_identity` key used for decisions (anchoring isn't decision-specific — one node identity signs everything this node anchors), submits to OpenTimestamps, stores the batch in a new `audit_anchors` table (same shape as `decision_anchors`). Writes one `audit.anchored` audit entry per tenant touched — this naturally becomes part of a *future* anchor batch itself, not the current one (the `UPDATE` stamping `anchor_id` on this cycle's rows already ran before that entry is written).
- Same cadence as decisions: `anchor_job::BATCH_SIZE_TRIGGER` (50) / `INTERVAL_TRIGGER_MS` (5 min), same `retry_pending_audit_ots_submissions` retry-on-next-cycle behavior if calendars are unreachable (`ots_status = 'calendar_unreachable'`).
- `crates/hsip-api/src/main.rs` — the same 10s-poll spawned task that drives decision anchoring also calls `run_audit_anchor_cycle` each tick; most ticks are a no-op for both.
- `crates/hsip-api/src/routes/audit.rs::proof` — `GET /v1/audit/:id/proof`. Same shape as `routes::decisions::proof`: reconstructs the batch's leaf set from `entry_hash`es and regenerates the inclusion proof on demand (not stored). `anchored: false` with no Merkle/anchor fields if the entry predates the chain or hasn't been picked up by a cycle yet.
- `crates/hsip-api/src/routes/audit.rs::verify_proof` — `POST /v1/audit/verify-proof`. **Takes no `TenantId`, no `State`, makes no DB call** — same "pure function, runnable by anyone" design as `routes::decisions::verify`. Recomputes `entry_hash` via `audit_log::compute_entry_hash` (exposed `pub(crate)` for this) from caller-supplied fields, then checks Merkle inclusion and the anchor signature if those are also supplied.
- `metrics::AUDIT_ANCHORED` (labels: `ots_status`) — twin of `metrics::DECISIONS_ANCHORED`.

---

## Decision Attestations (`/v1/decisions/*`)

Signs and hash-chains AI-agent decisions (starting with trading decisions) into a tamper-evident, independently verifiable record — proving *which* identity produced a decision without requiring anyone to trust HSIP's own database. Informed by the VeritasChain Protocol (VCP): RFC 6962 Merkle trees, RFC 8785 JCS canonicalization, Ed25519 signing. VCP-TRADE/VCP-GOV have no published schema as of this writing, so the accountability fields below are HSIP's own draft, tagged `hsip_gov_ext`, meant to be reconciled if/when VSO publishes one.

**Two-tier record by design:**
- Clear accountability metadata: `model_version`, `strategy_id`, `accountable_key`, `decision_type` — the part a regulator or auditor asks about first.
- Opaque `payload_hash`: SHA-256 of the caller's actual (never disclosed to HSIP) decision content. HSIP never receives or stores trade parameters, prices, sizes, etc. — only their hash. Disclosure of the preimage, if ever needed, happens entirely on the caller's side.

**`accountable_key` proof-of-possession (optional):** `accountable_key` used to be pure caller-asserted metadata — nothing checked that whoever submitted the decision actually controlled that key. `RecordDecisionRequest.accountable_key_signature` (optional, base64 Ed25519) closes that: a signature by `accountable_key`'s own private key over `hsip_core::canonical::accountable_proof_preimage_hash(accountable_key, tenant_id, model_version, strategy_id, decision_type, payload_hash)` — every field the caller can compute client-side before submitting (unlike `decision_id`/`prev_hash`/`timestamp_*`, which the server assigns and which can change across a hash-chain retry attempt). `tenant_id` is deliberately part of the preimage: without it, a real signature obtained for one tenant's decision could be replayed verbatim by a different tenant (or a different HSIP deployment) reusing the same `{model_version, strategy_id, decision_type, payload_hash}` values, none of which are secret — `payload_hash` itself is returned in every proof bundle and decision listing. A residual, lower-severity replay remains *within* the same tenant (a co-tenant agent reusing the exact same decision-content fields to misattribute a decision to the same `accountable_key`); closing that fully needs a per-decision server-issued nonce (a two-phase challenge/response protocol), out of scope for this pass — see THREAT_MODEL.md §4.24.

- `hsip-core::canonical::accountable_proof_preimage_hash` — the single source of truth for this formula (`SHA256(JCS({accountable_key, tenant_id, model_version, strategy_id, decision_type, payload_hash}))`), called identically by `record()` (verify before persisting), `proof()` (re-derive for the bundle), and `verify()` (independent, DB-free re-check) via `routes::decisions::verify_accountable_proof` — same "one formula, one place" pattern as `audit_log::compute_entry_hash`.
- Omitting `accountable_key_signature` is a pre-existing, still-valid call shape — additive, not breaking. `DecisionEnvelope::accountable_key_signature` (`#[serde(default)]`, empty string when unsupplied) is part of the canonical, signed envelope, so a third party re-running `verify()` sees exactly what was (or wasn't) claimed, not a server-side-only fact.
- A *claimed* but non-verifying signature is rejected outright (`400`, before any DB write) — "record it anyway but mark unverified" would let a caller silently think a decision was accountability-proven when it wasn't.
- `RecordDecisionResponse`/`DecisionSummary`/`DecisionProofBundle` all carry `accountable_key_verified: bool`; `VerifyDecisionResponse` carries `accountable_key_verified: Option<bool>` (`None` = no proof was ever claimed, doesn't invalidate the bundle; `Some(false)` = a claimed proof failed to verify, does invalidate it — folded into `valid` the same way Merkle-inclusion/anchor-signature checks already are).
- `db.rs` — `decisions.accountable_key_signature TEXT`, nullable (ignored-error `ALTER TABLE ADD COLUMN`). Added to `bin/hsip_migrate.rs`'s `TABLES` list per the standing invariant.
- **Verified two ways:** 3 new integration tests (valid proof recorded and independently re-verified through `proof()`/`verify()`; a claimed-but-wrong signature rejected `400`; omitting the field still records, `accountable_key_verified: false`) plus a real running-server check — recorded a decision with a genuine Ed25519 proof via the Python SDK, confirmed `record()`/`proof()`/`verify()` all agree it verified, confirmed a signature genuinely produced under a *different* `tenant_id` (proving the cross-tenant-replay binding actually works, not just compiles) is correctly rejected.
- **Python SDK:** `HSIPClient.accountable_proof_preimage_hash(...)` (static) computes the exact bytes to sign — this SDK has no cryptography dependency by design (pure stdlib), so signing itself is left to whatever Ed25519 library the caller already uses (`pynacl`, `cryptography`, etc.); `record_decision(..., accountable_key_signature=None)` passes the result through. Confirmed byte-for-byte identical to the Rust `accountable_proof_preimage_hash` output for the same inputs (all fields here are plain ASCII strings, so Python's `json.dumps(sort_keys=True, separators=(',',':'), ensure_ascii=False)` already matches RFC 8785 JCS for this specific flat shape — no general JCS number/unicode-escaping edge cases apply). **Node/Go SDK parity not yet ported** — deferred the same way Node/Go's own decision-attestation methods were originally deferred until a concrete caller needed them; port following the exact shape of the Python method above when one does.

**Implementation:**
- `crates/hsip-core/src/canonical.rs` — `DecisionEnvelope` struct + `canonical_bytes()`/`event_hash()` (JCS canonicalization via `serde_jcs`, then SHA-256). `timestamp_int` is kept as a *string* field, not a JSON number, to avoid IEEE-754 precision loss on large timestamps.
- `crates/hsip-core/src/merkle.rs` — pure RFC 6962 Merkle tree (`MerkleTree`, `leaf_hash`/`node_hash` with `0x00`/`0x01` domain-separation prefixes), inclusion proof generation and verification (`verify_inclusion`). No I/O.
- `crates/hsip-api/src/routes/decisions.rs`:
  - `record()` — `POST /v1/decisions`. Resolves the authenticated `api_keys` row, validates fields, chains to the tenant's last decision via `prev_hash`, signs `event_hash` with the tenant's Ed25519 identity. Retries on `UNIQUE(tenant_id, prev_hash)` conflict (another request extended the chain first) up to `MAX_ATTEMPTS`. Writes `decision.recorded` audit entry.
  - `list()` — `GET /v1/decisions`.
  - `proof()` — `GET /v1/decisions/:id/proof`. Returns the full self-contained bundle. If unanchored yet, `anchored: false` with signature-only proof. If anchored, reconstructs the batch's leaf set and regenerates the inclusion proof on demand (not stored — recomputed from `decisions.anchor_id`/`merkle_index`).
  - `verify()` — `POST /v1/decisions/verify`. **Takes no `TenantId`, no `State`, makes no DB call** — a pure function of its request body. This is the function meant to be run independently of HSIP entirely (by Predicta, a regulator, an acquirer's engineering review).
- `crates/hsip-api/src/anchor.rs` — OpenTimestamps calendar HTTP client. `submit_digest_to()` submits a batch's Merkle root to public calendars, stores the raw response as an opaque blob. `check_for_upgrade()` (`GET <calendar>/timestamp/<hex-digest>`) later asks the same calendar whether that submission has since been confirmed by a mined Bitcoin block. `contains_bitcoin_attestation()` detects confirmation via presence of OpenTimestamps' `BitcoinBlockHeaderAttestation` byte tag; `extract_pending_calendar_uri()` reads the originating calendar's URL back out of the stored `PendingAttestation` proof (no separate calendar-URL column needed). **MVP scope**: still does not parse or fully verify the `.ots` binary format's Merkle-path operations — confirmation detection is a tag-presence check, trusting the calendar's response the same way the initial "pending" submission already does. Calendar list/URL is always a parameter so tests can point at a `wiremock` server instead of the real network.
- `crates/hsip-api/src/anchor_job.rs` — `run_anchor_cycle()` (spawned on a timer in `main.rs`, ~10s poll). Anchors on a "whichever comes first" cadence: `BATCH_SIZE_TRIGGER` (50) unanchored decisions, or `INTERVAL_TRIGGER_MS` (5 min) elapsed with at least one waiting. Builds a `MerkleTree`, signs the root with the node-level `anchor_identity` key (**not** any tenant's identity — an anchor batch spans every tenant), submits to OpenTimestamps. If calendars are unreachable, local Merkle anchoring still proceeds (`ots_status = 'calendar_unreachable'`) and `retry_pending_ots_submissions()` retries on the next cycle. Writes one `decision.anchored` audit entry per tenant touched by the batch. The same file's `run_audit_anchor_cycle()` does the identical thing for `audit_entries` instead of `decisions` — see Audit Log Hash Chain → External Anchoring above. `run_upgrade_cycle()` (spawned on its own, much slower 15-minute timer — Bitcoin blocks land roughly every 10 minutes on average, so polling every 10s like submission does would just hammer the calendars) checks every `ots_status = 'pending'` row in both `decision_anchors` and `audit_anchors`, and flips it to `'confirmed'` once its calendar reports a Bitcoin attestation.

**Trust model — the signing-to-anchoring gap:** a signature proves authorship; it does not by itself prove the record wasn't deleted or reordered before the next anchor cycle publishes the batch's root externally (to OpenTimestamps/Bitcoin). That gap is bounded by the anchor cadence, and further mitigated client-side — see SDK `save_receipt()` below, which persists the signed receipt independently of this server the moment it's received.

**OpenTimestamps calendar submission is verified end-to-end against real calendars**, not just the mocked unit tests — previously blocked in every sandboxed dev environment this project had been built in (outbound HTTPS to `*.calendar.opentimestamps.org` 403'd at the egress proxy, confirmed via the proxy's own rejection log). The repo owner ran a real `hsip-api` server from an unrestricted Windows 11 network, confirmed raw connectivity to all three `DEFAULT_CALENDARS` (`Invoke-WebRequest POST <calendar>/digest`, real `HTTP 200`s), then recorded a real decision and let `anchor_job.rs` submit it for real — `GET /v1/decisions/:id/proof` came back `ots_status: "pending"` with a genuine calendar receipt, independently confirmed by decoding the `ots_proof` bytes and finding the calendar's own URL (`alice.btc.calendar.opentimestamps.org`) embedded in its response. See THREAT_MODEL.md §4.20 for the full writeup.

**The "upgrade" step (polling for a Bitcoin-confirmed proof) is now implemented** — `anchor_job::run_upgrade_cycle`. The `PENDING_ATTESTATION_TAG`/`BITCOIN_ATTESTATION_TAG` byte constants in `anchor.rs` are confirmed correct against the real captured `alice.btc.calendar.opentimestamps.org` response from the §4.20 verification (the pending tag's bytes, and the length-prefixed calendar-URI payload immediately following it, exactly match that real response — see `anchor.rs`'s `REAL_PENDING_PROOF` unit test fixture). The actual *upgrade* itself (a calendar reporting Bitcoin confirmation) has not yet been live-tested against a real calendar — that requires waiting out a real submission's confirmation window (potentially hours), which real-network verification hasn't covered yet. Covered instead by `tests/integration.rs::test_decision_anchor_upgrades_to_bitcoin_confirmed`, an end-to-end test against a mocked calendar that submits, anchors, mocks a confirmed response, runs the upgrade cycle, and confirms `ots_status` flips to `"confirmed"` both in the DB and through `GET /v1/decisions/:id/proof`.

---

## Browser Extension

Location: `browser-extension/`. Manifest V3. Works in Chrome, Edge, Firefox.

- Blocks 61 tracker domains via `declarativeNetRequest`
- Badge count of trackers blocked on current tab
- Shows HSIP server connection status (green dot when alive)
- Shows last 5 AI agent audit entries when HSIP is connected

```
manifest.json   Manifest V3 — permissions, host_permissions, background, content_scripts
background.js   Service worker: heartbeat every 30s, fetches /v1/audit, caches in chrome.storage.local
content.js      Scans PerformanceResourceTiming entries for tracker domains
popup.html/js   Extension popup UI (320px wide)
rules.json      61 declarativeNetRequest block rules
icons/          SVG icons at 16, 48, 128px
```

Install (dev): `chrome://extensions` → Developer mode → Load unpacked → select `browser-extension/`

Extension targets `http://127.0.0.1:7474` (desktop default). `manifest.json` also covers port 3000.

---

## Dashboard

`dashboard/` — React + Vite. **This is the active production dashboard.**

- Dev server: port 3001, proxies `/v1/*` → `http://localhost:3000`
- `npm run build` → `dashboard/dist/` → embedded via `rust-embed` with `--features embed-dashboard`
- `dashboard/src/api.js` — single `request()` helper used by all pages

**Navigation is a Simple/Expert mode toggle** (`dashboard/src/App.jsx`), not progressive disclosure — an earlier revision of this file documented a progressive-disclosure refactor (`PRIMARY_TABS`/`ADVANCED_TABS`, a `showAdv` toggle, no mode split) that a later commit (`830117a`, "comprehensive UI visual redesign") reintroduced the mode split on top of, without this file being updated to match. Corrected here rather than re-doing that refactor — the mode split is the actual, currently-shipped design:
- `SIMPLE_TABS` (10 tabs) — the "For Everyone" consumer-facing mode: Home, Finance, Messages, Traffic, Alibi, Consents, AI Watch, AI Decisions, Trackers, Protection.
- `EXPERT_TABS` (10 tabs) — the "Developer" mode: Identity, Consent, Messages, Credentials, Decisions, Trust, Discover, Audit, Keys, Admin.
- `mode` state, persisted to `localStorage('hsip_mode')`; `switchMode()` resets the active tab to that mode's default (`home` / `identity`). The toggle lives on the login screen (`For Everyone` / `Developer` buttons) and again in the sidebar footer once signed in.

**Expert-mode pages added to close prior dashboard gaps** (`dashboard/src/pages/`):
- `Trust.jsx` — `GET /v1/trust/peers` list, add (`POST /v1/trust/peer`) / remove (`DELETE /v1/trust/peers/:id`) a peer, and a "verify a signature from a trusted peer" tool (`POST /v1/trust/verify`).
- `Discover.jsx` — `GET /v1/agents/discover` results with a one-click "Register key" button per unregistered agent (`POST /v1/keys` with `agent_type: "ai_agent"`).
- `Admin.jsx` — master key fingerprint + rotate (`GET`/`POST /v1/admin/master-key/*`, with a confirm step before rotating) and root-admin list/grant/revoke (`GET/POST /v1/admin/root-admins*`). Both sections independently surface "your key isn't a root admin" rather than a raw error if the signed-in key lacks the flag.
- `Keys.jsx` — now shows and sets `role` (`owner`/`member`) per key, previously invisible in the UI.
- `Audit.jsx` — now shows a hash-chain-intact/broken indicator (`GET /v1/audit/verify`'s `valid`/`checked`/`unchained`/`first_break_id`) above the log table, with a manual re-check button.
- `Decisions.jsx` (pre-existing, not new) already covers anchor/proof status — a stale roadmap item claiming this was missing has been corrected.

**A real bug found while building the Trust page, not a pre-existing doc-only issue:** `trusted_peers` — the table every `routes/trust.rs` handler queries — was never created by `db::run_migrations()`. Every `/v1/trust/*` call 500'd with "no such table" on any fresh database since the federated-trust feature shipped; nothing had ever exercised it end-to-end before the dashboard's new Trust page did. Fixed in `db.rs` (added the table + its tenant index) and `bin/hsip_migrate.rs` (added to the migration tool's `TABLES` list, per the invariant below). Covered now by `tests/integration.rs::test_trust_add_list_verify_remove` — add/list/verify (valid and tampered signature)/remove, end-to-end over the real HTTP stack — where zero trust-route tests existed before.

---

## SDKs

`sdks/python/`, `sdks/node/`, `sdks/go/` — all updated to parity with agent governance APIs.

### Methods added to all three SDKs

| Method | API call |
|---|---|
| `register_agent(name, expires_days?)` | `POST /v1/keys` with `agent_type: "ai_agent"` |
| `list_agents()` | `GET /v1/agents` |
| `revoke_agent(name_or_id)` | resolves name → id, then `DELETE /v1/keys/:id` |
| `log_action(message)` | `POST /v1/messages/sign` with `[ACTION:...]` prefix |
| `discover_agents()` | `GET /v1/agents/discover` |

Python: `sdks/python/hsip/client.py`
Node: `sdks/node/src/index.js` + `sdks/node/src/index.d.ts`
Go: `sdks/go/hsip/client.go`

### Decision attestation methods — now in all three SDKs

| Python | Node | Go | API call |
|---|---|---|---|
| `hash_payload(payload: bytes)` (static) | `HSIPClient.hashPayload(payload)` (static) | `hsip.HashPayload(payload []byte) string` (package function, not a method — needs no client state) | Hex SHA-256 in the format `record_decision`/`recordDecision`/`RecordDecision` expects |
| `record_decision(accountable_key, model_version, strategy_id, decision_type, payload_hash, accountable_key_signature=None, receipt_dir=None)` | `recordDecision({accountableKey, modelVersion, strategyId, decisionType, payloadHash, receiptDir})` | `RecordDecision(RecordDecisionOpts{...})` | `POST /v1/decisions`; if a receipt dir is given, also calls the `save_receipt` equivalent. `accountable_key_signature` (Python only so far — see `accountable_proof_preimage_hash` below) is optional proof-of-possession, additive to existing callers |
| `save_receipt(receipt, receipt_dir)` (static) | `HSIPClient.saveReceipt(receipt, receiptDir)` (static) | `hsip.SaveReceipt(receipt, receiptDir)` (package function) | Writes `<receipt_dir>/<decision_id>.json` — the client-side mitigation for the signing-to-anchoring trust gap |
| `list_decisions()` | `listDecisions()` | `ListDecisions()` | `GET /v1/decisions` |
| `get_decision_proof(decision_id)` | `getDecisionProof(decisionId)` | `GetDecisionProof(decisionID)` | `GET /v1/decisions/:id/proof` |
| `verify_decision(bundle)` | `verifyDecision(bundle)` | `VerifyDecision(&VerifyDecisionRequest{...})` | `POST /v1/decisions/verify` — no API key, no DB; a pure function of `bundle` |
| `accountable_proof_preimage_hash(accountable_key, tenant_id, model_version, strategy_id, decision_type, payload_hash)` (static, Python only so far) | — (not yet ported) | — (not yet ported) | No API call — computes the exact bytes to sign for `accountable_key_signature`. This SDK has no cryptography dependency by design (pure stdlib); sign the returned bytes with whatever Ed25519 library the caller already uses and pass the result to `record_decision` |

Node/Go were deferred until Predicta's integration language was confirmed — ported once needed, following the exact shape of the Python methods above (same field names, snake_case in the wire JSON either way — only the local binding style changes: Python kwargs, a Node options object, a Go `RecordDecisionOpts`/`VerifyDecisionRequest` struct). `hashPayload`/`saveReceipt` (Node) and `HashPayload`/`SaveReceipt` (Go) need no server connection, so they're static/package-level rather than instance methods, matching Python's `@staticmethod`. Verified end-to-end against a real running server for both: recorded a decision, confirmed the receipt was written to disk, listed it back, fetched its proof, verified it (`valid: true`), and confirmed a tampered `event_hash` correctly comes back `valid: false` — for each language, not just one. `accountable_proof_preimage_hash`/`accountable_key_signature` are Python-only as of this writing — same "port once a concrete caller needs it" reasoning as the original Node/Go deferral.

### Opt-in HTTP replay protection — now sent by a real caller (Python SDK)

The server-side check (`auth.rs::check_replay_protection`, see HTTP Replay Protection above) existed with no HSIP-authored client actually sending the headers. `HSIPClient(api_key, base_url, replay_protection=False)` — a new constructor flag, default `False` (this SDK's only behavior before the flag existed, so no existing caller is affected unless it opts in). When `True`, every request carries `x-hsip-timestamp` (current Unix seconds) and a fresh `x-hsip-nonce` (`secrets.token_hex(16)`, regenerated per request — reusing one would self-lock the caller out on its own second call). Node/Go SDKs don't send these yet — same "port once needed" deferral as `accountable_key_signature` above.

**Verified against a real running server:** `replay_protection=False` behaves identically to before; `replay_protection=True` succeeds on repeated calls (confirming a fresh nonce each time, not an accidental self-collision); and — the actual replay scenario this exists for — two raw requests sent with the *same* fixed `x-hsip-timestamp`/`x-hsip-nonce` pair got `200` then `401 "Duplicate x-hsip-nonce for this key — request already processed"`, confirming the server-side defense that already existed is now reachable through a real client, not just curl.

---

## Security Self-Review (audit prep, not a substitute for a real third-party audit)

A full-codebase sweep — every route handler, the `TenantId` auth extractor, key-encryption code, admin/rotation code, `hsip-cli`, `hsip-mcp` — done as prep for the eventual third-party audit (THREAT_MODEL.md §7/§8 still list that as not-yet-done; this is self-review by the same person who built the features, which cannot substitute for independent review). Every candidate finding was independently re-verified against real running-server or running-process behavior before being treated as confirmed, not just re-read as code. Full writeup: THREAT_MODEL.md §4.19.

Three confirmed, fixed:

1. **Stored XSS via SVG upload (`routes/uploads.rs`)** — `POST /v1/uploads` accepted `image/svg+xml` (SVG is XML, can carry `<script>`/event-handler payloads), and the public, no-auth `GET /v1/uploads/:id` echoed the stored `content_type` back verbatim, so navigating to an uploaded SVG's URL ran the attacker's script same-origin as the dashboard's `localStorage`-held bearer token. Fixed: `upload()` now explicitly rejects `image/svg+xml` even though it matches the `image/*` prefix; `serve()` now also sends `x-content-type-options: nosniff` as defense-in-depth.
2. **Missing authentication on all `/v1/proxy/*` endpoints (`routes/proxy.rs`)** — all five handlers (`status`/`enable`/`disable`/`log`/`setup`) took no `TenantId` extractor, and this codebase has no global auth middleware — every route enforces auth individually via that extractor. Anyone reaching the HSIP port could enable/disable the local MITM proxy and read its full traffic log with zero credentials. Fixed: all five now take `_tenant: TenantId` as their first parameter, matching `dns.rs`'s equivalent handlers (not tenant-scoped beyond authentication — the proxy, like the DNS blocker, is a node-level resource).
3. **Hardcoded fallback JWT signing key (`hsip-cli/src/identity.rs`)** — `HSIP_KEY` fell back to a fixed hex string checked into this public repo whenever `HSIP_LOCAL_JWT_KEY_HEX` wasn't set; since `/token` requires no auth and accepts any caller-supplied `aud`, anyone who'd read this source could forge tokens for any relying party trusting an unconfigured `hsip identity-serve` broker. Fixed: unset key now falls back to a fresh 32-byte `OsRng`-generated key per process instead of a known constant.

Considered and explicitly not flagged: `credentials.rs::verify`'s cross-tenant-ID-only revocation lookup (intentional — caller already possesses the credential), `decisions.rs::verify`'s lack of anchor-key pinning (inherent to its documented pure-verification-function design), `hsip-mcp`'s `urlenc` Unicode-vs-UTF-8-byte percent-encoding bug (not reachable — its one call site is always base64/ASCII).

---

## System Health (`GET /v1/admin/system-health`)

Answers a question a QA review asked directly: **"can this recover automatically?"** has real "no" answers in this codebase — an incomplete master key rotation (§ Master Key Rotation above), a node with zero root-admin keys, an OTS anchor batch that's given up being auto-upgraded (§ Decision Attestations' `MAX_PENDING_UPGRADE_AGE_MS`). HSIP has no push-based alerting of its own (no email/webhook/PagerDuty — deliberately, same reasoning as `HSIP_ROTATION_HOOK`'s "HSIP never holds a new class of credentials"), so without something surfacing these, an operator — one person on a desktop, or a business running real infrastructure — would only find out by reading the database directly. Full writeup: THREAT_MODEL.md §4.22.

**Implementation:**
- `crates/hsip-api/src/system_health.rs` — `check()` runs three checks and returns `{healthy, checked_at_ms, issues: [{code, severity, summary, detail}]}`: `master_key_rotation_incomplete` (a `{master_key_path}.rotating` staging file still exists — critical), `zero_root_admins` (no active `is_root_admin=1` key — critical), `ots_anchors_abandoned` (count of `decision_anchors`/`audit_anchors` rows past `MAX_PENDING_UPGRADE_AGE_MS` still `pending` — warning). Pure, no metric side effects, so it's directly unit-testable.
- `check_and_update_metrics()` — same checks, plus refreshes `metrics::SYSTEM_HEALTH_ISSUES` (a `GaugeVec` by severity — a gauge, not a counter, so a resolved issue correctly drops back to zero).
- `routes::admin::system_health` — `GET /v1/admin/system-health`, root-admin gated like every other node-level admin route. Registered in `routes/mod.rs`.
- `main.rs` spawns a 5-minute background task calling `check_and_update_metrics` so `/metrics` stays current even if nobody polls the JSON endpoint — a business running real Prometheus alerting can fire on `hsip_system_health_issues{severity="critical"} > 0` without ever touching HSIP's own API.
- `hsip status` (CLI, `commands/agent.rs::status`) calls this endpoint first and prints any issues loudly at the top, before identity/agent/audit sections — the answer for the individual-desktop-user audience who isn't running Prometheus. A non-root-admin key gets a clear "unavailable, here's why" line instead of `hsip status` failing outright.

**A real bug found and fixed alongside this, not just a hypothetical:** three sites in `anchor_job.rs` (`retry_pending_ots_submissions`, `retry_pending_audit_ots_submissions`, `upgrade_one_anchor`) discarded their corrective `UPDATE`'s result via `let _ = ...` and logged/counted success unconditionally — a genuinely silent failure if that `UPDATE` ever actually failed. Fixed: all three now check `rows_affected() > 0` before declaring success; a failed or zero-rows update logs a warning instead and the row is naturally retried next cycle since its DB state never changed. Verified by a unit test that fetches a real row, deletes it, then calls the upgrade logic with the already-fetched row — a clean, realistic way to force a genuine zero-rows-affected `UPDATE` and confirm the success metric doesn't move.

**Verified end-to-end against a real running server**, not just unit tests: forced a genuine `zero_root_admins` state via direct DB edit, confirmed `hsip status` degraded gracefully; restored it; inserted a real 8-day-old pending anchor row; confirmed `GET /v1/admin/system-health`, `hsip status`, and `GET /metrics` all agreed on the same issue.

---

## Key Invariants — Do Not Break

- **Ed25519 private keys must always be encrypted** before writing to DB (`encrypt_signing_key()` in `key_encryption.rs`). Never store raw key bytes.
- **API keys stored as SHA-256 hashes only.** Never write the raw key to DB.
- **`pending_revocation` DashSet** must be updated before the async DB write when auto-revoking agent keys — blocks in-flight requests immediately.
- **Audit entries must be written** for all state-changing operations (identity creation, credential issuance/revocation, consent grant/revoke, key events, trust peer add/remove/verify).
- **In-memory SQLite tests require `max_connections = 1`** — each connection is a separate DB instance.
- **`crates/hsip-verify` is a normal workspace member** — `cargo build --workspace`/`cargo test --workspace` build and test it like any other crate. Don't re-add it to the root `Cargo.toml`'s `exclude` list; see "Including hsip-verify in the Build" below for why it was excluded before and what changed.
- **No migration files** — all schema is inline in `db::run_migrations()`. Add new tables/columns there.
- **Every SQL bind placeholder must be `$1, $2, ...` (PostgreSQL-numbered), never `?`.** `sqlx::Any` does not rewrite placeholder syntax per backend — `?` is a SQLite/MySQL-only token and is a hard syntax error on PostgreSQL. `$N` works identically on both backends (confirmed empirically — see "SQLite → PostgreSQL Migration" above), so there is never a reason to use `?` in this codebase.
- **Every `db.rs` column storing a millisecond-epoch timestamp or similarly wide value must be `BIGINT`, never `INTEGER`.** PostgreSQL's `INTEGER` is a real 4-byte `int4` (max ~2.1e9) and overflows on any real epoch-ms value (~1.7e12) — this silently broke every write against Postgres until found and fixed (see "SQLite → PostgreSQL Migration" above). Small bounded values (0/1 flags, in-batch Merkle indices) may stay `INTEGER`.
- **Every binary/blob column in `db.rs` must be `BYTEA`, never `BLOB`.** `BLOB` is a SQLite/MySQL-only type name and doesn't exist in PostgreSQL at all.
- **A new table added to `db.rs` must also be added to `bin/hsip_migrate.rs`'s `TABLES` list.** `hsip-migrate` doesn't discover tables dynamically — a table missing from that list silently isn't migrated.
- **CLI key resolution must use `commands::util::load_admin_key()`** — never write a local `load_admin_key()` in a command file.
- **`config.toml` must not be committed** — it forces server mode and breaks desktop-mode testing.
- **Decision payload content must never reach HSIP.** `routes::decisions::record` only ever accepts `payload_hash` (a caller-computed SHA-256 hex string) — never add a field for the actual decision content itself; that defeats the confidentiality design.
- **`hsip_core::canonical::accountable_proof_preimage_hash` must stay the single source of truth for the accountable_key proof-of-possession formula**, same reasoning as `audit_log::compute_entry_hash`. `routes::decisions::verify_accountable_proof` is the one place that calls it, and `record()`/`proof()`/`verify()` all call that — don't let any of them reimplement the JCS+SHA256 steps separately. `tenant_id` must stay part of the preimage — removing it reopens a cross-tenant signature-replay hole (see THREAT_MODEL.md §4.24).
- **A *claimed* but non-verifying `accountable_key_signature` must reject the write (`400`), never silently record with `accountable_key_verified: false`.** A caller who supplied a signature is asserting proof-of-possession; recording it anyway as "unverified" would look identical to a caller who never claimed proof at all, hiding a real tamper/bug from whoever reads the record later.
- **The anchor identity (`anchor_identity` table) must stay separate from tenant identities.** An anchor batch spans every tenant's decisions; do not sign an anchor root with any one tenant's key.
- **`POST /v1/decisions/verify` must stay DB-free and auth-free.** It's the one handler in `decisions.rs` deliberately without `TenantId`/`State` — a third party (regulator, acquirer's engineering review) needs to be able to run the equivalent check independently of this server. Don't add a database lookup to it.
- **`decisions` chain integrity relies on `UNIQUE(tenant_id, prev_hash)`.** Don't remove it or bypass it with raw inserts — it's what prevents the hash chain from forking under concurrent requests.
- **Never `INSERT INTO audit_entries` directly.** Always call `audit_log::record()` — it's what computes and links `prev_hash`/`entry_hash`. A raw insert produces an unchained (though still functionally fine) row that `GET /v1/audit/verify` will count as `unchained` rather than verify.
- **`POST /v1/audit/verify-proof` must stay DB-free and auth-free**, same reasoning and same invariant as `POST /v1/decisions/verify` above — it's the function a third party runs independently of this server. Don't add a database lookup to it.
- **`audit_log::compute_entry_hash` must stay the single source of truth for the entry-hash formula.** `audit_log::record()`, `audit_log::verify_chain()`, and `routes::audit::verify_proof` all call it — don't let `verify_proof` (or anything else) reimplement the BLAKE3 formula separately, or a formula change in one place silently breaks verification in the other.
- **`rate_limit_persistence::snapshot`/`load` must stay a periodic background job, not a write-through on every request.** The whole point of `rate_limiter`/`agent_tracker`/`sandbox_rate` being in-memory `DashMap`s is that the hot auth path never blocks on the database. Don't add a DB write inside `auth.rs::check_rate_limit`/`check_agent_velocity` themselves to "make persistence more accurate" — that defeats the reason they're in-memory at all.
- **A default rustls `CryptoProvider` must stay installed at the top of `main()`, before any TLS code runs.** `reqwest` and `axum-server` enable different rustls crypto-provider features (`ring` vs `aws-lc-rs`); with both compiled in, rustls can't auto-select one and the first `ServerConfig`/`ClientConfig` builder call anywhere in the process panics. Don't remove `rustls::crypto::aws_lc_rs::default_provider().install_default()` from `main()`, and mirror it (`ensure_crypto_provider()`) in any test that touches `rustls::ServerConfig` directly outside of `main()`.
- **`mtls.rs`'s `client_ca_path: None` path must stay byte-for-byte `RustlsConfig::from_pem_file`.** Don't route the no-mTLS case through the hand-built `ServerConfig` path "for consistency" — the whole backward-compatibility guarantee (every existing TLS-enabled deployment unaffected) depends on that branch being untouched.
- **Every new API route must actually be registered in `routes/mod.rs`.** A fully implemented handler with no `.route(...)` line compiles clean (Rust doesn't error on an unused `pub` function in a binary crate) and silently 404s — this has happened before (`agents::discover` shipped unwired for at least one prior revision of this file). If you add a handler, grep `routes/mod.rs` afterward to confirm it's there, not just that the crate builds.
- **CLI subcommands documented in this file must actually exist in the `Subcommand` enum.** `hsip agent discover` was documented here before the `AgentCmd::Discover` variant existed. Cross-check `crates/hsip-cli/src/commands/*.rs` against this file's command tables when either changes.
- **`state.master_key` is `Arc<RwLock<Vec<u8>>>`, not a plain `Arc<Vec<u8>>`.** Every read site must take a short-lived `.read().await` guard (don't hold it across unrelated `.await` points, especially network I/O — see the anchor loop in `main.rs` for the pattern of snapshotting instead). `routes::admin::rotate_master_key` is the only writer and holds `.write().await` for its whole operation, not just the final swap — see Master Key Rotation above for why.
- **`ALTER TABLE ... ADD COLUMN` migration lines must end in `.await;`, not `.await?;`.** The `let _ = sqlx::query(...).execute(pool).await?;` pattern still propagates the error via `?` even though the value is discarded — this broke every test with "duplicate column name" the first time it was gotten wrong in this file's history. Always double-check against the working `api_keys`/`audit_entries` examples already in `run_migrations()`.
- **Consent grant/revoke must record `granted_by_key_type`/`revoked_by=` (via `resolve_granting_key_type`).** Consent is the one place HSIP claims to represent authorization — losing track of whether a human or an AI agent (acting on its own credential) was the actor defeats the point of calling it "consent."
- **`HSIP_ROTATION_HOOK` must receive the key on stdin, never as a CLI argument.** Process arguments are visible via `ps`/process listings on some systems; stdin is not. Don't add a `--key <hex>`-style invocation as a "convenience" later. HSIP must never hold Vault/AWS/etc. credentials itself — the hook is the operator's own trusted tooling, not a new secret HSIP manages; don't add vendor SDKs to `hsip-api` to "simplify" this.
- **HTTP replay protection (`x-hsip-timestamp`/`x-hsip-nonce`) must stay fully opt-in.** `check_replay_protection` in `auth.rs` is a no-op when neither header is present — do not make either header mandatory, or every existing caller (all current SDKs, the CLI, the dashboard) breaks. If only one of the two is sent, that's a `400`, never silently treated as "not opted in" — a caller must not be able to think it's protected when it isn't.
- **`routes::keys::create`/`revoke` must keep checking `role == 'owner'` before mutating.** Before this existed, any active key in a tenant — including a low-privilege `ai_agent` key — could mint new `human` keys or revoke any other key in the same tenant, including the tenant's own owner key. Don't relax this to "any active key" again, and don't let `revoke` drop a tenant's last active `owner` key (it must return `409 Conflict` instead — see the last-owner guard).
- **`routes::admin::require_root_admin` must keep checking `is_root_admin`, never key name or tenant position.** The old `name == "admin" && tenant is first-ever` heuristic only ever supported one root admin; `POST /v1/admin/root-admins/grant`/`.../revoke` is the only sanctioned way to change who holds the flag, and `revoke` must keep refusing to drop the last remaining root admin on the node (`409 Conflict`) — there is no recovery path from zero root admins except editing the database directly.
- **New tenants' first key, and the bootstrap admin key, must get `role`/`is_root_admin` set explicitly on `INSERT`, not left to migration backfill.** `db.rs`'s backfill only fixes up rows that already existed *before* migrations ran on a given boot — a row created by `bootstrap_admin` or `routes::sandbox::provision` afterward, in the same process lifetime, was never touched by that pass and would be stuck with `role=NULL`/`is_root_admin=0` forever if the INSERT itself didn't set them.
- **Never discard a corrective/retry `UPDATE`'s result with `let _ = ...` and then unconditionally log or count success.** Found as a real bug in three `anchor_job.rs` sites (retry/upgrade logic all logged "succeeded" and incremented metrics regardless of whether the `UPDATE` actually affected any rows). Always match on the result and check `rows_affected() > 0` before declaring success; log a warning on failure or zero rows instead — the row's unchanged DB state means it'll naturally be retried next cycle, no extra logic needed.
- **`system_health::check()` must stay pure (no metric/logging side effects).** `check_and_update_metrics()` is the wrapper that also touches `metrics::SYSTEM_HEALTH_ISSUES` — keeping `check()` itself side-effect-free is what makes it directly unit-testable without needing to reset global metric state between tests.
- **Never embed a raw library/database error's `Display` text in an `ApiError::Internal` an ordinary (non-privileged) caller can see.** Log the real error server-side via `tracing::error!`, return a fixed generic message instead — see "Information Disclosure via Error Messages" above. This applies both to the `From<sqlx::Error>`/`From<anyhow::Error>` impls in `errors.rs` and to any hand-written `.map_err(|e| ApiError::Internal(e.to_string()))`/`format!("...: {e}")` call site, which bypasses those impls entirely and needs the same fix independently. Root-admin-gated diagnostic messages (most of `routes/admin.rs`) are the deliberate exception — that audience is trusted and the detail is the point.
- **`routes::keys::bind_client_cert` must only ever bind the fingerprint from the *calling connection's own* presented certificate**, never an arbitrary caller-supplied fingerprint string. The whole safety property of per-key mTLS binding depends on this: an owner can only bind a key to a certificate a connection has already proven possession of during its TLS handshake, so there's no way to accidentally (or maliciously) brick a key by binding it to a certificate nobody can ever present. Don't add a `"fingerprint": "..."` field to `BindClientCertRequest` as a "convenience" — that reopens exactly this hole.
- **Any file write containing the master key (initial generation in `config.rs::desktop_defaults`, or the rotation staging file in `routes/admin.rs::rotate_master_key`) must set `0o600` permissions on Unix immediately after writing.** `fs::write`/`File::create` leave a file at whatever the process umask allows — `0644`/world-readable on any default Unix umask, confirmed empirically — which for the master key specifically means "compromise = all signing keys exposed" (§5 of THREAT_MODEL.md) was one default umask away from being true with zero HSIP-level compromise at all. `rename()` on Unix preserves the *source* file's mode bits, not the destination's, so the rotation staging file needs its own explicit fix — copying `admin.key`'s permissions after the fact is not sufficient once rotation is involved.
- **Never use unbounded, caller-supplied free text as a Prometheus metric label value.** Found as a real bug in three metrics (`claim`, `tenant_id`, `decision_type` — see "Structured QA Pass" below) — each created one permanent time series per unique value for the life of the process (unbounded cardinality) and published that content to `/metrics`, which has no authentication unless an operator sets `METRICS_TOKEN`. Only use a metric label for a genuinely bounded, small, server-controlled set of values (a fixed enum, a severity level, `ok`/`invalid`) — if a field's value space isn't bounded by validation elsewhere in the same handler, it doesn't belong in a label; use an unlabeled counter instead.
- **Never write `let _ = audit_log::record(...).await;` at a call site where the operation being audited has already committed.** Use `audit_log::record_best_effort(...)` instead — it logs via `tracing::error!` and increments `metrics::AUDIT_WRITE_FAILURES` on failure rather than discarding the `Result` entirely. Found as a real bug at 9 call sites (`routes/admin.rs`, `routes/keys.rs`, `auth.rs`) — including one that already had a comment saying "still try to leave a trace" while doing exactly the opposite. A missing audit entry for an operation that otherwise succeeded is invisible to `GET /v1/audit/verify` (there's no row to detect as broken, just nothing there) — `record_best_effort` is the only thing that makes that gap observable at all. Still don't propagate the error with `?` at these sites — the underlying operation already succeeded, and failing the whole HTTP request over a downstream audit-write hiccup would incorrectly tell a caller their successful action failed.

---

## What Has Been Built

All commits on branch `claude/create-claude-md-pBtap`:

| What | Key files |
|---|---|
| Initial `CLAUDE.md` | `CLAUDE.md` |
| `hsip agent register/list/revoke` + `hsip status` | `crates/hsip-cli/src/commands/agent.rs`, `main.rs` |
| `hsip-mcp` full MCP server crate | `crates/hsip-mcp/` |
| Browser extension: `manifest.json`, port fix, AI audit panel | `browser-extension/` |
| `GET /v1/agents/discover` + `hsip agent discover` | `routes/agents.rs`, `commands/agent.rs` |
| Federated trust: `/v1/trust/*` routes + `hsip trust` CLI | `routes/trust.rs`, `commands/trust.rs`, `db.rs` |
| `hsip up` onboarding command | `commands/up.rs` |
| Dashboard progressive disclosure refactor (removed Simple/Expert) | `dashboard/src/App.jsx`, `App.css` |
| SDK parity: Python, Node.js, Go | `sdks/python/`, `sdks/node/`, `sdks/go/` |
| Remove `config.toml` from repo (renamed to `config.example.toml`) | root |
| Compiler warning cleanup | `proxy.rs`, `static_files.rs`, `main.rs`, `errors.rs`, `key_encryption.rs`, `db.rs` |
| Windows admin key path fix (`commands/util.rs`) | `commands/util.rs`, `agent.rs`, `trust.rs`, `up.rs`, `mod.rs` |
| Decision attestations: `/v1/decisions/*` + Merkle anchoring + Python SDK | `hsip-core/src/{merkle,canonical}.rs`, `hsip-api/src/{anchor,anchor_job}.rs`, `routes/decisions.rs`, `db.rs`, `sdks/python/hsip/client.py` |
| Wired `GET /v1/agents/discover` into the router (was a fully implemented, fully documented, never-registered handler — 404'd in production) | `routes/mod.rs` |
| Implemented `hsip agent discover` CLI subcommand (documented in this file since the discovery feature shipped, but the `AgentCmd::Discover` variant never existed) | `commands/agent.rs` |
| Fixed `agent.rs`'s local non-portable `load_admin_key()` (broke Windows for `agent register/list/revoke` + `status` — violated the invariant below) — now uses `commands::util::load_admin_key()` | `commands/agent.rs` |
| Audit log BLAKE3 hash chain: `audit_log::record()`, `GET /v1/audit/verify`, `UNIQUE(tenant_id, prev_hash)` chain-fork prevention — closes the THREAT_MODEL.md §4.8 gap (chain existed in `hsip-telemetry-guard`, was never wired into the HTTP audit table) | `audit_log.rs`, `routes/audit.rs`, `db.rs`, all route files writing audit entries |
| Consent grant/revoke now records `granted_by_key_type` (human/service/ai_agent) — the actor behind a "consent" was previously untracked | `routes/consent.rs`, `db.rs` |
| `HSIP_SANDBOX=true` gets a loud, unmissable startup warning + `metrics::SANDBOX_PROVISIONS` counter | `main.rs`, `metrics.rs`, `routes/sandbox.rs` |
| OpenTimestamps calendar dependency observability: `metrics::ANCHOR_CALENDAR_UNREACHABLE`, incremented on both the initial submission and retry paths | `metrics.rs`, `anchor_job.rs` |
| `HSIP_MASTER_KEY` env var now actually read by the real startup path (previously dead code) + SHA-256 fingerprint logging + "back this up" warning | `main.rs`, `key_encryption.rs` |
| Backoff + `metrics::CHAIN_WRITE_RETRIES` on the `audit_entries`/`decisions` hash-chain retry loops — was a tight no-delay retry loop, a thundering-herd risk at scale | `audit_log.rs`, `routes/decisions.rs` |
| AI-agent auto-revocation DB write now retries 3x with backoff and logs loudly on final failure instead of a silent fire-and-forget `let _ =` | `auth.rs` |
| Master key rotation: `POST /v1/admin/master-key/rotate`, `AppState.master_key` now `Arc<RwLock<Vec<u8>>>`, transactional re-encryption of `identities`/`anchor_identity`, staging-file + atomic-rename key persistence — the master key previously had no rotation path at all | `routes/admin.rs`, `state.rs`, `main.rs`, all master-key read sites |
| Read-only `GET /v1/admin/master-key/fingerprint` + `hsip keys master-fingerprint`/`rotate-master` CLI (interactive y/N confirm, `--yes` for scripts) — closed the gap where rotation and fingerprinting were only reachable by hand-rolling `curl` with bearer auth, unreachable for HSIP's original non-technical-user audience | `routes/admin.rs`, `commands/keys.rs`, `main.rs` |
| `HSIP_ROTATION_HOOK`: vendor-agnostic auto-rotation for `HSIP_MASTER_KEY`-sourced keys. Rotation no longer unconditionally refuses when the key is env-var-sourced — a configured hook script receives the new key on stdin and is trusted by exit code only; HSIP never holds Vault/AWS/etc. credentials itself | `routes/admin.rs` |
| HTTP replay protection: opt-in `x-hsip-timestamp`/`x-hsip-nonce` headers checked in the `TenantId` extractor, per-key nonce dedup in a swept `DashMap`, `metrics::REPLAY_REJECTED` — previously the HTTP API had no defense against a captured request being resent verbatim (THREAT_MODEL.md §4.3's documented gap), only rate limiting and key expiry | `auth.rs`, `state.rs`, `main.rs`, `metrics.rs` |
| RBAC beyond the bootstrap admin key: tenant-scoped `role` ('owner'\|'member') gating key create/revoke (previously any active key, including an `ai_agent` key, could mint or revoke any key in its tenant with no check) + node-level `is_root_admin` flag replacing the single hardcoded "`name==admin` in the first tenant" credential, with grant/revoke/list endpoints and CLI so more than one root admin can exist. Last-owner and last-root-admin lockout guards; `key.created`/`key.revoked`/`admin.root_admin_granted`/`admin.root_admin_revoked` audit entries (key create/revoke had none before); `metrics::ROOT_ADMIN_CHANGES` | `db.rs`, `main.rs`, `routes/keys.rs`, `routes/admin.rs`, `routes/sandbox.rs`, `routes/mod.rs`, `metrics.rs`, `commands/keys.rs` |
| Externally anchor the audit log: `anchor_job::run_audit_anchor_cycle` batches `audit_entries` (by `entry_hash`) into RFC 6962 Merkle trees, signs with the same node-level `anchor_identity` key used for decisions, submits to OpenTimestamps, stores in new `audit_anchors` table — closes THREAT_MODEL.md §4.8's "chain not anchored outside this database" gap the same way decisions were already anchored. `GET /v1/audit/:id/proof` (self-contained bundle) + `POST /v1/audit/verify-proof` (DB-free, third-party verifiable, mirrors `decisions::verify`); `metrics::AUDIT_ANCHORED` | `db.rs`, `anchor_job.rs`, `main.rs`, `routes/audit.rs`, `routes/mod.rs`, `metrics.rs`, `audit_log.rs` |
| Persist rate limiter across restarts: new `rate_limit_persistence.rs` periodically snapshots `rate_limiter`/`agent_tracker`/`sandbox_rate` (all previously in-memory-only) into a new `rate_limit_state` table every 30s, restores live windows at startup — closes the "restart silently resets every abuse-detection counter" gap without adding a DB write to the hot auth path | `rate_limit_persistence.rs`, `db.rs`, `state.rs`, `main.rs`, `lib.rs` |
| Mutual TLS: new `mtls.rs` + `[server.tls] client_ca_path` config option — server requires and verifies a client certificate signed by a configured CA before completing the TLS handshake, closing THREAT_MODEL.md's "no peer auth at transport layer" gap. Fully opt-in (`None` = byte-for-byte unchanged behavior). Also fixed a pre-existing latent panic: `reqwest`/`axum-server` enable different rustls crypto-provider features, so the first TLS operation in the process would have panicked the moment `[server.tls]` was ever actually enabled — now a default provider is installed explicitly at startup | `mtls.rs`, `config.rs`, `main.rs`, `config.example.toml` |
| SQLite → PostgreSQL migration tooling: new `hsip-migrate` binary copies an existing SQLite deployment into PostgreSQL, reusing `db::run_migrations` for the target schema. Found and fixed **two bugs that meant PostgreSQL had never actually worked at all**, fresh install or migration: every ms-epoch column was `INTEGER` (4-byte `int4` on Postgres, overflows on real timestamps — widened to `BIGINT`), `uploads.data`/`ots_proof` used the SQLite-only `BLOB` type (doesn't exist on Postgres — changed to `BYTEA`), and every one of ~150 parameterized queries used `?` placeholders (`sqlx::Any` doesn't rewrite these per backend — Postgres syntax-errors on `?` — rewritten to `$1, $2, ...`, which SQLite also accepts identically). Verified end-to-end against a real PostgreSQL 16 instance: populated a SQLite deployment via a real running server, migrated it, and confirmed the migrated server preserved identity/keys/audit-chain-validity and its anchor job ran successfully against Postgres | `db.rs`, `bin/hsip_migrate.rs`, `Cargo.toml`, `tests/postgres_compat.rs`, and every route/module file containing a `sqlx::query` call |
| `hsip-verify` (Z3 formal verification of consent non-forgery/temporal consistency/identity-binding soundness) is now a real workspace member instead of being excluded — `cargo build --workspace`/`cargo test --workspace` build and run its 9 unit + 10 integration tests, so its guarantees actually run in CI for the first time. No code changes inside the crate itself (only `cargo fmt`, since it had never been subject to this repo's formatting check before). Along the way, corrected a stale THREAT_MODEL.md claim that misattributed HSIP's post-quantum crypto (ML-KEM-768/ML-DSA-65) to `hsip-verify` — it actually lives in `hsip-core::pqc`, which was never excluded | root `Cargo.toml`, `crates/hsip-verify/` (formatting only) |
| Dashboard UI gaps: new Expert-mode Trust page (add/list/remove/verify a federated-trust peer), Discover page (one-click AI-agent key registration), and Admin page (master-key fingerprint/rotate + root-admin list/grant/revoke); `role` now shown/settable on the Keys page; a hash-chain-intact/broken indicator added to the Audit page. Corrected two stale docs claims found in the process: the Dashboard section's "progressive disclosure" description (a later UI-redesign commit reintroduced the Simple/Expert mode split without updating this file) and the roadmap's "Dashboard decisions page" item (it already existed). **Found and fixed a real bug** while building the Trust page: `trusted_peers` — the table every `routes/trust.rs` handler queries — was never created by `db::run_migrations`, so every `/v1/trust/*` call had 500'd since the feature shipped; nothing had exercised it end-to-end before. Verified against a real running server (not just unit tests): logged into the dashboard, added/listed a real trust peer over the fixed backend, confirmed the audit-verify indicator and admin panels render real data with zero failed requests | `db.rs`, `bin/hsip_migrate.rs`, `dashboard/src/App.jsx`, `dashboard/src/pages/{Trust,Discover,Admin,Keys,Audit}.jsx`, `tests/integration.rs` |
| Node/Go SDK parity for decision attestations: `hashPayload`/`recordDecision`/`saveReceipt`/`listDecisions`/`getDecisionProof`/`verifyDecision` ported to the Node SDK (plus `.d.ts` types) and `HashPayload`/`RecordDecision`/`SaveReceipt`/`ListDecisions`/`GetDecisionProof`/`VerifyDecision` to the Go SDK, matching the Python SDK's existing methods field-for-field (`HashPayload`/`SaveReceipt` are package-level functions in Go and static methods in Node/Python, not instance methods, since neither needs a server connection). Verified end-to-end against a real running server for both languages: recorded a decision, confirmed the receipt landed on disk via `save_receipt`, listed it back, fetched its proof bundle, verified it (`valid: true`), and confirmed a deliberately tampered `event_hash` correctly comes back `valid: false` | `sdks/node/src/index.js`, `sdks/node/src/index.d.ts`, `sdks/go/hsip/client.go` |
| Full-codebase security self-review (audit prep): found and fixed three confirmed vulnerabilities — stored XSS via SVG upload (`/v1/uploads` accepted and echoed back `image/svg+xml`), all five `/v1/proxy/*` endpoints reachable with zero authentication (no `TenantId` extractor), and a hardcoded fallback JWT signing key checked into the repo for the unauthenticated `hsip identity-serve` `/token` broker. Each re-verified against real running-server/process behavior after the fix, not just re-read code. See THREAT_MODEL.md §4.19 | `routes/uploads.rs`, `routes/proxy.rs`, `hsip-cli/src/identity.rs`, `CLAUDE.md`, `THREAT_MODEL.md`, `CODEMAP.md` |
| OpenTimestamps calendar submission verified against real calendars for the first time — previously blocked in every sandboxed dev environment (403 at the egress proxy, policy denial). Repo owner ran a real `hsip-api` server from an unrestricted Windows 11 network: confirmed raw connectivity to all three `DEFAULT_CALENDARS`, then recorded a real decision and confirmed `anchor_job.rs` submitted it and got back a genuine calendar receipt (`ots_status: "pending"`), independently verified by decoding `ots_proof` and finding the calendar's own URL embedded in the response bytes. Still open: Bitcoin-confirmation "upgrade" polling, unimplemented. See THREAT_MODEL.md §4.20 | `crates/hsip-api/src/anchor.rs`, `CLAUDE.md`, `THREAT_MODEL.md` |
| OpenTimestamps "upgrade" polling: `anchor_job::run_upgrade_cycle` (new 15-minute background timer, separate from the 10s submission loop) checks every `ots_status = 'pending'` anchor batch against the calendar that originally accepted it — read back out of the stored `ots_proof` blob (`anchor::extract_pending_calendar_uri`), no new DB column needed — and flips it to `'confirmed'` once that calendar reports a `BitcoinBlockHeaderAttestation` (`anchor::contains_bitcoin_attestation`). The tag byte constants are confirmed correct against the real `alice.btc.calendar.opentimestamps.org` response captured during §4.20's live verification (see `anchor.rs`'s `REAL_PENDING_PROOF` fixture test). The upgrade path itself (an actual Bitcoin confirmation coming back) hasn't been live-tested against a real calendar yet — that needs a real submission to sit through its confirmation window, which real-network verification hasn't covered — so it's covered by a full mocked-calendar integration test instead (`test_decision_anchor_upgrades_to_bitcoin_confirmed`), proving the whole submit → pending → confirmed pipeline end-to-end. New `metrics::ANCHOR_UPGRADED_TO_CONFIRMED` counter | `anchor.rs`, `anchor_job.rs`, `main.rs`, `metrics.rs`, `tests/integration.rs` |
| QA edge-case pass on the upgrade-polling feature above ("what happens at maximum / at infinity") surfaced two real, unbounded-growth gaps, both fixed same-session: the upgrade-check query had no `LIMIT` (a large backlog could make one 15-minute cycle's sequential calendar checks take longer than 15 minutes) — fixed with `anchor_job::MAX_UPGRADE_CHECKS_PER_CYCLE` (25, oldest-first); and a permanently-stuck-pending batch would be re-checked every 15 minutes forever, for the server's entire operational life — fixed with `anchor_job::MAX_PENDING_UPGRADE_AGE_MS` (7 days) plus a new `metrics::ANCHOR_UPGRADE_STALE` counter so it's observable rather than silent. Both verified by request-count assertions against a mock calendar (`test_upgrade_cycle_caps_checks_per_run`, `test_stale_pending_anchor_is_not_auto_polled`), not just by checking the resulting `ots_status` — proving the calendar genuinely wasn't contacted, not just that nothing happened to change | `anchor_job.rs`, `metrics.rs`, `tests/integration.rs` |
| Follow-up QA pass ("can this recover automatically?") found a real silent-failure bug — three `anchor_job.rs` sites logged/counted OTS retry-or-upgrade success without checking the corrective `UPDATE` actually affected any rows — fixed by checking `rows_affected() > 0` before declaring success, verified by a unit test that forces a genuine zero-rows-affected `UPDATE`. Also built the actual answer to "if it needs manual intervention, how would anyone find out": new `system_health.rs` module (checks for an incomplete master key rotation, zero root-admin keys, and abandoned OTS anchors), root-admin-gated `GET /v1/admin/system-health`, a `hsip_system_health_issues` Prometheus gauge refreshed every 5 minutes independent of API polling, and prominent surfacing at the top of `hsip status`. Verified end-to-end against a real running server, not just unit tests — forced a real `zero_root_admins` state via direct DB edit and confirmed all three surfaces (API, CLI, `/metrics`) agreed. See THREAT_MODEL.md §4.22 | `anchor_job.rs`, `system_health.rs`, `routes/admin.rs`, `routes/mod.rs`, `metrics.rs`, `main.rs`, `lib.rs`, `commands/agent.rs`, `tests/integration.rs` |
| Information disclosure via error messages — a sweep prompted by "what is exposed during debugging" found raw `sqlx`/`anyhow` error text (schema details, file paths, sometimes partial query text) reaching ordinary API callers two independent ways: via `errors.rs`'s `From` impls (any `?`-propagated DB/anyhow error), and via several routes' own manual `.map_err(|e| ApiError::Internal(e.to_string()))` calls that bypass those impls entirely. Both fixed — real error logged server-side via `tracing::error!`, a fixed generic message returned to the caller — across `errors.rs`, `auth.rs` (the `TenantId` extractor, the highest-traffic path in the codebase), `routes/credentials.rs`, `routes/messages.rs`, `routes/identity.rs`, `routes/decisions.rs`. Root-admin-gated diagnostic messages in `routes/admin.rs` deliberately left untouched — that audience is trusted and the detail is the point. New `errors.rs` test module (4 tests) proves the detail never reaches the client while hand-written safe messages are unaffected | `errors.rs`, `auth.rs`, `routes/credentials.rs`, `routes/messages.rs`, `routes/identity.rs`, `routes/decisions.rs` |
| Opt-in per-key mTLS client-certificate binding (`POST /v1/keys/:id/bind-client-cert`) — closes the gap mTLS above leaves open: authenticating the *connection* doesn't tie a specific *key*'s bearer token to a specific certificate, so a stolen bearer token still works from any client cert the CA will sign. New `mtls::ClientCertAcceptor` (wraps `RustlsAcceptor`, implements `axum_server::accept::Accept`, reads the peer certificate rustls already verified during the handshake, injects its SHA-256 fingerprint into every request's extensions via `tower_http::add_extension::AddExtension`) plus a new `api_keys.bound_client_cert_fingerprint` column; `auth.rs`'s `TenantId` extractor now requires an exact fingerprint match when a key has one bound. The binding endpoint only ever binds the *caller's own* presented certificate — never an arbitrary supplied string — so an owner can't brick a key against an unreproducible fingerprint. Fully opt-in: `NULL` (default) is zero behavior change. Verified by 4 new integration tests covering the full bind → reject-without-cert → reject-wrong-cert → accept-right-cert → clear → restored lifecycle, plus `mtls.rs`'s existing real-X.509 unit tests for the handshake-level verification this builds on | `mtls.rs`, `db.rs`, `bin/hsip_migrate.rs`, `auth.rs`, `routes/keys.rs`, `routes/mod.rs`, `Cargo.toml`, `tests/integration.rs` |
| `accountable_key` proof-of-possession (`RecordDecisionRequest.accountable_key_signature`, optional) — closes "accountable_key was pure caller-asserted metadata with no check at all." New `hsip_core::canonical::accountable_proof_preimage_hash` (deliberately includes `tenant_id` to block cross-tenant signature replay) + `routes::decisions::verify_accountable_proof`, called identically by `record()`/`proof()`/`verify()`. A claimed-but-invalid signature is rejected outright (`400`), never silently recorded as unverified. Also gave the existing but never-actually-sent HTTP replay protection its first real caller: `HSIPClient(..., replay_protection=True)` in the Python SDK. Both verified against a real running server, including confirming a signature produced under a *different* `tenant_id` is rejected and that a genuine replayed request gets `200` then `401`. See THREAT_MODEL.md §4.24 | `hsip-core/src/canonical.rs`, `routes/decisions.rs`, `db.rs`, `bin/hsip_migrate.rs`, `tests/integration.rs`, `sdks/python/hsip/client.py` |
| Structured QA pass (8 fixed lenses: attack surface, attacker's unshared assumptions, unverified trust, identity spoofing, time manipulation, secrets becoming public, debug exposure, post-compromise danger) — found and fixed two real bugs. First: the master key file (`~/.hsip/master.key`, generated by `Config::desktop_defaults` on every zero-config desktop install, and rewritten by the rotation staging file in `routes/admin.rs`) was written with no explicit permission mode — `0644`/world-readable on any default Unix umask, confirmed empirically — while `admin.key` right next to it already correctly got `0o600`. Fixed both write sites to `0o600` (Unix), verified by a unit test asserting exact mode bits and a real running-server boot. Second: three Prometheus metrics (`hsip_credentials_issued_total`, `hsip_messages_signed_total`, `hsip_decisions_recorded_total`) used unbounded caller-controlled strings (`claim`, `tenant_id`, `decision_type`) as label values — unbounded-cardinality growth for the life of the process, and publishing that content to `/metrics`, which has no authentication unless `METRICS_TOKEN` is set. Fixed by dropping all three labels; verified against a real running server that neither claim text nor tenant IDs appear in `/metrics` output anymore. See THREAT_MODEL.md §4.25 | `config.rs`, `routes/admin.rs`, `metrics.rs`, `routes/credentials.rs`, `routes/messages.rs`, `routes/decisions.rs`, `tests/integration.rs` |
| Observability QA pass (7 fixed lenses: how would I know this is failing, what metric changes first/last, which metric is misleading, what should alert, what's unobservable, what would I wish I had during an outage) — found and fixed a real silent-failure gap. 9 call sites (`routes/admin.rs` ×3, `routes/keys.rs` ×3, `auth.rs` ×3) wrote `let _ = audit_log::record(...).await;` for state-changing operations that had already committed (`key.created`, `key.revoked`, `master_key.rotated`, `admin.root_admin_granted`/`revoked`, `key.cert_bound`/`unbound`, `agent.anomaly_detected`, `agent.auto_revoked`, `agent.auto_revoke_failed`) — including one site whose own comment said "still try to leave a trace" while silently discarding the `Result` anyway. A failed write there meant the underlying operation succeeded but its audit-trail entry silently never existed, invisible to `GET /v1/audit/verify` (a missing row isn't a broken hash chain — there's nothing there to check) and to every metric and log this codebase had. New `audit_log::record_best_effort()` (logs via `tracing::error!`, increments new `metrics::AUDIT_WRITE_FAILURES{action}`) replaces all 9 sites — deliberately still not propagating the error with `?`, since failing the whole request over a downstream audit-write hiccup after the real work already succeeded would be its own bug. Verified by 3 new unit tests forcing a genuine write failure (dropping the `audit_entries` table out from under a live connection) plus a real running-server check: created a key with the audit table sabotaged mid-flight, confirmed the key creation still succeeded, the error logged with full context, and `hsip_audit_write_failures_total{action="key.created"}` incremented. See THREAT_MODEL.md §4.26 | `audit_log.rs`, `metrics.rs`, `routes/admin.rs`, `routes/keys.rs`, `auth.rs` |

---

## Roadmap

### Done ✓
- `hsip agent register/list/revoke` + `hsip status` CLI
- `hsip-mcp` MCP security gateway
- Browser extension fixed + AI agent activity panel
- `GET /v1/agents/discover` + `hsip agent discover`
- Federated trust: `/v1/trust/*` + `hsip trust add/list/remove/verify`
- `hsip up` onboarding command
- Dashboard single-mode refactor (progressive disclosure)
- SDK parity: Python, Node.js, Go
- Decision attestations: `/v1/decisions/*`, RFC 6962 Merkle anchoring, OpenTimestamps submission, Python SDK (`record_decision`/`save_receipt`/`get_decision_proof`/`verify_decision`)
- `GET /v1/agents/discover` actually wired into the router, and `hsip agent discover` actually implemented (both were documented as shipped but weren't connected)
- `agent.rs` CLI commands fixed to use the platform-aware `commands::util::load_admin_key()` (Windows was broken for the most common commands)
- Audit log BLAKE3 hash chain wired into the HTTP `audit_entries` table + `GET /v1/audit/verify` — closes THREAT_MODEL.md §4.8's previously-open gap
- Consent grant/revoke records `granted_by_key_type` — which kind of key authorized it
- `HSIP_SANDBOX` startup warning + `SANDBOX_PROVISIONS` metric; `HSIP_MASTER_KEY` env var actually wired + fingerprint logging; hash-chain retry backoff + `CHAIN_WRITE_RETRIES` metric; durable AI-agent auto-revocation writes; `ANCHOR_CALENDAR_UNREACHABLE` metric
- Master key rotation: `POST /v1/admin/master-key/rotate`
- `GET /v1/admin/master-key/fingerprint` (read-only) + `hsip keys master-fingerprint` / `hsip keys rotate-master` CLI — closes the "admin has to hand-roll curl with bearer auth" gap for the audience HSIP was originally built for
- `HSIP_ROTATION_HOOK` — vendor-agnostic auto-rotation for `HSIP_MASTER_KEY`-sourced deployments, without HSIP ever holding secrets-manager credentials
- HTTP replay protection — opt-in `x-hsip-timestamp`/`x-hsip-nonce` headers, closes THREAT_MODEL.md §4.3's previously-open "HTTP API has no per-request replay defense" gap without breaking any existing caller
- RBAC beyond the bootstrap admin key — tenant-scoped owner/member roles for key management, node-level `is_root_admin` flag + grant/revoke/list endpoints replacing the single-hardcoded-credential root-admin model. Still a flat capability (not scoped grants) and a two-tier role split (not fine-grained permissions) by design — see the RBAC section above for why that's the right amount for what HSIP needs today
- Externally anchor the audit log hash chain — same RFC 6962 Merkle + OpenTimestamps shape as decision attestations, applied to `audit_entries`; closes THREAT_MODEL.md §4.8's remaining "chain isn't anchored outside this database" gap
- Persist rate limiter across restarts — `rate_limit_persistence.rs` periodically snapshots the rate-limit/AI-agent-velocity/sandbox-provisioning DashMaps to a `rate_limit_state` table and restores live windows at startup; periodic snapshot (30s), not a write-through on every request, so the hot auth path still never touches the database
- Mutual TLS — opt-in `[server.tls] client_ca_path`, closes the "no peer auth at transport layer" gap for HSIP's HTTPS server; also fixed a pre-existing latent crypto-provider panic that would have hit the very first production use of `[server.tls]`
- SQLite → PostgreSQL migration tooling: `hsip-migrate` binary, plus the two underlying bugs (`INTEGER`/`BLOB` schema types, `?` vs `$N` bind placeholders) that meant PostgreSQL had never actually worked for any HSIP deployment — see "SQLite → PostgreSQL Migration" above. Verified end-to-end against a real PostgreSQL 16 instance, not just unit tests.
- `hsip-verify` included in the build — moved from excluded (`cargo build -p hsip-verify` only) to a real workspace member, so its Z3-backed formal-verification tests run under `cargo build --workspace`/`cargo test --workspace` like everything else. See "Including hsip-verify in the Build" above.
- Dashboard UI gaps closed: new Expert-mode Trust page (`Trust.jsx`, add/list/remove/verify), Discover page (`Discover.jsx`, one-click agent registration), Admin page (`Admin.jsx`, master-key fingerprint/rotate + root-admin list/grant/revoke), `role` column + selector added to Keys.jsx, hash-chain-intact/broken indicator added to Audit.jsx. Also corrected the Dashboard section's stale "progressive disclosure" claim (a later UI-redesign commit reintroduced the Simple/Expert mode split without updating docs — see "Dashboard" above) and the roadmap's stale "Dashboard decisions page" item (already existed). Found and fixed a real bug while building the Trust page: `trusted_peers` was never created by `db::run_migrations`, so every `/v1/trust/*` call had 500'd since the feature shipped — now fixed and covered by `test_trust_add_list_verify_remove`, the first integration test this route ever had.
- Node/Go SDK parity for decision attestations — `hashPayload`/`recordDecision`/`saveReceipt`/`listDecisions`/`getDecisionProof`/`verifyDecision` (Node) and `HashPayload`/`RecordDecision`/`SaveReceipt`/`ListDecisions`/`GetDecisionProof`/`VerifyDecision` (Go) now match the Python SDK's decision-attestation methods field-for-field. Verified end-to-end against a real running server for both languages: recorded a decision, confirmed the receipt was written to disk, listed it back, fetched its proof, verified it (`valid: true`), and confirmed a tampered `event_hash` comes back `valid: false`.
- Full-codebase security self-review (audit prep) — found and fixed three confirmed vulnerabilities: stored XSS via SVG upload in `/v1/uploads`, all five `/v1/proxy/*` endpoints missing authentication entirely, and a hardcoded fallback JWT signing key in `hsip identity-serve`. See "Security Self-Review" above and THREAT_MODEL.md §4.19. Not a substitute for the third-party audit still listed below — self-review by the same person who built the features under review.
- OpenTimestamps calendar submission verified end-to-end against real calendars, from a real unrestricted network — previously blocked (403, egress policy) in every sandboxed environment this project had been developed in. Confirmed both raw connectivity to all three `DEFAULT_CALENDARS` and the full HSIP pipeline (record a decision → `anchor_job.rs` submits it → genuine calendar receipt comes back, independently verified by decoding `ots_proof`). See THREAT_MODEL.md §4.20.
- OpenTimestamps "upgrade" polling — `anchor_job::run_upgrade_cycle` on its own 15-minute timer checks every `ots_status = 'pending'` anchor batch against the calendar that accepted it (read back out of the stored proof, no new column) and flips it to `'confirmed'` once that calendar reports a Bitcoin-block-header attestation. Tag-detection logic confirmed correct against a real captured calendar response from §4.20's verification; the confirmation path itself covered by a full mocked-calendar integration test rather than live-tested (waiting out a real submission's Bitcoin confirmation window wasn't practical to verify against the real network this round). A same-session QA edge-case pass then found and fixed two unbounded-growth gaps — an uncapped per-cycle query and a check-forever-with-no-expiry stuck-pending case — both bounded and covered by dedicated request-count tests. See THREAT_MODEL.md §4.21.
- System health surface — a follow-up QA pass on "can this recover automatically?" found a real silent-failure bug (three `anchor_job.rs` sites logging OTS retry/upgrade success without checking the corrective `UPDATE` actually landed, fixed and verified) and built `system_health.rs` plus `GET /v1/admin/system-health`, a `hsip_system_health_issues` Prometheus gauge, and prominent surfacing in `hsip status` — the actual mechanism for an operator (individual or business) to discover a condition needing manual intervention, since HSIP has no push-based alerting of its own. Verified end-to-end against a real running server. See THREAT_MODEL.md §4.22.
- Information disclosure via error messages — a "what is exposed during debugging" QA pass found raw `sqlx`/`anyhow` error text reaching ordinary API callers via `errors.rs`'s `From` impls and, independently, via several routes' own manual `.map_err(|e| ApiError::Internal(e.to_string()))` sites that bypass those impls. Both classes fixed — real error logged server-side, generic message returned — across `errors.rs`, `auth.rs`, `routes/credentials.rs`, `routes/messages.rs`, `routes/identity.rs`, `routes/decisions.rs`. Root-admin-gated admin diagnostics deliberately left as-is.
- Opt-in per-key mTLS client-certificate binding (`POST /v1/keys/:id/bind-client-cert`) — closes the remaining gap in Mutual TLS above: connection-level client-cert auth doesn't tie one specific key's bearer token to one specific certificate. A new `mtls::ClientCertAcceptor` extracts and fingerprints the peer certificate rustls already verified during the handshake and makes it available per-request; an owner-role key can then bind that fingerprint (only ever its *own* connection's, never an arbitrary string) onto any key in its tenant via the new endpoint, after which `auth.rs` requires an exact match on top of the bearer token. Fully opt-in — unset is zero behavior change.
- `accountable_key` proof-of-possession for decision attestations (`RecordDecisionRequest.accountable_key_signature`, optional) — previously `accountable_key` was pure caller-asserted metadata with no check at all. A caller can now prove they hold that key's private key by signing `hsip_core::canonical::accountable_proof_preimage_hash(accountable_key, tenant_id, model_version, strategy_id, decision_type, payload_hash)` — `tenant_id` is deliberately part of the preimage specifically to prevent a real signature from one tenant being replayed by another. `record()`/`proof()`/`verify()` all check it identically via a single shared function; a claimed-but-invalid signature is rejected outright (`400`), omitting it entirely stays a valid, backward-compatible call shape. Verified against a real running server via the Python SDK, including confirming a signature produced under a *different* tenant_id is correctly rejected — the cross-tenant-replay binding actually works, not just compiles. See THREAT_MODEL.md §4.24.
- Opt-in HTTP replay protection now sent by a real caller — `HSIPClient(..., replay_protection=True)` in the Python SDK sends fresh `x-hsip-timestamp`/`x-hsip-nonce` on every request. Closes the "server-side check exists but nothing sends it" gap for Python; Node/Go still don't send it, same "port once needed" reasoning as everything else in this deferred category. Verified against a real running server: a genuine replayed request (same timestamp+nonce pair sent twice) got `200` then `401`.
- Structured QA pass against eight fixed questions (attack surface, an attacker's unshared assumptions, unverified trusted input, identity spoofing, time manipulation, secrets becoming public, debug exposure, post-compromise danger) — found and fixed two real bugs, neither previously documented. The master key file (generated by `Config::desktop_defaults` on every zero-config desktop install, and rewritten during rotation) was written with no explicit file permission mode, `0644`/world-readable on any default Unix umask — while `admin.key` right next to it already correctly got `0o600`. Fixed both write sites; verified by a unit test asserting exact mode bits and a real server boot. Separately, three Prometheus metrics used unbounded caller-controlled strings (a credential's `claim` text, tenant UUIDs, `decision_type`) as label values — unbounded cardinality growth plus publishing that content to `/metrics`, unauthenticated by default. Fixed by dropping all three labels to plain unlabeled counters; verified against a real running server that neither leaks anymore. See THREAT_MODEL.md §4.25.
- Observability QA pass against seven fixed questions (how would I know this is failing, what metric changes first/last, which metric is misleading, what should alert, what's unobservable, what would I wish I had during an outage) — found and fixed a real silent-failure gap distinct from the anchor_job.rs one already fixed in §4.22. 9 call sites across `routes/admin.rs`, `routes/keys.rs`, and `auth.rs` discarded `audit_log::record()`'s result entirely for operations that had already committed (key creation/revocation, master key rotation, root-admin grant/revoke, mTLS cert binding, AI-agent anomaly/auto-revocation) — a failed write there meant the operation succeeded but its audit-trail entry silently never existed, with no metric, no log, and no way for `GET /v1/audit/verify` to detect a row that was never written. New `audit_log::record_best_effort()` — used at all 9 sites — logs the failure loudly and increments a new `hsip_audit_write_failures_total{action}` counter instead, without changing the deliberate choice not to fail the whole request over a downstream audit-write hiccup. Verified by 3 new unit tests forcing a genuine failure and a real running-server check that dropped the `audit_entries` table mid-flight and confirmed the operation still succeeded while the failure became visible in both logs and `/metrics`. See THREAT_MODEL.md §4.26.

### Remaining

- **`hsip up` federated-trust onboarding** — after `hsip up` succeeds, print: "Share your verify key with peers: `hsip status` shows it. They run `hsip trust add <label> <key>` to trust your messages."
- **Live-verify the OpenTimestamps upgrade path against a real calendar** — §4.21 built and tested `run_upgrade_cycle` against a mocked calendar; confirming a *real* calendar actually returns a Bitcoin-confirmed proof after enough time has passed is still open, since that requires waiting out a real confirmation window (potentially hours) rather than something checkable in one sitting.
- **Node/Go adoption of `accountable_key_signature`/`accountable_proof_preimage_hash`** — Python SDK only as of this writing. Port following the exact shape of the Python method when a concrete caller needs it, same pattern as the original Node/Go decision-attestation deferral.
- **Node/Go/CLI adoption of `x-hsip-timestamp`/`x-hsip-nonce`** — the server-side replay-protection check exists and the Python SDK now sends it (opt-in, `replay_protection=True`); Node, Go, and the CLI still don't. Port once a caller actually needs it, same pattern as decision-attestation SDK parity above — don't add it speculatively before there's a caller who needs it.
- **Real scoped-permission RBAC beyond flat `role`/`is_root_admin`** — the current model is deliberately two flat capabilities (tenant owner/member, node root-admin/not), not per-action scoped grants ("can rotate the master key but not grant root-admin to others," "can create ai_agent keys but not human keys," etc.). Fits everything HSIP needs today cleanly; revisit only when an operation shows up that the flat model genuinely can't express, same reasoning as before this round of work — don't bolt on a scoped-permissions engine speculatively.
- **Actual third-party security audit** — the self-review above (THREAT_MODEL.md §4.19) found and fixed three concrete vulnerabilities and is useful interim coverage, but it's still self-review by the same person who built the features under review, not independent verification. Stays open until a genuinely independent third party does one, per THREAT_MODEL.md §7/§8.

### Before adding new API routes
1. Add the route function in the relevant `crates/hsip-api/src/routes/*.rs` file
2. Register it in `crates/hsip-api/src/routes/mod.rs` — **and verify with `grep` that it's actually there**, not just that `cargo build` succeeds. An unregistered `pub async fn` handler compiles fine and silently 404s.
3. Add an audit entry write for any state-changing operation, via `audit_log::record()` — never a raw `INSERT INTO audit_entries`
4. Add an integration test in `crates/hsip-api/tests/integration.rs` using `test_app()`
5. Any `sqlx::query(...)` you write must use `$1, $2, ...` placeholders (never `?`) — see Key Invariants above

### Before adding new CLI commands
1. Add the variant to `Commands` enum in `crates/hsip-cli/src/main.rs`
2. Create `crates/hsip-cli/src/commands/<name>.rs`
3. Add `pub mod <name>;` to `commands/mod.rs`
4. Use `use super::util::load_admin_key;` — never write a local copy
5. Wire the match arm in `fn main()`

---

## CodeMap Protocol

**On session start:** Read `CODEMAP.md` before touching any code.

**After any function or variable change:** Update its entry in `CODEMAP.md` — `purpose`, `calls`, `called_by`, `mutates` fields as needed.

**After adding anything new:** Add its entry to `CODEMAP.md` under the correct file section.

**After deleting anything:** Remove its entry from `CODEMAP.md`.

**CODEMAP.md must be committed in the same commit as the code change** — never let the two diverge.
