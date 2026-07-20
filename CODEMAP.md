# CODEMAP.md — HSIP Codebase Function & Variable Reference

> Auto-generated. See `## CodeMap Protocol` in CLAUDE.md for maintenance rules.

---

## `crates/hsip-api/src/main.rs`

### `main`
- **type**: function
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: Binary entry point; sets up Tokio runtime and calls `run()`, writing any fatal error to disk. First installs the process-wide default rustls `CryptoProvider` (`aws_lc_rs`) before anything else — both `axum-server` (server TLS) and `reqwest` (HTTP client TLS, used for OpenTimestamps submission) enable different rustls crypto-provider features (`aws-lc-rs` vs `ring`), so more than one is compiled into this binary and rustls can't auto-select a default; without this call, the first TLS operation anywhere in the process panics. `install_default()`'s `Err` (meaning something else already won the race) is ignored.
- **inputs**: none
- **outputs**: `std::process::ExitCode`
- **calls**: `rustls::crypto::aws_lc_rs::default_provider().install_default`, `run`, `fatal`, `write_error_log`
- **called_by**: OS
- **mutates**: process-wide rustls `CryptoProvider` default (once); otherwise delegates to `run`

### `fatal`
- **type**: function
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: Prints a fatal error message to stderr and returns `ExitCode::FAILURE`.
- **inputs**: `msg: &str`
- **outputs**: `ExitCode`
- **calls**: `eprintln!`
- **called_by**: `main`
- **mutates**: stderr

### `write_error_log`
- **type**: function
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: Writes a startup error message to `hsip_error.log` in the data directory for GUI users who can't see stderr.
- **inputs**: `msg: &str`
- **outputs**: none (side-effect only)
- **calls**: `hsip_data_dir`, `fs::write`
- **called_by**: `main`
- **mutates**: filesystem (`hsip_error.log`)

### `run`
- **type**: function (async)
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: Full server startup: loads config, master key, DB, bootstraps admin, builds Axum router, binds TCP listener, serves. Restores rate-limit/AI-agent-velocity state via `rate_limit_persistence::load` before accepting traffic. Also spawns three background loops: the anchoring cycle (~10s poll, calls both `anchor_job::run_anchor_cycle` for decisions and `anchor_job::run_audit_anchor_cycle` for the audit log on every tick), a rate-limit state snapshot (`rate_limit_persistence::SNAPSHOT_INTERVAL_SECS` = 30s interval, calls `rate_limit_persistence::snapshot`), and a replay-nonce sweep (60s interval, `state.replay_nonces.retain(...)`) that removes expired `(key_id, nonce)` entries so opt-in HTTP replay protection (see `auth.rs::check_replay_protection`) can't grow the tracker unbounded. When `[server.tls]` is configured, delegates cert/key (and optional mutual-TLS client-CA) loading to `mtls::build_rustls_config` instead of calling `RustlsConfig::from_pem_file` directly — logs "Mutual TLS enabled" when `tls_config.client_ca_path` is set.
- **inputs**: none
- **outputs**: `Result<()>`
- **calls**: `Config::load`, `Config::desktop_defaults`, `init_logging`, `load_master_key`, `db::init`, `bootstrap_admin`, `build_cors_layer`, `AppState::new`, `router`, `create_shortcuts`, `anchor_job::run_anchor_cycle`, `anchor_job::run_audit_anchor_cycle`, `rate_limit_persistence::{load, snapshot}`, `db::now_ms`, `mtls::build_rustls_config`
- **called_by**: `main`
- **mutates**: filesystem (admin key), DB (migrations, initial tenant/key rows, `rate_limit_state` snapshots), `state.replay_nonces` DashMap (sweep removes expired entries), `state.rate_limiter`/`agent_tracker`/`sandbox_rate` (populated from persisted state at startup)

### `init_logging`
- **type**: function
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: Initialises `tracing_subscriber` with the configured log level and format (pretty or JSON).
- **inputs**: `config: &Config`
- **outputs**: none
- **calls**: `tracing_subscriber` builder chain
- **called_by**: `run`
- **mutates**: global tracing subscriber

### `load_master_key`
- **type**: function
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: `HSIP_MASTER_KEY` env var takes precedence over the file at `path` when set (hex-encoded 32 bytes either way) — this is what makes THREAT_MODEL.md's "point HSIP_MASTER_KEY at a secrets manager" mitigation actually work; previously nothing in the real startup path read that env var (a separate, `#[allow(dead_code)]`, never-called function in `key_encryption.rs` did, and was removed). Logs a SHA-256 fingerprint (never the key) either way, and a "back this up now" warning when loaded from a file. Returns `master_key_path: None` when sourced from the env var — `routes::admin::rotate_master_key` refuses to run when that's `None`, since there's no file this process can durably rewrite.
- **inputs**: `path: &str`
- **outputs**: `Result<(Vec<u8>, Option<String>)>`
- **calls**: `fs::read_to_string`, `hex::decode`, `master_key_fingerprint`
- **called_by**: `run`
- **mutates**: nothing

### `master_key_fingerprint`
- **type**: function
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: SHA-256 of the master key, truncated to first 8 bytes and hex-encoded — safe to log, lets an operator confirm a backup matches the key in use without ever printing the key itself.
- **inputs**: `key_bytes: &[u8]`
- **outputs**: `String`
- **calls**: `sha2::Sha256::digest`
- **called_by**: `load_master_key`
- **mutates**: nothing

### `build_cors_layer`
- **type**: function
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: Constructs a `tower_http::cors::CorsLayer` from the allowed_origins list in config (`*` = permissive).
- **inputs**: `config: &Config`
- **outputs**: `CorsLayer`
- **calls**: `CorsLayer` builder methods
- **called_by**: `run`
- **mutates**: nothing

### `metrics_handler`
- **type**: function (async)
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: Serves `GET /metrics` — renders Prometheus text exposition; requires `Bearer` token matching `METRICS_TOKEN` env var if set.
- **inputs**: `State(state): State<AppState>`, `headers: HeaderMap`
- **outputs**: `Result<String, StatusCode>`
- **calls**: `metrics::render`
- **called_by**: Axum router
- **mutates**: nothing

### `health_handler`
- **type**: function (async)
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: Serves `GET /health` returning `{"status":"ok","version":"0.2.0"}`.
- **inputs**: none
- **outputs**: `impl IntoResponse`
- **calls**: `Json`
- **called_by**: Axum router
- **mutates**: nothing

### `openapi_handler`
- **type**: function (async)
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: Serves `GET /openapi.json` — returns the embedded OpenAPI 3.0 spec as JSON.
- **inputs**: none
- **outputs**: `impl IntoResponse`
- **calls**: `Json`
- **called_by**: Axum router
- **mutates**: nothing

### `docs_handler`
- **type**: function (async)
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: Serves `GET /docs` — returns Swagger UI HTML that loads `/openapi.json`.
- **inputs**: none
- **outputs**: `impl IntoResponse`
- **calls**: `Html`
- **called_by**: Axum router
- **mutates**: nothing

### `create_shortcuts`
- **type**: function (async)
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: On Windows creates a Start Menu shortcut for the HSIP server (no-op on other platforms).
- **inputs**: none
- **outputs**: none
- **calls**: Windows COM APIs (cfg-gated)
- **called_by**: `run`
- **mutates**: filesystem (Start Menu on Windows)

### `maybe_self_install`
- **type**: function
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: Copies binary to a stable path on first run (macOS `~/Applications`, Linux `~/.local/bin`) so it survives Cargo target cleanup.
- **inputs**: none
- **outputs**: none
- **calls**: `std::env::current_exe`, `fs::copy`
- **called_by**: `run`
- **mutates**: filesystem

### `bootstrap_admin`
- **type**: function (async)
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: On first boot creates the default tenant, generates the admin API key with `role='owner'` and `is_root_admin=1` set explicitly on the `INSERT` (fresh installs aren't covered by `db.rs`'s upgrade backfill, since this row doesn't exist yet when migrations run), writes it to the admin key file, and prints it to stdout.
- **inputs**: `db: &Db`, `master_key: &[u8; 32]`, `config: &Config`
- **outputs**: `Result<()>`
- **calls**: `db` queries, `gen_key`, `fs::write`
- **called_by**: `run`
- **mutates**: DB (`tenants`, `api_keys`), filesystem (admin.key)

---

## `crates/hsip-api/src/config.rs`

### `Config`
- **type**: struct
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Top-level application configuration combining all sub-configs.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `run`, `Config::load`, `Config::desktop_defaults`
- **mutates**: nothing

### `ServerConfig`
- **type**: struct
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: TCP listener settings: host, port, optional TLS.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `Config`
- **mutates**: nothing

### `TlsConfig`
- **type**: struct
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Paths to TLS cert/key files, whether to require HTTPS, and optional `client_ca_path` (mutual TLS — see `mtls.rs`). `client_ca_path: None` (default) is server-only TLS, unchanged from before mTLS existed.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `ServerConfig`, `main.rs::run` (reads `client_ca_path` when starting the TLS listener), `mtls::build_rustls_config`
- **mutates**: nothing

### `DatabaseConfig`
- **type**: struct
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Database URL, max connection pool size, and migration flag.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `Config`
- **mutates**: nothing

### `SecurityConfig`
- **type**: struct
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Paths to master key and admin key files; per-key rate limit.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `Config`
- **mutates**: nothing

### `CorsConfig`
- **type**: struct
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: List of allowed CORS origins (`["*"]` = allow all).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `Config`
- **mutates**: nothing

### `MetricsConfig`
- **type**: struct
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Optional bearer token protecting `/metrics`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `Config`
- **mutates**: nothing

### `LoggingConfig`
- **type**: struct
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Log level string and output format enum.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `Config`
- **mutates**: nothing

### `LogFormat`
- **type**: enum
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Selects between `Pretty` (human) and `Json` (structured) tracing output.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `LoggingConfig`, `init_logging`
- **mutates**: nothing

### `normalise_sqlite_url`
- **type**: function
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Converts Windows backslashes in SQLite file paths to forward slashes; leaves `:memory:` untouched.
- **inputs**: `url: &str`
- **outputs**: `String`
- **calls**: `str::replace`
- **called_by**: `Config::load`
- **mutates**: nothing

### `default_true`
- **type**: function
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Serde default helper returning `true`.
- **inputs**: none
- **outputs**: `bool`
- **calls**: none
- **called_by**: serde derives on `TlsConfig`, `DatabaseConfig`
- **mutates**: nothing

### `default_max_connections`
- **type**: function
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Serde default returning `10` for DB pool size.
- **inputs**: none
- **outputs**: `u32`
- **calls**: none
- **called_by**: serde derive on `DatabaseConfig`
- **mutates**: nothing

### `default_rate_limit`
- **type**: function
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Serde default returning `60` requests/minute.
- **inputs**: none
- **outputs**: `u32`
- **calls**: none
- **called_by**: serde derive on `SecurityConfig`
- **mutates**: nothing

### `default_log_level`
- **type**: function
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Serde default returning `"info"` log level.
- **inputs**: none
- **outputs**: `String`
- **calls**: none
- **called_by**: serde derive on `LoggingConfig`
- **mutates**: nothing

### `Config::load`
- **type**: function
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Reads and parses a TOML config file, then applies env var overrides for DB URL, key paths, port, host, and CORS.
- **inputs**: `path: &str`
- **outputs**: `Result<Self>`
- **calls**: `fs::read_to_string`, `toml::from_str`, `normalise_sqlite_url`, `std::env::var`
- **called_by**: `run`
- **mutates**: nothing (reads only)

### `Config::validate`
- **type**: function
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Validates config fields: DB URL scheme, key file existence, TLS cert existence, port non-zero, valid log level.
- **inputs**: `&self`
- **outputs**: `Result<()>`
- **calls**: `Path::new().exists()`
- **called_by**: `run`
- **mutates**: nothing

### `hsip_data_dir`
- **type**: function
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Returns platform-aware HSIP data directory: `%APPDATA%\HSIP` on Windows, `~/.hsip` on Unix.
- **inputs**: none
- **outputs**: `PathBuf`
- **calls**: `std::env::var("APPDATA")`, `std::env::var("HOME")`
- **called_by**: `Config::desktop_defaults`, `write_error_log`
- **mutates**: nothing

### `Config::desktop_defaults`
- **type**: function
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Zero-config startup: creates data dir, generates master key on first run, creates empty admin key placeholder, returns full Config.
- **inputs**: none
- **outputs**: `Result<Self>`
- **calls**: `hsip_data_dir`, `fs::create_dir_all`, `OsRng.fill_bytes`, `hex::encode`, `fs::write`, `std::env::var`
- **called_by**: `run`
- **mutates**: filesystem (data dir, master.key, admin.key)

### `Config::default`
- **type**: function
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Provides default in-memory config for tests (port 3000, sqlite::memory:).
- **inputs**: none
- **outputs**: `Self`
- **calls**: none
- **called_by**: test harness
- **mutates**: nothing


---

## `crates/hsip-api/src/audit_log.rs`

### `record` (audit_log)
- **type**: function (async)
- **file**: `crates/hsip-api/src/audit_log.rs`
- **purpose**: The only sanctioned way to write to `audit_entries`. Reads the tenant's current chain tip (`entry_hash`), computes the new row's `entry_hash = BLAKE3(prev_hash || id || tenant_id || action || peer_verify_key || details || timestamp)`, and inserts. Retries up to `MAX_ATTEMPTS` on a `UNIQUE(tenant_id, prev_hash)` conflict — another concurrent writer extended the chain first, not a real error — mirroring `routes::decisions::record`'s pattern. Each conflict increments `metrics::CHAIN_WRITE_RETRIES{chain="audit"}` and waits via `chain_retry_backoff` before retrying, rather than spinning immediately.
- **inputs**: `db: &Db`, `tenant_id: &str`, `action: &str`, `peer_verify_key: Option<&str>`, `details: Option<&str>`, `timestamp: i64`
- **outputs**: `Result<String, sqlx::Error>` (the new entry's id)
- **calls**: `sqlx::query`, `compute_entry_hash`, `chain_retry_backoff`, `metrics::CHAIN_WRITE_RETRIES`
- **called_by**: every route handler and background task that writes an audit entry (`consent`, `credentials`, `identity`, `messages`, `trust`, `decisions`, `sandbox`, `auth::check_agent_velocity`, `anchor_job::run_anchor_cycle_with_calendars`, `anchor_job::run_audit_anchor_cycle_with_calendars`)
- **mutates**: `audit_entries` table

### `chain_retry_backoff`
- **type**: function (async)
- **file**: `crates/hsip-api/src/audit_log.rs`
- **purpose**: `pub(crate)` small randomized delay (`2ms * attempt` + 0-4ms jitter) between hash-chain write retries, shared by `audit_log::record` and `routes::decisions::record`. Without it, concurrent writers on the same tenant's chain retry in a tight loop with no delay — harmless at low volume, a self-inflicted thundering herd at scale.
- **inputs**: `attempt: u32`
- **outputs**: `()`
- **calls**: `rand::thread_rng`, `tokio::time::sleep`
- **called_by**: `record` (audit_log), `routes::decisions::record`
- **mutates**: nothing

### `compute_entry_hash`
- **type**: function
- **file**: `crates/hsip-api/src/audit_log.rs`
- **purpose**: `pub(crate)` (not private) — computes one entry's BLAKE3 hash from its chain-linked fields, with `0x00`-byte field separators to avoid concatenation-ambiguity collisions (e.g. `"ab"+"c"` vs `"a"+"bc"`). Exposed at crate visibility specifically so `routes::audit::verify_proof` can recompute it from caller-supplied fields with no DB call — the single source of truth for the formula, shared by writing, chain-verification, and proof-verification.
- **inputs**: `prev_hash`, `id`, `tenant_id`, `action`, `peer_verify_key`, `details`, `timestamp`
- **outputs**: `String` (hex-encoded BLAKE3 digest)
- **calls**: `blake3::Hasher`
- **called_by**: `record` (audit_log), `verify_chain` (audit_log), `routes::audit::verify_proof`
- **mutates**: nothing

### `ChainRow`
- **type**: struct
- **file**: `crates/hsip-api/src/audit_log.rs`
- **purpose**: One audit row as read back from the DB for chain verification — mirrors `audit_entries` columns.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `routes::audit::verify_chain`, `verify_chain` (audit_log)
- **mutates**: nothing

### `VerifyResult`
- **type**: struct
- **file**: `crates/hsip-api/src/audit_log.rs`
- **purpose**: Result of walking a tenant's chain: valid, checked (chained entries confirmed), unchained (pre-migration entries skipped), first_break_id.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `verify_chain` (audit_log)
- **mutates**: nothing

### `verify_chain` (audit_log)
- **type**: function
- **file**: `crates/hsip-api/src/audit_log.rs`
- **purpose**: Recomputes and checks a hash chain over pre-sorted (`ORDER BY timestamp ASC`) rows. Rows with NULL `entry_hash` (pre-migration) are counted in `unchained` and skipped rather than treated as breaks; any `prev_hash` mismatch or recomputed-hash mismatch stops at the first broken entry.
- **inputs**: `rows: &[ChainRow]`
- **outputs**: `VerifyResult`
- **calls**: `compute_entry_hash`
- **called_by**: `routes::audit::verify_chain`
- **mutates**: nothing

---

## `crates/hsip-api/src/auth.rs`

### `TenantId`
- **type**: struct
- **file**: `crates/hsip-api/src/auth.rs`
- **purpose**: Axum extractor that resolves a Bearer token to a verified tenant ID with rate-limit enforcement.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: every protected route handler via Axum extraction
- **mutates**: nothing (extractor pattern)

### `TenantId::from_request_parts`
- **type**: function (async)
- **file**: `crates/hsip-api/src/auth.rs`
- **purpose**: Implements `FromRequestParts`: extracts Bearer token, hashes it, looks up in DB, checks active/expiry/pending-revocation, enforces opt-in replay protection, rate limit, and AI velocity check.
- **inputs**: `parts: &mut Parts`, `state: &AppState`
- **outputs**: `Result<Self, ApiError>`
- **calls**: `hash_key`, `check_replay_protection`, `check_rate_limit`, `check_agent_velocity`, `sqlx::query`
- **called_by**: Axum extractor machinery
- **mutates**: `rate_limiter` DashMap (inserts/updates window), `agent_tracker` DashMap, `replay_nonces` DashMap

### `check_replay_protection`
- **type**: function
- **file**: `crates/hsip-api/src/auth.rs`
- **purpose**: Opt-in HTTP replay protection. No-op unless the caller sends both `x-hsip-timestamp` and `x-hsip-nonce`; if only one is present, rejects with 400. If both present, rejects with 401 when the timestamp is outside `REPLAY_TOLERANCE_SECS` (5 min) of server time, or when the `(key_id, nonce)` pair has already been seen within that window (checked via `DashMap::entry` for atomic check-and-insert).
- **inputs**: `key_id: &str`, `parts: &Parts`, `state: &AppState`
- **outputs**: `Result<(), ApiError>`
- **calls**: `now_ms`, `DashMap::entry`, `metrics::REPLAY_REJECTED`
- **called_by**: `TenantId::from_request_parts`
- **mutates**: `replay_nonces` DashMap (inserts new `(key_id, nonce)` entries with an expiry timestamp)

### `check_rate_limit`
- **type**: function
- **file**: `crates/hsip-api/src/auth.rs`
- **purpose**: Sliding-window rate limiter: allows up to `rate_limit_rpm` requests per minute per key; returns 429 if exceeded.
- **inputs**: `key_hash: &str`, `rate_limiter: &RateLimiter`, `limit: u32`
- **outputs**: `Result<(), ApiError>`
- **calls**: `DashMap::entry`, `RateWindow::new`
- **called_by**: `TenantId::from_request_parts`
- **mutates**: `RateLimiter` DashMap

### `check_agent_velocity`
- **type**: function (async)
- **file**: `crates/hsip-api/src/auth.rs`
- **purpose**: For `ai_agent` keys: logs `agent.anomaly_detected` audit entry if >100 req/min, auto-revokes key if >1000 req/min. The revocation DB write (`UPDATE api_keys SET active=0`) retries up to 3 times with backoff inside its spawned task — previously fire-and-forget with the `Result` discarded, meaning a failed write (or a crash before it landed) would silently leave `pending_revocation`'s in-memory block as the *only* thing stopping the key, which a process restart would erase with no record of why. On final failure now logs `tracing::error!`, increments `AGENT_ANOMALIES{event_type="auto_revoke_db_write_failed"}`, and writes an `agent.auto_revoke_failed` audit entry instead of claiming success.
- **inputs**: `key_id: &str`, `tenant_id: &str`, `state: &AppState`
- **outputs**: `()`
- **calls**: `AgentTracker::get`/`insert`, `sqlx::query` (key revoke, retried), `audit_log::record`, `metrics::AGENT_ANOMALIES`
- **called_by**: `TenantId::from_request_parts`
- **mutates**: `agent_tracker` DashMap, `pending_revocation` DashSet, DB (`api_keys` revocation, `audit_entries`)

### `rate_limit_rpm`
- **type**: function
- **file**: `crates/hsip-api/src/auth.rs`
- **purpose**: Reads `RATE_LIMIT_RPM` env var or returns the config default.
- **inputs**: `config_default: u32`
- **outputs**: `u32`
- **calls**: `std::env::var`
- **called_by**: `TenantId::from_request_parts`
- **mutates**: nothing

### `hash_key`
- **type**: function
- **file**: `crates/hsip-api/src/auth.rs`
- **purpose**: SHA-256 hashes a raw API key token and returns lowercase hex; ensures raw tokens are never stored.
- **inputs**: `key: &str`
- **outputs**: `String`
- **calls**: `sha2::Sha256::digest`, `hex::encode`
- **called_by**: `TenantId::from_request_parts`, `bootstrap_admin`, `routes/keys::gen_key`
- **mutates**: nothing

---

## `crates/hsip-api/src/db.rs`

### `Db`
- **type**: variable (type alias)
- **file**: `crates/hsip-api/src/db.rs`
- **purpose**: Type alias for `sqlx::AnyPool` — the shared DB connection pool used throughout the app.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `AppState`, all route handlers
- **mutates**: nothing

### `DRIVERS`
- **type**: variable (static `Once`)
- **file**: `crates/hsip-api/src/db.rs`
- **purpose**: Ensures sqlx SQLite/Postgres drivers are installed exactly once at startup.
- **inputs**: none
- **outputs**: none
- **calls**: `sqlx::any::install_default_drivers`
- **called_by**: `init`, `init_with_config`
- **mutates**: global sqlx driver registry

### `init`
- **type**: function (async)
- **file**: `crates/hsip-api/src/db.rs`
- **purpose**: Connects to DB using default `AnyPoolOptions`, runs migrations; used in tests with in-memory SQLite.
- **inputs**: `database_url: &str`
- **outputs**: `Result<Db>`
- **calls**: `DRIVERS.call_once`, `AnyPoolOptions::new().connect`, `run_migrations`
- **called_by**: test harness
- **mutates**: DB (creates schema)

### `init_with_config`
- **type**: function (async)
- **file**: `crates/hsip-api/src/db.rs`
- **purpose**: Connects to DB using `max_connections` from `DatabaseConfig`, runs migrations.
- **inputs**: `config: &DatabaseConfig`
- **outputs**: `Result<Db>`
- **calls**: `DRIVERS.call_once`, `AnyPoolOptions::new().max_connections().connect`, `run_migrations`
- **called_by**: `run`
- **mutates**: DB (creates schema)

### `run_migrations`
- **type**: function (async)
- **file**: `crates/hsip-api/src/db.rs`
- **purpose**: Inline SQL migrations: creates all tables (tenants, api_keys, identities, consents, messages, audit_entries, contacts, credentials, trusted_peers, uploads, anchor_identity, decision_anchors, decisions, audit_anchors, rate_limit_state) and adds missing columns idempotently. `trusted_peers` (`id`, `tenant_id`, `label`, `verify_key`, `added_at BIGINT`, `UNIQUE(tenant_id, verify_key)`) — federated trust store for `routes/trust.rs` — was documented here and in `CLAUDE.md`'s schema table as if it already existed, but this line was aspirational until it was actually added: the table itself was missing from this function since the federated-trust feature shipped, so every `/v1/trust/*` call 500'd with "no such table" on any real database. Found while building the dashboard's Trust page; fixed by actually adding the `CREATE TABLE`. `rate_limit_state` (`kind`, `state_key`, `count`, `anomaly_count`, `window_start_ms`, `updated_at`, `PRIMARY KEY (kind, state_key)`) is a periodic snapshot of the in-memory rate-limit/AI-agent-velocity DashMaps — see `rate_limit_persistence.rs`. `anchor_identity` is a singleton row holding the node-level Ed25519 key used to sign anchored Merkle roots (distinct from any tenant identity) — shared by both decision and audit-log anchoring, not decision-specific. `decision_anchors` holds one row per RFC 6962 Merkle batch of decisions (root, signature, OpenTimestamps proof/status); `audit_anchors` is the identical shape for batches of `audit_entries` (see External Anchoring in `CLAUDE.md`). `decisions` holds AI-agent decision attestations; `UNIQUE(tenant_id, prev_hash)` serializes each tenant's hash chain against concurrent inserts. `audit_entries` has nullable `prev_hash`/`entry_hash` columns (added via `ALTER TABLE ... ADD COLUMN`, ignored-error pattern for upgrades) plus a `UNIQUE(tenant_id, prev_hash)` index (`idx_audit_chain`) that serializes the audit BLAKE3 hash chain against concurrent writers the same way `decisions` does — see `audit_log.rs`. `audit_entries` also has nullable `anchor_id`/`merkle_index` columns (same ignored-error `ALTER TABLE` pattern, plus `idx_audit_anchor` index) mirroring `decisions.anchor_id`/`merkle_index` — which `audit_anchors` batch (if any) an entry's `entry_hash` was folded into. `consents` has a nullable `granted_by_key_type` column (same ignored-error `ALTER TABLE` pattern) recording which kind of key (human/service/ai_agent) authorized the grant. `api_keys` has nullable `role` ('owner'\|'member') and `is_root_admin INTEGER NOT NULL DEFAULT 0` columns (same ignored-error `ALTER TABLE` pattern), plus a one-time backfill on upgrade: the earliest-created key in each tenant becomes `'owner'` if unset, every other unset key becomes `'member'`, and the key named `admin` in the very first tenant ever created becomes `is_root_admin=1` — preserving the pre-RBAC bootstrap-admin behavior exactly across an upgrade. Fresh installs get both columns set directly by `bootstrap_admin`'s `INSERT` instead, since that row doesn't exist yet when this backfill runs.
- **inputs**: `db: &Db`
- **outputs**: `Result<()>`
- **calls**: `sqlx::query().execute(db)`
- **called_by**: `init`, `init_with_config`, `bin/hsip_migrate.rs::main` (creates the target schema before copying data — `pub` specifically so the migration binary can call it directly, guaranteeing the target schema can never drift from what the server itself expects)
- **mutates**: DB schema. All millisecond-epoch/wide-range columns are `BIGINT` (not `INTEGER` — PostgreSQL's `INTEGER` is a 4-byte `int4`, max ~2.1e9, which overflows on a real epoch-ms timestamp, ~1.7e12); binary columns are `BYTEA` (not `BLOB` — doesn't exist on PostgreSQL). Also includes a one-time, non-fatal `ALTER TABLE ... ALTER COLUMN ... TYPE BIGINT` widening pass for any PostgreSQL database whose tables were created by an older revision of this function (safe — those installs could `CREATE TABLE` but every real-timestamp `INSERT` already failed, so there's no data to lose). See "SQLite → PostgreSQL Migration" in `CLAUDE.md`.

### `now_ms`
- **type**: function
- **file**: `crates/hsip-api/src/db.rs`
- **purpose**: Returns current Unix timestamp in milliseconds as `i64`.
- **inputs**: none
- **outputs**: `i64`
- **calls**: `SystemTime::now().duration_since`
- **called_by**: route handlers needing timestamps
- **mutates**: nothing

---

## `crates/hsip-api/src/bin/hsip_migrate.rs`

Second `[[bin]]` target in `hsip-api`'s `Cargo.toml` (binary name `hsip-migrate`). Copies an existing SQLite HSIP deployment's data into PostgreSQL. Built while investigating "SQLite → PostgreSQL migration tooling," which surfaced that PostgreSQL had never actually worked for any HSIP deployment at all — see `db::run_migrations`'s entry above and `CLAUDE.md`'s "SQLite → PostgreSQL Migration" section for the two bugs (INTEGER/BLOB schema types, `?` vs `$N` bind placeholders) found and fixed alongside this tool.

### `Col` / `Table` / `Opts` / `Val`
- **type**: enums/structs
- **file**: `crates/hsip-api/src/bin/hsip_migrate.rs`
- **purpose**: `Col` tags each column's Rust type for generic copy (`Text`/`OptText`/`Int`/`OptInt`/`Blob`/`OptBlob`). `Table` pairs a table name with its `&[(&str, Col)]` column list. `Opts` holds parsed CLI flags (`from`, `to`, `yes`, `force`). `Val` is the owned-value enum `extract` decodes a source row's column into, so `copy_table`'s bind loop doesn't need per-column-type generic code at the call site.
- **called_by**: `TABLES`, `parse_args`, `extract`, `copy_table`

### `TABLES`
- **type**: variable (constant, `&[Table]`)
- **file**: `crates/hsip-api/src/bin/hsip_migrate.rs`
- **purpose**: Every table in `db::run_migrations`'s schema, with its exact column list and types, driving the copy loop. **Not** discovered dynamically — a table added to `db.rs` without a matching entry here silently isn't migrated (documented inline and in `CLAUDE.md`'s Key Invariants).
- **called_by**: `main`

### `parse_args`
- **type**: function
- **file**: `crates/hsip-api/src/bin/hsip_migrate.rs`
- **purpose**: Manual `--from`/`--to`/`--yes`/`--force`/`--help` argument parsing (no `clap` dependency added just for five flags).
- **inputs**: `std::env::args()`
- **outputs**: `Result<Opts>`
- **calls**: `print_usage` (on `--help`)
- **called_by**: `main`

### `redact`
- **type**: function
- **file**: `crates/hsip-api/src/bin/hsip_migrate.rs`
- **purpose**: Strips a `user:password@` connection-string password before it's ever printed to stdout (terminal scrollback, CI logs).
- **inputs**: `url: &str`
- **outputs**: `String`
- **called_by**: `main` (printing source/target URLs)

### `extract`
- **type**: function
- **file**: `crates/hsip-api/src/bin/hsip_migrate.rs`
- **purpose**: Reads one column from a source `AnyRow` into the owned `Val` matching its `Col` kind.
- **inputs**: `row: &AnyRow`, `idx: usize`, `kind: Col`
- **outputs**: `Result<Val>`
- **called_by**: `copy_table`

### `copy_table`
- **type**: function (async)
- **file**: `crates/hsip-api/src/bin/hsip_migrate.rs`
- **purpose**: `SELECT`s every row of one table from the source, then `INSERT`s each into the target inside the caller's transaction, using `$1, $2, ...` placeholders built from the table's column count.
- **inputs**: `source: &AnyPool`, `tx: &mut Transaction<'_, Any>`, `table: &Table`
- **outputs**: `Result<usize>` (rows copied)
- **calls**: `extract`
- **called_by**: `main`
- **mutates**: target DB (via `tx`, not committed until `main` calls `tx.commit()`)

### `row_count`
- **type**: function (async)
- **file**: `crates/hsip-api/src/bin/hsip_migrate.rs`
- **purpose**: `SELECT COUNT(*)` on a pool — used both for the "target already has data" safety check and the post-copy source/target verification pass.
- **inputs**: `pool: &AnyPool`, `table: &str`
- **outputs**: `Result<i64>`
- **called_by**: `main`

### `main` (hsip_migrate)
- **type**: function (async)
- **file**: `crates/hsip-api/src/bin/hsip_migrate.rs`
- **purpose**: Parses args, prompts for confirmation unless `--yes`, connects to both databases, calls `hsip_api::db::run_migrations` to create the target schema, refuses a non-empty target without `--force`, copies every `TABLES` entry inside one target-side transaction, commits, then verifies row counts match on both sides post-copy. Never writes to the source database.
- **inputs**: none (reads `std::env::args()`)
- **outputs**: `Result<()>`
- **calls**: `parse_args`, `redact`, `hsip_api::db::run_migrations`, `row_count`, `copy_table`
- **called_by**: OS (binary entry point)
- **mutates**: target DB only

---

## `crates/hsip-api/src/state.rs`

### `ProxyEvent`
- **type**: struct
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: A single captured proxy request event with timestamp, method, host, path, status, size, category, and blocked flag.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `push_event`, `proxy::log`
- **mutates**: nothing

### `ProxyShared`
- **type**: struct
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: Shared state for the proxy thread: running flag, ring buffer of events, and listener port.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `AppState`, proxy route handlers
- **mutates**: nothing

### `ProxyShared::new`
- **type**: function
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: Constructs a `ProxyShared` with stopped state and empty buffer.
- **inputs**: none
- **outputs**: `Self`
- **calls**: `Arc::new`, `RwLock::new`, `VecDeque::new`
- **called_by**: `AppState::new`
- **mutates**: nothing

### `ProxyState`
- **type**: struct
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: Runtime mutable state inside `ProxyShared`: running bool, events ring buffer, port.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `ProxyShared`
- **mutates**: nothing

### `VelocityRecord`
- **type**: struct
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: Per-agent rolling request count with timestamp for velocity anomaly detection.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `AgentTracker`, `check_agent_velocity`
- **mutates**: nothing

### `VelocityRecord::new`
- **type**: function
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: Initialises a velocity record with count=1 and current timestamp.
- **inputs**: none
- **outputs**: `Self`
- **calls**: `now_ms`
- **called_by**: `check_agent_velocity`
- **mutates**: nothing

### `VelocityRecord::from_parts`
- **type**: function
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: Reconstructs a velocity record from persisted values (request_count, anomaly_count, window_start_ms) — used when restoring state from the last `rate_limit_persistence` snapshot at startup.
- **inputs**: `request_count: u64`, `anomaly_count: u64`, `window_start_ms: i64`
- **outputs**: `Self`
- **calls**: none
- **called_by**: `rate_limit_persistence::load`
- **mutates**: nothing

### `RateWindow`
- **type**: struct
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: Sliding one-minute window tracking request count and start time for per-key rate limiting.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `RateLimiter` DashMap values, `check_rate_limit`
- **mutates**: nothing

### `RateWindow::new`
- **type**: function
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: Creates a fresh rate window starting now with count=1.
- **inputs**: none
- **outputs**: `Self`
- **calls**: `now_ms`
- **called_by**: `check_rate_limit`
- **mutates**: nothing

### `RateWindow::from_parts`
- **type**: function
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: Reconstructs a rate window from persisted values (count, window_start_ms) — used when restoring `rate_limiter`/`sandbox_rate` state from the last `rate_limit_persistence` snapshot at startup.
- **inputs**: `count: u64`, `window_start_ms: i64`
- **outputs**: `Self`
- **calls**: none
- **called_by**: `rate_limit_persistence::load`
- **mutates**: nothing

### `AgentTracker`
- **type**: variable (type alias)
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: `DashMap<String, VelocityRecord>` keyed by key_hash for AI agent velocity tracking. Periodically snapshotted to the `rate_limit_state` table and restored at startup — see `rate_limit_persistence.rs`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `AppState`, `check_agent_velocity`, `rate_limit_persistence::{snapshot, load}`
- **mutates**: nothing

### `RateLimiter`
- **type**: variable (type alias)
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: `DashMap<String, RateWindow>` keyed by key_hash for per-key rate limiting. Periodically snapshotted to the `rate_limit_state` table and restored at startup — see `rate_limit_persistence.rs`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `AppState`, `check_rate_limit`, `rate_limit_persistence::{snapshot, load}`
- **mutates**: nothing

### `PendingRevocation`
- **type**: variable (type alias)
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: `DashSet<String>` of key hashes that have been revoked in-memory before DB write completes.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `AppState`, `TenantId::from_request_parts`, `check_agent_velocity`
- **mutates**: nothing

### `DnsState`
- **type**: struct
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: Holds the join handle for the DNS server task and a log of blocked DNS queries.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `AppState`
- **mutates**: nothing

### `ReplayNonceTracker`
- **type**: variable (type alias)
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: `Arc<DashMap<String, i64>>` keyed by `"{key_id}:{nonce}"`, value is the ms timestamp after which the entry may be swept. Opt-in — only populated for requests sending both `x-hsip-timestamp` and `x-hsip-nonce`. A background sweep task in `main.rs` (60s interval) removes expired entries so this can't grow unbounded.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `AppState`, `auth.rs::check_replay_protection`, `main.rs` sweep task
- **mutates**: nothing

### `AppState`
- **type**: struct
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: Axum shared state: DB pool, master key, rate limiter, agent tracker, pending revocations, replay-protection nonce tracker, DNS state, proxy shared buffer, sandbox provision rate limiter. `master_key` is `Arc<RwLock<Vec<u8>>>` (not a plain `Arc<Vec<u8>>`) so `routes::admin::rotate_master_key` can swap it live without a restart; every handler that used to deref it directly now takes a short-lived `.read().await` guard first. `master_key_path: Option<Arc<String>>` is `Some` only when the key came from a file (vs. `HSIP_MASTER_KEY`) — that's what rotation checks before running.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: all Axum route handlers via `State<AppState>` extractor
- **mutates**: nothing (container for shared state)

### `AppState::new`
- **type**: function
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: Constructs `AppState` with `master_key_path: None` (rotation-via-API disabled). Kept for test-harness call sites that don't need a file-backed key; production startup uses `new_with_master_key_path`.
- **inputs**: `db: Db`, `master_key: Vec<u8>`
- **outputs**: `Self`
- **calls**: `Self::new_with_master_key_path`
- **called_by**: `tests/integration.rs` test helpers, `rate_limit_persistence.rs`'s own `#[cfg(test)]` tests
- **mutates**: nothing

### `AppState::new_with_master_key_path`
- **type**: function
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: Constructs `AppState`, wrapping `master_key` in `Arc<RwLock<_>>` and storing `master_key_path` (file path the key can be durably rewritten to, or `None` if sourced from `HSIP_MASTER_KEY`). Initialises all DashMaps including `sandbox_rate` and `replay_nonces`.
- **inputs**: `db: Db`, `master_key: Vec<u8>`, `master_key_path: Option<String>`
- **outputs**: `Self`
- **calls**: `DashMap::new`, `DashSet::new`, `ProxyShared::new`, `RwLock::new`
- **called_by**: `main.rs`'s `run` (production startup), `tests/integration.rs::test_app_with_admin_and_key_file`
- **mutates**: nothing

### `SandboxRate`
- **type**: type alias
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: `Arc<DashMap<String, RateWindow>>` — IP-keyed rate limiter for `POST /v1/sandbox/provision`. Limits to 5 provisions per IP per hour. Periodically snapshotted to the `rate_limit_state` table and restored at startup — see `rate_limit_persistence.rs`.
- **called_by**: `sandbox::check_provision_rate`, `rate_limit_persistence::{snapshot, load}`


---

## `crates/hsip-api/src/key_encryption.rs`

### `derive_encryption_key`
- **type**: function
- **file**: `crates/hsip-api/src/key_encryption.rs`
- **purpose**: Derives a 32-byte ChaCha20-Poly1305 key from the master key via HKDF-SHA256 with a fixed info string. No per-tenant derivation — the master key itself is the only secret input.
- **inputs**: `master_key: &[u8]`
- **outputs**: `[u8; 32]`
- **calls**: `hkdf::Hkdf::new`, `expand`
- **called_by**: `encrypt_signing_key`, `decrypt_signing_key`
- **mutates**: nothing

### `encrypt_signing_key`
- **type**: function
- **file**: `crates/hsip-api/src/key_encryption.rs`
- **purpose**: Encrypts a 32-byte Ed25519 signing key with ChaCha20-Poly1305 using a random 12-byte nonce; returns base64 of `nonce || ciphertext+tag`.
- **inputs**: `key_bytes: &[u8; 32]`, `master_key: &[u8]`
- **outputs**: `String`
- **calls**: `derive_encryption_key`, `OsRng`, `ChaCha20Poly1305::encrypt`
- **called_by**: `identity::create_or_get`, `identity::rotate`, `routes::admin::rotate_master_key`
- **mutates**: nothing (returns new string)

### `decrypt_signing_key`
- **type**: function
- **file**: `crates/hsip-api/src/key_encryption.rs`
- **purpose**: Decrypts a base64 `nonce || ciphertext+tag` blob back to the 32-byte Ed25519 signing key. Errors (not panics) on wrong master key, matching `key decryption failed — wrong HSIP_MASTER_KEY?`.
- **inputs**: `encrypted_b64: &str`, `master_key: &[u8]`
- **outputs**: `anyhow::Result<[u8; 32]>`
- **calls**: `derive_encryption_key`, `BASE64::decode`, `ChaCha20Poly1305::decrypt`
- **called_by**: `identity::load_signing_key`, `messages::sign`, `anchor_job::load_or_create_anchor_identity`, `routes::admin::rotate_master_key`
- **mutates**: nothing

Note: this file previously also had a `load_master_key()` reading `HSIP_MASTER_KEY` directly — it was `#[allow(dead_code)]` and never called from the real startup path (a second, real master-key-loading function lived in `main.rs` and never consulted the env var). Removed; `main.rs::load_master_key` is now the one implementation and does read `HSIP_MASTER_KEY`.

---

## `crates/hsip-api/src/errors.rs`

### `ApiError`
- **type**: enum
- **file**: `crates/hsip-api/src/errors.rs`
- **purpose**: Typed error enum for all API failures: Unauthorized, Forbidden, NotFound, BadRequest, Conflict, TooManyRequests, Internal. `Conflict` is used by `routes::decisions::record` when the `UNIQUE(tenant_id, prev_hash)` retry loop is exhausted under high contention.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: all route handlers
- **mutates**: nothing

### `ApiError::into_response`
- **type**: function
- **file**: `crates/hsip-api/src/errors.rs`
- **purpose**: Converts `ApiError` to an Axum HTTP response with appropriate status code and JSON error body.
- **inputs**: `self`
- **outputs**: `Response`
- **calls**: `Json`, `StatusCode`
- **called_by**: Axum error handling
- **mutates**: nothing

### `ApiResult<T>`
- **type**: variable (type alias)
- **file**: `crates/hsip-api/src/errors.rs`
- **purpose**: Convenience alias for `Result<T, ApiError>` used as all route return types.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: all route handlers
- **mutates**: nothing

---

## `crates/hsip-api/src/static_files.rs`

### `serve`
- **type**: function (async)
- **file**: `crates/hsip-api/src/static_files.rs`
- **purpose**: Serves embedded dashboard files from `rust-embed`; strips `/` prefix and falls back to `index.html` for SPA routing.
- **inputs**: `uri: Uri`
- **outputs**: `impl IntoResponse`
- **calls**: `Assets::get`, `mime_guess`
- **called_by**: Axum router (catch-all route, `embed-dashboard` feature only)
- **mutates**: nothing

### `not_found`
- **type**: function (async)
- **file**: `crates/hsip-api/src/static_files.rs`
- **purpose**: Returns 404 response for missing static assets.
- **inputs**: none
- **outputs**: `impl IntoResponse`
- **calls**: `StatusCode::NOT_FOUND`
- **called_by**: `serve`
- **mutates**: nothing

---

## `crates/hsip-api/src/metrics.rs`

### `REQUESTS_TOTAL`
- **type**: variable (static `Counter`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Prometheus counter for total HTTP requests handled.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: request middleware
- **mutates**: counter value

### `AUTH_FAILURES`
- **type**: variable (static `Counter`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Prometheus counter for authentication failures.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `TenantId::from_request_parts`
- **mutates**: counter value

### `CREDENTIALS_ISSUED`
- **type**: variable (static `Counter`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Prometheus counter for verifiable credentials issued.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `credentials::issue`
- **mutates**: counter value

### `CREDENTIALS_VERIFIED`
- **type**: variable (static `Counter`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Prometheus counter for credential verifications performed.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `credentials::verify`
- **mutates**: counter value

### `AGENT_ANOMALIES`
- **type**: variable (static `Counter`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Prometheus counter for AI agent velocity anomalies detected.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `check_agent_velocity`
- **mutates**: counter value

### `MESSAGES_SIGNED`
- **type**: variable (static `Counter`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Prometheus counter for messages signed via Ed25519.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `messages::sign`
- **mutates**: counter value

### `ACTIVE_TENANTS`
- **type**: variable (static `Gauge`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Prometheus gauge tracking number of active tenants.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `bootstrap_admin`, tenant lifecycle
- **mutates**: gauge value

### `DECISIONS_RECORDED`
- **type**: variable (static `CounterVec`, label `decision_type`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Prometheus counter for decision attestations recorded, by `decision_type`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `routes::decisions::record`
- **mutates**: counter value

### `DECISIONS_ANCHORED`
- **type**: variable (static `CounterVec`, label `ots_status`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Prometheus counter for decision batches anchored, by `ots_status` (`pending` or `calendar_unreachable`).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `anchor_job::run_anchor_cycle_with_calendars`
- **mutates**: counter value

### `AUDIT_ANCHORED`
- **type**: variable (static `CounterVec`, label `ots_status`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Twin of `DECISIONS_ANCHORED` for audit-log batches, by `ots_status` (`pending` or `calendar_unreachable`).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `anchor_job::run_audit_anchor_cycle_with_calendars`
- **mutates**: counter value

### `DECISIONS_VERIFIED`
- **type**: variable (static `CounterVec`, label `result`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Prometheus counter for `POST /v1/decisions/verify` calls, by result (`valid`/`invalid`).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `routes::decisions::verify`
- **mutates**: counter value

### `SANDBOX_PROVISIONS`
- **type**: variable (static `Counter`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Count of unauthenticated tenant provisions via `HSIP_SANDBOX=true`'s `POST /v1/sandbox/provision` — the one endpoint requiring no bearer key. Watch this if that env var is enabled anywhere unexpectedly.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `routes::sandbox::provision`
- **mutates**: counter value

### `ANCHOR_CALENDAR_UNREACHABLE`
- **type**: variable (static `Counter`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Count of anchor cycles/retries where every configured OpenTimestamps calendar failed. Makes the dominant external dependency for decision-attestation anchoring observable over time instead of only visible per-anchor via `ots_status='calendar_unreachable'`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `anchor_job::run_anchor_cycle_with_calendars`, `anchor_job::retry_pending_ots_submissions`
- **mutates**: counter value

### `CHAIN_WRITE_RETRIES`
- **type**: variable (static `CounterVec`, label `chain`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Count of `UNIQUE(tenant_id, prev_hash)` retry attempts in the per-tenant hash chains, by `chain` (`"audit"` or `"decisions"`). Near-zero at low volume; a rising rate is the "only matters at scale" signal for chain write contention.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `audit_log::record`, `routes::decisions::record`
- **mutates**: counter value

### `MASTER_KEY_ROTATIONS`
- **type**: variable (static `Counter`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Count of successful master key rotations. Should only ever move in small, rare, deliberate increments.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `routes::admin::rotate_master_key`
- **mutates**: counter value

### `REPLAY_REJECTED`
- **type**: variable (static `CounterVec`, label `reason`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Count of requests rejected by the opt-in replay-protection check, by `reason` (`malformed_headers`, `timestamp_out_of_window`, `duplicate_nonce`). Zero unless a caller opts in by sending `x-hsip-timestamp`/`x-hsip-nonce`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `auth.rs::check_replay_protection`
- **mutates**: counter value

### `ROOT_ADMIN_CHANGES`
- **type**: variable (static `CounterVec`, label `action`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Count of root-admin flag grants/revocations via `POST /v1/admin/root-admins/*`, by `action` (`granted`\|`revoked`). Should only ever move in small, rare, deliberate increments, same as `MASTER_KEY_ROTATIONS`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `routes::admin::grant_root_admin`, `routes::admin::revoke_root_admin`
- **mutates**: counter value

### `init` (metrics)
- **type**: function
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Registers all Prometheus metrics with the global registry.
- **inputs**: none
- **outputs**: none
- **calls**: `prometheus::register_*`
- **called_by**: `run`
- **mutates**: global Prometheus registry

### `render`
- **type**: function
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Encodes all registered metrics to Prometheus text exposition format.
- **inputs**: none
- **outputs**: `String`
- **calls**: `prometheus::gather`, `TextEncoder::encode`
- **called_by**: `metrics_handler`
- **mutates**: nothing

---

## `crates/hsip-api/src/mtls.rs`

Optional mutual TLS for the HTTPS server (`[server.tls] client_ca_path`).
HSIP has no dedicated node-to-node network protocol — "federated trust"
(`routes::trust`) is offline key registration, not a live channel — but
the existing `[server.tls]` HTTPS server only ever authenticated itself
to clients, never the reverse. When `client_ca_path` is set, the server
refuses to complete a TLS handshake with any client that doesn't present
a certificate signed by that CA, on top of (not instead of) bearer-token
auth. `client_ca_path: None` (the default) takes the exact same code
path (`RustlsConfig::from_pem_file`) as before this module existed —
fully backward compatible.

### `build_rustls_config`
- **type**: function (async)
- **file**: `crates/hsip-api/src/mtls.rs`
- **purpose**: Entry point called from `main.rs`. `client_ca_path: None` → delegates directly to `RustlsConfig::from_pem_file` (unchanged pre-mTLS behavior). `client_ca_path: Some` → builds a `rustls::ServerConfig` with client-cert verification on a blocking thread, then wraps it via `RustlsConfig::from_config`.
- **inputs**: `cert_path: &str`, `key_path: &str`, `client_ca_path: Option<&str>`
- **outputs**: `Result<RustlsConfig>`
- **calls**: `RustlsConfig::from_pem_file`, `tokio::task::spawn_blocking`, `build_server_config`, `RustlsConfig::from_config`
- **called_by**: `main.rs::run`
- **mutates**: nothing

### `build_server_config`
- **type**: function
- **file**: `crates/hsip-api/src/mtls.rs`
- **purpose**: Loads the server cert chain, private key, and client verifier, then builds a `rustls::ServerConfig` requiring client certificates. Sets `alpn_protocols` to `[h2, http/1.1]` — the same set axum-server's own `from_pem_file` path sets, otherwise HTTP/2 negotiation would silently not offer h2 on this hand-built config.
- **inputs**: `cert_path: &str`, `key_path: &str`, `ca_path: &str`
- **outputs**: `Result<ServerConfig>`
- **calls**: `load_certs`, `load_private_key`, `load_client_verifier`, `ServerConfig::builder`
- **called_by**: `build_rustls_config` (inside `spawn_blocking`)
- **mutates**: nothing

### `load_certs`
- **type**: function
- **file**: `crates/hsip-api/src/mtls.rs`
- **purpose**: Parses a PEM file into one or more `CertificateDer` (the server's own cert chain).
- **inputs**: `path: &str`
- **outputs**: `Result<Vec<CertificateDer<'static>>>`
- **calls**: `std::fs::read`, `CertificateDer::pem_slice_iter`
- **called_by**: `build_server_config`
- **mutates**: nothing

### `load_private_key`
- **type**: function
- **file**: `crates/hsip-api/src/mtls.rs`
- **purpose**: Scans a PEM file for the first parseable private key (a PEM file may have other sections, e.g. a cert, before the key) — mirrors axum-server's own `config_from_pem` key-scanning behavior.
- **inputs**: `path: &str`
- **outputs**: `Result<PrivateKeyDer<'static>>`
- **calls**: `std::fs::read`, `PrivateKeyDer::pem_slice_iter`
- **called_by**: `build_server_config`
- **mutates**: nothing

### `load_client_verifier`
- **type**: function
- **file**: `crates/hsip-api/src/mtls.rs`
- **purpose**: Parses one or more CA certificates from `client_ca_path` into a `RootCertStore` and builds a `WebPkiClientVerifier` that *requires* (not merely allows) every connecting client to present a certificate chaining to one of them. Kept separate from TLS-listener setup so it's unit-testable without binding a real socket. Note for operators: client certs must carry the `clientAuth` Extended Key Usage extension or `rustls-webpki` rejects them with a "certificate unknown" TLS alert even when correctly chained to a trusted CA — see `config.example.toml`.
- **inputs**: `ca_path: &str`
- **outputs**: `Result<Arc<dyn rustls::server::danger::ClientCertVerifier>>`
- **calls**: `std::fs::read`, `CertificateDer::pem_slice_iter`, `RootCertStore::add`, `WebPkiClientVerifier::builder`
- **called_by**: `build_server_config`
- **mutates**: nothing

---

## `crates/hsip-api/src/rate_limit_persistence.rs`

Periodic persistence of the in-memory rate-limit / AI-agent-velocity
DashMaps (`AppState.rate_limiter`, `.agent_tracker`, `.sandbox_rate`) so a
restart doesn't silently reset abuse-detection counters. Deliberately a
periodic snapshot, not a write-through on every request — see the
module-level doc comment for why.

### `SNAPSHOT_INTERVAL_SECS`
- **type**: variable (constant)
- **file**: `crates/hsip-api/src/rate_limit_persistence.rs`
- **purpose**: How often (30s) the snapshot loop in `main.rs` flushes in-memory state to `rate_limit_state`. Also the upper bound on how much state a crash or unclean restart can lose.
- **called_by**: `main.rs`'s spawned snapshot loop

### `load`
- **type**: function (async)
- **file**: `crates/hsip-api/src/rate_limit_persistence.rs`
- **purpose**: Restores persisted rate-limit/velocity state into the in-memory DashMaps. Called once at startup, before the server accepts traffic. Skips rows whose window has already expired (`now - window_start_ms >= RATE_WINDOW_MS`/`SANDBOX_WINDOW_MS`) — nothing meaningful to restore, they'd reset to fresh on first use anyway.
- **inputs**: `db: &Db`, `state: &AppState`
- **outputs**: `anyhow::Result<()>`
- **calls**: `sqlx::query`, `RateWindow::from_parts`, `VelocityRecord::from_parts`
- **called_by**: `main.rs`'s `run` (production startup, best-effort — logs a warning rather than failing startup on error)
- **mutates**: `state.rate_limiter`/`agent_tracker`/`sandbox_rate` DashMaps

### `snapshot`
- **type**: function (async)
- **file**: `crates/hsip-api/src/rate_limit_persistence.rs`
- **purpose**: Upserts the current contents of all three trackers into `rate_limit_state` — one row per live key/IP per `kind`. A tracker entry that's since been evicted from memory (key revoked, etc.) simply leaves its last-known row in place rather than deleting it; not worth an extra query to prune a handful of harmless stale rows.
- **inputs**: `db: &Db`, `state: &AppState`
- **outputs**: `anyhow::Result<()>`
- **calls**: `upsert`
- **called_by**: `main.rs`'s spawned snapshot loop (every `SNAPSHOT_INTERVAL_SECS`)
- **mutates**: `rate_limit_state` table

### `upsert` (rate_limit_persistence)
- **type**: function (async)
- **file**: `crates/hsip-api/src/rate_limit_persistence.rs`
- **purpose**: `INSERT ... ON CONFLICT (kind, state_key) DO UPDATE` for one tracker row. Standard upsert syntax supported identically by SQLite and PostgreSQL via `sqlx::AnyPool`.
- **inputs**: `db: &Db`, `kind: &str`, `key: &str`, `count: u64`, `anomaly_count: u64`, `window_start_ms: i64`, `now: i64`
- **outputs**: `anyhow::Result<()>`
- **calls**: `sqlx::query`
- **called_by**: `snapshot`
- **mutates**: `rate_limit_state` table

---

## `crates/hsip-api/src/routes/mod.rs`

### `router`
- **type**: function
- **file**: `crates/hsip-api/src/routes/mod.rs`
- **purpose**: Builds the complete Axum router with all `/v1/*` routes, static file serving, and shared state.
- **inputs**: `state: AppState`
- **outputs**: `Router`
- **calls**: `Router::new`, `Router::nest`, all route module `router()` functions
- **called_by**: `run`
- **mutates**: nothing

---

## `crates/hsip-api/src/routes/identity.rs`

### `create_or_get`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/identity.rs`
- **purpose**: `POST /v1/identity` — creates a new Ed25519 keypair for the tenant if one doesn't exist, encrypts the signing key, stores it, returns verify key.
- **inputs**: `State(state): State<AppState>`, `tenant: TenantId`
- **outputs**: `ApiResult<Json<IdentityResponse>>`
- **calls**: `ed25519_dalek::SigningKey::generate`, `encrypt_signing_key`, `sqlx::query`, `audit_log::record`
- **called_by**: Axum router
- **mutates**: DB (`identities`), writes audit entry

### `get` (identity)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/identity.rs`
- **purpose**: `GET /v1/identity` — returns the tenant's current Ed25519 verify key.
- **inputs**: `State(state): State<AppState>`, `tenant: TenantId`
- **outputs**: `ApiResult<Json<IdentityResponse>>`
- **calls**: `sqlx::query`
- **called_by**: Axum router
- **mutates**: nothing

### `rotate`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/identity.rs`
- **purpose**: `POST /v1/identity/rotate` — generates new Ed25519 keypair, replaces existing in DB, writes audit entry.
- **inputs**: `State(state): State<AppState>`, `tenant: TenantId`
- **outputs**: `ApiResult<Json<IdentityResponse>>`
- **calls**: `ed25519_dalek::SigningKey::generate`, `encrypt_signing_key`, `sqlx::query`, `audit_log::record`
- **called_by**: Axum router
- **mutates**: DB (`identities`), audit entry

### `load_signing_key`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/identity.rs`
- **purpose**: Helper — loads and decrypts the tenant's Ed25519 signing key from DB.
- **inputs**: `db: &Db`, `master_key: &[u8; 32]`, `tenant_id: &str`
- **outputs**: `Result<ed25519_dalek::SigningKey, ApiError>`
- **calls**: `sqlx::query`, `decrypt_signing_key`
- **called_by**: `messages::sign`, `credentials::issue`
- **mutates**: nothing

---

## `crates/hsip-api/src/routes/consent.rs`

### `GrantRequest`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/consent.rs`
- **purpose**: JSON body for `POST /v1/consent/grant`: peer_verify_key, scope, expires_ms.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `grant`
- **mutates**: nothing

### `RevokeRequest`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/consent.rs`
- **purpose**: JSON body for `POST /v1/consent/revoke`: peer_verify_key.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `revoke`
- **mutates**: nothing

### `PaginationParams`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/consent.rs`
- **purpose**: Query params for consent list: limit, offset.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list` (consent)
- **mutates**: nothing

### `ConsentRecord`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/consent.rs`
- **purpose**: Serialised consent row returned in list/get responses. Includes `granted_by_key_type: Option<String>` — "human" | "service" | "ai_agent", the `agent_type` of the key that granted this consent (`None` for rows written before this field existed). Answers "did a human authorize this, or did an agent approve its own action" — previously untracked.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list`, `get`, `grant`, `revoke` (consent)
- **mutates**: nothing

### `resolve_granting_key_type`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/consent.rs`
- **purpose**: Resolves the `agent_type` of the API key that authenticated the current request (re-derives from the `Authorization` header + `api_keys` lookup, same pattern as `routes::decisions::record`'s `agent_key_id` resolution — `TenantId` alone doesn't carry it).
- **inputs**: `db: &Db`, `headers: &HeaderMap`, `tenant_id: &str`
- **outputs**: `ApiResult<String>`
- **calls**: `hash_key`, `sqlx::query`
- **called_by**: `grant`, `revoke` (consent)
- **mutates**: nothing

### `validate_peer_key`
- **type**: function
- **file**: `crates/hsip-api/src/routes/consent.rs`
- **purpose**: Validates a base64-encoded peer verify key is 32 bytes (valid Ed25519 pubkey length).
- **inputs**: `key: &str`
- **outputs**: `Result<(), ApiError>`
- **calls**: `base64::decode`
- **called_by**: `grant`, `revoke`, `get`
- **mutates**: nothing

### `effective_status`
- **type**: function
- **file**: `crates/hsip-api/src/routes/consent.rs`
- **purpose**: Returns `"expired"` if consent has passed its `expires_ms`, otherwise the stored status string.
- **inputs**: `status: &str`, `expires_ms: Option<i64>`
- **outputs**: `&str`
- **calls**: `now_ms`
- **called_by**: `list`, `get` (consent)
- **mutates**: nothing

### `grant`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/consent.rs`
- **purpose**: `POST /v1/consent/grant` — upserts consent record with status `granted` and `granted_by_key_type`, writes audit entry (details include `granted_by=<type>`).
- **inputs**: `State(state)`, `tenant`, `headers: HeaderMap`, `Json(req)`
- **outputs**: `ApiResult<Json<ConsentRecord>>`
- **calls**: `validate_peer_key`, `resolve_granting_key_type`, `sqlx::query`, `now_ms`, `audit_log::record`
- **called_by**: Axum router
- **mutates**: DB (`consents`, `audit_entries`)

### `revoke` (consent)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/consent.rs`
- **purpose**: `POST /v1/consent/revoke` — updates consent status to `revoked`, writes audit entry with details `revoked_by=<key type>`.
- **inputs**: `State(state)`, `tenant`, `headers: HeaderMap`, `Json(req)`
- **outputs**: `ApiResult<Json<ConsentRecord>>`
- **calls**: `validate_peer_key`, `resolve_granting_key_type`, `sqlx::query`, `audit_log::record`
- **called_by**: Axum router
- **mutates**: DB (`consents`, `audit_entries`)

### `list` (consent)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/consent.rs`
- **purpose**: `GET /v1/consent` — returns paginated list of all consent records for the tenant.
- **inputs**: `State(state)`, `tenant`, `Query(params)`
- **outputs**: `ApiResult<Json<Vec<ConsentRecord>>>`
- **calls**: `sqlx::query_as`, `effective_status`
- **called_by**: Axum router
- **mutates**: nothing

### `get` (consent)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/consent.rs`
- **purpose**: `GET /v1/consent/:peer_key` — returns consent status for a specific peer.
- **inputs**: `State(state)`, `tenant`, `Path(peer_key)`
- **outputs**: `ApiResult<Json<ConsentRecord>>`
- **calls**: `validate_peer_key`, `sqlx::query`
- **called_by**: Axum router
- **mutates**: nothing

---

## `crates/hsip-api/src/routes/messages.rs`

### `SignRequest`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/messages.rs`
- **purpose**: JSON body for `POST /v1/messages/sign`: content string.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `sign`
- **mutates**: nothing

### `SignResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/messages.rs`
- **purpose**: Response from sign: message_id, content, signature (hex), verify_key (base64), timestamp.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `sign`
- **mutates**: nothing

### `VerifyRequest`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/messages.rs`
- **purpose**: JSON body for `POST /v1/messages/verify`: content, signature, verify_key.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `verify`
- **mutates**: nothing

### `VerifyResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/messages.rs`
- **purpose**: Response from verify: valid bool, message if valid.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `verify`
- **mutates**: nothing

### `MessageRecord`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/messages.rs`
- **purpose**: Serialised message row: id, content, signature, timestamp.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list` (messages)
- **mutates**: nothing

### `sign`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/messages.rs`
- **purpose**: `POST /v1/messages/sign` — signs content with tenant's Ed25519 key, stores record, increments MESSAGES_SIGNED counter, writes audit entry.
- **inputs**: `State(state)`, `tenant`, `Json(req)`
- **outputs**: `ApiResult<Json<SignResponse>>`
- **calls**: `load_signing_key`, `ed25519_dalek::SigningKey::sign`, `sqlx::query`, `audit_log::record`, `metrics::MESSAGES_SIGNED.inc`
- **called_by**: Axum router
- **mutates**: DB (`messages`, `audit_entries`)

### `verify` (messages)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/messages.rs`
- **purpose**: `POST /v1/messages/verify` — verifies an Ed25519 signature against provided content and verify key.
- **inputs**: `State(state)`, `tenant`, `Json(req)`
- **outputs**: `ApiResult<Json<VerifyResponse>>`
- **calls**: `ed25519_dalek::VerifyingKey::verify_strict`, `audit_log::record`
- **called_by**: Axum router
- **mutates**: nothing

### `list` (messages)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/messages.rs`
- **purpose**: `GET /v1/messages` — returns the tenant's signed message history.
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<Vec<MessageRecord>>>`
- **calls**: `sqlx::query_as`
- **called_by**: Axum router
- **mutates**: nothing


---

## `crates/hsip-api/src/routes/credentials.rs`

### `IssueRequest`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/credentials.rs`
- **purpose**: JSON body for `POST /v1/credentials`: claim map and optional user_token.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `issue`
- **mutates**: nothing

### `CredentialPayload`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/credentials.rs`
- **purpose**: Canonical JSON structure signed by the issuer: claim, issued_at, user_token, issuer_verify_key.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `issue`, `verify` (credentials)
- **mutates**: nothing

### `IssueResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/credentials.rs`
- **purpose**: Response from credential issuance: id, claim, signature (hex), verify_key, issued_at.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `issue`
- **mutates**: nothing

### `CredentialRecord`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/credentials.rs`
- **purpose**: Serialised credential row for list response: id, claim, signature, verify_key, revoked flag.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list` (credentials)
- **mutates**: nothing

### `canonical_json`
- **type**: function
- **file**: `crates/hsip-api/src/routes/credentials.rs`
- **purpose**: Serialises a `CredentialPayload` to deterministic JSON for signing/verification.
- **inputs**: `payload: &CredentialPayload`
- **outputs**: `Result<String>`
- **calls**: `serde_json::to_string`
- **called_by**: `issue`, `verify` (credentials)
- **mutates**: nothing

### `issue`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/credentials.rs`
- **purpose**: `POST /v1/credentials` — creates, signs, and stores a verifiable credential; increments CREDENTIALS_ISSUED metric.
- **inputs**: `State(state)`, `tenant`, `Json(req)`
- **outputs**: `ApiResult<Json<IssueResponse>>`
- **calls**: `load_signing_key`, `canonical_json`, `ed25519_dalek::SigningKey::sign`, `sqlx::query`, `audit_log::record`, `metrics::CREDENTIALS_ISSUED.inc`
- **called_by**: Axum router
- **mutates**: DB (`credentials`, `audit_entries`)

### `verify` (credentials)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/credentials.rs`
- **purpose**: `POST /v1/credentials/verify` — verifies a credential signature against its payload; checks revocation status; increments CREDENTIALS_VERIFIED metric.
- **inputs**: `State(state)`, `tenant`, `Json(req)`
- **outputs**: `ApiResult<Json<VerifyResponse>>`
- **calls**: `canonical_json`, `ed25519_dalek::VerifyingKey::verify_strict`, `sqlx::query`, `audit_log::record`, `metrics::CREDENTIALS_VERIFIED.inc`
- **called_by**: Axum router
- **mutates**: nothing (read-only verification)

### `revoke` (credentials)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/credentials.rs`
- **purpose**: `DELETE /v1/credentials/:id` — marks credential as revoked in DB, writes audit entry.
- **inputs**: `State(state)`, `tenant`, `Path(id)`
- **outputs**: `ApiResult<StatusCode>`
- **calls**: `sqlx::query`, `audit_log::record`
- **called_by**: Axum router
- **mutates**: DB (`credentials.revoked`, `audit_entries`)

### `list` (credentials)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/credentials.rs`
- **purpose**: `GET /v1/credentials` — returns all credentials for the tenant.
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<Vec<CredentialRecord>>>`
- **calls**: `sqlx::query_as`
- **called_by**: Axum router
- **mutates**: nothing

---

## `crates/hsip-api/src/routes/keys.rs`

### `CreateKeyRequest`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/keys.rs`
- **purpose**: JSON body for `POST /v1/keys`: name, agent_type (human/service/ai_agent), optional expires_in_days, optional role ('owner'\|'member', defaults to 'member').
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `create`
- **mutates**: nothing

### `resolve_caller_role`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/keys.rs`
- **purpose**: Resolves the `role` ('owner'\|'member'\|NULL) of the API key that authenticated this request, scoped to its own tenant. Re-parses the Authorization header and re-queries `api_keys` itself — same pattern as `routes::admin::require_root_admin` and `routes::consent::resolve_granting_key_type` — since `TenantId` only carries the resolved tenant_id, not the calling key's own attributes.
- **inputs**: `db: &Db`, `headers: &HeaderMap`, `tenant_id: &str`
- **outputs**: `ApiResult<Option<String>>`
- **calls**: `hash_key`, `sqlx::query`
- **called_by**: `create` (keys), `revoke` (keys)
- **mutates**: nothing

### `CreateKeyResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/keys.rs`
- **purpose**: Response from key creation: id, key (raw, returned only once), name, agent_type, role, created_at, expires_at.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `create`
- **mutates**: nothing

### `KeyRecord`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/keys.rs`
- **purpose**: Serialised key row for list response: id, name, agent_type, role, created_at, expires_at, active flag.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list` (keys)
- **mutates**: nothing

### `create` (keys)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/keys.rs`
- **purpose**: `POST /v1/keys` — requires the caller's own key to be `role='owner'` in this tenant (previously any active key, including a low-privilege `ai_agent` key, could mint new keys with no check). Generates new API key, stores its SHA-256 hash and the requested `role` (default 'member'), returns raw key once. Writes a `key.created` audit entry.
- **inputs**: `State(state)`, `tenant`, `headers`, `Json(req)`
- **outputs**: `ApiResult<Json<CreateKeyResponse>>`
- **calls**: `resolve_caller_role`, `gen_key`, `hash_key`, `sqlx::query`, `audit_log::record`
- **called_by**: Axum router
- **mutates**: DB (`api_keys`, `audit_entries`)

### `list` (keys)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/keys.rs`
- **purpose**: `GET /v1/keys` — returns all API keys for the tenant (no raw key values), including inactive ones. Stays open to any active tenant key regardless of role — informational only, not a mutation.
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<Vec<KeyRecord>>>`
- **calls**: `sqlx::query`
- **called_by**: Axum router
- **mutates**: nothing

### `revoke` (keys)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/keys.rs`
- **purpose**: `DELETE /v1/keys/:id` — requires the caller's own key to be `role='owner'` in this tenant (previously any active key could revoke any other key in the tenant with no check, including the tenant's own owner key). Refuses (`409 Conflict`) to revoke a tenant's last remaining active `owner` key — would otherwise lock the tenant out of managing its own keys. Deactivates the key in DB, adds to `pending_revocation` set for immediate blocking, writes a `key.revoked` audit entry.
- **inputs**: `State(state)`, `tenant`, `headers`, `Path(key_id)`
- **outputs**: `ApiResult<Json<serde_json::Value>>`
- **calls**: `resolve_caller_role`, `sqlx::query`, `state.pending_revocation.insert`, `audit_log::record`
- **called_by**: Axum router
- **mutates**: DB (`api_keys.active`), `pending_revocation` DashSet, `agent_tracker`/`rate_limiter` DashMaps (removed), `audit_entries`

### `gen_key`
- **type**: function
- **file**: `crates/hsip-api/src/routes/keys.rs`
- **purpose**: Generates a cryptographically random API key with `hsip_` prefix and 64 hex chars.
- **inputs**: none
- **outputs**: `String`
- **calls**: `OsRng.fill_bytes`, `hex::encode`
- **called_by**: `create`, `bootstrap_admin`
- **mutates**: nothing

---

## `crates/hsip-api/src/routes/admin.rs`

### `require_root_admin`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/admin.rs`
- **purpose**: Authorization gate for node-level (not tenant-scoped) operations. Requires the calling key's `is_root_admin` column to be `1` (`SELECT is_root_admin FROM api_keys WHERE key_hash=? AND active=1` — not scoped to any tenant, since the flag is node-wide). Replaced an earlier "`name == 'admin'` and tenant is the first ever created" heuristic that only ever supported exactly one root admin; `grant_root_admin`/`revoke_root_admin` are now the sanctioned way to change who holds the flag.
- **inputs**: `db: &Db`, `headers: &HeaderMap`
- **outputs**: `ApiResult<()>`
- **calls**: `hash_key`, `sqlx::query`
- **called_by**: `rotate_master_key`, `master_key_fingerprint`, `list_root_admins`, `grant_root_admin`, `revoke_root_admin`
- **mutates**: nothing

### `fingerprint` (admin)
- **type**: function
- **file**: `crates/hsip-api/src/routes/admin.rs`
- **purpose**: First 8 bytes of SHA-256(key), hex-encoded — safe to log/return over HTTP; lets an operator confirm which key is in use without ever exposing the key itself.
- **inputs**: `key_bytes: &[u8]`
- **outputs**: `String`
- **calls**: `sha2::Sha256::digest`
- **called_by**: `rotate_master_key`, `master_key_fingerprint`
- **mutates**: nothing

### `MasterKeyFingerprintResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/admin.rs`
- **purpose**: Response for `GET /v1/admin/master-key/fingerprint`: fingerprint, master_key_path (`None` when sourced from `HSIP_MASTER_KEY`), rotation_available (whether rotation currently has anywhere to persist a new key — file-backed, or env-var-sourced with `HSIP_ROTATION_HOOK` set).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `master_key_fingerprint`
- **mutates**: nothing

### `master_key_fingerprint`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/admin.rs`
- **purpose**: `GET /v1/admin/master-key/fingerprint` — read-only, no mutation. Returns the SHA-256 fingerprint of the master key currently in use, gated by the same `require_root_admin` check as rotation. Exists so an operator can confirm a backup file matches production without either grepping logs or triggering an actual rotation.
- **inputs**: `State(state)`, `_tenant: TenantId`, `headers: HeaderMap`
- **outputs**: `ApiResult<Json<MasterKeyFingerprintResponse>>`
- **calls**: `require_root_admin`, `fingerprint`, `resolve_persistence`
- **called_by**: Axum router
- **mutates**: nothing

### `KeyPersistence`
- **type**: enum
- **file**: `crates/hsip-api/src/routes/admin.rs`
- **purpose**: Where a rotated key can be durably persisted — `File(String)` (a path this process owns) or `Hook(String)` (an `HSIP_ROTATION_HOOK` command to invoke). Resolved once up front so rotation either has somewhere to put the new key or refuses before touching the database.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `resolve_persistence`, `rotate_master_key`
- **mutates**: nothing

### `resolve_persistence`
- **type**: function
- **file**: `crates/hsip-api/src/routes/admin.rs`
- **purpose**: Returns `Some(KeyPersistence::File(path))` if `state.master_key_path` is set; otherwise `Some(KeyPersistence::Hook(cmd))` if `HSIP_ROTATION_HOOK` is a non-empty env var; otherwise `None` (rotation must refuse).
- **inputs**: `state: &AppState`
- **outputs**: `Option<KeyPersistence>`
- **calls**: `std::env::var`
- **called_by**: `rotate_master_key`, `master_key_fingerprint`
- **mutates**: nothing

### `run_rotation_hook`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/admin.rs`
- **purpose**: Spawns the configured `HSIP_ROTATION_HOOK` executable, writes the new key hex-encoded to its stdin (never as a process argument — visible via `ps` on some systems), sets `HSIP_ROTATION_OLD_FINGERPRINT`/`HSIP_ROTATION_NEW_FINGERPRINT` env vars for the hook's own logging, and waits up to `ROTATION_HOOK_TIMEOUT_SECS` (30s) for it to exit. Only a zero exit code is treated as success; stderr is captured and capped at 2000 chars in the error message on failure.
- **inputs**: `hook_path: &str`, `old_key: &[u8]`, `new_key: &[u8]`
- **outputs**: `anyhow::Result<()>`
- **calls**: `tokio::process::Command`, `tokio::time::timeout`
- **called_by**: `rotate_master_key`
- **mutates**: nothing directly — the hook process itself is expected to write to wherever the operator's secrets manager lives

### `RotateMasterKeyResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/admin.rs`
- **purpose**: Response for `POST /v1/admin/master-key/rotate`: identities_reencrypted, anchor_identity_reencrypted, old/new key fingerprints, master_key_path (`Some` for file-backed) XOR rotation_hook (`Some` for hook-backed), a note (different wording per persistence mode).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `rotate_master_key`
- **mutates**: nothing

### `rotate_master_key`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/admin.rs`
- **purpose**: `POST /v1/admin/master-key/rotate` — generates a new 32-byte master key, re-encrypts every `identities.signing_key_b64` row and the singleton `anchor_identity` row under it inside one DB transaction, then persists the new key via whichever `KeyPersistence` mode `resolve_persistence` returns (file: staging file write + `fsync` + atomic rename; hook: `run_rotation_hook`) *before* committing the transaction, then swaps the in-memory key. Holds `state.master_key.write().await` for the *entire* operation (not just the final swap) — see the function's doc comment for the concurrency race this closes. Refuses with `ApiError::BadRequest` if `resolve_persistence` returns `None` (env-var-sourced key, no hook configured). Writes one `master_key.rotated` audit entry per tenant touched and increments `metrics::MASTER_KEY_ROTATIONS`.
- **inputs**: `State(state)`, `_tenant: TenantId`, `headers: HeaderMap`
- **outputs**: `ApiResult<Json<RotateMasterKeyResponse>>`
- **calls**: `require_root_admin`, `resolve_persistence`, `state.db.begin`, `key_encryption::{decrypt_signing_key, encrypt_signing_key}`, `std::fs::{File::create, rename}`, `run_rotation_hook`, `audit_log::record`, `fingerprint`
- **called_by**: Axum router
- **mutates**: `identities`, `anchor_identity` tables (within a transaction), the master key file on disk or the hook's own target, `state.master_key` (in-memory)

### `RootAdminKeyRequest`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/admin.rs`
- **purpose**: JSON body for `POST /v1/admin/root-admins/grant` and `.../revoke`: `key_id` of the target key (any tenant).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `grant_root_admin`, `revoke_root_admin`
- **mutates**: nothing

### `RootAdminRecord`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/admin.rs`
- **purpose**: Serialised root-admin key row for `GET /v1/admin/root-admins`: id, tenant_id, name, created_at.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list_root_admins`
- **mutates**: nothing

### `root_admin_count`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/admin.rs`
- **purpose**: `SELECT COUNT(*) FROM api_keys WHERE is_root_admin=1 AND active=1` — used by `revoke_root_admin` to refuse dropping the last one.
- **inputs**: `db: &Db`
- **outputs**: `ApiResult<i64>`
- **calls**: `sqlx::query`
- **called_by**: `revoke_root_admin`
- **mutates**: nothing

### `list_root_admins`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/admin.rs`
- **purpose**: `GET /v1/admin/root-admins` — root-admin-gated. Lists every active key holding the flag, across all tenants, ordered by created_at.
- **inputs**: `State(state)`, `_tenant: TenantId`, `headers: HeaderMap`
- **outputs**: `ApiResult<Json<Vec<RootAdminRecord>>>`
- **calls**: `require_root_admin`, `sqlx::query`
- **called_by**: Axum router
- **mutates**: nothing

### `grant_root_admin`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/admin.rs`
- **purpose**: `POST /v1/admin/root-admins/grant` — root-admin-gated. Sets `is_root_admin=1` on an active target key (any tenant, by id) — the mechanism that makes more than one root admin possible, since `bootstrap_admin` only ever creates the first one. Refuses if the target key is revoked. Writes an `admin.root_admin_granted` audit entry on the target's own tenant and increments `metrics::ROOT_ADMIN_CHANGES{action="granted"}`.
- **inputs**: `State(state)`, `_tenant: TenantId`, `headers: HeaderMap`, `Json(req): Json<RootAdminKeyRequest>`
- **outputs**: `ApiResult<Json<serde_json::Value>>`
- **calls**: `require_root_admin`, `sqlx::query`, `audit_log::record`, `metrics::ROOT_ADMIN_CHANGES`
- **called_by**: Axum router
- **mutates**: `api_keys.is_root_admin`, `audit_entries`

### `revoke_root_admin`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/admin.rs`
- **purpose**: `POST /v1/admin/root-admins/revoke` — root-admin-gated. Clears `is_root_admin` on a target key. Refuses (`409 Conflict`) via `root_admin_count` if this is the last root admin on the node — there is no recovery path from zero root admins except editing the database directly. Writes an `admin.root_admin_revoked` audit entry and increments `metrics::ROOT_ADMIN_CHANGES{action="revoked"}`.
- **inputs**: `State(state)`, `_tenant: TenantId`, `headers: HeaderMap`, `Json(req): Json<RootAdminKeyRequest>`
- **outputs**: `ApiResult<Json<serde_json::Value>>`
- **calls**: `require_root_admin`, `root_admin_count`, `sqlx::query`, `audit_log::record`, `metrics::ROOT_ADMIN_CHANGES`
- **called_by**: Axum router
- **mutates**: `api_keys.is_root_admin`, `audit_entries`

---

## `crates/hsip-api/src/routes/agents.rs`

### `capabilities`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/agents.rs`
- **purpose**: `GET /v1/agent/capabilities` — returns machine-readable HSIP capability spec for AI system prompts.
- **inputs**: none
- **outputs**: `impl IntoResponse`
- **calls**: `Json`
- **called_by**: Axum router
- **mutates**: nothing

### `AgentStats`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/agents.rs`
- **purpose**: Per-agent stats in list response: id, name, created_at, requests_last_minute, anomaly_count.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list` (agents)
- **mutates**: nothing

### `list` (agents)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/agents.rs`
- **purpose**: `GET /v1/agents` — returns all `ai_agent` type keys with live velocity stats from `agent_tracker`.
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<Vec<AgentStats>>>`
- **calls**: `sqlx::query_as`, `state.agent_tracker.get`
- **called_by**: Axum router
- **mutates**: nothing

### `PROBE_PORTS`
- **type**: variable (constant array)
- **file**: `crates/hsip-api/src/routes/agents.rs`
- **purpose**: List of 12 localhost ports probed for running AI agents: Ollama (11434), Vite (5173), wrangler (8787), generic HTTP agent (8080), uvicorn (8000), Flask (5000), generic agent (4000, 9000), dashboard (3001), dev-api (3000), LM Studio (1234), Jupyter (8888).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `discover`
- **mutates**: nothing

### `DiscoveredAgent`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/agents.rs`
- **purpose**: Entry in discover response: port, url, hint (service name), description, already_registered, suggested_name.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `discover`
- **mutates**: nothing

### `discover`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/agents.rs`
- **purpose**: `GET /v1/agents/discover` — probes `PROBE_PORTS` with 150ms TCP timeout, returns discovered services with registration status.
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<Vec<DiscoveredAgent>>>`
- **calls**: `tokio::spawn`, `TcpStream::connect`, `sqlx::query`
- **called_by**: Axum router
- **mutates**: nothing

---

## `crates/hsip-api/src/routes/audit.rs`

### `AuditQuery`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/audit.rs`
- **purpose**: Query params for `GET /v1/audit`: limit (max 500), offset, action filter.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list` (audit)
- **mutates**: nothing

### `AuditEntry`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/audit.rs`
- **purpose**: Serialised audit row: id, action, peer_verify_key, details, timestamp, prev_hash, entry_hash. The last two are `None` for rows written before the audit hash chain existed.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list` (audit)
- **mutates**: nothing

### `list` (audit)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/audit.rs`
- **purpose**: `GET /v1/audit` — returns paginated, optionally filtered audit log entries for the tenant, including chain fields.
- **inputs**: `State(state)`, `tenant`, `Query(params)`
- **outputs**: `ApiResult<Json<Vec<AuditEntry>>>`
- **calls**: `sqlx::query`
- **called_by**: Axum router
- **mutates**: nothing

### `AuditVerifyResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/audit.rs`
- **purpose**: Response for `GET /v1/audit/verify`: valid, checked (chained entries checked), unchained (pre-migration entries skipped), first_break_id.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `verify_chain` (audit route)
- **mutates**: nothing

### `verify_chain` (audit route)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/audit.rs`
- **purpose**: `GET /v1/audit/verify` — fetches the tenant's audit rows in chain order and recomputes the BLAKE3 hash chain server-side, reporting whether it's intact. Detects tampering by an attacker with direct DB write access, which application-level access controls alone cannot.
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<AuditVerifyResponse>>`
- **calls**: `sqlx::query`, `audit_log::verify_chain`
- **called_by**: Axum router
- **mutates**: nothing

### `AuditProofBundle`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/audit.rs`
- **purpose**: Full self-contained verification bundle for one audit entry — same shape as `routes::decisions::DecisionProofBundle`: the entry's own fields, plus `anchored`/`merkle_root`/`merkle_index`/`inclusion_proof`/`anchor_signature`/`anchor_verify_key`/`ots_status`/`ots_proof`, all `None` when not yet anchored.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `proof` (audit route)
- **mutates**: nothing

### `proof` (audit route)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/audit.rs`
- **purpose**: `GET /v1/audit/:id/proof` — the full self-contained verification bundle for one audit entry, same pattern as `routes::decisions::proof`. `anchored: false` if the entry predates the hash chain (no `entry_hash`) or hasn't been picked up by an anchor cycle yet (`anchor_id IS NULL`). Otherwise reconstructs the anchor batch's leaf set from `entry_hash`es (ordered by `merkle_index`), rebuilds the `MerkleTree`, verifies the recomputed root matches the stored `audit_anchors.merkle_root`, and regenerates the inclusion proof on demand (not stored).
- **inputs**: `State(state)`, `tenant`, `Path(id)`
- **outputs**: `ApiResult<Json<AuditProofBundle>>`
- **calls**: `sqlx::query`, `hsip_core::merkle::MerkleTree`, `ProofStepDto::from`
- **called_by**: Axum router
- **mutates**: nothing

### `VerifyAuditProofRequest` / `VerifyAuditProofResponse`
- **type**: structs
- **file**: `crates/hsip-api/src/routes/audit.rs`
- **purpose**: Request/response types for `POST /v1/audit/verify-proof` — the disclosed entry fields plus optional merkle_root/inclusion_proof/anchor_signature/anchor_verify_key, and the verification result (valid, entry_hash_matches, merkle_inclusion_valid, anchor_signature_valid, reason).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `verify_proof` (audit route)
- **mutates**: nothing

### `verify_proof` (audit route)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/audit.rs`
- **purpose**: `POST /v1/audit/verify-proof` — pure verification of an `AuditProofBundle`-shaped request, same "no `TenantId`, no `State`, no DB call" design as `routes::decisions::verify`. Recomputes `entry_hash` via `audit_log::compute_entry_hash` from the disclosed fields and compares to the claimed value; if `merkle_root`/`inclusion_proof` are present, checks Merkle inclusion; if `anchor_signature`/`anchor_verify_key` are also present, checks the anchor signature over the root. `valid` requires all present checks to pass.
- **inputs**: `Json(req): Json<VerifyAuditProofRequest>`
- **outputs**: `Json<VerifyAuditProofResponse>`
- **calls**: `audit_log::compute_entry_hash`, `hsip_core::merkle::verify_inclusion`, `anchor_job::verify_anchor_signature`
- **called_by**: Axum router
- **mutates**: nothing

---

## `crates/hsip-api/src/routes/dns.rs`

### `DnsStatusResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/dns.rs`
- **purpose**: Response for `GET /v1/dns/status`: running bool, port, blocked_count.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `status` (dns)
- **mutates**: nothing

### `EnableRequest`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/dns.rs`
- **purpose**: JSON body for `POST /v1/dns/enable`: optional port (defaults to 5300).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `enable` (dns)
- **mutates**: nothing

### `DnsLogEntry`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/dns.rs`
- **purpose**: A single DNS block log entry: timestamp, domain, qtype, action.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `log` (dns)
- **mutates**: nothing

### `DnsLogResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/dns.rs`
- **purpose**: Response for `GET /v1/dns/log`: list of `DnsLogEntry`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `log` (dns)
- **mutates**: nothing

### `status` (dns)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/dns.rs`
- **purpose**: `GET /v1/dns/status` — returns whether the DNS blocker is running and blocked count.
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<DnsStatusResponse>>`
- **calls**: `state.dns.read()`
- **called_by**: Axum router
- **mutates**: nothing

### `enable` (dns)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/dns.rs`
- **purpose**: `POST /v1/dns/enable` — starts the hsip-dns UDP server on the specified port if not already running.
- **inputs**: `State(state)`, `tenant`, `Json(req)`
- **outputs**: `ApiResult<Json<DnsStatusResponse>>`
- **calls**: `hsip_dns::start`, `tokio::spawn`
- **called_by**: Axum router
- **mutates**: `state.dns` (spawns task, updates running flag)

### `disable` (dns)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/dns.rs`
- **purpose**: `POST /v1/dns/disable` — signals the DNS server task to stop.
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<DnsStatusResponse>>`
- **calls**: `state.dns.write()` (sets stop flag)
- **called_by**: Axum router
- **mutates**: `state.dns`

### `log` (dns)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/dns.rs`
- **purpose**: `GET /v1/dns/log` — returns recent DNS block log entries from in-memory buffer.
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<DnsLogResponse>>`
- **calls**: `state.dns.read()`
- **called_by**: Axum router
- **mutates**: nothing

---

## `crates/hsip-api/src/routes/proxy.rs`

### `tracker_category`
- **type**: function
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: Returns a category label string (e.g. "advertising", "analytics") for a given tracker hostname.
- **inputs**: `host: &str`
- **outputs**: `Option<&'static str>`
- **calls**: `TRACKERS` lookup
- **called_by**: `handle_connection`, `push_event`
- **mutates**: nothing

### `TRACKERS`
- **type**: variable (static map)
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: Static map of known tracker hostnames to category labels used for proxy blocking decisions.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `tracker_category`
- **mutates**: nothing

### `ProxyStatus`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: Response for proxy status/enable/disable: running bool, port, stats summary.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `status`, `enable`, `disable`
- **mutates**: nothing

### `ProxyStats`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: Aggregated proxy statistics: total requests, blocked count, top blocked domains.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `status`, `compute_stats`
- **mutates**: nothing

### `status` (proxy)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: `GET /v1/proxy/status` — returns proxy running state and stats. Previously shipped with no `TenantId` parameter at all (reachable with zero credentials); fixed by prepending `_tenant: TenantId` — see security-review §4.19 in `CLAUDE.md`/`THREAT_MODEL.md`.
- **inputs**: `_tenant: TenantId`, `State(state)`
- **outputs**: `ApiResult<Json<ProxyStatus>>`
- **calls**: `compute_stats`
- **called_by**: Axum router
- **mutates**: nothing

### `enable` (proxy)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: `POST /v1/proxy/enable` — starts the MITM proxy thread on specified port. Previously shipped with no `TenantId` parameter at all (reachable with zero credentials); fixed by prepending `_tenant: TenantId`.
- **inputs**: `_tenant: TenantId`, `State(state)`
- **outputs**: `ApiResult<Json<ProxyStatus>>`
- **calls**: `run_proxy_thread`, `std::thread::spawn`
- **called_by**: Axum router
- **mutates**: `state.proxy` (running flag, port)

### `disable` (proxy)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: `POST /v1/proxy/disable` — signals the proxy thread to stop. Previously shipped with no `TenantId` parameter at all (reachable with zero credentials); fixed by prepending `_tenant: TenantId`.
- **inputs**: `_tenant: TenantId`, `State(state)`
- **outputs**: `ApiResult<Json<ProxyStatus>>`
- **calls**: `state.proxy.write()` (sets running false)
- **called_by**: Axum router
- **mutates**: `state.proxy`

### `log` (proxy)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: `GET /v1/proxy/log` — returns recent proxy event ring buffer contents. Previously shipped with no `TenantId` parameter at all (reachable with zero credentials — anyone could read the full traffic log); fixed by prepending `_tenant: TenantId`.
- **inputs**: `_tenant: TenantId`, `State(state)`
- **outputs**: `ApiResult<Json<Vec<ProxyEvent>>>`
- **calls**: `state.proxy.read()`
- **called_by**: Axum router
- **mutates**: nothing

### `SetupInstructions`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: Response for `GET /v1/proxy/setup`: OS-specific instructions for configuring system proxy.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `setup`
- **mutates**: nothing

### `setup`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: `GET /v1/proxy/setup` — returns OS-specific proxy configuration instructions. Previously shipped with no `TenantId` parameter at all (reachable with zero credentials); fixed by prepending `_tenant: TenantId`.
- **inputs**: `_tenant: TenantId`, `State(state)`
- **outputs**: `ApiResult<Json<SetupInstructions>>`
- **calls**: none
- **called_by**: Axum router
- **mutates**: nothing

### `now_ms` (proxy)
- **type**: function
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: Local copy of `db::now_ms` for use within the proxy thread (avoids async context requirement).
- **inputs**: none
- **outputs**: `i64`
- **calls**: `SystemTime::now`
- **called_by**: `push_event`, `handle_connection`
- **mutates**: nothing

### `push_event`
- **type**: function
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: Appends a `ProxyEvent` to the ring buffer (max 1000 entries, oldest dropped).
- **inputs**: `shared: &ProxyShared`, `event: ProxyEvent`
- **outputs**: none
- **calls**: `shared.write()`
- **called_by**: `handle_connection`
- **mutates**: `ProxyShared` ring buffer

### `run_proxy_thread`
- **type**: function
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: Binds TCP listener and accepts connections in a loop until `running` flag is cleared; spawns `handle_connection` per connection.
- **inputs**: `shared: Arc<ProxyShared>`, `port: u16`
- **outputs**: none
- **calls**: `TcpListener::bind`, `handle_connection`
- **called_by**: `enable` (spawned in thread)
- **mutates**: `ProxyShared` via `push_event`

### `handle_connection`
- **type**: function
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: Handles a single proxy client connection: reads HTTP request, dispatches to `tunnel_connect` (CONNECT) or `relay_http`.
- **inputs**: `stream: TcpStream`, `shared: Arc<ProxyShared>`
- **outputs**: none
- **calls**: `tunnel_connect`, `relay_http`, `push_event`
- **called_by**: `run_proxy_thread`
- **mutates**: network stream, `ProxyShared` ring buffer

### `tunnel_connect`
- **type**: function
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: Handles HTTP CONNECT tunneling: connects to upstream, sends 200, then relays bidirectional traffic.
- **inputs**: `stream: TcpStream`, `host: &str`, `port: u16`
- **outputs**: none
- **calls**: `TcpStream::connect`, `io::copy`
- **called_by**: `handle_connection`
- **mutates**: network streams

### `relay_http`
- **type**: function
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: Relays a plain HTTP request to its upstream host, returns the response to the client.
- **inputs**: `stream: TcpStream`, `request: &str`, `host: &str`
- **outputs**: none
- **calls**: `TcpStream::connect`, `stream.write_all`, `io::copy`
- **called_by**: `handle_connection`
- **mutates**: network streams

### `resolve`
- **type**: function
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: Extracts the `Host:` header from a raw HTTP request string.
- **inputs**: `request: &str`
- **outputs**: `Option<String>`
- **calls**: string parsing
- **called_by**: `handle_connection`
- **mutates**: nothing

### `html_escape`
- **type**: function
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: Escapes `<`, `>`, `&` for safe HTML embedding in proxy block page responses.
- **inputs**: `s: &str`
- **outputs**: `String`
- **calls**: `str::replace`
- **called_by**: `relay_http` (block page rendering)
- **mutates**: nothing

### `compute_stats`
- **type**: function
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: Aggregates the proxy ring buffer to compute total requests, blocked count, and top-5 blocked domains.
- **inputs**: `shared: &ProxyShared`
- **outputs**: `ProxyStats`
- **calls**: `shared.read()`
- **called_by**: `status`
- **mutates**: nothing


---

## `crates/hsip-api/src/routes/contacts.rs`

### `AddContactRequest`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/contacts.rs`
- **purpose**: JSON body for `POST /v1/contacts`: nickname and verify_key (base64).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `add`
- **mutates**: nothing

### `ContactRecord`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/contacts.rs`
- **purpose**: Serialised contact row: id, nickname, verify_key, created_at.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list`, `add`
- **mutates**: nothing

### `list` (contacts)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/contacts.rs`
- **purpose**: `GET /v1/contacts` — returns all contacts for the tenant.
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<Vec<ContactRecord>>>`
- **calls**: `sqlx::query_as`
- **called_by**: Axum router
- **mutates**: nothing

### `add` (contacts)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/contacts.rs`
- **purpose**: `POST /v1/contacts` — stores a new contact (nickname + verify key) for the tenant.
- **inputs**: `State(state)`, `tenant`, `Json(req)`
- **outputs**: `ApiResult<Json<ContactRecord>>`
- **calls**: `sqlx::query`
- **called_by**: Axum router
- **mutates**: DB (`contacts`)

### `remove` (contacts)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/contacts.rs`
- **purpose**: `DELETE /v1/contacts/:id` — deletes a contact by ID.
- **inputs**: `State(state)`, `tenant`, `Path(id)`
- **outputs**: `ApiResult<StatusCode>`
- **calls**: `sqlx::query`
- **called_by**: Axum router
- **mutates**: DB (`contacts`)

---

## `crates/hsip-api/src/routes/trust.rs`

### `TrustedPeer`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/trust.rs`
- **purpose**: Serialised trusted peer row: id, label, verify_key, added_at.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list`, `add`
- **mutates**: nothing

### `AddPeerRequest`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/trust.rs`
- **purpose**: JSON body for `POST /v1/trust/peer`: label (human name) and verify_key (hex Ed25519 pubkey).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `add` (trust)
- **mutates**: nothing

### `TrustVerifyRequest`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/trust.rs`
- **purpose**: JSON body for `POST /v1/trust/verify`: peer label, message content, signature hex.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `verify` (trust)
- **mutates**: nothing

### `TrustVerifyResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/trust.rs`
- **purpose**: Response from trust verify: valid bool, label, message.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `verify` (trust)
- **mutates**: nothing

### `add` (trust)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/trust.rs`
- **purpose**: `POST /v1/trust/peer` — validates key bytes, upserts trusted peer, writes `trust.peer_added` audit entry.
- **inputs**: `State(state)`, `tenant`, `Json(req)`
- **outputs**: `ApiResult<Json<TrustedPeer>>`
- **calls**: `hex::decode`, `ed25519_dalek::VerifyingKey::from_bytes`, `sqlx::query`, `audit_log::record`
- **called_by**: Axum router
- **mutates**: DB (`trusted_peers`, `audit_entries`)

### `list` (trust)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/trust.rs`
- **purpose**: `GET /v1/trust/peers` — returns all trusted peers ordered by `added_at DESC`.
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<Vec<TrustedPeer>>>`
- **calls**: `sqlx::query_as`
- **called_by**: Axum router
- **mutates**: nothing

### `remove` (trust)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/trust.rs`
- **purpose**: `DELETE /v1/trust/peers/:id` — removes a trusted peer and writes `trust.peer_removed` audit entry.
- **inputs**: `State(state)`, `tenant`, `Path(id)`
- **outputs**: `ApiResult<StatusCode>`
- **calls**: `sqlx::query`, `audit_log::record`
- **called_by**: Axum router
- **mutates**: DB (`trusted_peers`, `audit_entries`)

### `verify` (trust)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/trust.rs`
- **purpose**: `POST /v1/trust/verify` — looks up peer by label, verifies Ed25519 signature, writes `trust.verify_ok` or `trust.verify_failed` audit entry.
- **inputs**: `State(state)`, `tenant`, `Json(req)`
- **outputs**: `ApiResult<Json<TrustVerifyResponse>>`
- **calls**: `sqlx::query`, `hex::decode`, `ed25519_dalek::VerifyingKey::verify`, `audit_log::record`
- **called_by**: Axum router
- **mutates**: DB (`audit_entries`)

---

## `crates/hsip-api/src/routes/tenant.rs`

### `EraseResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/tenant.rs`
- **purpose**: Response from GDPR erase: tables_cleared list, timestamp.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `erase`
- **mutates**: nothing

### `erase`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/tenant.rs`
- **purpose**: `POST /v1/tenant/erase` — deletes all data for the tenant across all tables (GDPR right to erasure).
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<EraseResponse>>`
- **calls**: `sqlx::query` (DELETE from all tables)
- **called_by**: Axum router
- **mutates**: DB (all tenant rows deleted)

### `info`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/tenant.rs`
- **purpose**: `GET /v1/tenant` — returns tenant name and creation timestamp.
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<TenantInfo>>`
- **calls**: `sqlx::query`
- **called_by**: Axum router
- **mutates**: nothing

---

## `crates/hsip-api/src/routes/sandbox.rs`

### `ProvisionResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/sandbox.rs`
- **purpose**: Response body for `POST /v1/sandbox/provision` — contains the trial API key, expiry, base URL, and ready-to-run curl quickstart commands.
- **inputs**: none
- **outputs**: serialised JSON

### `Quickstart`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/sandbox.rs`
- **purpose**: Nested field in `ProvisionResponse`; five ready-to-paste curl commands covering sign, identity, audit, consent, and capabilities.
- **inputs**: none
- **outputs**: serialised JSON

### `provision`
- **type**: function (async handler)
- **file**: `crates/hsip-api/src/routes/sandbox.rs`
- **purpose**: `POST /v1/sandbox/provision` — no auth required. Creates an isolated tenant + 24-hour trial API key with `role='owner'` set explicitly (it's the tenant's only key, and has to be able to manage any further keys the trial user creates). Returns credentials with embedded quickstart curl commands. Only active when `HSIP_SANDBOX=true` env var is set. Rate-limited to 5 provisions per source IP per hour.
- **inputs**: `State<AppState>`, `HeaderMap`
- **outputs**: `ApiResult<Json<ProvisionResponse>>`
- **calls**: `client_ip`, `check_provision_rate`, `now_ms`, `hash_key`, `ms_to_iso`, sqlx queries (INSERT tenants, INSERT api_keys), `audit_log::record`
- **called_by**: Axum router (`POST /v1/sandbox/provision`)
- **mutates**: DB (tenants, api_keys, audit_entries), `state.sandbox_rate`

### `client_ip`
- **type**: function
- **file**: `crates/hsip-api/src/routes/sandbox.rs`
- **purpose**: Extracts client IP from `X-Forwarded-For` header (Railway/proxy) or returns "unknown".
- **inputs**: `&HeaderMap`
- **outputs**: `String`
- **called_by**: `provision`

### `check_provision_rate`
- **type**: function
- **file**: `crates/hsip-api/src/routes/sandbox.rs`
- **purpose**: IP-keyed rate limiter: max 5 provisions per IP per 60-minute window using `state.sandbox_rate` DashMap. Returns `TooManyRequests` if exceeded.
- **inputs**: `ip: &str`, `state: &AppState`
- **outputs**: `Result<(), ApiError>`
- **called_by**: `provision`
- **mutates**: `state.sandbox_rate`

### `ms_to_iso`
- **type**: function (`pub(crate)`)
- **file**: `crates/hsip-api/src/routes/sandbox.rs`
- **purpose**: Converts Unix millisecond timestamp to ISO 8601 UTC string without chrono dependency. Made `pub(crate)` so `routes::decisions::record` can reuse it for `DecisionEnvelope.timestamp_iso` instead of duplicating the calendar-math implementation.
- **inputs**: `ms: i64`
- **outputs**: `String` (e.g. `"2026-06-21T14:32:00Z"`)
- **called_by**: `provision`, `routes::decisions::record`

---

## `crates/hsip-api/src/routes/uploads.rs`

### `UploadResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/uploads.rs`
- **purpose**: Response from image upload: url path to access the uploaded file.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `upload`
- **mutates**: nothing

### `upload`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/uploads.rs`
- **purpose**: `POST /v1/uploads` — authenticated, multipart/form-data, images only (max 8MB), stores bytes in the `uploads` DB table (not the filesystem), returns a public URL. Rejects `image/svg+xml` even though it matches the `image/*` prefix check — SVG is XML and can carry a `<script>`/event-handler payload that executes if `serve` below is navigated to directly, so it was closed as a stored-XSS vector (security-review §4.19).
- **inputs**: `State(state)`, `TenantId(tenant_id)`, `Multipart`
- **outputs**: `Result<Json<UploadResponse>, (StatusCode, Json<Value>)>`
- **calls**: `sqlx::query` (INSERT into `uploads`), `axum::extract::Multipart`
- **called_by**: Axum router
- **mutates**: DB (`uploads` table)

### `serve` (uploads)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/uploads.rs`
- **purpose**: `GET /v1/uploads/:id` — public (no auth), serves the raw stored bytes with their stored `content_type`. Now also sends `x-content-type-options: nosniff` as defense-in-depth alongside the SVG rejection in `upload` — stops a browser from sniffing a mismatched-declared-type file into something script-capable (security-review §4.19).
- **inputs**: `State(state)`, `Path(id)`
- **outputs**: `impl IntoResponse`
- **calls**: `sqlx::query` (SELECT from `uploads`)
- **called_by**: Axum router
- **mutates**: nothing


---

## `crates/hsip-api/src/routes/decisions.rs`

AI-agent decision attestations: sign, chain, anchor, and independently verify
a record of "this identity made this decision." Two-tier record: accountability
metadata (`model_version`, `strategy_id`, `accountable_key`, tagged
`hsip_gov_ext` as HSIP's own draft ahead of the unpublished VCP-GOV) is clear
text; the actual decision content is never sent to or stored by HSIP, only
its `payload_hash`.

### `RecordDecisionRequest` / `RecordDecisionResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: Request/response bodies for `POST /v1/decisions`. Response is the full signed receipt (`envelope`, `event_hash`, `signature`, `issuer_verify_key`) meant to be persisted client-side (see SDK `save_receipt`).
- **called_by**: `record`

### `DecisionSummary`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: Row shape for `GET /v1/decisions` listing.
- **called_by**: `list`

### `ProofStepDto`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: Wire format (hex hash + "left"/"right") for `hsip_core::merkle::ProofStep`. `From`/`TryFrom` impls convert to/from the core type.
- **called_by**: `proof`, `verify`

### `DecisionProofBundle`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: Full self-contained verification bundle returned by `GET /v1/decisions/:id/proof` — everything a third party needs to independently verify authorship and (once anchored) tamper-evidence, with zero further calls to this server.
- **called_by**: `proof`

### `VerifyDecisionRequest` / `VerifyDecisionResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: Request/response for `POST /v1/decisions/verify`.
- **called_by**: `verify`

### `record`
- **type**: function (async handler)
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: `POST /v1/decisions` — resolves the authenticated `api_keys` row, validates fields, builds a `DecisionEnvelope` chained to the tenant's last decision (`prev_hash`), signs its `event_hash` with the tenant's Ed25519 identity, inserts it. Retries on `UNIQUE(tenant_id, prev_hash)` conflict up to `MAX_ATTEMPTS` (another request extended the chain first). Writes `decision.recorded` audit entry, increments `DECISIONS_RECORDED`.
- **inputs**: `State(state)`, `tenant: TenantId`, `headers: HeaderMap`, `Json(req): Json<RecordDecisionRequest>`
- **outputs**: `ApiResult<Json<RecordDecisionResponse>>`
- **calls**: `load_signing_key`, `hsip_core::canonical::event_hash`, `ms_to_iso`, `hash_key`, sqlx queries, `audit_log::record`, `audit_log::chain_retry_backoff`, `metrics::CHAIN_WRITE_RETRIES`
- **called_by**: Axum router
- **mutates**: DB (`decisions`, `audit_entries`)

### `list` (decisions)
- **type**: function (async handler)
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: `GET /v1/decisions` — lists the tenant's decisions, newest first.
- **inputs**: `State(state)`, `tenant: TenantId`
- **outputs**: `ApiResult<Json<Vec<DecisionSummary>>>`
- **called_by**: Axum router
- **mutates**: nothing

### `proof`
- **type**: function (async handler)
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: `GET /v1/decisions/:id/proof` — builds the full proof bundle. If unanchored, returns `anchored: false` with signature-only proof. If anchored, reconstructs the batch's leaf set from `decisions.anchor_id` ordered by `merkle_index`, rebuilds the `MerkleTree`, regenerates the inclusion proof, and defensively re-checks the recomputed root against the stored `decision_anchors.merkle_root`.
- **inputs**: `State(state)`, `tenant: TenantId`, `Path(id)`
- **outputs**: `ApiResult<Json<DecisionProofBundle>>`
- **calls**: `hsip_core::merkle::MerkleTree::from_leaves`, `MerkleTree::inclusion_proof`
- **called_by**: Axum router
- **mutates**: nothing

### `verify`
- **type**: function (async handler)
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: `POST /v1/decisions/verify` — pure verification of a self-contained bundle. Deliberately takes no `TenantId` and no `State`; makes no database call. Recomputes `event_hash` from the disclosed envelope, verifies the Ed25519 signature, and (if anchor fields are present) verifies RFC 6962 inclusion and the anchor signature. This is the function meant to be run independently of HSIP entirely.
- **inputs**: `Json(req): Json<VerifyDecisionRequest>`
- **outputs**: `Json<VerifyDecisionResponse>`
- **calls**: `hsip_core::canonical::event_hash`, `hsip_core::merkle::verify_inclusion`, `anchor_job::verify_anchor_signature`
- **called_by**: Axum router
- **mutates**: nothing

---

## `crates/hsip-api/src/anchor.rs`

OpenTimestamps calendar HTTP client — network I/O only, no DB. See module
docs for MVP scope (opaque blob storage, no `.ots` parsing, no upgrade
polling yet) and the sandbox connectivity caveat (egress policy blocks
`*.calendar.opentimestamps.org`, confirmed via the sandbox's own proxy
rejection log).

### `DEFAULT_CALENDARS`
- **type**: variable (const `&[&str]`)
- **file**: `crates/hsip-api/src/anchor.rs`
- **purpose**: Public OpenTimestamps calendar server URLs tried in order.
- **called_by**: `anchor_job::run_anchor_cycle`

### `CalendarReceipt`
- **type**: struct
- **file**: `crates/hsip-api/src/anchor.rs`
- **purpose**: One calendar's raw, opaque, not-yet-Bitcoin-confirmed response to a digest submission.
- **called_by**: `submit_digest_to`, `anchor_job`

### `submit_digest_to`
- **type**: function (async)
- **file**: `crates/hsip-api/src/anchor.rs`
- **purpose**: `POST <calendar>/digest` with the raw 32-byte digest as body, per calendar HTTP protocol. Tries each given calendar in turn, returns first success. Calendar list is a parameter (not hardcoded) so tests can point this at a local `wiremock` server instead of the real network.
- **inputs**: `calendars: &[&str]`, `digest: &[u8; 32]`
- **outputs**: `Result<CalendarReceipt>`
- **calls**: `reqwest::Client`
- **called_by**: `anchor_job::run_anchor_cycle_with_calendars`, `anchor_job::retry_pending_ots_submissions`
- **mutates**: nothing (network I/O only)

---

## `crates/hsip-api/src/anchor_job.rs`

Batches unanchored decisions, and separately unanchored audit-log entries,
into RFC 6962 Merkle trees on a "whichever comes first" cadence
(`BATCH_SIZE_TRIGGER` rows, or `INTERVAL_TRIGGER_MS` elapsed) and submits
each root to OpenTimestamps. DB-touching orchestration; `anchor.rs` is the
network client it calls into. Decisions and audit-log entries are anchored
by twin functions (`run_anchor_cycle*` / `run_audit_anchor_cycle*`) sharing
the same node-level anchor identity, since anchoring was never
decision-specific.

### `load_or_create_anchor_identity`
- **type**: function (async)
- **file**: `crates/hsip-api/src/anchor_job.rs`
- **purpose**: Loads the node-level anchor signing key from `anchor_identity` (singleton row), creating it on first use. Distinct from any tenant's identity since an anchor batch spans every tenant's rows — shared by both decision and audit-log anchoring. Handles the race of two anchor cycles both trying to create the row (loser re-reads the winner's row).
- **inputs**: `db: &Db`, `master_key: &[u8]`
- **outputs**: `anyhow::Result<SigningKey>`
- **calls**: `key_encryption::{encrypt_signing_key, decrypt_signing_key}`
- **called_by**: `run_anchor_cycle_with_calendars`, `run_audit_anchor_cycle_with_calendars`
- **mutates**: DB (`anchor_identity`, at most once ever)

### `AnchorSummary`
- **type**: struct
- **file**: `crates/hsip-api/src/anchor_job.rs`
- **purpose**: Result of one anchor cycle (`anchor_id`, `leaf_count`, `ots_status`), logged by the caller in `main.rs`'s spawned loop. Shared return type for both the decision and audit-log anchor cycles.
- **called_by**: `run_anchor_cycle_with_calendars`, `run_audit_anchor_cycle_with_calendars`, `main.rs`

### `run_anchor_cycle` / `run_anchor_cycle_with_calendars`
- **type**: function (async)
- **file**: `crates/hsip-api/src/anchor_job.rs`
- **purpose**: `run_anchor_cycle` is the production entry point (default public calendars). `run_anchor_cycle_with_calendars` does the real work: retries stuck OTS submissions, checks whether a batch is due, builds a `MerkleTree` over unanchored decisions' `event_hash`es, signs the root with the anchor identity, submits to OpenTimestamps (proceeding with local-only anchoring if that fails — `ots_status = 'calendar_unreachable'`), inserts `decision_anchors`, stamps `anchor_id`/`merkle_index` onto each covered decision, writes one `decision.anchored` audit entry per distinct tenant touched.
- **inputs**: `db: &Db`, `master_key: &[u8]`, (`calendars: &[&str]` for the `_with_calendars` form)
- **outputs**: `anyhow::Result<Option<AnchorSummary>>` (`None` when nothing was due)
- **calls**: `hsip_core::merkle::MerkleTree`, `anchor::submit_digest_to`, `load_or_create_anchor_identity`, `audit_log::record`, `metrics::ANCHOR_CALENDAR_UNREACHABLE`, `metrics::DECISIONS_ANCHORED`
- **called_by**: `main.rs`'s spawned anchor loop (which now snapshots `state.master_key` via a short-lived read lock each tick rather than holding it for the cycle's network I/O); integration tests call `run_anchor_cycle_with_calendars` directly against a mock calendar
- **mutates**: DB (`decision_anchors`, `decisions.anchor_id`/`merkle_index`, `audit_entries`)

### `run_audit_anchor_cycle` / `run_audit_anchor_cycle_with_calendars`
- **type**: function (async)
- **file**: `crates/hsip-api/src/anchor_job.rs`
- **purpose**: Twin of `run_anchor_cycle`/`run_anchor_cycle_with_calendars` for the audit log: batches `audit_entries` where `anchor_id IS NULL AND entry_hash IS NOT NULL` (rows predating the hash chain are excluded — nothing to commit to a leaf) by `entry_hash` instead of `event_hash`, into `audit_anchors` instead of `decision_anchors`. Signs with the same `anchor_identity` key. Writes one `audit.anchored` audit entry per distinct tenant touched — becomes part of a future batch itself, not this one (the `UPDATE` stamping `anchor_id` already ran before that entry is written, so no same-cycle recursion). Closes THREAT_MODEL.md §4.8's "chain not anchored outside this database" gap.
- **inputs**: `db: &Db`, `master_key: &[u8]`, (`calendars: &[&str]` for the `_with_calendars` form)
- **outputs**: `anyhow::Result<Option<AnchorSummary>>` (`None` when nothing was due)
- **calls**: `hsip_core::merkle::MerkleTree`, `anchor::submit_digest_to`, `load_or_create_anchor_identity`, `audit_log::record`, `metrics::ANCHOR_CALENDAR_UNREACHABLE`, `metrics::AUDIT_ANCHORED`, `retry_pending_audit_ots_submissions`
- **called_by**: `main.rs`'s spawned anchor loop (same tick as `run_anchor_cycle`); integration tests call `run_audit_anchor_cycle_with_calendars` directly against a mock calendar
- **mutates**: DB (`audit_anchors`, `audit_entries.anchor_id`/`merkle_index`, further `audit_entries` rows via `audit_log::record`)

### `retry_pending_ots_submissions`
- **type**: function (async)
- **file**: `crates/hsip-api/src/anchor_job.rs`
- **purpose**: Re-attempts OpenTimestamps submission for anchors stuck at `ots_status = 'calendar_unreachable'`. Best-effort — logs and moves on if a retry fails again, incrementing `metrics::ANCHOR_CALENDAR_UNREACHABLE` so the dependency's degraded state is visible over time, not just per-anchor.
- **inputs**: `db: &Db`, `calendars: &[&str]`
- **outputs**: none
- **calls**: `anchor::submit_digest_to`, `metrics::ANCHOR_CALENDAR_UNREACHABLE`
- **called_by**: `run_anchor_cycle_with_calendars`
- **mutates**: DB (`decision_anchors.ots_proof`/`ots_status` on success)

### `retry_pending_audit_ots_submissions`
- **type**: function (async)
- **file**: `crates/hsip-api/src/anchor_job.rs`
- **purpose**: Twin of `retry_pending_ots_submissions` against `audit_anchors` instead of `decision_anchors`.
- **inputs**: `db: &Db`, `calendars: &[&str]`
- **outputs**: none
- **calls**: `anchor::submit_digest_to`, `metrics::ANCHOR_CALENDAR_UNREACHABLE`
- **called_by**: `run_audit_anchor_cycle_with_calendars`
- **mutates**: DB (`audit_anchors.ots_proof`/`ots_status` on success)

### `verify_anchor_signature`
- **type**: function
- **file**: `crates/hsip-api/src/anchor_job.rs`
- **purpose**: Verifies an Ed25519 signature over a Merkle root against a given verify key. Pure — no DB. Generic over what was anchored, so both decisions and audit-log verification reuse it.
- **inputs**: `root: &[u8; 32]`, `signature: &[u8; 64]`, `verify_key: &[u8; 32]`
- **outputs**: `bool`
- **called_by**: `routes::decisions::verify`, `routes::audit::verify_proof`

---

## `crates/hsip-cli/src/main.rs`

### `Commands`
- **type**: enum
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: Top-level clap subcommand enum: Keygen, Init, Export, Import, Consent, Session, Token, Discover, Reputation, Daemon, Audit, Agent, Trust, Keys, Up, Status, Diag.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `main`
- **mutates**: nothing

### `main` (cli)
- **type**: function
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: CLI entry point: parses args, dispatches to subcommand handlers.
- **inputs**: none
- **outputs**: none
- **calls**: `clap::Parser::parse`, all command `run()` functions
- **called_by**: OS
- **mutates**: varies per subcommand

### `run_demo_site`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: Starts a local HTTP demo site that simulates a relying party for consent flow testing.
- **inputs**: `args: DemoArgs`
- **outputs**: `Result<()>`
- **calls**: `axum::serve`, `start_local_consent_http`
- **called_by**: `main` (Demo subcommand)
- **mutates**: network (binds port)

### `now_ms` (cli/main)
- **type**: function
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: Returns current Unix timestamp in milliseconds.
- **inputs**: none
- **outputs**: `u64`
- **calls**: `SystemTime::now`
- **called_by**: various CLI functions
- **mutates**: nothing

### `rand_range`
- **type**: function
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: Returns a cryptographically random u64 in `[min, max)`.
- **inputs**: `min: u64`, `max: u64`
- **outputs**: `u64`
- **calls**: `OsRng`
- **called_by**: `run_demo_site`
- **mutates**: nothing

### `mean_interval_ms`
- **type**: function
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: Computes mean inter-request interval in ms from a request rate (requests/second).
- **inputs**: `rate: f64`
- **outputs**: `u64`
- **calls**: none
- **called_by**: `run_demo_site`
- **mutates**: nothing

### `ensure_identity_silent`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: Creates an HSIP identity via the API if one doesn't exist yet; silences errors.
- **inputs**: `client: &reqwest::Client`, `api_url: &str`, `api_key: &str`
- **outputs**: none
- **calls**: `client.post`
- **called_by**: `run_demo_site`
- **mutates**: DB (via API call)

### `ensure_daemon_running`
- **type**: function
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: Checks if hsip-api is reachable at the default URL; starts it if not.
- **inputs**: none
- **outputs**: `Result<()>`
- **calls**: `reqwest::get`, `std::process::Command`
- **called_by**: `main` (Up/Daemon subcommands)
- **mutates**: OS process table

### `origin_allowed`
- **type**: function
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: Checks whether an HTTP origin is in the allowed list for local consent HTTP server.
- **inputs**: `origin: &str`, `allowed: &[String]`
- **outputs**: `bool`
- **calls**: none
- **called_by**: `start_local_consent_http`
- **mutates**: nothing

### `start_local_consent_http`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: Binds a local HTTP endpoint for consent challenge/response during the demo flow.
- **inputs**: `api_url: String`, `api_key: String`, `allowed_origins: Vec<String>`
- **outputs**: `Result<()>`
- **calls**: `axum::Router`, `axum::serve`
- **called_by**: `run_demo_site`
- **mutates**: network (binds `CONSENT_HTTP_ADDR`)

### `verify_local_token_str`
- **type**: function
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: Parses and verifies a HSIP consent token string against a local signing key.
- **inputs**: `token_str: &str`, `signing_key: &SigningKey`
- **outputs**: `Result<ConsentToken>`
- **calls**: `token::verify_token_signature`, `serde_json::from_str`
- **called_by**: `start_local_consent_http`
- **mutates**: nothing

### `CONSENT_HTTP_ADDR`
- **type**: variable (constant)
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: Local address for the consent HTTP server: `127.0.0.1:7475`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `start_local_consent_http`
- **mutates**: nothing

### `DEMO_HTTP_ADDR`
- **type**: variable (constant)
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: Local address for the demo site HTTP server: `127.0.0.1:7476`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `run_demo_site`
- **mutates**: nothing

### `TAG_E1`, `TAG_E2`, `TAG_D`
- **type**: variable (constants)
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: CBOR/AAD tag bytes for ephemeral key exchange phases (E1=initiator ephemeral, E2=responder ephemeral, D=data).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `commands::handshake`
- **mutates**: nothing

### `LABEL_CONSENT_V1`
- **type**: variable (constant)
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: HKDF label string for consent token key derivation: `"hsip-consent-v1"`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `token::issue_token`
- **mutates**: nothing

### `AAD_CONTROL`, `AAD_DATA`, `AAD_PING`
- **type**: variable (constants)
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: Additional authenticated data tags for ChaCha20-Poly1305 message frames (control channel, data channel, ping).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `commands::handshake`
- **mutates**: nothing


---

## `crates/hsip-cli/src/commands/agent.rs`

### `AgentCmd`
- **type**: enum
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: Clap subcommand enum for `hsip agent`: Register, List, Revoke, Discover.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `run` (agent)
- **mutates**: nothing

### `CreateKeyResponse` (agent)
- **type**: struct
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: Deserialises the API response from `POST /v1/keys`: id, key, name, agent_type, created_at.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `register`
- **mutates**: nothing

### `AgentStats` (cli)
- **type**: struct
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: Deserialises `GET /v1/agents` list entry: id, name, created_at, requests_last_minute, anomaly_count.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list`, `status`
- **mutates**: nothing

### `ApiClient` (agent)
- **type**: struct
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: Simple HTTP client wrapper holding `reqwest::Client`, base URL, and API key for agent commands.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `register`, `list`, `revoke`, `status`, `discover`
- **mutates**: nothing

### `ApiClient::new` (agent)
- **type**: function
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: Constructs `ApiClient` with given base URL and API key.
- **inputs**: `api_url: String`, `api_key: String`
- **outputs**: `Self`
- **calls**: `reqwest::Client::new`
- **called_by**: `run` (agent)
- **mutates**: nothing

### `ApiClient::post` (agent)
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: Sends authenticated POST with JSON body and deserialises response.
- **inputs**: `path: &str`, `body: &impl Serialize`
- **outputs**: `Result<T>`
- **calls**: `client.post().bearer_auth().json().send()`
- **called_by**: `register`
- **mutates**: nothing (network only)

### `ApiClient::get` (agent)
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: Sends authenticated GET and deserialises JSON response.
- **inputs**: `path: &str`
- **outputs**: `Result<T>`
- **calls**: `client.get().bearer_auth().send()`
- **called_by**: `list`, `status`, `discover`
- **mutates**: nothing (network only)

### `ApiClient::delete` (agent)
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: Sends authenticated DELETE request.
- **inputs**: `path: &str`
- **outputs**: `Result<()>`
- **calls**: `client.delete().bearer_auth().send()`
- **called_by**: `revoke`
- **mutates**: nothing (network only)

### `run` (agent)
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: Dispatches `AgentCmd` subcommands to `register`, `list`, `revoke`, `discover`, or `status`.
- **inputs**: `cmd: AgentCmd`, `api_url: String`, `key: String`
- **outputs**: `Result<()>`
- **calls**: `register`, `list`, `revoke`, `discover`, `status`
- **called_by**: `main`
- **mutates**: varies

### `register`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: `hsip agent register <name>` — creates an `ai_agent` API key via `POST /v1/keys`, prints the key.
- **inputs**: `client: &ApiClient`, `name: String`, `expires_days: Option<u32>`
- **outputs**: `Result<()>`
- **calls**: `client.post`, `println!`
- **called_by**: `run` (agent)
- **mutates**: DB via API

### `list` (agent cli)
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: `hsip agent list` — fetches and prints all ai_agent keys with stats table.
- **inputs**: `client: &ApiClient`
- **outputs**: `Result<()>`
- **calls**: `client.get`, `println!`, `format_timestamp`, `truncate`
- **called_by**: `run` (agent)
- **mutates**: nothing

### `revoke` (agent cli)
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: `hsip agent revoke <name-or-id>` — finds key by name or ID, deletes via `DELETE /v1/keys/:id`.
- **inputs**: `client: &ApiClient`, `name_or_id: String`
- **outputs**: `Result<()>`
- **calls**: `client.get`, `client.delete`
- **called_by**: `run` (agent)
- **mutates**: DB via API

### `status` (agent cli)
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: `hsip status` — prints a summary table of server health, identity, and active agents.
- **inputs**: `client: &ApiClient`
- **outputs**: `Result<()>`
- **calls**: `client.get` (health, identity, agents), `println!`
- **called_by**: `run` (agent)
- **mutates**: nothing

### `discover` (agent cli)
- **type**: function
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: `hsip agent discover` — calls `GET /v1/agents/discover`, prints each candidate's URL, hint, description, registration status, and (if unregistered) a suggested `hsip agent register` command.
- **inputs**: `api_url: Option<String>`, `key: Option<String>`
- **outputs**: `Result<()>`
- **calls**: `ApiClient::new`, `client.get`, `println!`
- **called_by**: `run` (agent)
- **mutates**: nothing

### `DiscoveredAgent` (cli)
- **type**: struct
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: Deserialises one entry from `GET /v1/agents/discover`: url, hint, description, already_registered, suggested_name.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `discover` (agent cli)
- **mutates**: nothing

### `load_admin_key` (agent module — delegates)
- **type**: function
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: Re-exports `super::util::load_admin_key` to avoid writing local copy.
- **inputs**: none
- **outputs**: `Result<String>`
- **calls**: `util::load_admin_key`
- **called_by**: `run` (agent)
- **mutates**: nothing

### `format_timestamp`
- **type**: function
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: Formats a Unix millisecond timestamp as a human-readable date string.
- **inputs**: `ms: i64`
- **outputs**: `String`
- **calls**: `chrono::DateTime`
- **called_by**: `list`, `status`
- **mutates**: nothing

### `truncate`
- **type**: function
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: Truncates a string to `max` chars, appending `…` if cut.
- **inputs**: `s: &str`, `max: usize`
- **outputs**: `String`
- **calls**: `str::chars().take`
- **called_by**: `list`
- **mutates**: nothing

### `DEFAULT_API_URL` (agent)
- **type**: variable (constant)
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: Default HSIP API base URL: `http://127.0.0.1:7474`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `run` (agent)
- **mutates**: nothing

---

## `crates/hsip-cli/src/commands/trust.rs`

### `TrustCmd`
- **type**: enum
- **file**: `crates/hsip-cli/src/commands/trust.rs`
- **purpose**: Clap subcommand enum for `hsip trust`: Add, List, Remove, Verify.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `run` (trust)
- **mutates**: nothing

### `TrustedPeer` (cli)
- **type**: struct
- **file**: `crates/hsip-cli/src/commands/trust.rs`
- **purpose**: Deserialises trusted peer from API: id, label, verify_key, added_at.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list`, `remove`
- **mutates**: nothing

### `TrustVerifyResponse` (cli)
- **type**: struct
- **file**: `crates/hsip-cli/src/commands/trust.rs`
- **purpose**: Deserialises verify response from API: valid bool, label, message.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `verify` (trust cli)
- **mutates**: nothing

### `run` (trust)
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/trust.rs`
- **purpose**: Dispatches `TrustCmd` variants to `add`, `list`, `remove`, or `verify`.
- **inputs**: `cmd: TrustCmd`, `api_url: String`, `key: String`
- **outputs**: `Result<()>`
- **calls**: `add`, `list`, `remove`, `verify`
- **called_by**: `main`
- **mutates**: varies

### `add` (trust cli)
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/trust.rs`
- **purpose**: `hsip trust add <label> <verify-key>` — posts to `POST /v1/trust/peer`.
- **inputs**: `client: &ApiClient`, `label: String`, `verify_key: String`
- **outputs**: `Result<()>`
- **calls**: `client.post`
- **called_by**: `run` (trust)
- **mutates**: DB via API

### `list` (trust cli)
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/trust.rs`
- **purpose**: `hsip trust list` — fetches and prints all trusted peers as a table.
- **inputs**: `client: &ApiClient`
- **outputs**: `Result<()>`
- **calls**: `client.get`, `format_ago`, `truncate`
- **called_by**: `run` (trust)
- **mutates**: nothing

### `remove` (trust cli)
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/trust.rs`
- **purpose**: `hsip trust remove <id>` — sends `DELETE /v1/trust/peers/:id`.
- **inputs**: `client: &ApiClient`, `id: String`
- **outputs**: `Result<()>`
- **calls**: `client.delete`
- **called_by**: `run` (trust)
- **mutates**: DB via API

### `verify` (trust cli)
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/trust.rs`
- **purpose**: `hsip trust verify --from <label> <content> <signature>` — posts to `POST /v1/trust/verify`, prints result.
- **inputs**: `client: &ApiClient`, `from_label: String`, `content: String`, `signature: String`
- **outputs**: `Result<()>`
- **calls**: `client.post`
- **called_by**: `run` (trust)
- **mutates**: DB (audit entry) via API

### `truncate` (trust)
- **type**: function
- **file**: `crates/hsip-cli/src/commands/trust.rs`
- **purpose**: Truncates string for table display.
- **inputs**: `s: &str`, `max: usize`
- **outputs**: `String`
- **calls**: `str::chars().take`
- **called_by**: `list` (trust cli)
- **mutates**: nothing

### `format_ago`
- **type**: function
- **file**: `crates/hsip-cli/src/commands/trust.rs`
- **purpose**: Formats a timestamp as a human-readable "N days ago" / "N hours ago" string.
- **inputs**: `ms: i64`
- **outputs**: `String`
- **calls**: `now_ms`, arithmetic
- **called_by**: `list` (trust cli)
- **mutates**: nothing

### `DEFAULT_API_URL` (trust)
- **type**: variable (constant)
- **file**: `crates/hsip-cli/src/commands/trust.rs`
- **purpose**: Default HSIP API base URL: `http://127.0.0.1:7474`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `run` (trust)
- **mutates**: nothing


---

## `crates/hsip-cli/src/commands/keys.rs`

### `KeysCmd`
- **type**: enum
- **file**: `crates/hsip-cli/src/commands/keys.rs`
- **purpose**: Clap subcommand enum for `hsip keys`: MasterFingerprint, RotateMaster, ListRootAdmins, GrantRootAdmin, RevokeRootAdmin (Rotate/Revoke have a `--yes` flag to skip the interactive confirmation prompt).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `run` (keys)
- **mutates**: nothing

### `FingerprintResponse` / `RotateResponse`
- **type**: structs
- **file**: `crates/hsip-cli/src/commands/keys.rs`
- **purpose**: Deserialize `GET /v1/admin/master-key/fingerprint` (fingerprint, master_key_path, rotation_available) and `POST /v1/admin/master-key/rotate` (adds rotation_hook alongside master_key_path — exactly one of the two is `Some`) responses respectively.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `master_fingerprint`, `rotate_master`
- **mutates**: nothing

### `RootAdminRecord` / `RootAdminChangeResponse`
- **type**: structs
- **file**: `crates/hsip-cli/src/commands/keys.rs`
- **purpose**: Deserialize `GET /v1/admin/root-admins` (list of id/tenant_id/name/created_at) and `POST /v1/admin/root-admins/grant`/`.../revoke` (`RootAdminChangeResponse` uses `#[serde(alias)]` so one field covers both endpoints' differently-named `granted`/`revoked` key) responses respectively.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list_root_admins`, `grant_root_admin`, `revoke_root_admin` (cli)
- **mutates**: nothing

### `ApiClient` (keys)
- **type**: struct
- **file**: `crates/hsip-cli/src/commands/keys.rs`
- **purpose**: Same `reqwest::blocking` wrapper pattern as `agent.rs`/`trust.rs`. `get`, `post` (empty JSON body), and `post_json` (caller-supplied body, used by grant/revoke). Uses `commands::util::load_admin_key()` — not a local copy.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `master_fingerprint`, `rotate_master`, `list_root_admins`, `grant_root_admin`, `revoke_root_admin` (cli)
- **mutates**: nothing

### `run` (keys)
- **type**: function
- **file**: `crates/hsip-cli/src/commands/keys.rs`
- **purpose**: Dispatches `KeysCmd` subcommands to `master_fingerprint`, `rotate_master`, `list_root_admins`, `grant_root_admin`, or `revoke_root_admin`.
- **inputs**: `cmd: KeysCmd`
- **outputs**: `Result<()>`
- **calls**: `master_fingerprint`, `rotate_master`, `list_root_admins`, `grant_root_admin`, `revoke_root_admin` (cli)
- **called_by**: `main`
- **mutates**: varies

### `master_fingerprint`
- **type**: function
- **file**: `crates/hsip-cli/src/commands/keys.rs`
- **purpose**: `hsip keys master-fingerprint` — calls `GET /v1/admin/master-key/fingerprint`, prints the fingerprint, its source (file path or `HSIP_MASTER_KEY`), and the shell command to independently hash a local backup file for comparison.
- **inputs**: `api_url: Option<String>`, `key: Option<String>`
- **outputs**: `Result<()>`
- **calls**: `ApiClient::new`, `client.get`, `println!`
- **called_by**: `run` (keys)
- **mutates**: nothing

### `rotate_master`
- **type**: function
- **file**: `crates/hsip-cli/src/commands/keys.rs`
- **purpose**: `hsip keys rotate-master` — unless `--yes`, prints what the operation does and requires typing `yes` at an interactive stdin prompt before calling `POST /v1/admin/master-key/rotate`. Built as a CLI command specifically because HSIP's original audience was non-technical users, for whom a security control only reachable via hand-rolled `curl` + bearer auth would be effectively unreachable; `--yes` keeps it scriptable for automated/scheduled rotation.
- **inputs**: `api_url: Option<String>`, `key: Option<String>`, `yes: bool`
- **outputs**: `Result<()>`
- **calls**: `ApiClient::new`, `std::io::stdin().read_line`, `client.post`, `println!`
- **called_by**: `run` (keys)
- **mutates**: DB (`identities`, `anchor_identity`) + master key file + in-memory key, via the API — this CLI process itself mutates nothing locally

### `list_root_admins` (cli)
- **type**: function
- **file**: `crates/hsip-cli/src/commands/keys.rs`
- **purpose**: `hsip keys list-root-admins` — calls `GET /v1/admin/root-admins`, prints id/name/tenant/created_at for each, plus the grant/revoke command hints.
- **inputs**: `api_url: Option<String>`, `key: Option<String>`
- **outputs**: `Result<()>`
- **calls**: `ApiClient::new`, `client.get`, `println!`
- **called_by**: `run` (keys)
- **mutates**: nothing

### `grant_root_admin` (cli)
- **type**: function
- **file**: `crates/hsip-cli/src/commands/keys.rs`
- **purpose**: `hsip keys grant-root-admin <target-key-id>` — calls `POST /v1/admin/root-admins/grant` with `{"key_id": target_key_id}` via `client.post_json`, prints confirmation.
- **inputs**: `target_key_id: String`, `api_url: Option<String>`, `key: Option<String>`
- **outputs**: `Result<()>`
- **calls**: `ApiClient::new`, `client.post_json`, `println!`
- **called_by**: `run` (keys)
- **mutates**: `api_keys.is_root_admin` via the API — this CLI process itself mutates nothing locally

### `revoke_root_admin` (cli)
- **type**: function
- **file**: `crates/hsip-cli/src/commands/keys.rs`
- **purpose**: `hsip keys revoke-root-admin <target-key-id>` — unless `--yes`, prints what the operation does and requires typing `yes` at an interactive stdin prompt before calling `POST /v1/admin/root-admins/revoke`. Same non-technical-audience reasoning as `rotate_master`.
- **inputs**: `target_key_id: String`, `api_url: Option<String>`, `key: Option<String>`, `yes: bool`
- **outputs**: `Result<()>`
- **calls**: `ApiClient::new`, `std::io::stdin().read_line`, `client.post_json`, `println!`
- **called_by**: `run` (keys)
- **mutates**: `api_keys.is_root_admin` via the API — this CLI process itself mutates nothing locally

---

## `crates/hsip-cli/src/commands/up.rs`

### `UpArgs`
- **type**: struct
- **file**: `crates/hsip-cli/src/commands/up.rs`
- **purpose**: Clap args for `hsip up`: optional `--api-url` and `--no-browser` flag.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `run` (up)
- **mutates**: nothing

### `IdentityResponse` (up)
- **type**: struct
- **file**: `crates/hsip-cli/src/commands/up.rs`
- **purpose**: Deserialises identity response: verify_key, created_at.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `get_identity`
- **mutates**: nothing

### `run` (up)
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/up.rs`
- **purpose**: `hsip up` — checks server health, starts it if down, ensures identity exists, opens dashboard in browser, prints welcome box.
- **inputs**: `args: UpArgs`
- **outputs**: `Result<()>`
- **calls**: `probe_health`, `find_hsip_api_bin`, `get_identity`, `open_in_browser`, `print_start_hint`
- **called_by**: `main`
- **mutates**: process table (may spawn server), browser

### `probe_health`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/up.rs`
- **purpose**: Polls `GET /health` up to `HEALTH_RETRIES` times with 1-second backoff; returns true when server responds.
- **inputs**: `api_url: &str`
- **outputs**: `bool`
- **calls**: `reqwest::get`
- **called_by**: `run` (up)
- **mutates**: nothing

### `find_hsip_api_bin`
- **type**: function
- **file**: `crates/hsip-cli/src/commands/up.rs`
- **purpose**: Searches PATH and well-known locations for the `hsip-api` binary.
- **inputs**: none
- **outputs**: `Option<PathBuf>`
- **calls**: `which::which`, `std::env::current_exe`
- **called_by**: `run` (up)
- **mutates**: nothing

### `get_identity`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/up.rs`
- **purpose**: Calls `POST /v1/identity` to create/get the tenant's Ed25519 identity; returns verify key.
- **inputs**: `api_url: &str`, `api_key: &str`
- **outputs**: `Result<IdentityResponse>`
- **calls**: `reqwest::Client::post`
- **called_by**: `run` (up)
- **mutates**: DB via API

### `open_in_browser`
- **type**: function
- **file**: `crates/hsip-cli/src/commands/up.rs`
- **purpose**: Opens the HSIP dashboard URL in the system default browser.
- **inputs**: `url: &str`
- **outputs**: none
- **calls**: `open::that` / platform shell command
- **called_by**: `run` (up)
- **mutates**: nothing (launches process)

### `print_start_hint`
- **type**: function
- **file**: `crates/hsip-cli/src/commands/up.rs`
- **purpose**: Prints the HSIP welcome box with server URL, verify key, and usage hints to stdout.
- **inputs**: `api_url: &str`, `verify_key: &str`
- **outputs**: none
- **calls**: `println!`
- **called_by**: `run` (up)
- **mutates**: stdout

### `DEFAULT_API_URL` (up)
- **type**: variable (constant)
- **file**: `crates/hsip-cli/src/commands/up.rs`
- **purpose**: Default HSIP API base URL: `http://127.0.0.1:7474`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `run` (up)
- **mutates**: nothing

### `HEALTH_RETRIES`
- **type**: variable (constant)
- **file**: `crates/hsip-cli/src/commands/up.rs`
- **purpose**: Number of health-check retries when waiting for server to start: `10`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `probe_health`
- **mutates**: nothing

---

## `crates/hsip-cli/src/commands/util.rs`

### `admin_key_path`
- **type**: function
- **file**: `crates/hsip-cli/src/commands/util.rs`
- **purpose**: Returns platform-aware path to admin.key: `%APPDATA%\HSIP\admin.key` on Windows, `~/.hsip/admin.key` on Unix.
- **inputs**: none
- **outputs**: `PathBuf`
- **calls**: `std::env::var("APPDATA")`, `std::env::var("HOME")`
- **called_by**: `load_admin_key`
- **mutates**: nothing

### `load_admin_key`
- **type**: function
- **file**: `crates/hsip-cli/src/commands/util.rs`
- **purpose**: Reads the admin API key from the platform-aware key file; returns trimmed string.
- **inputs**: none
- **outputs**: `Result<String>`
- **calls**: `admin_key_path`, `fs::read_to_string`
- **called_by**: all CLI command `run()` functions needing auth
- **mutates**: nothing

---

## `crates/hsip-cli/src/commands/diag.rs`

### `run_diag`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/diag.rs`
- **purpose**: `hsip diag` — runs all diagnostic sections and prints a full system report.
- **inputs**: `args: DiagArgs`
- **outputs**: `Result<()>`
- **calls**: `print_identity_section`, `print_config_section`, `print_env_section`, `print_endpoints_section`, `nonce_replay_selftest`
- **called_by**: `main`
- **mutates**: nothing

### `print_identity_section`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/diag.rs`
- **purpose**: Prints the local identity (verify key, creation time) fetched from the API.
- **inputs**: `api_url: &str`, `api_key: &str`
- **outputs**: none
- **calls**: `reqwest::get`
- **called_by**: `run_diag`
- **mutates**: stdout

### `print_config_section`
- **type**: function
- **file**: `crates/hsip-cli/src/commands/diag.rs`
- **purpose**: Prints resolved config file path and key file paths.
- **inputs**: none
- **outputs**: none
- **calls**: `admin_key_path`, `println!`
- **called_by**: `run_diag`
- **mutates**: stdout

### `print_env_section`
- **type**: function
- **file**: `crates/hsip-cli/src/commands/diag.rs`
- **purpose**: Prints relevant environment variables (HSIP_API_KEY, HSIP_API_URL, etc.) with redaction for secrets.
- **inputs**: none
- **outputs**: none
- **calls**: `print_env_var`
- **called_by**: `run_diag`
- **mutates**: stdout

### `print_env_var`
- **type**: function
- **file**: `crates/hsip-cli/src/commands/diag.rs`
- **purpose**: Prints one environment variable, redacting the value if it looks like a secret.
- **inputs**: `name: &str`, `redact: bool`
- **outputs**: none
- **calls**: `std::env::var`, `println!`
- **called_by**: `print_env_section`
- **mutates**: stdout

### `print_endpoints_section`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/diag.rs`
- **purpose**: Probes all known HSIP API endpoints and prints reachability status.
- **inputs**: `api_url: &str`, `api_key: &str`
- **outputs**: none
- **calls**: `reqwest::get`
- **called_by**: `run_diag`
- **mutates**: stdout

### `nonce_replay_selftest`
- **type**: function
- **file**: `crates/hsip-cli/src/commands/diag.rs`
- **purpose**: Runs the hsip-core `NonceWindow` self-test and prints pass/fail.
- **inputs**: none
- **outputs**: none
- **calls**: `hsip_core::nonce::NonceWindow`
- **called_by**: `run_diag`
- **mutates**: stdout

---

## `crates/hsip-cli/src/commands/handshake.rs`

### `run_listen`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/handshake.rs`
- **purpose**: `hsip handshake listen` — waits for an incoming UDP handshake and completes the X25519 key exchange.
- **inputs**: `args: ListenArgs`
- **outputs**: `Result<()>`
- **calls**: `UdpSocket::bind`, `ChaCha20Poly1305::encrypt/decrypt`
- **called_by**: `main`
- **mutates**: network

### `run_connect`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/commands/handshake.rs`
- **purpose**: `hsip handshake connect` — initiates UDP handshake to a peer address and completes key exchange.
- **inputs**: `args: ConnectArgs`
- **outputs**: `Result<()>`
- **calls**: `UdpSocket::connect`, `ChaCha20Poly1305`
- **called_by**: `main`
- **mutates**: network


---

## `crates/hsip-cli/src/identity.rs`

### `HSIP_KEY`
- **type**: variable (`Lazy<Vec<u8>>`)
- **file**: `crates/hsip-cli/src/identity.rs`
- **purpose**: HMAC-SHA256 key used to sign the local `/token` broker's JWTs. Reads `HSIP_LOCAL_JWT_KEY_HEX` if set (stable key across restarts); previously fell back to a fixed, publicly-known hex string checked into this open-source repo when unset — since `/token` requires no auth and accepts any caller-supplied `aud`, that let anyone who'd read this source forge a valid token for any relying party trusting an unconfigured broker. Fixed (security-review §4.19) to fall back to a fresh 32-byte `OsRng`-generated key per process instead — an unconfigured broker is now unpredictable key material, not a publicly known secret.
- **inputs**: none
- **outputs**: none
- **calls**: `std::env::var`, `hex::decode`, `rand::rngs::OsRng::fill_bytes`
- **called_by**: `token`
- **mutates**: nothing (initialized once, `Lazy`)

### `Status`
- **type**: struct
- **file**: `crates/hsip-cli/src/identity.rs`
- **purpose**: `/status` response: `ok: bool`, `version: &'static str`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `run_identity_broker` (inline handler)
- **mutates**: nothing

### `TokenReq`
- **type**: struct
- **file**: `crates/hsip-cli/src/identity.rs`
- **purpose**: `/token` request body: `aud: Option<String>`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `token`
- **mutates**: nothing

### `TokenResp`
- **type**: struct
- **file**: `crates/hsip-cli/src/identity.rs`
- **purpose**: `/token` response: signed JWT string.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `token`
- **mutates**: nothing

### `run_identity_broker`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/identity.rs`
- **purpose**: Starts a local HTTP broker (`HSIP_IDENTITY_ADDR`, default `127.0.0.1:9100`) serving `/status`, `/token`, `/demo` — lets a local web page request a short-lived signed identity token without a username/password.
- **inputs**: none
- **outputs**: `anyhow::Result<()>`
- **calls**: `axum::serve`, `token`, `demo`
- **called_by**: `main` (`hsip identity-serve`)
- **mutates**: network (binds port)

### `token`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/identity.rs`
- **purpose**: `POST /token` handler: builds `iss`/`sub`/`iat`/`exp`/`aud` claims, signs them with `HSIP_KEY` via HMAC-SHA256, returns the JWT. No authentication on the caller — anyone reaching the broker's port can request a token, by design (local demo login flow); the signing key itself is what changed in the security fix, not this handler's auth model.
- **inputs**: `Json(req): Json<TokenReq>`
- **outputs**: `impl IntoResponse`
- **calls**: `HmacSha256::new_from_slice`, `Token::sign_with_key`
- **called_by**: `run_identity_broker` (Axum route)
- **mutates**: nothing

### `demo`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/identity.rs`
- **purpose**: Serves a static HTML/JS demo page that calls `/token` on button click and displays a truncated token.
- **inputs**: none
- **outputs**: `impl IntoResponse`
- **calls**: `Html`
- **called_by**: `run_identity_broker` (Axum route)
- **mutates**: nothing

---

## `crates/hsip-cli/src/discovery.rs`

### `PeerEntry`
- **type**: struct
- **file**: `crates/hsip-cli/src/discovery.rs`
- **purpose**: A discovered HSIP peer: address, verify_key, name, last_seen timestamp.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `Directory`, `add`, `list`
- **mutates**: nothing

### `Directory`
- **type**: struct
- **file**: `crates/hsip-cli/src/discovery.rs`
- **purpose**: In-memory list of `PeerEntry` items loaded from/saved to the local peer directory file.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list`, `add`, `remove`
- **mutates**: nothing

### `dir_path`
- **type**: function
- **file**: `crates/hsip-cli/src/discovery.rs`
- **purpose**: Returns path to the peer directory JSON file: `~/.hsip/peers.json`.
- **inputs**: none
- **outputs**: `PathBuf`
- **calls**: `std::env::var("HOME")`
- **called_by**: `list`, `save`
- **mutates**: nothing

### `list` (discovery)
- **type**: function
- **file**: `crates/hsip-cli/src/discovery.rs`
- **purpose**: Loads and returns all known peers from the directory file.
- **inputs**: none
- **outputs**: `Result<Directory>`
- **calls**: `dir_path`, `fs::read_to_string`, `serde_json::from_str`
- **called_by**: CLI discovery commands
- **mutates**: nothing

### `save`
- **type**: function
- **file**: `crates/hsip-cli/src/discovery.rs`
- **purpose**: Serialises and writes the `Directory` back to the peer directory file.
- **inputs**: `dir: &Directory`
- **outputs**: `Result<()>`
- **calls**: `dir_path`, `serde_json::to_string_pretty`, `fs::write`
- **called_by**: `add`, `remove`
- **mutates**: filesystem (`~/.hsip/peers.json`)

### `add` (discovery)
- **type**: function
- **file**: `crates/hsip-cli/src/discovery.rs`
- **purpose**: Adds or updates a peer entry in the directory and saves.
- **inputs**: `entry: PeerEntry`
- **outputs**: `Result<()>`
- **calls**: `list`, `save`
- **called_by**: CLI discovery commands
- **mutates**: `~/.hsip/peers.json`

### `remove` (discovery)
- **type**: function
- **file**: `crates/hsip-cli/src/discovery.rs`
- **purpose**: Removes a peer by address from the directory and saves.
- **inputs**: `address: &str`
- **outputs**: `Result<()>`
- **calls**: `list`, `save`
- **called_by**: CLI discovery commands
- **mutates**: `~/.hsip/peers.json`

---

## `crates/hsip-cli/src/config.rs`

### `Config` (cli)
- **type**: struct
- **file**: `crates/hsip-cli/src/config.rs`
- **purpose**: CLI-level configuration: api_url, api_key, and network/policy sub-configs.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `read_config`
- **mutates**: nothing

### `Net`
- **type**: struct
- **file**: `crates/hsip-cli/src/config.rs`
- **purpose**: Network config for CLI: timeout_ms, retry_count.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `Config` (cli)
- **mutates**: nothing

### `Policy`
- **type**: struct
- **file**: `crates/hsip-cli/src/config.rs`
- **purpose**: Policy settings for CLI: auto_consent bool, consent_ttl_ms.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `Config` (cli)
- **mutates**: nothing

### `apply`
- **type**: function
- **file**: `crates/hsip-cli/src/config.rs`
- **purpose**: Applies env var overrides to a `Config (cli)` struct.
- **inputs**: `config: &mut Config`
- **outputs**: none
- **calls**: `std::env::var`
- **called_by**: `read_config`
- **mutates**: `Config` fields

### `read_config`
- **type**: function
- **file**: `crates/hsip-cli/src/config.rs`
- **purpose**: Reads CLI config from `~/.hsip/cli.toml` (if present), applies env overrides, returns Config.
- **inputs**: none
- **outputs**: `Result<Config>`
- **calls**: `default_path`, `fs::read_to_string`, `toml::from_str`, `apply`
- **called_by**: CLI subcommands needing config
- **mutates**: nothing

### `default_path`
- **type**: function
- **file**: `crates/hsip-cli/src/config.rs`
- **purpose**: Returns the default CLI config file path: `~/.hsip/cli.toml`.
- **inputs**: none
- **outputs**: `PathBuf`
- **calls**: `std::env::var("HOME")`
- **called_by**: `read_config`
- **mutates**: nothing

### `set_if_empty`
- **type**: function
- **file**: `crates/hsip-cli/src/config.rs`
- **purpose**: Sets a String field to the given value only if it is currently empty.
- **inputs**: `field: &mut String`, `value: String`
- **outputs**: none
- **calls**: `String::is_empty`
- **called_by**: `apply`
- **mutates**: `field`


---

## `crates/hsip-cli/src/token.rs`

### `Capability`
- **type**: enum
- **file**: `crates/hsip-cli/src/token.rs`
- **purpose**: Possible consent capabilities: Read, Write, Admin, Custom(String).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `ConsentToken`
- **mutates**: nothing

### `ConsentToken`
- **type**: struct
- **file**: `crates/hsip-cli/src/token.rs`
- **purpose**: Decoded consent token payload: subject, audience, scope, capabilities, issued_at, expires_at, nonce.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `issue_token`, `verify_token`
- **mutates**: nothing

### `TokenIssueArgs`
- **type**: struct
- **file**: `crates/hsip-cli/src/token.rs`
- **purpose**: Clap args for `hsip token issue`: audience, capabilities, ttl.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `issue_token`
- **mutates**: nothing

### `TokenVerifyArgs`
- **type**: struct
- **file**: `crates/hsip-cli/src/token.rs`
- **purpose**: Clap args for `hsip token verify`: token string.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `verify_token`
- **mutates**: nothing

### `TokenCmd`
- **type**: enum
- **file**: `crates/hsip-cli/src/token.rs`
- **purpose**: Clap subcommand enum for `hsip token`: Issue, Verify.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `main`
- **mutates**: nothing

### `issue_token`
- **type**: function
- **file**: `crates/hsip-cli/src/token.rs`
- **purpose**: Generates and signs a `ConsentToken` using the local Ed25519 key; prints the base64-encoded token.
- **inputs**: `args: TokenIssueArgs`, `signing_key: &SigningKey`
- **outputs**: `Result<()>`
- **calls**: `generate_signed_token`, `println!`
- **called_by**: `main`
- **mutates**: nothing

### `verify_token`
- **type**: function
- **file**: `crates/hsip-cli/src/token.rs`
- **purpose**: Decodes and verifies a `ConsentToken`, printing its contents or an error.
- **inputs**: `args: TokenVerifyArgs`, `verify_key: &VerifyingKey`
- **outputs**: `Result<()>`
- **calls**: `check_token_expiration`, `verify_token_signature`
- **called_by**: `main`
- **mutates**: nothing

### `check_token_expiration`
- **type**: function
- **file**: `crates/hsip-cli/src/token.rs`
- **purpose**: Returns error if token's `expires_at` is in the past.
- **inputs**: `token: &ConsentToken`
- **outputs**: `Result<()>`
- **calls**: `current_timestamp_ms`
- **called_by**: `verify_token`
- **mutates**: nothing

### `verify_token_signature`
- **type**: function
- **file**: `crates/hsip-cli/src/token.rs`
- **purpose**: Verifies the Ed25519 signature on a token's canonical bytes.
- **inputs**: `token: &ConsentToken`, `verify_key: &VerifyingKey`
- **outputs**: `Result<()>`
- **calls**: `serialize_token_for_signing`, `ed25519_dalek::VerifyingKey::verify_strict`
- **called_by**: `verify_token`, `verify_local_token_str`
- **mutates**: nothing

### `generate_signed_token`
- **type**: function
- **file**: `crates/hsip-cli/src/token.rs`
- **purpose**: Creates a `ConsentToken`, signs it with the Ed25519 key, base64-encodes and returns the string.
- **inputs**: `payload: ConsentToken`, `signing_key: &SigningKey`
- **outputs**: `Result<String>`
- **calls**: `serialize_token_for_signing`, `signing_key.sign`, `base64::encode`
- **called_by**: `issue_token`, `start_local_consent_http`
- **mutates**: nothing

### `serialize_token_for_signing`
- **type**: function
- **file**: `crates/hsip-cli/src/token.rs`
- **purpose**: Canonically serialises a `ConsentToken` to bytes for signing/verification.
- **inputs**: `token: &ConsentToken`
- **outputs**: `Result<Vec<u8>>`
- **calls**: `serde_json::to_vec`
- **called_by**: `generate_signed_token`, `verify_token_signature`
- **mutates**: nothing

### `current_timestamp_ms` (token)
- **type**: function
- **file**: `crates/hsip-cli/src/token.rs`
- **purpose**: Returns current Unix millisecond timestamp as `u64`.
- **inputs**: none
- **outputs**: `u64`
- **calls**: `SystemTime::now`
- **called_by**: `issue_token`, `check_token_expiration`
- **mutates**: nothing

---

## `crates/hsip-cli/src/rekey.rs`

### `RotateArgs`
- **type**: struct
- **file**: `crates/hsip-cli/src/rekey.rs`
- **purpose**: Clap args for `hsip rekey rotate`: key file path, peer addresses.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `rotate_key_make_rebind`
- **mutates**: nothing

### `RevokeArgs`
- **type**: struct
- **file**: `crates/hsip-cli/src/rekey.rs`
- **purpose**: Clap args for `hsip rekey revoke`: revocation reason.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `revoke_current`
- **mutates**: nothing

### `RebindProof`
- **type**: struct
- **file**: `crates/hsip-cli/src/rekey.rs`
- **purpose**: Signed proof that a new key is bound to the old key: old_verify_key, new_verify_key, timestamp, signature.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `rotate_key_make_rebind`
- **mutates**: nothing

### `RevocationRecord`
- **type**: struct
- **file**: `crates/hsip-cli/src/rekey.rs`
- **purpose**: Signed revocation record: verify_key, reason, timestamp, signature.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `revoke_current`
- **mutates**: nothing

### `rotate_key_make_rebind`
- **type**: function
- **file**: `crates/hsip-cli/src/rekey.rs`
- **purpose**: Generates a new Ed25519 keypair, creates a signed rebind proof linking old to new key, saves both to files.
- **inputs**: `args: RotateArgs`
- **outputs**: `Result<()>`
- **calls**: `ed25519_dalek::SigningKey::generate`, `signing_key.sign`, `write_json`
- **called_by**: `main`
- **mutates**: filesystem (new key file, rebind proof JSON)

### `revoke_current`
- **type**: function
- **file**: `crates/hsip-cli/src/rekey.rs`
- **purpose**: Creates and signs a revocation record for the current key, saves it to `revocation.json`.
- **inputs**: `args: RevokeArgs`
- **outputs**: `Result<()>`
- **calls**: `signing_key.sign`, `write_json`
- **called_by**: `main`
- **mutates**: filesystem (`revocation.json`)

### `write_json`
- **type**: function
- **file**: `crates/hsip-cli/src/rekey.rs`
- **purpose**: Serialises a value to pretty JSON and writes to the given path.
- **inputs**: `path: &str`, `value: &impl Serialize`
- **outputs**: `Result<()>`
- **calls**: `serde_json::to_string_pretty`, `fs::write`
- **called_by**: `rotate_key_make_rebind`, `revoke_current`
- **mutates**: filesystem

### `current_timestamp_ms` (rekey)
- **type**: function
- **file**: `crates/hsip-cli/src/rekey.rs`
- **purpose**: Returns current Unix millisecond timestamp as `u64`.
- **inputs**: none
- **outputs**: `u64`
- **calls**: `SystemTime::now`
- **called_by**: `rotate_key_make_rebind`, `revoke_current`
- **mutates**: nothing

---

## `crates/hsip-cli/src/cmd_rep.rs`

### `RepArgs`
- **type**: struct
- **file**: `crates/hsip-cli/src/cmd_rep.rs`
- **purpose**: Clap args for `hsip reputation` commands: peer address, log path.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `run_rep`
- **mutates**: nothing

### `RepCmd`
- **type**: enum
- **file**: `crates/hsip-cli/src/cmd_rep.rs`
- **purpose**: Subcommand enum for reputation: Score, Log, Verify.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `run_rep`
- **mutates**: nothing

### `run_rep`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/cmd_rep.rs`
- **purpose**: Dispatches reputation subcommands: score lookup, decision log append, signature verification.
- **inputs**: `args: RepArgs`
- **outputs**: `Result<()>`
- **calls**: `load_signing_key_and_peer_id`, `load_keys_for_verify`, `parse_decision_type`
- **called_by**: `main`
- **mutates**: log file (append on Log)

### `default_log_path`
- **type**: function
- **file**: `crates/hsip-cli/src/cmd_rep.rs`
- **purpose**: Returns default reputation log path: `~/.hsip/reputation.log`.
- **inputs**: none
- **outputs**: `String`
- **calls**: `home_dir_string`
- **called_by**: `RepArgs` default
- **mutates**: nothing

### `home_dir_string`
- **type**: function
- **file**: `crates/hsip-cli/src/cmd_rep.rs`
- **purpose**: Returns `$HOME` as a String, falling back to `.`.
- **inputs**: none
- **outputs**: `String`
- **calls**: `std::env::var("HOME")`
- **called_by**: `default_log_path`
- **mutates**: nothing

### `load_signing_key_and_peer_id`
- **type**: function
- **file**: `crates/hsip-cli/src/cmd_rep.rs`
- **purpose**: Loads the local Ed25519 signing key from file and derives the peer ID (verify key hex).
- **inputs**: `key_path: &str`
- **outputs**: `Result<(SigningKey, String)>`
- **calls**: `fs::read`, `ed25519_dalek::SigningKey::from_bytes`, `hex::encode`
- **called_by**: `run_rep`
- **mutates**: nothing

### `load_keys_for_verify`
- **type**: function
- **file**: `crates/hsip-cli/src/cmd_rep.rs`
- **purpose**: Loads a peer's Ed25519 verify key from hex string for signature verification.
- **inputs**: `hex_key: &str`
- **outputs**: `Result<VerifyingKey>`
- **calls**: `hex::decode`, `ed25519_dalek::VerifyingKey::from_bytes`
- **called_by**: `run_rep`
- **mutates**: nothing

### `parse_decision_type`
- **type**: function
- **file**: `crates/hsip-cli/src/cmd_rep.rs`
- **purpose**: Parses a reputation decision type string into an enum variant (Allow, Deny, Flag, Observe).
- **inputs**: `s: &str`
- **outputs**: `Result<DecisionType>`
- **calls**: string matching
- **called_by**: `run_rep`
- **mutates**: nothing


---

## `crates/hsip-mcp/src/main.rs`

### `RpcRequest`
- **type**: struct
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: Incoming MCP JSON-RPC 2.0 request: id (optional), method, params.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `handle`
- **mutates**: nothing

### `RpcResponse`
- **type**: struct
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: Outgoing MCP JSON-RPC 2.0 response: jsonrpc version, id, result or error.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `handle`, `send`
- **mutates**: nothing

### `RpcResponse::ok`
- **type**: function
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: Constructs a successful `RpcResponse` with a result value.
- **inputs**: `id: Value`, `result: Value`
- **outputs**: `Self`
- **calls**: none
- **called_by**: `handle`
- **mutates**: nothing

### `RpcResponse::err`
- **type**: function
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: Constructs an error `RpcResponse` with a code and message.
- **inputs**: `id: Value`, `code: i32`, `message: String`
- **outputs**: `Self`
- **calls**: `serde_json::json!`
- **called_by**: `handle`
- **mutates**: nothing

### `ApiClient` (mcp)
- **type**: struct
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: HTTP client for MCP server: `reqwest::Client`, base URL, API key.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `call_tool`
- **mutates**: nothing

### `ApiClient::post` (mcp)
- **type**: function (async)
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: Sends authenticated POST with JSON body to HSIP API, returns `Value`.
- **inputs**: `path: &str`, `body: Value`
- **outputs**: `Result<Value>`
- **calls**: `client.post().bearer_auth().json().send()`
- **called_by**: `call_tool`
- **mutates**: nothing (network)

### `ApiClient::get` (mcp)
- **type**: function (async)
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: Sends authenticated GET to HSIP API, returns `Value`.
- **inputs**: `path: &str`
- **outputs**: `Result<Value>`
- **calls**: `client.get().bearer_auth().send()`
- **called_by**: `call_tool`
- **mutates**: nothing (network)

### `load_admin_key` (mcp)
- **type**: function
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: Reads admin key from platform-aware path for MCP server startup.
- **inputs**: none
- **outputs**: `Result<String>`
- **calls**: `admin_key_path`, `fs::read_to_string`
- **called_by**: `main` (mcp)
- **mutates**: nothing

### `tool_list`
- **type**: function
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: Returns the JSON array of MCP tool definitions (sign_message, verify_message, get_identity, grant_consent, check_consent, revoke_consent, log_action, get_recent_actions).
- **inputs**: none
- **outputs**: `Value`
- **calls**: `serde_json::json!`
- **called_by**: `handle` (tools/list method)
- **mutates**: nothing

### `call_tool`
- **type**: function (async)
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: Dispatches MCP `tools/call` to the appropriate HSIP API endpoint based on tool name.
- **inputs**: `api: &ApiClient`, `name: &str`, `params: &Value`
- **outputs**: `Value`
- **calls**: `api.post`, `api.get`, `tool_result`, `tool_error`, `urlenc`
- **called_by**: `handle`
- **mutates**: DB via API (for sign, consent, log actions)

### `main` (mcp)
- **type**: function (async)
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: MCP server entry point: reads API key/URL from env, starts JSON-RPC loop reading from stdin.
- **inputs**: none
- **outputs**: `Result<()>`
- **calls**: `load_admin_key`, `handle`, `send`
- **called_by**: OS
- **mutates**: stdout (JSON-RPC responses)

### `handle`
- **type**: function (async)
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: Handles one JSON-RPC request: dispatches `initialize`, `ping`, `tools/list`, `tools/call`; ignores notifications (no `id`).
- **inputs**: `req: RpcRequest`, `api: &ApiClient`
- **outputs**: `Option<RpcResponse>`
- **calls**: `tool_list`, `call_tool`, `RpcResponse::ok`, `RpcResponse::err`
- **called_by**: `main` (mcp)
- **mutates**: nothing directly

### `send`
- **type**: function
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: Serialises `RpcResponse` to JSON and writes to stdout with newline.
- **inputs**: `resp: RpcResponse`
- **outputs**: none
- **calls**: `serde_json::to_string`, `println!`
- **called_by**: `main` (mcp)
- **mutates**: stdout

### `req_str`
- **type**: function
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: Extracts a required string field from MCP params, returning error if missing.
- **inputs**: `params: &Value`, `key: &str`
- **outputs**: `Result<String>`
- **calls**: `params[key].as_str()`
- **called_by**: `call_tool`
- **mutates**: nothing

### `tool_result`
- **type**: function
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: Wraps an API response value as an MCP tool result content array.
- **inputs**: `value: Value`
- **outputs**: `Value`
- **calls**: `serde_json::json!`
- **called_by**: `call_tool`
- **mutates**: nothing

### `tool_error`
- **type**: function
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: Wraps an error string as an MCP tool error result.
- **inputs**: `msg: String`
- **outputs**: `Value`
- **calls**: `serde_json::json!`
- **called_by**: `call_tool`
- **mutates**: nothing

### `urlenc`
- **type**: function
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: URL-encodes a string for use in query parameters.
- **inputs**: `s: &str`
- **outputs**: `String`
- **calls**: `percent_encoding::utf8_percent_encode`
- **called_by**: `call_tool`
- **mutates**: nothing

### `DEFAULT_API_URL` (mcp)
- **type**: variable (constant)
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: Default HSIP API base URL: `http://127.0.0.1:7474`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `main` (mcp)
- **mutates**: nothing

### `SERVER_NAME`
- **type**: variable (constant)
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: MCP server name string: `"hsip-mcp"`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `handle` (initialize response)
- **mutates**: nothing

### `SERVER_VERSION`
- **type**: variable (constant)
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: MCP server version string: `"0.2.0"`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `handle` (initialize response)
- **mutates**: nothing

### `PROTOCOL_VERSION`
- **type**: variable (constant)
- **file**: `crates/hsip-mcp/src/main.rs`
- **purpose**: MCP protocol version string: `"2024-11-05"`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `handle` (initialize response)
- **mutates**: nothing


---

## `crates/hsip-core/src/aad.rs`

### `AAD_HELLO`
- **type**: variable (constant `&[u8]`)
- **file**: `crates/hsip-core/src/aad.rs`
- **purpose**: Additional authenticated data tag for initial handshake hello frame.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: handshake code
- **mutates**: nothing

### `AAD_CONSENT`
- **type**: variable (constant `&[u8]`)
- **file**: `crates/hsip-core/src/aad.rs`
- **purpose**: AAD tag for consent protocol frames.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: consent protocol code
- **mutates**: nothing

### `AAD_DATA`
- **type**: variable (constant `&[u8]`)
- **file**: `crates/hsip-core/src/aad.rs`
- **purpose**: AAD tag for encrypted data payload frames.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: data channel code
- **mutates**: nothing

### `AAD_TICKET`
- **type**: variable (constant `&[u8]`)
- **file**: `crates/hsip-core/src/aad.rs`
- **purpose**: AAD tag for session ticket frames.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: session management code
- **mutates**: nothing

### `AAD_REKEY`
- **type**: variable (constant `&[u8]`)
- **file**: `crates/hsip-core/src/aad.rs`
- **purpose**: AAD tag for key rotation (re-key) frames.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: rekey protocol code
- **mutates**: nothing

### `AAD_STATUS`
- **type**: variable (constant `&[u8]`)
- **file**: `crates/hsip-core/src/aad.rs`
- **purpose**: AAD tag for status/ping frames.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: heartbeat/status code
- **mutates**: nothing

---

## `crates/hsip-core/src/nonce.rs`

### `NonceError`
- **type**: enum
- **file**: `crates/hsip-core/src/nonce.rs`
- **purpose**: Error variants for nonce validation: ZeroNonce, TooOld (outside 64-slot window), Replay (already seen).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `NonceWindow::check_and_update`
- **mutates**: nothing

### `NonceWindow`
- **type**: struct
- **file**: `crates/hsip-core/src/nonce.rs`
- **purpose**: Sliding 64-slot bitmap window for replay-attack prevention; tracks `max_seen` counter and a 64-bit bitmap.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: session code, `nonce_replay_selftest`
- **mutates**: nothing (updated via `check_and_update`)

### `NonceWindow::new`
- **type**: function (const)
- **file**: `crates/hsip-core/src/nonce.rs`
- **purpose**: Creates a zeroed `NonceWindow` (no nonces seen yet).
- **inputs**: none
- **outputs**: `Self`
- **calls**: none
- **called_by**: session initialisation
- **mutates**: nothing

### `NonceWindow::max_seen`
- **type**: function
- **file**: `crates/hsip-core/src/nonce.rs`
- **purpose**: Returns the highest nonce value seen so far.
- **inputs**: `&self`
- **outputs**: `u64`
- **calls**: none
- **called_by**: diagnostics
- **mutates**: nothing

### `NonceWindow::check_and_update`
- **type**: function
- **file**: `crates/hsip-core/src/nonce.rs`
- **purpose**: Validates a nonce against the window (rejects zero, too-old, and replays), then records it.
- **inputs**: `&mut self`, `nonce: u64`
- **outputs**: `Result<(), NonceError>`
- **calls**: bitmap operations
- **called_by**: decryption path
- **mutates**: `self.max_seen`, `self.bitmap`

---

## `crates/hsip-core/src/merkle.rs`

Pure RFC 6962 Merkle tree construction and inclusion proofs — no I/O. Leaf
hash `H(0x00||data)` and internal node hash `H(0x01||data)` use distinct
domain prefixes so a leaf hash can never be replayed as an internal node
hash (the second-preimage attack RFC 6962 prefixing prevents). Used by
`hsip-api`'s `anchor_job.rs` (building batches) and `routes/decisions.rs`
(building/verifying inclusion proofs).

### `Side` / `ProofStep`
- **type**: enum / struct
- **file**: `crates/hsip-core/src/merkle.rs`
- **purpose**: One step of an inclusion proof: a sibling hash and which side of the running accumulator it combines on when folding from leaf toward root.
- **called_by**: `MerkleTree::inclusion_proof`, `verify_inclusion`

### `leaf_hash` / `node_hash`
- **type**: function
- **file**: `crates/hsip-core/src/merkle.rs`
- **purpose**: `SHA256(0x00||data)` and `SHA256(0x01||left||right)` respectively — the RFC 6962 domain-separated hash primitives.
- **inputs**: `data: &[u8]` / `left: &[u8; 32], right: &[u8; 32]`
- **outputs**: `[u8; 32]`
- **called_by**: `MerkleTree::from_leaves`, `mth`, `audit_path`, `verify_inclusion`

### `MerkleTree`
- **type**: struct
- **file**: `crates/hsip-core/src/merkle.rs`
- **purpose**: A batch of leaf-hashed entries. `from_leaves` builds it (panics on an empty entry list — an anchor batch must contain at least one decision); `root()` computes the RFC 6962 `MTH`; `inclusion_proof(index)` computes the RFC 6962 `PATH`.
- **inputs**: `entries: &[T: AsRef<[u8]>]` (constructor)
- **outputs**: `[u8; 32]` (`root`), `Vec<ProofStep>` (`inclusion_proof`)
- **calls**: `leaf_hash`, `mth`, `audit_path`
- **called_by**: `hsip-api`'s `anchor_job::run_anchor_cycle_with_calendars`, `routes::decisions::proof`

### `verify_inclusion`
- **type**: function
- **file**: `crates/hsip-core/src/merkle.rs`
- **purpose**: Verifies that `leaf_data` at a claimed position is included under `root`, given an inclusion proof. The function a third party runs with zero knowledge of anything but the leaf's own data, its proof, and the published root — never touches a database.
- **inputs**: `leaf_data: &[u8]`, `proof: &[ProofStep]`, `root: &[u8; 32]`
- **outputs**: `bool`
- **called_by**: `hsip-api`'s `routes::decisions::verify`

---

## `crates/hsip-core/src/canonical.rs`

Canonical JSON encoding (RFC 8785 JCS, via the `serde_jcs` crate) and event
hashing for signed decision records. Deliberately not the alphabetical-
`BTreeMap` trick `hsip-api`'s `routes/credentials.rs::canonical_json` uses —
JCS is what the VeritasChain Protocol (VCP) mandates, and unlike the
`BTreeMap` shortcut it's correct for nested structures and exact number
formatting.

### `HSIP_GOV_EXT_VERSION`
- **type**: variable (const `&str`)
- **file**: `crates/hsip-core/src/canonical.rs`
- **purpose**: Version tag for HSIP's own draft of "which fields describe AI-agent decision accountability," since VCP-GOV is referenced by the VCP spec but has no published schema as of this writing. Lets the schema be reconciled later without silently pretending to be an official VCP module.
- **called_by**: `hsip-api`'s `routes::decisions::record`

### `DecisionEnvelope`
- **type**: struct
- **file**: `crates/hsip-core/src/canonical.rs`
- **purpose**: The signed envelope for one AI-agent decision attestation. Two-tier: `model_version`/`strategy_id`/`accountable_key`/`decision_type` are clear accountability metadata; `payload_hash` is an opaque SHA-256 of content HSIP never receives. `prev_hash` chains to the tenant's previous decision (empty string for the first). `timestamp_int` is kept as a string, not a JSON number, so canonicalization never risks IEEE-754-double precision loss on large timestamps.
- **called_by**: `hsip-api`'s `routes::decisions::{record, proof, verify}`

### `canonical_bytes` / `event_hash`
- **type**: function
- **file**: `crates/hsip-core/src/canonical.rs`
- **purpose**: `canonical_bytes` serializes a `DecisionEnvelope` per RFC 8785 JCS (deterministic across implementations). `event_hash` is `SHA256(JCS(envelope))` — the value that gets Ed25519-signed and fed into the Merkle tree as leaf data.
- **inputs**: `envelope: &DecisionEnvelope`
- **outputs**: `Result<Vec<u8>, serde_json::Error>` / `Result<[u8; 32], serde_json::Error>`
- **calls**: `serde_jcs::to_vec`, `sha2::Sha256::digest`
- **called_by**: `hsip-api`'s `routes::decisions::{record, proof, verify}`

---

## `dashboard/src/api.js`

### `BASE`
- **type**: variable (constant)
- **file**: `dashboard/src/api.js`
- **purpose**: Base URL for all API requests: empty string (relative) so Vite proxy routes `/v1/*` to localhost.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `request`
- **mutates**: nothing

### `request`
- **type**: function (async)
- **file**: `dashboard/src/api.js`
- **purpose**: Central fetch helper: prepends `BASE`, attaches `Authorization: Bearer <key>` from localStorage, returns parsed JSON or throws on error.
- **inputs**: `path: string`, `options?: RequestInit`
- **outputs**: `Promise<any>`
- **calls**: `fetch`, `JSON.parse`, `localStorage.getItem`
- **called_by**: every dashboard page component
- **mutates**: nothing

### `uploadImage`
- **type**: function (async)
- **file**: `dashboard/src/api.js`
- **purpose**: Uploads a `File` object to `POST /v1/uploads` using `FormData`; returns the response JSON.
- **inputs**: `file: File`
- **outputs**: `Promise<{url: string}>`
- **calls**: `fetch`, `FormData`
- **called_by**: `Messages` page
- **mutates**: server filesystem via API

---

## `dashboard/src/App.jsx`

Navigation is a **Simple/Expert mode toggle**, not progressive disclosure — this section previously documented an earlier progressive-disclosure refactor (`SIMPLE_TABS`/`EXPERT_TABS` described as "primary"/"Advanced behind a toggle", `showAdv`, `navigateTo`) that a later UI-redesign commit reintroduced the mode split on top of, without this file being updated. Rewritten here to match what's actually in the file — see "Dashboard" in `CLAUDE.md` for the same correction and why.

### `App`
- **type**: function (React component)
- **file**: `dashboard/src/App.jsx`
- **purpose**: Root application component: login screen (with the Simple/Expert mode toggle), sidebar nav, and renders the active tab's page component for whichever mode is active.
- **inputs**: none
- **outputs**: JSX
- **calls**: `handleLogin`, `handleGetTrialKey`, `logout`, `switchMode`
- **called_by**: `main.jsx` (React root)
- **mutates**: `localStorage` (`hsip_api_key`, `hsip_mode`, `hsip_onboarding_done`)

### `SIMPLE_TABS`
- **type**: variable (constant array)
- **file**: `dashboard/src/App.jsx`
- **purpose**: The 10 "For Everyone" (consumer-facing) nav tabs: Home, Finance, Messages, Traffic, Alibi, Consents, AI Watch, AI Decisions, Trackers, Protection.
- **called_by**: `App` (when `mode === 'simple'`)

### `EXPERT_TABS`
- **type**: variable (constant array)
- **file**: `dashboard/src/App.jsx`
- **purpose**: The 10 "Developer" nav tabs: Identity, Consent, Messages, Credentials, Decisions, Trust, Discover, Audit, Keys, Admin. Trust/Discover/Admin added to close dashboard UI gaps — see `CLAUDE.md`.
- **called_by**: `App` (when `mode === 'expert'`)

### `switchMode`
- **type**: function
- **file**: `dashboard/src/App.jsx`
- **purpose**: Switches between `'simple'`/`'expert'`, persists the choice to `localStorage('hsip_mode')`, and resets the active tab to that mode's default landing tab (`home` / `identity`).
- **inputs**: `m: 'simple' | 'expert'`
- **outputs**: none
- **calls**: `setMode`, `localStorage.setItem`, `setTab`
- **called_by**: login-screen mode buttons, sidebar footer "Dev Mode"/"Simple Mode" button
- **mutates**: `localStorage`, React state (`mode`, `tab`)

### `handleLogin`
- **type**: function (async)
- **file**: `dashboard/src/App.jsx`
- **purpose**: Validates the entered key via `POST /v1/identity` (auto-creates an identity if none exists yet), stores it in `localStorage` on success, and shows onboarding on first Simple-mode login.
- **inputs**: form submit event
- **outputs**: none
- **calls**: `request`, `localStorage.setItem`
- **called_by**: login form submit
- **mutates**: `localStorage`, React state

### `handleGetTrialKey`
- **type**: function (async)
- **file**: `dashboard/src/App.jsx`
- **purpose**: Calls `POST /v1/sandbox/provision` for a one-click 24-hour trial key, then signs in with it automatically.
- **inputs**: none
- **outputs**: none
- **calls**: `fetch`, `request`, `localStorage.setItem`
- **called_by**: "Try it free" button on the login screen
- **mutates**: `localStorage`, React state

### `logout`
- **type**: function
- **file**: `dashboard/src/App.jsx`
- **purpose**: Clears the API key from `localStorage` and resets the app to the login screen.
- **inputs**: none
- **outputs**: none
- **calls**: `localStorage.removeItem`, `setAuthed`
- **called_by**: sidebar "Sign out" button
- **mutates**: `localStorage`, React state


---

## `dashboard/src/pages/HomeDashboard.jsx`

### `HomeDashboard`
- **type**: function (React component)
- **file**: `dashboard/src/pages/HomeDashboard.jsx`
- **purpose**: Main home page: shows live threat score, wall of shame trackers, HSIP quick actions, and what-is panel.
- **inputs**: none (reads localStorage for API key)
- **outputs**: JSX
- **calls**: `request`, `CreepMeter`, `WallOfShame`
- **called_by**: `App`
- **mutates**: nothing

### `WALL_OF_SHAME`
- **type**: variable (constant array)
- **file**: `dashboard/src/pages/HomeDashboard.jsx`
- **purpose**: Hardcoded list of well-known tracker companies shown in the "Wall of Shame" panel.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `WallOfShame`
- **mutates**: nothing

### `RISK_SCORE`
- **type**: variable (constant object)
- **file**: `dashboard/src/pages/HomeDashboard.jsx`
- **purpose**: Risk score thresholds and labels (low/medium/high/critical) for the CreepMeter.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `scoreTracker`, `creepTier`
- **mutates**: nothing

### `scoreTracker`
- **type**: function
- **file**: `dashboard/src/pages/HomeDashboard.jsx`
- **purpose**: Calculates numeric risk score from blocked tracker count using `RISK_SCORE` thresholds.
- **inputs**: `count: number`
- **outputs**: `number`
- **calls**: none
- **called_by**: `HomeDashboard`
- **mutates**: nothing

### `creepTier`
- **type**: function
- **file**: `dashboard/src/pages/HomeDashboard.jsx`
- **purpose**: Returns the tier label (low/medium/high/critical) for a given score.
- **inputs**: `score: number`
- **outputs**: `string`
- **calls**: none
- **called_by**: `CreepMeter`
- **mutates**: nothing

### `lookupDomain`
- **type**: function (async)
- **file**: `dashboard/src/pages/HomeDashboard.jsx`
- **purpose**: Checks if a domain is in the HSIP tracker blocklist via the proxy log API.
- **inputs**: `domain: string`
- **outputs**: `Promise<boolean>`
- **calls**: `request`
- **called_by**: `HomeDashboard`
- **mutates**: nothing

### `CreepMeter`
- **type**: function (React component)
- **file**: `dashboard/src/pages/HomeDashboard.jsx`
- **purpose**: Visual gauge showing the current privacy threat score as an arc meter.
- **inputs**: `score: number`, `tier: string`
- **outputs**: JSX
- **calls**: none
- **called_by**: `HomeDashboard`
- **mutates**: nothing

### `WallOfShame`
- **type**: function (React component)
- **file**: `dashboard/src/pages/HomeDashboard.jsx`
- **purpose**: Renders the list of worst offender tracker companies with blocked counts.
- **inputs**: `trackers: array`
- **outputs**: JSX
- **calls**: none
- **called_by**: `HomeDashboard`
- **mutates**: nothing

### `ACTIONS`
- **type**: variable (constant array)
- **file**: `dashboard/src/pages/HomeDashboard.jsx`
- **purpose**: Quick action button definitions for the Home page action grid.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `HomeDashboard`
- **mutates**: nothing

### `WHAT_IS`
- **type**: variable (constant array)
- **file**: `dashboard/src/pages/HomeDashboard.jsx`
- **purpose**: "What is HSIP?" explainer bullet points shown on the home page.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `HomeDashboard`
- **mutates**: nothing

---

## `dashboard/src/pages/FinanceDashboard.jsx`

### `FinanceDashboard`
- **type**: function (React component)
- **file**: `dashboard/src/pages/FinanceDashboard.jsx`
- **purpose**: Finance/market intelligence page: shows market gap analysis, use cases, regulatory landscape, and live sign demo.
- **inputs**: none
- **outputs**: JSX
- **calls**: `request`, `GapCard`, `UseCaseCard`, `ComparisonTable`, `LiveSignDemo`
- **called_by**: `App`
- **mutates**: nothing

### `MARKET_GAPS`
- **type**: variable (constant array)
- **file**: `dashboard/src/pages/FinanceDashboard.jsx`
- **purpose**: List of fintech market gaps that HSIP addresses.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `FinanceDashboard`
- **mutates**: nothing

### `USE_CASES`
- **type**: variable (constant array)
- **file**: `dashboard/src/pages/FinanceDashboard.jsx`
- **purpose**: HSIP financial use case scenarios with descriptions.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `FinanceDashboard`
- **mutates**: nothing

### `REGULATIONS`
- **type**: variable (constant array)
- **file**: `dashboard/src/pages/FinanceDashboard.jsx`
- **purpose**: Financial regulations list (GDPR, CCPA, PSD2, SOX) with compliance notes.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `FinanceDashboard`
- **mutates**: nothing

### `COMPARISON_ROWS`
- **type**: variable (constant array)
- **file**: `dashboard/src/pages/FinanceDashboard.jsx`
- **purpose**: Feature comparison rows: HSIP vs centralized identity providers.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `ComparisonTable`
- **mutates**: nothing

### `GapCard`
- **type**: function (React component)
- **file**: `dashboard/src/pages/FinanceDashboard.jsx`
- **purpose**: Renders one market gap card with title, description, and HSIP solution.
- **inputs**: `gap: object`
- **outputs**: JSX
- **calls**: none
- **called_by**: `FinanceDashboard`
- **mutates**: nothing

### `UseCaseCard`
- **type**: function (React component)
- **file**: `dashboard/src/pages/FinanceDashboard.jsx`
- **purpose**: Renders one use case card with icon, title, and description.
- **inputs**: `uc: object`
- **outputs**: JSX
- **calls**: none
- **called_by**: `FinanceDashboard`
- **mutates**: nothing

### `RegBadge`
- **type**: function (React component)
- **file**: `dashboard/src/pages/FinanceDashboard.jsx`
- **purpose**: Renders a regulatory badge pill with regulation name.
- **inputs**: `reg: object`
- **outputs**: JSX
- **calls**: none
- **called_by**: `FinanceDashboard`
- **mutates**: nothing

### `CompCell`
- **type**: function (React component)
- **file**: `dashboard/src/pages/FinanceDashboard.jsx`
- **purpose**: Renders one comparison table cell with checkmark or cross.
- **inputs**: `value: boolean | string`
- **outputs**: JSX
- **calls**: none
- **called_by**: `ComparisonTable`
- **mutates**: nothing

### `ComparisonTable`
- **type**: function (React component)
- **file**: `dashboard/src/pages/FinanceDashboard.jsx`
- **purpose**: Renders the full HSIP vs alternatives feature comparison table.
- **inputs**: none
- **outputs**: JSX
- **calls**: `CompCell`
- **called_by**: `FinanceDashboard`
- **mutates**: nothing

### `DEMO_SCENARIOS`
- **type**: variable (constant array)
- **file**: `dashboard/src/pages/FinanceDashboard.jsx`
- **purpose**: Live demo scenario definitions with sample messages to sign.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `LiveSignDemo`
- **mutates**: nothing

### `LiveSignDemo`
- **type**: function (React component)
- **file**: `dashboard/src/pages/FinanceDashboard.jsx`
- **purpose**: Interactive demo: signs a message via `POST /v1/messages/sign` and displays the Ed25519 signature.
- **inputs**: none
- **outputs**: JSX
- **calls**: `request`
- **called_by**: `FinanceDashboard`
- **mutates**: DB via API (creates signed message record)


---

## `dashboard/src/pages/AIWatch.jsx`

### `formatActivity`
- **type**: function
- **file**: `dashboard/src/pages/AIWatch.jsx`
- **purpose**: Formats an audit entry object into a human-readable activity string for display.
- **inputs**: `entry: object`
- **outputs**: `string`
- **calls**: none
- **called_by**: `AgentCard`
- **mutates**: nothing

### `CopyBox`
- **type**: function (React component)
- **file**: `dashboard/src/pages/AIWatch.jsx`
- **purpose**: Renders a read-only code box with a copy-to-clipboard button.
- **inputs**: `text: string`
- **outputs**: JSX
- **calls**: `navigator.clipboard.writeText`
- **called_by**: `PlatformGuides`, `ConnectDialog`
- **mutates**: clipboard

### `PlatformGuides`
- **type**: function (React component)
- **file**: `dashboard/src/pages/AIWatch.jsx`
- **purpose**: Renders platform-specific setup instructions (Claude Desktop, Cursor, VS Code) for hsip-mcp.
- **inputs**: `apiKey: string`
- **outputs**: JSX
- **calls**: `CopyBox`
- **called_by**: `ConnectDialog`
- **mutates**: nothing

### `ConnectDialog`
- **type**: function (React component)
- **file**: `dashboard/src/pages/AIWatch.jsx`
- **purpose**: Modal dialog showing MCP connection setup instructions for the user's AI tools.
- **inputs**: `onClose: function`, `apiKey: string`
- **outputs**: JSX
- **calls**: `PlatformGuides`
- **called_by**: `AIWatch`
- **mutates**: nothing

### `AgentCard`
- **type**: function (React component)
- **file**: `dashboard/src/pages/AIWatch.jsx`
- **purpose**: Renders one AI agent card: name, status, request rate, anomaly count, recent activity.
- **inputs**: `agent: object`, `activity: array`
- **outputs**: JSX
- **calls**: `formatActivity`
- **called_by**: `AIWatch`
- **mutates**: nothing

### `AIWatch`
- **type**: function (React component)
- **file**: `dashboard/src/pages/AIWatch.jsx`
- **purpose**: AI agent monitoring page: lists registered agents with live stats, shows connect dialog.
- **inputs**: none
- **outputs**: JSX
- **calls**: `request`, `AgentCard`, `ConnectDialog`
- **called_by**: `App`
- **mutates**: nothing

---

## `dashboard/src/pages/TrackerInspector.jsx`

### `RiskBadge`
- **type**: function (React component)
- **file**: `dashboard/src/pages/TrackerInspector.jsx`
- **purpose**: Renders a color-coded risk level badge (low/medium/high/critical).
- **inputs**: `level: string`
- **outputs**: JSX
- **calls**: none
- **called_by**: `TrackerCard`, `LookupResult`
- **mutates**: nothing

### `TrackerCard`
- **type**: function (React component)
- **file**: `dashboard/src/pages/TrackerInspector.jsx`
- **purpose**: Renders one tracker domain card with name, category, blocked count, and risk badge.
- **inputs**: `tracker: object`
- **outputs**: JSX
- **calls**: `RiskBadge`
- **called_by**: `TrackerInspector`
- **mutates**: nothing

### `LookupResult`
- **type**: function (React component)
- **file**: `dashboard/src/pages/TrackerInspector.jsx`
- **purpose**: Renders the result panel after a domain lookup: blocked/allowed with risk badge.
- **inputs**: `result: object`
- **outputs**: JSX
- **calls**: `RiskBadge`
- **called_by**: `TrackerInspector`
- **mutates**: nothing

### `TrackerInspector`
- **type**: function (React component)
- **file**: `dashboard/src/pages/TrackerInspector.jsx`
- **purpose**: Tracker inspection page: domain lookup, top blocked trackers list from proxy log.
- **inputs**: none
- **outputs**: JSX
- **calls**: `request`, `TrackerCard`, `LookupResult`
- **called_by**: `App`
- **mutates**: nothing

---

## `dashboard/src/pages/ProtectionSetup.jsx`

### `BLOCKABLE`
- **type**: variable (constant array)
- **file**: `dashboard/src/pages/ProtectionSetup.jsx`
- **purpose**: List of tracker domains that can be blocked, with risk levels and categories.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `ProtectionSetup`
- **mutates**: nothing

### `CRITICAL_COUNT`
- **type**: variable (constant)
- **file**: `dashboard/src/pages/ProtectionSetup.jsx`
- **purpose**: Number of critical-risk trackers in the `BLOCKABLE` list.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `ProtectionSetup`
- **mutates**: nothing

### `HIGH_COUNT`
- **type**: variable (constant)
- **file**: `dashboard/src/pages/ProtectionSetup.jsx`
- **purpose**: Number of high-risk trackers in the `BLOCKABLE` list.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `ProtectionSetup`
- **mutates**: nothing

### `buildHostsContent`
- **type**: function
- **file**: `dashboard/src/pages/ProtectionSetup.jsx`
- **purpose**: Generates `/etc/hosts` file content to block all trackers in `BLOCKABLE`.
- **inputs**: none
- **outputs**: `string`
- **calls**: none
- **called_by**: `downloadHosts`, `ProtectionSetup`
- **mutates**: nothing

### `downloadHosts`
- **type**: function
- **file**: `dashboard/src/pages/ProtectionSetup.jsx`
- **purpose**: Triggers a browser download of the generated hosts file.
- **inputs**: none
- **outputs**: none
- **calls**: `buildHostsContent`, `URL.createObjectURL`
- **called_by**: download button in `ProtectionSetup`
- **mutates**: nothing (browser download only)

### `DNS_PORT`
- **type**: variable (constant)
- **file**: `dashboard/src/pages/ProtectionSetup.jsx`
- **purpose**: Default DNS blocker port: `5300`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `DnsSection`
- **mutates**: nothing

### `DNS_OS_STEPS`
- **type**: variable (constant object)
- **file**: `dashboard/src/pages/ProtectionSetup.jsx`
- **purpose**: OS-specific steps (Windows, macOS, Linux) for configuring system DNS to use the HSIP blocker.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `DnsSection`
- **mutates**: nothing

### `DnsSection`
- **type**: function (React component)
- **file**: `dashboard/src/pages/ProtectionSetup.jsx`
- **purpose**: DNS blocker control panel: start/stop DNS server, show OS-specific configuration steps.
- **inputs**: none
- **outputs**: JSX
- **calls**: `request`
- **called_by**: `ProtectionSetup`
- **mutates**: DNS server state via API

### `DnsStepCard`
- **type**: function (React component)
- **file**: `dashboard/src/pages/ProtectionSetup.jsx`
- **purpose**: Renders one OS-specific DNS setup step card.
- **inputs**: `step: object`
- **outputs**: JSX
- **calls**: none
- **called_by**: `DnsSection`
- **mutates**: nothing

### `OS_STEPS`
- **type**: variable (constant object)
- **file**: `dashboard/src/pages/ProtectionSetup.jsx`
- **purpose**: OS-specific proxy configuration steps for Windows, macOS, Linux.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `ProtectionSetup`
- **mutates**: nothing

### `StepCard`
- **type**: function (React component)
- **file**: `dashboard/src/pages/ProtectionSetup.jsx`
- **purpose**: Renders one numbered proxy setup step card with instructions.
- **inputs**: `step: object`, `index: number`
- **outputs**: JSX
- **calls**: none
- **called_by**: `ProtectionSetup`
- **mutates**: nothing

### `ProtectionSetup`
- **type**: function (React component)
- **file**: `dashboard/src/pages/ProtectionSetup.jsx`
- **purpose**: Full protection setup page: hosts file download, DNS blocker, proxy setup steps.
- **inputs**: none
- **outputs**: JSX
- **calls**: `DnsSection`, `downloadHosts`, `StepCard`
- **called_by**: `App`
- **mutates**: nothing

---

## `dashboard/src/pages/NetworkMonitor.jsx`

### `CAT_COLOR`
- **type**: variable (constant object)
- **file**: `dashboard/src/pages/NetworkMonitor.jsx`
- **purpose**: Maps tracker category names to display colors for the event log.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `catStyle`, `EventRow`
- **mutates**: nothing

### `catStyle`
- **type**: function
- **file**: `dashboard/src/pages/NetworkMonitor.jsx`
- **purpose**: Returns a CSS style object for a tracker category color.
- **inputs**: `cat: string`
- **outputs**: `object`
- **calls**: `CAT_COLOR`
- **called_by**: `EventRow`
- **mutates**: nothing

### `SetupWizard`
- **type**: function (React component)
- **file**: `dashboard/src/pages/NetworkMonitor.jsx`
- **purpose**: Proxy setup wizard shown when proxy is not running; guides user through enabling it.
- **inputs**: `onEnable: function`
- **outputs**: JSX
- **calls**: none
- **called_by**: `NetworkMonitor`
- **mutates**: nothing

### `StatsBar`
- **type**: function (React component)
- **file**: `dashboard/src/pages/NetworkMonitor.jsx`
- **purpose**: Renders proxy stats summary bar: total requests, blocked count, block rate percentage.
- **inputs**: `stats: object`
- **outputs**: JSX
- **calls**: none
- **called_by**: `NetworkMonitor`
- **mutates**: nothing

### `EventRow`
- **type**: function (React component)
- **file**: `dashboard/src/pages/NetworkMonitor.jsx`
- **purpose**: Renders one proxy event row in the traffic log table.
- **inputs**: `event: object`
- **outputs**: JSX
- **calls**: `catStyle`
- **called_by**: `NetworkMonitor`
- **mutates**: nothing

### `TopBlocked`
- **type**: function (React component)
- **file**: `dashboard/src/pages/NetworkMonitor.jsx`
- **purpose**: Shows top-5 blocked domains with counts.
- **inputs**: `domains: array`
- **outputs**: JSX
- **calls**: none
- **called_by**: `NetworkMonitor`
- **mutates**: nothing

### `NetworkMonitor`
- **type**: function (React component)
- **file**: `dashboard/src/pages/NetworkMonitor.jsx`
- **purpose**: Traffic monitor page: polls proxy log every 2s, shows stats and live event feed.
- **inputs**: none
- **outputs**: JSX
- **calls**: `request`, `SetupWizard`, `StatsBar`, `EventRow`, `TopBlocked`
- **called_by**: `App`
- **mutates**: proxy state via API (enable/disable)


---

## `dashboard/src/pages/Messages.jsx`

### `fmtTime`
- **type**: function
- **file**: `dashboard/src/pages/Messages.jsx`
- **purpose**: Formats a Unix ms timestamp as short time string (HH:MM).
- **inputs**: `ms: number`
- **outputs**: `string`
- **calls**: `Date`
- **called_by**: `Bubble`
- **mutates**: nothing

### `fmtFull`
- **type**: function
- **file**: `dashboard/src/pages/Messages.jsx`
- **purpose**: Formats a Unix ms timestamp as full date+time string.
- **inputs**: `ms: number`
- **outputs**: `string`
- **calls**: `Date`
- **called_by**: `Bubble` (tooltip)
- **mutates**: nothing

### `keyFp`
- **type**: function
- **file**: `dashboard/src/pages/Messages.jsx`
- **purpose**: Returns a short key fingerprint: first 8 chars of base64 verify key.
- **inputs**: `key: string`
- **outputs**: `string`
- **calls**: none
- **called_by**: `Bubble`, `MyAddress`
- **mutates**: nothing

### `makeShareText`
- **type**: function
- **file**: `dashboard/src/pages/Messages.jsx`
- **purpose**: Generates a shareable text block with a signed message's content, signature, and verify key.
- **inputs**: `msg: object`
- **outputs**: `string`
- **calls**: none
- **called_by**: `Bubble`
- **mutates**: nothing

### `parseProof`
- **type**: function
- **file**: `dashboard/src/pages/Messages.jsx`
- **purpose**: Parses pasted "Alibi proof" text into content/signature/key components.
- **inputs**: `text: string`
- **outputs**: `object | null`
- **calls**: string parsing
- **called_by**: `ReceiveDialog`
- **mutates**: nothing

### `AddContactDialog`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Messages.jsx`
- **purpose**: Modal dialog for adding a new contact by nickname and verify key.
- **inputs**: `onClose: function`, `onAdd: function`
- **outputs**: JSX
- **calls**: `request`
- **called_by**: `Messages`
- **mutates**: DB via API (contacts)

### `ReceiveDialog`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Messages.jsx`
- **purpose**: Modal dialog for pasting and verifying an "Alibi proof" from a contact.
- **inputs**: `onClose: function`
- **outputs**: JSX
- **calls**: `parseProof`, `request`
- **called_by**: `Messages`
- **mutates**: nothing

### `isImageUrl`
- **type**: function
- **file**: `dashboard/src/pages/Messages.jsx`
- **purpose**: Returns true if a string ends with a common image extension.
- **inputs**: `url: string`
- **outputs**: `boolean`
- **calls**: `String.endsWith`
- **called_by**: `Bubble`
- **mutates**: nothing

### `Bubble`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Messages.jsx`
- **purpose**: Renders one chat bubble with message content, timestamp, signature, share button.
- **inputs**: `msg: object`, `isMine: boolean`
- **outputs**: JSX
- **calls**: `fmtTime`, `fmtFull`, `keyFp`, `makeShareText`, `isImageUrl`
- **called_by**: `Thread`
- **mutates**: clipboard (on share)

### `Thread`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Messages.jsx`
- **purpose**: Renders the message thread for a selected contact.
- **inputs**: `contact: object`, `messages: array`, `onSend: function`
- **outputs**: JSX
- **calls**: `Bubble`
- **called_by**: `Messages`
- **mutates**: nothing

### `MyAddress`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Messages.jsx`
- **purpose**: Shows the user's own HSIP address (verify key fingerprint) for sharing.
- **inputs**: `verifyKey: string`
- **outputs**: JSX
- **calls**: `keyFp`, `navigator.clipboard.writeText`
- **called_by**: `Messages`
- **mutates**: clipboard

### `Messages`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Messages.jsx`
- **purpose**: Secure messaging page: contact list, message thread, send/receive signed messages.
- **inputs**: none
- **outputs**: JSX
- **calls**: `request`, `Thread`, `MyAddress`, `AddContactDialog`, `ReceiveDialog`
- **called_by**: `App`
- **mutates**: DB via API (messages, contacts)

---

## `dashboard/src/pages/Consent.jsx`

### `Consent`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Consent.jsx`
- **purpose**: Advanced consent management page: grant, revoke, list consents by peer verify key.
- **inputs**: none
- **outputs**: JSX
- **calls**: `request`
- **called_by**: `App`
- **mutates**: DB via API (consents)

---

## `dashboard/src/pages/ConsentWallet.jsx`

### `DURATION_OPTIONS`
- **type**: variable (constant array)
- **file**: `dashboard/src/pages/ConsentWallet.jsx`
- **purpose**: Preset consent duration options (1 hour, 24 hours, 7 days, 30 days, permanent).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `ConsentWallet`
- **mutates**: nothing

### `timeUntil`
- **type**: function
- **file**: `dashboard/src/pages/ConsentWallet.jsx`
- **purpose**: Returns a human-readable "expires in X" string from an expiry timestamp.
- **inputs**: `ms: number`
- **outputs**: `string`
- **calls**: `Date`
- **called_by**: `ConsentWallet`
- **mutates**: nothing

### `ConsentWallet`
- **type**: function (React component)
- **file**: `dashboard/src/pages/ConsentWallet.jsx`
- **purpose**: Consent wallet (Consents tab): shows all active consents with expiry, allows granting new ones with duration picker.
- **inputs**: none
- **outputs**: JSX
- **calls**: `request`, `timeUntil`
- **called_by**: `App`
- **mutates**: DB via API (consents)

---

## `dashboard/src/pages/Credentials.jsx`

### `Credentials`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Credentials.jsx`
- **purpose**: Verifiable credentials page: issue new credentials, list existing, verify or revoke.
- **inputs**: none
- **outputs**: JSX
- **calls**: `request`
- **called_by**: `App`
- **mutates**: DB via API (credentials)

---

## `dashboard/src/pages/Identity.jsx`

### `Identity`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Identity.jsx`
- **purpose**: Identity management page: shows Ed25519 verify key, allows key rotation.
- **inputs**: none
- **outputs**: JSX
- **calls**: `request`
- **called_by**: `App`
- **mutates**: DB via API (identity)

---

## `dashboard/src/pages/Keys.jsx`

### `Keys`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Keys.jsx`
- **purpose**: API key management page: create, list, and revoke API keys for human/service/ai_agent types. Also shows/sets each key's tenant `role` ('owner'/'member') — previously invisible in the dashboard, curl/CLI-only.
- **inputs**: none
- **outputs**: JSX
- **calls**: `request` (`GET/POST/DELETE /v1/keys*`)
- **called_by**: `App`
- **mutates**: DB via API (`api_keys`)

---

## `dashboard/src/pages/Trust.jsx`

### `Trust`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Trust.jsx`
- **purpose**: Federated trust management: add/list/remove a trusted peer's Ed25519 verify key by label (`POST /v1/trust/peer`, `GET /v1/trust/peers`, `DELETE /v1/trust/peers/:id`), plus a "verify a signature from a trusted peer" tool (`POST /v1/trust/verify`) showing valid/invalid without needing to re-paste the raw key. New — closes a dashboard UI gap; building this surfaced that `trusted_peers` was never created by `db::run_migrations` (see `db.rs`'s entry and CLAUDE.md's "Dashboard" section).
- **inputs**: `apiKey: string`
- **outputs**: JSX
- **calls**: `request`
- **called_by**: `App` (Expert mode, `trust` tab)
- **mutates**: DB via API (`trusted_peers`), writes `trust.*` audit entries (server-side)

---

## `dashboard/src/pages/Discover.jsx`

### `Discover`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Discover.jsx`
- **purpose**: Scans well-known localhost ports for running AI agents/MCP servers (`GET /v1/agents/discover`) and offers a one-click "Register key" button per unregistered result (`POST /v1/keys` with `agent_type: "ai_agent"`, `name` from the probe's `suggested_name`). New — closes a dashboard UI gap; previously this data was only reachable via `hsip agent discover`.
- **inputs**: `apiKey: string`
- **outputs**: JSX
- **calls**: `request`
- **called_by**: `App` (Expert mode, `discover` tab)
- **mutates**: DB via API (`api_keys`, when registering)

---

## `dashboard/src/pages/Admin.jsx`

Node-level administration: master key fingerprint/rotation and root-admin list/grant/revoke. New — closes a dashboard UI gap; both operations were previously curl/CLI-only (`hsip keys master-fingerprint`/`rotate-master`, `hsip keys list-root-admins`/`grant-root-admin`/`revoke-root-admin`). Both sub-components independently surface "your key isn't a root admin" rather than a raw error when `require_root_admin` rejects the signed-in key, since every endpoint here is root-admin-gated.

### `MasterKeyCard`
- **type**: function (React component, internal to `Admin.jsx`)
- **file**: `dashboard/src/pages/Admin.jsx`
- **purpose**: Shows the running master key's fingerprint/source/rotation-availability (`GET /v1/admin/master-key/fingerprint`) and a rotate button gated behind an inline confirm step (`POST /v1/admin/master-key/rotate`) — mirrors the CLI's interactive `yes` confirmation.
- **inputs**: `apiKey: string`
- **outputs**: JSX
- **calls**: `request`
- **called_by**: `Admin`
- **mutates**: DB via API (re-encrypts every tenant's `identities` row on rotate), the running process's in-memory master key

### `RootAdminsCard`
- **type**: function (React component, internal to `Admin.jsx`)
- **file**: `dashboard/src/pages/Admin.jsx`
- **purpose**: Lists active root-admin keys (`GET /v1/admin/root-admins`), grants root-admin to a key by ID (`POST /v1/admin/root-admins/grant`), and revokes it (`POST /v1/admin/root-admins/revoke`) — the Revoke button is disabled client-side when only one root admin remains, mirroring the server's last-root-admin lockout guard (the server remains the source of truth for this check).
- **inputs**: `apiKey: string`
- **outputs**: JSX
- **calls**: `request`
- **called_by**: `Admin`
- **mutates**: DB via API (`api_keys.is_root_admin`)

### `Admin`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Admin.jsx`
- **purpose**: Composes `MasterKeyCard` + `RootAdminsCard`.
- **inputs**: `apiKey: string`
- **outputs**: JSX
- **called_by**: `App` (Expert mode, `admin` tab)

---

## `dashboard/src/pages/Audit.jsx`

### `Audit`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Audit.jsx`
- **purpose**: Audit log viewer: filterable list of audit entries for the tenant, plus a hash-chain-intact/broken indicator (`GET /v1/audit/verify` — `valid`/`checked`/`unchained`/`first_break_id`) with a manual re-check button. The indicator is new — closes a dashboard UI gap; previously a broken chain was only visible by calling the API directly.
- **inputs**: `apiKey: string`
- **outputs**: JSX
- **calls**: `request` (`GET /v1/audit`, `GET /v1/audit/verify`)
- **called_by**: `App`
- **mutates**: nothing

---

## `dashboard/src/pages/ProveIt.jsx`

### `ProveIt`
- **type**: function (React component)
- **file**: `dashboard/src/pages/ProveIt.jsx`
- **purpose**: "Alibi" page: signs a timestamped statement and generates shareable proof text.
- **inputs**: none
- **outputs**: JSX
- **calls**: `request`
- **called_by**: `App`
- **mutates**: DB via API (signed message)

---

## `dashboard/src/pages/Onboarding.jsx`

### `STEPS`
- **type**: variable (constant array)
- **file**: `dashboard/src/pages/Onboarding.jsx`
- **purpose**: Onboarding wizard step definitions: Welcome, Stores, Cannot, Local, Consent.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `Onboarding`
- **mutates**: nothing

### `WelcomeStep`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Onboarding.jsx`
- **purpose**: Onboarding step 1: HSIP welcome screen with product overview.
- **inputs**: `onNext: function`
- **outputs**: JSX
- **calls**: none
- **called_by**: `Onboarding`
- **mutates**: nothing

### `StoresStep`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Onboarding.jsx`
- **purpose**: Onboarding step 2: explains what data HSIP stores locally.
- **inputs**: `onNext: function`, `onBack: function`
- **outputs**: JSX
- **calls**: none
- **called_by**: `Onboarding`
- **mutates**: nothing

### `CannotStep`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Onboarding.jsx`
- **purpose**: Onboarding step 3: explains what HSIP cannot do (no cloud sync, no backup).
- **inputs**: `onNext: function`, `onBack: function`
- **outputs**: JSX
- **calls**: none
- **called_by**: `Onboarding`
- **mutates**: nothing

### `LocalStep`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Onboarding.jsx`
- **purpose**: Onboarding step 4: explains local-first architecture and data sovereignty.
- **inputs**: `onNext: function`, `onBack: function`
- **outputs**: JSX
- **calls**: none
- **called_by**: `Onboarding`
- **mutates**: nothing

### `ConsentStep`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Onboarding.jsx`
- **purpose**: Onboarding step 5: explains consent protocol; completion sets `hsip_onboarding_done` in localStorage.
- **inputs**: `onDone: function`, `onBack: function`
- **outputs**: JSX
- **calls**: `localStorage.setItem`
- **called_by**: `Onboarding`
- **mutates**: `localStorage`

### `Onboarding`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Onboarding.jsx`
- **purpose**: Multi-step onboarding wizard shown on first launch; manages step navigation.
- **inputs**: `onComplete: function`
- **outputs**: JSX
- **calls**: `WelcomeStep`, `StoresStep`, `CannotStep`, `LocalStep`, `ConsentStep`
- **called_by**: `App`
- **mutates**: `localStorage` (via `ConsentStep`)


---

## `browser-extension/background.js`

### `HSIP_API`
- **type**: variable (constant)
- **file**: `browser-extension/background.js`
- **purpose**: Base URL for HSIP API requests from the extension: `http://127.0.0.1:7474`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `checkHsipConnection`, `fetchAgentActivity`
- **mutates**: nothing

### `tabStats`
- **type**: variable
- **file**: `browser-extension/background.js`
- **purpose**: In-memory map of tab ID → blocked tracker count for the current session.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: tab listeners, `loadStats`
- **mutates**: nothing

### `getApiKey`
- **type**: function (async)
- **file**: `browser-extension/background.js`
- **purpose**: Reads the HSIP API key from `chrome.storage.local`.
- **inputs**: none
- **outputs**: `Promise<string | undefined>`
- **calls**: `chrome.storage.local.get`
- **called_by**: `checkHsipConnection`, `fetchAgentActivity`
- **mutates**: nothing

### `checkHsipConnection`
- **type**: function (async)
- **file**: `browser-extension/background.js`
- **purpose**: Polls `GET /health` every 30 seconds; stores connection status in `chrome.storage.local`.
- **inputs**: none
- **outputs**: none
- **calls**: `fetch`, `getApiKey`, `chrome.storage.local.set`
- **called_by**: service worker alarm
- **mutates**: `chrome.storage.local` (connection status)

### `fetchAgentActivity`
- **type**: function (async)
- **file**: `browser-extension/background.js`
- **purpose**: Fetches the last 5 AI agent audit entries from `GET /v1/audit` and caches them in `chrome.storage.local`.
- **inputs**: none
- **outputs**: none
- **calls**: `fetch`, `getApiKey`, `chrome.storage.local.set`
- **called_by**: service worker alarm
- **mutates**: `chrome.storage.local` (agent activity cache)

---

## `browser-extension/popup.js`

### `HSIP_DASHBOARD`
- **type**: variable (constant)
- **file**: `browser-extension/popup.js`
- **purpose**: URL to open when the user clicks "Open Dashboard": `http://127.0.0.1:7474`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: dashboard button handler
- **mutates**: nothing

### `getCurrentTabId`
- **type**: function (async)
- **file**: `browser-extension/popup.js`
- **purpose**: Returns the currently active browser tab's ID.
- **inputs**: none
- **outputs**: `Promise<number>`
- **calls**: `chrome.tabs.query`
- **called_by**: `loadStats`
- **mutates**: nothing

### `loadStats`
- **type**: function (async)
- **file**: `browser-extension/popup.js`
- **purpose**: Loads tracker stats for the current tab from `tabStats` and renders the popup.
- **inputs**: none
- **outputs**: none
- **calls**: `getCurrentTabId`, `chrome.storage.local.get`, `renderCount`, `renderDomains`
- **called_by**: popup `DOMContentLoaded`
- **mutates**: DOM

### `getHsipStatus`
- **type**: function (async)
- **file**: `browser-extension/popup.js`
- **purpose**: Reads HSIP connection status from `chrome.storage.local` and renders status indicator.
- **inputs**: none
- **outputs**: none
- **calls**: `chrome.storage.local.get`, `renderHsipStatus`, `renderActivity`
- **called_by**: popup `DOMContentLoaded`
- **mutates**: DOM

### `saveApiKey`
- **type**: function (async)
- **file**: `browser-extension/popup.js`
- **purpose**: Saves user-entered HSIP API key to `chrome.storage.local`.
- **inputs**: `key: string`
- **outputs**: none
- **calls**: `chrome.storage.local.set`
- **called_by**: API key form submit handler
- **mutates**: `chrome.storage.local`

### `renderCount`
- **type**: function
- **file**: `browser-extension/popup.js`
- **purpose**: Updates the tracker blocked count badge in the popup UI.
- **inputs**: `count: number`
- **outputs**: none
- **calls**: `document.getElementById`
- **called_by**: `loadStats`
- **mutates**: DOM

### `renderDomains`
- **type**: function
- **file**: `browser-extension/popup.js`
- **purpose**: Renders the list of blocked tracker domains for the current tab.
- **inputs**: `domains: string[]`
- **outputs**: none
- **calls**: `document.createElement`, `escapeHtml`
- **called_by**: `loadStats`
- **mutates**: DOM

### `renderHsipStatus`
- **type**: function
- **file**: `browser-extension/popup.js`
- **purpose**: Renders the HSIP server connection status dot (green/red) in the popup.
- **inputs**: `connected: boolean`
- **outputs**: none
- **calls**: `document.getElementById`
- **called_by**: `getHsipStatus`
- **mutates**: DOM

### `renderActivity`
- **type**: function
- **file**: `browser-extension/popup.js`
- **purpose**: Renders the last 5 AI agent audit entries in the popup activity panel.
- **inputs**: `activity: array`
- **outputs**: none
- **calls**: `document.createElement`, `escapeHtml`, `timeAgo`
- **called_by**: `getHsipStatus`
- **mutates**: DOM

### `escapeHtml`
- **type**: function
- **file**: `browser-extension/popup.js`
- **purpose**: Escapes `<`, `>`, `&` for safe HTML insertion.
- **inputs**: `s: string`
- **outputs**: `string`
- **calls**: `String.replace`
- **called_by**: `renderDomains`, `renderActivity`
- **mutates**: nothing

### `timeAgo`
- **type**: function
- **file**: `browser-extension/popup.js`
- **purpose**: Converts a Unix ms timestamp to a "N minutes ago" human-readable string.
- **inputs**: `ms: number`
- **outputs**: `string`
- **calls**: `Date.now`
- **called_by**: `renderActivity`
- **mutates**: nothing

---

## `browser-extension/content.js`

### `TRACKER_DOMAINS`
- **type**: variable (constant array)
- **file**: `browser-extension/content.js`
- **purpose**: List of 61 known tracker domain strings to detect via `PerformanceResourceTiming`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `isTrackerDomain`
- **mutates**: nothing

### `isTrackerDomain`
- **type**: function
- **file**: `browser-extension/content.js`
- **purpose**: Returns true if a URL string contains any domain in `TRACKER_DOMAINS`.
- **inputs**: `url: string`
- **outputs**: `boolean`
- **calls**: `TRACKER_DOMAINS.some`
- **called_by**: `scanResourceTimings`
- **mutates**: nothing

### `scanResourceTimings`
- **type**: function
- **file**: `browser-extension/content.js`
- **purpose**: Scans `performance.getEntriesByType("resource")` for tracker domain requests and reports them.
- **inputs**: none
- **outputs**: none
- **calls**: `isTrackerDomain`, `reportBlocked`
- **called_by**: `runScan`
- **mutates**: nothing

### `reportBlocked`
- **type**: function
- **file**: `browser-extension/content.js`
- **purpose**: Sends a `chrome.runtime.sendMessage` with the blocked domain info to the background service worker.
- **inputs**: `domain: string`
- **outputs**: none
- **calls**: `chrome.runtime.sendMessage`
- **called_by**: `scanResourceTimings`
- **mutates**: `tabStats` in background.js (via message)

### `runScan`
- **type**: function
- **file**: `browser-extension/content.js`
- **purpose**: Entry point: waits for page load then runs `scanResourceTimings`; sets a `MutationObserver` for dynamic content.
- **inputs**: none
- **outputs**: none
- **calls**: `scanResourceTimings`, `MutationObserver`
- **called_by**: content script injection
- **mutates**: nothing


---

## `sdks/python/hsip/client.py`

### `HSIPError`
- **type**: struct (exception class)
- **file**: `sdks/python/hsip/client.py`
- **purpose**: Custom exception for HSIP API errors; carries status_code and message.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `HSIPClient._request`
- **mutates**: nothing

### `HSIPClient`
- **type**: struct (class)
- **file**: `sdks/python/hsip/client.py`
- **purpose**: Python SDK client for the HSIP API; manages base URL, API key, and HTTP session.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: end-user SDK code
- **mutates**: nothing

### `HSIPClient.__init__`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: Initialises client with base URL and API key; creates `requests.Session` with auth header.
- **inputs**: `base_url: str`, `api_key: str`
- **outputs**: none
- **calls**: `requests.Session`
- **called_by**: SDK users
- **mutates**: `self.session`

### `HSIPClient._request`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: Internal HTTP helper: sends request, raises `HSIPError` on non-2xx.
- **inputs**: `method: str`, `path: str`, `**kwargs`
- **outputs**: `dict`
- **calls**: `self.session.request`, `response.raise_for_status`
- **called_by**: all public methods
- **mutates**: nothing

### `HSIPClient.get_or_create_identity`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `POST /v1/identity` — creates or retrieves Ed25519 identity.
- **inputs**: `self`
- **outputs**: `dict`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: DB via API

### `HSIPClient.get_identity`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `GET /v1/identity` — retrieves current identity.
- **inputs**: `self`
- **outputs**: `dict`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: nothing

### `HSIPClient.grant_consent`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `POST /v1/consent/grant` — grants consent to a peer.
- **inputs**: `self`, `peer_verify_key: str`, `scope: str`, `expires_ms: int | None`
- **outputs**: `dict`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: DB via API

### `HSIPClient.revoke_consent`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `POST /v1/consent/revoke` — revokes consent from a peer.
- **inputs**: `self`, `peer_verify_key: str`
- **outputs**: `dict`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: DB via API

### `HSIPClient.list_consents`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `GET /v1/consent` — lists all consents.
- **inputs**: `self`
- **outputs**: `list`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: nothing

### `HSIPClient.get_consent`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `GET /v1/consent/:peer_key` — checks consent for a specific peer.
- **inputs**: `self`, `peer_verify_key: str`
- **outputs**: `dict`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: nothing

### `HSIPClient.sign_message`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `POST /v1/messages/sign` — signs a message with Ed25519.
- **inputs**: `self`, `content: str`
- **outputs**: `dict`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: DB via API

### `HSIPClient.verify_message`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `POST /v1/messages/verify` — verifies an Ed25519 signature.
- **inputs**: `self`, `content: str`, `signature: str`, `verify_key: str`
- **outputs**: `dict`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: nothing

### `HSIPClient.list_messages`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `GET /v1/messages` — lists all signed messages.
- **inputs**: `self`
- **outputs**: `list`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: nothing

### `HSIPClient.get_audit_log`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `GET /v1/audit` — fetches audit log entries.
- **inputs**: `self`, `limit: int`, `action: str | None`
- **outputs**: `list`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: nothing

### `HSIPClient.create_key`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `POST /v1/keys` — creates a new API key.
- **inputs**: `self`, `name: str`, `agent_type: str`
- **outputs**: `dict`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: DB via API

### `HSIPClient.list_keys`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `GET /v1/keys` — lists all API keys.
- **inputs**: `self`
- **outputs**: `list`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: nothing

### `HSIPClient.revoke_key`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `DELETE /v1/keys/:id` — revokes an API key.
- **inputs**: `self`, `key_id: str`
- **outputs**: none
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: DB via API

### `HSIPClient.register_agent`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `POST /v1/keys` with `agent_type: "ai_agent"` — registers an AI agent key.
- **inputs**: `self`, `name: str`, `expires_days: int | None`
- **outputs**: `dict`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: DB via API

### `HSIPClient.list_agents`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `GET /v1/agents` — lists all AI agent keys with velocity stats.
- **inputs**: `self`
- **outputs**: `list`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: nothing

### `HSIPClient.revoke_agent`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: Finds agent by name then calls `DELETE /v1/keys/:id`.
- **inputs**: `self`, `name_or_id: str`
- **outputs**: none
- **calls**: `list_agents`, `revoke_key`
- **called_by**: SDK users
- **mutates**: DB via API

### `HSIPClient.log_action`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: Signs a message with `[ACTION:...]` prefix for explicit audit trail entries.
- **inputs**: `self`, `message: str`
- **outputs**: `dict`
- **calls**: `sign_message`
- **called_by**: SDK users
- **mutates**: DB via API

### `HSIPClient.discover_agents`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `GET /v1/agents/discover` — probes localhost ports for running AI agents.
- **inputs**: `self`
- **outputs**: `list`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: nothing

### `HSIPClient.hash_payload`
- **type**: function (static)
- **file**: `sdks/python/hsip/client.py`
- **purpose**: Hex-encoded SHA-256 of a decision payload the caller wants attested. Kept as a static helper so callers get the exact encoding `record_decision`'s `payload_hash` expects without reimplementing it.
- **inputs**: `payload: bytes`
- **outputs**: `str`
- **calls**: `hashlib.sha256`
- **called_by**: SDK users, `record_decision` callers

### `HSIPClient.record_decision`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `POST /v1/decisions` — signs and chains one AI-agent decision attestation. If `receipt_dir` is given, immediately persists the receipt via `save_receipt` — the client-side mitigation for the gap between signing and the next anchor cycle (see `anchor_job.rs`).
- **inputs**: `self`, `accountable_key: str`, `model_version: str`, `strategy_id: str`, `decision_type: str`, `payload_hash: str`, `receipt_dir: Optional[str]`
- **outputs**: `dict`
- **calls**: `_request`, `save_receipt`
- **called_by**: SDK users (e.g. Predicta's trading loop)
- **mutates**: DB via API; filesystem if `receipt_dir` given

### `HSIPClient.save_receipt`
- **type**: function (static)
- **file**: `sdks/python/hsip/client.py`
- **purpose**: Writes a decision receipt to `<receipt_dir>/<decision_id>.json`. Callable independently of `record_decision` (e.g. to re-save a receipt fetched later via `get_decision_proof`).
- **inputs**: `receipt: dict`, `receipt_dir: str`
- **outputs**: `str` (path written)
- **called_by**: `record_decision`, SDK users
- **mutates**: filesystem

### `HSIPClient.list_decisions`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `GET /v1/decisions` — lists this tenant's decision attestations, newest first.
- **inputs**: `self`
- **outputs**: `list`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: nothing

### `HSIPClient.get_decision_proof`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `GET /v1/decisions/:id/proof` — full self-contained verification bundle. `anchored` is `False` until the next anchor cycle runs.
- **inputs**: `self`, `decision_id: str`
- **outputs**: `dict`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: nothing

### `HSIPClient.verify_decision`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `POST /v1/decisions/verify` — thin wrapper over an endpoint that itself takes no API key and touches no database; documented as such so callers know they could reimplement this check independently of HSIP entirely.
- **inputs**: `self`, `bundle: dict`
- **outputs**: `dict`
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: nothing

Ported to the Node and Go SDKs — see their sections below (`HSIPClient.hashPayload` etc. / `hsip.HashPayload` etc.). Field names and behavior match these Python methods exactly; only the local binding style differs (Python kwargs, a Node options object, a Go opts/request struct).

---

## `sdks/node/src/index.js`

> All methods mirror the Python SDK (camelCase naming). Only unique items noted.

### `HSIPClient` (Node)
- **type**: struct (class)
- **file**: `sdks/node/src/index.js`
- **purpose**: Node.js SDK client; uses `node-fetch` or global `fetch`; same API surface as Python SDK.
- **inputs**: `baseUrl: string`, `apiKey: string`
- **outputs**: none
- **calls**: `fetch`
- **called_by**: Node.js user code
- **mutates**: nothing

### `HSIPClient.request` (Node)
- **type**: function (async)
- **file**: `sdks/node/src/index.js`
- **purpose**: Internal fetch helper with `Authorization: Bearer` header; throws `HSIPError` on non-2xx.
- **inputs**: `method: string`, `path: string`, `body?: object`
- **outputs**: `Promise<object>`
- **calls**: `fetch`, `response.json`
- **called_by**: all public methods
- **mutates**: nothing

### `HSIPClient.hashPayload` (Node)
- **type**: function (static)
- **file**: `sdks/node/src/index.js`
- **purpose**: Hex-encoded SHA-256 of a decision payload, ready for `payloadHash`. Mirrors Python's `hash_payload`.
- **inputs**: `payload: Buffer | string`
- **outputs**: `string`
- **calls**: Node's `crypto.createHash('sha256')`
- **called_by**: SDK users, `recordDecision` callers

### `HSIPClient.recordDecision` (Node)
- **type**: function (async)
- **file**: `sdks/node/src/index.js`
- **purpose**: `POST /v1/decisions` — signs and chains one AI-agent decision attestation. If `receiptDir` is given, immediately persists the receipt via `saveReceipt`.
- **inputs**: `{accountableKey, modelVersion, strategyId, decisionType, payloadHash, receiptDir?}`
- **outputs**: `Promise<RecordDecisionResponse>`
- **calls**: `_request`, `HSIPClient.saveReceipt`
- **called_by**: SDK users
- **mutates**: DB via API; filesystem if `receiptDir` given

### `HSIPClient.saveReceipt` (Node)
- **type**: function (static)
- **file**: `sdks/node/src/index.js`
- **purpose**: Writes a decision receipt to `<receiptDir>/<decision_id>.json`. Callable independently of `recordDecision`.
- **inputs**: `receipt: object`, `receiptDir: string`
- **outputs**: `string` (path written)
- **calls**: `fs.mkdirSync`, `fs.writeFileSync`
- **called_by**: `recordDecision`, SDK users
- **mutates**: filesystem

### `HSIPClient.listDecisions` / `getDecisionProof` / `verifyDecision` (Node)
- **type**: functions (async)
- **file**: `sdks/node/src/index.js`
- **purpose**: `GET /v1/decisions`, `GET /v1/decisions/:id/proof`, and `POST /v1/decisions/verify` respectively — same semantics as the Python SDK's `list_decisions`/`get_decision_proof`/`verify_decision` (see those entries above; `verifyDecision` calls an endpoint that itself takes no API key and touches no database).
- **calls**: `_request`
- **called_by**: SDK users
- **mutates**: nothing

---

## `sdks/go/hsip/client.go`

### `Client` (Go)
- **type**: struct
- **file**: `sdks/go/hsip/client.go`
- **purpose**: Go SDK client: BaseURL, APIKey, HTTPClient fields.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: Go user code
- **mutates**: nothing

### `HashPayload` (Go)
- **type**: function (package-level, not a `Client` method)
- **file**: `sdks/go/hsip/client.go`
- **purpose**: Hex-encoded SHA-256 of a decision payload, ready for `RecordDecisionOpts.PayloadHash`. Package-level rather than a method since it needs no server connection — matches Python's `@staticmethod` / Node's `static`.
- **inputs**: `payload []byte`
- **outputs**: `string`
- **calls**: `crypto/sha256`, `encoding/hex`
- **called_by**: Go SDK users, `RecordDecision` callers

### `SaveReceipt` (Go)
- **type**: function (package-level)
- **file**: `sdks/go/hsip/client.go`
- **purpose**: Writes a decision receipt to `<receiptDir>/<decision_id>.json`. Callable independently of `RecordDecision`.
- **inputs**: `receipt *RecordDecisionResponse`, `receiptDir string`
- **outputs**: `(string, error)` (path written)
- **calls**: `os.MkdirAll`, `os.WriteFile`
- **called_by**: `Client.RecordDecision`, Go SDK users
- **mutates**: filesystem

### `Client.RecordDecision` (Go)
- **type**: function
- **file**: `sdks/go/hsip/client.go`
- **purpose**: `POST /v1/decisions` — signs and chains one AI-agent decision attestation. If `opts.ReceiptDir` is non-empty, immediately persists the receipt via `SaveReceipt`.
- **inputs**: `opts RecordDecisionOpts`
- **outputs**: `(*RecordDecisionResponse, error)`
- **calls**: `Client.do`, `SaveReceipt`
- **called_by**: Go SDK users
- **mutates**: DB via API; filesystem if `opts.ReceiptDir` given

### `Client.ListDecisions` / `GetDecisionProof` / `VerifyDecision` (Go)
- **type**: functions
- **file**: `sdks/go/hsip/client.go`
- **purpose**: `GET /v1/decisions`, `GET /v1/decisions/:id/proof`, and `POST /v1/decisions/verify` respectively — same semantics as the Python SDK's `list_decisions`/`get_decision_proof`/`verify_decision` (`VerifyDecision` calls an endpoint that itself takes no API key and touches no database).
- **calls**: `Client.do`
- **called_by**: Go SDK users
- **mutates**: nothing

### `APIError` (Go)
- **type**: struct
- **file**: `sdks/go/hsip/client.go`
- **purpose**: Go SDK error type: StatusCode int, Message string.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `Client.do`
- **mutates**: nothing

### `New` (Go)
- **type**: function
- **file**: `sdks/go/hsip/client.go`
- **purpose**: Constructs a `Client` with given base URL and API key.
- **inputs**: `baseURL: string`, `apiKey: string`
- **outputs**: `*Client`
- **calls**: `http.DefaultClient`
- **called_by**: Go user code
- **mutates**: nothing

### `Client.do` (Go)
- **type**: function
- **file**: `sdks/go/hsip/client.go`
- **purpose**: Internal HTTP helper: sets auth header, executes request, decodes JSON response or returns `APIError`.
- **inputs**: `method, path string`, `body interface{}`
- **outputs**: `(json.RawMessage, error)`
- **calls**: `http.NewRequest`, `json.NewEncoder`, `json.NewDecoder`
- **called_by**: all public methods
- **mutates**: nothing

> All HSIP API methods (`GetOrCreateIdentity`, `GrantConsent`, `SignMessage`, `RegisterAgent`, `LogAction`, `DiscoverAgents`, etc.) follow the same pattern as Python SDK — they call `Client.do` with the appropriate path and body.

---

## `crates/hsip-verify/src/lib.rs`

Formal verification of three HSIP security properties via the Z3 SMT solver: consent non-forgery, temporal consistency (revocation is permanent), and identity-binding soundness (no peer-ID collisions). See the crate's own `README.md` for the formal specifications. Now a normal workspace member (previously excluded — `cargo build --workspace`/`cargo test --workspace` didn't build or run it) — see "Including hsip-verify in the Build" in `CLAUDE.md` for why and what changed.

### `VerificationConfig`
- **type**: struct
- **file**: `crates/hsip-verify/src/lib.rs`
- **purpose**: Tunable bounds for the symbolic model (number of symbolic peers/timestamps considered, etc.) passed to `Verifier::new`.
- **called_by**: `Verifier::new`, crate consumers (`examples/verify_hsip.rs`, `tests/verification_tests.rs`)

### `Verifier`
- **type**: struct
- **file**: `crates/hsip-verify/src/lib.rs`
- **purpose**: Owns a Z3 `Context` and runs each property proof against it.
- **calls**: `z3::Context::new`
- **called_by**: crate consumers

### `Verifier::new`
- **type**: function
- **file**: `crates/hsip-verify/src/lib.rs`
- **purpose**: Builds a `Verifier` (and its underlying Z3 context) from a `VerificationConfig`.
- **inputs**: `config: VerificationConfig`
- **outputs**: `Self`
- **mutates**: nothing

### `Verifier::verify_all`
- **type**: function
- **file**: `crates/hsip-verify/src/lib.rs`
- **purpose**: Runs all three property proofs and aggregates them into one `VerificationReport`.
- **inputs**: `&self`
- **outputs**: `VerificationReport`
- **calls**: `verify_consent_non_forgery`, `verify_temporal_consistency`, `verify_identity_binding`
- **called_by**: crate consumers, `tests/verification_tests.rs::test_full_verification_suite`
- **mutates**: nothing (each call builds a fresh Z3 `Solver`)

### `Verifier::verify_consent_non_forgery`
- **type**: function
- **file**: `crates/hsip-verify/src/lib.rs`
- **purpose**: Proves a valid consent signature can only be produced by the holder of the corresponding private key — encodes the property as a Z3 formula and checks it's unsatisfiable to violate (i.e. proven, not just spot-tested).
- **inputs**: `&self`
- **outputs**: `PropertyResult` (proven / violated-with-counterexample, via `counterexample.rs`)
- **called_by**: `verify_all`

### `Verifier::verify_temporal_consistency`
- **type**: function
- **file**: `crates/hsip-verify/src/lib.rs`
- **purpose**: Proves that once consent is revoked at time `t`, it stays revoked for every `t' > t` — no timing-attack window where a stale "still granted" read is possible.
- **inputs**: `&self`
- **outputs**: `PropertyResult`
- **called_by**: `verify_all`

### `Verifier::verify_identity_binding`
- **type**: function
- **file**: `crates/hsip-verify/src/lib.rs`
- **purpose**: Proves a peer ID uniquely determines a single public key — no two distinct keys can derive the same peer ID (collision resistance of the identity-binding function within the symbolic model's bounds).
- **inputs**: `&self`
- **outputs**: `PropertyResult`
- **called_by**: `verify_all`

### `VerificationReport`
- **type**: struct
- **file**: `crates/hsip-verify/src/lib.rs`
- **purpose**: Aggregates named `PropertyResult`s from one `verify_all` run.
- **calls**: none
- **called_by**: `Verifier::verify_all`, crate consumers

### `VerificationReport::all_proven` / `has_violations` / `summary` / `get_result` / `results`
- **type**: functions
- **file**: `crates/hsip-verify/src/lib.rs`
- **purpose**: Query helpers over a completed report — whether every property proved, whether any was violated, a human-readable summary string, and lookup by property name.
- **called_by**: crate consumers, `examples/verify_hsip.rs`

---

## `crates/hsip-verify/src/models.rs`

Symbolic Z3 models of the consent protocol's state (grants, revocations, signatures) that `lib.rs`'s property proofs are built on top of, plus concrete (non-symbolic) equivalents exercised by `tests/verification_tests.rs`'s `*_concrete` tests as a sanity cross-check against the symbolic proofs.

---

## `crates/hsip-verify/src/properties.rs`

Z3 formula builders for the three properties themselves (consent non-forgery, temporal consistency, identity binding) — the actual `∀`/`∃` encodings referenced in the crate's `README.md`, invoked by `lib.rs`'s `Verifier::verify_*` methods.

---

## `crates/hsip-verify/src/counterexample.rs`

- **purpose**: When a property proof's negation is satisfiable (i.e. the property doesn't hold), builds a human-readable `Counterexample` from the Z3 model that satisfied it — concrete values showing exactly how the property fails, not just a bare "unsat/sat" result.
- **called_by**: `lib.rs`'s `Verifier::verify_*` methods (on the violated path)

