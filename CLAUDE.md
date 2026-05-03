# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Project Is

HSIP (High Security Internet Protocol) is a self-hosted, single-binary local identity server. It provides cryptographic identity (Ed25519), consent management, message signing, verifiable credentials, AI agent governance, DNS tracker blocking, and a tamper-proof audit trail.

**Strategic direction (from Kimi analysis in `KIMI_ANALYSIS.md`):** The pivot is to own the "AI agent identity" niche — local-first, zero-config, the tool that gives every AI agent a cryptographic identity and consent-gated audit trail. Think Tailscale for AI agent security.

**The server runs at:**
- `http://127.0.0.1:7474` — desktop mode (no `config.toml`)
- `http://127.0.0.1:3000` — server mode (with `config.toml`)

---

## Build Commands

```bash
# Dev API only (no embedded dashboard) → port 7474
cargo run -p hsip-api

# Dev with hot-reload dashboard (API on :3000 via config.toml, dashboard on :3001)
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
| `hsip-cli` | `hsip-cli` | CLI tool. Now includes `agent` subcommands and `status` command. | Active development |
| `hsip-mcp` | `hsip-mcp` | **NEW.** MCP server (JSON-RPC over stdio). Exposes HSIP tools to AI clients. | Active development |
| `hsip-common` | — | Shared types. | Stable |
| `hsip-intercept` | — | Cross-platform network interception (Windows/Android/Linux/macOS). | Stable |
| `hsip-verify` | — | **EXCLUDED from workspace.** Requires Z3 SMT solver. Build separately. | Do not add to Cargo.toml members |
| `hsip-session`, `hsip-net`, `hsip-auth`, `hsip-reputation`, `hsip-gateway`, `hsip-regenerative`, `hsip-telemetry-guard`, `hsip-integration-sdk` | — | Supporting crates in workspace but not actively integrated into production flow. | Stable — don't touch unless needed |

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
POST/GET  /v1/identity           Ed25519 keypair create/get
POST      /v1/identity/rotate    Rotate signing key (old credentials still verifiable by issuer_verify_key)
POST/GET  /v1/consent/*          Grant, revoke, list, check consent
POST/GET  /v1/messages/*         Sign and verify messages
POST/GET/DELETE /v1/credentials/* Issue, verify, revoke verifiable credentials
POST/GET/DELETE /v1/keys/*       API key management (human / service / ai_agent types)
GET       /v1/agents             List ai_agent keys with live velocity stats
GET       /v1/agent/capabilities Machine-readable HSIP capability spec for AI system prompts
GET       /v1/audit              Audit log (filterable, max 500 entries)
POST/GET  /v1/tenant/*           Tenant info + GDPR erase
POST/GET  /v1/dns/*              DNS blocker start/stop/status/log
POST/GET  /v1/proxy/*            Proxy traffic monitor start/stop/status/log
POST/GET/DELETE /v1/contacts/*   Contact management
GET       /health                {"status":"ok","version":"0.2.0"}
GET       /metrics               Prometheus metrics
GET       /openapi.json          OpenAPI 3.0 spec
GET       /docs                  Swagger UI
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

### Master Key Sources (in priority order)

1. Key file at path from `config.toml` → `[security] master_key_path` (server mode)
2. `~/.hsip/master.key` (Linux/macOS) or `%APPDATA%\HSIP\master.key` (Windows) — desktop mode
3. `HSIP_MASTER_KEY` env var (hex string) — legacy fallback

### Configuration Modes

| Mode | Trigger | Port | Data dir |
|---|---|---|---|
| Desktop defaults | No `config.toml`, or `embed-dashboard` feature | 7474 | `~/.hsip/` (auto-created) |
| Server mode | `config.toml` present or `HSIP_CONFIG` env set | 3000 | Configured in toml |

`DATABASE_URL` env var overrides the database URL in either mode.

---

## `hsip-cli` — CLI Tool

Binary: `hsip-cli`. Main file: `crates/hsip-cli/src/main.rs` (large — uses clap derive).

New subcommands added this session live in `crates/hsip-cli/src/commands/agent.rs`:

```bash
# AI agent governance
hsip agent register <name> [--expires-days N] [--api-url URL] [--key K]
hsip agent list [--api-url URL] [--key K]
hsip agent revoke <name-or-id> [--api-url URL] [--key K]
hsip status [--api-url URL] [--key K]
```

**Key resolution for agent commands (highest priority first):**
1. `--key` flag
2. `HSIP_API_KEY` env var
3. `~/.hsip/admin.key` file

**URL resolution:**
1. `--api-url` flag
2. `HSIP_API_URL` env var
3. `http://127.0.0.1:7474`

`hsip agent revoke` accepts the agent **name** (not just UUID) — it looks up the key ID by calling `/v1/agents` first, then calls `DELETE /v1/keys/:id`.

The existing commands in `main.rs` handle: keygen, init, key import/export (plain + encrypted), consent over UDP, session management, token issue/verify, discovery, reputation, daemon, audit export/verify/query. Do not delete these — they are separate functionality.

---

## `hsip-mcp` — MCP Server

**New crate** (`crates/hsip-mcp/`). Binary: `hsip-mcp`.

Speaks Model Context Protocol (JSON-RPC 2.0 over stdio). AI clients (Claude Desktop, Cursor, Continue) add it to their MCP config once. Every tool call then routes through HSIP — identity, consent, and audit are automatic.

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

### MCP protocol handled

`initialize`, `ping`, `tools/list`, `tools/call`. Notifications (no `id` field) are silently ignored per spec.

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

---

## Browser Extension

Location: `browser-extension/`. Manifest V3. Works in Chrome, Edge, Firefox.

### What it does

- Blocks 61 tracker domains via `declarativeNetRequest` (zero performance cost)
- Badge count of trackers blocked on current tab
- Shows HSIP server connection status (green dot when alive)
- **Shows last 5 AI agent audit entries when HSIP is connected** — "identity.created — just now", "agent.anomaly_detected — 3m ago"

### File layout

```
manifest.json   Manifest V3 — permissions, host_permissions, background, content_scripts
background.js   Service worker: tracks per-tab blocked counts, heartbeat every 30s,
                fetches /v1/audit on every heartbeat, caches in chrome.storage.local
content.js      Scans PerformanceResourceTiming entries for tracker domains, reports to background
popup.html      Extension popup UI (320px wide)
popup.js        Reads cached stats + activity from background, renders popup
rules.json      61 declarativeNetRequest block rules
icons/          SVG icons at 16, 48, 128px
```

### Install (dev)

1. `chrome://extensions` → Developer mode → Load unpacked → select `browser-extension/`
2. Click shield icon → enter key from `~/.hsip/admin.key`

### Port

Extension always targets `http://127.0.0.1:7474` (desktop default). If running in server mode on port 3000, the `host_permissions` in `manifest.json` cover that too.

### What was fixed in this session

- `manifest.json` was completely missing (extension could not load at all)
- Port was hardcoded to `7777` in `background.js` and `popup.js` — corrected to `7474`
- AI agent activity panel added to popup

---

## Dashboard

`dashboard/` — React + Vite. **This is the active production dashboard.** `dashboard_src_only/` is a legacy artifact — do not touch it.

- Dev server: port 3001, proxies `/v1/*` → `http://localhost:3000`
- `npm run build` → `dashboard/dist/` → embedded via `rust-embed` with `--features embed-dashboard`
- `dashboard/src/api.js` — single `request()` helper used by all pages

**UI modes:**
- **Simple** (end-user): Home, Messages, Traffic Monitor, Alibi, Consents, AI Watch, Trackers, Protection
- **Expert** (developer): Identity, Consent, Messages, Credentials, Audit, Keys

---

## Key Invariants — Do Not Break

- **Ed25519 private keys must always be encrypted** before writing to DB (`encrypt_signing_key()` in `key_encryption.rs`). Never store raw key bytes.
- **API keys stored as SHA-256 hashes only.** Never write the raw key to DB.
- **`pending_revocation` DashSet** must be updated before the async DB write when auto-revoking agent keys — blocks in-flight requests immediately.
- **Audit entries must be written** for all state-changing operations (identity creation, credential issuance/revocation, consent grant/revoke, key events).
- **In-memory SQLite tests require `max_connections = 1`** — each connection is a separate DB instance.
- **`crates/hsip-verify` stays excluded from workspace** — do not add to root `Cargo.toml` members.
- **`dashboard_src_only/` is not the active dashboard** — never update it. Active dashboard is `dashboard/`.
- **No migration files** — all schema is inline in `db::run_migrations()`. Add new tables/columns there.

---

## What Has Been Built (This Session)

Everything below is committed to `main` and working:

| Commit | What | Files |
|---|---|---|
| `3bbce07` | Added `CLAUDE.md` to repo root | `CLAUDE.md` |
| `2a5f3c5` | Added Kimi strategic analysis | `KIMI_ANALYSIS.md` |
| `0210c1a` | `hsip agent register/list/revoke` + `hsip status` CLI commands | `crates/hsip-cli/src/commands/agent.rs`, `main.rs`, `Cargo.toml` |
| `ac28b87` | `hsip-mcp` — full MCP server crate | `crates/hsip-mcp/` |
| `6911f0f` | Browser extension: added `manifest.json`, fixed port 7777→7474, added AI agent activity panel | `browser-extension/` |

---

## Roadmap — What To Build Next

From `KIMI_ANALYSIS.md`, in priority order. Items marked ✓ are done.

### Done ✓
- `hsip agent register/list/revoke` CLI (Sprint 1)
- `hsip-mcp` MCP security gateway (Sprint 1)
- Browser extension fixed + AI agent activity panel (Sprint 2)

### Next Up
**Sprint 2 (remaining):**
- **Auto-discovery of local agents** — scan running processes / localhost ports for MCP-compatible servers and suggest registering them as HSIP agents. Entry point: new route `GET /v1/agents/discover` in `crates/hsip-api/src/routes/agents.rs` + a CLI command `hsip agent discover`.

**Sprint 3:**
- **SDK polish** — Python, Node.js, Go SDKs live in `sdks/`. They exist but are minimal. Bring them to parity with the new agent governance APIs (register, revoke, log_action).
- **Dashboard single-mode refactor** — remove Simple/Expert split per Kimi recommendation. Progressive disclosure instead. Work in `dashboard/src/App.jsx`.

**Later:**
- Mobile app for personal identity
- Federated trust between HSIP nodes
- `hsip up` magic-moment onboarding command

### Before touching the dashboard
Read `dashboard/src/App.jsx` to understand the Simple/Expert mode toggle — it uses `localStorage('hsip_mode')`. All pages receive `apiKey` as a prop. The `request()` helper in `dashboard/src/api.js` is the only HTTP entry point.

### Before adding new API routes
1. Add the route function in the relevant `crates/hsip-api/src/routes/*.rs` file
2. Register it in `crates/hsip-api/src/routes/mod.rs`
3. Add an audit entry write for any state-changing operation
4. Add an integration test in `crates/hsip-api/tests/integration.rs` using `test_app()`

### Before adding new CLI commands
1. Add the variant to the `Commands` enum in `crates/hsip-cli/src/main.rs`
2. If it calls the HSIP API, put the logic in `crates/hsip-cli/src/commands/agent.rs` (or a new file under `commands/`)
3. Wire the match arm at the bottom of `fn main()`
4. The `ApiClient` struct in `agent.rs` handles key/URL resolution — reuse it

### Before adding new MCP tools
1. Add the tool definition to `tool_list()` in `crates/hsip-mcp/src/main.rs`
2. Add a match arm in `call_tool()`
3. Use `api.post()` or `api.get()` — the `ApiClient` handles auth and error formatting
