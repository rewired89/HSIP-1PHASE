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

# hsip-verify is EXCLUDED from the workspace (needs Z3 SMT solver — build separately)
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

## Crate Map (16 total in workspace)

| Crate | Binary | Role | Status |
|---|---|---|---|
| `hsip-api` | `hsip-api` | REST API server. Axum + Tokio. All HTTP routes, auth, rate limiting, DB. The only deployable binary. | Core — do not break |
| `hsip-core` | — | Crypto primitives: Ed25519, X25519, ChaCha20-Poly1305, consent protocol, nonce tracking, RFC 6962 Merkle trees (`merkle.rs`), RFC 8785 JCS canonicalization (`canonical.rs`). No I/O. | Core — do not break |
| `hsip-dns` | — | UDP DNS server (:5300). Hardcoded tracker blocklist; forwards to 1.1.1.1:53. | Working |
| `hsip-cli` | `hsip-cli` | CLI tool. Includes `agent`, `trust`, `up`, `status` subcommands. | Active development |
| `hsip-mcp` | `hsip-mcp` | MCP server (JSON-RPC over stdio). Exposes HSIP tools to AI clients. | Active development |
| `hsip-common` | — | Shared types. | Stable |
| `hsip-intercept` | — | Cross-platform network interception (Windows/Android/Linux/macOS). | Stable |
| `hsip-verify` | — | **EXCLUDED from workspace.** Requires Z3 SMT solver. Build separately. | Do not add to Cargo.toml members |
| `hsip-session`, `hsip-net`, `hsip-auth`, `hsip-reputation`, `hsip-gateway`, `hsip-regenerative`, `hsip-telemetry-guard`, `hsip-integration-sdk` | — | Supporting crates in workspace but not actively integrated. | Stable — don't touch unless needed |

---

## `hsip-api` Internals

```
main.rs           Startup: config → master key → DB → bootstrap admin → Axum router → spawns anchor loop
config.rs         Two modes: Config::load("config.toml") or Config::desktop_defaults()
db.rs             AnyPool (sqlx). ALL migrations are inline SQL in run_migrations() — no migration files.
auth.rs           TenantId extractor: SHA-256(Bearer) → DB lookup → rate limit → AI velocity check
state.rs          AppState: DB + DashMap rate limiter + agent tracker + DNS handle + proxy ring buffer
key_encryption.rs ChaCha20-Poly1305 + HKDF-SHA256 encryption for Ed25519 private keys at rest
anchor.rs         OpenTimestamps calendar HTTP client (network I/O only, no DB) — see Decision Attestations below
anchor_job.rs     Batches unanchored decisions into a Merkle tree on a timer, submits root to OpenTimestamps
audit_log.rs      BLAKE3 hash-chained writes to audit_entries — see Audit Log Hash Chain below
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
GET       /v1/agents                List ai_agent keys with live velocity stats
GET       /v1/agents/discover       Probe localhost ports for running AI agents / MCP servers
GET       /v1/agent/capabilities    Machine-readable HSIP capability spec for AI system prompts
GET       /v1/audit                 Audit log (filterable, max 500 entries)
GET       /v1/audit/verify           Recompute this tenant's BLAKE3 audit hash chain and report whether it's intact
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

### Database Schema

All tables created at startup in `db::run_migrations()` — **no separate migration files**.

| Table | Key columns | Notes |
|---|---|---|
| `tenants` | id, name, created_at | — |
| `api_keys` | id, tenant_id, key_hash, name, agent_type, role, is_root_admin, expires_at, active | Raw key never stored. `role` ('owner'\|'member') gates tenant-scoped key management (create/revoke other keys in the same tenant) — see `routes/keys.rs`. `is_root_admin` (0/1) gates node-level operations spanning every tenant (master key rotation) — see `routes/admin.rs` and RBAC below. |
| `identities` | tenant_id, signing_key_b64, verify_key_b64 | Private key encrypted with ChaCha20-Poly1305 |
| `consents` | id, tenant_id, peer_verify_key, status, expires_ms, granted_by_key_type | UNIQUE(tenant_id, peer_verify_key). `granted_by_key_type` (human/service/ai_agent) records which kind of key authorized the grant — nullable, pre-migration rows are NULL. |
| `messages` | id, tenant_id, content, signature, timestamp | — |
| `audit_entries` | id, tenant_id, action, peer_verify_key, details, timestamp, prev_hash, entry_hash | Append-only — never delete. `prev_hash`/`entry_hash` form a per-tenant BLAKE3 hash chain (nullable — pre-migration rows unchained); write only via `audit_log::record()`, never a raw INSERT. |
| `contacts` | id, tenant_id, nickname, verify_key | — |
| `credentials` | id, tenant_id, claim, user_token, issuer_verify_key, signature, revoked | — |
| `trusted_peers` | id, tenant_id, label, verify_key, added_at | UNIQUE(tenant_id, verify_key). Federated trust store. |
| `decisions` | id, tenant_id, agent_key_id, accountable_key, model_version, strategy_id, decision_type, payload_hash, prev_hash, event_hash, signature, anchor_id, merkle_index | UNIQUE(tenant_id, prev_hash) — hash-chains each tenant's decisions, prevents forks under concurrent inserts. `payload_hash` only — actual decision content never stored. |
| `decision_anchors` | id, merkle_root, leaf_count, anchor_signature, anchor_verify_key, ots_proof, ots_status | One row per RFC 6962 Merkle batch. `ots_status`: `pending` \| `calendar_unreachable`. |
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

---

## Decision Attestations (`/v1/decisions/*`)

Signs and hash-chains AI-agent decisions (starting with trading decisions) into a tamper-evident, independently verifiable record — proving *which* identity produced a decision without requiring anyone to trust HSIP's own database. Informed by the VeritasChain Protocol (VCP): RFC 6962 Merkle trees, RFC 8785 JCS canonicalization, Ed25519 signing. VCP-TRADE/VCP-GOV have no published schema as of this writing, so the accountability fields below are HSIP's own draft, tagged `hsip_gov_ext`, meant to be reconciled if/when VSO publishes one.

**Two-tier record by design:**
- Clear accountability metadata: `model_version`, `strategy_id`, `accountable_key`, `decision_type` — the part a regulator or auditor asks about first.
- Opaque `payload_hash`: SHA-256 of the caller's actual (never disclosed to HSIP) decision content. HSIP never receives or stores trade parameters, prices, sizes, etc. — only their hash. Disclosure of the preimage, if ever needed, happens entirely on the caller's side.

**Implementation:**
- `crates/hsip-core/src/canonical.rs` — `DecisionEnvelope` struct + `canonical_bytes()`/`event_hash()` (JCS canonicalization via `serde_jcs`, then SHA-256). `timestamp_int` is kept as a *string* field, not a JSON number, to avoid IEEE-754 precision loss on large timestamps.
- `crates/hsip-core/src/merkle.rs` — pure RFC 6962 Merkle tree (`MerkleTree`, `leaf_hash`/`node_hash` with `0x00`/`0x01` domain-separation prefixes), inclusion proof generation and verification (`verify_inclusion`). No I/O.
- `crates/hsip-api/src/routes/decisions.rs`:
  - `record()` — `POST /v1/decisions`. Resolves the authenticated `api_keys` row, validates fields, chains to the tenant's last decision via `prev_hash`, signs `event_hash` with the tenant's Ed25519 identity. Retries on `UNIQUE(tenant_id, prev_hash)` conflict (another request extended the chain first) up to `MAX_ATTEMPTS`. Writes `decision.recorded` audit entry.
  - `list()` — `GET /v1/decisions`.
  - `proof()` — `GET /v1/decisions/:id/proof`. Returns the full self-contained bundle. If unanchored yet, `anchored: false` with signature-only proof. If anchored, reconstructs the batch's leaf set and regenerates the inclusion proof on demand (not stored — recomputed from `decisions.anchor_id`/`merkle_index`).
  - `verify()` — `POST /v1/decisions/verify`. **Takes no `TenantId`, no `State`, makes no DB call** — a pure function of its request body. This is the function meant to be run independently of HSIP entirely (by Predicta, a regulator, an acquirer's engineering review).
- `crates/hsip-api/src/anchor.rs` — OpenTimestamps calendar HTTP client (`submit_digest_to`). Submits a batch's Merkle root to public calendars, stores the raw response as an opaque blob. **MVP scope**: does not parse the `.ots` binary format and does not yet poll for Bitcoin-confirmation ("upgrade"). Calendar list is a parameter so tests can point it at a `wiremock` server instead of the real network.
- `crates/hsip-api/src/anchor_job.rs` — `run_anchor_cycle()` (spawned on a timer in `main.rs`, ~10s poll). Anchors on a "whichever comes first" cadence: `BATCH_SIZE_TRIGGER` (50) unanchored decisions, or `INTERVAL_TRIGGER_MS` (5 min) elapsed with at least one waiting. Builds a `MerkleTree`, signs the root with the node-level `anchor_identity` key (**not** any tenant's identity — an anchor batch spans every tenant), submits to OpenTimestamps. If calendars are unreachable, local Merkle anchoring still proceeds (`ots_status = 'calendar_unreachable'`) and `retry_pending_ots_submissions()` retries on the next cycle. Writes one `decision.anchored` audit entry per tenant touched by the batch.

**Trust model — the signing-to-anchoring gap:** a signature proves authorship; it does not by itself prove the record wasn't deleted or reordered before the next anchor cycle publishes the batch's root externally (to OpenTimestamps/Bitcoin). That gap is bounded by the anchor cadence, and further mitigated client-side — see SDK `save_receipt()` below, which persists the signed receipt independently of this server the moment it's received.

**Known sandbox limitation:** OpenTimestamps calendar submission could not be live-tested during development — this repo's dev sandbox blocks outbound HTTPS to `*.calendar.opentimestamps.org` by egress policy (confirmed via the sandbox's own proxy rejection log, not assumed). Verify real connectivity before relying on this in production; `anchor.rs`'s unit tests cover the HTTP client logic against a mock server, which is not a substitute for that check.

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

**The Simple/Expert mode split has been removed.** The dashboard now uses progressive disclosure:
- `PRIMARY_TABS` (8 tabs, always visible): Home, Messages, Traffic Monitor, Alibi, Consents, AI Watch, Trackers, Protection
- `ADVANCED_TABS` (5 dev tabs, behind toggle): Identity, Consent, Messages, Credentials, Audit, Keys
- `showAdv` boolean state — toggled by "Advanced ▾" button in the nav
- `navigateTo(id)` helper auto-expands Advanced section when navigating to an advanced tab
- No `localStorage('hsip_mode')` — the old mode toggle is gone

`dashboard/src/App.css` has `.adv-toggle` and `.adv-tab` styles for the advanced section.

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

### Decision attestation methods — **Python SDK only so far**

| Method | API call |
|---|---|
| `hash_payload(payload: bytes)` | Static helper — hex SHA-256 in the format `record_decision` expects |
| `record_decision(accountable_key, model_version, strategy_id, decision_type, payload_hash, receipt_dir=None)` | `POST /v1/decisions`; if `receipt_dir` given, also calls `save_receipt` |
| `save_receipt(receipt, receipt_dir)` | Writes `<receipt_dir>/<decision_id>.json` — the client-side mitigation for the signing-to-anchoring trust gap |
| `list_decisions()` | `GET /v1/decisions` |
| `get_decision_proof(decision_id)` | `GET /v1/decisions/:id/proof` |
| `verify_decision(bundle)` | `POST /v1/decisions/verify` |

**Not yet ported to Node/Go** — deferred until Predicta's integration language is confirmed (this was added to unblock a specific proof-of-concept, not as full SDK parity). Port following the existing "same pattern as Python SDK" convention once needed.

---

## Key Invariants — Do Not Break

- **Ed25519 private keys must always be encrypted** before writing to DB (`encrypt_signing_key()` in `key_encryption.rs`). Never store raw key bytes.
- **API keys stored as SHA-256 hashes only.** Never write the raw key to DB.
- **`pending_revocation` DashSet** must be updated before the async DB write when auto-revoking agent keys — blocks in-flight requests immediately.
- **Audit entries must be written** for all state-changing operations (identity creation, credential issuance/revocation, consent grant/revoke, key events, trust peer add/remove/verify).
- **In-memory SQLite tests require `max_connections = 1`** — each connection is a separate DB instance.
- **`crates/hsip-verify` stays excluded from workspace** — do not add to root `Cargo.toml` members.
- **No migration files** — all schema is inline in `db::run_migrations()`. Add new tables/columns there.
- **CLI key resolution must use `commands::util::load_admin_key()`** — never write a local `load_admin_key()` in a command file.
- **`config.toml` must not be committed** — it forces server mode and breaks desktop-mode testing.
- **Decision payload content must never reach HSIP.** `routes::decisions::record` only ever accepts `payload_hash` (a caller-computed SHA-256 hex string) — never add a field for the actual decision content itself; that defeats the confidentiality design.
- **The anchor identity (`anchor_identity` table) must stay separate from tenant identities.** An anchor batch spans every tenant's decisions; do not sign an anchor root with any one tenant's key.
- **`POST /v1/decisions/verify` must stay DB-free and auth-free.** It's the one handler in `decisions.rs` deliberately without `TenantId`/`State` — a third party (regulator, acquirer's engineering review) needs to be able to run the equivalent check independently of this server. Don't add a database lookup to it.
- **`decisions` chain integrity relies on `UNIQUE(tenant_id, prev_hash)`.** Don't remove it or bypass it with raw inserts — it's what prevents the hash chain from forking under concurrent requests.
- **Never `INSERT INTO audit_entries` directly.** Always call `audit_log::record()` — it's what computes and links `prev_hash`/`entry_hash`. A raw insert produces an unchained (though still functionally fine) row that `GET /v1/audit/verify` will count as `unchained` rather than verify.
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

### Remaining

- **`hsip up` federated-trust onboarding** — after `hsip up` succeeds, print: "Share your verify key with peers: `hsip status` shows it. They run `hsip trust add <label> <key>` to trust your messages."
- **Dashboard trust page** — Add a "Trust" tab (Advanced section) showing `GET /v1/trust/peers` with add/remove UI. Wire to the new `/v1/trust/*` routes.
- **Dashboard discover page** — show `/v1/agents/discover` results with one-click register buttons.
- **Integration tests for trust routes** — add to `crates/hsip-api/tests/integration.rs` using `test_app()`.
- **Verify OpenTimestamps connectivity in a non-sandboxed environment** — `anchor.rs` was only tested against a mocked calendar; confirm the real submission protocol works before depending on it for compliance purposes. Reconfirmed still blocked as of this revision: outbound HTTPS to `*.calendar.opentimestamps.org` 403s at the proxy in this environment too (see THREAT_MODEL.md §7), same as every prior sandboxed environment this project has been developed in. This needs to be checked from an actually-unrestricted network before v1.0, not re-confirmed as still-blocked from another sandbox.
- **OpenTimestamps "upgrade" polling** — currently a batch's `ots_proof` is just the calendar's initial pending-commitment bytes; poll calendars later to obtain a fully Bitcoin-confirmed proof and flip `ots_status` to `confirmed`.
- **Node/Go SDK parity for decision attestation** — `record_decision`/`save_receipt`/`get_decision_proof`/`verify_decision` exist only in the Python SDK; port once a Node/Go caller needs them.
- **Dashboard decisions page** — show `GET /v1/decisions` with anchor/proof status, similar to the trust/discover pages above.
- **Dashboard audit verify indicator** — surface `GET /v1/audit/verify`'s `valid`/`unchained` fields somewhere in the Audit tab so a broken chain is visible without calling the API directly.
- **Externally anchor the audit log hash chain** — decisions get Merkle-batched and submitted to OpenTimestamps (see Decision Attestations above); the audit log's BLAKE3 chain (see Audit Log Hash Chain above) is self-verifiable but not yet anchored outside this database, so an attacker with DB write access can still delete the whole chain undetected, just not alter what remains. Same shape of fix as decisions, not yet done for audit.
- **Document and test a real SQLite → PostgreSQL migration path** — `DATABASE_URL` pointing at Postgres works for a fresh install (`db::run_migrations()` is backend-agnostic SQL), but there's no tested procedure for moving an existing SQLite deployment's data over. Treat it as a fresh install until this exists.
- **Dashboard: surface master key rotation + its audit trail** — no UI for `POST /v1/admin/master-key/rotate` yet; an admin has to call it directly.
- **SDK/CLI adoption of `x-hsip-timestamp`/`x-hsip-nonce`** — the server-side replay-protection check exists, but no HSIP-authored client sends these headers yet. Port once a caller (SDK or CLI) actually needs replay protection, same pattern as decision-attestation SDK parity above — don't add it speculatively to every SDK before there's a caller who needs it.
- **Real scoped-permission RBAC beyond flat `role`/`is_root_admin`** — the current model is deliberately two flat capabilities (tenant owner/member, node root-admin/not), not per-action scoped grants ("can rotate the master key but not grant root-admin to others," "can create ai_agent keys but not human keys," etc.). Fits everything HSIP needs today cleanly; revisit only when an operation shows up that the flat model genuinely can't express, same reasoning as before this round of work — don't bolt on a scoped-permissions engine speculatively.
- **Dashboard: surface tenant `role` and root-admin management** — no UI yet for seeing/changing a key's `role`, or for `GET/POST /v1/admin/root-admins*`; both are curl/CLI-only today.

### Before adding new API routes
1. Add the route function in the relevant `crates/hsip-api/src/routes/*.rs` file
2. Register it in `crates/hsip-api/src/routes/mod.rs` — **and verify with `grep` that it's actually there**, not just that `cargo build` succeeds. An unregistered `pub async fn` handler compiles fine and silently 404s.
3. Add an audit entry write for any state-changing operation, via `audit_log::record()` — never a raw `INSERT INTO audit_entries`
4. Add an integration test in `crates/hsip-api/tests/integration.rs` using `test_app()`

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
