# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Project Is

HSIP (High Security Internet Protocol) is a self-hosted, single-binary local identity server. It provides cryptographic identity (Ed25519), consent management, message signing, verifiable credentials, AI agent governance, DNS tracker blocking, and a tamper-proof audit trail.

**Strategic direction:** Own the "AI agent identity" niche — local-first, zero-config, the tool that gives every AI agent a cryptographic identity and consent-gated audit trail. Think Tailscale for AI agent security. See `KIMI_ANALYSIS.md` for full analysis.

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
| `hsip-core` | — | Crypto primitives: Ed25519, X25519, ChaCha20-Poly1305, consent protocol, nonce tracking. No I/O. | Core — do not break |
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
main.rs           Startup: config → master key → DB → bootstrap admin → Axum router
config.rs         Two modes: Config::load("config.toml") or Config::desktop_defaults()
db.rs             AnyPool (sqlx). ALL migrations are inline SQL in run_migrations() — no migration files.
auth.rs           TenantId extractor: SHA-256(Bearer) → DB lookup → rate limit → AI velocity check
state.rs          AppState: DB + DashMap rate limiter + agent tracker + DNS handle + proxy ring buffer
key_encryption.rs ChaCha20-Poly1305 + HKDF-SHA256 encryption for Ed25519 private keys at rest
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
POST/GET  /v1/tenant/*              Tenant info + GDPR erase
POST/GET  /v1/dns/*                 DNS blocker start/stop/status/log
POST/GET  /v1/proxy/*               Proxy traffic monitor start/stop/status/log
POST/GET/DELETE /v1/contacts/*      Contact management
POST      /v1/trust/peer            Add a trusted peer (label + Ed25519 verify key)
GET       /v1/trust/peers           List trusted peers
DELETE    /v1/trust/peers/:id       Remove a trusted peer
POST      /v1/trust/verify          Verify a message signature from a trusted peer by label
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
5. Per-key rate limit (300 req/min default, `RATE_LIMIT_RPM` env overrides)
6. `agent_type='ai_agent'` only: velocity check (>100/min logs anomaly; >1000/min auto-revokes)

### Database Schema

All tables created at startup in `db::run_migrations()` — **no separate migration files**.

| Table | Key columns | Notes |
|---|---|---|
| `tenants` | id, name, created_at | — |
| `api_keys` | id, tenant_id, key_hash, name, agent_type, expires_at, active | Raw key never stored |
| `identities` | tenant_id, signing_key_b64, verify_key_b64 | Private key encrypted with ChaCha20-Poly1305 |
| `consents` | id, tenant_id, peer_verify_key, status, expires_ms | UNIQUE(tenant_id, peer_verify_key) |
| `messages` | id, tenant_id, content, signature, timestamp | — |
| `audit_entries` | id, tenant_id, action, details, timestamp | Append-only — never delete |
| `contacts` | id, tenant_id, nickname, verify_key | — |
| `credentials` | id, tenant_id, claim, user_token, issuer_verify_key, signature, revoked | — |
| `trusted_peers` | id, tenant_id, label, verify_key, added_at | UNIQUE(tenant_id, verify_key). Federated trust store. |

### Master Key Sources (in priority order)

1. Key file at path from `config.toml` → `[security] master_key_path` (server mode)
2. `~/.hsip/master.key` (Linux/macOS) or `%APPDATA%\HSIP\master.key` (Windows) — desktop mode
3. `HSIP_MASTER_KEY` env var (hex string) — legacy fallback

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

Implementation: `crates/hsip-api/src/routes/agents.rs` — `discover()` async handler.

Ports probed: Ollama :11434, LM Studio :1234, Jupyter :8888, Vite :5173, Create React App :3000, Next.js :3001, FastAPI/uvicorn :8000, Flask :5000, Express :4000, Deno :8080, Node-RED :1880, Gradio :7860.

CLI: `hsip agent discover` in `agent.rs`.

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

`dashboard/` — React + Vite. **This is the active production dashboard.** `dashboard_src_only/` is a legacy artifact — do not touch it.

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

---

## Key Invariants — Do Not Break

- **Ed25519 private keys must always be encrypted** before writing to DB (`encrypt_signing_key()` in `key_encryption.rs`). Never store raw key bytes.
- **API keys stored as SHA-256 hashes only.** Never write the raw key to DB.
- **`pending_revocation` DashSet** must be updated before the async DB write when auto-revoking agent keys — blocks in-flight requests immediately.
- **Audit entries must be written** for all state-changing operations (identity creation, credential issuance/revocation, consent grant/revoke, key events, trust peer add/remove/verify).
- **In-memory SQLite tests require `max_connections = 1`** — each connection is a separate DB instance.
- **`crates/hsip-verify` stays excluded from workspace** — do not add to root `Cargo.toml` members.
- **`dashboard_src_only/` is not the active dashboard** — never update it. Active dashboard is `dashboard/`.
- **No migration files** — all schema is inline in `db::run_migrations()`. Add new tables/columns there.
- **CLI key resolution must use `commands::util::load_admin_key()`** — never write a local `load_admin_key()` in a command file.
- **`config.toml` must not be committed** — it forces server mode and breaks desktop-mode testing.

---

## What Has Been Built

All commits on branch `claude/create-claude-md-pBtap`:

| What | Key files |
|---|---|
| Initial `CLAUDE.md` | `CLAUDE.md` |
| Kimi strategic analysis | `KIMI_ANALYSIS.md` |
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

### Remaining

- **`hsip up` federated-trust onboarding** — after `hsip up` succeeds, print: "Share your verify key with peers: `hsip status` shows it. They run `hsip trust add <label> <key>` to trust your messages."
- **Dashboard trust page** — Add a "Trust" tab (Advanced section) showing `GET /v1/trust/peers` with add/remove UI. Wire to the new `/v1/trust/*` routes.
- **Dashboard discover page** — show `/v1/agents/discover` results with one-click register buttons.
- **Integration tests for trust routes** — add to `crates/hsip-api/tests/integration.rs` using `test_app()`.

### Before adding new API routes
1. Add the route function in the relevant `crates/hsip-api/src/routes/*.rs` file
2. Register it in `crates/hsip-api/src/routes/mod.rs`
3. Add an audit entry write for any state-changing operation
4. Add an integration test in `crates/hsip-api/tests/integration.rs` using `test_app()`

### Before adding new CLI commands
1. Add the variant to `Commands` enum in `crates/hsip-cli/src/main.rs`
2. Create `crates/hsip-cli/src/commands/<name>.rs`
3. Add `pub mod <name>;` to `commands/mod.rs`
4. Use `use super::util::load_admin_key;` — never write a local copy
5. Wire the match arm in `fn main()`
