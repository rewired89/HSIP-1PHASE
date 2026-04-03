# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Project Is

HSIP (High Security Internet Protocol) is a self-hosted, single-binary local identity server. It provides cryptographic identity (Ed25519), consent management, message signing, verifiable credentials, AI agent governance, DNS tracker blocking, and a tamper-proof audit trail. The server runs at `http://127.0.0.1:7474` by default (desktop mode) or port 3000 (when using `config.toml`).

## Build Commands

```bash
# Dev API only (no embedded dashboard, uses desktop defaults → port 7474)
cargo run -p hsip-api

# Dev with hot-reload dashboard (API on :3000 via config.toml, dashboard on :3001)
cargo run -p hsip-api          # reads config.toml, port 3000
cd dashboard && npm install && npm run dev   # port 3001, proxies /v1 → localhost:3000

# Production binary with embedded dashboard
cd dashboard && npm install && npm run build && cd ..
cargo build --release -p hsip-api --features hsip-api/embed-dashboard

# Build a single crate
cargo build -p hsip-core

# hsip-verify is excluded from the workspace (needs Z3 SMT solver):
cargo build -p hsip-verify
```

## Test Commands

```bash
# Full suite (238 tests)
cargo test --workspace

# Single crate
cargo test -p hsip-api
cargo test -p hsip-core

# Single test (exact match)
cargo test -p hsip-api test_credential_issue_verify_revoke -- --nocapture --exact

# RFC 8439 crypto compliance vectors
cargo test -p hsip-core rfc8439 -- --nocapture

# Replay attack prevention (nonce tests)
cargo test -p hsip-core nonce -- --nocapture

# Lint + format check
cargo clippy --workspace -- -D warnings
cargo fmt --check

# Dependency audit
cargo audit
```

## Architecture

### Crate Map

The workspace has 14 crates (`crates/`). The primary ones to understand:

| Crate | Role |
|---|---|
| `hsip-api` | The REST API server — the only deployable binary. Axum + Tokio. Owns all HTTP routes, auth, rate limiting, and DB access. |
| `hsip-core` | Cryptographic primitives: Ed25519 identity, X25519 sessions, ChaCha20-Poly1305 AEAD, consent protocol, nonce tracking. No I/O. |
| `hsip-dns` | UDP DNS server (default :5300). Checks hostnames against a hardcoded tracker blocklist; forwards everything else to 1.1.1.1:53. Controlled via `AppState.dns`. |
| `hsip-common` | Shared types across crates. |
| `hsip-intercept` | Cross-platform (Windows/Android/Linux/macOS) network interception. |
| `hsip-verify` | **Excluded from workspace** — requires Z3 SMT solver built from source. Build separately. |

All other crates (`hsip-session`, `hsip-net`, `hsip-auth`, `hsip-reputation`, `hsip-cli`, `hsip-gateway`, `hsip-regenerative`, `hsip-telemetry-guard`, `hsip-integration-sdk`) are in the workspace but `hsip-api` is the integration point for production use.

### `hsip-api` Internals

```
main.rs          Startup: loads config → master key → DB → bootstrap admin → builds Axum router
config.rs        Two startup modes: Config::load("config.toml") for server, Config::desktop_defaults() for end-users
db.rs            AnyPool (sqlx) — SQLite or PostgreSQL. All migrations are inline SQL in run_migrations().
auth.rs          TenantId extractor: SHA-256(Bearer token) → DB lookup → rate limit → AI agent velocity check
state.rs         AppState: DB pool + in-memory DashMap rate limiter + agent velocity tracker + DNS handle + proxy state
key_encryption.rs  ChaCha20-Poly1305 + HKDF-SHA256 encryption for Ed25519 private keys at rest
routes/          One file per domain: identity, consent, messages, credentials, keys, agents, audit, tenant, dns, proxy, contacts
```

### Request Auth Flow

Every protected endpoint goes through the `TenantId` extractor in `auth.rs`:
1. Extract `Bearer <token>` from `Authorization` header
2. Hash it with SHA-256 — raw tokens are never stored
3. Look up `key_hash` in `api_keys` table; check `active=1` and `expires_at`
4. Check `pending_revocation` DashSet (immediate in-memory block before DB write completes)
5. Check per-key rate limit (default 300 req/min, `RATE_LIMIT_RPM` env overrides)
6. For `agent_type='ai_agent'`: check velocity (>100/min logs anomaly; >1000/min auto-revokes key)

### Database

All tables are created at startup in `db::run_migrations()` — there are no separate migration files. The schema covers: `tenants`, `api_keys`, `identities`, `consents`, `messages`, `audit_entries`, `contacts`, `credentials`.

**Critical:** The `identities.signing_key_b64` column stores the Ed25519 private key **encrypted** with ChaCha20-Poly1305. The master key (32-byte hex) comes from either:
- A key file at `~/.hsip/master.key` (Linux/macOS) or `%APPDATA%\HSIP\master.key` (Windows) in desktop mode
- The path in `config.toml` → `[security] master_key_path` in server mode
- **Legacy fallback:** `HSIP_MASTER_KEY` env var (hex string) — see `key_encryption::load_master_key()`

### Configuration Modes

| Mode | When | Port | Config source |
|---|---|---|---|
| **Desktop defaults** | No `config.toml` found, or `embed-dashboard` feature | 7474 | `Config::desktop_defaults()` — auto-creates `~/.hsip/` |
| **Server mode** | `config.toml` present (or `HSIP_CONFIG` env) | 3000 (default) | `Config::load()` |

`DATABASE_URL` env var overrides the database URL in either mode.

### Dashboard

`dashboard/` — React + Vite. **This is the production dashboard.**

- Dev server: port 3001, proxies `/v1/*` → `http://localhost:3000` (matches server mode config.toml port)
- `npm run build` outputs to `dashboard/dist/` which gets embedded into the binary with `--features hsip-api/embed-dashboard` via `rust-embed`
- Two UI modes: **Simple** (end-user tabs: Home, Messages, Traffic Monitor, Alibi, Consents, AI Watch, Trackers, Protection) and **Expert** (developer tabs: Identity, Consent, Messages, Credentials, Audit, Keys)
- `dashboard/src/api.js` — single `request()` helper used by all pages

`dashboard_src_only/` — legacy artifact with fewer pages. Do not update this; the active dashboard is `dashboard/`.

### Feature Flag: `embed-dashboard`

When building with `--features hsip-api/embed-dashboard`:
- `static_files.rs` serves `dashboard/dist/` via `rust-embed` as a fallback handler
- On Windows, enables `windows_subsystem = "windows"` (no console) and self-install logic (`maybe_self_install()`)
- Desktop auto-opens the browser after bind

Without this feature (dev mode), API serves only `/v1/*` routes; the dashboard is served separately by Vite.

### First-Run Bootstrap

On first startup, if the `tenants` table is empty, `bootstrap_admin()` creates:
1. A default tenant
2. An admin API key (`hsip_<hex>`)
3. Prints the key to stdout and writes it to `admin_key_path` (default `~/.hsip/admin.key`)

The raw API key is shown **once** — only the SHA-256 hash is stored in the DB.

### DNS Tracker Blocker

`hsip-dns` runs as a UDP server on :5300. The tracker blocklist is a hardcoded `static TRACKER_DOMAINS` array in `lib.rs`. The `DnsHandle` is stored in `AppState.dns` (wrapped in `Arc<Mutex<Option<DnsHandle>>>`). Routes in `routes/dns.rs` start/stop it and query its event log.

### Proxy Traffic Monitor

`AppState.proxy` (`ProxyShared`) holds a ring buffer of the last 500 HTTP/HTTPS proxy events. Routes in `routes/proxy.rs` start/stop the proxy and stream its log.

### Integration Tests

All integration tests are in `crates/hsip-api/tests/integration.rs`. Each test calls `test_app()` which spins up a fresh in-memory SQLite database (unique per-test via a UUID in the URL) and returns `(app, admin_key)`. Tests use `tower::ServiceExt::oneshot()` — no real TCP socket needed.

### `.cargo/config.toml`

Contains Windows-specific Android NDK linker paths. These are no-ops on Linux/macOS unless you're cross-compiling for Android targets.

## Key Invariants (Do Not Break)

- **Ed25519 private keys must always be encrypted** before writing to the DB (`encrypt_signing_key()` in `key_encryption.rs`). Never store raw key bytes.
- **API keys are stored as SHA-256 hashes only.** Never write the raw key to the DB.
- **`pending_revocation` DashSet** must be updated before the async DB write when auto-revoking agent keys, so in-flight requests are blocked immediately.
- **Audit entries** must be written for all state-changing operations (identity creation, credential issuance/revocation, consent grant/revoke, key events).
- **In-memory SQLite for tests** requires `max_connections = 1` (each connection gets a separate DB instance otherwise).
- `crates/hsip-verify` stays excluded from the workspace — do not add it to `Cargo.toml` members.
