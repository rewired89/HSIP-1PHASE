# CODEMAP.md — HSIP Codebase Function & Variable Reference

> Auto-generated. See `## CodeMap Protocol` in CLAUDE.md for maintenance rules.

---

## `crates/hsip-api/src/main.rs`

### `main`
- **type**: function
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: Binary entry point; sets up Tokio runtime and calls `run()`, writing any fatal error to disk.
- **inputs**: none
- **outputs**: `std::process::ExitCode`
- **calls**: `run`, `fatal`, `write_error_log`
- **called_by**: OS
- **mutates**: nothing (delegates to `run`)

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
- **purpose**: Full server startup: loads config, master key, DB, bootstraps admin, builds Axum router, binds TCP listener, serves.
- **inputs**: none
- **outputs**: `Result<()>`
- **calls**: `Config::load`, `Config::desktop_defaults`, `init_logging`, `load_master_key`, `db::init`, `bootstrap_admin`, `build_cors_layer`, `AppState::new`, `router`, `create_shortcuts`
- **called_by**: `main`
- **mutates**: filesystem (admin key), DB (migrations, initial tenant/key rows)

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
- **purpose**: Reads master key from file path in config (hex-encoded 32 bytes) or falls back to `HSIP_MASTER_KEY` env var.
- **inputs**: `config: &Config`
- **outputs**: `Result<[u8; 32]>`
- **calls**: `fs::read_to_string`, `hex::decode`
- **called_by**: `run`
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
- **purpose**: On first boot creates the default tenant, generates the admin API key, writes it to the admin key file, and prints it to stdout.
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
- **purpose**: Paths to TLS cert/key files and whether to require HTTPS.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `ServerConfig`
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
- **purpose**: Implements `FromRequestParts`: extracts Bearer token, hashes it, looks up in DB, checks active/expiry/pending-revocation, enforces rate limit and AI velocity check.
- **inputs**: `parts: &mut Parts`, `state: &AppState`
- **outputs**: `Result<Self, ApiError>`
- **calls**: `hash_key`, `check_rate_limit`, `check_agent_velocity`, `sqlx::query`
- **called_by**: Axum extractor machinery
- **mutates**: `rate_limiter` DashMap (inserts/updates window), `agent_tracker` DashMap

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
- **type**: function
- **file**: `crates/hsip-api/src/auth.rs`
- **purpose**: For `ai_agent` keys: logs anomaly audit entry if >100 req/min, auto-revokes key if >1000 req/min.
- **inputs**: `key_hash: &str`, `key_id: &str`, `tenant_id: &str`, `agent_tracker: &AgentTracker`, `db: &Db`
- **outputs**: `Result<(), ApiError>`
- **calls**: `AgentTracker::entry`, `sqlx::query` (audit insert, key revoke)
- **called_by**: `TenantId::from_request_parts`
- **mutates**: `agent_tracker` DashMap, DB (`api_keys` revocation, `audit_entries`)

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
- **purpose**: Inline SQL migrations: creates all tables (tenants, api_keys, identities, consents, messages, audit_entries, contacts, credentials, trusted_peers) and adds missing columns idempotently.
- **inputs**: `db: &Db`
- **outputs**: `Result<()>`
- **calls**: `sqlx::query().execute(db)`
- **called_by**: `init`, `init_with_config`
- **mutates**: DB schema

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

### `AgentTracker`
- **type**: variable (type alias)
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: `DashMap<String, VelocityRecord>` keyed by key_hash for AI agent velocity tracking.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `AppState`, `check_agent_velocity`
- **mutates**: nothing

### `RateLimiter`
- **type**: variable (type alias)
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: `DashMap<String, RateWindow>` keyed by key_hash for per-key rate limiting.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `AppState`, `check_rate_limit`
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

### `AppState`
- **type**: struct
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: Axum shared state: DB pool, config, master key, rate limiter, agent tracker, pending revocations, DNS state, proxy shared buffer.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: all Axum route handlers via `State<AppState>` extractor
- **mutates**: nothing (container for shared state)

### `AppState::new`
- **type**: function
- **file**: `crates/hsip-api/src/state.rs`
- **purpose**: Constructs `AppState` from DB pool, config, and master key.
- **inputs**: `db: Db`, `config: Config`, `master_key: [u8; 32]`
- **outputs**: `Self`
- **calls**: `DashMap::new`, `DashSet::new`, `ProxyShared::new`
- **called_by**: `run`
- **mutates**: nothing


---

## `crates/hsip-api/src/key_encryption.rs`

### `derive_encryption_key`
- **type**: function
- **file**: `crates/hsip-api/src/key_encryption.rs`
- **purpose**: Derives a 32-byte ChaCha20-Poly1305 key from master key + tenant ID via HKDF-SHA256.
- **inputs**: `master_key: &[u8; 32]`, `tenant_id: &str`
- **outputs**: `[u8; 32]`
- **calls**: `hkdf::Hkdf::new`, `expand`
- **called_by**: `encrypt_signing_key`, `decrypt_signing_key`
- **mutates**: nothing

### `load_master_key`
- **type**: function
- **file**: `crates/hsip-api/src/key_encryption.rs`
- **purpose**: Reads hex-encoded 32-byte master key from file, validates length; alias used in tests.
- **inputs**: `path: &str`
- **outputs**: `Result<[u8; 32]>`
- **calls**: `fs::read_to_string`, `hex::decode`
- **called_by**: test helpers
- **mutates**: nothing

### `encrypt_signing_key`
- **type**: function
- **file**: `crates/hsip-api/src/key_encryption.rs`
- **purpose**: Encrypts a 32-byte Ed25519 signing key with ChaCha20-Poly1305 using a random nonce; returns `nonce_hex:ciphertext_hex`.
- **inputs**: `signing_key: &[u8; 32]`, `master_key: &[u8; 32]`, `tenant_id: &str`
- **outputs**: `Result<String>`
- **calls**: `derive_encryption_key`, `OsRng`, `ChaCha20Poly1305::encrypt`
- **called_by**: `identity::create_or_get`, `identity::rotate`
- **mutates**: nothing (returns new string)

### `decrypt_signing_key`
- **type**: function
- **file**: `crates/hsip-api/src/key_encryption.rs`
- **purpose**: Decrypts a `nonce_hex:ciphertext_hex` blob back to the 32-byte Ed25519 signing key.
- **inputs**: `encrypted: &str`, `master_key: &[u8; 32]`, `tenant_id: &str`
- **outputs**: `Result<[u8; 32]>`
- **calls**: `derive_encryption_key`, `hex::decode`, `ChaCha20Poly1305::decrypt`
- **called_by**: `identity::load_signing_key`, `messages::sign`
- **mutates**: nothing

---

## `crates/hsip-api/src/errors.rs`

### `ApiError`
- **type**: enum
- **file**: `crates/hsip-api/src/errors.rs`
- **purpose**: Typed error enum for all API failures: Unauthorized, Forbidden, NotFound, BadRequest, TooManyRequests, Internal.
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
- **calls**: `ed25519_dalek::SigningKey::generate`, `encrypt_signing_key`, `sqlx::query`
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
- **calls**: `ed25519_dalek::SigningKey::generate`, `encrypt_signing_key`, `sqlx::query`
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
- **purpose**: Serialised consent row returned in list/get responses.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list`, `get` (consent)
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
- **purpose**: `POST /v1/consent/grant` — upserts consent record with status `granted`, writes audit entry.
- **inputs**: `State(state)`, `tenant`, `Json(req)`
- **outputs**: `ApiResult<Json<ConsentRecord>>`
- **calls**: `validate_peer_key`, `sqlx::query`, `now_ms`
- **called_by**: Axum router
- **mutates**: DB (`consents`, `audit_entries`)

### `revoke` (consent)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/consent.rs`
- **purpose**: `POST /v1/consent/revoke` — updates consent status to `revoked`, writes audit entry.
- **inputs**: `State(state)`, `tenant`, `Json(req)`
- **outputs**: `ApiResult<Json<ConsentRecord>>`
- **calls**: `validate_peer_key`, `sqlx::query`
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
- **calls**: `load_signing_key`, `ed25519_dalek::SigningKey::sign`, `sqlx::query`, `metrics::MESSAGES_SIGNED.inc`
- **called_by**: Axum router
- **mutates**: DB (`messages`, `audit_entries`)

### `verify` (messages)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/messages.rs`
- **purpose**: `POST /v1/messages/verify` — verifies an Ed25519 signature against provided content and verify key.
- **inputs**: `State(state)`, `tenant`, `Json(req)`
- **outputs**: `ApiResult<Json<VerifyResponse>>`
- **calls**: `ed25519_dalek::VerifyingKey::verify_strict`
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
- **calls**: `load_signing_key`, `canonical_json`, `ed25519_dalek::SigningKey::sign`, `sqlx::query`, `metrics::CREDENTIALS_ISSUED.inc`
- **called_by**: Axum router
- **mutates**: DB (`credentials`, `audit_entries`)

### `verify` (credentials)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/credentials.rs`
- **purpose**: `POST /v1/credentials/verify` — verifies a credential signature against its payload; checks revocation status; increments CREDENTIALS_VERIFIED metric.
- **inputs**: `State(state)`, `tenant`, `Json(req)`
- **outputs**: `ApiResult<Json<VerifyResponse>>`
- **calls**: `canonical_json`, `ed25519_dalek::VerifyingKey::verify_strict`, `sqlx::query`, `metrics::CREDENTIALS_VERIFIED.inc`
- **called_by**: Axum router
- **mutates**: nothing (read-only verification)

### `revoke` (credentials)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/credentials.rs`
- **purpose**: `DELETE /v1/credentials/:id` — marks credential as revoked in DB, writes audit entry.
- **inputs**: `State(state)`, `tenant`, `Path(id)`
- **outputs**: `ApiResult<StatusCode>`
- **calls**: `sqlx::query`
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
- **purpose**: JSON body for `POST /v1/keys`: name, agent_type (human/service/ai_agent), optional expires_at.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `create`
- **mutates**: nothing

### `CreateKeyResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/keys.rs`
- **purpose**: Response from key creation: id, key (raw, returned only once), name, agent_type, created_at.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `create`
- **mutates**: nothing

### `KeyRecord`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/keys.rs`
- **purpose**: Serialised key row for list response: id, name, agent_type, created_at, expires_at, active flag.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list` (keys)
- **mutates**: nothing

### `create` (keys)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/keys.rs`
- **purpose**: `POST /v1/keys` — generates new API key, stores its SHA-256 hash, returns raw key once.
- **inputs**: `State(state)`, `tenant`, `Json(req)`
- **outputs**: `ApiResult<Json<CreateKeyResponse>>`
- **calls**: `gen_key`, `hash_key`, `sqlx::query`
- **called_by**: Axum router
- **mutates**: DB (`api_keys`, `audit_entries`)

### `list` (keys)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/keys.rs`
- **purpose**: `GET /v1/keys` — returns all active API keys for the tenant (no raw key values).
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<Vec<KeyRecord>>>`
- **calls**: `sqlx::query_as`
- **called_by**: Axum router
- **mutates**: nothing

### `revoke` (keys)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/keys.rs`
- **purpose**: `DELETE /v1/keys/:id` — deactivates the key in DB and adds to `pending_revocation` set for immediate blocking.
- **inputs**: `State(state)`, `tenant`, `Path(id)`
- **outputs**: `ApiResult<StatusCode>`
- **calls**: `sqlx::query`, `state.pending_revocation.insert`
- **called_by**: Axum router
- **mutates**: DB (`api_keys.active`), `pending_revocation` DashSet

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
- **purpose**: List of 12 localhost ports probed for running AI agents: Ollama, LM Studio, Jupyter, Vite, CRA, Next.js, FastAPI, Flask, Express, Deno, Node-RED, Gradio.
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
- **purpose**: Serialised audit row: id, action, details, timestamp.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `list` (audit)
- **mutates**: nothing

### `list` (audit)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/audit.rs`
- **purpose**: `GET /v1/audit` — returns paginated, optionally filtered audit log entries for the tenant.
- **inputs**: `State(state)`, `tenant`, `Query(params)`
- **outputs**: `ApiResult<Json<Vec<AuditEntry>>>`
- **calls**: `sqlx::query_as`
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
- **purpose**: `GET /v1/proxy/status` — returns proxy running state and stats.
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<ProxyStatus>>`
- **calls**: `compute_stats`
- **called_by**: Axum router
- **mutates**: nothing

### `enable` (proxy)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: `POST /v1/proxy/enable` — starts the MITM proxy thread on specified port.
- **inputs**: `State(state)`, `tenant`, `Json(req)`
- **outputs**: `ApiResult<Json<ProxyStatus>>`
- **calls**: `run_proxy_thread`, `std::thread::spawn`
- **called_by**: Axum router
- **mutates**: `state.proxy` (running flag, port)

### `disable` (proxy)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: `POST /v1/proxy/disable` — signals the proxy thread to stop.
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<ProxyStatus>>`
- **calls**: `state.proxy.write()` (sets running false)
- **called_by**: Axum router
- **mutates**: `state.proxy`

### `log` (proxy)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/proxy.rs`
- **purpose**: `GET /v1/proxy/log` — returns recent proxy event ring buffer contents.
- **inputs**: `State(state)`, `tenant`
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
- **purpose**: `GET /v1/proxy/setup` — returns OS-specific proxy configuration instructions.
- **inputs**: `State(state)`, `tenant`
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
- **calls**: `hex::decode`, `ed25519_dalek::VerifyingKey::from_bytes`, `sqlx::query`
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
- **calls**: `sqlx::query`
- **called_by**: Axum router
- **mutates**: DB (`trusted_peers`, `audit_entries`)

### `verify` (trust)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/trust.rs`
- **purpose**: `POST /v1/trust/verify` — looks up peer by label, verifies Ed25519 signature, writes `trust.verify_ok` or `trust.verify_failed` audit entry.
- **inputs**: `State(state)`, `tenant`, `Json(req)`
- **outputs**: `ApiResult<Json<TrustVerifyResponse>>`
- **calls**: `sqlx::query`, `hex::decode`, `ed25519_dalek::VerifyingKey::verify`
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
- **purpose**: `POST /v1/uploads` — saves a multipart image upload to the data directory, returns its URL path.
- **inputs**: `State(state)`, `tenant`, `Multipart`
- **outputs**: `ApiResult<Json<UploadResponse>>`
- **calls**: `fs::write`, `axum::extract::Multipart`
- **called_by**: Axum router
- **mutates**: filesystem (uploads directory)

### `serve` (uploads)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/uploads.rs`
- **purpose**: `GET /v1/uploads/:filename` — serves a previously uploaded file from disk.
- **inputs**: `State(state)`, `Path(filename)`
- **outputs**: `ApiResult<impl IntoResponse>`
- **calls**: `fs::read`, `mime_guess`
- **called_by**: Axum router
- **mutates**: nothing


---

## `crates/hsip-cli/src/main.rs`

### `Commands`
- **type**: enum
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: Top-level clap subcommand enum: Keygen, Init, Export, Import, Consent, Session, Token, Discover, Reputation, Daemon, Audit, Agent, Trust, Up, Status, Diag.
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
- **type**: variable (constant)
- **file**: `crates/hsip-cli/src/identity.rs`
- **purpose**: Environment variable name for the HSIP API key: `"HSIP_API_KEY"`.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `run_identity_broker`
- **mutates**: nothing

### `Status`
- **type**: struct
- **file**: `crates/hsip-cli/src/identity.rs`
- **purpose**: Identity broker status response: connected bool, verify_key, server_url.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `run_identity_broker`
- **mutates**: nothing

### `TokenReq`
- **type**: struct
- **file**: `crates/hsip-cli/src/identity.rs`
- **purpose**: Token request payload from a relying party: scope, audience, nonce.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `token`
- **mutates**: nothing

### `TokenResp`
- **type**: struct
- **file**: `crates/hsip-cli/src/identity.rs`
- **purpose**: Token response: signed JWT-like token string.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `token`
- **mutates**: nothing

### `run_identity_broker`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/identity.rs`
- **purpose**: Starts a local HTTP broker that lets web pages request signed identity tokens from the HSIP server.
- **inputs**: `args: BrokerArgs`
- **outputs**: `Result<()>`
- **calls**: `axum::serve`, `token`, `demo`
- **called_by**: `main`
- **mutates**: network (binds port)

### `token`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/identity.rs`
- **purpose**: Broker handler: receives `TokenReq`, forwards signing request to HSIP API, returns `TokenResp`.
- **inputs**: `State`, `Json(req): Json<TokenReq>`
- **outputs**: `impl IntoResponse`
- **calls**: `reqwest::Client::post` (to `/v1/messages/sign`)
- **called_by**: `run_identity_broker` (Axum route)
- **mutates**: DB via API

### `demo`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/identity.rs`
- **purpose**: Serves a simple HTML demo page showing identity broker integration.
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

### `App`
- **type**: function (React component)
- **file**: `dashboard/src/App.jsx`
- **purpose**: Root application component: manages login state, navigation between tabs, and progressive disclosure of Advanced tabs.
- **inputs**: none
- **outputs**: JSX
- **calls**: `handleLogin`, `logout`, `navigateTo`, `switchMode`
- **called_by**: `main.jsx` (React root)
- **mutates**: `localStorage` (api key, onboarding state)

### `SIMPLE_TABS`
- **type**: variable (constant array)
- **file**: `dashboard/src/App.jsx`
- **purpose**: 8 primary nav tabs always visible: Home, Messages, Traffic Monitor, Alibi, Consents, AI Watch, Trackers, Protection.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `App`
- **mutates**: nothing

### `EXPERT_TABS`
- **type**: variable (constant array)
- **file**: `dashboard/src/App.jsx`
- **purpose**: 5 Advanced nav tabs shown behind toggle: Identity, Consent, Credentials, Keys, Audit.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `App`
- **mutates**: nothing

### `switchMode`
- **type**: function
- **file**: `dashboard/src/App.jsx`
- **purpose**: Toggles the `showAdv` state to expand/collapse the Advanced section.
- **inputs**: none
- **outputs**: none
- **calls**: `setShowAdv`
- **called_by**: "Advanced ▾" button in nav
- **mutates**: React state (`showAdv`)

### `handleLogin`
- **type**: function (async)
- **file**: `dashboard/src/App.jsx`
- **purpose**: Validates API key via `GET /health`, stores key in localStorage on success.
- **inputs**: `key: string`
- **outputs**: none
- **calls**: `request`, `localStorage.setItem`
- **called_by**: login form submit
- **mutates**: `localStorage`

### `logout`
- **type**: function
- **file**: `dashboard/src/App.jsx`
- **purpose**: Clears API key from localStorage and resets app to login screen.
- **inputs**: none
- **outputs**: none
- **calls**: `localStorage.removeItem`, `setApiKey`
- **called_by**: logout button
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
- **purpose**: API key management page: create, list, and revoke API keys for human/service/ai_agent types.
- **inputs**: none
- **outputs**: JSX
- **calls**: `request`
- **called_by**: `App`
- **mutates**: DB via API (api_keys)

---

## `dashboard/src/pages/Audit.jsx`

### `Audit`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Audit.jsx`
- **purpose**: Audit log viewer: paginated, filterable list of all audit entries for the tenant.
- **inputs**: none
- **outputs**: JSX
- **calls**: `request`
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

