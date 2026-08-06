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
- **purpose**: Full server startup: loads config, master key, DB, bootstraps admin, builds Axum router, binds TCP listener, serves. Restores rate-limit/AI-agent-velocity state via `rate_limit_persistence::load` before accepting traffic. Also spawns five background loops: the anchoring cycle (~10s poll, calls both `anchor_job::run_anchor_cycle` for decisions and `anchor_job::run_audit_anchor_cycle` for the audit log on every tick), the OpenTimestamps upgrade-poll cycle (15-minute interval — deliberately much slower than the 10s anchor loop, since Bitcoin blocks land roughly every 10 minutes on average — calls `anchor_job::run_upgrade_cycle` to flip confirmed `ots_status = 'pending'` batches to `'confirmed'`; see THREAT_MODEL.md §4.21), a system-health metrics refresh (5-minute interval, calls `system_health::check_and_update_metrics` so `metrics::SYSTEM_HEALTH_ISSUES` stays current even if nobody polls `GET /v1/admin/system-health`; see THREAT_MODEL.md §4.22), a rate-limit state snapshot (`rate_limit_persistence::SNAPSHOT_INTERVAL_SECS` = 30s interval, calls `rate_limit_persistence::snapshot`), and a replay-nonce sweep (60s interval, `state.replay_nonces.retain(...)`) that removes expired `(key_id, nonce)` entries so opt-in HTTP replay protection (see `auth.rs::check_replay_protection`) can't grow the tracker unbounded. When `[server.tls]` is configured, delegates cert/key (and optional mutual-TLS client-CA) loading to `mtls::build_rustls_config` instead of calling `RustlsConfig::from_pem_file` directly — logs "Mutual TLS enabled" when `tls_config.client_ca_path` is set. When `client_ca_path` is additionally set, binds via `axum_server::bind(...).acceptor(mtls::ClientCertAcceptor::new(tls_config))` instead of `axum_server::bind_rustls(...)` — the only way a request ever carries a `mtls::ClientCertFingerprint` extension for `auth.rs`'s per-key binding check to read. The plain (no client CA) TLS branch keeps calling `axum_server::bind_rustls(...)` exactly as before, unchanged.
- **inputs**: none
- **outputs**: `Result<()>`
- **calls**: `Config::load`, `Config::desktop_defaults`, `init_logging`, `load_master_key`, `db::init`, `bootstrap_admin`, `build_cors_layer`, `AppState::new`, `router`, `create_shortcuts`, `anchor_job::run_anchor_cycle`, `anchor_job::run_audit_anchor_cycle`, `anchor_job::run_upgrade_cycle`, `system_health::check_and_update_metrics`, `rate_limit_persistence::{load, snapshot}`, `db::now_ms`, `mtls::build_rustls_config`, `mtls::ClientCertAcceptor::new`
- **called_by**: `main`
- **mutates**: filesystem (admin key), DB (migrations, initial tenant/key rows, `rate_limit_state` snapshots, `decision_anchors`/`audit_anchors` `ots_status` via the upgrade cycle), `state.replay_nonces` DashMap (sweep removes expired entries), `state.rate_limiter`/`agent_tracker`/`sandbox_rate` (populated from persisted state at startup), `metrics::SYSTEM_HEALTH_ISSUES` gauge values

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

### `to_wide`
- **type**: function
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: `#[cfg(all(windows, feature = "embed-dashboard"))]`. Converts a Rust `&str` to a null-terminated UTF-16 `Vec<u16>` for building a Win32 `PCWSTR` — caller must keep the returned `Vec` alive for as long as any `PCWSTR` built from it is used, since `PCWSTR` is just a borrowed pointer.
- **inputs**: `s: &str`
- **outputs**: `Vec<u16>`
- **calls**: `OsStr::encode_wide`
- **called_by**: `write_shortcut`
- **mutates**: nothing

### `write_shortcut`
- **type**: function (unsafe)
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: `#[cfg(all(windows, feature = "embed-dashboard"))]`. Writes one `.lnk` shortcut at `dest` pointing at `target_exe`, via the real Windows Shell COM API (`IShellLinkW` + `IPersistFile`) — replaced the third-party `mslnk` crate (tiny, single-maintainer, version-0.1 dependency, identified as the highest abandonment risk in the tree) with Microsoft's own `windows` crate, already transitively present via tokio/mio. Requires COM already initialized on the calling thread (handled by the caller, `create_shortcuts`).
- **inputs**: `dest: &Path`, `target_exe: &str`
- **outputs**: `windows::core::Result<()>`
- **calls**: `CoCreateInstance`, `IShellLinkW::SetPath`, `.cast::<IPersistFile>()`, `IPersistFile::Save`, `to_wide`
- **called_by**: `create_shortcuts`
- **mutates**: filesystem (writes the `.lnk` file)

### `create_shortcuts`
- **type**: function
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: `#[cfg(all(windows, feature = "embed-dashboard"))]`. Writes Desktop + Start Menu shortcuts pointing at `target_exe`. Brackets both `write_shortcut` calls in one `CoInitializeEx`/`CoUninitialize` pair. Distinguishes three `CoInitializeEx` outcomes rather than a plain success/failure check: `S_OK`/`S_FALSE` mean this call owns a COM reference and must call `CoUninitialize`; `RPC_E_CHANGED_MODE` means COM was already initialized on this (tokio-worker, potentially reused) thread under a *different* concurrency model by something else in the process — no reference was acquired, so `CoUninitialize` must NOT be called, but the existing apartment is still usable so shortcut creation proceeds rather than aborting. Any other failure HRESULT aborts. Returns a human-readable log fragment (`"shortcut ok: <path>"` / `"shortcut FAILED: <path> — <windows::core::Error>"` per attempted shortcut) instead of discarding each result — the binary runs with `windows_subsystem = "windows"` (no console), so this is the only way a real user could ever see a failure.
- **inputs**: `target_exe: &Path`
- **outputs**: `String` (log fragment)
- **calls**: `dirs::desktop_dir`, `CoInitializeEx`, `write_shortcut`, `CoUninitialize`
- **called_by**: `maybe_self_install`
- **mutates**: filesystem (Desktop + Start Menu `.lnk` files)

### `maybe_self_install`
- **type**: function
- **file**: `crates/hsip-api/src/main.rs`
- **purpose**: `#[cfg(all(windows, feature = "embed-dashboard"))]`. If not already running from `%LOCALAPPDATA%\HSIP\hsip.exe`: creates that directory, copies the current exe there, calls `create_shortcuts` (whether or not the copy succeeded, as long as the installed exe is present — the exe may be locked because HSIP is already running), launches the installed copy (only if freshly copied, to avoid spawning a second server), and exits this process. Writes `install.log` in the install directory — the only place a real user can see what happened, since there's no console — folding in `create_shortcuts`'s per-shortcut log fragment rather than a second, differently-visible reporting path.
- **inputs**: none
- **outputs**: none
- **calls**: `std::env::current_exe`, `fs::copy`, `create_shortcuts`, `fs::write` (install.log), `std::process::Command::spawn`, `std::process::exit`
- **called_by**: `main`
- **mutates**: filesystem (`%LOCALAPPDATA%\HSIP\`, install.log, shortcuts via `create_shortcuts`), spawns a child process

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
- **purpose**: Zero-config startup: creates data dir, generates master key on first run (owner-only permissions — see `write_master_key_with_owner_only_permissions`), creates empty admin key placeholder, returns full Config.
- **inputs**: none
- **outputs**: `Result<Self>`
- **calls**: `hsip_data_dir`, `fs::create_dir_all`, `OsRng.fill_bytes`, `write_master_key_with_owner_only_permissions`, `fs::write`, `std::env::var`
- **called_by**: `run`
- **mutates**: filesystem (data dir, master.key, admin.key)

### `write_master_key_with_owner_only_permissions`
- **type**: function
- **file**: `crates/hsip-api/src/config.rs`
- **purpose**: Writes a freshly generated master key (hex-encoded) to `path` and restricts it to `0o600` on Unix immediately after writing. Found during a QA pass ("which secret eventually becomes public") that plain `fs::write` leaves a file at whatever the process umask allows — `0644`/world-readable on any default Unix umask, confirmed empirically — which meant `master.key` was readable by any other local user account or process on a shared host with zero HSIP-level compromise, unlike `admin.key`, which `main.rs::bootstrap_admin` already correctly restricts to `0o600`. Extracted as its own function specifically so it's directly unit-testable against a tempdir path without needing to mutate the process-global `HOME`/`APPDATA` env vars `hsip_data_dir()` reads.
- **inputs**: `path: &Path`, `raw: &[u8; 32]`
- **outputs**: `Result<()>`
- **calls**: `fs::write`, `fs::set_permissions` (Unix only)
- **called_by**: `Config::desktop_defaults`
- **mutates**: filesystem

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
- **called_by**: only `anchor_job.rs`'s two background-job cycles call this directly with `?` (`run_anchor_cycle_with_calendars`, `run_audit_anchor_cycle_with_calendars`) — correctly, since there's no live HTTP caller for a failure to mislead there; the periodic loop already logs and retries next tick. Every route-handler call site goes through `record_best_effort` below instead, as of §4.27's sweep (`routes::decisions::record` was the last holdout, using `?` directly, until the concurrency test below found the bug that motivated fixing it).
- **mutates**: `audit_entries` table

### `record_best_effort`
- **type**: function (async)
- **file**: `crates/hsip-api/src/audit_log.rs`
- **purpose**: Wraps `record` for every route-handler call site — 23 in total — where the state-changing operation an audit entry describes has already committed by the time this runs, so failing the whole request over a downstream audit-write hiccup would be wrong (the real action already succeeded) and these sites can't propagate the error with `?` the way `anchor_job.rs`'s background-job callers of `record` do. Built in two passes: §4.26 found 9 sites using the *silent* wrong shape (`let _ = record(...).await;` — no logging, no metric, found asking "what cannot currently be observed"); §4.27 found 13 more using the *loud-but-wrong* shape (`record(...).await?` — turning an already-successful operation into a confusing `500`, found only once this codebase had a genuinely concurrent test to reveal the failure). Logs via `tracing::error!` (tenant_id, action, underlying error) and increments `metrics::AUDIT_WRITE_FAILURES{action}` on failure instead of either discarding or propagating the `Result`.
- **inputs**: same as `record`, except `action: &'static str` (always one of a small hardcoded set — safe as a metric label, unlike caller-supplied free text)
- **outputs**: `()`
- **calls**: `record`, `metrics::AUDIT_WRITE_FAILURES`, `tracing::error!`
- **called_by**: `routes::admin::{rotate_master_key, grant_root_admin, revoke_root_admin}`, `routes::keys::{create, revoke, bind_client_cert}`, `auth::check_agent_velocity`, `routes::decisions::record`, `routes::consent::{grant, revoke}`, `routes::credentials::{issue, verify, revoke}`, `routes::identity::{create_or_get, rotate}`, `routes::messages::{sign, verify}`, `routes::sandbox::provision`, `routes::trust::{add, remove, verify}`
- **mutates**: `audit_entries` table (via `record`); `metrics::AUDIT_WRITE_FAILURES` on failure

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
- **purpose**: Implements `FromRequestParts`: extracts Bearer token, hashes it, looks up in DB (now also fetching `bound_client_cert_fingerprint`), checks active/expiry/pending-revocation, enforces the opt-in per-key mTLS client-certificate binding, opt-in replay protection, rate limit, and AI velocity check. The mTLS-binding check compares `bound_client_cert_fingerprint` (if set) against the connection's `mtls::ClientCertFingerprint` request extension (inserted by `mtls::ClientCertAcceptor` during a real TLS handshake) — mismatch or absent extension rejects with 401 and `metrics::AUTH_FAILURES{reason="client_cert_mismatch"}`.
- **inputs**: `parts: &mut Parts`, `state: &AppState`
- **outputs**: `Result<Self, ApiError>`
- **calls**: `hash_key`, `internal_db_error`, `check_replay_protection`, `check_rate_limit`, `check_agent_velocity`, `sqlx::query`
- **called_by**: Axum extractor machinery
- **mutates**: `rate_limiter` DashMap (inserts/updates window), `agent_tracker` DashMap, `replay_nonces` DashMap

### `internal_db_error`
- **type**: function
- **file**: `crates/hsip-api/src/auth.rs`
- **purpose**: Shared `sqlx::Error -> ApiError` mapper for the `TenantId` extractor's manual `.map_err(...)` call sites (these bypass `errors.rs`'s `From<sqlx::Error>` impl, so needed the same fix independently). Logs the real error via `tracing::error!` server-side and returns a fixed `ApiError::Internal("internal server error")` — never the raw `sqlx::Error` text — to the caller.
- **inputs**: `e: sqlx::Error`
- **outputs**: `ApiError`
- **calls**: `tracing::error!`
- **called_by**: `TenantId::from_request_parts` (4 sites)
- **mutates**: nothing

### `check_replay_protection`
- **type**: function
- **file**: `crates/hsip-api/src/auth.rs`
- **purpose**: Opt-in HTTP replay protection. No-op unless the caller sends both `x-hsip-timestamp` and `x-hsip-nonce`; if only one is present, rejects with 400. If both present, rejects with 401 when the timestamp is outside `replay_tolerance_secs()` of server time, or when the `(key_id, nonce)` pair has already been seen within that window (checked via `DashMap::entry` for atomic check-and-insert).
- **inputs**: `key_id: &str`, `parts: &Parts`, `state: &AppState`
- **outputs**: `Result<(), ApiError>`
- **calls**: `now_ms`, `replay_tolerance_secs`, `DashMap::entry`, `metrics::REPLAY_REJECTED`
- **called_by**: `TenantId::from_request_parts`
- **mutates**: `replay_nonces` DashMap (inserts new `(key_id, nonce)` entries with an expiry timestamp)

### `replay_tolerance_secs`
- **type**: function
- **file**: `crates/hsip-api/src/auth.rs`
- **purpose**: Reads `HSIP_REPLAY_TOLERANCE_SECS` env var (default 300s) — was previously a hardcoded `const` with no override, unlike `rate_limit_rpm` below, found and fixed via an architectural-resilience QA pass ("what if latency increases tenfold"): a fixed window with no way to widen it would reject legitimate requests outright if real-world latency or clock skew ever exceeded it, with no mitigation short of a code change. Doc comment carries an explicit operator caution: widening this value directly widens the replay window it exists to close, so it shouldn't be treated as a casual convenience knob.
- **inputs**: none
- **outputs**: `i64`
- **calls**: `std::env::var`
- **called_by**: `check_replay_protection`
- **mutates**: nothing

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
- **purpose**: Inline SQL migrations: creates all tables (tenants, api_keys, identities, consents, messages, audit_entries, contacts, credentials, trusted_peers, uploads, anchor_identity, decision_anchors, decisions, audit_anchors, rate_limit_state) and adds missing columns idempotently. `trusted_peers` (`id`, `tenant_id`, `label`, `verify_key`, `added_at BIGINT`, `UNIQUE(tenant_id, verify_key)`) — federated trust store for `routes/trust.rs` — was documented here and in `CLAUDE.md`'s schema table as if it already existed, but this line was aspirational until it was actually added: the table itself was missing from this function since the federated-trust feature shipped, so every `/v1/trust/*` call 500'd with "no such table" on any real database. Found while building the dashboard's Trust page; fixed by actually adding the `CREATE TABLE`. `rate_limit_state` (`kind`, `state_key`, `count`, `anomaly_count`, `window_start_ms`, `updated_at`, `PRIMARY KEY (kind, state_key)`) is a periodic snapshot of the in-memory rate-limit/AI-agent-velocity DashMaps — see `rate_limit_persistence.rs`. `anchor_identity` is a singleton row holding the node-level Ed25519 key used to sign anchored Merkle roots (distinct from any tenant identity) — shared by both decision and audit-log anchoring, not decision-specific. `decision_anchors` holds one row per RFC 6962 Merkle batch of decisions (root, signature, OpenTimestamps proof/status); `audit_anchors` is the identical shape for batches of `audit_entries` (see External Anchoring in `CLAUDE.md`). `decisions` holds AI-agent decision attestations; `UNIQUE(tenant_id, prev_hash)` serializes each tenant's hash chain against concurrent inserts. `decisions` also has a nullable `accountable_key_signature TEXT` column (same ignored-error `ALTER TABLE` pattern) — `NULL`/empty means `accountable_key` is pure caller-asserted metadata, unchanged; set means it's a base64 Ed25519 signature by `accountable_key`'s own private key, verified against `hsip_core::canonical::accountable_proof_preimage_hash` — see `routes/decisions.rs::verify_accountable_proof`. `audit_entries` has nullable `prev_hash`/`entry_hash` columns (added via `ALTER TABLE ... ADD COLUMN`, ignored-error pattern for upgrades) plus a `UNIQUE(tenant_id, prev_hash)` index (`idx_audit_chain`) that serializes the audit BLAKE3 hash chain against concurrent writers the same way `decisions` does — see `audit_log.rs`. `audit_entries` also has nullable `anchor_id`/`merkle_index` columns (same ignored-error `ALTER TABLE` pattern, plus `idx_audit_anchor` index) mirroring `decisions.anchor_id`/`merkle_index` — which `audit_anchors` batch (if any) an entry's `entry_hash` was folded into. `consents` has a nullable `granted_by_key_type` column (same ignored-error `ALTER TABLE` pattern) recording which kind of key (human/service/ai_agent) authorized the grant. `api_keys` has nullable `role` ('owner'\|'member') and `is_root_admin INTEGER NOT NULL DEFAULT 0` columns (same ignored-error `ALTER TABLE` pattern), plus a one-time backfill on upgrade: the earliest-created key in each tenant becomes `'owner'` if unset, every other unset key becomes `'member'`, and the key named `admin` in the very first tenant ever created becomes `is_root_admin=1` — preserving the pre-RBAC bootstrap-admin behavior exactly across an upgrade. Fresh installs get both columns set directly by `bootstrap_admin`'s `INSERT` instead, since that row doesn't exist yet when this backfill runs. `api_keys` also has a nullable `bound_client_cert_fingerprint TEXT` column (same ignored-error `ALTER TABLE` pattern) — `NULL` (the default) means the key authenticates on bearer token alone, unchanged; set (via `POST /v1/keys/:id/bind-client-cert`) means `auth.rs` additionally requires that exact TLS client-certificate fingerprint on the connection — see `mtls.rs`. `decisions` also has a nullable `issuer_verify_key TEXT` column (same ignored-error `ALTER TABLE` pattern) — the per-transaction derived signing key's public bytes (see `hsip_core::tx_key`); `NULL` means the row predates per-transaction key derivation and was signed directly with the tenant's root identity key, which `routes::decisions::proof` falls back to for those rows. New `submitted_receipts` table (`id`, `collector_tenant_id`, `submitter_label`, `receipt_type`, `source_tenant_id`, `source_record_id`, `bundle_json`, `valid`, `submitted_at BIGINT`, `UNIQUE(collector_tenant_id, receipt_type, source_tenant_id, source_record_id)`) — a "collector" node's inbox of already-verified proof bundles submitted by other, independent HSIP instances (see `routes/receipts.rs`); the `UNIQUE` constraint makes re-submitting the same receipt to the same collector a clean `409` instead of silent duplication.
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
- **purpose**: Every table in `db::run_migrations`'s schema, with its exact column list and types, driving the copy loop. **Not** discovered dynamically — a table added to `db.rs` without a matching entry here silently isn't migrated (documented inline and in `CLAUDE.md`'s Key Invariants). `api_keys`' column list includes `("bound_client_cert_fingerprint", Col::OptText)` — added alongside `db.rs`'s new column per the same invariant. `decisions`' column list includes `("accountable_key_signature", Col::OptText)`, same reasoning.
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

### `derive_field_encryption_key`
- **type**: function
- **file**: `crates/hsip-api/src/key_encryption.rs`
- **purpose**: Derives a 32-byte ChaCha20-Poly1305 key from the master key via HKDF-SHA256, using a distinct info string (`"hsip-field-encryption-v1"`) from `derive_encryption_key`'s (`"hsip-key-encryption-v1"`) — domain separation, so a field-encrypted value can never decrypt as a signing key (or vice versa) even under the same master key, and a bug specific to one call site's usage pattern can't be leveraged against the other.
- **inputs**: `master_key: &[u8]`
- **outputs**: `[u8; 32]`
- **calls**: `hkdf::Hkdf::new`, `expand`
- **called_by**: `encrypt_field`, `decrypt_field`
- **mutates**: nothing

### `encrypt_field` / `decrypt_field`
- **type**: function
- **file**: `crates/hsip-api/src/key_encryption.rs`
- **purpose**: Application-level field encryption at rest for `messages.content` and `credentials.claim`/`user_token` — same nonce(12)‖ciphertext+tag Base64 wire format as `encrypt_signing_key`/`decrypt_signing_key`, but for variable-length UTF-8 strings via the domain-separated `derive_field_encryption_key`. Chosen over SQLCipher/whole-database encryption because `sqlx::Any` spans both SQLite and PostgreSQL — a SQLite-specific extension wouldn't cover Postgres deployments, and this reuses the exact same primitive already used for signing keys with zero new dependencies. Deliberately does *not* cover `audit_entries.details` — that column is hashed as part of the BLAKE3 chain (`audit_log::compute_entry_hash`) and `GET /v1/audit/verify-proof`'s pure, DB-free, caller-supplied-plaintext contract would need a "hash the plaintext, encrypt only what's persisted" redesign not attempted in this pass.
- **inputs**: `plaintext: &str, master_key: &[u8]` / `encrypted_b64: &str, master_key: &[u8]`
- **outputs**: `String` / `anyhow::Result<String>`
- **calls**: `derive_field_encryption_key`, `OsRng`, `ChaCha20Poly1305::encrypt`/`decrypt`
- **called_by**: `routes::messages::{sign, list}`, `routes::credentials::{issue, list}`
- **mutates**: nothing

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

### `From<anyhow::Error> for ApiError` / `From<sqlx::Error> for ApiError`
- **type**: function (trait impl)
- **file**: `crates/hsip-api/src/errors.rs`
- **purpose**: Converts a caught `anyhow`/`sqlx` error (via `?`) into `ApiError`. Previously embedded the raw error's `Display` text directly into `ApiError::Internal`, which `into_response` sends verbatim to the HTTP caller — real DB/internal error detail (schema names, query fragments) leaking with no debug-only gate. Now logs the real error server-side via `tracing::error!` and returns a fixed `"internal server error"` message instead; `sqlx::Error::RowNotFound` still maps to a clean `ApiError::NotFound`, unchanged.
- **inputs**: `e: anyhow::Error` / `e: sqlx::Error`
- **outputs**: `ApiError`
- **calls**: `tracing::error!`
- **called_by**: every route handler's `?`-propagated `sqlx`/`anyhow` errors
- **mutates**: nothing (logs only)

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
- **type**: variable (static `Counter`, unlabeled)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Prometheus counter for verifiable credentials issued. Deliberately *not* labeled by `claim` — found during a QA pass that it previously was, which meant one permanent Prometheus time series per unique claim string ever issued (unbounded cardinality) and the claim's actual free-text content published to the unauthenticated-by-default `/metrics` endpoint. Fixed by dropping the label; the metric now answers only "how many total," which is the only aggregate it was ever actually used for.
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
- **type**: variable (static `Counter`, unlabeled)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Prometheus counter for messages signed via Ed25519. Deliberately *not* labeled by `tenant_id` — same fix and reasoning as `CREDENTIALS_ISSUED` above: a per-tenant label meant one permanent time series per tenant that ever signed a message (unbounded growth, worse with `HSIP_SANDBOX=true`'s free self-service tenant provisioning) and enumerated every tenant's UUID to anyone reaching the unauthenticated-by-default `/metrics` endpoint.
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
- **type**: variable (static `Counter`, unlabeled)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Prometheus counter for decision attestations recorded. Previously labeled by `decision_type` — dropped for the same reason as `CREDENTIALS_ISSUED`/`MESSAGES_SIGNED` above: `decision_type` is caller-supplied free text (up to 64 chars) with no enum constraint anywhere in this codebase, so it was equally unbounded-cardinality-unsafe as a label.
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

### `ANCHOR_UPGRADED_TO_CONFIRMED`
- **type**: variable (static `Counter`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Count of anchor batches (decisions or audit-log) upgraded from `ots_status = 'pending'` to `'confirmed'` — a calendar reported Bitcoin confirmation. Near-zero is fine (confirmation legitimately takes time); a batch stuck `pending` for a very long time without ever incrementing this is worth an operator noticing.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `anchor_job::upgrade_one_anchor`
- **mutates**: counter value

### `ANCHOR_UPGRADE_STALE`
- **type**: variable (static `Counter`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Count of anchor batches that exceeded `anchor_job::MAX_PENDING_UPGRADE_AGE_MS` (7 days) still `pending` and stopped being auto-polled. Should stay zero in normal operation; a rising count means calendars are failing to confirm submissions long-term. The underlying anchor data stays intact either way — this only tracks loss of *automatic* re-checking, not data loss.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `anchor_job::upgrade_one_anchor`
- **mutates**: counter value

### `SYSTEM_HEALTH_ISSUES`
- **type**: variable (static `GaugeVec`, label `severity`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Current count of unresolved `system_health::check` issues, by `severity` (`critical`\|`warning`). A gauge, not a counter — reflects state as of the last check, so it correctly drops back to zero once an issue resolves. The mechanism a business running real Prometheus alerting fires on (`hsip_system_health_issues{severity="critical"} > 0`) without needing to poll HSIP's own API.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `system_health::check_and_update_metrics`
- **mutates**: gauge values

### `AUDIT_WRITE_FAILURES`
- **type**: variable (static `CounterVec`, label `action`)
- **file**: `crates/hsip-api/src/metrics.rs`
- **purpose**: Failed writes to `audit_entries` at `audit_log::record_best_effort` call sites — see that function's doc comment. `action` is always one of a small, fixed set of hardcoded string literals from this codebase's own call sites (`key.created`, `master_key.rotated`, etc.), never caller-supplied free text, so it's safe as a label unlike `CREDENTIALS_ISSUED`'s former `claim` label or `MESSAGES_SIGNED`'s former `tenant` label (see those metrics' doc comments). Should be zero in normal operation; any nonzero value means an audit-trail entry is missing for an operation that otherwise succeeded.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `audit_log::record_best_effort`
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

### `ClientCertFingerprint`
- **type**: struct (`pub struct ClientCertFingerprint(pub Option<String>)`)
- **file**: `crates/hsip-api/src/mtls.rs`
- **purpose**: Per-connection request-extension value carrying the SHA-256 hex fingerprint of the client's presented TLS certificate, if any. Inserted into every request's `http::Extensions` by `ClientCertAcceptor`. `None` on a connection with no client certificate (plain TLS, or mTLS not configured); the extension is entirely absent on a connection `ClientCertAcceptor` never wrapped (the plain server-TLS or non-TLS path). Read by `auth.rs::TenantId::from_request_parts` to enforce a key's `bound_client_cert_fingerprint`, and directly by `routes::keys::bind_client_cert` to read the *caller's own* connection's fingerprint when binding.
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `ClientCertAcceptor::accept` (constructs it), `auth.rs::TenantId::from_request_parts` and `routes::keys::bind_client_cert` (read it via `parts.extensions.get`/`Extension` extractor)
- **mutates**: nothing

### `cert_fingerprint`
- **type**: function
- **file**: `crates/hsip-api/src/mtls.rs`
- **purpose**: SHA-256 hex digest of a certificate's raw DER bytes — the fingerprint format stored in `api_keys.bound_client_cert_fingerprint` and compared against on every request to a bound key.
- **inputs**: `cert: &CertificateDer<'_>`
- **outputs**: `String`
- **calls**: `sha2::Sha256::digest`, `hex::encode`
- **called_by**: `ClientCertAcceptor::accept`
- **mutates**: nothing

### `ClientCertAcceptor`
- **type**: struct + `impl Accept<I, S>`
- **file**: `crates/hsip-api/src/mtls.rs`
- **purpose**: Wraps `axum_server::tls_rustls::RustlsAcceptor` to additionally extract the client's presented certificate (already verified against `client_ca_path` by rustls itself during the handshake — this only reads what the handshake already decided) and make its fingerprint available to every request on that connection via `ClientCertFingerprint`. Delegates the actual TLS handshake to the inner `RustlsAcceptor` completely unchanged; only constructed when `client_ca_path` is configured (see `main.rs::run`) — the plain server-TLS path continues to use `axum_server::bind_rustls` directly, untouched, preserving the same "unaffected until an operator opts in" guarantee `build_rustls_config`'s `None` branch already makes.
- **inputs**: `stream: I`, `service: S` (via the `Accept` trait's `accept` method)
- **outputs**: `(Self::Stream, AddExtension<S, ClientCertFingerprint>)` wrapped in a boxed future
- **calls**: `RustlsAcceptor::accept`, `TlsStream::get_ref`, `ServerConnection::peer_certificates`, `cert_fingerprint`, `AddExtension::new`
- **called_by**: `main.rs::run` (constructed and bound via `axum_server::bind` + `.acceptor(...)` when `client_ca_path` is `Some`)
- **mutates**: nothing (wraps/delegates only)

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
- **purpose**: Builds the complete Axum router with all `/v1/*` routes, static file serving, and shared state. Includes `POST /v1/keys/:id/bind-client-cert` → `keys::bind_client_cert`. Includes `POST /v1/receipts/submit`, `GET /v1/receipts`, `GET /v1/receipts/:id` → `receipts::{submit, list, get_one}` — registered here per the standing invariant that a handler with no `.route(...)` line compiles fine and silently 404s.
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
- **purpose**: `POST /v1/messages/sign` — signs the request's plaintext content with the tenant's Ed25519 key (signing happens before encryption, against the request body, never the database), then encrypts `content` via `key_encryption::encrypt_field` right before the `INSERT` — an attacker with database read access alone can no longer read message content in plaintext. Increments MESSAGES_SIGNED counter, writes audit entry (only ever logs `peer_verify_key`, never `content`, so no leak-into-audit-log fix was needed here the way `credentials.rs` needed one).
- **inputs**: `State(state)`, `tenant`, `Json(req)`
- **outputs**: `ApiResult<Json<SignResponse>>`
- **calls**: `load_signing_key`, `ed25519_dalek::SigningKey::sign`, `key_encryption::encrypt_field`, `sqlx::query`, `audit_log::record`, `metrics::MESSAGES_SIGNED.inc`
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
- **purpose**: `GET /v1/messages` — returns the tenant's signed message history; decrypts each row's `content` via `key_encryption::decrypt_field` after `SELECT`, before returning to the authenticated caller.
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<Vec<MessageRecord>>>`
- **calls**: `sqlx::query_as`, `key_encryption::decrypt_field`
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
- **purpose**: `POST /v1/credentials` — signs the plaintext canonical JSON (unaffected by encryption — reads from the request, not the DB), then encrypts `claim`/`user_token` via `key_encryption::encrypt_field` before the `INSERT`. Increments CREDENTIALS_ISSUED metric. A real leak was found and fixed here: the audit-log call previously passed the plaintext `req.claim` as the `details` argument to `audit_log::record_best_effort`, meaning the exact plaintext the column encryption had just protected was landing unencrypted in `audit_entries.details` on every issue — fixed by logging the credential ID instead, matching what `revoke` already did correctly.
- **inputs**: `State(state)`, `tenant`, `Json(req)`
- **outputs**: `ApiResult<Json<IssueResponse>>`
- **calls**: `load_signing_key`, `canonical_json`, `ed25519_dalek::SigningKey::sign`, `key_encryption::encrypt_field`, `sqlx::query`, `audit_log::record_best_effort`, `metrics::CREDENTIALS_ISSUED.inc`
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
- **purpose**: `GET /v1/credentials` — returns all credentials for the tenant; decrypts `claim`/`user_token` via `key_encryption::decrypt_field` after `SELECT`.
- **inputs**: `State(state)`, `tenant`
- **outputs**: `ApiResult<Json<Vec<CredentialRecord>>>`
- **calls**: `sqlx::query_as`, `key_encryption::decrypt_field`
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

### `BindClientCertRequest` / `BindClientCertResponse`
- **type**: structs
- **file**: `crates/hsip-api/src/routes/keys.rs`
- **purpose**: `BindClientCertRequest` — JSON body for `POST /v1/keys/:id/bind-client-cert`: `clear: Option<bool>` (`true` removes an existing binding instead of setting one). `BindClientCertResponse` — the key's id and its resulting `bound_client_cert_fingerprint` (`None` after a clear or if binding failed validation).
- **inputs**: none
- **outputs**: none
- **calls**: none
- **called_by**: `bind_client_cert`
- **mutates**: nothing

### `bind_client_cert`
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/keys.rs`
- **purpose**: `POST /v1/keys/:id/bind-client-cert` — owner-role gated (same privilege boundary as `create`/`revoke` above). Binds `api_keys.bound_client_cert_fingerprint` on a key in the caller's own tenant to the fingerprint from the *caller's own current connection* (read from the `mtls::ClientCertFingerprint` request extension via `Option<Extension<...>>` — `Option` because a non-mTLS caller has no such extension at all), never an arbitrary caller-supplied string — this is what prevents an owner from bricking a key by binding it to a fingerprint nobody can ever present. Without a presented certificate and without `{"clear":true}`, returns `400 BadRequest`. `{"clear":true}` clears an existing binding instead (no certificate required for that path). Writes `key.cert_bound`/`key.cert_unbound` audit entries.
- **inputs**: `State(state)`, `tenant`, `headers`, `Path(key_id)`, `presented: Option<Extension<mtls::ClientCertFingerprint>>`, `Json(req)`
- **outputs**: `ApiResult<Json<BindClientCertResponse>>`
- **calls**: `resolve_caller_role`, `sqlx::query`, `audit_log::record`
- **called_by**: Axum router
- **mutates**: DB (`api_keys.bound_client_cert_fingerprint`, `audit_entries`)

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

### `system_health` (route)
- **type**: function (async)
- **file**: `crates/hsip-api/src/routes/admin.rs`
- **purpose**: `GET /v1/admin/system-health` — read-only, root-admin gated like every other node-level admin route. Aggregates conditions needing operator attention that HSIP can detect but not fix by itself (incomplete master key rotation, zero root admins, abandoned OTS anchors) — see THREAT_MODEL.md §4.22.
- **inputs**: `State(state)`, `_tenant: TenantId`, `headers: HeaderMap`
- **outputs**: `ApiResult<Json<system_health::SystemHealth>>`
- **calls**: `require_root_admin`, `system_health::check_and_update_metrics`
- **called_by**: Axum router
- **mutates**: `metrics::SYSTEM_HEALTH_ISSUES` gauge values (via `check_and_update_metrics`), nothing else

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
- **purpose**: `POST /v1/admin/master-key/rotate` — generates a new 32-byte master key, re-encrypts every `identities.signing_key_b64` row and the singleton `anchor_identity` row under it inside one DB transaction, then persists the new key via whichever `KeyPersistence` mode `resolve_persistence` returns (file: staging file write + `0o600` permission fix (Unix) + `fsync` + atomic rename; hook: `run_rotation_hook`) *before* committing the transaction, then swaps the in-memory key. Holds `state.master_key.write().await` for the *entire* operation (not just the final swap) — see the function's doc comment for the concurrency race this closes. Refuses with `ApiError::BadRequest` if `resolve_persistence` returns `None` (env-var-sourced key, no hook configured). Writes one `master_key.rotated` audit entry per tenant touched and increments `metrics::MASTER_KEY_ROTATIONS`. The staging file's permissions are fixed explicitly (rather than relying on the umask) because `rename()` on Unix preserves the *source* file's mode bits, not the destination's — without this, rotation would silently downgrade the master key file back to world-readable even on a host where the original file had correctly been `chmod 600`'d (found during the same QA pass as `config.rs::write_master_key_with_owner_only_permissions`).
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
- **purpose**: Request/response bodies for `POST /v1/decisions`. Response is the full signed receipt (`envelope`, `event_hash`, `signature`, `issuer_verify_key`, `accountable_key_verified`) meant to be persisted client-side (see SDK `save_receipt`). `RecordDecisionRequest.accountable_key_signature` (`Option<String>`, `#[serde(default)]`) is the optional proof-of-possession field — see `verify_accountable_proof` below.
- **called_by**: `record`

### `DecisionSummary`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: Row shape for `GET /v1/decisions` listing. `accountable_key_verified: bool` — whether `accountable_key_signature` was supplied and verifies, derived from whether the stored `decisions.accountable_key_signature` column is non-empty. `agent_key_id: String` (the `api_keys.id` of the connection that recorded this decision — distinct from `accountable_key`, which is caller-asserted and may be shared across several connections in the same tenant), `agent_name: Option<String>` (that connection's friendly name), and `agent_type: Option<String>` (`'human'|'service'|'ai_agent'`) — all three `None`/absent-friendly-name for a since-revoked key — let a caller answer "which named agent, of what kind, did this" without a second lookup. Added so the dashboard's Decisions views could show/filter by agent and render a plain "AI agent" vs "Human" label instead of just a raw key.
- **called_by**: `list`

### `ListDecisionsQuery`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: Query-param extractor for `GET /v1/decisions`: `agent_key_id: Option<String>` (narrow to one connection), `since_ms`/`until_ms: Option<i64>` (epoch-ms time window, matching `decisions.created_at`'s own storage format). All optional — omitting all three preserves the prior unfiltered-list behavior exactly.
- **called_by**: `list`

### `ProofStepDto`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: Wire format (hex hash + "left"/"right") for `hsip_core::merkle::ProofStep`. `From`/`TryFrom` impls convert to/from the core type.
- **called_by**: `proof`, `verify`

### `DecisionProofBundle`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: Full self-contained verification bundle returned by `GET /v1/decisions/:id/proof` — everything a third party needs to independently verify authorship and (once anchored) tamper-evidence, with zero further calls to this server. `accountable_key_verified: bool` is re-derived (not merely copied from a stored flag) via `verify_accountable_proof` on every call.
- **called_by**: `proof`

### `VerifyDecisionRequest` / `VerifyDecisionResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: Request/response for `POST /v1/decisions/verify`. `VerifyDecisionResponse.accountable_key_verified: Option<bool>` — `None` when `envelope.accountable_key_signature` is empty (nothing claimed, doesn't invalidate the bundle), `Some(bool)` otherwise (`Some(false)` does invalidate it, folded into `valid` the same way `merkle_inclusion_valid`/`anchor_signature_valid` already are).
- **called_by**: `verify`

### `verify_accountable_proof`
- **type**: function
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: Checks a claimed `accountable_key_signature` against `accountable_key` over `hsip_core::canonical::accountable_proof_preimage_hash`. Returns `None` when no signature was supplied at all (empty string), `Some(bool)` otherwise — including `Some(false)` for a malformed (non-base64, wrong length) signature or key, since a claimed-but-garbage proof is a real verification failure, not "nothing to check." Single call site for this check — `record()` (verify before persisting), `proof()` (re-derive for the bundle), and `verify()` (the independent, DB-free re-check) all call it, so the logic can't drift between "checked at write time" and "checked by a third party."
- **inputs**: `accountable_key_b64: &str`, `accountable_key_signature: &str`, `tenant_id: &str`, `model_version: &str`, `strategy_id: &str`, `decision_type: &str`, `payload_hash: &str`
- **outputs**: `Option<bool>`
- **calls**: `hsip_core::canonical::accountable_proof_preimage_hash`, `ed25519_dalek::VerifyingKey::verify`
- **called_by**: `record`, `proof`, `verify`
- **mutates**: nothing

### `record`
- **type**: function (async handler)
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: `POST /v1/decisions` — resolves the authenticated `api_keys` row, validates fields, verifies `accountable_key_signature` if supplied (rejecting the whole request with `400` before any DB write if a *claimed* signature doesn't verify — never silently records it as unverified), builds a `DecisionEnvelope` chained to the tenant's last decision (`prev_hash`). Signs `event_hash` with a fresh, single-use key derived per decision via `hsip_core::tx_key::derive_transaction_signing_key(root_seed, tenant_id, decision_id)` — **not** the tenant's static root identity key directly — derived inside the `MAX_ATTEMPTS` retry loop since each retry generates a fresh `decision_id` needing its own derived key. Persists the derived public key as `decisions.issuer_verify_key`. Retries on `UNIQUE(tenant_id, prev_hash)` conflict up to `MAX_ATTEMPTS` (another request extended the chain first). Writes `decision.recorded` audit entry via `audit_log::record_best_effort` (found via a genuinely-concurrent test, `test_concurrent_decision_writes_do_not_fork_the_chain` — the original `.await?` on this already-committed write turned a downstream audit-write failure under real contention into a confusing `500` for a decision that had, in fact, already been recorded), increments `DECISIONS_RECORDED`.
- **inputs**: `State(state)`, `tenant: TenantId`, `headers: HeaderMap`, `Json(req): Json<RecordDecisionRequest>`
- **outputs**: `ApiResult<Json<RecordDecisionResponse>>`
- **calls**: `load_signing_key`, `hsip_core::canonical::event_hash`, `hsip_core::tx_key::derive_transaction_signing_key`, `verify_accountable_proof`, `ms_to_iso`, `hash_key`, sqlx queries, `audit_log::record_best_effort`, `audit_log::chain_retry_backoff`, `metrics::CHAIN_WRITE_RETRIES`
- **called_by**: Axum router
- **mutates**: DB (`decisions`, `audit_entries`)

### `list` (decisions)
- **type**: function (async handler)
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: `GET /v1/decisions` — lists the tenant's decisions, newest first, optionally narrowed by `ListDecisionsQuery` (`?agent_key_id=`, `?since_ms=`, `?until_ms=`). `LEFT JOIN`s `api_keys` on `agent_key_id` to resolve each row's `agent_name`/`agent_type` in the same query rather than a per-row lookup; a since-revoked connection still lists its past decisions, just with both `null`. Each optional filter is expressed as `($N IS NULL OR col = $N)` in one fixed query rather than building SQL dynamically, so it still works identically on both `sqlx::Any` backends.
- **inputs**: `State(state)`, `tenant: TenantId`, `Query(filter): Query<ListDecisionsQuery>`
- **outputs**: `ApiResult<Json<Vec<DecisionSummary>>>`
- **called_by**: Axum router
- **mutates**: nothing

### `proof`
- **type**: function (async handler)
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: `GET /v1/decisions/:id/proof` — builds the full proof bundle. Reads the decision's stored `issuer_verify_key` (the per-transaction derived key — see `record`), falling back to the tenant's root `identities.verify_key_b64` for `NULL` rows that predate per-transaction key derivation. If unanchored, returns `anchored: false` with signature-only proof. If anchored, reconstructs the batch's leaf set from `decisions.anchor_id` ordered by `merkle_index`, rebuilds the `MerkleTree`, regenerates the inclusion proof, and defensively re-checks the recomputed root against the stored `decision_anchors.merkle_root`. Also re-derives `accountable_key_verified` via `verify_accountable_proof` on every call rather than trusting a stored flag.
- **inputs**: `State(state)`, `tenant: TenantId`, `Path(id)`
- **outputs**: `ApiResult<Json<DecisionProofBundle>>`
- **calls**: `hsip_core::merkle::MerkleTree::from_leaves`, `MerkleTree::inclusion_proof`, `verify_accountable_proof`
- **called_by**: Axum router
- **mutates**: nothing

### `verify`
- **type**: function (async handler)
- **file**: `crates/hsip-api/src/routes/decisions.rs`
- **purpose**: `POST /v1/decisions/verify` — pure verification of a self-contained bundle. Deliberately takes no `TenantId` and no `State`; makes no database call. Recomputes `event_hash` from the disclosed envelope, verifies the Ed25519 signature, verifies RFC 6962 inclusion and the anchor signature if those fields are present, and independently re-checks `envelope.accountable_key_signature` via `verify_accountable_proof`. This is the function meant to be run independently of HSIP entirely.
- **inputs**: `Json(req): Json<VerifyDecisionRequest>`
- **outputs**: `Json<VerifyDecisionResponse>`
- **calls**: `hsip_core::canonical::event_hash`, `hsip_core::merkle::verify_inclusion`, `anchor_job::verify_anchor_signature`, `verify_accountable_proof`
- **called_by**: Axum router
- **mutates**: nothing

---

## `crates/hsip-api/src/anchor.rs`

OpenTimestamps calendar HTTP client — network I/O only, no DB. See module
docs for MVP scope (opaque blob storage, no full `.ots` Merkle-path parsing)
and THREAT_MODEL.md §4.20/§4.21 for real-network submission and
upgrade-polling verification. Upgrade polling (checking whether a pending
submission has since been Bitcoin-confirmed) is implemented as of §4.21 —
see `check_for_upgrade`/`contains_bitcoin_attestation`/
`extract_pending_calendar_uri` below.

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

### `PENDING_ATTESTATION_TAG` / `BITCOIN_ATTESTATION_TAG`
- **type**: variable (const `[u8; 8]`)
- **file**: `crates/hsip-api/src/anchor.rs`
- **purpose**: OpenTimestamps' protocol-defined byte tags marking a `PendingAttestation` (calendar has it queued, not yet in Bitcoin) vs. `BitcoinBlockHeaderAttestation` (confirmed by a mined block) inside a serialized proof. Matches the reference Python implementation (`opentimestamps/core/notary.py`); the pending tag confirmed empirically against a real captured calendar response (THREAT_MODEL.md §4.20/§4.21).
- **called_by**: `contains_bitcoin_attestation`, `extract_pending_calendar_uri`

### `contains_bitcoin_attestation`
- **type**: function
- **file**: `crates/hsip-api/src/anchor.rs`
- **purpose**: Whether a calendar's response contains a `BitcoinBlockHeaderAttestation` tag — i.e. this submission is Bitcoin-confirmed. A tag-presence check, not a full Merkle-path verification (documented MVP scope, same trust level as the initial "pending" submission already had).
- **inputs**: `proof_bytes: &[u8]`
- **outputs**: `bool`
- **called_by**: `anchor_job::upgrade_one_anchor`

### `read_varuint`
- **type**: function (private)
- **file**: `crates/hsip-api/src/anchor.rs`
- **purpose**: Reads an OpenTimestamps-style base-128 varint (7 payload bits/byte, high bit = continuation — same scheme as protobuf/LEB128).
- **inputs**: `bytes: &[u8]`, `pos: &mut usize`
- **outputs**: `Option<u64>`
- **called_by**: `extract_pending_calendar_uri`

### `extract_pending_calendar_uri`
- **type**: function
- **file**: `crates/hsip-api/src/anchor.rs`
- **purpose**: Reads the originating calendar's URL back out of a stored `PendingAttestation` proof — `decision_anchors`/`audit_anchors` don't have a separate calendar-URL column, since the URI is already embedded in what's stored at submission time. Layout (tag, outer length varint, inner length varint, UTF-8 URI) confirmed against a real captured calendar response.
- **inputs**: `proof_bytes: &[u8]`
- **outputs**: `Option<String>`
- **calls**: `read_varuint`
- **called_by**: `anchor_job::upgrade_one_anchor`

### `check_for_upgrade`
- **type**: function (async)
- **file**: `crates/hsip-api/src/anchor.rs`
- **purpose**: `GET <calendar>/timestamp/<hex-digest>` — asks a calendar whether a previously-submitted digest has since been Bitcoin-confirmed, per the real OpenTimestamps calendar HTTP protocol. `Ok(None)` for "nothing new yet" (any non-success response, including simply not upgraded yet) is the expected common case, not an error; `Err` only on genuine unreachability.
- **inputs**: `calendar_url: &str`, `digest: &[u8; 32]`
- **outputs**: `Result<Option<Vec<u8>>>`
- **calls**: `reqwest::Client`
- **called_by**: `anchor_job::upgrade_one_anchor`
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
- **purpose**: Re-attempts OpenTimestamps submission for anchors stuck at `ots_status = 'calendar_unreachable'`. Best-effort — logs and moves on if a retry fails again, incrementing `metrics::ANCHOR_CALENDAR_UNREACHABLE` so the dependency's degraded state is visible over time, not just per-anchor. Fixed (THREAT_MODEL.md §4.22): previously discarded the corrective `UPDATE`'s result via `let _ = ...` and logged "succeeded" unconditionally; now checks `rows_affected() > 0` and logs a warning instead if the write didn't actually land.
- **inputs**: `db: &Db`, `calendars: &[&str]`
- **outputs**: none
- **calls**: `anchor::submit_digest_to`, `metrics::ANCHOR_CALENDAR_UNREACHABLE`
- **called_by**: `run_anchor_cycle_with_calendars`
- **mutates**: DB (`decision_anchors.ots_proof`/`ots_status` on genuine success)

### `retry_pending_audit_ots_submissions`
- **type**: function (async)
- **file**: `crates/hsip-api/src/anchor_job.rs`
- **purpose**: Twin of `retry_pending_ots_submissions` against `audit_anchors` instead of `decision_anchors`. Same silent-failure fix applied.
- **inputs**: `db: &Db`, `calendars: &[&str]`
- **outputs**: none
- **calls**: `anchor::submit_digest_to`, `metrics::ANCHOR_CALENDAR_UNREACHABLE`
- **called_by**: `run_audit_anchor_cycle_with_calendars`
- **mutates**: DB (`audit_anchors.ots_proof`/`ots_status` on genuine success)

### `verify_anchor_signature`
- **type**: function
- **file**: `crates/hsip-api/src/anchor_job.rs`
- **purpose**: Verifies an Ed25519 signature over a Merkle root against a given verify key. Pure — no DB. Generic over what was anchored, so both decisions and audit-log verification reuse it.
- **inputs**: `root: &[u8; 32]`, `signature: &[u8; 64]`, `verify_key: &[u8; 32]`
- **outputs**: `bool`
- **called_by**: `routes::decisions::verify`, `routes::audit::verify_proof`

### `run_upgrade_cycle`
- **type**: function (async)
- **file**: `crates/hsip-api/src/anchor_job.rs`
- **purpose**: Public entry point for OpenTimestamps "upgrade" polling (THREAT_MODEL.md §4.21) — checks every `decision_anchors`/`audit_anchors` row still at `ots_status = 'pending'` and flips confirmed ones to `'confirmed'`. Meant to run on its own, much slower timer than the anchor-submission loop — Bitcoin blocks land roughly every 10 minutes on average.
- **inputs**: `db: &Db`
- **outputs**: none
- **calls**: `upgrade_pending_decision_anchors`, `upgrade_pending_audit_anchors`
- **called_by**: `main.rs`'s spawned 15-minute upgrade-poll loop; integration tests call this directly against a mock calendar

### `MAX_UPGRADE_CHECKS_PER_CYCLE`
- **type**: variable (const `i64`, value `25`)
- **file**: `crates/hsip-api/src/anchor_job.rs`
- **purpose**: Per-cycle cap on how many `pending` anchor rows `upgrade_pending_decision_anchors`/`upgrade_pending_audit_anchors` check. Added after a QA edge-case pass found the original unbounded query could, under a large backlog, make one 15-minute cycle's sequential calendar checks (each with a 15s timeout) take longer than the gap before the next cycle. Verified by `tests/integration.rs::test_upgrade_cycle_caps_checks_per_run` (30 pending rows seeded, asserts exactly 25 calendar requests).
- **called_by**: `upgrade_pending_decision_anchors`, `upgrade_pending_audit_anchors`

### `MAX_PENDING_UPGRADE_AGE_MS`
- **type**: variable (const `i64`, value `7 * 24 * 60 * 60 * 1000` — 7 days)
- **file**: `crates/hsip-api/src/anchor_job.rs`
- **purpose**: Outer bound on how long a still-`pending` anchor row keeps being auto-checked. Added after a QA edge-case pass found that, without it, a batch whose calendar never confirms would be re-checked every 15 minutes for the server's entire operational lifetime. Rows older than this stop being auto-polled (the anchor data itself stays fully valid — signature and Merkle proof still verify — it just isn't auto-upgraded further); `metrics::ANCHOR_UPGRADE_STALE` tracks how many have crossed it. Verified by `tests/integration.rs::test_stale_pending_anchor_is_not_auto_polled` (an 8-day-old row against a calendar that would confirm immediately if asked — asserts zero requests were made).
- **called_by**: `upgrade_one_anchor`

### `upgrade_pending_decision_anchors` / `upgrade_pending_audit_anchors`
- **type**: function (async, private)
- **file**: `crates/hsip-api/src/anchor_job.rs`
- **purpose**: Queries `decision_anchors`/`audit_anchors` for up to `MAX_UPGRADE_CHECKS_PER_CYCLE` rows at `ots_status = 'pending'`, oldest (`created_at ASC`) first, and delegates each to `upgrade_one_anchor`. Twins, same shape as `retry_pending_ots_submissions`/`retry_pending_audit_ots_submissions`.
- **inputs**: `db: &Db`
- **outputs**: none
- **calls**: `upgrade_one_anchor`
- **called_by**: `run_upgrade_cycle`

### `upgrade_one_anchor`
- **type**: function (async, private)
- **file**: `crates/hsip-api/src/anchor_job.rs`
- **purpose**: Shared upgrade-check logic for one anchor row (decision or audit). First checks the row's age against `MAX_PENDING_UPGRADE_AGE_MS`, skipping (and incrementing `metrics::ANCHOR_UPGRADE_STALE`) without any network call if it's past that bound. Otherwise reads the originating calendar's URL back out of the row's stored `ots_proof` (`anchor::extract_pending_calendar_uri` — no dedicated calendar-URL column), calls `anchor::check_for_upgrade` against it, and on a detected `BitcoinBlockHeaderAttestation` (`anchor::contains_bitcoin_attestation`) updates that row's `ots_proof`/`ots_status = 'confirmed'`. `table` is always a hardcoded literal from this module (never external input) — same no-injection-risk reasoning already applied to `bin/hsip_migrate.rs`'s table-driven copy. Fixed (THREAT_MODEL.md §4.22): previously discarded the `UPDATE`'s result via `let _ = ...` and logged/counted success unconditionally; now checks `rows_affected() > 0` before declaring success, logging a warning instead on a zero-rows or failed update. Verified by `zero_rows_affected_is_not_counted_as_a_successful_upgrade` — fetches a row, deletes it, then calls this function with the already-fetched row to force a genuine zero-rows-affected `UPDATE`.
- **inputs**: `db: &Db`, `table: &'static str`, `row: &AnyRow`
- **outputs**: none
- **calls**: `anchor::extract_pending_calendar_uri`, `anchor::check_for_upgrade`, `anchor::contains_bitcoin_attestation`, `metrics::ANCHOR_UPGRADED_TO_CONFIRMED`, `metrics::ANCHOR_UPGRADE_STALE`
- **called_by**: `upgrade_pending_decision_anchors`, `upgrade_pending_audit_anchors`
- **mutates**: DB (`decision_anchors`/`audit_anchors` `ots_proof`/`ots_status` on a genuinely confirmed upgrade)

---

## `crates/hsip-api/src/routes/receipts.rs`

Receipt collection — lets a business run HSIP purely locally on every employee's/agent's own machine and still get one centralized audit trail, without a shared database holding everyone's raw operational data. A "collector" is just an ordinary HSIP instance/tenant whose operator accepts `POST /v1/receipts/submit` calls from other, independent instances. Each submission is a self-contained proof bundle — exactly what `GET /v1/decisions/:id/proof` / `GET /v1/audit/:id/proof` already return on the *submitting* instance — never the actual decision payload or any private key material. The collector independently re-verifies every submission using the same DB-free verification logic a third party would run before ever storing it.

### `SubmitReceiptRequest` / `SubmitReceiptResponse`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/receipts.rs`
- **purpose**: Request: `submitter_label` (caller-supplied, informational only — not a verified identity claim), `receipt_type` (`"decision"` \| `"audit"`), `bundle` (the verbatim proof bundle JSON from the submitting instance's own proof endpoint). Response: `id`, `valid`, `source_tenant_id`, `source_record_id`.
- **called_by**: `submit`

### `ReceiptSummary` / `ReceiptDetail`
- **type**: struct
- **file**: `crates/hsip-api/src/routes/receipts.rs`
- **purpose**: `ReceiptSummary` — list-view row (no bundle body). `ReceiptDetail` — full stored record including the original `bundle` JSON.
- **called_by**: `list`, `get_one`

### `submit`
- **type**: function (async handler)
- **file**: `crates/hsip-api/src/routes/receipts.rs`
- **purpose**: `POST /v1/receipts/submit` — deserializes the caller-supplied `bundle` into the exact same `VerifyDecisionRequest`/`VerifyAuditProofRequest` shape `routes::decisions::verify`/`routes::audit::verify_proof` already accept, and calls those functions **directly as ordinary async functions** (same process, not over HTTP) to independently re-verify before ever storing. A bundle that fails to deserialize, or verifies `false`, is rejected `400` and never reaches the `INSERT`. Duplicate submission (same `collector_tenant_id`/`receipt_type`/`source_tenant_id`/`source_record_id`) hits the table's `UNIQUE` constraint, mapped to a clean `409 Conflict`. Writes `receipt.submitted` audit entry via `audit_log::record_best_effort`.
- **inputs**: `State(state)`, `tenant: TenantId`, `Json(req): Json<SubmitReceiptRequest>`
- **outputs**: `ApiResult<Json<SubmitReceiptResponse>>`
- **calls**: `routes::decisions::verify`, `routes::audit::verify_proof`, `sqlx::query`, `audit_log::record_best_effort`
- **called_by**: Axum router
- **mutates**: DB (`submitted_receipts`, `audit_entries`)

### `list` (receipts)
- **type**: function (async handler)
- **file**: `crates/hsip-api/src/routes/receipts.rs`
- **purpose**: `GET /v1/receipts` — summaries only, newest first, scoped to the calling collector tenant.
- **inputs**: `State(state)`, `tenant: TenantId`
- **outputs**: `ApiResult<Json<Vec<ReceiptSummary>>>`
- **calls**: `sqlx::query`
- **called_by**: Axum router
- **mutates**: nothing

### `get_one` (receipts)
- **type**: function (async handler)
- **file**: `crates/hsip-api/src/routes/receipts.rs`
- **purpose**: `GET /v1/receipts/:id` — full detail including the original bundle, for a deeper audit.
- **inputs**: `State(state)`, `tenant: TenantId`, `Path(id)`
- **outputs**: `ApiResult<Json<ReceiptDetail>>`
- **calls**: `sqlx::query`
- **called_by**: Axum router
- **mutates**: nothing

---

## `crates/hsip-api/src/system_health.rs`

Aggregates conditions needing a human operator's attention that the rest of
this codebase can detect but cannot fix by itself — see THREAT_MODEL.md
§4.22. Backs `GET /v1/admin/system-health`, `metrics::SYSTEM_HEALTH_ISSUES`,
and `hsip status`'s health section.

### `HealthIssue`
- **type**: struct
- **file**: `crates/hsip-api/src/system_health.rs`
- **purpose**: One detected issue: `code`, `severity` (`"critical"`\|`"warning"`), `summary`, `detail`.
- **called_by**: `check`, `SystemHealth`

### `SystemHealth`
- **type**: struct
- **file**: `crates/hsip-api/src/system_health.rs`
- **purpose**: `check`'s result: `healthy` (`true` iff `issues` is empty), `checked_at_ms`, `issues: Vec<HealthIssue>`. Serialized directly as `GET /v1/admin/system-health`'s response body.
- **called_by**: `routes::admin::system_health`

### `check`
- **type**: function (async)
- **file**: `crates/hsip-api/src/system_health.rs`
- **purpose**: Runs all three checks and returns the aggregated `SystemHealth`. Deliberately pure — no metric or logging side effects — so it's directly unit-testable without resetting global metric state between tests.
- **inputs**: `db: &Db`, `master_key_path: Option<&str>`
- **outputs**: `SystemHealth`
- **calls**: `check_master_key_rotation_incomplete`, `check_zero_root_admins`, `check_abandoned_ots_anchors`
- **called_by**: `check_and_update_metrics`

### `check_and_update_metrics`
- **type**: function (async)
- **file**: `crates/hsip-api/src/system_health.rs`
- **purpose**: Calls `check`, then refreshes `metrics::SYSTEM_HEALTH_ISSUES` (by severity) so `/metrics` reflects the current state.
- **inputs**: `db: &Db`, `master_key_path: Option<&str>`
- **outputs**: `SystemHealth`
- **calls**: `check`, `metrics::SYSTEM_HEALTH_ISSUES`
- **called_by**: `routes::admin::system_health`, `main.rs`'s spawned 5-minute health-refresh loop
- **mutates**: `metrics::SYSTEM_HEALTH_ISSUES` gauge values

### `check_master_key_rotation_incomplete`
- **type**: function
- **file**: `crates/hsip-api/src/system_health.rs`
- **purpose**: Critical issue if `{master_key_path}.rotating` still exists on disk — the staging file `routes::admin::rotate_master_key` deliberately leaves behind if it crashes between committing the DB under a new key and renaming the staging file onto the real path.
- **inputs**: `master_key_path: Option<&str>`
- **outputs**: `Option<HealthIssue>`
- **called_by**: `check`

### `check_zero_root_admins`
- **type**: function (async)
- **file**: `crates/hsip-api/src/system_health.rs`
- **purpose**: Critical issue if `COUNT(*) FROM api_keys WHERE is_root_admin = 1 AND active = 1` is zero — a state the grant/revoke API already refuses to allow, but direct database tampering could still cause, with no recovery path except editing `api_keys` directly.
- **inputs**: `db: &Db`
- **outputs**: `Option<HealthIssue>`
- **called_by**: `check`

### `check_abandoned_ots_anchors` / `count_stale_pending`
- **type**: function (async)
- **file**: `crates/hsip-api/src/system_health.rs`
- **purpose**: Warning issue if any `decision_anchors`/`audit_anchors` rows have exceeded `anchor_job::MAX_PENDING_UPGRADE_AGE_MS` still `ots_status = 'pending'` — mirrors what makes `metrics::ANCHOR_UPGRADE_STALE` increment in `anchor_job.rs`, surfaced here as a queryable issue instead of only a counter.
- **inputs**: `db: &Db`
- **outputs**: `Option<HealthIssue>` (`check_abandoned_ots_anchors`), `i64` (`count_stale_pending`)
- **calls**: `count_stale_pending` (called twice, once per table)
- **called_by**: `check`

---

## `crates/hsip-cli/src/main.rs`

### `Commands`
- **type**: enum
- **file**: `crates/hsip-cli/src/main.rs`
- **purpose**: Top-level clap subcommand enum: Keygen, Init, Export, Import, Consent, Session, Token, Discover, Reputation, Daemon, Audit, Agent, Trust, Keys, Up, Status, Diag, Receipts.
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

### `SystemHealthResponse` / `HealthIssue` (cli)
- **type**: struct
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: Deserializes `GET /v1/admin/system-health`'s JSON response (`{healthy, issues: [{severity, summary, detail}]}`) for `status` to print.
- **called_by**: `status` (agent cli)

### `status` (agent cli)
- **type**: function
- **file**: `crates/hsip-cli/src/commands/agent.rs`
- **purpose**: `hsip status` — prints identity, active agents, and recent audit activity. Now also calls `GET /v1/admin/system-health` first and prints any issues loudly at the very top, before every other section — the individual-desktop-user answer to "how would I know something needs manual intervention" (THREAT_MODEL.md §4.22). A non-root-admin key gets a clear "unavailable, requires a root-admin key" line instead of the whole command failing.
- **inputs**: `api_url: Option<String>`, `key: Option<String>`
- **outputs**: `Result<()>`
- **calls**: `ApiClient::new`, `client.get` (`/v1/admin/system-health`, `/v1/identity`, `/v1/agents`, `/v1/audit`), `println!`
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
- **purpose**: `hsip up` — checks server health, starts it if down, ensures identity exists, opens dashboard in browser, prints welcome box followed by a federated-trust onboarding hint (`hsip status` to show your verify key, `hsip trust add` for peers to trust it).
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

## `crates/hsip-cli/src/commands/receipts.rs`

Client-side half of `routes::receipts` — `hsip receipts submit`. Fetches a local decision/audit proof bundle from *this* machine's own instance, then submits it to a remote "collector" using a separate, collector-scoped bearer key.

### `ReceiptsCmd`
- **type**: enum
- **file**: `crates/hsip-cli/src/commands/receipts.rs`
- **purpose**: `Submit { id, r#type ("decision" default or "audit"), label, collector_url, collector_key (env `HSIP_COLLECTOR_KEY`), api_url, key }` — the two credentials (`key` for this machine's own local instance, `collector_key` for the remote collector) are deliberately distinct, since they authenticate to two different machines.
- **called_by**: `main.rs`'s `Commands::Receipts` match arm

### `submit` (receipts CLI)
- **type**: function
- **file**: `crates/hsip-cli/src/commands/receipts.rs`
- **purpose**: Resolves local key via `--key`/`HSIP_API_KEY`/`util::load_admin_key()` same as every other command. `GET`s the proof bundle from `{local_base}/v1/decisions/:id/proof` or `/v1/audit/:id/proof` depending on `--type`, then `POST`s `{submitter_label, receipt_type, bundle}` to `{collector_url}/v1/receipts/submit` using the collector key. Prints the collector's confirmation (receipt ID, source tenant/record ID, verified status).
- **inputs**: `id: String`, `receipt_type: String`, `label: String`, `collector_url: String`, `collector_key: String`, `api_url: Option<String>`, `key: Option<String>`
- **outputs**: `Result<()>`
- **calls**: `util::load_admin_key`, `reqwest::blocking::Client`
- **called_by**: `run`
- **mutates**: nothing locally (network calls only)

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
- **purpose**: The signed envelope for one AI-agent decision attestation. Two-tier: `model_version`/`strategy_id`/`accountable_key`/`decision_type` are clear accountability metadata; `payload_hash` is an opaque SHA-256 of content HSIP never receives. `prev_hash` chains to the tenant's previous decision (empty string for the first). `timestamp_int` is kept as a string, not a JSON number, so canonicalization never risks IEEE-754-double precision loss on large timestamps. `accountable_key_signature` (`#[serde(default)]`, empty string when unsupplied) is a base64 Ed25519 signature by `accountable_key`'s own private key over `accountable_proof_preimage_hash(...)`, proving whoever submitted the decision actually holds that key rather than merely naming it — optional proof-of-possession, part of the canonical signed envelope so a third party re-running `verify()` sees exactly what was (or wasn't) claimed.
- **called_by**: `hsip-api`'s `routes::decisions::{record, proof, verify}`

### `canonical_bytes` / `event_hash`
- **type**: function
- **file**: `crates/hsip-core/src/canonical.rs`
- **purpose**: `canonical_bytes` serializes a `DecisionEnvelope` per RFC 8785 JCS (deterministic across implementations). `event_hash` is `SHA256(JCS(envelope))` — the value that gets Ed25519-signed and fed into the Merkle tree as leaf data.
- **inputs**: `envelope: &DecisionEnvelope`
- **outputs**: `Result<Vec<u8>, serde_json::Error>` / `Result<[u8; 32], serde_json::Error>`
- **calls**: `serde_jcs::to_vec`, `sha2::Sha256::digest`
- **called_by**: `hsip-api`'s `routes::decisions::{record, proof, verify}`

### `AccountableProofPreimage` / `accountable_proof_preimage_hash`
- **type**: struct (private) + function (public)
- **file**: `crates/hsip-core/src/canonical.rs`
- **purpose**: `AccountableProofPreimage` is the narrow set of fields `accountable_key`'s own signature attests to — deliberately smaller than `DecisionEnvelope`: only fields the caller can compute *before* submitting `POST /v1/decisions` (unlike `decision_id`/`prev_hash`/`timestamp_*`, server-assigned and possibly different across a hash-chain retry). `accountable_proof_preimage_hash` is `SHA256(JCS(AccountableProofPreimage))` — the exact bytes `accountable_key` must sign to produce `DecisionEnvelope::accountable_key_signature`. `tenant_id` is included specifically to bind the proof to one tenant on one deployment — without it, a real signature for one tenant's decision could be replayed by a different tenant reusing the same (non-secret) `{model_version, strategy_id, decision_type, payload_hash}` values. Single source of truth for this formula — `hsip-api`'s `routes::decisions::verify_accountable_proof` is the one place that calls it, in turn called identically by `record()`/`proof()`/`verify()`.
- **inputs**: `accountable_key: &str`, `tenant_id: &str`, `model_version: &str`, `strategy_id: &str`, `decision_type: &str`, `payload_hash: &str`
- **outputs**: `Result<[u8; 32], serde_json::Error>`
- **calls**: `serde_jcs::to_vec`, `sha2::Sha256::digest`
- **called_by**: `hsip-api`'s `routes::decisions::verify_accountable_proof`; Python SDK's `HSIPClient.accountable_proof_preimage_hash` is an independent reimplementation of the same formula (confirmed byte-identical output for identical input before being trusted)

---

## `crates/hsip-core/src/tx_key.rs`

Per-transaction signing-key derivation (HKDF-SHA256), pure — no I/O, same discipline as `merkle.rs`/`canonical.rs`. Raised directly as a design question: could decisions be signed with a key that "rotates into randomness" per transaction, impossible for an attacker to link or steal as one static master key, while still letting an authorized audit confirm every one of a tenant's transaction keys descends from that same tenant? The first framing of the idea (embedding a literal recognizable substring across otherwise-random-looking keys) was flagged before building anything — a fixed, attacker-visible substring is self-defeating (more linkable, not less, and shrinks actual entropy). The corrected approach is standard cryptographic key derivation, not tagging.

### `derive_transaction_signing_key`
- **type**: function
- **file**: `crates/hsip-core/src/tx_key.rs`
- **purpose**: HKDF-SHA256 with `tenant_id` as salt (domain separation between tenants) and `"hsip-tx-key-v1|" + transaction_id` as info (binds the derived key to exactly one transaction). Deterministically derives a fresh, single-use Ed25519 signing key per decision from the tenant's root seed. Two properties together: unlinkable to an outside observer (HKDF output is computationally indistinguishable from random without `root_seed` — two decisions from the same tenant produce unrelated-looking public keys on the wire), and re-derivable/verifiable by anyone holding `root_seed` (the tenant itself, or an auditor given temporary access). Does **not** make a tenant's own decisions unlinkable within HSIP's own database — `tenant_id` sits right next to every decision there regardless, for ordinary multi-tenant operation.
- **inputs**: `root_seed: &[u8; 32]`, `tenant_id: &str`, `transaction_id: &str`
- **outputs**: `ed25519_dalek::SigningKey`
- **calls**: `hkdf::Hkdf::new`, `expand`
- **called_by**: `hsip-api`'s `routes::decisions::record` (inside the `MAX_ATTEMPTS` chain-retry loop, since each retry generates a fresh `decision_id` needing its own derived key)

### `verify_transaction_key_derivation`
- **type**: function
- **file**: `crates/hsip-core/src/tx_key.rs`
- **purpose**: Audit-side check — re-derives the transaction key from the same inputs and compares its public bytes against a claimed verify key. This is the audit story the original design question asked for: proving a set of transaction keys all trace back to one identity, without HSIP storing any new secret to make that provable.
- **inputs**: `root_seed: &[u8; 32]`, `tenant_id: &str`, `transaction_id: &str`, `claimed_verify_key: &[u8; 32]`
- **outputs**: `bool`
- **calls**: `derive_transaction_signing_key`
- **called_by**: not yet wired into any HTTP route (available for an auditor with temporary root-seed access to run directly); covered by unit tests

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

## `dashboard/src/index.jsx`

### (module body — no named exports)
- **type**: entry point
- **file**: `dashboard/src/index.jsx`
- **purpose**: The real React entry point — `index.html` loads `/src/index.jsx` directly (confirmed via `index.html`'s `<script type="module" src="/src/index.jsx">`). Mounts `<App />` into `#root` inside `React.StrictMode`.
- **calls**: `ReactDOM.createRoot`, `App`
- **called_by**: `index.html` (module script tag)
- **mutates**: DOM (`#root`)

---

## `dashboard/src/main.jsx`

**Dead file, not part of the running app.** Found while completing this document's coverage: this file does the same job as `index.jsx` (creates the React root and renders `<App />`) but nothing references it — `index.html` loads `index.jsx`, not this file. Almost certainly a leftover from the original Vite React template scaffold that was never deleted once the project renamed/restructured its real entry point. Safe to delete; not documented further since it isn't live code.

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

### `BASE_URL`
- **type**: const
- **file**: `dashboard/src/pages/AIWatch.jsx`
- **purpose**: The absolute origin used in copy-paste setup snippets (Siri Shortcut URL, Claude Desktop system prompt, capabilities URL) shown to the user for connecting an external AI. `window.location.origin` — dynamic, not a fixed port, so it's correct whether the dashboard is served from 7474 (desktop), 3000 (server mode), or an embedded/production/Docker port. Previously hardcoded to `http://127.0.0.1:7777` — the wrong port for every deployment mode this project actually documents — which silently broke every one of these copy-paste snippets. Real bug, found and fixed alongside the Decisions-page agent-filter work below.

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

## `dashboard/src/data/trackers.js`

Static data module, not logic — HSIP's consumer-facing tracker knowledge base, feeding `TrackerInspector.jsx`. Its own header comment says it's "sourced from `hsip-telemetry-guard/src/known_endpoints.rs`" with consumer-friendly descriptions added on top.

### `RISK_LEVEL` / `CATEGORIES`
- **type**: variable (const object / const array)
- **file**: `dashboard/src/data/trackers.js`
- **purpose**: `RISK_LEVEL` maps `critical`/`high`/`medium`/`low` to a display label and color pair. `CATEGORIES` is the fixed list of tracker categories used for the Tracker Inspector's filter UI (Session Recording, Ad Network, Analytics, Social, Microsoft, Crash Reporting, Data Broker, Email Tracking, Fingerprinting, plus "All").
- **called_by**: `TrackerInspector.jsx`

### `TRACKERS`
- **type**: variable (const array)
- **file**: `dashboard/src/data/trackers.js`
- **purpose**: The tracker knowledge base itself — one entry per known tracker domain (`vendor`, `domain`, a plain-English one-liner, a longer `description`, `category`, `risk`, `safeToBlock`). Covers session-recording tools (Hotjar, FullStory, Microsoft Clarity, etc.), ad networks, social pixels, analytics platforms, Microsoft telemetry endpoints, data brokers, email-open trackers, and fingerprinting services. `safeToBlock: false` is used for a handful of entries (e.g. Sentry, Firebase Crashlytics, Mailchimp) where blocking could break legitimate functionality the user likely wants (crash reports, email delivery) rather than pure surveillance.
- **called_by**: `TrackerInspector.jsx`

### `TRACKER_STATS`
- **type**: variable (const object)
- **file**: `dashboard/src/data/trackers.js`
- **purpose**: Derived summary counts (`total`, `critical`, `high`, `safeToBlock`) computed once from `TRACKERS` at module load, so page components don't recompute the same `.filter().length` on every render.
- **called_by**: `TrackerInspector.jsx`

### `FIRST_PARTY_BRANDS`
- **type**: variable (const object)
- **file**: `dashboard/src/data/trackers.js`
- **purpose**: A separate lookup keyed by bare domain (google.com, amazon.com, netflix.com, etc.) for well-known first-party sites users commonly search for in the Tracker Inspector — these are *not* third-party trackers themselves, but the entry explains what data collection the company itself does and cross-references any `TRACKERS` entries it operates (e.g. `google.com` → `google-analytics.com`, `doubleclick.net`). Exists so a user searching "google.com" gets a real, honest answer instead of "not a tracker, nothing found."
- **called_by**: `TrackerInspector.jsx`

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
- **purpose**: Modal dialog for pasting and verifying a signed message a contact sent through some other channel (email/Slack/etc.) — HSIP never delivers it; copy updated to say this explicitly instead of implying an inbox.
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
- **purpose**: Renders the signed-message history for a selected contact and the compose box; signing (✍️) does not send anything — copy now says so — the user must copy and deliver the signed text themselves via "Copy last message to share."
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

## `dashboard/src/pages/Decisions.jsx`

Expert-mode AI Decisions page — pre-existing, not new (a stale roadmap item once claimed this was missing from the dashboard; corrected in CLAUDE.md). Already covers anchor/proof status, not just record/list.

### `sha256Hex`
- **type**: function (async)
- **file**: `dashboard/src/pages/Decisions.jsx`
- **purpose**: Browser-side SHA-256 via `crypto.subtle.digest` — mirrors the server-side design goal that HSIP only ever receives a hash of a decision's real content, never the content itself. Used by the "+ Connect" flow's generated code snippets, not to hash anything the dashboard itself submits.
- **inputs**: `text: string`
- **outputs**: `Promise<string>` (hex digest)
- **calls**: `crypto.subtle.digest`
- **called_by**: `Decisions`

### `ConnectDialog`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Decisions.jsx`
- **purpose**: Registers a new `ai_agent`-type key (`POST /v1/keys`) for connecting an external trading bot / AI agent, then shows the raw key once alongside copy-paste-ready Python and curl snippets for calling `record_decision`/`POST /v1/decisions` — pre-filled with the current identity's `accountable_key` so the snippet works without edits beyond the caller's own `payload_hash`.
- **inputs**: `apiKey: string`, `identity: object`, `onDone: function`, `onClose: function`
- **outputs**: JSX
- **calls**: `request` (`POST /v1/keys`)
- **called_by**: `Decisions`
- **mutates**: DB via API (`api_keys`)

### `ProofPanel`
- **type**: function (React component)
- **file**: `dashboard/src/pages/Decisions.jsx`
- **purpose**: Fetches and displays one decision's full proof bundle (`GET /v1/decisions/:id/proof`) — signature, anchor/Merkle status, `accountable_key_verified` — the UI surface for "prove this decision is real" without needing curl.
- **inputs**: `decisionId: string`, `apiKey: string`, `onClose: function`
- **outputs**: JSX
- **calls**: `request` (`GET /v1/decisions/:id/proof`)
- **called_by**: `Decisions`
- **mutates**: nothing

### `Decisions`
- **type**: function (React component, default export)
- **file**: `dashboard/src/pages/Decisions.jsx`
- **purpose**: Lists recorded decisions (`GET /v1/decisions`), shows relative time via `timeAgo`, opens `ConnectDialog` to register a new agent and `ProofPanel` to inspect a specific decision's proof/anchor status.
- **inputs**: `apiKey: string`
- **outputs**: JSX
- **calls**: `request`, `timeAgo`
- **called_by**: `App` (Expert mode, `decisions` tab)
- **mutates**: nothing directly (delegates to `ConnectDialog` for key creation)

---

## `dashboard/src/pages/DecisionsSimple.jsx`

Simple-mode ("For Everyone") counterpart to `Decisions.jsx`, under the "AI Decisions" tab — same underlying data, presented without Expert-mode's proof/anchor detail density.

### `DecisionRow`
- **type**: function (React component)
- **file**: `dashboard/src/pages/DecisionsSimple.jsx`
- **purpose**: One decision's plain-language summary row for the Simple-mode list — deliberately less technical than Expert mode's raw hash/signature display. Collapsed row shows a `🤖 <agent name> · <AI agent|Human|Service>` badge (from `d.agent_name`/`d.agent_type`, falling back to "Unknown connection" for a since-revoked key) plus an absolute, human-readable date (`formatDateTime`) alongside the relative one. Expanding a row opens a plain-language "Who / What / When / Status" fact block *before* the existing prose/JSON — meant to be readable by a non-technical reviewer (e.g. a compliance officer or executive) with nobody explaining the crypto to them. `toggle()` now auto-runs `verify()` against the freshly-fetched proof (passed directly as a `bundle` argument, since React state isn't updated synchronously) so the "Status" line shows a real, independently-checked verdict — genuine/tampered — the instant the row opens, not only after a manual click; the existing "Double-check it's genuine" button still works as an on-demand re-check (`onClick={() => verify()}`, explicitly no-arg since React would otherwise pass the click event as `bundle`).
- **inputs**: `d: object` (decision record — now includes `agent_key_id`/`agent_name`/`agent_type` from `GET /v1/decisions`), `apiKey: string`
- **outputs**: JSX
- **calls**: `request` (as needed for detail expansion), `formatDateTime`, `timeAgo`
- **called_by**: `DecisionsSimple`
- **mutates**: nothing

### `formatDateTime`
- **type**: function
- **file**: `dashboard/src/pages/DecisionsSimple.jsx`
- **purpose**: Formats an ISO timestamp into an absolute, locale-formatted date+time (e.g. "August 4, 2026 at 1:45 PM") via `Date.toLocaleString({dateStyle:'long', timeStyle:'short'})`. Added because `timeAgo` alone ("4 minutes ago") is meaningless once a review happens days or months later — a real audit/compliance read needs the actual date.
- **called_by**: `DecisionRow`

### `AGENT_TYPE_LABELS`
- **type**: const (module-level)
- **file**: `dashboard/src/pages/DecisionsSimple.jsx`
- **purpose**: Maps `api_keys.agent_type` (`'ai_agent'|'human'|'service'`) to a plain-English label ("AI agent"/"Human"/"Service") for display in `DecisionRow`.
- **called_by**: `DecisionRow`

### `ConnectSimpleDialog`
- **type**: function (React component)
- **file**: `dashboard/src/pages/DecisionsSimple.jsx`
- **purpose**: Simple-mode's version of `Decisions.jsx`'s `ConnectDialog` — same underlying `POST /v1/keys` agent registration, plainer-language copy for a non-developer audience.
- **inputs**: `apiKey: string`, `identity: object`, `onDone: function`, `onClose: function`
- **outputs**: JSX
- **calls**: `request` (`POST /v1/keys`)
- **called_by**: `DecisionsSimple`
- **mutates**: DB via API (`api_keys`)

### `TIME_WINDOWS`
- **type**: const (module-level)
- **file**: `dashboard/src/pages/DecisionsSimple.jsx`
- **purpose**: Maps the time-window filter's option values (`'all' | '24h' | '7d' | '30d'`) to a millisecond duration (or `null` for "all"), used to compute `since_ms` for `GET /v1/decisions`.
- **called_by**: `DecisionsSimple`'s `loadDecisions`

### `DecisionsSimple`
- **type**: function (React component, default export)
- **file**: `dashboard/src/pages/DecisionsSimple.jsx`
- **purpose**: The "AI Decisions" tab in Simple ("For Everyone") mode — lists decisions via `DecisionRow`, offers the same "+ Connect" agent-registration flow as Expert mode's `Decisions.jsx` via `ConnectSimpleDialog`, without anchor/proof internals. Now also fetches `GET /v1/agents` (`loadAgents`) to populate an "All agents" / named-agent filter dropdown, and a time-window dropdown (`TIME_WINDOWS`) — both drive query params (`agent_key_id`, `since_ms`) on `GET /v1/decisions`, re-fetching (and restarting the 10s poll) whenever either filter changes, so a bank/business user can answer "what did this specific agent do in this specific window" instead of scrolling an unfiltered feed.
- **inputs**: `apiKey: string`
- **outputs**: JSX
- **calls**: `request`, `timeAgo`, `loadAgents`
- **called_by**: `App` (Simple mode, `ai-decisions` tab)
- **mutates**: nothing directly (delegates to `ConnectSimpleDialog` for key creation)

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

## `sdks/python/hsip/__init__.py`

### (module body — no named functions)
- **type**: package init
- **file**: `sdks/python/hsip/__init__.py`
- **purpose**: Re-exports `HSIPClient`/`HSIPError` from `client.py` so callers can `from hsip import HSIPClient` instead of `from hsip.client import HSIPClient`. Sets `__version__ = "0.1.0"`.
- **called_by**: any consumer of the `hsip` Python package

---

## `sdks/python/setup.py`

### (module body — no named functions)
- **type**: packaging metadata (setuptools)
- **file**: `sdks/python/setup.py`
- **purpose**: `setuptools.setup()` call declaring the `hsip-sdk` package (`python_requires >= 3.8`, no cryptography dependency — deliberate, per CLAUDE.md's SDK design, so signing is left to whatever Ed25519 library the caller already uses).
- **called_by**: `pip install`

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
- **purpose**: Initialises client with `api_key`, `base_url`, and `replay_protection` (default `False`). Corrects a stale entry here that claimed a `requests.Session` — this SDK is deliberately pure stdlib (`urllib.request`), no third-party HTTP dependency at all. `replay_protection` is opt-in HTTP replay protection: `False` (default, and this SDK's only behavior before the flag existed) means every existing caller is unaffected unless it deliberately opts in.
- **inputs**: `api_key: str`, `base_url: str = "http://localhost:3000"`, `replay_protection: bool = False`
- **outputs**: none
- **calls**: none
- **called_by**: SDK users
- **mutates**: `self.api_key`, `self.base_url`, `self.replay_protection`

### `HSIPClient._request`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: Internal HTTP helper via `urllib.request` (no third-party HTTP library): builds the request, sends it, raises `HSIPError` on a non-2xx `HTTPError`. When `self.replay_protection` is `True`, adds `x-hsip-timestamp` (current Unix seconds) and `x-hsip-nonce` (`secrets.token_hex(16)`, fresh per call — reusing a fixed value would cause the client to lock itself out on its own second request, since the server dedups per `(key_id, nonce)`) to every request.
- **inputs**: `method: str`, `path: str`, `body: Optional[Dict] = None`
- **outputs**: `Any` (parsed JSON)
- **calls**: `urllib.request.urlopen`, `secrets.token_hex`, `time.time`
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

### `HSIPClient.accountable_proof_preimage_hash`
- **type**: function (static)
- **file**: `sdks/python/hsip/client.py`
- **purpose**: The exact 32 bytes `accountable_key`'s own private key must sign to produce `accountable_key_signature` for `record_decision` — an independent Python reimplementation of `hsip_core::canonical::accountable_proof_preimage_hash`, confirmed byte-for-byte identical output to the Rust implementation for the same input. Uses `json.dumps(sort_keys=True, separators=(",",":"), ensure_ascii=False)` rather than a full RFC 8785 JCS library — valid specifically because all six fields in this preimage are plain ASCII strings, so none of JCS's number-formatting or non-ASCII-escaping edge cases apply. This SDK has no cryptography dependency by design; the caller signs the returned bytes with whatever Ed25519 library they already use.
- **inputs**: `accountable_key: str`, `tenant_id: str`, `model_version: str`, `strategy_id: str`, `decision_type: str`, `payload_hash: str`
- **outputs**: `bytes` (32 bytes)
- **calls**: `json.dumps`, `hashlib.sha256`
- **called_by**: SDK users, before calling `record_decision` with `accountable_key_signature`
- **mutates**: nothing

### `HSIPClient.record_decision`
- **type**: function
- **file**: `sdks/python/hsip/client.py`
- **purpose**: `POST /v1/decisions` — signs and chains one AI-agent decision attestation. `accountable_key_signature` (optional, base64 Ed25519 — see `accountable_proof_preimage_hash` above) is additive proof-of-possession; omitting it is a pre-existing, still-valid call shape. If `receipt_dir` is given, immediately persists the receipt via `save_receipt` — the client-side mitigation for the gap between signing and the next anchor cycle (see `anchor_job.rs`).
- **inputs**: `self`, `accountable_key: str`, `model_version: str`, `strategy_id: str`, `decision_type: str`, `payload_hash: str`, `accountable_key_signature: Optional[str] = None`, `receipt_dir: Optional[str] = None`
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


---

## `crates/hsip-dns/src/lib.rs`

HSIP's UDP DNS tracker-blocking resolver — a lightweight server bound to `127.0.0.1:<port>` (5300 by default) that returns NXDOMAIN for a curated tracker/ad blocklist and transparently forwards everything else to `1.1.1.1:53`. This file has a real, already-fixed security history: the upstream-forwarding path used to bind an unconnected `0.0.0.0:0` socket, `send_to` the query, then `recv_from` and relay back *whatever arrived* with no check on source address or transaction ID — classic DNS response spoofing (any attacker who could reach the ephemeral forwarding port could race the real upstream). The fix, present in the code below, is two-layered: `.connect(upstream)` on the forwarding socket (kernel-level source filtering — the OS itself refuses datagrams from any address other than the connected peer) plus `response_transaction_id_matches` as defense-in-depth against a misbehaving/compromised-but-correctly-addressed upstream.

### `TRACKER_DOMAINS`
- **type**: variable (static `&[(&str, &str, &str)]`)
- **file**: `crates/hsip-dns/src/lib.rs`
- **purpose**: Hardcoded blocklist of `(domain_suffix, vendor, category)` tuples — Google/Meta/Microsoft/Apple analytics and ad-tech domains, session-recording tools (Hotjar, FullStory, LogRocket, etc.), and major ad networks. Deliberately excludes dual-purpose domains (`facebook.com`, `google.com`, `youtube.com` themselves) to avoid breaking normal browsing — only subdomains/services *exclusively* used for tracking are listed.
- **called_by**: `lookup_block`, `DnsHandle::blocklist_size`
- **mutates**: nothing (compile-time constant)

### `now_ms`
- **type**: function
- **file**: `crates/hsip-dns/src/lib.rs`
- **purpose**: Current Unix epoch time in milliseconds, defaulting to 0 on clock error rather than panicking.
- **outputs**: `i64`
- **calls**: `SystemTime::now`, `duration_since`
- **called_by**: `handle_query` (for `DnsLogEntry::timestamp_ms`)
- **mutates**: nothing

### `lookup_block`
- **type**: function
- **file**: `crates/hsip-dns/src/lib.rs`
- **purpose**: Case-insensitive, trailing-dot-tolerant suffix match of `hostname` against `TRACKER_DOMAINS` — matches either an exact domain or any subdomain of it (`.suffix`).
- **inputs**: `hostname: &str`
- **outputs**: `Option<(&'static str, &'static str)>` (vendor, category)
- **calls**: nothing beyond stdlib string ops
- **called_by**: `handle_query`
- **mutates**: nothing

### `parse_qname`
- **type**: function
- **file**: `crates/hsip-dns/src/lib.rs`
- **purpose**: Hand-rolled DNS QNAME parser starting at a byte offset — walks length-prefixed labels, follows compression pointers (`0xC0` high bits), and guards against compression-pointer loops via a `visited` `HashSet` of positions (an infinite loop here would be a trivial DoS otherwise). Returns the dotted hostname plus the offset just past QTYPE/QCLASS so the caller knows where the question section ends.
- **inputs**: `buf: &[u8]`, `pos: usize`
- **outputs**: `Option<(String, usize)>` — `None` on any malformed/truncated/looping input
- **calls**: nothing beyond stdlib
- **called_by**: `handle_query`
- **mutates**: nothing

### `build_nxdomain`
- **type**: function
- **file**: `crates/hsip-dns/src/lib.rs`
- **purpose**: Builds a minimal NXDOMAIN response by copying the query's transaction ID and question section verbatim, preserving the RD (recursion desired) bit and setting QR=1/RA=1/RCODE=3. Returns an empty `Vec` for any query shorter than a DNS header (12 bytes), which the caller checks before sending.
- **inputs**: `query: &[u8]`
- **outputs**: `Vec<u8>`
- **calls**: nothing beyond stdlib
- **called_by**: `handle_query`
- **mutates**: nothing

### `DnsStats`
- **type**: struct
- **file**: `crates/hsip-dns/src/lib.rs`
- **purpose**: Live atomic counters (`queries_total`, `blocked_total`) exposed to `hsip-api`'s `/v1/dns/status` route via the shared `DnsHandle`.
- **called_by**: `handle_query` (increments), `hsip-api::routes::dns`

### `DnsLogEntry`
- **type**: struct
- **file**: `crates/hsip-dns/src/lib.rs`
- **purpose**: One recent-activity record (domain, blocked flag, vendor/category if blocked, timestamp) — serializable, surfaced via `/v1/dns/log`.
- **called_by**: `handle_query`, `hsip-api::routes::dns`

### `DnsLog`
- **type**: struct
- **file**: `crates/hsip-dns/src/lib.rs`
- **purpose**: Rolling circular buffer capped at 200 entries (`VecDeque` behind a `tokio::sync::RwLock`) so the recent-activity log can't grow unbounded over a long-running desktop session.
- **calls**: `RwLock::write`
- **called_by**: `start` (constructs), `handle_query` (pushes)
- **mutates**: its own `entries` deque

### `DnsLog::push`
- **type**: function (async)
- **file**: `crates/hsip-dns/src/lib.rs`
- **purpose**: Appends one entry, evicting the oldest (`pop_front`) once the buffer hits 200 — a fixed-size ring, not a growing log.
- **inputs**: `&self`, `entry: DnsLogEntry`
- **mutates**: `self.entries`

### `DnsHandle`
- **type**: struct
- **file**: `crates/hsip-dns/src/lib.rs`
- **purpose**: Cloneable handle returned by `start()` — carries shared `stats`/`log` `Arc`s, the bound `port`, and a `shutdown_tx` broadcast sender used to stop the background resolver task without killing the whole process.
- **called_by**: `hsip-api::routes::dns` (start/stop/status/log handlers), `state.rs`'s `AppState`

### `DnsHandle::shutdown`
- **type**: function
- **file**: `crates/hsip-dns/src/lib.rs`
- **purpose**: Signals the resolver loop to exit by sending on `shutdown_tx`; the `let _ =` discards the send error, which only happens if the loop has already exited (nothing to notify).
- **mutates**: sends on the internal broadcast channel

### `DnsHandle::blocklist_size`
- **type**: function
- **file**: `crates/hsip-dns/src/lib.rs`
- **purpose**: Reports the total number of tracker entries — used by the dashboard/API to show "N trackers blocked" without hardcoding the count separately.
- **outputs**: `usize`
- **calls**: `TRACKER_DOMAINS.len()`

### `start`
- **type**: function (async)
- **file**: `crates/hsip-dns/src/lib.rs`
- **purpose**: Binds a UDP socket on `127.0.0.1:<port>`, constructs the shared stats/log/shutdown-channel state, and spawns `resolver_loop` as a background Tokio task, returning immediately with a `DnsHandle`. Fails (propagates `io::Error`) if the port is already bound — this is what surfaces as "DNS resolver failed to start" in `routes/dns.rs`.
- **inputs**: `port: u16`
- **outputs**: `std::io::Result<DnsHandle>`
- **calls**: `UdpSocket::bind`, `tokio::spawn(resolver_loop(...))`
- **called_by**: `hsip-api::routes::dns::enable` (via `AppState`)
- **mutates**: binds a real OS socket; spawns a task

### `resolver_loop`
- **type**: function (async)
- **file**: `crates/hsip-dns/src/lib.rs`
- **purpose**: The resolver's main event loop — `tokio::select!`s between a shutdown signal and incoming datagrams on the bound socket. Each received query is handed off to a freshly `tokio::spawn`ed `handle_query` task so one slow/stuck upstream lookup can't block subsequent queries.
- **inputs**: `socket: Arc<UdpSocket>`, `stats: Arc<DnsStats>`, `log: Arc<DnsLog>`, `stop: broadcast::Receiver<()>`
- **calls**: `socket.recv_from`, `tokio::spawn(handle_query(...))`
- **called_by**: `start`
- **mutates**: nothing directly (delegates to spawned tasks)

### `handle_query`
- **type**: function (async)
- **file**: `crates/hsip-dns/src/lib.rs`
- **purpose**: The core per-query handler and the site of the DNS-spoofing fix. Parses the QNAME, checks it against the blocklist (logs + replies NXDOMAIN + increments `blocked_total` if matched), otherwise forwards to `1.1.1.1:53`. The forwarding socket is bound on `0.0.0.0:0` (not loopback-only — a query could theoretically arrive from anywhere reachable, though in practice only loopback traffic reaches this resolver) and then **`.connect(upstream)`ed before sending** — this is the primary fix: a connected UDP socket has the OS itself silently drop any datagram whose source address doesn't match the connected peer, closing the response-spoofing window an unconnected `send_to`/`recv_from` pair left wide open. `response_transaction_id_matches` is then checked as a second, cheap layer even against the genuinely-connected peer. A 3-second timeout on the upstream `recv` prevents a silent/dead upstream from leaking the spawned task.
- **inputs**: `socket: Arc<UdpSocket>` (the resolver's own listening socket, used to reply to the client), `stats: Arc<DnsStats>`, `log: Arc<DnsLog>`, `query: Vec<u8>`, `client: SocketAddr`, `upstream: SocketAddr`
- **calls**: `parse_qname`, `lookup_block`, `build_nxdomain`, `DnsLog::push`, `UdpSocket::bind("0.0.0.0:0")`, `fwd.connect`, `fwd.send`, `fwd.recv` (via `tokio::time::timeout`), `response_transaction_id_matches`, `socket.send_to`
- **called_by**: `resolver_loop` (spawned per query)
- **mutates**: `stats` atomics, `log`'s ring buffer; sends real UDP packets

### `response_transaction_id_matches`
- **type**: function
- **file**: `crates/hsip-dns/src/lib.rs`
- **purpose**: Cheap defense-in-depth check that the response's first two bytes (DNS transaction ID) equal the query's — catches response confusion even from the correctly-connected upstream peer (e.g. a misbehaving or compromised resolver), on top of the `connect()`-based OS-level source filtering that's the primary defense.
- **inputs**: `query: &[u8]`, `response: &[u8]`
- **outputs**: `bool`
- **called_by**: `handle_query`
- **mutates**: nothing

---

## `crates/hsip-core/src/lib.rs`

Crate root — declares every public module of `hsip-core`, the crypto/protocol primitives crate this project's CLAUDE.md marks "Core — do not break." Two crate-level lint allows apply workspace-wide (`clippy::doc_markdown`, `clippy::missing_const_for_fn`). Notable structure: `crypto` is declared as an inline module block (`pub mod crypto { pub mod aead; pub mod labels; pub mod nonce; }`) rather than a `crypto/mod.rs` `pub mod` line for its children — both `aead`/`labels`/`nonce` under `crypto::` and a separate top-level `nonce` module (used by `error.rs`/`hello.rs`-adjacent replay-window code) coexist as genuinely distinct types, not a re-export of one by the other. `pqc` is the only module gated behind a Cargo feature (`#[cfg(feature = "pqc")]`), which is enabled by default in `Cargo.toml` (`default = ["pqc"]`).
- **Modules declared**: `aad`, `consent`, `consent_policy`, `error`, `hello`, `liveness`, `nonce`, `session`, `session_resumption`, `traffic_shaping`, `crypto` (with nested `aead`/`labels`/`nonce`), `canonical`, `identity`, `keystore`, `merkle`, `tx_key`, `wire`, `constant_time`, `secure_memory`, and feature-gated `pqc`.
- **Notable omission**: `verification.rs` exists on disk in this directory but has **no `pub mod verification;` line anywhere in this file** (confirmed by grep — nothing declares it). It is not part of the compiled crate at all; see the `verification.rs` entry below for what that means in practice.

---

## `crates/hsip-core/src/consent.rs`

Implements HSIP's original UDP-based consent request/response protocol — a requester asks to use some content (identified by a BLAKE3 CID) for a stated purpose, and a responder cryptographically signs an allow/deny decision bound to that specific request. Distinct from, and older than, `consent_policy.rs` (which evaluates *how* to decide) and the HTTP-API-level `consents` table in `hsip-api` (which is the actual persisted consent record system) — this module is the wire-level signed-message construction/validation layer for the peer-to-peer UDP flow.

### `ConsentRequestFlags`
- **type**: struct
- **file**: `crates/hsip-core/src/consent.rs`
- **purpose**: Policy/reputation signals attached to a request for downstream decision-making (unknown peer, prior denial, failed-attempt count, rate-limited, suspicious). `Default` deliberately sets `unknown_peer: true` — the safe default assumption for a peer with no other flags set.
- **called_by**: `ConsentRequestMetadata`, `consent_policy::ConsentPolicy::evaluate`

### `ConsentRequestMetadata`
- **type**: struct
- **file**: `crates/hsip-core/src/consent.rs`
- **purpose**: Extended context for evaluating one request — cryptographically-derived `peer_id`, the caller-supplied (unverified) `purpose` string, a signature-verified `timestamp_ms`, and the `ConsentRequestFlags` above.
- **called_by**: `consent_policy::ConsentPolicy::evaluate`

### `ConsentRequest`
- **type**: struct
- **file**: `crates/hsip-core/src/consent.rs`
- **purpose**: The wire-format signed request: version, requester peer ID + pubkey hex, content CID, purpose, expiry/timestamp, a random 12-byte nonce, and the Ed25519 signature (all hex-encoded for JSON transport).
- **called_by**: `create_signed_request`, `validate_request`, `create_signed_response`, `validate_response`

### `ConsentResponse`
- **type**: struct
- **file**: `crates/hsip-core/src/consent.rs`
- **purpose**: The wire-format signed response, cryptographically bound to a specific request via `request_hash_hex` (a BLAKE3 hash of the request's signed-string form, not the request's own signature) — decision is `"allow"`/`"deny"`, with a TTL that must be zero for denials.
- **called_by**: `create_signed_response`, `validate_response`

### `cid_hex`
- **type**: function
- **file**: `crates/hsip-core/src/consent.rs`
- **purpose**: BLAKE3 content identifier as a hex string — how a requester names "this content" without HSIP needing to see the content itself beyond its hash, the same content-hash-only philosophy later reused by decision attestations' `payload_hash`.
- **inputs**: `bytes: &[u8]`
- **outputs**: `String`
- **calls**: `blake3::hash`, `hex::encode`
- **mutates**: nothing

### `derive_peer_id`
- **type**: function
- **file**: `crates/hsip-core/src/consent.rs`
- **purpose**: Thin re-export of `identity::peer_id_from_pubkey` under a `hsip_identity_module` private alias — exists so this file's public API surface reads as `consent::derive_peer_id` without requiring every caller to import `identity` directly.
- **inputs**: `vk: &VerifyingKey`
- **outputs**: `String`
- **calls**: `identity::peer_id_from_pubkey`
- **called_by**: `create_signed_request`, `validate_request`, `create_signed_response`, `validate_response`
- **mutates**: nothing

### `serialize_request_for_signature` / `serialize_response_for_signature`
- **type**: function (private)
- **file**: `crates/hsip-core/src/consent.rs`
- **purpose**: Produce the exact pipe-delimited string that gets Ed25519-signed for a request/response — a hand-built format string (`"CONSENT_REQUEST|v=...|pid=...|..."`), not JCS/canonical-JSON like the newer `canonical.rs` decision-attestation format. Both the signer and every verifier must call the same function, which they do — this is the single source of truth for "what bytes actually got signed."
- **inputs**: `&ConsentRequest` / `&ConsentResponse`
- **outputs**: `String`
- **called_by**: `create_signed_request`/`validate_request` (request); `create_signed_response`/`validate_response` (response)
- **mutates**: nothing

### `create_signed_request`
- **type**: function
- **file**: `crates/hsip-core/src/consent.rs`
- **purpose**: Builds a `ConsentRequest` (deriving the peer ID from the verify key, generating a fresh random 12-byte nonce via `OsRng`) and signs its canonical string form with the requester's Ed25519 key.
- **inputs**: `signing_key: &SigningKey`, `verify_key: &VerifyingKey`, `content_id: String`, `usage_purpose: String`, `expiration_timestamp: u64`, `current_timestamp: u64`
- **outputs**: `ConsentRequest`
- **calls**: `derive_peer_id`, `OsRng::fill_bytes`, `serialize_request_for_signature`, `signing_key.sign`
- **mutates**: nothing (returns new value)

### `validate_request`
- **type**: function
- **file**: `crates/hsip-core/src/consent.rs`
- **purpose**: Full cryptographic validation of an incoming request — decodes and reconstructs the `VerifyingKey`, recomputes the expected peer ID and checks it matches the claimed one (catches a request whose `peer_id` doesn't actually derive from its own `pub_key`), then verifies the Ed25519 signature via `verify_strict` (rejects malleable/non-canonical signatures, stricter than plain `verify`).
- **inputs**: `request: &ConsentRequest`
- **outputs**: `Result<(), String>`
- **calls**: `hex::decode`, `VerifyingKey::from_bytes`, `derive_peer_id`, `serialize_request_for_signature`, `verifying_key.verify_strict`
- **mutates**: nothing

### `create_signed_response`
- **type**: function
- **file**: `crates/hsip-core/src/consent.rs`
- **purpose**: Builds and signs a `ConsentResponse` bound to `original_request` via a BLAKE3 hash of the request's signed-string form — binding is by content hash of the exact bytes that were signed, not by the request's signature itself.
- **inputs**: `signing_key: &SigningKey`, `verify_key: &VerifyingKey`, `original_request: &ConsentRequest`, `authorization_decision: &str`, `time_to_live: u64`, `current_timestamp: u64`
- **outputs**: `Result<ConsentResponse, String>`
- **calls**: `serialize_request_for_signature`, `blake3::hash`, `derive_peer_id`, `serialize_response_for_signature`, `signing_key.sign`
- **mutates**: nothing

### `validate_response`
- **type**: function
- **file**: `crates/hsip-core/src/consent.rs`
- **purpose**: Validates a response's binding to its request (hash match), the responder's key/peer-ID consistency, that `decision` is exactly `"allow"` or `"deny"`, that a `"deny"` carries `ttl_ms == 0` (a denial with a nonzero TTL would be a logical contradiction — "denied but valid for N ms" makes no sense), and finally the Ed25519 signature via `verify_strict`.
- **inputs**: `response: &ConsentResponse`, `original_request: &ConsentRequest`
- **outputs**: `Result<(), String>`
- **calls**: `serialize_request_for_signature`, `blake3::hash`, `hex::decode`, `VerifyingKey::from_bytes`, `derive_peer_id`, `serialize_response_for_signature`, `verifying_key.verify_strict`
- **mutates**: nothing

---

## `crates/hsip-core/src/consent_policy.rs`

Policy-based evaluation layer sitting on top of `consent.rs`'s cryptographic primitives — decides *how* to handle an already-parsed request (auto-deny, queue for human review, auto-accept, or silently reject) based on caller-supplied flags. Does not itself check prior-grant state ("AutoAccept" is documented as decided by a consent-cache layer elsewhere, not by this module).

### `PolicyDecision`
- **type**: enum
- **file**: `crates/hsip-core/src/consent_policy.rs`
- **purpose**: The four possible outcomes of policy evaluation — `AutoDeny`, `QueueForReview`, `AutoAccept`, `SilentReject` (the last for malformed/suspicious traffic that shouldn't even be logged as a normal denial).
- **called_by**: `ConsentPolicy::evaluate`

### `PolicyReason`
- **type**: enum
- **file**: `crates/hsip-core/src/consent_policy.rs`
- **purpose**: Audit-loggable reason code paired with every `PolicyDecision` — includes structured variants (`TooManyAttempts { count }`, `CustomPolicyRule { rule_id }`) rather than a bare string, so callers can match on reason without string parsing.
- **called_by**: `ConsentPolicy::evaluate`

### `ConsentPolicy`
- **type**: struct
- **file**: `crates/hsip-core/src/consent_policy.rs`
- **purpose**: User-configurable policy knobs (`deny_unknown_peers`, `max_failed_attempts`, `deny_previously_denied`). `Default` is permissive-leaning (queues unknown peers rather than denying, 5-attempt threshold, allows retry after denial) — `strict()` and `permissive()` are named presets for the two ends of that spectrum.
- **called_by**: consumers of the consent protocol deciding how to handle an incoming request (not currently wired into `hsip-api`'s HTTP `/v1/consent/*` routes, which use their own simpler grant/revoke model — this is the peer-to-peer UDP-layer policy engine)

### `ConsentPolicy::evaluate`
- **type**: function
- **file**: `crates/hsip-core/src/consent_policy.rs`
- **purpose**: Ordered rule evaluation: suspicious → silent reject; rate-limited → auto-deny; too many failed attempts → auto-deny; previously-denied (if policy says so) → auto-deny; unknown peer (if policy says so) → auto-deny; otherwise unknown peer → queue for review. The ordering matters — suspicious/rate-limit checks run before the unknown-peer checks so a malicious unknown peer gets silently rejected rather than merely queued.
- **inputs**: `&self`, `metadata: &ConsentRequestMetadata`
- **outputs**: `(PolicyDecision, PolicyReason)`
- **mutates**: nothing

### `ConsentPolicy::strict` / `ConsentPolicy::permissive`
- **type**: function
- **file**: `crates/hsip-core/src/consent_policy.rs`
- **purpose**: Named constructors for the two preset policies — `strict()` denies unknown peers outright and denies retries after 3 failed attempts; `permissive()` queues everything and tolerates 10 failed attempts before denying.
- **outputs**: `ConsentPolicy`

---

## `crates/hsip-core/src/constant_time.rs`

Side-channel-hardening primitives — constant-time comparisons and conditional operations meant to prevent timing attacks from leaking secret values (token comparison, key material, signature checks). `#[inline(never)]` is used deliberately on several functions specifically to stop the compiler from optimizing away the very branchless behavior these functions exist to guarantee.

### `constant_time_compare`
- **type**: function
- **file**: `crates/hsip-core/src/constant_time.rs`
- **purpose**: Byte-slice equality where timing depends on neither the position of the first differing byte nor the values involved — XORs every byte pair and ORs the results into one accumulator, only branching on the final aggregate. Length mismatch still short-circuits (a length difference isn't considered secret in this design).
- **inputs**: `a: &[u8]`, `b: &[u8]`
- **outputs**: `bool`
- **called_by**: `constant_time_compare_str`
- **mutates**: nothing

### `constant_time_compare_str`
- **type**: function
- **file**: `crates/hsip-core/src/constant_time.rs`
- **purpose**: String-typed wrapper over `constant_time_compare`, for tokens/session IDs/API keys.
- **inputs**: `a: &str`, `b: &str`
- **outputs**: `bool`
- **calls**: `constant_time_compare`
- **mutates**: nothing

### `constant_time_select`
- **type**: function
- **file**: `crates/hsip-core/src/constant_time.rs`
- **purpose**: Branchless `if choice { a } else { b }` for a single byte, via a bitmask derived from `choice` (`0xFF`/`0x00`) rather than a conditional jump.
- **inputs**: `choice: bool`, `a: u8`, `b: u8`
- **outputs**: `u8`
- **called_by**: `constant_time_conditional_copy`
- **mutates**: nothing

### `constant_time_conditional_copy`
- **type**: function
- **file**: `crates/hsip-core/src/constant_time.rs`
- **purpose**: Copies `src` into `dst` only if `choice` is true, but always reads `src` and writes `dst` regardless — the point is that memory-access *pattern* doesn't leak `choice` even though the result does depend on it. Panics (`assert_eq!`) if the two slices' lengths differ.
- **inputs**: `choice: bool`, `dst: &mut [u8]`, `src: &[u8]`
- **calls**: `constant_time_select`
- **mutates**: `dst`

### `verify_signature_ct`
- **type**: function
- **file**: `crates/hsip-core/src/constant_time.rs`
- **purpose**: Thin wrapper over `ed25519_dalek`'s `VerifyingKey::verify` — the doc comment notes ed25519-dalek's own verification is already constant-time, so this mostly exists to give this crate's callers one uniform, local error type (`SignatureError`) rather than depending directly on `ed25519_dalek`'s.
- **inputs**: `public_key: &[u8; 32]`, `message: &[u8]`, `signature: &[u8; 64]`
- **outputs**: `Result<(), SignatureError>`
- **calls**: `VerifyingKey::from_bytes`, `Verifier::verify`
- **mutates**: nothing

### `constant_time_less_than_u64` / `constant_time_equal_u64`
- **type**: function
- **file**: `crates/hsip-core/src/constant_time.rs`
- **purpose**: Constant-time `<` and `==` for `u64` — the less-than check uses `overflowing_sub`'s borrow flag rather than a comparison operator; equality XORs and checks for zero, same pattern as `constant_time_compare`.
- **inputs**: `a: u64`, `b: u64`
- **outputs**: `bool`
- **mutates**: nothing

### `secure_zero`
- **type**: function
- **file**: `crates/hsip-core/src/constant_time.rs`
- **purpose**: Zeros a byte buffer using `std::ptr::write_volatile` per byte plus a `SeqCst` compiler fence, specifically to defeat dead-store elimination — a plain `for b in data { *b = 0 }` can be optimized away entirely if the compiler proves `data` is never read again, which is exactly the case right before a key/password goes out of scope.
- **inputs**: `data: &mut [u8]`
- **calls**: `std::ptr::write_volatile`, `std::sync::atomic::compiler_fence`
- **mutates**: `data` (zeroes it)

### `SignatureError`
- **type**: enum
- **file**: `crates/hsip-core/src/constant_time.rs`
- **purpose**: Local error type (`InvalidPublicKey`/`InvalidSignature`/`VerificationFailed`) for `verify_signature_ct`, implementing `Display`/`std::error::Error`. Distinct from `hsip_core::error::HsipErrorCode` (the numeric wire-level codes) and unrelated to `ed25519_dalek::SignatureError` despite the same name.
- **called_by**: `verify_signature_ct`

---

## `crates/hsip-core/src/crypto/mod.rs`

Thin module-declaration file for `hsip_core::crypto` — declares `aead`, `labels`, `nonce` as children and re-exports all three under a `primitives` sub-module (`pub use super::{aead,labels,nonce}` with `#[doc(inline)]`) purely as a documentation/ergonomics convenience; `crypto::primitives::aead` and `crypto::aead` refer to the same module either way. Contains an empty `#[cfg(test)] mod crypto_tests {}` placeholder with no actual tests — the real tests for this module tree live in the top-level `tests/aad_labels.rs` and `tests/nonce_integrity.rs` integration tests plus each child file's own `#[cfg(test)]` block.

---

## `crates/hsip-core/src/crypto/aead.rs`

### `PacketKind`
- **type**: enum
- **file**: `crates/hsip-core/src/crypto/aead.rs`
- **purpose**: Which of the three wire roles (`Hello`, `E1`, `E2` — the consent-handshake message stages) an AEAD operation is for; used purely to select the right AAD label, never serialized onto the wire itself.
- **called_by**: `encrypt`, `decrypt`

### `aad` (private)
- **type**: function
- **file**: `crates/hsip-core/src/crypto/aead.rs`
- **purpose**: Maps a `PacketKind` to its canonical 36-byte AAD via `labels::aad_for` — the one place that binds "which packet role" to "which label constant."
- **inputs**: `kind: PacketKind`
- **outputs**: `[u8; 36]`
- **calls**: `labels::aad_for`
- **called_by**: `encrypt`, `decrypt`
- **mutates**: nothing

### `encrypt`
- **type**: function
- **file**: `crates/hsip-core/src/crypto/aead.rs`
- **purpose**: ChaCha20-Poly1305 encryption authenticating the canonical per-`PacketKind` AAD — domain-separates ciphertexts by wire role so a Hello-stage ciphertext can never be replayed/reinterpreted as an E1/E2 one even under key reuse across roles. Avoids `GenericArray`'s deprecated `from_slice`/`clone_from_slice` helpers in favor of array `.into()`.
- **inputs**: `kind: PacketKind`, `key: &[u8; 32]`, `nonce: &[u8; 12]`, `plaintext: &[u8]`
- **outputs**: `Result<Vec<u8>, String>`
- **calls**: `ChaCha20Poly1305::new`, `aad`, `aead.encrypt`
- **called_by**: `hsip-net`'s handshake/session code (per its own imports of `crypto::aead`)
- **mutates**: nothing

### `decrypt`
- **type**: function
- **file**: `crates/hsip-core/src/crypto/aead.rs`
- **purpose**: Inverse of `encrypt` — decrypts and authenticates against the same per-`PacketKind` AAD; a ciphertext produced under a different `PacketKind` (or a tampered AAD/ciphertext) fails authentication rather than silently decrypting into garbage.
- **inputs**: `kind: PacketKind`, `key: &[u8; 32]`, `nonce: &[u8; 12]`, `ciphertext: &[u8]`
- **outputs**: `Result<Vec<u8>, String>`
- **calls**: `ChaCha20Poly1305::new`, `aad`, `aead.decrypt`
- **mutates**: nothing

---

## `crates/hsip-core/src/crypto/labels.rs`

Canonical AAD-construction constants — every ChaCha20-Poly1305 operation across the HSIP wire protocol (not the `hsip-api` HTTP layer, which has its own separate `key_encryption.rs`/field-encryption AAD scheme) must build its AAD through `aad_for` to get proper domain separation.

### `PROTOCOL_ID` / `PROTOCOL_VERSION` / `CIPHERSUITE`
- **type**: variable (const)
- **file**: `crates/hsip-core/src/crypto/labels.rs`
- **purpose**: Fixed identity bytes baked into every AAD — `b"HSIP"`, `0x0002` (current wire version — bumping this intentionally breaks compatibility with older peers), and `b"CHACHA20-POLY1305"`. Binds every ciphertext to "this exact protocol, this exact version, this exact cipher," preventing cross-version or cross-cipher confusion attacks.
- **called_by**: `aad_for`

### `AAD_LABEL_HELLO` / `AAD_LABEL_E1` / `AAD_LABEL_E2`
- **type**: variable (const)
- **file**: `crates/hsip-core/src/crypto/labels.rs`
- **purpose**: Per-message-role labels distinguishing the three consent-handshake stages within the AAD.
- **called_by**: `crypto::aead::aad` (via `PacketKind` matching)

### `aad_for`
- **type**: function
- **file**: `crates/hsip-core/src/crypto/labels.rs`
- **purpose**: Builds the canonical fixed-layout 36-byte AAD: `[PROTOCOL_ID(4) | VERSION_LE(2) | CIPHERSUITE padded to 18 | LABEL padded to 12]`. Both the ciphersuite and label fields are truncated (`.min(...)`) rather than validated-and-rejected if they exceed their padded width — silently accepting an oversized label would truncate it rather than erroring, worth noting if a longer label is ever introduced.
- **inputs**: `label: &[u8]`
- **outputs**: `[u8; 36]`
- **called_by**: `crypto::aead::aad`
- **mutates**: nothing

---

## `crates/hsip-core/src/crypto/nonce.rs`

A distinct nonce abstraction from the top-level `hsip_core::nonce` module (used by `error.rs`'s `NonceError`/replay-window checks) — this one is `[session_id: u32 BE | counter: u64 BE]`-structured and used by `hsip-net`'s UDP layer for per-session monotonic nonce generation/verification, not by the HTTP API or the consent protocol.

### `Nonce`
- **type**: struct
- **file**: `crates/hsip-core/src/crypto/nonce.rs`
- **purpose**: A 12-byte ChaCha20-Poly1305 nonce with structured accessors — first 4 bytes are a `session_id`, last 8 are a `counter`, both big-endian. Deterministic and collision-resistant for a given `(session_id, counter)` pair as long as counters never repeat within a session.
- **called_by**: `NonceGen::next_nonce`

### `NonceGen`
- **type**: struct
- **file**: `crates/hsip-core/src/crypto/nonce.rs`
- **purpose**: Monotonic nonce generator for one session — holds a fixed `session_id` and an ever-incrementing `counter`. `next_nonce` panics (`expect`) on `u64` counter overflow rather than silently wrapping, since a wrapped counter would mean nonce reuse under the same key (catastrophic for ChaCha20-Poly1305).
- **called_by**: `hsip-net`'s per-session encryption path (per its import of `crypto::nonce`)

### `NonceTracker`
- **type**: struct
- **file**: `crates/hsip-core/src/crypto/nonce.rs`
- **purpose**: Receiver-side replay/reordering guard — tracks the highest `(session_id, counter)` seen and rejects anything not strictly increasing within the same session. A session-ID change resets tracking, but the *first* counter of a new session must still be `>= 1` (never `0`), same rule as a brand-new tracker's first-ever nonce.
- **inputs** (for `accept`): `&mut self`, `nonce: &Nonce`
- **outputs** (for `accept`): `Result<(), &'static str>`
- **called_by**: `hsip-net`'s UDP receive path (anti-replay enforcement)
- **mutates**: `self.last_session`, `self.last_counter`

---

## `crates/hsip-core/src/error.rs`

### `HsipErrorCode`
- **type**: enum (`#[repr(u16)]`)
- **file**: `crates/hsip-core/src/error.rs`
- **purpose**: Stable numeric error codes meant to go on the wire, into logs, or into CLI output — deliberately namespaced by leading digit (1xxx handshake/HELLO, 2xxx nonce/replay, 3xxx session/crypto, 9xxx generic) so a numeric code alone hints at the failure category without needing the string description.
- **calls** (via its `From` impls): none itself; `description()` is a pure match
- **called_by**: anything converting a `HelloError`/`NonceError`/`SessionError` into a single unified code for logging/wire transport
- **mutates**: nothing

### `HsipErrorCode::as_u16` / `description`
- **type**: function
- **file**: `crates/hsip-core/src/error.rs`
- **purpose**: `as_u16` is the raw wire value; `description` is the fixed human-readable string per variant — used together by the `Display` impl (`"{code} ({description})"`).
- **mutates**: nothing

### `From<HelloError> for HsipErrorCode` / `From<NonceError> for HsipErrorCode` / `From<SessionError> for HsipErrorCode`
- **type**: function
- **file**: `crates/hsip-core/src/error.rs`
- **purpose**: One-way mappings from each subsystem's own rich error enum down to a flat numeric code — e.g. every `SessionError::Crypto(_)` variant, regardless of its internal `&'static str` detail, collapses to the single `SessionCryptoFailure` code. This is an intentional information loss: the numeric code is for cross-boundary signaling (wire/logs), the original typed error is for in-process handling.
- **inputs**: `HelloError` / `NonceError` / `SessionError`
- **outputs**: `HsipErrorCode`
- **mutates**: nothing

---

## `crates/hsip-core/src/hello.rs`

The HSIP HELLO message — the first thing two peers exchange over UDP, before any consent negotiation. Establishes protocol version, a capability bitmask, the sender's identity, and a timestamp, all Ed25519-signed.

### `PeerId`
- **type**: struct
- **file**: `crates/hsip-core/src/hello.rs`
- **purpose**: Minimal 32-byte peer identifier for the HELLO handshake — directly the Ed25519 verifying-key bytes (`from_verifying_key` is a straight copy, not a hash like `identity::peer_id_from_pubkey`'s BLAKE3-derived string form). Two different "peer ID" concepts coexist in this crate deliberately: this raw-key one for the binary HELLO wire format, and the Base32-hash one in `identity.rs` for human-facing display.
- **called_by**: `HelloMessage::new`, `HelloMessage::with_capabilities`, `session_resumption.rs`'s `TicketPayload`

### `HSIP_VERSION_1`
- **type**: variable (const `u8`)
- **file**: `crates/hsip-core/src/hello.rs`
- **purpose**: The only currently-supported wire protocol version; `SignedHello::verify` rejects anything else as `UnsupportedVersion` — this is HSIP's downgrade-protection mechanism for the HELLO handshake.
- **called_by**: `HelloMessage::new`, `SignedHello::verify`

### `CAP_ENCRYPTED_SESSIONS` / `CAP_CONSENT_LAYER` / `CAP_REPLAY_GUARD` / `CAP_NONCE_WINDOW` / `CAP_SESSION_RESUMPTION` / `CAP_HYBRID_PQC`
- **type**: variable (const `u32`, bit flags)
- **file**: `crates/hsip-core/src/hello.rs`
- **purpose**: Bitmask capability flags a peer advertises in its HELLO. `CAP_HYBRID_PQC` (bit 16, a deliberately high bit leaving room between it and the low classical-capability bits) signals support for the hybrid X25519+ML-KEM-768 / Ed25519+ML-DSA-65 post-quantum scheme in `pqc.rs`.
- **called_by**: `HelloCapabilities::default_local`

### `HelloCapabilities`
- **type**: struct
- **file**: `crates/hsip-core/src/hello.rs`
- **purpose**: Type-safe wrapper around the raw `u32` capability bitmask with helper methods for checking/intersecting capabilities. `default_local()` conditionally includes `CAP_HYBRID_PQC` via the compile-time `PQC_CAP_BIT` constant, which is `0` when the crate is built without the `pqc` feature — so a non-PQC build never advertises PQC support even accidentally.
- **called_by**: `HelloMessage`, `session_resumption.rs`'s `TicketPayload`

### `HelloCapabilities::supports` / `intersect` / `any_common`
- **type**: function
- **file**: `crates/hsip-core/src/hello.rs`
- **purpose**: `supports` checks a single bit; `intersect` ANDs two capability sets (used for negotiation); `any_common` is a boolean "did negotiation produce anything" check, used by `SignedHello::negotiated_capabilities` to detect a hard failure (zero common capabilities) versus a successful but reduced negotiation.
- **mutates**: nothing (all `const fn`)

### `HelloMessage`
- **type**: struct
- **file**: `crates/hsip-core/src/hello.rs`
- **purpose**: The unsigned HELLO body — protocol version, capabilities, `PeerId`, and a millisecond timestamp. This is exactly what gets signed via `to_sig_bytes`, never anything else.
- **called_by**: `SignedHello::sign`

### `HelloMessage::to_sig_bytes` (private)
- **type**: function
- **file**: `crates/hsip-core/src/hello.rs`
- **purpose**: Deterministic fixed-layout byte serialization for signing: `[version:1][capabilities:4 LE][peer_id:32][timestamp_ms:8 LE]` (45 bytes total) — a hand-rolled binary format, not JSON/JCS, since this is a low-level UDP handshake message where minimizing bytes-on-the-wire matters more than human readability.
- **outputs**: `[u8; 45]`
- **called_by**: `SignedHello::sign`, `SignedHello::verify`
- **mutates**: nothing

### `SignedHello`
- **type**: struct
- **file**: `crates/hsip-core/src/hello.rs`
- **purpose**: A `HelloMessage` plus its Ed25519 `Signature`. Deliberately does **not** derive `Serialize`/`Deserialize` (noted in a doc comment) specifically to avoid needing serde support on `ed25519_dalek::Signature` — wire encoding for this type is handled at a lower level than serde.
- **called_by**: `hsip-net`'s handshake code

### `HelloError`
- **type**: enum
- **file**: `crates/hsip-core/src/hello.rs`
- **purpose**: `UnsupportedVersion(u8)` / `BadSignature` / `NoCommonCapabilities` / `BadTimestamp` — the four ways a HELLO can fail validation. Maps into `HsipErrorCode` via `error.rs`'s `From` impl for cross-module numeric-code reporting.
- **called_by**: `SignedHello::verify`, `SignedHello::negotiated_capabilities`

### `SignedHello::sign`
- **type**: function
- **file**: `crates/hsip-core/src/hello.rs`
- **purpose**: Signs a `HelloMessage`'s deterministic byte form with the sender's Ed25519 key. Caller is responsible (per the doc comment) for ensuring `hello.peer_id` is actually consistent with `signing_key` — this function does not verify that itself.
- **inputs**: `hello: HelloMessage`, `signing_key: &SigningKey`
- **outputs**: `SignedHello`
- **calls**: `HelloMessage::to_sig_bytes`, `signing_key.sign`
- **mutates**: nothing

### `SignedHello::verify`
- **type**: function
- **file**: `crates/hsip-core/src/hello.rs`
- **purpose**: Three-step validation in order: protocol version match (downgrade protection), timestamp within `max_skew_ms` of `now_ms` (both too-old and too-far-future are rejected — `ts + max_skew_ms < now_ms || ts > now_ms + max_skew_ms`), then Ed25519 signature verification. Any failure short-circuits to the corresponding `HelloError` variant without checking the remaining steps.
- **inputs**: `&self`, `verifying_key: &VerifyingKey`, `now_ms: u64`, `max_skew_ms: u64`
- **outputs**: `Result<(), HelloError>`
- **calls**: `HelloMessage::to_sig_bytes`, `verifying_key.verify`
- **called_by**: `hsip-net`'s handshake receive path
- **mutates**: nothing

### `SignedHello::negotiated_capabilities`
- **type**: function
- **file**: `crates/hsip-core/src/hello.rs`
- **purpose**: Intersects the remote peer's advertised capabilities (from an already-`verify()`ed `SignedHello`) with the caller's own `local_caps`, erroring `NoCommonCapabilities` if the result is empty — meant to be called only after `verify()` has already succeeded, per the doc comment.
- **inputs**: `&self`, `local_caps: HelloCapabilities`
- **outputs**: `Result<HelloCapabilities, HelloError>`
- **calls**: `HelloCapabilities::intersect`
- **mutates**: nothing

---

## `crates/hsip-core/src/identity.rs`

### `peer_id_from_pubkey`
- **type**: function
- **file**: `crates/hsip-core/src/identity.rs`
- **purpose**: Derives a human-facing PeerID as the first 26 Base32 (`BASE32_NOPAD`) characters of `blake3(public_key_bytes)`. Distinct from `hello.rs::PeerId`, which is the raw 32-byte verifying key itself used in the binary wire format — this one is a shorter, display-friendly, one-way-hashed identifier (you cannot recover the public key from it, unlike `hello::PeerId`).
- **inputs**: `verifying_key: &VerifyingKey`
- **outputs**: `String`
- **calls**: `compute_blake3_hash`, `BASE32_NOPAD.encode`
- **called_by**: `consent::derive_peer_id`
- **mutates**: nothing

### `compute_blake3_hash` (private)
- **type**: function
- **file**: `crates/hsip-core/src/identity.rs`
- **purpose**: Thin wrapper over `blake3::Hasher` producing a fixed 32-byte digest.
- **inputs**: `data: &[u8]`
- **outputs**: `[u8; 32]`
- **called_by**: `peer_id_from_pubkey`
- **mutates**: nothing

### `generate_keypair`
- **type**: function
- **file**: `crates/hsip-core/src/identity.rs`
- **purpose**: Fresh Ed25519 keypair from `OsRng` — the standard entry point for creating a new HSIP identity anywhere in the crate/downstream crates that need one outside of `hsip-api`'s own DB-persisted identity flow (which has its own key-generation call site in `hsip-api::identity`).
- **outputs**: `(SigningKey, VerifyingKey)`
- **calls**: `SigningKey::generate`
- **mutates**: nothing

### `sk_to_hex` / `vk_to_hex`
- **type**: function
- **file**: `crates/hsip-core/src/identity.rs`
- **purpose**: Lowercase-hex encoding of a signing/verifying key's raw 32 bytes. The doc comment on `sk_to_hex` explicitly flags that production systems should use secure storage (PKCS#8/encrypted keystore) rather than a bare hex string — this function itself makes no attempt at that; see `keystore.rs` for the (still-plaintext-private-key, dev-mode) storage layer that actually persists keys to disk.
- **inputs**: `&SigningKey` / `&VerifyingKey`
- **outputs**: `String`
- **calls**: `hex::encode`
- **mutates**: nothing

---

## `crates/hsip-core/src/keystore.rs`

Local on-disk keypair persistence to a JSON file under the OS config directory (`~/.config/HSIP/keystore.json` on Linux, platform-equivalent elsewhere via the `dirs` crate). Explicitly documented in this file's own comments as **dev-mode**: the private key is stored in plaintext hex inside the JSON, with only Unix file-permission hardening (`0o600`) as protection — this is a different, older mechanism from `hsip-api`'s `key_encryption.rs`, which actually encrypts the signing key at rest with ChaCha20-Poly1305 under a master key. Not to be confused with that production path.

### `KeyPairStorage` (private)
- **type**: struct
- **file**: `crates/hsip-core/src/keystore.rs`
- **purpose**: The on-disk JSON shape — just `pub_hex`/`priv_hex` hex strings, no encryption, no KDF, no salt.
- **called_by**: `save_keypair`, `load_keypair`

### `keystore_file_location` (private)
- **type**: function
- **file**: `crates/hsip-core/src/keystore.rs`
- **purpose**: Resolves `<config_dir>/HSIP/keystore.json`, falling back to the current directory (`"."`) if the OS config directory can't be determined, and creates the `HSIP` directory if missing (best-effort — `let _ =` on `create_dir_all`).
- **outputs**: `PathBuf`
- **calls**: `dirs::config_dir`, `fs::create_dir_all`
- **called_by**: `save_keypair`, `load_keypair`
- **mutates**: filesystem (creates a directory)

### `save_keypair`
- **type**: function
- **file**: `crates/hsip-core/src/keystore.rs`
- **purpose**: Serializes both keys to hex, writes pretty-printed JSON to the keystore file, and — Unix only — tightens the file's permissions to `0o600` immediately after creation via `apply_unix_file_permissions`. No equivalent hardening exists for Windows in this file (unlike `hsip-api`'s master-key file, which this project's CLAUDE.md flags as needing `0o600` explicitly on Unix but has no Windows ACL equivalent documented either).
- **inputs**: `signing_key: &SigningKey`, `verifying_key: &VerifyingKey`
- **outputs**: `Result<(), String>`
- **calls**: `serde_json::to_string_pretty`, `fs::File::create`, `apply_unix_file_permissions` (cfg(unix)), `file_handle.write_all`
- **mutates**: filesystem (writes/overwrites the keystore file)

### `apply_unix_file_permissions` (private, `#[cfg(unix)]`)
- **type**: function
- **file**: `crates/hsip-core/src/keystore.rs`
- **purpose**: Sets mode `0o600` on the just-written keystore file. Best-effort: both the metadata read and the `set_permissions` call silently swallow errors (`if let Ok(...)`, `let _ =`) rather than propagating a failure — a permissions-hardening step that can silently no-op is a real, if minor, gap relative to the explicit-and-checked hardening this project's CLAUDE.md later mandated for the `hsip-api` master-key file.
- **inputs**: `path: &PathBuf`
- **calls**: `fs::File::open`, `metadata.permissions`, `fs::set_permissions`
- **called_by**: `save_keypair`
- **mutates**: filesystem (file permission bits)

### `load_keypair`
- **type**: function
- **file**: `crates/hsip-core/src/keystore.rs`
- **purpose**: Reads and parses the keystore JSON, reconstructs the `SigningKey` from the 32-byte private key seed, derives the `VerifyingKey` from it, and cross-checks that the derived public key matches the stored `pub_hex` — catching a corrupted/hand-edited keystore file where the two no longer agree, rather than silently trusting the stored public key.
- **outputs**: `Result<(SigningKey, VerifyingKey), String>`
- **calls**: `fs::File::open`, `serde_json::from_str`, `hex::decode`, `SigningKey::from_bytes`, `signing_key.verifying_key`
- **mutates**: nothing (read-only)

---

## `crates/hsip-core/src/liveness.rs`

Pure keepalive/timeout decision logic for HSIP sessions — deliberately does not send or receive any packets itself (per its own module doc comment); it only answers "should I ping now" and "is this session dead," leaving the actual PING/PONG framing and socket I/O to `hsip-net`.

### `KeepaliveConfig`
- **type**: struct
- **file**: `crates/hsip-core/src/liveness.rs`
- **purpose**: Tunable thresholds — `idle_after_ms` (15s default) before pinging starts, `ping_interval_ms` (5s) between pings once idle, `max_missed_pings` (3) before declaring death, and a `hard_timeout_ms` (60s) absolute ceiling that kills a session regardless of ping history.
- **called_by**: `KeepaliveState::should_send_ping`, `KeepaliveState::is_dead`, `evaluate_liveness`

### `KeepaliveState`
- **type**: struct
- **file**: `crates/hsip-core/src/liveness.rs`
- **purpose**: Per-session mutable liveness tracking — last RX/TX/ping timestamps and a missed-ping counter. `on_data_received`/`on_pong_received` both reset `missed_pings` to 0 (any sign of life clears the counter, not just an explicit pong), while `on_ping_sent` increments it optimistically (assumes the ping will go unanswered until proven otherwise by a subsequent `on_pong_received`/`on_data_received`).
- **called_by**: `evaluate_liveness`, `hsip-net`'s session-management loop

### `KeepaliveState::should_send_ping`
- **type**: function
- **file**: `crates/hsip-core/src/liveness.rs`
- **purpose**: True only if the session has been idle (no RX) for at least `idle_after_ms`, AND either no ping has ever been sent or at least `ping_interval_ms` has elapsed since the last one — prevents ping-spamming an idle-but-not-yet-ping-threshold session.
- **inputs**: `&self`, `cfg: &KeepaliveConfig`, `now_ms: u64`
- **outputs**: `bool`
- **mutates**: nothing

### `KeepaliveState::is_dead`
- **type**: function
- **file**: `crates/hsip-core/src/liveness.rs`
- **purpose**: True if either the missed-ping count has hit the configured max, OR the hard timeout has elapsed since the last RX regardless of ping history — the hard timeout is a backstop that kills a session even if, for whatever reason, no pings were ever attempted.
- **inputs**: `&self`, `cfg: &KeepaliveConfig`, `now_ms: u64`
- **outputs**: `bool`
- **mutates**: nothing

### `evaluate_liveness`
- **type**: function
- **file**: `crates/hsip-core/src/liveness.rs`
- **purpose**: Convenience wrapper bundling both `should_send_ping` and `is_dead` checks into one `LivenessStatus` value for a single call site to consume.
- **inputs**: `cfg: &KeepaliveConfig`, `state: &KeepaliveState`, `now_ms: u64`
- **outputs**: `LivenessStatus`
- **calls**: `KeepaliveState::should_send_ping`, `KeepaliveState::is_dead`
- **mutates**: nothing

---

## `crates/hsip-core/src/pqc.rs`

Hybrid classical+post-quantum cryptography — X25519+ML-KEM-768-style KEM and Ed25519+ML-DSA-65-style signatures, gated entirely behind `#![cfg(feature = "pqc")]` (enabled by default in `hsip-core`'s `Cargo.toml`). **Note on naming**: the actual dependencies used are `pqcrypto-kyber`'s `kyber768` and `pqcrypto-dilithium`'s `dilithium3` — Kyber-768 and Dilithium3 are the pre-standardization names for what NIST finalized as ML-KEM-768 (FIPS 203) and (a close relative of) ML-DSA-65 (FIPS 204); this module's own doc comments and this project's CLAUDE.md refer to them by the NIST names throughout, so the two naming schemes should be read as referring to the same algorithm family in this codebase, not two different things. The hybrid design's stated rationale: security holds even if *either* the classical or the post-quantum half is broken.

### `HYBRID_KEM_LABEL` / `HYBRID_SIG_LABEL`
- **type**: variable (const `&[u8]`)
- **file**: `crates/hsip-core/src/pqc.rs`
- **purpose**: Domain-separation labels for the hybrid constructions, fed into HKDF when combining classical and PQ shared secrets.
- **called_by**: `combine_shared_secrets`

### `PqcError`
- **type**: enum
- **file**: `crates/hsip-core/src/pqc.rs`
- **purpose**: Unified error type across both the KEM and signature halves — covers keygen/encapsulate/decapsulate/sign/verify failures plus format errors and `SecretConsumed` (the one-time-use X25519 ephemeral secret being reused).
- **called_by**: every fallible function in this module

### `HybridKemKeypair`
- **type**: struct
- **file**: `crates/hsip-core/src/pqc.rs`
- **purpose**: Holds an X25519 `EphemeralSecret` (wrapped in `Option` so it can be `.take()`n and consumed exactly once) plus a full static Kyber-768 keypair. `is_consumed()` reports whether the X25519 half has already been used in a decapsulation.
- **called_by**: `hybrid_decapsulate`

### `HybridKemKeypair::generate`
- **type**: function
- **file**: `crates/hsip-core/src/pqc.rs`
- **purpose**: Generates both the X25519 ephemeral and the Kyber-768 static keypair fresh from `OsRng`.
- **outputs**: `Self`
- **calls**: `EphemeralSecret::random_from_rng`, `kyber768::keypair`
- **mutates**: nothing (constructs new state)

### `HybridKemKeypair::x25519_public_bytes` / `kyber_pk_bytes` / `public_bytes`
- **type**: function
- **file**: `crates/hsip-core/src/pqc.rs`
- **purpose**: Accessors for the two public-key halves individually, and `public_bytes()` for the concatenated wire form (`X25519(32) || Kyber-768 PK(1184)` = 1216 bytes total).
- **outputs**: `[u8; 32]` / `Vec<u8>` / `Vec<u8>`
- **mutates**: nothing

### `HybridCiphertext`
- **type**: struct
- **file**: `crates/hsip-core/src/pqc.rs`
- **purpose**: Wire-format hybrid KEM ciphertext — 32-byte X25519 ephemeral public key concatenated with the 1088-byte Kyber-768 ciphertext (`SIZE = 1120`). `from_bytes` checks `bytes.len() < Self::SIZE` but doesn't reject *extra* trailing bytes beyond that length — a caller passing an oversized buffer wouldn't get an error, just a truncated parse.
- **called_by**: `hybrid_encapsulate`, `hybrid_decapsulate`

### `hybrid_encapsulate`
- **type**: function
- **file**: `crates/hsip-core/src/pqc.rs`
- **purpose**: Encapsulates to a peer's hybrid public key — fresh X25519 ephemeral Diffie-Hellman plus Kyber-768 encapsulation, combined via `combine_shared_secrets`. Rejects a malformed-length Kyber public key upfront before doing any classical crypto work.
- **inputs**: `peer_x25519_pub: &[u8; 32]`, `peer_kyber_pk: &[u8]`
- **outputs**: `Result<(HybridCiphertext, [u8; 32]), PqcError>`
- **calls**: `EphemeralSecret::random_from_rng`, `x_eph.diffie_hellman`, `kyber768::PublicKey::from_bytes`, `kyber768::encapsulate`, `combine_shared_secrets`
- **mutates**: nothing (produces new values)

### `hybrid_decapsulate`
- **type**: function
- **file**: `crates/hsip-core/src/pqc.rs`
- **purpose**: Decapsulates a `HybridCiphertext` using our own keypair — `.take()`s the `Option<EphemeralSecret>` so a second call on the same keypair fails with `SecretConsumed` rather than silently reusing (and thus compromising) the same ephemeral secret twice.
- **inputs**: `our_keypair: &mut HybridKemKeypair`, `ciphertext: &HybridCiphertext`
- **outputs**: `Result<[u8; 32], PqcError>`
- **calls**: `x_secret.diffie_hellman`, `kyber768::Ciphertext::from_bytes`, `kyber768::decapsulate`, `combine_shared_secrets`
- **mutates**: `our_keypair.x25519_secret` (consumes it to `None`)

### `combine_shared_secrets` (private)
- **type**: function
- **file**: `crates/hsip-core/src/pqc.rs`
- **purpose**: Concatenates the X25519 and Kyber shared secrets as HKDF-SHA256 input keying material, expands under `HYBRID_KEM_LABEL`, and explicitly `zeroize()`s the concatenated IKM buffer afterward — the intermediate combined-but-unexpanded secret material doesn't linger in memory any longer than needed.
- **inputs**: `x_shared: &[u8]`, `kyber_shared: &[u8]`
- **outputs**: `Result<[u8; 32], PqcError>`
- **calls**: `Hkdf::<Sha256>::new`, `hk.expand`, `ikm.zeroize`
- **called_by**: `hybrid_encapsulate`, `hybrid_decapsulate`
- **mutates**: nothing lasting (zeroizes its own local buffer)

### `HybridSignature`
- **type**: struct
- **file**: `crates/hsip-core/src/pqc.rs`
- **purpose**: Concatenated Ed25519 (64 bytes) + Dilithium3 (3293 bytes) signature, `SIZE = 3357`.
- **called_by**: `HybridSigningKeypair::sign`, `HybridVerifyingKey::verify`

### `HybridSigningKeypair`
- **type**: struct
- **file**: `crates/hsip-core/src/pqc.rs`
- **purpose**: Holds both a full Ed25519 keypair and a full Dilithium3 keypair together — unlike the KEM side, both signing secrets are static/reusable (no ephemeral consumption model), since signing keys aren't supposed to be single-use the way a KEM ephemeral is.
- **called_by**: callers needing to produce hybrid signatures (e.g. `hsip-net` if/when PQC signing is wired in beyond capability negotiation)

### `HybridSigningKeypair::sign`
- **type**: function
- **file**: `crates/hsip-core/src/pqc.rs`
- **purpose**: Produces both an Ed25519 signature and a Dilithium3 detached signature over the same message, packaged as one `HybridSignature` — both halves must later verify for the overall signature to be considered valid.
- **inputs**: `&self`, `message: &[u8]`
- **outputs**: `HybridSignature`
- **calls**: `self.ed25519_sk.sign`, `dilithium3::detached_sign`
- **mutates**: nothing

### `HybridVerifyingKey`
- **type**: struct
- **file**: `crates/hsip-core/src/pqc.rs`
- **purpose**: The public counterpart to `HybridSigningKeypair` — Ed25519 verifying key + Dilithium3 public key, `SIZE = 1984` bytes serialized form.
- **called_by**: signature verification call sites

### `HybridVerifyingKey::verify`
- **type**: function
- **file**: `crates/hsip-core/src/pqc.rs`
- **purpose**: Requires **both** the Ed25519 and Dilithium3 signatures to independently verify against the same message — the hybrid security property (an attacker must break both algorithms, not just one) is enforced here at the verification step, not just claimed by the data layout.
- **inputs**: `&self`, `message: &[u8]`, `signature: &HybridSignature`
- **outputs**: `Result<(), PqcError>`
- **calls**: `self.ed25519_vk.verify`, `dilithium3::verify_detached_signature`
- **mutates**: nothing

### `PqcCapabilities`
- **type**: struct
- **file**: `crates/hsip-core/src/pqc.rs`
- **purpose**: A 2-bit capability flag pair (`mlkem768`, `mldsa65`) for protocol negotiation, encodable to/from a single byte. `NONE`/`FULL` are named constants for the two extremes; `intersect` ANDs two peers' capabilities together the same way `hello::HelloCapabilities::intersect` does for the broader capability bitmask.
- **called_by**: PQC-aware handshake negotiation (not the same bitmask as `hello::CAP_HYBRID_PQC` — this is a separate, finer-grained per-algorithm negotiation used once PQC itself is already known to be supported)

---

## `crates/hsip-core/src/secure_memory.rs`

Zeroize-on-drop wrappers for sensitive in-memory data, defending against memory dumps, swap-file exposure, cold-boot attacks, and memory-reuse leaks of stale secrets.

### `SecureBytes`
- **type**: struct
- **file**: `crates/hsip-core/src/secure_memory.rs`
- **purpose**: A `Vec<u8>` wrapper that zeroizes its contents on `Drop`. `Deref`/`DerefMut` to `[u8]` make it usable mostly like a plain byte slice; the custom `Debug` impl prints only `"SecureBytes([REDACTED N bytes])"`, so an accidental `{:?}` logging call can't leak the secret. `into_vec()` deliberately bypasses the zeroize-on-drop (via `mem::take` + `mem::forget`) when the caller is explicitly taking ownership and will handle zeroizing themselves — this is a real, callable escape hatch from the safety guarantee, worth knowing before assuming every `SecureBytes` is unconditionally protected end-to-end.
- **called_by**: anywhere in this crate/downstream crates holding raw secret bytes that need this in-memory hygiene (not currently used by `hsip-api`'s own key-encryption path, which manages zeroing differently via `key_encryption.rs`)

### `SecureKey<const N: usize>`
- **type**: struct
- **file**: `crates/hsip-core/src/secure_memory.rs`
- **purpose**: Same zeroize-on-drop/redacted-`Debug` pattern as `SecureBytes` but for fixed-size arrays (Ed25519/ChaCha20 keys). `from_slice` panics via `assert_eq!` on a length mismatch rather than returning a `Result` — a deliberate fail-fast choice for what should always be a compile-time-known-size operation.
- **called_by**: callers holding fixed-size key material

### `SecureString`
- **type**: struct
- **file**: `crates/hsip-core/src/secure_memory.rs`
- **purpose**: Zeroize-on-drop wrapper for `String`-typed secrets (passwords, tokens). Uses `unsafe { self.data.as_bytes_mut() }` to zero the underlying `String` buffer directly, since `String` has no safe mutable-byte-access API — this is sound only because the zeroed bytes are never read back as UTF-8 afterward (the value is being destroyed, not reused).
- **called_by**: callers holding password/token-like secrets as owned strings

### `try_lock_memory`
- **type**: function (three `#[cfg]`-gated variants: `unix`, `windows`, neither)
- **file**: `crates/hsip-core/src/secure_memory.rs`
- **purpose**: Best-effort request to the OS to pin the given memory range so it's never swapped to disk — `mlock` on Unix, `VirtualLock` on Windows, an unconditional `Err` stub on anything else. Explicitly advisory: the doc comment notes this typically requires elevated privileges and the OS may still swap under memory pressure regardless of a success return; callers are expected to continue without memory locking on failure rather than treating it as fatal.
- **inputs**: `ptr: *const u8`, `len: usize`
- **outputs**: `Result<(), String>`
- **calls**: `libc::mlock` (unix) / `winapi::um::memoryapi::VirtualLock` (windows)
- **mutates**: OS-level memory paging behavior for the given range (best-effort)

---

## `crates/hsip-core/src/session.rs`

Two layers in one file: low-level counter-nonce AEAD helpers (`seal_with_counter`/`open_with_counter`) for callers who want to manage nonces themselves, and a higher-level `ManagedSession` that owns nonce generation, enforces a rekey policy, and can wire in a consent-revocation check.

### `MAX_SESSION_AGE` / `MAX_PACKETS_BEFORE_REKEY` / `MAX_NONCE_COUNTER`
- **type**: variable (const)
- **file**: `crates/hsip-core/src/session.rs`
- **purpose**: Rekey-policy thresholds — 1 hour of session age, 100,000 packets, or (independently) `u64::MAX - 1` nonce-counter headroom before `SessionNonceSalt::derive` refuses to hand out another nonce. The `- 1` on the nonce ceiling leaves one value of margin rather than running exactly up to the type's maximum.
- **called_by**: `ManagedSession::check_limits`, `SessionNonceSalt::derive`

### `SessionError`
- **type**: enum
- **file**: `crates/hsip-core/src/session.rs`
- **purpose**: `NonceMismatch{expected,got}` / `Crypto(&'static str)` / `NonceExhausted` / `RekeyRequired` / `ConsentRevoked` — every failure mode a session's encrypt/decrypt/rekey path can hit. Maps into `HsipErrorCode` via `error.rs`.
- **called_by**: `seal_with_counter`, `open_with_counter`, every `ManagedSession` method

### `AeadMeta`
- **type**: struct
- **file**: `crates/hsip-core/src/session.rs`
- **purpose**: Tiny carrier for a caller-supplied monotonic `nonce_counter`, used by the low-level `seal_with_counter`/`open_with_counter` pair. The doc comment is explicit that this pair does **not** enforce monotonicity itself — the caller must guarantee the counter never repeats under a given key; `ManagedSession` (below) is the version that actually enforces this.
- **called_by**: `seal_with_counter`, `open_with_counter`

### `nonce_from_counter` (private)
- **type**: function
- **file**: `crates/hsip-core/src/session.rs`
- **purpose**: Builds a 12-byte nonce as `[0,0,0,0 | counter_be(8)]` — the low-level counter-only layout, distinct from `ManagedSession`'s salted layout below.
- **inputs**: `counter: u64`
- **outputs**: `Nonce`
- **called_by**: `seal_with_counter`, `open_with_counter`
- **mutates**: nothing

### `seal_with_counter` / `open_with_counter`
- **type**: function
- **file**: `crates/hsip-core/src/session.rs`
- **purpose**: Bare ChaCha20-Poly1305 seal/open using a counter-derived nonce with no AAD and no policy enforcement beyond `open_with_counter`'s explicit check that the supplied `AeadMeta.nonce_counter` equals the caller's `expected_counter` (catching a caller passing mismatched metadata, though not preventing nonce reuse by itself — that's the caller's job per `AeadMeta`'s doc comment).
- **inputs**: `key_bytes: &[u8; 32]`, `meta: &AeadMeta` (+ `expected_counter: u64` for open), `plaintext`/`ciphertext: &[u8]`
- **outputs**: `Result<Vec<u8>, SessionError>` / `Result<(u8, Vec<u8>), SessionError>` (the `u8` tag is a placeholder always `0`, reserved for future framing)
- **calls**: `nonce_from_counter`, `ChaCha20Poly1305::encrypt`/`decrypt`
- **mutates**: nothing

### `SessionNonceSalt` (private)
- **type**: struct
- **file**: `crates/hsip-core/src/session.rs`
- **purpose**: Per-session 4-byte random salt combined with an 8-byte counter to derive nonces (`[salt(4) | counter_be(8)]`) — unlike the bare counter layout above, this makes nonce collisions across *different* sessions using the same key vanishingly unlikely even if their counters happen to overlap, since each session has its own salt.
- **called_by**: `ManagedSession`

### `SessionNonceSalt::derive`
- **type**: function
- **file**: `crates/hsip-core/src/session.rs`
- **purpose**: Builds the salted nonce for a given counter, refusing (`NonceExhausted`) once the counter would exceed `MAX_NONCE_COUNTER`.
- **inputs**: `&self`, `counter: u64`
- **outputs**: `Result<Nonce, SessionError>`
- **called_by**: `ManagedSession::encrypt`, `ManagedSession::decrypt`
- **mutates**: nothing

### `ManagedSession`
- **type**: struct
- **file**: `crates/hsip-core/src/session.rs`
- **purpose**: The safe, policy-enforcing session wrapper — owns the AEAD cipher, the salted nonce state, session start time, packets-sent count, and an optional `consent_check` closure (`Box<dyn Fn() -> bool + Send + Sync>`) that gets consulted before *every* encrypt/decrypt. This is what gives HSIP's consent model teeth at the crypto layer: if the closure starts returning `false` (consent revoked), the session immediately refuses further encrypt/decrypt with `ConsentRevoked`, mid-session, with no separate close/teardown step required. Does not perform the handshake itself — only manages AEAD usage safely once a key is already established.
- **called_by**: `hsip-net`'s per-connection session management

### `ManagedSession::new`
- **type**: function
- **file**: `crates/hsip-core/src/session.rs`
- **purpose**: Constructs a session from an already-derived 32-byte AEAD key and a 4-byte per-session nonce salt (both expected to come from the handshake), with `consent_check` initially unset.
- **inputs**: `key_bytes: &[u8; 32]`, `nonce_salt: [u8; 4]`
- **outputs**: `Self`
- **calls**: `ChaCha20Poly1305::new`, `SessionNonceSalt::new`
- **mutates**: nothing (constructs new state)

### `ManagedSession::with_consent_check` / `attach_consent_check`
- **type**: function
- **file**: `crates/hsip-core/src/session.rs`
- **purpose**: Builder-style (`with_consent_check`, consumes and returns `self`) and mutate-in-place (`attach_consent_check`) ways to install the revocation-check closure — the two exist so a caller can either set it up during construction or bolt it onto an already-running session (e.g. once a consent record becomes available after the session started).
- **mutates**: `self.consent_check`

### `ManagedSession::check_limits` (private)
- **type**: function
- **file**: `crates/hsip-core/src/session.rs`
- **purpose**: Runs before every encrypt/decrypt: consent check first (cheapest, and the one with the most time-sensitive consequence if skipped), then session age vs. `MAX_SESSION_AGE`, then packet count vs. `MAX_PACKETS_BEFORE_REKEY` — any failure short-circuits the whole operation before any crypto work happens.
- **inputs**: `&self`
- **outputs**: `Result<(), SessionError>`
- **called_by**: `encrypt`, `decrypt`
- **mutates**: nothing

### `ManagedSession::encrypt` / `decrypt`
- **type**: function
- **file**: `crates/hsip-core/src/session.rs`
- **purpose**: `encrypt` runs policy checks, derives the next salted nonce from the current `packets_sent` counter, encrypts, and only then increments the counter — deliberately *after* a successful encrypt, so a failed encrypt doesn't burn a nonce value. `decrypt` takes an explicit `counter` from the caller (the sender's counter at time of encryption) rather than tracking its own — the receiver doesn't maintain a parallel counter, it trusts the wire-carried value (paired with whatever anti-replay mechanism sits above this, e.g. `crypto::nonce::NonceTracker`, to actually prevent counter reuse from an attacker).
- **inputs**: `&mut self` (encrypt) / `&self` (decrypt), `plaintext`/`ciphertext: &[u8]`, `aad: &[u8]`, (+`counter: u64` for decrypt)
- **outputs**: `Result<(u64, Vec<u8>), SessionError>` / `Result<Vec<u8>, SessionError>`
- **calls**: `check_limits`, `SessionNonceSalt::derive`, `cipher.encrypt`/`decrypt`
- **mutates**: `self.packets_sent` (encrypt only, after success)

### `ManagedSession::encrypt_with_shaping` / `decrypt_with_shaping`
- **type**: function
- **file**: `crates/hsip-core/src/session.rs`
- **purpose**: Wrap `encrypt`/`decrypt` with `traffic_shaping::add_padding`/`remove_padding` and (on the send side) `apply_timing_jitter` — the recommended path for privacy-sensitive traffic, per the doc comment, since it normalizes packet size and timing to resist traffic analysis.
- **calls**: `traffic_shaping::add_padding`/`remove_padding`, `traffic_shaping::apply_timing_jitter`, `self.encrypt`/`decrypt`
- **mutates**: same as `encrypt`/`decrypt`

### `ManagedSession::stats`
- **type**: function
- **file**: `crates/hsip-core/src/session.rs`
- **purpose**: Exposes `(elapsed, packets_sent)` for monitoring/logging call sites.
- **outputs**: `(Duration, u64)`
- **mutates**: nothing

---

## `crates/hsip-core/src/session_resumption.rs`

Encrypted, self-contained session-resumption tickets — a peer that already completed a full HELLO+consent handshake can reconnect later without re-negotiating consent, by presenting a ticket the server previously issued. Fresh X25519/ChaCha20 keys are still generated per actual connection (per the file's own top comment) — the ticket only vouches for identity/capabilities/validity window, it isn't a session-key cache.

### `TicketEncryptionKey`
- **type**: struct
- **file**: `crates/hsip-core/src/session_resumption.rs`
- **purpose**: Wraps the server-side static 32-byte key used to encrypt/decrypt every ticket — must persist across server restarts, or every ticket issued before a restart becomes permanently unredeemable.
- **called_by**: `issue_resumption_ticket`, `validate_resumption_ticket`

### `TicketPolicy`
- **type**: struct
- **file**: `crates/hsip-core/src/session_resumption.rs`
- **purpose**: Caps ticket validity duration (`max_validity_duration_ms`, default 60,000ms = 1 minute) — a deliberately short default window, consistent with tickets being a short-lived resumption aid rather than a long-lived credential.
- **called_by**: `issue_resumption_ticket`

### `TicketPayload`
- **type**: struct
- **file**: `crates/hsip-core/src/session_resumption.rs`
- **purpose**: The decrypted ticket contents — `peer_id`, negotiated `caps`, `issued_at_ms`, `expires_at_ms`. Serialized to a fixed 52-byte binary layout (`PAYLOAD_SIZE = 32+4+8+8`), not JSON, since this is the ciphertext's plaintext and every byte counts toward the ticket's wire size.
- **called_by**: `issue_resumption_ticket`, `validate_resumption_ticket`

### `TicketError`
- **type**: enum
- **file**: `crates/hsip-core/src/session_resumption.rs`
- **purpose**: `ExcessiveLifetime` (requested validity exceeds policy) / `InsufficientLength` (ticket too short to even contain a valid format) / `AuthenticationFailure` (AEAD tag check failed — covers both tampering and a wrong key) / `TicketExpired` / `FutureTicket` (issued-at is after the current time — signals possible clock skew rather than assuming benign).
- **called_by**: `issue_resumption_ticket`, `validate_resumption_ticket`

### `TICKET_LABEL`
- **type**: variable (const `&[u8]`)
- **file**: `crates/hsip-core/src/session_resumption.rs`
- **purpose**: `b"HSIP-TICKET-V1"` — the AEAD associated data binding every ticket ciphertext to this specific purpose/version, so a ticket ciphertext can't be confused with (or replayed as) some other AEAD use of the same key.
- **called_by**: `issue_resumption_ticket`, `validate_resumption_ticket`

### `serialize_payload` / `deserialize_payload` (private)
- **type**: function
- **file**: `crates/hsip-core/src/session_resumption.rs`
- **purpose**: Fixed-offset binary (de)serialization of `TicketPayload` to/from the 52-byte plaintext layout described in the module's top comment.
- **called_by**: `issue_resumption_ticket` / `validate_resumption_ticket`
- **mutates**: nothing

### `issue_resumption_ticket`
- **type**: function
- **file**: `crates/hsip-core/src/session_resumption.rs`
- **purpose**: Validates the requested `lifetime_ms` against policy, builds the payload, encrypts it in place with a fresh random 12-byte nonce (`encrypt_in_place` — reuses the plaintext buffer for the ciphertext rather than allocating separately), and returns `nonce || ciphertext+tag` as the opaque ticket blob.
- **inputs**: `key: &TicketEncryptionKey`, `policy: &TicketPolicy`, `peer_id: PeerId`, `caps: HelloCapabilities`, `current_time_ms: u64`, `lifetime_ms: u64`
- **outputs**: `Result<Vec<u8>, TicketError>`
- **calls**: `serialize_payload`, `ChaCha20Poly1305::new`, `OsRng::fill_bytes`, `cipher.encrypt_in_place`
- **mutates**: nothing (returns new bytes)

### `validate_resumption_ticket`
- **type**: function
- **file**: `crates/hsip-core/src/session_resumption.rs`
- **purpose**: Length-checks the ticket (`>= 12 + 52 + 16` bytes for nonce+payload+AEAD tag), decrypts+authenticates it, re-validates the decrypted length exactly equals `PAYLOAD_SIZE`, then checks temporal validity — rejecting both a ticket used before its own `issued_at_ms` (`FutureTicket`, a clock-skew signal) and one used after `expires_at_ms` (`TicketExpired`). The `_policy` parameter is accepted but unused in this function (the lifetime cap was already enforced at issuance time, not at validation time).
- **inputs**: `key: &TicketEncryptionKey`, `_policy: &TicketPolicy`, `ticket_data: &[u8]`, `current_time_ms: u64`
- **outputs**: `Result<TicketPayload, TicketError>`
- **calls**: `ChaCha20Poly1305::new`, `cipher.decrypt_in_place`, `deserialize_payload`
- **mutates**: nothing

---

## `crates/hsip-core/src/traffic_shaping.rs`

Metadata-protection helpers — padding plaintext to fixed target sizes and adding timing jitter, to resist traffic-analysis attacks that infer content from packet size/timing patterns rather than breaking the encryption itself.

### `PAD_TARGETS`
- **type**: variable (const `&[usize]`)
- **file**: `crates/hsip-core/src/traffic_shaping.rs`
- **purpose**: The three MTU-safe bucket sizes (512/1024/1200 bytes) every padded packet is rounded up to — chosen to avoid IP fragmentation while still giving an observer only 3 possible packet-length buckets to distinguish, rather than the plaintext's exact length.
- **called_by**: `add_padding`

### `add_padding`
- **type**: function
- **file**: `crates/hsip-core/src/traffic_shaping.rs`
- **purpose**: Pads `plaintext` up to the next `PAD_TARGETS` bucket large enough to hold it plus a 1-byte ISO-7816-4-style marker (`0x80`) and a 2-byte big-endian padding length — random (not zero) padding bytes, so the padding itself doesn't look distinctively patterned on the wire. Falls back to the largest bucket (1200) if the input is already too large for any listed target, which quietly becomes a no-op-sized ceiling rather than an error for oversized input.
- **inputs**: `plaintext: &[u8]`
- **outputs**: `Vec<u8>`
- **called_by**: `session::ManagedSession::encrypt_with_shaping`
- **mutates**: nothing

### `remove_padding`
- **type**: function
- **file**: `crates/hsip-core/src/traffic_shaping.rs`
- **purpose**: Inverse of `add_padding` — reads the trailing 2-byte length, locates the data/padding boundary, and verifies the `0x80` marker byte at that boundary before trusting the recovered plaintext; any inconsistency (too-short input, an implausible padding length, or a missing marker) is rejected rather than silently returning corrupted data.
- **inputs**: `padded: &[u8]`
- **outputs**: `Result<Vec<u8>, &'static str>`
- **called_by**: `session::ManagedSession::decrypt_with_shaping`
- **mutates**: nothing

### `apply_timing_jitter`
- **type**: function
- **file**: `crates/hsip-core/src/traffic_shaping.rs`
- **purpose**: Blocks the current thread for a random 50–200ms delay before a packet is sent, to decorrelate send timing from the underlying event that triggered it. Uses `std::thread::sleep` — on an async/Tokio call path (as in `ManagedSession::encrypt_with_shaping`) this blocks the executor thread rather than yielding, a real operational tradeoff worth knowing if this is ever called from a shared async runtime under load.
- **mutates**: blocks the calling thread (real wall-clock delay)

### `TrafficShapingConfig`
- **type**: struct
- **file**: `crates/hsip-core/src/traffic_shaping.rs`
- **purpose**: Toggles for padding/jitter/cover-traffic, with padding and jitter **on** by default and cover traffic **off** by default (bandwidth overhead is opt-in). `from_env()` reads `HSIP_DISABLE_PADDING`/`HSIP_DISABLE_TIMING_JITTER`/`HSIP_ENABLE_COVER_TRAFFIC`/`HSIP_COVER_TRAFFIC_INTERVAL_MS` — note the inverted sense of the first two (presence of the env var *disables* a default-on feature) versus the last two (presence *enables* a default-off one).
- **called_by**: wherever traffic-shaping config is read at startup (not wired into `hsip-api`'s own config loading as of this file — this is `hsip-net`/session-layer configuration, separate from `hsip-api`'s `config.rs`)

### `TrafficShapingConfig::print_banner`
- **type**: function
- **file**: `crates/hsip-core/src/traffic_shaping.rs`
- **purpose**: Prints a human-readable startup summary of the active shaping configuration directly to stdout via `println!` — a diagnostic/CLI convenience, not a `tracing`-instrumented log line like the rest of this codebase typically uses.
- **mutates**: stdout

---

## `crates/hsip-core/src/verification.rs`

**This file is not part of the compiled `hsip-core` crate.** `lib.rs` declares every other module in this assignment via `pub mod ...;` but has no `pub mod verification;` (or private `mod verification;`) line anywhere — confirmed by grepping the whole `src/` tree for `mod verification`, which returns nothing. The file also references a `"verification"` Cargo feature (`#[cfg(feature = "verification")]`) that doesn't exist in `hsip-core`'s `Cargo.toml` at all (only `pqc` is a real feature there), and calls `hsip_verify::{Verifier, VerificationConfig}` — `hsip-verify` is a real, separate workspace crate (the Z3-based formal-verification crate this project's CLAUDE.md describes at length), but `hsip-core`'s own `Cargo.toml` does not depend on it. In short: this looks like an early draft or an abandoned integration point for wiring `hsip-verify`'s formal checks into `hsip-core` itself, left on disk but never connected — the actual integration this project ended up with is `hsip-verify` as its own independent workspace member (see CLAUDE.md's "Including hsip-verify in the Build"), not through this file. Read the code below as "what this file would do if it were ever wired in," not as live, exercised behavior.

### `initialize_with_verification`
- **type**: function (two variants: `#[cfg(feature = "verification")]` real impl, `#[cfg(not(...))]` stub — but see above, neither is ever compiled since the module itself is undeclared)
- **file**: `crates/hsip-core/src/verification.rs`
- **purpose**: Intended entry point to run `hsip-verify`'s Z3-backed checks (consent non-forgery, temporal consistency, identity binding) once at startup and report pass/fail, printing violated-property names to stderr on failure. The stub variant (for builds without the feature) unconditionally returns `true` with a warning printed, so verification failure would never block startup even if this were wired in and the feature were off.
- **inputs**: `verbose: bool`
- **outputs**: `bool`
- **calls** (real variant): `hsip_verify::Verifier::new`, `verifier.verify_all`
- **mutates**: nothing (would print to stdout/stderr)

### `quick_verification_check`
- **type**: function (same two-variant split as above)
- **file**: `crates/hsip-core/src/verification.rs`
- **purpose**: A faster/quieter variant (2s timeout instead of 5s, no counterexample generation, silent) intended for a lighter-weight check than full startup verification. Same caveat as above — not part of the compiled crate.
- **outputs**: `bool`
- **calls** (real variant): `hsip_verify::Verifier::new`, `verifier.verify_all`
- **mutates**: nothing

---

## `crates/hsip-core/src/wire/mod.rs`

### `MAX_HELLO_SIZE` / `MAX_CONSENT_REQUEST_SIZE` / `MAX_CONSENT_RESPONSE_SIZE` / `MAX_CONTROL_FRAME_SIZE`
- **type**: variable (const `usize`)
- **file**: `crates/hsip-core/src/wire/mod.rs`
- **purpose**: Fixed upper bounds (1024 / 2048 / 2048 / 4096 bytes) for the corresponding message types on the wire — a receiver checking incoming packet length against these before attempting to parse gets a cheap first line of defense against oversized/malformed input, though this module itself doesn't enforce them (they're just the published constants; enforcement is the caller's, e.g. `hsip-net`'s job).
- **called_by**: `hsip-net`'s packet-size validation (per naming convention, not confirmed by this file directly)

---

## `crates/hsip-core/src/wire/prefix.rs`

### `HSIP_MAGIC` / `HSIP_VER` / `PREFIX_LEN`
- **type**: variable (const)
- **file**: `crates/hsip-core/src/wire/prefix.rs`
- **purpose**: The fixed 6-byte prefix (`b"HSIP"` + big-endian `u16` version `0x0002`) every HSIP UDP packet must start with — the doc comment notes `HSIP_VER` must be kept matching "your current wire version," i.e. this constant and `crypto::labels::PROTOCOL_VERSION` (also `0x0002`) are two independently-maintained copies of the same logical version number in different modules, not derived from one shared source — a version bump would need updating both.
- **called_by**: `write_prefix`, `check_prefix`

### `write_prefix`
- **type**: function
- **file**: `crates/hsip-core/src/wire/prefix.rs`
- **purpose**: Appends the 6-byte magic+version prefix to an outgoing packet buffer — always called before the rest of a packet's bytes are written.
- **inputs**: `buf: &mut Vec<u8>`
- **calls**: nothing beyond stdlib
- **called_by**: outgoing-packet construction in `hsip-net`
- **mutates**: `buf` (appends bytes)

### `check_prefix`
- **type**: function
- **file**: `crates/hsip-core/src/wire/prefix.rs`
- **purpose**: Cheap first-pass validation letting a receiver quickly reject any packet that isn't HSIP at all (wrong magic) or is from an incompatible wire version, before spending any effort parsing it further — exactly the kind of fast-reject check `hsip-dns`-style resolvers or `hsip-net`'s UDP receive loop would want at the very top of packet handling.
- **inputs**: `pkt: &[u8]`
- **outputs**: `bool`
- **called_by**: incoming-packet validation in `hsip-net`
- **mutates**: nothing

---
## `crates/hsip-net/src/lib.rs`

Crate root for `hsip-net` — HSIP's UDP-based peer-to-peer protocol implementation (handshake, consent request/response, control-plane messaging), separate from `hsip-api`'s HTTP server. Per `CLAUDE.md`, this crate is "supporting, not actively integrated" into the main product surface (the CLI's `hsip-cli` binary is the only real consumer, via a handful of demo/handshake subcommands) — most of its modules exist as hardening layers of varying integration status. Re-exports its modules three ways: directly (`pub mod X`), and grouped under `protocol`/`transport`/`security` facade modules that just `pub use` the same items under a more descriptive namespace — no additional logic lives in the facades themselves.

### `protocol` / `transport` / `security`
- **type**: module (facade, re-export only)
- **file**: `crates/hsip-net/src/lib.rs`
- **purpose**: Purely organizational re-export modules — `protocol` re-exports `handshake_io`/`hello`, `transport` re-exports `udp`, `security` re-exports `connection_guard`/`consent_cache`/`guard`/`input_validator`/`rate_limiter`/`tls_wrapper`. Let callers write `hsip_net::security::guard::Guard` instead of `hsip_net::guard::Guard` if they prefer the grouped naming; both paths resolve to the same items.
- **calls**: nothing
- **called_by**: nothing internally — a convenience surface for external callers
- **mutates**: nothing

---

## `crates/hsip-net/src/config.rs`

### `NetConfig`
- **type**: struct
- **file**: `crates/hsip-net/src/config.rs`
- **purpose**: Deserializable config struct for the network layer — optional identity path override, UDP listen address (default `127.0.0.1:9100`), and a debug flag. Distinct from `hsip-api`'s `Config` (server mode/desktop mode) — this is `hsip-net`'s own, much smaller config surface, read from `~/.hsip/config.toml` (or `$HSIP_HOME/config.toml`) if present.
- **calls**: none
- **called_by**: `NetConfig::load` (constructs it via `toml::from_str`), CLI code that wants network defaults
- **mutates**: nothing

### `NetConfig::default_path`
- **type**: function
- **file**: `crates/hsip-net/src/config.rs`
- **purpose**: Resolves the config file location — `$HSIP_HOME/config.toml` if that env var is set, else `~/.hsip/config.toml`. Falls back to `.` as the home directory if `dirs::home_dir()` fails (e.g. no `$HOME` set), rather than panicking.
- **outputs**: `PathBuf`
- **calls**: `std::env::var`, `dirs::home_dir`
- **called_by**: `NetConfig::load`
- **mutates**: nothing

### `NetConfig::load`
- **type**: function
- **file**: `crates/hsip-net/src/config.rs`
- **purpose**: Loads config from the resolved path if it exists; returns `NetConfig::default()` on any failure (missing file, unreadable, or malformed TOML) rather than propagating an error — a fully optional, best-effort config load, never a hard startup dependency.
- **outputs**: `Self`
- **calls**: `Self::default_path`, `fs::read_to_string`, `toml::from_str`
- **called_by**: CLI/net setup code wanting network defaults
- **mutates**: nothing (reads filesystem only)

### `NetConfig::debug_banner`
- **type**: function
- **file**: `crates/hsip-net/src/config.rs`
- **purpose**: Prints a one-line `[ConfigDebug]` summary of the loaded config to stderr — a debug/diagnostic aid, not gated on the `debug` flag itself (caller decides when to invoke it).
- **calls**: `eprintln!`
- **called_by**: CLI diagnostic paths
- **mutates**: stderr (prints)

---

## `crates/hsip-net/src/connection_guard.rs`

The module's own doc comment states plainly: **"STATUS: NOT CURRENTLY INTEGRATED."** This is a from-scratch connection-limiting/bandwidth-tracking layer (max concurrent connections, idle timeout, bandwidth-per-connection) built as a candidate replacement/complement for `guard.rs`, but never wired into `udp.rs` or any protocol handler — `guard.rs`'s `Guard` is the module that's actually live. Kept for potential future use; every item below is fully implemented and unit-tested but has zero real callers in this workspace outside its own test module.

### `ConnectionLimits`
- **type**: struct
- **file**: `crates/hsip-net/src/connection_guard.rs`
- **purpose**: Configuration for `ConnectionTracker` — max total concurrent connections (default 1000), idle timeout (5 min), handshake timeout (10s), I/O timeout (30s), max bandwidth per connection (10 MB/s).
- **calls**: none
- **called_by**: `ConnectionTracker::new` (unused elsewhere in the workspace)
- **mutates**: nothing

### `ConnectionTracker`
- **type**: struct
- **file**: `crates/hsip-net/src/connection_guard.rs`
- **purpose**: Global, `Clone`-able (via internal `Arc` sharing) tracker of active connection count and cumulative bytes sent/received, gating new connections against `ConnectionLimits::max_total_connections`.
- **calls**: none directly (atomics)
- **called_by**: nothing in production code — only its own `#[cfg(test)]` module
- **mutates**: its own `AtomicUsize`/`AtomicU64` counters

### `ConnectionTracker::try_acquire`
- **type**: function
- **file**: `crates/hsip-net/src/connection_guard.rs`
- **purpose**: Attempts to claim one connection slot; returns `ConnectionError::TooManyConnections` if `active_connections >= max_total_connections`, otherwise increments the counter and returns an RAII `ConnectionGuard` that releases the slot on `Drop`.
- **outputs**: `Result<ConnectionGuard, ConnectionError>`
- **calls**: `AtomicUsize::load`/`fetch_add`
- **called_by**: test module only (not integrated)
- **mutates**: `active_connections` counter

### `ConnectionTracker::stats` / `ConnectionTracker::release`
- **type**: function
- **file**: `crates/hsip-net/src/connection_guard.rs`
- **purpose**: `stats()` snapshots current counters into a `ConnectionStats`; `release()` (private, called only from `ConnectionGuard::drop`) decrements the active-connection count.
- **outputs**: `ConnectionStats` / `()`
- **calls**: atomic loads
- **called_by**: `ConnectionGuard::drop` (release)
- **mutates**: `active_connections` (release)

### `ConnectionGuard`
- **type**: struct
- **file**: `crates/hsip-net/src/connection_guard.rs`
- **purpose**: RAII handle for one acquired connection slot — tracks its own creation time, last-activity time, and bytes sent/received; releases its slot from the shared `ConnectionTracker` automatically on `Drop`, so a guard going out of scope (normal return or panic unwind) can't leak a slot.
- **calls**: none
- **called_by**: `ConnectionTracker::try_acquire` (constructs), unused elsewhere
- **mutates**: its own last-activity/bytes counters; on drop, the tracker's active-connection count

### `ConnectionGuard::is_idle` / `touch` / `record_sent` / `record_received` / `check_bandwidth` / `age`
- **type**: function
- **file**: `crates/hsip-net/src/connection_guard.rs`
- **purpose**: Per-connection bookkeeping helpers — `is_idle` compares time since last activity to a caller-supplied timeout; `touch` refreshes the activity timestamp; `record_sent`/`record_received` add to both this connection's and the tracker's global byte counters (and implicitly `touch()`); `check_bandwidth` computes bytes/sec since connection creation (skips the check in the first second to avoid a divide-by-near-zero false positive) and flags `ConnectionError::BandwidthExceeded` if over `ConnectionLimits::max_bandwidth_per_conn`; `age` returns elapsed time since creation.
- **outputs**: `bool` / `()` / `()` / `()` / `Result<(), ConnectionError>` / `Duration`
- **calls**: `Instant::now`, atomic ops
- **called_by**: none in production code (unintegrated module)
- **mutates**: last-activity timestamp, byte counters (sent/received variants)

### `ConnectionError`
- **type**: enum
- **file**: `crates/hsip-net/src/connection_guard.rs`
- **purpose**: `TooManyConnections` | `BandwidthExceeded` | `Timeout` — error type for this module's checks, with a `Display`/`Error` impl.
- **called_by**: `ConnectionTracker::try_acquire`, `ConnectionGuard::check_bandwidth`

---

## `crates/hsip-net/src/consent_cache.rs`

### `ConsentCache`
- **type**: struct
- **file**: `crates/hsip-net/src/consent_cache.rs`
- **purpose**: In-memory `HashMap<String, Instant>` mapping a requester ID to when its cached "allow" decision expires, backing instant consent revocation for live sessions. Not thread-safe on its own — `SharedConsentCache` below is the version actually used across threads/sessions.
- **calls**: none
- **called_by**: `SharedConsentCache` (wraps one behind a lock)
- **mutates**: its own `allow_until` map

### `ConsentCache::new` / `is_allowed` / `insert_allow` / `revoke`
- **type**: function
- **file**: `crates/hsip-net/src/consent_cache.rs`
- **purpose**: `new(ttl_ms)` sets the cache's TTL for future allow entries. `is_allowed(requester)` returns `true` only if a non-expired entry exists — an expired entry is lazily evicted on lookup rather than by a background sweep. `insert_allow(requester)` (re)inserts an allow entry expiring `ttl_ms` from now. `revoke(requester)` removes the entry outright; the doc comment notes this is what makes revocation "instant" — any session with `attach_consent_check` wired to this peer will see the next `encrypt`/`decrypt` call fail with `SessionError::ConsentRevoked` rather than waiting for a stale allow to time out naturally.
- **inputs**: `requester: &str` (empty string is treated as never-allowed / no-op on insert/revoke, not an error)
- **outputs**: `bool` (is_allowed) / `()` (others)
- **calls**: `Instant::now`, `HashMap` ops
- **called_by**: `SharedConsentCache`'s matching methods
- **mutates**: `allow_until` map

### `SharedConsentCache`
- **type**: struct
- **file**: `crates/hsip-net/src/consent_cache.rs`
- **purpose**: `Clone`-able, `Arc<RwLock<ConsentCache>>`-backed wrapper so the same consent cache can be shared across the control-plane loop in `udp.rs` and any per-session consent-check closures it hands out — every clone points at the same underlying cache.
- **calls**: `ConsentCache::new`
- **called_by**: `udp.rs::listen_control` (constructs one with a 5-minute TTL)
- **mutates**: nothing itself (delegates to the inner lock)

### `SharedConsentCache::is_allowed` / `insert_allow` / `revoke`
- **type**: function
- **file**: `crates/hsip-net/src/consent_cache.rs`
- **purpose**: Thread-safe pass-throughs to the same-named `ConsentCache` methods, taking the `RwLock` write guard even for the read-like `is_allowed` (since it performs lazy eviction on miss/expiry, it's a mutating operation, not a pure read).
- **inputs**: `peer_id: &str`
- **outputs**: `bool` (is_allowed) / `()` (others)
- **calls**: `RwLock::write`, `ConsentCache::{is_allowed,insert_allow,revoke}`
- **called_by**: `udp.rs::handle_control_message` (insert_allow on a granted consent request), `create_check_callback`'s closure (is_allowed)
- **mutates**: the shared inner `ConsentCache`

### `SharedConsentCache::create_check_callback`
- **type**: function
- **file**: `crates/hsip-net/src/consent_cache.rs`
- **purpose**: Builds a `Fn() -> bool + Send + Sync + 'static` closure bound to one `peer_id`, capturing a clone of the cache — meant to be handed to `ManagedSession::attach_consent_check` so every encrypt/decrypt on that session re-checks live consent state rather than a one-time snapshot from handshake time.
- **inputs**: `peer_id: String`
- **outputs**: `impl Fn() -> bool + Send + Sync + 'static`
- **calls**: `self.clone()`, `ConsentCache::is_allowed` (via the returned closure)
- **called_by**: `udp.rs::handle_control_message` (once per session direction, rx and tx, when a consent request is granted)
- **mutates**: nothing itself; the returned closure reads (and lazily mutates via eviction) the shared cache when invoked

---

## `crates/hsip-net/src/guard.rs`

This is the **actively integrated** rate-limiting/abuse-tracking layer, wired directly into `udp.rs`'s control-plane loop — unlike `connection_guard.rs`/`rate_limiter.rs`, which document themselves as unintegrated alternatives. `Guard` combines several independent per-IP sliding-window counters (E1 handshakes/5s, bad signatures/min, control frames/min, consent requests/min), a static IP blocklist ("tracker wall") loaded from disk, frame-size ceilings, and a peer "pinning" mechanism (auto-allow for a configured number of minutes after a consent grant).

### `GuardCfg` (alias: `GuardConfig`)
- **type**: struct
- **file**: `crates/hsip-net/src/guard.rs`
- **purpose**: All tunables for `Guard` — enable flag, pin duration, per-window rate limits (E1/5s, bad-sig/min, control-frame/min, consent-request/min), and max accepted sizes for control frames/HELLO/consent request/consent response, pulled from `hsip_core::wire`'s size constants by default. `GuardConfig` is kept as a type alias for back-compat with older call sites that used that name.
- **calls**: none
- **called_by**: `Guard::new`, `udp.rs::listen_control`
- **mutates**: nothing

### `WindowCounter`
- **type**: struct (private)
- **file**: `crates/hsip-net/src/guard.rs`
- **purpose**: A single sliding-window rate counter — a `VecDeque<Instant>` of hit timestamps, evicted from the front whenever they age out of `window`. `hit()` records a new timestamp, evicts stale ones, and errors if the resulting in-window count exceeds `limit`. One `WindowCounter` is created lazily per IP per counter-kind (E1, bad-sig, control, consent-request) inside `Guard`'s `HashMap`s.
- **calls**: `Instant::now`, `VecDeque` ops
- **called_by**: `Guard::on_e1`/`on_bad_sig`/`on_control`/`on_consent_request`
- **mutates**: its own `times` deque

### `Guard`
- **type**: struct
- **file**: `crates/hsip-net/src/guard.rs`
- **purpose**: The live abuse-prevention state machine for the UDP control plane — per-IP `WindowCounter`s for four event kinds, a set of currently-pinned (auto-trusted) peer IDs with expiry times, aggregate block-event/blocked-IP stats (persisted to `~/.hsip/guard_stats.json` on every new block), and a static IP blocklist ("tracker wall") loaded once at construction from `~/.hsip/tracker_blocklist.txt`.
- **calls**: `load_blocklist` (at construction)
- **called_by**: `udp.rs::listen_control` (one `Guard` per control-plane listener instance)
- **mutates**: its own counters/sets; `~/.hsip/guard_stats.json` on disk (via `mark_blocked`)

### `Guard::new`
- **type**: function
- **file**: `crates/hsip-net/src/guard.rs`
- **purpose**: Constructs a fresh `Guard` from a `GuardCfg`, loading the static tracker-IP blocklist from disk at this point (not re-read afterward — a blocklist change requires restarting the listener to take effect).
- **inputs**: `cfg: GuardCfg`
- **outputs**: `Self`
- **calls**: `load_blocklist`
- **called_by**: `udp.rs::listen_control`
- **mutates**: nothing (reads the blocklist file)

### `Guard::debug_banner`
- **type**: function
- **file**: `crates/hsip-net/src/guard.rs`
- **purpose**: Prints a one-line summary of the active config (all rate limits, size ceilings, padding sizes) plus a note if the tracker wall is non-empty — a startup diagnostic, kept under this name specifically because `udp.rs` calls it by that name (back-compat comment in source).
- **calls**: `eprintln!`
- **called_by**: `udp.rs::listen_control`
- **mutates**: stderr

### `Guard::is_blocklisted` / `mark_blocked` / `blocked_stats`
- **type**: function
- **file**: `crates/hsip-net/src/guard.rs`
- **purpose**: `is_blocklisted` checks the static tracker-wall set. `mark_blocked` (private) increments the in-memory block counters and immediately persists a `GuardStats` snapshot to `~/.hsip/guard_stats.json` — called from every one of the `on_*`/`validate_*` methods below whenever they reject a request, so the stats file is always current after any block, not batched. `blocked_stats` exposes the current `(blocked_events, blocked_ips.len())` tuple to in-process callers (e.g. a status command).
- **outputs**: `bool` / `()` / `(u64, usize)`
- **calls**: `persist_stats`
- **called_by**: every `on_*`/`validate_*` method below (mark_blocked); external status/diagnostic code (blocked_stats)
- **mutates**: `blocked_events`/`blocked_ips` fields; `~/.hsip/guard_stats.json`

### `Guard::on_control_frame` / `on_control` / `on_e1` / `on_bad_sig` / `on_consent_request`
- **type**: function
- **file**: `crates/hsip-net/src/guard.rs`
- **purpose**: The five rate/size gates the control-plane loop calls on every relevant inbound event. `on_control_frame` layers a `max_frame_len` size check on top of `on_control`'s rate check. Each checks the tracker-wall blocklist first (immediate reject + `mark_blocked`, logged to stderr), then hits the appropriate per-IP `WindowCounter`, marking the IP blocked on overflow. All are no-ops (always `Ok(())`) when `cfg.enable` is false.
- **inputs**: `ip: IpAddr`, plus `len: usize` for the size-checked variants
- **outputs**: `Result<(), String>`
- **calls**: `is_blocklisted`, `mark_blocked`, `WindowCounter::hit`
- **called_by**: `udp.rs::receive_e1_initiation` (on_e1), `udp.rs::process_control_messages` (on_control_frame), `udp.rs::evaluate_consent_request` (on_bad_sig) — `on_consent_request` is defined but not currently called from `udp.rs` (no per-request rate gate wired into `evaluate_consent_request` beyond the bad-sig check)
- **mutates**: the relevant per-IP `WindowCounter`; blocked-stats state on rejection

### `Guard::pin` / `is_pinned`
- **type**: function
- **file**: `crates/hsip-net/src/guard.rs`
- **purpose**: `pin` marks a peer ID as trusted for `cfg.pin_minutes` minutes (called after a consent grant). `is_pinned` checks — and opportunistically garbage-collects — whether a pin is still live; an expired pin is removed from both `pinned`/`pin_until` maps on the check that discovers it, rather than by a background sweep.
- **inputs**: `peer_id: &str`
- **outputs**: `()` / `bool`
- **calls**: `Instant::now`
- **called_by**: `udp.rs::evaluate_consent_request` (pin, on an "allow" decision); nothing currently calls `is_pinned` from `udp.rs` — it's exposed but not wired into the control-message-handling rate/trust logic yet
- **mutates**: `pinned`/`pin_until` maps

### `Guard::validate_consent_response_size` / `validate_hello_size`
- **type**: function
- **file**: `crates/hsip-net/src/guard.rs`
- **purpose**: Pure size-ceiling checks (no rate window) for consent responses and HELLO messages — meant to run before expensive downstream work (e.g. signature verification) so an oversized message is rejected cheaply.
- **inputs**: `ip: IpAddr`, `len: usize`
- **outputs**: `Result<(), String>`
- **calls**: `mark_blocked` (on rejection)
- **called_by**: not currently called from `udp.rs`'s control loop (`hello.rs`'s HELLO handling doesn't invoke `Guard` at all) — defined and tested but not yet wired into the live message path
- **mutates**: blocked-stats state on rejection

### `stats_path` / `persist_stats` / `blocklist_path` / `load_blocklist`
- **type**: function (private)
- **file**: `crates/hsip-net/src/guard.rs`
- **purpose**: Filesystem helpers — `stats_path`/`blocklist_path` resolve `~/.hsip/guard_stats.json` and `~/.hsip/tracker_blocklist.txt` respectively (via `dirs::home_dir()`, `None` if unavailable). `persist_stats` best-effort writes the stats JSON (creates the parent dir, silently no-ops on any I/O error — `let _ = ...`). `load_blocklist` reads the blocklist file line-by-line, skipping blank lines and `#`-comments, parsing each remaining line as an `IpAddr` and silently skipping unparseable ones.
- **calls**: `fs::create_dir_all`, `fs::write`/`fs::read_to_string`, `IpAddr::parse`
- **called_by**: `Guard::mark_blocked` (persist_stats), `Guard::new` (load_blocklist)
- **mutates**: `~/.hsip/guard_stats.json` (persist_stats); reads (not writes) the blocklist file

---

## `crates/hsip-net/src/handshake_io.rs`

A tiny demo/diagnostic module — not the real cryptographic handshake (that's `udp.rs`'s `InitiatorHandshake`/`ResponderHandshake`). Backs the `hsip-cli` `handshake-listen`/`handshake-connect` demo subcommands (`crates/hsip-cli/src/commands/handshake.rs`) for manually exercising basic UDP connectivity with a fixed, unsigned payload.

### `recv_and_verify_hello`
- **type**: function
- **file**: `crates/hsip-net/src/handshake_io.rs`
- **purpose**: Binds a UDP socket and blocks forever waiting for exactly one inbound datagram, then prints who it came from and how large it was. Despite the name, performs **no actual verification** — it doesn't check the payload's contents, format, or any signature; "verify" here just means "received something and printed it."
- **inputs**: `bind_addr: &str`
- **outputs**: `std::io::Result<()>`
- **calls**: `UdpSocket::bind`, `UdpSocket::recv_from`
- **called_by**: `hsip-cli`'s `handshake-listen` command (`commands/handshake.rs`)
- **mutates**: nothing (network I/O only, no state)

### `send_hello`
- **type**: function
- **file**: `crates/hsip-net/src/handshake_io.rs`
- **purpose**: Sends a single fixed, unsigned demo payload (`b"HSIP_DEMO_HELLO_v1"`) to the given address from an ephemeral socket — a connectivity smoke-test, not a real HELLO (compare `hello.rs::build_hello`, which produces a signed, structured `Hello`).
- **inputs**: `dest_addr: &str`
- **outputs**: `std::io::Result<()>`
- **calls**: `UdpSocket::bind`, `UdpSocket::send_to`
- **called_by**: `hsip-cli`'s `handshake-connect` command (`commands/handshake.rs`)
- **mutates**: nothing (network I/O only)

---

## `crates/hsip-net/src/hello.rs`

Builds and verifies the real, cryptographically signed `Hello` peer-discovery message — distinct from `handshake_io.rs`'s unsigned demo payload and from `udp.rs`'s `TAG_E1`/`TAG_E2` ephemeral-key handshake. A `Hello` announces a peer's identity, public key, and capability list, self-signed with that peer's long-term Ed25519 identity key.

### `Hello`
- **type**: struct
- **file**: `crates/hsip-net/src/hello.rs`
- **purpose**: Wire format for a signed peer-announcement message: message type tag, base32 peer ID, hex-encoded Ed25519 public key, a capability string list (e.g. `"consent=1"`), a millisecond timestamp, a base64 nonce, and a hex-encoded signature over all of the above.
- **calls**: none
- **called_by**: `build_hello` (constructs), `verify_hello` (validates), `udp.rs::hello::send_hello_with_retry`/`listen_hello`
- **mutates**: nothing

### `detect_local_capabilities`
- **type**: function (private)
- **file**: `crates/hsip-net/src/hello.rs`
- **purpose**: Returns a hardcoded capability list (`pqc=0`, `dtn=1`, `mesh=1`, `sat=0`, `consent=1`) — not actually probed from runtime feature flags, just a fixed announcement of what this build supports.
- **outputs**: `Vec<String>`
- **called_by**: `build_hello`
- **mutates**: nothing

### `generate_signature_payload`
- **type**: function (private)
- **file**: `crates/hsip-net/src/hello.rs`
- **purpose**: Builds the canonical `"HELLO|peer_id|pubkey_hex|caps_joined|ts|nonce"` string that gets signed (by the sender) and reconstructed-and-checked (by the verifier) — the single source of truth for what bytes actually get signed, called identically from both `build_hello` and `verify_hello` so the two can never drift apart.
- **inputs**: `peer_identity: &str`, `pubkey_encoded: &str`, `capability_list: &[String]`, `timestamp: u64`, `nonce_encoded: &str`
- **outputs**: `String`
- **called_by**: `build_hello`, `verify_hello`
- **mutates**: nothing

### `build_hello`
- **type**: function
- **file**: `crates/hsip-net/src/hello.rs`
- **purpose**: Constructs a fully signed `Hello` for the given identity keypair — derives the peer ID from the verifying key, generates a fresh 12-byte random nonce (`OsRng`), assembles the canonical signing payload, and Ed25519-signs it.
- **inputs**: `signing_key: &SigningKey`, `verifying_key: &VerifyingKey`, `current_timestamp_ms: u64`
- **outputs**: `Hello`
- **calls**: `peer_id_from_pubkey`, `generate_signature_payload`, `SigningKey::sign`
- **called_by**: `udp.rs::hello::send_hello_with_retry`
- **mutates**: nothing (reads OS RNG)

### `verify_hello`
- **type**: function
- **file**: `crates/hsip-net/src/hello.rs`
- **purpose**: Independently validates a received `Hello`: decodes and reconstructs the `VerifyingKey` from `pub_key_hex`, checks that the claimed `peer_id` actually derives from that key (binding check — a `Hello` can't claim someone else's peer ID while using its own key), rebuilds the canonical signing payload, and verifies the Ed25519 signature via `verify_strict` (rejects malleable/non-canonical signatures, not just `verify`'s looser check).
- **inputs**: `hello_msg: &Hello`
- **outputs**: `Result<(), String>`
- **calls**: `hex::decode`, `VerifyingKey::from_bytes`, `peer_id_from_pubkey`, `generate_signature_payload`, `VerifyingKey::verify_strict`
- **called_by**: intended verifier of a received `Hello` — not currently called from `udp.rs::hello::listen_hello` (which only strips the wire prefix and prints the raw JSON, doesn't parse/verify it as a `Hello`); available for callers that do want to validate one
- **mutates**: nothing (pure verification)

---

## `crates/hsip-net/src/input_validator.rs`

Pure input-sanitization helpers for the UDP protocol layer — size/format/character-class checks meant to run before any expensive processing (crypto, parsing) on network-supplied strings. All functions are pure and side-effect-free.

### `MAX_MESSAGE_SIZE` / `MAX_CONSENT_PURPOSE_LENGTH` / `MAX_DESTINATION_LENGTH` / `MAX_PEER_ID_LENGTH` / `MAX_SIGNATURE_LENGTH` / `MAX_PUBLIC_KEY_LENGTH` / `MAX_NONCE_LENGTH`
- **type**: variable (constant)
- **file**: `crates/hsip-net/src/input_validator.rs`
- **purpose**: Fixed upper bounds (1100 bytes for messages, 512 for a consent purpose string, 253 for a destination/domain, 64 for a peer ID, 128 for a signature, 64 for a public key, 32 for a nonce) used by the `validate_*` functions below to reject oversized inputs before further processing — sized to stay comfortably under typical UDP MTU limits.
- **called_by**: the corresponding `validate_*` function in this file

### `ValidationError`
- **type**: enum
- **file**: `crates/hsip-net/src/input_validator.rs`
- **purpose**: `TooLarge`/`InvalidFormat`/`InvalidCharacters`/`Empty`, each carrying the offending field's name — a uniform, field-labeled error shape for every validator in this module, with a `Display`/`Error` impl.
- **called_by**: every `validate_*` function

### `validate_destination`
- **type**: function
- **file**: `crates/hsip-net/src/input_validator.rs`
- **purpose**: Accepts either a parseable IP address (any format `std::net::IpAddr` understands, including IPv6) or a domain-name-shaped string (alphanumeric/`.`/`-`/`:` only, not starting or ending with `.`/`-`). Rejects empty, oversized, or malformed values.
- **inputs**: `dest: &str`
- **outputs**: `Result<(), ValidationError>`
- **calls**: `str::parse::<IpAddr>`
- **called_by**: intended for any protocol handler accepting a caller-supplied destination (no current call site found in this workspace — available for callers that add one)
- **mutates**: nothing

### `validate_peer_id`
- **type**: function
- **file**: `crates/hsip-net/src/input_validator.rs`
- **purpose**: Checks a peer ID is non-empty, within `MAX_PEER_ID_LENGTH`, and composed only of valid Base32 characters (`A`-`Z`, `2`-`7` — note this matches `hsip_core::identity`'s base32-derived peer ID format).
- **inputs**: `peer_id: &str`
- **outputs**: `Result<(), ValidationError>`
- **mutates**: nothing

### `validate_hex_string`
- **type**: function
- **file**: `crates/hsip-net/src/input_validator.rs`
- **purpose**: Generic hex-string validator (used for signatures, public keys, etc.) — non-empty, within a caller-supplied max length, all-hex-digit, and even length (hex encodes whole bytes, so an odd-length string can't be decoded).
- **inputs**: `hex: &str`, `max_len: usize`, `field_name: &str`
- **outputs**: `Result<(), ValidationError>`
- **mutates**: nothing

### `validate_message_size`
- **type**: function
- **file**: `crates/hsip-net/src/input_validator.rs`
- **purpose**: Rejects a zero-length or over-`MAX_MESSAGE_SIZE` message.
- **inputs**: `size: usize`
- **outputs**: `Result<(), ValidationError>`
- **mutates**: nothing

### `sanitize_for_log`
- **type**: function
- **file**: `crates/hsip-net/src/input_validator.rs`
- **purpose**: Strips control characters (except newline/tab) from a string and truncates to 256 chars before it's written to a log — prevents a malicious peer from injecting terminal escape sequences or forging fake log lines via crafted input, and bounds log-line size.
- **inputs**: `s: &str`
- **outputs**: `String`
- **mutates**: nothing
- **called_by**: no current call site in this workspace found — a hardening helper available to any code logging peer-supplied strings

### `validate_nonce`
- **type**: function
- **file**: `crates/hsip-net/src/input_validator.rs`
- **purpose**: Checks a nonce is non-empty, within `MAX_NONCE_LENGTH`, and composed of alphanumeric or base64 characters (`+`/`/`/`=`).
- **inputs**: `nonce: &str`
- **outputs**: `Result<(), ValidationError>`
- **mutates**: nothing

---

## `crates/hsip-net/src/rate_limiter.rs`

Like `connection_guard.rs`, this module's doc comment states **"STATUS: NOT CURRENTLY INTEGRATED"** — a token-bucket-based rate limiter with connection-count tracking and a 3-strikes IP ban, fully implemented and tested but not called from `udp.rs` or anywhere else in the workspace. `guard.rs`'s sliding-window `Guard` is the module actually wired into the live control-plane path.

### `RateLimitConfig`
- **type**: struct
- **file**: `crates/hsip-net/src/rate_limiter.rs`
- **purpose**: Tunables for `RateLimiter` — requests/sec (default 100), burst capacity (200 tokens), ban duration (5 min) after repeated violations, and max connections per IP (10).
- **called_by**: `RateLimiter::new` (unused elsewhere)

### `TokenBucket` (private)
- **type**: struct
- **file**: `crates/hsip-net/src/rate_limiter.rs`
- **purpose**: Classic token-bucket per IP — refills continuously based on elapsed time (`tokens += elapsed_secs * rate`, capped at `burst_capacity`), tracks a violation count, and bans the IP (sets `banned_until`) once violations reach 3.
- **calls**: `Instant::now`
- **called_by**: `RateLimiter::check_request`
- **mutates**: its own `tokens`/`last_refill`/`violations`/`banned_until` fields

### `TokenBucket::refill` / `is_banned` / `try_consume`
- **type**: function (private)
- **file**: `crates/hsip-net/src/rate_limiter.rs`
- **purpose**: `refill` updates the token count based on elapsed time since last refill. `is_banned` checks whether `banned_until` is still in the future. `try_consume` is banned-check → refill → consume-one-token-or-record-violation, printing a stderr line and setting a 5-minute (configurable) ban once 3 violations accumulate.
- **outputs**: `()` / `bool` / `bool`
- **calls**: `Instant::now`
- **called_by**: `RateLimiter::check_request` (try_consume, which calls the other two internally)
- **mutates**: bucket fields; prints to stderr on ban

### `RateLimiter`
- **type**: struct
- **file**: `crates/hsip-net/src/rate_limiter.rs`
- **purpose**: Per-IP `HashMap<IpAddr, TokenBucket>` (behind an `Arc<RwLock<_>>`) for request-rate limiting, plus a separate per-IP connection counter for `check_connection`/`release_connection`. Not integrated into any live listener in this workspace — see module-level status note above.
- **calls**: none
- **called_by**: nothing in production code — only its own `#[cfg(test)]` module
- **mutates**: its `buckets`/`connections` maps

### `RateLimiter::check_request` / `check_connection` / `release_connection` / `cleanup`
- **type**: function
- **file**: `crates/hsip-net/src/rate_limiter.rs`
- **purpose**: `check_request` looks up (or creates) an IP's bucket and applies `try_consume`. `check_connection`/`release_connection` are a simple per-IP counter gated at `max_connections_per_ip`, incrementing/decrementing (and removing the entry once it hits zero). `cleanup` (meant to be called periodically by an external caller — nothing in this workspace does) prunes buckets untouched for 5+ minutes (unless still banned) and zero-count connection entries.
- **inputs**: `ip: IpAddr`
- **outputs**: `Result<(), RateLimitError>` (check_request/check_connection) / `()` (release_connection, cleanup)
- **calls**: `TokenBucket::try_consume`/`is_banned`
- **called_by**: nothing in production code (unintegrated)
- **mutates**: `buckets`/`connections` maps

### `RateLimitError`
- **type**: enum
- **file**: `crates/hsip-net/src/rate_limiter.rs`
- **purpose**: `RateExceeded` | `Banned` | `TooManyConnections`, with `Display`/`Error` impls.
- **called_by**: `RateLimiter`'s check methods

---

## `crates/hsip-net/src/tls_wrapper.rs`

**Important caveat found while reading this file, not documented anywhere in its own comments:** despite the module doc comment's claims about TLS 1.3, MITM protection, and downgrade-attack resistance, `TlsStream::connect`'s actual implementation performs **no real TLS handshake at all** — it opens a plain `TcpStream` and wraps it in `MockTlsStream`, whose `peer_certificate_valid()` unconditionally returns `true` and which reads/writes the raw, unencrypted TCP bytes straight through. The module comment itself flags this: `MockTlsStream`'s doc says "replace with rustls in production." As it stands, any code that actually used this module for a "TLS-wrapped" connection would get zero cryptographic protection while believing certificate verification succeeded. Not currently called from anywhere in this workspace (grep found no call sites outside its own test module) — a stub/placeholder API surface, not a wired-in security layer.

### `TlsConfig`
- **type**: struct
- **file**: `crates/hsip-net/src/tls_wrapper.rs`
- **purpose**: Declared TLS policy — minimum version (defaults to `Tls13`), allowed cipher suites, whether to verify certificates (default `true`), connect timeout (10s), and whether to require perfect forward secrecy (default `true`). None of these fields are actually enforced by `TlsStream::connect`'s real (mock) implementation beyond `connect_timeout`.
- **called_by**: `TlsStream::connect`

### `TlsVersion` / `CipherSuite`
- **type**: enum
- **file**: `crates/hsip-net/src/tls_wrapper.rs`
- **purpose**: `TlsVersion` (`Tls12`/`Tls13`, ordered). `CipherSuite` lists three TLS 1.3 AEAD suites (AES-256-GCM/SHA-384, ChaCha20-Poly1305/SHA-256, AES-128-GCM/SHA-256) — declarative labels only; no cipher is actually negotiated since no real TLS handshake occurs.
- **called_by**: `TlsConfig::default`, `TlsStream`

### `TlsStream`
- **type**: struct
- **file**: `crates/hsip-net/src/tls_wrapper.rs`
- **purpose**: Wraps a boxed `TlsStreamTrait` object (currently always `MockTlsStream` — see caveat above) along with the peer address and (claimed, not negotiated) cipher suite. Implements `Read`/`Write` by delegating to the inner stream.
- **calls**: inner stream's `read`/`write`
- **called_by**: none in this workspace outside its own tests

### `TlsStream::connect`
- **type**: function
- **file**: `crates/hsip-net/src/tls_wrapper.rs`
- **purpose**: Validates hostname (non-empty, ≤253 chars) and port (non-zero), opens a real `TcpStream::connect_timeout`, sets 30s read/write timeouts, then wraps it in `MockTlsStream` — **no TLS handshake happens here** (see module-level caveat). Returns `Self` with `cipher_suite` hardcoded to `Tls13Chacha20Poly1305Sha256` regardless of what actually happened on the wire.
- **inputs**: `host: &str`, `port: u16`, `config: &TlsConfig`
- **outputs**: `Result<Self, TlsError>`
- **calls**: `TcpStream::connect_timeout`, `MockTlsStream::new`
- **called_by**: none in this workspace outside its own tests
- **mutates**: opens a real OS-level TCP connection

### `TlsStream::cipher_suite` / `peer_address` / `is_tls13` / `verify_peer`
- **type**: function
- **file**: `crates/hsip-net/src/tls_wrapper.rs`
- **purpose**: Simple accessors. `verify_peer` calls the inner stream's `peer_certificate_valid()`, which for `MockTlsStream` always returns `true` — so `verify_peer()` can never actually fail against the current implementation, regardless of `TlsConfig::verify_certificates`.
- **outputs**: `Option<CipherSuite>` / `&str` / `bool` / `Result<(), TlsError>`
- **called_by**: none in this workspace outside its own tests

### `MockTlsStream`
- **type**: struct (private)
- **file**: `crates/hsip-net/src/tls_wrapper.rs`
- **purpose**: The placeholder "TLS" stream — just a raw `TcpStream` underneath. `peer_certificate_valid()` is hardcoded `true`; `Read`/`Write` pass straight through to the plain TCP socket with zero encryption. Its own doc comment says to "replace with rustls in production," confirming this was always meant to be swapped out, not a finished feature.
- **calls**: `TcpStream::read`/`write`/`flush`
- **called_by**: `TlsStream::connect` (the only constructor path)
- **mutates**: the underlying TCP stream

### `TlsError`
- **type**: enum
- **file**: `crates/hsip-net/src/tls_wrapper.rs`
- **purpose**: `InvalidHostname` | `InvalidPort` | `ConnectionFailed` | `HandshakeFailed` | `CertificateVerificationFailed` | `UnsupportedVersion` | `WeakCipherSuite` — several of these variants (`HandshakeFailed`, `UnsupportedVersion`, `WeakCipherSuite`) are declared but never actually produced anywhere in this file, since no real handshake/cipher-negotiation logic exists yet to fail in those ways.
- **called_by**: `TlsStream::connect`, `TlsStream::verify_peer`

---

## `crates/hsip-net/src/udp.rs`

The real, live UDP transport and control-plane protocol — the one module in this crate genuinely wired end-to-end and used by `hsip-cli`. Implements an X25519 ephemeral-key handshake (`TAG_E1`/`TAG_E2`), ChaCha20-Poly1305-backed encrypted control messages (`TAG_D`) carrying the consent-request/response protocol from `hsip-core::consent`, integration with `guard.rs`'s `Guard` for abuse prevention, `consent_cache.rs`'s `SharedConsentCache` for instant revocation, and optional reputation-based filtering (`hsip-reputation`) plus decoy traffic (`HSIP_DECOY_ADDR`) for basic traffic-analysis resistance. Also re-exports a small `hello` submodule (distinct from top-level `hello.rs`) providing signed-HELLO send/listen helpers used directly by `hsip-cli`.

### `hello::listen_hello`
- **type**: function
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: Binds a blocking UDP socket and loops forever, printing any inbound packet that has a valid HSIP wire prefix (stripping the prefix and printing the rest as a lossy UTF-8 string) — does **not** parse it into a `Hello` struct or call `hello.rs::verify_hello`, so a malicious peer's unsigned/invalid payload is printed identically to a real signed `Hello`. Purely a debug/demo listener.
- **inputs**: `addr: &str`
- **outputs**: `Result<()>`
- **calls**: `UdpSocket::bind`, `hsip_core::wire::prefix::check_prefix`
- **called_by**: `hsip-cli`'s hello-listen command (`main.rs`)
- **mutates**: nothing (network I/O only)

### `hello::send_hello` / `hello::send_hello_with_retry`
- **type**: function
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: Builds a real signed `Hello` (via the top-level `hello.rs::build_hello`), wraps it with the HSIP wire prefix, and sends it via UDP with exponential-backoff retry (0ms/1s/2s/4s, 3 retries by default → ~7s total timeout) — meant for UDP's inherent unreliability, unlike `handshake_io.rs::send_hello`'s single unsigned demo send. Rejects the packet outright if it exceeds `hsip_core::wire::MAX_HELLO_SIZE` (MTU-safety), before ever attempting to send.
- **inputs**: `sk: &SigningKey`, `vk: &VerifyingKey`, `to: &str`, `now_ms: u64`, (send_hello_with_retry adds) `max_retries: u32`
- **outputs**: `Result<()>`
- **calls**: `hello::build_hello` (top-level module), `hsip_core::wire::prefix::write_prefix`, `UdpSocket::send_to`
- **called_by**: `hsip-cli`'s hello-connect command (`main.rs`)
- **mutates**: nothing (network I/O; sleeps between retries)

### `random_salt` / `derive_session_key_from_shared` / `hsip_consent_label`
- **type**: function (private)
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: Crypto helpers underpinning session setup. `random_salt` generates a 4-byte `OsRng` salt for `ManagedSession::new`. `derive_session_key_from_shared` runs HKDF-SHA256 over an X25519 shared secret, using the peer label's bytes as the HKDF `info` parameter, to produce the actual AEAD session key. `hsip_consent_label` returns the fixed `b"CONSENTv1"` label used to domain-separate this protocol's derived keys from any other use of the same shared secret.
- **calls**: `Hkdf::<Sha256>::new`/`expand`, `OsRng::fill_bytes`
- **called_by**: `InitiatorHandshake::complete_exchange`, `ResponderHandshake::finalize_sessions`
- **mutates**: nothing (random_salt reads OS RNG)

### `InitiatorHandshake` / `ResponderHandshake`
- **type**: struct (private)
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: The two sides of the ephemeral X25519 key exchange. `InitiatorHandshake` generates its own ephemeral keypair, builds the `TAG_E1` packet to send first, and later completes the exchange once it receives the responder's `TAG_E2` public key, producing one `ManagedSession`. `ResponderHandshake` is constructed directly from a received E1 key, computes the shared secret immediately, builds the `TAG_E2` reply packet, and produces two independent sessions (`rx_session`/`tx_session`) from the same shared secret — separate session objects for each direction, both derived under the same `CONSENTv1` label (direction is distinguished by which socket/rekey state each side actually uses it for, not by a different label).
- **calls**: `Ephemeral::generate`/`into_shared`, `derive_session_key_from_shared`, `ManagedSession::new`
- **called_by**: `perform_client_exchange` (Initiator), `listen_control` (Responder)
- **mutates**: nothing (constructs new session state)

### `spawn_decoy_if_env`
- **type**: function (private)
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: If `HSIP_DECOY_ADDR` is set, spawns a background thread that binds a UDP socket and, for every inbound packet, replies after a small pseudo-random delay with a malformed-looking HSIP-prefixed packet (tag `0xFF`, padded to a variable length derived from the input size and a rolling counter) — basic traffic-analysis resistance, making a passive observer's job of distinguishing real control traffic from noise slightly harder. No-op if the env var is unset or empty.
- **calls**: `UdpSocket::bind`, `std::thread::spawn`, `hsip_core::wire::prefix::write_prefix`
- **called_by**: `listen_control`
- **mutates**: spawns a detached background thread; opens a UDP socket

### `listen_control`
- **type**: function
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: The main server-side entry point for the control plane — sets up `Guard`, `PolicyCfg` (reputation enforcement config from env), a `SharedConsentCache` (5-minute TTL), binds the listening socket, optionally spawns the decoy responder, waits for exactly one E1 handshake (single-peer-at-a-time design — see gotcha below), completes the responder handshake, loads this node's own signing identity (via `hsip_core::keystore::load_keypair`), then hands off to `process_control_messages` for the actual message loop.
- **inputs**: `addr: &str`
- **outputs**: `Result<()>`
- **calls**: `Guard::new`/`debug_banner`, `PolicyCfg::from_env`/`print_banner`, `SharedConsentCache::new`, `UdpSocket::bind`, `spawn_decoy_if_env`, `receive_e1_initiation`, `ResponderHandshake::from_received_e1`/`build_e2_packet`/`finalize_sessions`, `hsip_core::keystore::load_keypair`, `process_control_messages`
- **called_by**: `hsip-cli`'s control-listen command
- **mutates**: binds a socket; loads identity from disk; spawns the decoy thread if configured

### `receive_e1_initiation`
- **type**: function (private)
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: Blocks (polling with a 1ms sleep on `WouldBlock`) until an inbound datagram matches the E1 frame shape (correct wire prefix, `TAG_E1`, long enough to contain a 32-byte key), silently discarding anything else (malformed frames, non-E1 tags) rather than erroring — an attacker sending garbage just gets ignored, not rate-limited itself at this stage beyond `guard.on_e1` being called once a valid-shaped E1 is seen.
- **inputs**: `sock: &UdpSocket`, `guard: &mut Guard`
- **outputs**: `Result<(SocketAddr, XPublicKey)>`
- **calls**: `UdpSocket::recv_from`, `hsip_core::wire::prefix::check_prefix`, `Guard::on_e1`
- **called_by**: `listen_control`
- **mutates**: `guard`'s E1 rate-window state

### `process_control_messages`
- **type**: function (private)
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: The main receive loop after handshake completion — for every inbound `TAG_D` frame, runs it through `guard.on_control_frame` (size + rate check), decrypts it via the rx session, and dispatches to `handle_control_message`. Non-control frames are silently ignored; `WouldBlock` triggers a 5ms sleep; a genuine socket error other than the Windows `WSAECONNRESET` special-case propagates and ends the loop. Marked `#[allow(clippy::too_many_arguments)]` per this codebase's documented precedent (mirrors `hsip-net`'s own `udp.rs::handle_control_message` and the workspace-wide clippy cleanup noted in `CLAUDE.md`).
- **inputs**: `sock: &UdpSocket`, `rx_session: &mut ManagedSession`, `tx_session: &mut ManagedSession`, `guard: &mut Guard`, `policy: &PolicyCfg`, `signing_key: &SigningKey`, `verifying_key: &VerifyingKey`, `consent_cache: &SharedConsentCache`
- **outputs**: `Result<()>` (never returns on the happy path — runs until an unrecoverable I/O error)
- **calls**: `Guard::on_control_frame`, `decrypt_control_frame`, `handle_control_message`
- **called_by**: `listen_control`
- **mutates**: session/guard state indirectly via the functions it calls; loops forever

### `decrypt_control_frame`
- **type**: function (private)
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: Parses a `TAG_D` frame's 8-byte big-endian counter and ciphertext, then decrypts via `ManagedSession::decrypt` with the fixed `AAD_LABEL_E2` associated data. Returns `None` (not an error) on a decrypt failure or a `RekeyRequired` result — the latter is explicitly logged as "not implemented," meaning this protocol has no working rekey mechanism yet and a session needing one will simply have its messages silently dropped from that point on.
- **inputs**: `raw_frame: &[u8]`, `session: &mut ManagedSession`, `aad: &[u8]`
- **outputs**: `Option<Vec<u8>>`
- **calls**: `ManagedSession::decrypt`
- **called_by**: `process_control_messages`
- **mutates**: session's internal replay/counter state (via `decrypt`)

### `handle_control_message`
- **type**: function (private)
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: Dispatches a decrypted plaintext by trying to parse it as (in order) a `ConsentRequest`, then a `ConsentResponse`, then falling back to printing it as raw JSON if neither matches. For a `ConsentRequest`: evaluates the decision via `evaluate_consent_request`, and if granted, wires up **instant revocation** by attaching a `SharedConsentCache`-backed check callback to both the rx and tx sessions (so a later `revoke()` call on that peer ID immediately breaks encrypt/decrypt on this exact session, not just future new sessions) before signing and sending back a `ConsentResponse`. For a `ConsentResponse`, only logs it — no further protocol action (the client side that requested it handles interpretation separately). `#[allow(clippy::too_many_arguments)]`, same reasoning as `process_control_messages`.
- **inputs**: `plaintext: Vec<u8>`, `peer_addr: SocketAddr`, `sock: &UdpSocket`, `rx_session/tx_session: &mut ManagedSession`, `guard: &mut Guard`, `policy: &PolicyCfg`, `reputation_store: &mut Option<Store>`, `signing_key: &SigningKey`, `verifying_key: &VerifyingKey`, `aad: &[u8]`, `consent_cache: &SharedConsentCache`
- **outputs**: `Result<()>`
- **calls**: `evaluate_consent_request`, `SharedConsentCache::create_check_callback`/`insert_allow`, `ManagedSession::attach_consent_check`, `build_response_with_decision`, `send_encrypted_response`
- **called_by**: `process_control_messages`
- **mutates**: attaches consent-check closures to both sessions; inserts into the shared consent cache; sends a UDP response

### `evaluate_consent_request`
- **type**: function (private)
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: Decides `"allow"` or `"deny"` for an inbound `ConsentRequest`. First checks the request's own embedded signature via `hsip_core::consent::validate_request` (invalid → deny, and flags the sender's IP via `guard.on_bad_sig`). If still allowed and reputation enforcement is on (`HSIP_ENFORCE_REP=1`), an empty requester ID is auto-denied, otherwise a `hsip_reputation::store::Store` is lazily opened (from `~/.hsip/reputation.log`) and the requester's score checked against `HSIP_REP_THRESHOLD` (default -6) — below threshold denies. A granted decision also pins the requester in `Guard` for future auto-trust.
- **inputs**: `request: &ConsentRequest`, `peer_addr: SocketAddr`, `guard: &mut Guard`, `policy: &PolicyCfg`, `reputation_store: &mut Option<Store>`
- **outputs**: `Result<String>` (`"allow"` or `"deny"`)
- **calls**: `hsip_core::consent::validate_request`, `Guard::on_bad_sig`/`pin`, `hsip_reputation::store::Store::open`/`compute_score`
- **called_by**: `handle_control_message`
- **mutates**: `guard`'s bad-sig window / pin state; lazily opens and reads the reputation store (first call per loop iteration only — the `Option` caches it across messages within one `process_control_messages` invocation)

### `send_encrypted_response`
- **type**: function (private)
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: Serializes a `ConsentResponse` to JSON, encrypts it via the tx session, frames it with the wire prefix + `TAG_D` + big-endian counter, and sends it — errors from `sock.send_to` are explicitly swallowed (`.ok()`), so a failed reply send is silent from this function's perspective (the caller has no way to detect it failed).
- **inputs**: `sock: &UdpSocket`, `session: &mut ManagedSession`, `response: &ConsentResponse`, `dest: SocketAddr`, `aad: &[u8]`
- **outputs**: `Result<()>`
- **calls**: `ManagedSession::encrypt`, `hsip_core::wire::prefix::write_prefix`
- **called_by**: `handle_control_message`
- **mutates**: tx session's internal counter/replay state; sends a UDP packet

### `send_consent_request` / `send_consent_response`
- **type**: function
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: Public client-side entry points — serialize the given `ConsentRequest`/`ConsentResponse` to JSON and hand it to `perform_client_exchange`, which does a full fresh handshake per call (no session reuse across separate `send_consent_request`/`send_consent_response` invocations).
- **inputs**: `to: &str`, `req: &ConsentRequest` / `resp: &ConsentResponse`
- **outputs**: `Result<()>`
- **calls**: `perform_client_exchange`
- **called_by**: `hsip-cli`'s consent-request/consent-response commands (`main.rs`)
- **mutates**: nothing directly (delegates)

### `perform_client_exchange`
- **type**: function (private)
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: Full client-side round trip for one payload: binds an ephemeral socket (2s read timeout), sends E1, blocks for the E2 reply, completes the handshake into one session, encrypts the payload, and sends it as a `TAG_D` frame — a single fire-and-forget send with no acknowledgment that the server actually processed it (the caller only knows the local send succeeded, not that a response arrived, since this function returns before any reply is awaited).
- **inputs**: `server_addr: &str`, `payload: &[u8]`
- **outputs**: `Result<()>`
- **calls**: `InitiatorHandshake::new`/`build_e1_packet`/`complete_exchange`, `receive_e2_response`, `ManagedSession::encrypt`
- **called_by**: `send_consent_request`, `send_consent_response`
- **mutates**: nothing beyond the ephemeral session/socket it creates

### `receive_e2_response`
- **type**: function (private)
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: Reads one datagram and validates it's a well-formed `TAG_E2` frame (correct prefix, minimum length, correct tag byte), extracting the responder's X25519 public key. Errors (rather than silently retrying) on any malformed frame — unlike the server-side `receive_e1_initiation`, which silently discards bad input and keeps waiting.
- **inputs**: `sock: &UdpSocket`
- **outputs**: `Result<XPublicKey>`
- **calls**: `UdpSocket::recv_from`, `hsip_core::wire::prefix::check_prefix`
- **called_by**: `perform_client_exchange`
- **mutates**: nothing

### `is_windows_connection_reset`
- **type**: function (private)
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: Checks whether an I/O error's raw OS error code is `10054` (`WSAECONNRESET`) — a Windows-specific quirk where a previous ICMP port-unreachable response can cause a subsequent unrelated `recv_from` to fail spuriously on a connectionless UDP socket. Both `spawn_decoy_if_env`'s loop and `process_control_messages` special-case this by continuing the loop instead of treating it as a fatal error.
- **inputs**: `e: &std::io::Error`
- **outputs**: `bool`
- **called_by**: `spawn_decoy_if_env`, `process_control_messages`
- **mutates**: nothing

### `build_response_with_decision`
- **type**: function (private)
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: Constructs and self-signs a `ConsentResponse`: hashes the original request (SHA-256) to bind the response to it, fills in the responder's peer ID/pubkey, sets a TTL, then signs the response with `sig_hex` temporarily cleared (so the signature never signs over itself) before restoring the computed signature into the final struct.
- **inputs**: `sk: &SigningKey`, `vk: &VerifyingKey`, `req: &ConsentRequest`, `decision: String`, `ttl_ms: u64`
- **outputs**: `Result<ConsentResponse>`
- **calls**: `Sha256::update`/`finalize`, `peer_id_from_pubkey`, `vk_to_hex`, `SigningKey::sign`, `current_timestamp_ms`
- **called_by**: `handle_control_message`
- **mutates**: nothing

### `current_timestamp_ms`
- **type**: function (private)
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: Returns the current Unix time in milliseconds. Panics (via `.unwrap()`) if system time is set before the Unix epoch — an accepted, extremely unlikely edge case, not defensively handled.
- **outputs**: `u64`
- **called_by**: `build_response_with_decision`
- **mutates**: nothing

### `PolicyCfg`
- **type**: struct (private)
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: Reputation-enforcement config read from env at startup: `HSIP_ENFORCE_REP=1` enables score-based filtering of consent requesters, `HSIP_REP_THRESHOLD` sets the minimum acceptable score (default -6), and the reputation log path is fixed at `~/.hsip/reputation.log`.
- **calls**: `std::env::var`, `dirs::home_dir`
- **called_by**: `listen_control` (constructs via `from_env`), `evaluate_consent_request` (reads)
- **mutates**: nothing

### `PolicyCfg::from_env` / `print_banner`
- **type**: function
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: `from_env` reads the three env-derived fields above. `print_banner` logs a one-line summary to stderr, plus a separate note if `HSIP_REQUIRE_VALID_SIG=1` is set (though nothing in this file currently reads that variable beyond the banner text itself — it's not wired into `evaluate_consent_request`'s actual signature-checking logic, which always validates the request signature regardless of this flag).
- **outputs**: `Self` / `()`
- **called_by**: `listen_control`
- **mutates**: stderr (print_banner)

### `TAG_E1` / `TAG_E2` / `TAG_D`
- **type**: variable (constant)
- **file**: `crates/hsip-net/src/udp.rs`
- **purpose**: Single-byte wire-protocol frame tags — `0xE1` initial ephemeral key exchange, `0xE2` responder's ephemeral key reply, `0xD0` an encrypted data/control frame. Every frame on this UDP transport is prefixed with the shared HSIP wire prefix (`hsip_core::wire::prefix`) followed by one of these tag bytes.
- **called_by**: every packet-building/parsing function in this file

---

## `crates/hsip-auth/src/lib.rs`

Crate root for `hsip-auth` — a small, standalone device-identity and JWT-like-token module distinct from `hsip-api`'s tenant/API-key auth system. Per `CLAUDE.md`, this is a "supporting crate, not actively integrated" into the main HTTP product — its real consumer is `hsip-cli`, which uses it for a device-local identity (separate from the HSIP HTTP server's own tenant identity) and for issuing short-lived consent tokens. `keystore` is also re-exported under the alias `key_storage`. A private `auth_internal` module contains a single reserved, unused placeholder function (`_reserved_for_auth_expansion`) with no current purpose — literally a no-op stub for future use.

### `key_storage`
- **type**: module (re-export alias)
- **file**: `crates/hsip-auth/src/lib.rs`
- **purpose**: `#[doc(inline)] pub use keystore as key_storage` — lets callers refer to this module by either name; no distinct behavior.
- **called_by**: nothing found using the `key_storage` alias in this workspace (all call sites use `hsip_auth::keystore` or the crate's own `crate::keystore` directly)

---

## `crates/hsip-auth/src/identity.rs`

Manages a single device-local Ed25519 identity persisted via `keystore.rs` — this is **not** the same identity as `hsip-core::identity`/`hsip-api`'s tenant identities; it's a separate keypair used specifically by this crate's token-issuance flow and by `hsip-cli`'s device-identity-related subcommands.

### `ensure_device_identity`
- **type**: function
- **file**: `crates/hsip-auth/src/identity.rs`
- **purpose**: Loads the existing device identity from `keystore::load()` if present; on any load failure (most commonly "file doesn't exist yet"), generates a brand-new Ed25519 keypair and persists it via `keystore::save`. The "on any error, just create a new one" behavior means a corrupted (not just missing) key file would also silently be overwritten with a fresh identity rather than surfacing the corruption as a distinct error.
- **outputs**: `Result<(SigningKey, VerifyingKey)>`
- **calls**: `keystore::load`, `create_and_store_new_identity` → `keystore::save`
- **called_by**: `peer_id_b64`, `public_key_hex`, `tokens::issue_consent`, `hsip-cli`'s `main.rs` (`auth_identity::peer_id_b64` call sites)
- **mutates**: writes `~/.hsip/id_auth.json` the first time it's called with no existing key

### `create_and_store_new_identity`
- **type**: function (private)
- **file**: `crates/hsip-auth/src/identity.rs`
- **purpose**: Generates a fresh Ed25519 keypair via `OsRng` and immediately persists it to disk.
- **outputs**: `Result<(SigningKey, VerifyingKey)>`
- **calls**: `SigningKey::generate`, `keystore::save`
- **called_by**: `ensure_device_identity`
- **mutates**: writes `~/.hsip/id_auth.json`

### `peer_id_b64`
- **type**: function
- **file**: `crates/hsip-auth/src/identity.rs`
- **purpose**: Returns this device's identity public key, standard-base64-encoded — note this is a different encoding (standard base64) from `hsip-core::identity::peer_id_from_pubkey`'s base32 peer-ID format used elsewhere in the workspace (e.g. `hello.rs`'s `Hello.peer_id`), so a `hsip-auth` peer ID and an `hsip-core`/`hsip-net` peer ID for the same underlying key are not interchangeable strings.
- **outputs**: `Result<String>`
- **calls**: `ensure_device_identity`
- **called_by**: `hsip-cli`'s `main.rs` (device-identity display/status commands)
- **mutates**: nothing directly (may trigger identity creation via `ensure_device_identity`)

### `public_key_hex`
- **type**: function
- **file**: `crates/hsip-auth/src/identity.rs`
- **purpose**: Returns this device's identity public key, hex-encoded — used as the JWT `kid` (key ID) header field in `tokens.rs::issue_consent`.
- **outputs**: `Result<String>`
- **calls**: `ensure_device_identity`
- **called_by**: `tokens::issue_consent`
- **mutates**: nothing directly (may trigger identity creation)

---

## `crates/hsip-auth/src/keystore.rs`

Raw filesystem persistence for the `hsip-auth` device identity — a **plaintext** JSON key file, distinct from and much simpler than `hsip-api`'s `key_encryption.rs` (which encrypts signing keys at rest with ChaCha20-Poly1305 + HKDF from a master key). This module has no encryption at all: the raw 32-byte Ed25519 seed is written to disk as a hex string in `~/.hsip/id_auth.json` with no access-control or permission-mode hardening (no `0o600` chmod call, unlike the invariant `CLAUDE.md` documents for `hsip-api`'s master-key/admin-key files) — a real gap if this crate's identity is ever depended on for anything security-sensitive in production use, though its current integration is limited (see crate-level note above).

### `path`
- **type**: function (private)
- **file**: `crates/hsip-auth/src/keystore.rs`
- **purpose**: Resolves the fixed identity file location, `~/.hsip/id_auth.json`. Uses `dirs::home_dir().expect("home")` — will panic outright if no home directory can be determined, unlike `hsip-net::config.rs`'s equivalent path resolution, which falls back to `.` instead of panicking.
- **outputs**: `PathBuf`
- **called_by**: `load`, `save`
- **mutates**: nothing

### `load`
- **type**: function
- **file**: `crates/hsip-auth/src/keystore.rs`
- **purpose**: Reads and parses `~/.hsip/id_auth.json`, extracts the `sk_hex` field, decodes it as hex into a 32-byte seed, and reconstructs both the `SigningKey` and its `VerifyingKey`. Fails (propagates via `?`) if the file doesn't exist, isn't valid JSON, or is missing `sk_hex` — this is the failure path `identity::ensure_device_identity` catches to trigger fresh-key generation.
- **outputs**: `Result<(SigningKey, VerifyingKey)>`
- **calls**: `fs::read_to_string`, `serde_json::from_str`, `hex::decode`, `SigningKey::from_bytes`
- **called_by**: `identity::ensure_device_identity`
- **mutates**: nothing (read-only)

### `save`
- **type**: function
- **file**: `crates/hsip-auth/src/keystore.rs`
- **purpose**: Writes the signing key's seed and verifying key, both hex-encoded, plus a literal `"note": "HSIP auth identity (device-local). KEEP PRIVATE."` reminder string, as pretty-printed JSON to `~/.hsip/id_auth.json` — creating the parent directory first if needed (best-effort, `.ok()`-swallowed). Sets **no explicit file permissions** — writes with whatever the process umask allows, the same class of gap `CLAUDE.md`'s Key Invariants section documents (and requires `0o600` for) on `hsip-api`'s master-key file, but that fix was never applied here.
- **inputs**: `sk: &SigningKey`, `vk: &VerifyingKey`
- **outputs**: `Result<()>`
- **calls**: `fs::create_dir_all`, `fs::write`, `serde_json::to_string_pretty`
- **called_by**: `identity::create_and_store_new_identity`
- **mutates**: writes `~/.hsip/id_auth.json` (world-readable under a typical default umask — see caveat above)

---

## `crates/hsip-auth/src/tokens.rs`

Issues a compact, hand-rolled JWT-like token (`base64url(header).base64url(payload).base64url(signature)`) signed with the `hsip-auth` device identity's Ed25519 key, for scoped, time-limited "consent" tokens. **Notably, this module only issues tokens — there is no corresponding `verify`/`decode` function in this file** (or found elsewhere in this crate); whatever consumes these tokens must implement its own verification against the device's known public key. Also distinct from `hsip-api`'s own JWT usage (`hsip-cli/src/identity.rs`'s `identity-serve` broker, referenced in `CLAUDE.md`'s Security Self-Review section) — this is `hsip-auth`'s own, separate token format.

### `JwsHeader` / `Claims` (private)
- **type**: struct
- **file**: `crates/hsip-auth/src/tokens.rs`
- **purpose**: `JwsHeader` — fixed `alg: "EdDSA"`, `typ: "JWT"`, and a `kid` set to the device's hex public key. `Claims` — standard-shaped JWT claims (`iss` fixed to `"hsip-device"`, `sub` fixed to `"device"`, caller-supplied `aud`, `iat`/`exp` Unix timestamps, and a caller-supplied `scopes` list).
- **called_by**: `issue_consent`

### `issue_consent`
- **type**: function
- **file**: `crates/hsip-auth/src/tokens.rs`
- **purpose**: Builds and signs one of these tokens: resolves (or creates) the device identity, sets `iat`=now and `exp`=now+`ttl_secs`, base64url-encodes (no padding) the header and claims JSON, Ed25519-signs the `"{header}.{claims}"` string, and returns the three-part dot-joined token. Computes a SHA-256 digest of the signing input (`_digest`) but never uses or returns it — dead computation left in, per its own comment, as "not strictly needed for EdDSA, but ok to leave out" (i.e., a leftover from an earlier design, harmless but wasted work on every call).
- **inputs**: `scopes: &[&str]`, `ttl_secs: u64`, `aud: &str`
- **outputs**: `Result<String>`
- **calls**: `identity::ensure_device_identity`, `identity::public_key_hex`, `SigningKey::sign`, `base64` URL_SAFE_NO_PAD encoding
- **called_by**: `hsip-cli`'s `main.rs` (issues a consent token for a caller-specified audience/scope, e.g. around line 2193)
- **mutates**: nothing directly (may trigger device-identity creation via `ensure_device_identity`)

---
## `crates/hsip-intercept/src/lib.rs`

Crate root for `hsip-intercept`, HSIP's cross-platform "Private DM Intercept" system: it watches for the user about to send a message through a traditional platform (Instagram, Gmail, WhatsApp, etc.) via OS-level accessibility/window events, matches the event against known UI patterns, and offers to route the message through HSIP's own consent-based protocol instead. Declares the always-compiled modules (`config`, `error`, `event`, `overlay`, `patterns`, `privacy`, `router`) plus one `#[cfg(target_os = "...")]`-gated platform module (`windows`/`android`/`linux`/`macos`) that supplies the concrete `EventMonitor`/`InterceptOverlay` implementations `InterceptCoordinator::new` wires together. The module doc's "Platform Support" table claims Windows and Android are "production-ready" — reading the actual platform source (see the `android/` and `windows/` sections below) shows this is not accurate for Android (every Android type is an `unimplemented!()` stub) and only partially accurate for Windows (event detection and UI creation are real, but recipient extraction/messenger-window opening are still placeholders).

### `InterceptCoordinator`
- **type**: struct
- **file**: `crates/hsip-intercept/src/lib.rs`
- **purpose**: Main coordinator/owner of the whole intercept pipeline for one process: holds the platform `EventMonitor`, the `PatternMatcher`, the platform `InterceptOverlay`, the `HSIPRouter`, and the receiving half of the event channel. One instance drives the entire "detect → match → prompt → route" loop for the life of the process.
- **calls**: n/a (struct definition)
- **called_by**: whatever binary embeds this crate (no in-crate caller — this is the library's top-level entry point)
- **mutates**: n/a

### `InterceptCoordinator::new`
- **type**: function (async)
- **file**: `crates/hsip-intercept/src/lib.rs`
- **purpose**: Constructs a fully-wired `InterceptCoordinator` for the current platform. Creates the event channel, picks the platform-specific `EventMonitor`/`InterceptOverlay` implementation via `#[cfg(target_os = "...")]` blocks, loads (or builds default) the pattern database, and creates the `HSIPRouter`. On a platform that is none of windows/android/linux/macos (e.g. FreeBSD, WASM), returns `InterceptError::UnsupportedPlatform` instead of compiling a stub — there's no generic no-op fallback.
- **inputs**: `config: InterceptConfig`
- **outputs**: `Result<Self>`
- **calls**: `{windows,android,linux,macos}::{...}EventMonitor::new`, `PatternMatcher::load_from_config`, `{windows,android,linux,macos}::{...}Overlay::new`, `HSIPRouter::new`
- **called_by**: crate consumer (binary embedding hsip-intercept)
- **mutates**: nothing (pure construction; the platform monitor/overlay constructors themselves may touch OS state, e.g. registering a window class)

### `InterceptCoordinator::run`
- **type**: function (async)
- **file**: `crates/hsip-intercept/src/lib.rs`
- **purpose**: The main event loop. Starts the platform event monitor, then loops forever draining `event_rx` and dispatching each `MessagingEvent` to `handle_event`, logging (not propagating) any per-event error so one bad event can't kill the whole loop.
- **inputs**: `mut self`
- **outputs**: `Result<()>`
- **calls**: `self.event_monitor.start()`, `self.handle_event`
- **called_by**: crate consumer, once coordinator construction succeeds
- **mutates**: `self.event_monitor`'s running state; consumes `self.event_rx`

### `InterceptCoordinator::handle_event`
- **type**: function (async, private)
- **file**: `crates/hsip-intercept/src/lib.rs`
- **purpose**: Per-event pipeline: optionally sleeps for a random jitter (`privacy::add_timing_jitter`) if `config.privacy.timing_obfuscation` is set, then runs the event through `PatternMatcher::match_event`, and if a pattern matched *and* its confidence clears `config.min_confidence`, shows the intercept overlay. A match below the confidence threshold is logged and dropped rather than shown.
- **inputs**: `event: MessagingEvent`
- **outputs**: `Result<()>`
- **calls**: `privacy::add_timing_jitter`, `self.pattern_matcher.match_event`, `self.show_intercept_overlay`
- **called_by**: `run`
- **mutates**: nothing directly (may trigger the overlay/router side effects transitively)

### `InterceptCoordinator::show_intercept_overlay`
- **type**: function (async, private)
- **file**: `crates/hsip-intercept/src/lib.rs`
- **purpose**: Extracts a recipient hint, shows the platform overlay, and acts on the returned `UserChoice`: `SendPrivately` opens the HSIP messenger via `self.router`; `Continue` is a no-op; `DisableForApp(platform)` persists that platform as disabled by mutating and saving `self.config`.
- **inputs**: `event: &MessagingEvent`, `_pattern: &TriggerPattern` (unused beyond logging context at the call site)
- **outputs**: `Result<()>`
- **calls**: `self.extract_recipient`, `self.overlay.show`, `self.router.open_messenger`, `self.config.disable_platform`, `self.config.save`
- **called_by**: `handle_event`
- **mutates**: `self.config` (on `DisableForApp`) and the on-disk config file (`InterceptConfig::save`)

### `InterceptCoordinator::extract_recipient`
- **type**: function (async, private)
- **file**: `crates/hsip-intercept/src/lib.rs`
- **purpose**: Best-effort recipient extraction. Checks `event.metadata["recipient"]` first (set directly by some event monitors), then falls back to a platform-specific extractor (`windows::extract_recipient_from_window`, `android::extract_recipient_from_view`, etc.). Linux and macOS don't actually have a case wired here despite both exposing an `extract_recipient_from_window` function in their own modules — on those two targets this always falls through to `None` unless the event's own metadata already carried a `"recipient"` key.
- **inputs**: `event: &MessagingEvent`
- **outputs**: `Option<String>`
- **calls**: `windows::extract_recipient_from_window`, `android::extract_recipient_from_view`
- **called_by**: `show_intercept_overlay`
- **mutates**: nothing

---

## `crates/hsip-intercept/src/config.rs`

`InterceptConfig` and its nested settings structs — the persisted, user-editable configuration for the whole intercept system. Serialized as JSON to a per-user config directory.

### `InterceptConfig`
- **type**: struct
- **file**: `crates/hsip-intercept/src/config.rs`
- **purpose**: Top-level configuration: global enable flag, confidence threshold, per-platform enable/disable sets, pattern DB path, and the three nested config groups (`privacy`, `overlay`, `messenger`). `Default` is deliberately opt-in (`enabled: false`) with a curated starter set of `enabled_platforms` (Instagram, Facebook, WhatsApp, Gmail).
- **calls**: n/a
- **called_by**: `InterceptCoordinator::new`, every platform `EventMonitor`/`Overlay` constructor
- **mutates**: n/a

### `PrivacyConfig`
- **type**: struct
- **file**: `crates/hsip-intercept/src/config.rs`
- **purpose**: Privacy-enhancing feature toggles: timing obfuscation (on by default, 50–500ms jitter range), message padding (off by default — adds overhead), metadata stripping (on by default), and a `cover_traffic` flag reserved for a not-yet-implemented future feature (see `privacy::start_cover_traffic`).
- **calls**: n/a
- **called_by**: `InterceptCoordinator::handle_event` (`timing_obfuscation` check)
- **mutates**: n/a

### `OverlayConfig`
- **type**: struct
- **file**: `crates/hsip-intercept/src/config.rs`
- **purpose**: Overlay presentation settings — screen `position`, auto-dismiss `timeout_seconds` (0 = never), whether to show the first-run tutorial, and light/dark/system `theme`. Consumed directly by each platform's overlay implementation (e.g. `WindowsOverlay::calculate_overlay_position`, `LinuxOverlay`/`MacOSOverlay`'s notification timeout).
- **calls**: n/a
- **called_by**: platform overlay implementations
- **mutates**: n/a

### `OverlayPosition`
- **type**: enum
- **file**: `crates/hsip-intercept/src/config.rs`
- **purpose**: Screen corner (or center) to anchor the overlay: `TopRight` (default), `TopLeft`, `BottomRight`, `BottomLeft`, `Center`. Only actually affects layout on Windows (`WindowsOverlay::calculate_overlay_position`) — the Linux/macOS overlays are OS desktop notifications, which don't take an on-screen position.
- **called_by**: `windows::overlay::WindowsOverlay::calculate_overlay_position`

### `OverlayTheme`
- **type**: enum
- **file**: `crates/hsip-intercept/src/config.rs`
- **purpose**: `Light`/`Dark`/`System` theme selector for the overlay. Declared and defaulted (`System`) but not read by any current platform overlay implementation — theming is not actually wired into rendering yet.

### `MessengerConfig`
- **type**: struct
- **file**: `crates/hsip-intercept/src/config.rs`
- **purpose**: Settings for the (mostly unimplemented) HSIP Messenger window: auto-open on intercept, default consent duration, offline message queueing and its max size. None of the messenger-window stubs (`windows::open_messenger_window`, `android::open_messenger_activity`) currently read these fields — they're forward-declared for the messenger feature this crate's `router.rs` is a stub for.

### `InterceptConfig::load`
- **type**: function
- **file**: `crates/hsip-intercept/src/config.rs`
- **purpose**: Reads and JSON-deserializes a config file from an explicit path. Wraps the read I/O error in `InterceptError::Config`; a deserialization failure surfaces as `InterceptError::Json` via `?`'s `From` conversion.
- **inputs**: `path: &PathBuf`
- **outputs**: `Result<Self>`
- **calls**: `std::fs::read_to_string`, `serde_json::from_str`
- **mutates**: nothing (read-only)

### `InterceptConfig::save`
- **type**: function
- **file**: `crates/hsip-intercept/src/config.rs`
- **purpose**: Serializes `self` as pretty JSON and writes it to the path returned by `get_config_path` (not the path `load` was originally read from — `save` always targets the canonical per-user config directory, so loading from a custom path and saving doesn't round-trip to the same file).
- **inputs**: `&self`
- **outputs**: `Result<()>`
- **calls**: `self.get_config_path`, `serde_json::to_string_pretty`, `std::fs::write`
- **called_by**: `InterceptCoordinator::show_intercept_overlay` (on `DisableForApp`)
- **mutates**: filesystem (`<config_dir>/hsip/intercept_config.json`)

### `InterceptConfig::get_config_path`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/config.rs`
- **purpose**: Computes `<platform config dir>/hsip/intercept_config.json`, creating the `hsip` directory if missing (errors from `create_dir_all` are silently discarded via `.ok()` — a failure to create the directory only surfaces later when `save`'s `fs::write` itself fails).
- **outputs**: `PathBuf`
- **calls**: `dirs::config_dir`, `std::fs::create_dir_all`
- **called_by**: `save`
- **mutates**: filesystem (creates the config directory as a side effect)

### `InterceptConfig::is_platform_enabled`
- **type**: function
- **file**: `crates/hsip-intercept/src/config.rs`
- **purpose**: A platform counts as enabled only if the system is globally `enabled`, the platform is in `enabled_platforms`, and it is *not* also in `disabled_platforms` — the disabled set always wins over the enabled set, so `enable_platform` has to explicitly remove from `disabled_platforms` to actually re-enable something a user had turned off.
- **inputs**: `platform: PlatformType`
- **outputs**: `bool`
- **called_by**: every platform event monitor's `poll_once`/`handle_window_change` (gates whether a detected messaging window is worth turning into a `MessagingEvent`)
- **mutates**: nothing

### `InterceptConfig::disable_platform`
- **type**: function
- **file**: `crates/hsip-intercept/src/config.rs`
- **purpose**: Adds a platform to `disabled_platforms` (doesn't touch `enabled_platforms`).
- **inputs**: `&mut self`, `platform: PlatformType`
- **called_by**: `InterceptCoordinator::show_intercept_overlay` (`DisableForApp` case)
- **mutates**: `self.disabled_platforms`

### `InterceptConfig::enable_platform`
- **type**: function
- **file**: `crates/hsip-intercept/src/config.rs`
- **purpose**: Removes a platform from `disabled_platforms` and adds it to `enabled_platforms` — the pairing needed because `is_platform_enabled` treats `disabled_platforms` membership as an override.
- **inputs**: `&mut self`, `platform: PlatformType`
- **mutates**: `self.disabled_platforms`, `self.enabled_platforms`

---

## `crates/hsip-intercept/src/error.rs`

### `Result`
- **type**: variable (type alias)
- **file**: `crates/hsip-intercept/src/error.rs`
- **purpose**: `std::result::Result<T, InterceptError>` — the crate-wide result type used by every public fallible function in `hsip-intercept`.

### `InterceptError`
- **type**: enum
- **file**: `crates/hsip-intercept/src/error.rs`
- **purpose**: Crate-wide error type, one variant per subsystem (`EventMonitor`, `PatternMatch`, `Overlay`, `Router`, `Config`, `Permission`, `UnsupportedPlatform`, plus `#[from]` conversions for `std::io::Error` and `serde_json::Error`, an opaque `HSIPCore(String)`, and a catch-all `#[error(transparent)] Other(#[from] anyhow::Error)`). Built with `thiserror` so each variant gets a `Display` message and the `#[from]` variants participate in `?`-based error propagation automatically.
- **called_by**: every fallible function across this crate

---

## `crates/hsip-intercept/src/event.rs`

Shared event vocabulary and the `EventMonitor` trait every platform implements.

### `PlatformType`
- **type**: enum
- **file**: `crates/hsip-intercept/src/event.rs`
- **purpose**: The set of messaging platforms/apps this crate recognizes (Instagram, Facebook, WhatsApp, Gmail, Outlook, Slack, Discord, Telegram, Signal, Messenger, Twitter, LinkedIn, Unknown). `Copy + Eq + Hash` so it can be used as a `HashSet`/`HashMap` key (see `InterceptConfig`'s platform sets and `PatternMatcher`'s cache).
- **calls**: n/a

### `PlatformType::from_process_name`
- **type**: function
- **file**: `crates/hsip-intercept/src/event.rs`
- **purpose**: Classifies a platform from a process/package name via a fixed, case-insensitive `contains` chain (checked in enum declaration order — e.g. "facebook" or "fb" maps to `Facebook`, "x.com" maps to `Twitter`). Falls back to `Unknown` if nothing matches. This is the one, shared platform-classification heuristic every platform's event monitor (`linux`, `macos`, `windows`) reuses instead of writing its own.
- **inputs**: `name: &str`
- **outputs**: `PlatformType`
- **called_by**: `linux::event_monitor::LinuxEventMonitor::poll_once`, `macos::event_monitor::MacOSEventMonitor::poll_once`, `windows::event_monitor::WindowsEventMonitor::handle_window_change`/`start`'s inline polling loop
- **mutates**: nothing

### `MessagingEvent`
- **type**: struct
- **file**: `crates/hsip-intercept/src/event.rs`
- **purpose**: The normalized, cross-platform representation of one detected UI event: platform, event type, UTC timestamp, process name, optional window title, a free-form `metadata` map (class name, resource ID, etc. — platform-specific keys), and a `confidence` score (0.0–1.0) that the pattern matcher and coordinator both use to decide whether to actually intercept.
- **calls**: n/a
- **called_by**: constructed by every platform event monitor, consumed by `PatternMatcher::match_event`, `InterceptCoordinator::handle_event`, `OverlayContent::from_event`

### `EventType`
- **type**: enum
- **file**: `crates/hsip-intercept/src/event.rs`
- **purpose**: The kind of OS-level UI event observed: `Click`, `Focus`, `WindowChange`, `ValueChange`, `Custom`. In practice every current platform implementation only ever emits `WindowChange` (all four poll active-window/frontmost-app state on a timer) — `Click`/`Focus`/`ValueChange` are declared for a finer-grained accessibility-event backend that isn't implemented yet.

### `EventMonitor`
- **type**: trait (async)
- **file**: `crates/hsip-intercept/src/event.rs`
- **purpose**: The abstraction every platform's concrete monitor implements (`start`/`stop`/`is_running`/`event_sender`). `InterceptCoordinator` holds one as `Box<dyn EventMonitor>`, so it never needs `#[cfg(target_os)]` logic of its own beyond the initial construction.
- **called_by**: `InterceptCoordinator::run` (`start`), implemented by `LinuxEventMonitor`, `MacOSEventMonitor`, `WindowsEventMonitor`, and the always-`unimplemented!()` `AndroidEventMonitor`

### `MessagingEvent::new`
- **type**: function
- **file**: `crates/hsip-intercept/src/event.rs`
- **purpose**: Constructs a `MessagingEvent` with a fresh UTC timestamp, empty metadata, no window title, and a default confidence of `0.5` — callers then chain the `with_*` builder methods to fill in the rest.
- **inputs**: `platform: PlatformType`, `event_type: EventType`, `process_name: String`
- **outputs**: `Self`

### `MessagingEvent::with_metadata` / `with_window_title` / `with_confidence`
- **type**: function (builder methods)
- **file**: `crates/hsip-intercept/src/event.rs`
- **purpose**: Consuming builder methods that insert a metadata key/value, set the window title, or set the confidence score (clamped to `[0.0, 1.0]` in `with_confidence` — the only one of the three that validates its input). Each returns `Self` for chaining.
- **inputs**: varies (`impl Into<String>` pairs, or `f64`)
- **outputs**: `Self`
- **called_by**: every platform event monitor's event-construction code

---

## `crates/hsip-intercept/src/overlay.rs`

The overlay abstraction and its shared, platform-independent content builder.

### `UserChoice`
- **type**: enum
- **file**: `crates/hsip-intercept/src/overlay.rs`
- **purpose**: What the user decided when shown the intercept prompt: `SendPrivately` (route via HSIP), `Continue` (proceed with the original platform, the default on dismiss/timeout everywhere this is implemented), or `DisableForApp(PlatformType)` (stop intercepting this platform going forward).
- **called_by**: `InterceptCoordinator::show_intercept_overlay`, every platform overlay's `show` implementation

### `InterceptOverlay`
- **type**: trait (async)
- **file**: `crates/hsip-intercept/src/overlay.rs`
- **purpose**: The abstraction every platform's overlay UI implements — `show` (present the prompt and block for a choice), `hide`, `is_visible`. `InterceptCoordinator` holds one as `Box<dyn InterceptOverlay>`.
- **called_by**: `InterceptCoordinator::show_intercept_overlay`; implemented by `LinuxOverlay`, `MacOSOverlay`, `WindowsOverlay`, and the always-`unimplemented!()` `AndroidOverlay`

### `OverlayContent`
- **type**: struct
- **file**: `crates/hsip-intercept/src/overlay.rs`
- **purpose**: Platform-independent title/message/recipient/tutorial-flag content for the overlay, built once from a `MessagingEvent` and reused verbatim by every platform's rendering code (each platform only supplies its own presentation mechanism — layered Win32 window, `notify-rust` desktop notification, etc.).
- **called_by**: every platform overlay's `show` method

### `OverlayContent::from_event`
- **type**: function
- **file**: `crates/hsip-intercept/src/overlay.rs`
- **purpose**: Builds the prompt text for a real intercept — includes the recipient's name and platform (`{:?}` debug-formatted) if a recipient was resolved, otherwise a generic "send through HSIP instead?" message. Always sets `show_tutorial: false` (the first-run tutorial is a separate, explicitly-requested `OverlayContent::tutorial()`).
- **inputs**: `event: &MessagingEvent`, `recipient: Option<&str>`
- **outputs**: `Self`
- **called_by**: `WindowsOverlay::show`, `LinuxOverlay::show`, `MacOSOverlay::show`

### `OverlayContent::tutorial`
- **type**: function
- **file**: `crates/hsip-intercept/src/overlay.rs`
- **purpose**: Builds the fixed first-time-user explanation of what the intercept system does. `show_tutorial: true` — but no current platform overlay implementation actually branches on this flag or calls this constructor (`OverlayConfig::show_tutorial` is read from config but the tutorial content itself is unused code as of this file).
- **outputs**: `Self`

---

## `crates/hsip-intercept/src/patterns.rs`

The rule-based (not ML) recognizer that decides whether a `MessagingEvent` looks like a real "about to send a message" action, and with how much confidence.

### `PatternDatabase`
- **type**: struct
- **file**: `crates/hsip-intercept/src/patterns.rs`
- **purpose**: Versioned, serializable collection of `PlatformPattern`s — either loaded from a JSON file at `InterceptConfig.pattern_db_path` or generated in-memory via `PatternMatcher::default_database`.

### `PlatformPattern`
- **type**: struct
- **file**: `crates/hsip-intercept/src/patterns.rs`
- **purpose**: All `TriggerPattern`s associated with one `PlatformType`.

### `TriggerPattern`
- **type**: struct
- **file**: `crates/hsip-intercept/src/patterns.rs`
- **purpose**: A single matchable rule: which platform it belongs to, what kind of UI signal to look for (`TriggerType`), the literal substring `value` to match, and the `confidence` score awarded on a match.
- **called_by**: `InterceptCoordinator::handle_event` (receives the matched pattern for logging/threshold check), `PatternMatcher::match_event`/`match_pattern`

### `TriggerType`
- **type**: enum
- **file**: `crates/hsip-intercept/src/patterns.rs`
- **purpose**: What kind of event metadata a `TriggerPattern` matches against: `AccessibilityId`, `ClassName`, `WindowTitle`, `TextContent`, `ProcessName`, `AutomationId`. Serialized `snake_case` for the on-disk JSON pattern DB format.

### `PatternMatcher`
- **type**: struct
- **file**: `crates/hsip-intercept/src/patterns.rs`
- **purpose**: The matching engine: holds the loaded `PatternDatabase` plus a `HashMap` cache keyed by a composite string of `(process_name, window_title, resource_id)` so repeated identical events (the common case under 500ms polling, where the same window stays focused across ticks) skip re-running every pattern.
- **called_by**: `InterceptCoordinator` (constructed once, held as `pattern_matcher`)

### `PatternMatcher::load_from_config`
- **type**: function
- **file**: `crates/hsip-intercept/src/patterns.rs`
- **purpose**: Loads the pattern DB from `config.pattern_db_path` if that file exists on disk, otherwise falls back to `default_database()` — so a fresh install with no pattern file works out of the box without requiring the caller to ship one.
- **inputs**: `config: &InterceptConfig`
- **outputs**: `Result<Self>`
- **calls**: `Self::load_database`, `Self::default_database`
- **called_by**: `InterceptCoordinator::new`

### `PatternMatcher::load_database`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/patterns.rs`
- **purpose**: Reads and JSON-deserializes a `PatternDatabase` from a file path, wrapping the read error in `InterceptError::PatternMatch`.
- **inputs**: `path: &std::path::Path`
- **outputs**: `Result<PatternDatabase>`
- **called_by**: `load_from_config`

### `PatternMatcher::default_database`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/patterns.rs`
- **purpose**: Hardcoded starter pattern set covering Instagram (DM inbox button/thread view/"Send Message" text), Facebook (Messenger button/window title), Gmail (Compose window title/button), and WhatsApp (chat input field) — each with its own confidence weight (0.70–0.95).
- **outputs**: `PatternDatabase`
- **called_by**: `load_from_config` (fallback), directly in tests

### `PatternMatcher::match_event`
- **type**: function
- **file**: `crates/hsip-intercept/src/patterns.rs`
- **purpose**: Finds the best (highest-confidence) matching `TriggerPattern` for an event's platform, checking the cache first. Filters the database to patterns whose `platform` equals the event's platform, evaluates each with `match_pattern`, and keeps whichever produces the single highest confidence score (a strict `>` comparison, so on a tie the first-encountered pattern wins). Caches the result (including a `None` miss) keyed by the composite cache key.
- **inputs**: `&mut self`, `event: &MessagingEvent`
- **outputs**: `Result<Option<TriggerPattern>>`
- **calls**: `self.match_pattern`
- **called_by**: `InterceptCoordinator::handle_event`
- **mutates**: `self.cache`

### `PatternMatcher::match_pattern`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/patterns.rs`
- **purpose**: Evaluates one `TriggerPattern` against one event by substring-matching the pattern's `value` against whichever event field its `TriggerType` designates (falling back between related metadata keys for some types, e.g. `AccessibilityId` checks `accessibility_id` then `resource_id`; `TextContent` checks `text_content` then `content_description`). Returns the pattern's confidence on a hit, `None` on a miss.
- **inputs**: `&self`, `pattern: &TriggerPattern`, `event: &MessagingEvent`
- **outputs**: `Option<f64>`
- **called_by**: `match_event`
- **mutates**: nothing

### `PatternMatcher::save_database`
- **type**: function
- **file**: `crates/hsip-intercept/src/patterns.rs`
- **purpose**: Serializes the current in-memory database as pretty JSON and writes it to a given path. Not currently called anywhere in this crate — exists as an API for a caller wanting to persist a modified/learned pattern set, but nothing in this crate mutates `self.database` after construction.
- **inputs**: `&self`, `path: &std::path::Path`
- **outputs**: `Result<()>`
- **calls**: `serde_json::to_string_pretty`, `std::fs::write`
- **mutates**: filesystem (writes to `path`)

---

## `crates/hsip-intercept/src/privacy.rs`

Standalone, mostly-pure privacy-enhancing utility functions. Not orchestrated as a struct — each function is called independently by whatever code opts into it (currently just `InterceptCoordinator::handle_event`'s timing-jitter call; the rest are unused-but-available or explicit not-yet-implemented placeholders).

### `add_timing_jitter`
- **type**: function (async)
- **file**: `crates/hsip-intercept/src/privacy.rs`
- **purpose**: Sleeps for a random delay in the default 50–500ms range, to mask exactly-when-the-user-acted timing patterns from anything observing IPC/network timing.
- **calls**: `add_timing_jitter_range`
- **called_by**: `InterceptCoordinator::handle_event` (only if `config.privacy.timing_obfuscation`)
- **mutates**: nothing (delays the calling task only)

### `add_timing_jitter_range`
- **type**: function (async)
- **file**: `crates/hsip-intercept/src/privacy.rs`
- **purpose**: Sleeps for a uniformly random duration between `min_ms` and `max_ms` inclusive.
- **inputs**: `min_ms: u64`, `max_ms: u64`
- **calls**: `rand::thread_rng().gen_range`, `tokio::time::sleep`
- **called_by**: `add_timing_jitter`

### `normalize_timestamp`
- **type**: function
- **file**: `crates/hsip-intercept/src/privacy.rs`
- **purpose**: Rounds a UTC timestamp down to the nearest 5-minute boundary (minute, second, and nanosecond all zeroed/floored) to prevent precise send-time correlation. Not currently called by any other function in this crate — a standalone utility available to a caller, same as `pad_message`/`strip_image_metadata` below.
- **inputs**: `ts: chrono::DateTime<chrono::Utc>`
- **outputs**: `chrono::DateTime<chrono::Utc>`
- **mutates**: nothing

### `pad_message`
- **type**: function
- **file**: `crates/hsip-intercept/src/privacy.rs`
- **purpose**: Pads a byte buffer up to the next fixed bucket size (256/512/1024/2048/4096/8192 bytes; beyond 8192 it adds a flat 256-byte pad instead of jumping to a next bucket) to hide the true message length, using random bytes for the padding rather than zeros (harder to distinguish padding from content by pattern). Not wired into any message-sending path in this crate yet — `PrivacyConfig.message_padding` exists as a config flag but nothing reads it to call this function.
- **inputs**: `message: &[u8]`
- **outputs**: `Vec<u8>`
- **calls**: `rand::random::<u8>()`
- **mutates**: nothing (returns a new buffer)

### `strip_image_metadata`
- **type**: function
- **file**: `crates/hsip-intercept/src/privacy.rs`
- **purpose**: Placeholder for EXIF-stripping (GPS, camera model, timestamps, software tag) — currently just logs a warning and returns the input unchanged. Explicitly marked `TODO` in a comment; needs an image-processing dependency (e.g. `image-rs`/`kamadak-exif`) not yet added.
- **inputs**: `image_data: &[u8]`
- **outputs**: `Result<Vec<u8>, String>`
- **mutates**: nothing (no-op today)

### `start_cover_traffic`
- **type**: function (async)
- **file**: `crates/hsip-intercept/src/privacy.rs`
- **purpose**: Placeholder for a Phase-3 cover-traffic feature (dummy packets at regular intervals to mask real message timing) — currently only logs a warning and returns immediately, doing nothing.
- **inputs**: `_intensity: CoverTrafficIntensity`
- **mutates**: nothing (no-op today)

### `CoverTrafficIntensity`
- **type**: enum
- **file**: `crates/hsip-intercept/src/privacy.rs`
- **purpose**: `Low`/`Medium`/`High` intensity levels for the not-yet-implemented cover-traffic feature (roughly 1 packet/minute, /10s, /second respectively, per the doc comments — not enforced anywhere yet since `start_cover_traffic` is a no-op).

---

## `crates/hsip-intercept/src/router.rs`

The (still largely stubbed) HSIP-side routing logic that would take over once a user chooses "send privately." Most of this file's methods are explicit `TODO`s with `warn!` logs rather than working implementations — this is the least-finished module in the crate outside of Android.

### `PeerID`
- **type**: variable (type alias)
- **file**: `crates/hsip-intercept/src/router.rs`
- **purpose**: `pub type PeerID = hsip_core::hello::PeerId` — re-exports `hsip-core`'s peer identity type under this crate's own name so callers of `HSIPRouter` don't need a direct `hsip_core` import.

### `HSIPRouter`
- **type**: struct
- **file**: `crates/hsip-intercept/src/router.rs`
- **purpose**: Intended to route intercepted messages through HSIP's consent handshake and encrypted session, but is currently a stub — it holds a cloned `InterceptConfig` (marked `#[allow(dead_code)]` with a comment noting it's kept for the future `hsip-core` integration this router is a placeholder for) and every method beyond construction either does nothing useful yet or explicitly logs "not yet implemented."

### `HSIPRouter::new`
- **type**: function (async)
- **file**: `crates/hsip-intercept/src/router.rs`
- **purpose**: Trivial constructor — clones the passed config into the struct. Never fails in practice (always returns `Ok`) despite the `Result` return type.
- **inputs**: `config: &InterceptConfig`
- **outputs**: `Result<Self>`
- **called_by**: `InterceptCoordinator::new`

### `HSIPRouter::open_messenger`
- **type**: function (async)
- **file**: `crates/hsip-intercept/src/router.rs`
- **purpose**: Entry point for "user chose SendPrivately." If a recipient string is present, tries `resolve_recipient` first; on a resolved `PeerID` calls `start_session_with_peer` (itself a stub), otherwise falls back to `open_messenger_manual` with the raw recipient hint. With no recipient at all, opens the manual/blank messenger directly.
- **inputs**: `recipient: Option<String>`
- **outputs**: `Result<()>`
- **calls**: `self.resolve_recipient`, `self.start_session_with_peer`, `self.open_messenger_manual`
- **called_by**: `InterceptCoordinator::show_intercept_overlay`

### `HSIPRouter::resolve_recipient`
- **type**: function (async, private)
- **file**: `crates/hsip-intercept/src/router.rs`
- **purpose**: Attempts to turn a free-text recipient string into a `PeerID`. Currently only two paths exist and neither is functional: a `"peer_"`-prefixed string logs a warning and always returns `None` (PeerID parsing itself is a `TODO`), and `lookup_contact` (local contact-book lookup) is also a stub that always returns `None`. DHT lookup and deep-link resolution are noted as future work, not present at all.
- **inputs**: `&self`, `recipient: &str`
- **outputs**: `Result<Option<PeerID>>`
- **calls**: `self.lookup_contact`
- **called_by**: `open_messenger`

### `HSIPRouter::lookup_contact`
- **type**: function (async, private)
- **file**: `crates/hsip-intercept/src/router.rs`
- **purpose**: Stub for a local contact-book lookup — always returns `Ok(None)`, forcing manual recipient entry every time.
- **inputs**: `&self`, `_name: &str`
- **outputs**: `Result<Option<PeerID>>`
- **called_by**: `resolve_recipient`

### `HSIPRouter::start_session_with_peer`
- **type**: function (async, private)
- **file**: `crates/hsip-intercept/src/router.rs`
- **purpose**: Stub for establishing an actual HSIP consent-and-session flow with a resolved peer — logs a warning that this isn't implemented and returns `Ok(())` immediately.
- **inputs**: `&self`, `peer_id: PeerID`
- **outputs**: `Result<()>`
- **called_by**: `open_messenger`

### `HSIPRouter::open_messenger_manual`
- **type**: function (async, private)
- **file**: `crates/hsip-intercept/src/router.rs`
- **purpose**: Dispatches to the platform's own messenger-window opener (`windows::open_messenger_window`/`android::open_messenger_activity`) when built for those targets; on Linux/macOS there's no `#[cfg]` arm at all, so this is a silent no-op there beyond the log line.
- **inputs**: `&self`, `hint: Option<String>`
- **outputs**: `Result<()>`
- **calls**: `windows::open_messenger_window`, `android::open_messenger_activity`
- **called_by**: `open_messenger`

### `HSIPRouter::send_message`
- **type**: function (async)
- **file**: `crates/hsip-intercept/src/router.rs`
- **purpose**: Public API for sending a message through an established HSIP session — entirely unimplemented (logs a warning, returns `Ok(())`, ignores the message content parameter). Not called from anywhere else in this crate yet.
- **inputs**: `peer_id: PeerID`, `_message: String`
- **outputs**: `Result<()>`

---

## `crates/hsip-intercept/src/android/mod.rs`, `android/event_monitor.rs`, `android/messenger.rs`, `android/overlay.rs`

Android is meant to use `AccessibilityService` for event monitoring, `WindowManager` (`TYPE_APPLICATION_OVERLAY`) for the overlay, and a JNI bridge to talk to this Rust core — but as of this code, **none of that is actually implemented**. `android/mod.rs` conditionally re-exports `event_monitor`/`overlay`/`messenger` from real files only `#[cfg(target_os = "android")]`; on every other target it defines its own inline stub modules with the same names so the crate still compiles for non-Android targets. Critically, the *real*, Android-only files (`android/event_monitor.rs`, `android/overlay.rs`, `android/messenger.rs`) are themselves nothing but `unimplemented!()` bodies — so even a genuine Android build of this crate panics the instant any of these are called. Despite `lib.rs`'s module doc calling Android "production-ready," there is no working Android implementation in this codebase at all.

### `AndroidEventMonitor` (real, `android/event_monitor.rs`)
- **type**: struct
- **file**: `crates/hsip-intercept/src/android/event_monitor.rs`
- **purpose**: Unit struct standing in for a real Android event monitor. `AndroidEventMonitor::new` panics via `unimplemented!("Android event monitor — JNI bridge not yet implemented")` rather than returning an error — a caller (`InterceptCoordinator::new`) that reaches this on a real Android build crashes the process instead of getting a `Result::Err` it could handle.
- **inputs**: `_tx: mpsc::Sender<MessagingEvent>`, `_config: &InterceptConfig`
- **outputs**: `Result<Box<dyn EventMonitor>>` (never actually returns — always panics)
- **called_by**: `InterceptCoordinator::new` (`#[cfg(target_os = "android")]` branch)
- **mutates**: nothing (panics before doing anything)

### `AndroidOverlay` (real, `android/overlay.rs`)
- **type**: struct
- **file**: `crates/hsip-intercept/src/android/overlay.rs`
- **purpose**: Same pattern as `AndroidEventMonitor` — `AndroidOverlay::new` is `unimplemented!("Android overlay — WindowManager bridge not yet implemented")`.
- **inputs**: `_config: &InterceptConfig`
- **outputs**: `Result<Box<dyn InterceptOverlay>>` (never returns)
- **called_by**: `InterceptCoordinator::new` (`#[cfg(target_os = "android")]` branch)

### `open_messenger_activity` (real, `android/messenger.rs`)
- **type**: function (async)
- **file**: `crates/hsip-intercept/src/android/messenger.rs`
- **purpose**: `unimplemented!("Android messenger activity — JNI bridge not yet implemented")`.
- **inputs**: `_hint: Option<String>`
- **outputs**: `Result<()>` (never returns)
- **called_by**: `HSIPRouter::open_messenger_manual` (`#[cfg(target_os = "android")]` branch)

### `extract_recipient_from_view` (real, `android/messenger.rs`)
- **type**: function
- **file**: `crates/hsip-intercept/src/android/messenger.rs`
- **purpose**: `unimplemented!("Android recipient extraction — JNI bridge not yet implemented")`.
- **inputs**: `_event: &MessagingEvent`
- **outputs**: `Result<String>` (never returns)
- **called_by**: `InterceptCoordinator::extract_recipient` (`#[cfg(target_os = "android")]` branch)

### Non-Android stub modules (`android/mod.rs`, `#[cfg(not(target_os = "android"))]`)
- **type**: function (multiple, inline modules `event_monitor`/`overlay`/`messenger`)
- **file**: `crates/hsip-intercept/src/android/mod.rs`
- **purpose**: Second copy of the same type/function names (`AndroidEventMonitor::new`, `AndroidOverlay::new`, `open_messenger_activity`, `extract_recipient_from_view`), all likewise `unimplemented!()`, that exist purely so `hsip-intercept` compiles as a library on non-Android hosts (e.g. so `cargo build --workspace` on Linux/macOS/Windows dev machines doesn't fail on a missing `android` submodule). These are never reachable in practice since every call site is itself `#[cfg(target_os = "android")]`-gated — they exist for compilation completeness only, not for any runtime path.
- **mutates**: nothing (unreachable in practice)

---

## `crates/hsip-intercept/src/linux/mod.rs`, `linux/event_monitor.rs`, `linux/overlay.rs`

Linux is the one platform in this crate with a genuinely complete, non-stub implementation for both halves of the pipeline. Event detection uses subprocess polling — `xdotool` on X11 (`getactivewindow getwindowname`/`getwindowpid`) or, on Wayland (where there's no portable way to query the focused window without a compositor-specific protocol), a fallback scan of `/proc/<pid>/comm` for known messaging-app process names. The overlay is a real desktop notification via the `notify-rust` crate (libnotify/D-Bus), with three action buttons (send via HSIP / continue / disable for this app).

### `extract_recipient_from_window` (`linux/mod.rs`)
- **type**: function
- **file**: `crates/hsip-intercept/src/linux/mod.rs`
- **purpose**: Best-effort recipient parsing from a Linux window title. Tries a fixed set of known prefixes (`"Chat with "`, `"Direct: "`, `"DM: "`) first, splitting the remainder on space/em-dash/en-dash/hyphen to isolate just the name; if no prefix matches, falls back to splitting the whole title on an em-dash/en-dash (e.g. `"Alice – Telegram Desktop"` → `"Alice"`), accepting the result only if it's non-empty and under 64 characters. Returns an `EventMonitor` error if nothing usable was found.
- **inputs**: `event: &crate::event::MessagingEvent`
- **outputs**: `crate::Result<String>`
- **called_by**: *not currently called* — `InterceptCoordinator::extract_recipient` only has `#[cfg]` arms for windows/android, so this function is dead code on the actual Linux runtime path today despite being fully implemented and exported.
- **mutates**: nothing

### `LinuxEventMonitor`
- **type**: struct
- **file**: `crates/hsip-intercept/src/linux/event_monitor.rs`
- **purpose**: Polls the focused window every 500ms on a dedicated blocking thread (`tokio::task::spawn_blocking`), classifies the platform, computes a confidence score from window-title keywords, and emits a `MessagingEvent` on any observed change. Holds an `Arc<AtomicBool>` `running` flag shared between the async handle and the polling thread so `stop()` can signal the loop to exit without needing a channel.
- **called_by**: `InterceptCoordinator::new` (`#[cfg(target_os = "linux")]` branch)

### `LinuxEventMonitor::new`
- **type**: function
- **file**: `crates/hsip-intercept/src/linux/event_monitor.rs`
- **purpose**: Constructs the monitor in a non-running state; doesn't touch the OS at all until `start()` is called.
- **inputs**: `event_tx: mpsc::Sender<MessagingEvent>`, `config: &InterceptConfig`
- **outputs**: `Result<Box<dyn EventMonitor>>`

### `LinuxEventMonitor::is_wayland`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/linux/event_monitor.rs`
- **purpose**: Detects Wayland by checking `WAYLAND_DISPLAY` (any value) or `XDG_SESSION_TYPE == "wayland"` (case-insensitively). This decides which of the two detection strategies `poll_once` uses.
- **outputs**: `bool`
- **called_by**: `start`, `poll_once`

### `LinuxEventMonitor::x11_active_window_title`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/linux/event_monitor.rs`
- **purpose**: Shells out to `xdotool getactivewindow getwindowname` and returns the trimmed stdout, or `None` if the subprocess fails, exits non-zero, or the title is empty.
- **outputs**: `Option<String>`
- **calls**: `std::process::Command::new("xdotool")`
- **called_by**: `poll_once`

### `LinuxEventMonitor::x11_active_window_process`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/linux/event_monitor.rs`
- **purpose**: Shells out to `xdotool getactivewindow getwindowpid` to get the focused window's PID, then reads `/proc/<pid>/comm` for its process name.
- **outputs**: `Option<String>`
- **calls**: `std::process::Command::new("xdotool")`, `Self::read_proc_comm`
- **called_by**: `poll_once`

### `LinuxEventMonitor::read_proc_comm`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/linux/event_monitor.rs`
- **purpose**: Reads `/proc/<pid>/comm` (the kernel-truncated-to-15-char process name) and trims it.
- **inputs**: `pid: u32`
- **outputs**: `Option<String>`
- **called_by**: `x11_active_window_process`, `scan_proc_for_messaging_apps`, unit test `test_proc_comm_read`

### `LinuxEventMonitor::scan_proc_for_messaging_apps`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/linux/event_monitor.rs`
- **purpose**: Wayland fallback: walks every entry of `/proc`, keeps only numeric (PID) directory names, reads each one's `comm`, and case-insensitively substring-matches it against a fixed list of known messaging-app process names (Telegram, Signal, Slack, Discord, WhatsApp variants, Thunderbird, Evolution, Geary, Fractal, Element, Nheko, Ferdi/Ferdium, Rambox). Notably includes both `"telegram-deskto"` (the kernel's 15-char-truncated `comm` value) and the untruncated `"Telegram Desktop"` as separate list entries to cover both possible reported forms.
- **outputs**: `Vec<(u32, String)>` (pid, process name pairs)
- **calls**: `std::fs::read_dir("/proc")`, `Self::read_proc_comm`
- **called_by**: `poll_once` (Wayland branch)

### `LinuxEventMonitor::poll_once`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/linux/event_monitor.rs`
- **purpose**: One polling tick. On Wayland, uses `scan_proc_for_messaging_apps` and takes the first match's process name with no window title (title is always `None` in that branch — Wayland offers no portable window-title query here). On X11, queries both title and process via `xdotool`. Compares against the previous tick's values; if unchanged, does nothing. On change, classifies platform, checks `config.is_platform_enabled`, scores confidence from a fixed keyword list in the window title (0.85 if any keyword like "compose"/"chat"/"dm" is present, else 0.55 baseline), builds a `MessagingEvent`, and spawns a task to send it on the channel (so a slow/full channel doesn't block the polling loop itself).
- **inputs**: `&self`, `last_title: &mut Option<String>`, `last_process: &mut Option<String>`
- **calls**: `Self::is_wayland`, `Self::scan_proc_for_messaging_apps`, `Self::x11_active_window_title`, `Self::x11_active_window_process`, `PlatformType::from_process_name`, `self.config.is_platform_enabled`, `MessagingEvent::new`/`with_confidence`/`with_window_title`, `tokio::spawn`
- **called_by**: `start`'s polling loop
- **mutates**: `*last_title`, `*last_process` (via the caller's mutable references); sends on `self.event_tx`

### `LinuxEventMonitor::start` / `stop` / `is_running` / `event_sender` (trait impl)
- **type**: function (async, trait impl of `EventMonitor`)
- **file**: `crates/hsip-intercept/src/linux/event_monitor.rs`
- **purpose**: `start` is idempotent (no-op if already running), logs which display server was detected, warns (doesn't fail) if `xdotool` is missing on an X11 session, then spawns a `spawn_blocking` task running a `poll_once` + 500ms-sleep loop until `running` flips false. `stop` just flips the shared `AtomicBool` — the blocking thread notices on its next loop iteration and exits (there's no join/await on that thread from `stop` itself, so `stop()` returning doesn't guarantee the thread has actually finished yet).
- **outputs**: `Result<()>` / `bool` / `&mpsc::Sender<MessagingEvent>`
- **calls**: `Self::is_wayland`, `poll_once`, `tokio::task::spawn_blocking`
- **called_by**: `InterceptCoordinator::run` (`start`)
- **mutates**: `self.running` (AtomicBool)

### `LinuxOverlay`
- **type**: struct
- **file**: `crates/hsip-intercept/src/linux/overlay.rs`
- **purpose**: Wraps a desktop notification (via `notify_rust`) as the intercept prompt. Tracks `current_platform` in an `Arc<Mutex<PlatformType>>` purely so the blocking notification thread's `DisableForApp` action (which doesn't know the platform) can be substituted with the real one back on the async side after the blocking call returns.
- **called_by**: `InterceptCoordinator::new` (`#[cfg(target_os = "linux")]` branch)

### `ACTION_HSIP` / `ACTION_CONTINUE` / `ACTION_DISABLE`
- **type**: variable (constants, `&str`)
- **file**: `crates/hsip-intercept/src/linux/overlay.rs`
- **purpose**: The three D-Bus notification action IDs (`"hsip"`, `"continue"`, `"disable"`) registered on the notification and matched against the actual clicked action in `show_notification`.
- **called_by**: `LinuxOverlay::show_notification`

### `LinuxOverlay::new`
- **type**: function
- **file**: `crates/hsip-intercept/src/linux/overlay.rs`
- **purpose**: Constructs the overlay in a not-yet-shown state.
- **inputs**: `config: &InterceptConfig`
- **outputs**: `Result<Box<dyn InterceptOverlay>>`

### `LinuxOverlay::build_body`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/linux/overlay.rs`
- **purpose**: Trivial passthrough of `content.message` — kept as its own function (rather than inlined) so it's directly unit-testable without going through the async/blocking notification machinery.
- **inputs**: `content: &OverlayContent`
- **outputs**: `String`
- **called_by**: `show`

### `LinuxOverlay::show_notification`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/linux/overlay.rs`
- **purpose**: Builds and shows a `notify_rust::Notification` with the three actions above and a configurable timeout (`Timeout::Never` if `timeout_secs == 0`), then blocks (`handle.wait_for_action`) until the user clicks an action or the notification is dismissed — default on dismiss/timeout is `UserChoice::Continue`. This is a blocking D-Bus call, hence always invoked from inside `spawn_blocking` by its caller.
- **inputs**: `summary: &str`, `body: &str`, `timeout_secs: u32`
- **outputs**: `Result<UserChoice>`
- **calls**: `notify_rust::Notification::show`, `handle.wait_for_action`
- **called_by**: `show` (via `tokio::task::spawn_blocking`)

### `LinuxOverlay::show` / `hide` / `is_visible` (trait impl)
- **type**: function (async, trait impl of `InterceptOverlay`)
- **file**: `crates/hsip-intercept/src/linux/overlay.rs`
- **purpose**: `show` records the event's platform (for the `DisableForApp` substitution), builds the overlay content, then runs `show_notification` on a blocking thread and awaits it; if the result was `DisableForApp` (with a placeholder `Unknown` platform baked in from inside the blocking closure, since that closure has no access to the real platform), substitutes back the real captured platform. `hide` is a no-op (notifications self-dismiss or are dismissed by the user; there's no persistent handle retained to close). `is_visible` always returns `false` — visibility isn't tracked, since the notification daemon owns that state.
- **outputs**: `Result<UserChoice>` / `Result<()>` / `bool`
- **calls**: `OverlayContent::from_event`, `Self::build_body`, `Self::show_notification` (via `spawn_blocking`)
- **called_by**: `InterceptCoordinator::show_intercept_overlay`
- **mutates**: `self.current_platform`

---

## `crates/hsip-intercept/src/macos/mod.rs`, `macos/event_monitor.rs`, `macos/overlay.rs`

macOS, like Linux, has a real (not stubbed) implementation for both halves, but deliberately avoids Objective-C/Swift/CoreFoundation bindings for this MVP: event detection shells out to `osascript` (AppleScript via System Events) to poll the frontmost application's name and window title every 500ms, and the overlay uses `notify-rust`'s macOS backend (`mac-notification-sys`, User Notification Center) for a banner notification. The module doc notes real CoreFoundation bindings are planned for a later phase to avoid the subprocess overhead and get lower latency.

### `extract_recipient_from_window` (`macos/mod.rs`)
- **type**: function
- **file**: `crates/hsip-intercept/src/macos/mod.rs`
- **purpose**: Same best-effort window-title recipient parsing as the Linux equivalent, with macOS-specific prefixes (`"Chat with "`, `"DM with "`, `"Message to "`) tried first, falling back to splitting on em-dash/en-dash/hyphen and taking the first non-empty, under-64-character segment.
- **inputs**: `event: &crate::event::MessagingEvent`
- **outputs**: `crate::Result<String>`
- **called_by**: *not currently called* — same gap as Linux's equivalent function; `InterceptCoordinator::extract_recipient` has no `#[cfg(target_os = "macos")]` arm, so this is dead code on the actual runtime path despite being exported and implemented.
- **mutates**: nothing

### `MacOSEventMonitor`
- **type**: struct
- **file**: `crates/hsip-intercept/src/macos/event_monitor.rs`
- **purpose**: Polls the frontmost application/window every 500ms on a `spawn_blocking` thread, same `Arc<AtomicBool>` `running`-flag pattern as `LinuxEventMonitor`. Requires the process to already hold the macOS "Accessibility" (and/or "Screen Recording") privacy permission for the AppleScript queries to succeed — `start()` logs a loud warning about this requirement but does not itself verify or request the permission.
- **called_by**: `InterceptCoordinator::new` (`#[cfg(target_os = "macos")]` branch)

### `SCRIPT_APP_NAME` / `SCRIPT_WINDOW_TITLE`
- **type**: variable (constants, `&str`, raw AppleScript source)
- **file**: `crates/hsip-intercept/src/macos/event_monitor.rs`
- **purpose**: The two AppleScript snippets run via `osascript -e`: one returns the name of the frontmost process (via `System Events`), the other returns the title of that process's first window, or empty string if it has no windows.
- **called_by**: `sample_frontmost` (via `run_applescript`)

### `MacOSEventMonitor::new`
- **type**: function
- **file**: `crates/hsip-intercept/src/macos/event_monitor.rs`
- **purpose**: Constructs the monitor in a non-running state.
- **inputs**: `event_tx: mpsc::Sender<MessagingEvent>`, `config: &InterceptConfig`
- **outputs**: `Result<Box<dyn EventMonitor>>`

### `MacOSEventMonitor::run_applescript`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/macos/event_monitor.rs`
- **purpose**: Runs `osascript -e <script>` as a subprocess, returning trimmed stdout on success (and non-empty output), or `None` on subprocess failure or a non-zero exit (logging stderr at debug level in that case).
- **inputs**: `script: &str`
- **outputs**: `Option<String>`
- **calls**: `std::process::Command::new("osascript")`
- **called_by**: `sample_frontmost`

### `MacOSEventMonitor::sample_frontmost`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/macos/event_monitor.rs`
- **purpose**: Runs both AppleScript queries and returns the pair of results.
- **outputs**: `(Option<String>, Option<String>)` (app name, window title)
- **calls**: `Self::run_applescript` (×2)
- **called_by**: `poll_once`

### `MacOSEventMonitor::is_messaging_app`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/macos/event_monitor.rs`
- **purpose**: Case-insensitive substring match of an app name against a fixed list of known macOS messaging/mail/meeting apps (Messages, Telegram, Signal, Slack, Discord, WhatsApp, Messenger, Mimestream, Spark, Mail, Airmail, Outlook, Zoom, Teams, Skype).
- **inputs**: `app_name: &str`
- **outputs**: `bool`
- **called_by**: `poll_once`, unit test `test_messaging_app_detection`

### `MacOSEventMonitor::has_messaging_title`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/macos/event_monitor.rs`
- **purpose**: Scores a window title against two keyword tiers — "strong" keywords ("compose", "new message", "direct message", "dm", "send message") yield confidence 0.90; "weak" keywords ("message", "chat", "conversation", "inbox") yield 0.70; no match yields `(false, 0.50)`.
- **inputs**: `title: &str`
- **outputs**: `(bool, f64)` (whether any keyword hit, and the associated confidence)
- **called_by**: `poll_once`, unit test `test_messaging_title_detection`

### `MacOSEventMonitor::poll_once`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/macos/event_monitor.rs`
- **purpose**: One polling tick: samples frontmost app+title, skips if unchanged or if there's no app name at all, skips if the app isn't a recognized messaging app (`is_messaging_app`), classifies platform and checks `config.is_platform_enabled`. Confidence is 0.60 baseline for an app-name match alone, replaced by `has_messaging_title`'s score if the title itself also hits a keyword. Builds and asynchronously sends a `MessagingEvent` (via `tokio::spawn`, same non-blocking-send pattern as Linux).
- **inputs**: `&self`, `last_app: &mut Option<String>`, `last_title: &mut Option<String>`
- **calls**: `Self::sample_frontmost`, `Self::is_messaging_app`, `PlatformType::from_process_name`, `self.config.is_platform_enabled`, `Self::has_messaging_title`, `MessagingEvent::new`, `tokio::spawn`
- **called_by**: `start`'s polling loop
- **mutates**: `*last_app`, `*last_title`; sends on `self.event_tx`

### `MacOSEventMonitor::start` / `stop` / `is_running` / `event_sender` (trait impl)
- **type**: function (async, trait impl of `EventMonitor`)
- **file**: `crates/hsip-intercept/src/macos/event_monitor.rs`
- **purpose**: `start` first sanity-checks that `osascript` runs at all (`osascript -e 1`), returning an error immediately if not — the one platform monitor in this crate that actually validates its subprocess dependency up front rather than only discovering its absence on first real query. Then warns about the Accessibility permission requirement and spawns the same `spawn_blocking` poll loop pattern as Linux. `stop` flips the shared `AtomicBool`.
- **outputs**: `Result<()>` / `bool` / `&mpsc::Sender<MessagingEvent>`
- **calls**: `std::process::Command::new("osascript")`, `poll_once`, `tokio::task::spawn_blocking`
- **called_by**: `InterceptCoordinator::run`
- **mutates**: `self.running`

### `MacOSOverlay`
- **type**: struct
- **file**: `crates/hsip-intercept/src/macos/overlay.rs`
- **purpose**: Notification-based overlay using `notify-rust`'s macOS backend. Tracks `current_platform` (same `DisableForApp`-substitution reasoning as `LinuxOverlay`) and additionally a `visible: Arc<Mutex<bool>>` flag that Linux's overlay doesn't bother tracking. Doc comment notes macOS 10.14+ restricts custom notification action buttons to App Store apps unless using `UNUserNotificationCenter` directly, so for this MVP a click on the notification body is simply treated as "Send via HSIP" rather than presenting real per-action buttons.
- **called_by**: `InterceptCoordinator::new` (`#[cfg(target_os = "macos")]` branch)

### `MacOSOverlay::new`
- **type**: function
- **file**: `crates/hsip-intercept/src/macos/overlay.rs`
- **purpose**: Constructs the overlay, initially not visible.
- **inputs**: `config: &InterceptConfig`
- **outputs**: `Result<Box<dyn InterceptOverlay>>`

### `MacOSOverlay::show_notification`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/macos/overlay.rs`
- **purpose**: Shows a `notify_rust::Notification` (with `"default"`/`"close"` actions, though as noted only the click/dismiss distinction really matters on macOS) and blocks for the user's response via `wait_for_action`; maps `"default"` → `SendPrivately`, anything else (including `"__closed"`) → `Continue`.
- **inputs**: `summary: &str`, `body: &str`, `timeout_secs: u32`
- **outputs**: `Result<UserChoice>`
- **calls**: `notify_rust::Notification::show`, `handle.wait_for_action`
- **called_by**: `show` (via `spawn_blocking`)

### `MacOSOverlay::show` / `hide` / `is_visible` (trait impl)
- **type**: function (async, trait impl of `InterceptOverlay`)
- **file**: `crates/hsip-intercept/src/macos/overlay.rs`
- **purpose**: `show` records platform and sets `visible = true` before dispatching to the blocking `show_notification` call, resetting `visible = false` once it returns (inside the same `spawn_blocking` closure, so visibility flips back the instant the blocking call itself completes, not when the async caller resumes); substitutes the real platform into any `DisableForApp` result the same way Linux does. `hide` just sets `visible = false` directly (macOS notifications are OS-managed; there's no window handle to destroy). `is_visible` reads the tracked flag — unlike Linux, which hardcodes `false`.
- **outputs**: `Result<UserChoice>` / `Result<()>` / `bool`
- **calls**: `OverlayContent::from_event`, `Self::show_notification` (via `spawn_blocking`)
- **called_by**: `InterceptCoordinator::show_intercept_overlay`
- **mutates**: `self.current_platform`, `self.visible`

---

## `crates/hsip-intercept/src/windows/mod.rs`, `windows/event_monitor.rs`, `windows/messenger.rs`, `windows/overlay.rs`, `windows/utils.rs`

Windows uses the real Win32/COM UI Automation API (`IUIAutomation`) plus `SetWinEventHook` for event detection, and a hand-built layered (`WS_EX_LAYERED`) always-on-top popup window, drawn with raw GDI calls, for the overlay — the only platform in this crate that renders its own custom window rather than delegating to an OS notification center. Event detection is real (COM `IUIAutomation` is initialized and a genuine 500ms polling loop via `GetForegroundWindow` detects window changes and sends events), but two important pieces are still placeholders: the `SetWinEventHook`-based event-hook path is registered but its callback (`win_event_proc`) never actually forwards anything to the event channel (just logs), so all real event delivery in practice comes from the separate polling loop inside `start()`, not the hooks; and recipient/messenger-window handling (`messenger.rs`) is largely unimplemented, falling back to a debug-only `MessageBoxW` placeholder.

### `IUIAutomation` / `IUIAutomationElement` / `UIA_PATTERN_ID` (`windows/mod.rs`)
- **type**: variable (re-exports)
- **file**: `crates/hsip-intercept/src/windows/mod.rs`
- **purpose**: Re-exports these `windows` crate Win32 UI Automation types under `crate::windows::` so downstream code doesn't need its own direct `windows::Win32::UI::Accessibility` import.

### `SendSyncWrapper<T>`
- **type**: struct
- **file**: `crates/hsip-intercept/src/windows/event_monitor.rs`
- **purpose**: A thin wrapper that unsafely asserts `Send + Sync` for a COM object (`IUIAutomation`) so it can live inside `WindowsEventMonitor`, which itself must be `Send`/`Sync` to satisfy the `EventMonitor: Send + Sync` trait bound. The safety comment asserts this is sound for apartment-threaded COM objects "when properly initialized per-thread" — a claim not otherwise enforced by the type system here.
- **called_by**: `WindowsEventMonitor` (as the type of its `automation` field)

### `WindowsEventMonitor`
- **type**: struct
- **file**: `crates/hsip-intercept/src/windows/event_monitor.rs`
- **purpose**: Holds the event channel sender, cloned config, a shared `running` flag, and the (optionally uninitialized until `start()`) `IUIAutomation` COM instance wrapped in `SendSyncWrapper`.
- **called_by**: `InterceptCoordinator::new` (`#[cfg(target_os = "windows")]` branch)

### `WindowsEventMonitor::new`
- **type**: function
- **file**: `crates/hsip-intercept/src/windows/event_monitor.rs`
- **purpose**: Constructs the monitor with `automation` empty (`SendSyncWrapper::none()`) and not running — no COM/Win32 calls happen until `start()`.
- **inputs**: `event_tx: mpsc::Sender<MessagingEvent>`, `config: &InterceptConfig`
- **outputs**: `Result<Box<dyn EventMonitor>>`

### `WindowsEventMonitor::initialize_automation`
- **type**: function (private, unsafe body)
- **file**: `crates/hsip-intercept/src/windows/event_monitor.rs`
- **purpose**: Calls `CoInitializeEx(None, COINIT_APARTMENTTHREADED)` then `CoCreateInstance(&CUIAutomation, ...)` to obtain a real `IUIAutomation` COM object, storing it in `self.automation`. Unlike `main.rs`'s later-written `create_shortcuts` (see that entry for comparison), this does **not** distinguish `RPC_E_CHANGED_MODE`/`S_FALSE` from a hard failure — any non-`S_OK` `CoInitializeEx` result is treated uniformly as an error via `.ok()?`, and there is no matching `CoUninitialize` call anywhere in this function (cleanup only happens in `stop()`, and only unconditionally, regardless of whether initialization here actually succeeded or which apartment-mode outcome occurred).
- **outputs**: `Result<()>`
- **calls**: `CoInitializeEx`, `CoCreateInstance`
- **called_by**: `start`
- **mutates**: `self.automation`; process-wide COM apartment state on the calling thread

### `WindowsEventMonitor::register_event_handlers`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/windows/event_monitor.rs`
- **purpose**: Intended to register `IUIAutomationEventHandler`-based event handlers (button-click `InvokePattern` events) but that path is an explicit `TODO` — it currently only calls `register_win_event_hooks`, the `SetWinEventHook`-based polling-adjacent mechanism, as a stand-in.
- **outputs**: `Result<()>`
- **calls**: `self.register_win_event_hooks`
- **called_by**: `start`

### `WindowsEventMonitor::register_win_event_hooks`
- **type**: function (private, unsafe body)
- **file**: `crates/hsip-intercept/src/windows/event_monitor.rs`
- **purpose**: Registers two `SetWinEventHook` hooks (`EVENT_OBJECT_FOCUS` and `EVENT_OBJECT_INVOKED`), both pointed at the same `win_event_proc` callback, with `WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS`. Fails if either `SetWinEventHook` call returns an invalid handle. **The registered hooks are effectively dead weight in this version**: `win_event_proc` (below) never forwards anything into `self.event_tx` — real event delivery instead comes entirely from the independent polling loop spawned in `start()`.
- **outputs**: `Result<()>`
- **calls**: `SetWinEventHook`
- **called_by**: `register_event_handlers`

### `WindowsEventMonitor::win_event_proc`
- **type**: function (unsafe extern "system")
- **file**: `crates/hsip-intercept/src/windows/event_monitor.rs`
- **purpose**: The raw Windows callback invoked by the OS for the registered hook events. Filters to window-level events only (`id_object == OBJID_WINDOW.0`), fetches window info via `get_window_info`, and only *logs* it at debug level — the code comment explicitly notes sending the event onward would need thread-local storage or global state to reach `event_tx` from this free-standing `extern "system"` function, and that wiring was never added. Effectively a no-op beyond logging.
- **inputs**: raw Win32 hook parameters (`HWINEVENTHOOK`, `event: u32`, `hwnd: HWND`, `id_object: i32`, etc.)
- **calls**: `Self::get_window_info`
- **called_by**: the OS, via the hooks registered in `register_win_event_hooks`
- **mutates**: nothing (logging only)

### `WindowsEventMonitor::get_window_info`
- **type**: function (private, unsafe)
- **file**: `crates/hsip-intercept/src/windows/event_monitor.rs`
- **purpose**: Given an `HWND`, fetches its title (`GetWindowTextW`), class name (`GetClassNameW`), and owning process's image name (`GetWindowThreadProcessId` → `OpenProcess` → `QueryFullProcessImageNameW`, falling back to `"unknown"` if the query fails) into a `WindowInfo`. Explicitly closes the opened process handle (`CloseHandle`) before returning.
- **inputs**: `hwnd: HWND`
- **outputs**: `Result<WindowInfo>`
- **calls**: `GetWindowTextW`, `GetClassNameW`, `GetWindowThreadProcessId`, `OpenProcess`, `QueryFullProcessImageNameW`, `CloseHandle`
- **called_by**: `win_event_proc`, `poll_ui_changes`, `start`'s inline polling loop
- **mutates**: opens/closes a process handle (transient OS resource)

### `WindowsEventMonitor::poll_ui_changes`
- **type**: function (async, private)
- **file**: `crates/hsip-intercept/src/windows/event_monitor.rs`
- **purpose**: An `async` polling loop (documented as "fallback if event hooks don't work") that calls `GetForegroundWindow` every 500ms, and on a detected window change calls `handle_window_change`. **Not actually invoked from `start()`** — `start()` instead spawns its own separate, near-identical polling logic inline inside a `spawn_blocking` closure (see below), so this function and `handle_window_change` are effectively dead code duplicating that inline loop's logic.
- **outputs**: `Result<()>`
- **calls**: `GetForegroundWindow`, `Self::get_window_info`, `self.handle_window_change`
- **called_by**: *nothing currently* — dead code as of this file
- **mutates**: nothing directly (delegates to `handle_window_change`)

### `WindowsEventMonitor::handle_window_change`
- **type**: function (async, private)
- **file**: `crates/hsip-intercept/src/windows/event_monitor.rs`
- **purpose**: Classifies platform from the window's process name, and if `is_messaging_window` says yes, builds a `MessagingEvent` (bumping confidence to 0.85 if the title contains "compose"/"message"/"chat") and sends it on `self.event_tx`. Only reachable via `poll_ui_changes`, which is itself unreferenced — see above.
- **inputs**: `&self`, `window_info: &WindowInfo`
- **calls**: `PlatformType::from_process_name`, `self.is_messaging_window`, `MessagingEvent::new`
- **called_by**: `poll_ui_changes` (dead path)
- **mutates**: sends on `self.event_tx`

### `WindowsEventMonitor::is_messaging_window`
- **type**: function (private)
- **file**: `crates/hsip-intercept/src/windows/event_monitor.rs`
- **purpose**: Returns `false` immediately if the platform is disabled per config; otherwise checks the window title against a fixed keyword list (compose/message/chat/direct/dm/messenger/inbox/conversation).
- **inputs**: `&self`, `window_info: &WindowInfo`, `platform: PlatformType`
- **outputs**: `bool`
- **calls**: `self.config.is_platform_enabled`
- **called_by**: `handle_window_change`, unit test `test_messaging_window_detection`
- **mutates**: nothing

### `WindowsEventMonitor::start` / `stop` / `is_running` / `event_sender` (trait impl)
- **type**: function (async, trait impl of `EventMonitor`)
- **file**: `crates/hsip-intercept/src/windows/event_monitor.rs`
- **purpose**: `start` initializes COM/`IUIAutomation`, registers (functionally inert) event hooks, then spawns a `spawn_blocking` task containing **its own independently written 500ms polling loop** — duplicating (rather than calling) `poll_ui_changes`/`handle_window_change`'s logic inline: it grabs `tokio::runtime::Handle::current()` up front so the blocking-thread closure can still `rt.spawn` an async send of the `MessagingEvent` back onto the tokio runtime. `stop` flips `running` to false and unconditionally calls `CoUninitialize()` regardless of whether `initialize_automation` actually succeeded or what apartment-mode outcome `CoInitializeEx` returned.
- **outputs**: `Result<()>` / `bool` / `&mpsc::Sender<MessagingEvent>`
- **calls**: `self.initialize_automation`, `self.register_event_handlers`, `tokio::task::spawn_blocking`, `GetForegroundWindow`, `WindowsEventMonitor::get_window_info`, `PlatformType::from_process_name`, `MessagingEvent::new`, `CoUninitialize`
- **called_by**: `InterceptCoordinator::run`
- **mutates**: `self.running`, `self.automation`; process-wide COM state

### `WindowInfo`
- **type**: struct (private)
- **file**: `crates/hsip-intercept/src/windows/event_monitor.rs`
- **purpose**: Plain title/class_name/process_name bundle used to detect "did the foreground window change" via `PartialEq`/`Eq` comparison between polling ticks.
- **called_by**: `get_window_info`, `poll_ui_changes`, `handle_window_change`, `start`'s inline loop

### `open_messenger_window`
- **type**: function (async)
- **file**: `crates/hsip-intercept/src/windows/messenger.rs`
- **purpose**: Intended to launch/activate the HSIP Messenger window, but this is an explicit `TODO` (options listed in comments: spawn an `hsip-cli messenger` subprocess, IPC to an existing daemon, or a WebView2/native UI — none implemented). Logs a warning and, only in debug builds (`#[cfg(debug_assertions)]`), shows a raw `MessageBoxW` placeholder dialog naming the intended recipient. In release builds this function does nothing observable at all beyond the log line.
- **inputs**: `recipient_hint: Option<String>`
- **outputs**: `Result<()>`
- **calls**: `MessageBoxW` (debug builds only)
- **called_by**: `HSIPRouter::open_messenger_manual` (`#[cfg(target_os = "windows")]` branch)
- **mutates**: nothing persistent (a transient debug-only dialog box)

### `extract_recipient_from_window` (`windows/messenger.rs`)
- **type**: function
- **file**: `crates/hsip-intercept/src/windows/messenger.rs`
- **purpose**: Parses two hardcoded window-title shapes: Gmail's `"Compose - <email> - Gmail"` (extracts the substring between `"Compose - "` and the following `" - "`), and Instagram's `"Direct - @username"` (finds the `@` and takes the run of non-whitespace/non-`)` characters after it). Falls back to `event.metadata["recipient"]` if set, otherwise errors. Unlike the Linux/macOS equivalents, this is pattern-matched to two specific literal title formats rather than a general prefix/separator heuristic.
- **inputs**: `event: &MessagingEvent`
- **outputs**: `Result<String>`
- **called_by**: `InterceptCoordinator::extract_recipient` (`#[cfg(target_os = "windows")]` branch)
- **mutates**: nothing

### `to_wide_string` / `from_wide_string` (`windows/utils.rs`)
- **type**: function
- **file**: `crates/hsip-intercept/src/windows/utils.rs`
- **purpose**: `to_wide_string` converts a Rust `&str` to a null-terminated UTF-16 `Vec<u16>` for Win32 API calls expecting a wide string; `from_wide_string` does the reverse, stopping at the first null terminator (or the slice's end if none is found) and lossily converting invalid UTF-16 sequences. These are general-purpose duplicates of the same wide-string conversion `main.rs::to_wide` in `hsip-api` implements independently for its own Windows shortcut-creation code — the two crates don't share this helper.
- **inputs/outputs**: `to_wide_string(s: &str) -> Vec<u16>`; `from_wide_string(wide: &[u16]) -> String`
- **called_by**: not currently called elsewhere in this crate (`windows/overlay.rs` and `windows/event_monitor.rs` each inline their own ad hoc UTF-16 conversions rather than reusing these) — available utility functions, exercised only by this file's own unit test
- **mutates**: nothing

### `SendableHwnd`
- **type**: struct
- **file**: `crates/hsip-intercept/src/windows/overlay.rs`
- **purpose**: Wraps a raw `HWND` and unsafely asserts `Send + Sync` so it can be stored in `Arc<Mutex<Option<SendableHwnd>>>` and survive across `.await` points inside async functions — `HWND` itself isn't `Send`, and the surrounding code specifically structures `show`/`wait_for_choice` to fetch the handle out of this wrapper only inside non-async scopes, to avoid holding a raw `HWND` live across an `await`.
- **called_by**: `WindowsOverlay` (as the type inside its `hwnd` field)

### `WindowsOverlay`
- **type**: struct
- **file**: `crates/hsip-intercept/src/windows/overlay.rs`
- **purpose**: Renders the intercept prompt as a real custom Win32 layered window (`WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW`), positioned per `OverlayConfig.position`, drawn with raw GDI in its `window_proc`. Tracks the current window handle and the resolved `UserChoice` behind `Arc<Mutex<...>>` so they can be shared safely between the async caller and the dedicated OS message-loop thread.
- **called_by**: `InterceptCoordinator::new` (`#[cfg(target_os = "windows")]` branch)

### `WindowsOverlay::new`
- **type**: function
- **file**: `crates/hsip-intercept/src/windows/overlay.rs`
- **purpose**: Constructs the overlay with no window yet created and no choice recorded.
- **inputs**: `config: &InterceptConfig`
- **outputs**: `Result<Box<dyn InterceptOverlay>>`

### `WindowsOverlay::create_overlay_window`
- **type**: function (private, unsafe body)
- **file**: `crates/hsip-intercept/src/windows/overlay.rs`
- **purpose**: Registers a window class (`"HSIPInterceptOverlay"`, black background brush), computes screen position from `calculate_overlay_position`, creates a topmost, tool, layered popup window at that position/size via `CreateWindowExW`, sets 240/255 alpha transparency via `SetLayeredWindowAttributes`, then shows and updates it. Returns the resulting `HWND`.
- **inputs**: `&self`, `content: &OverlayContent`
- **outputs**: `Result<HWND>`
- **calls**: `GetModuleHandleW`, `RegisterClassW`, `self.calculate_overlay_position`, `CreateWindowExW`, `SetLayeredWindowAttributes`, `ShowWindow`, `UpdateWindow`
- **called_by**: `show`
- **mutates**: creates an OS window (visible screen state)

### `WindowsOverlay::calculate_overlay_position`
- **type**: function (private, unsafe body)
- **file**: `crates/hsip-intercept/src/windows/overlay.rs`
- **purpose**: Computes a fixed 400×200 window's `(x, y)` origin for each `OverlayPosition` variant based on `GetSystemMetrics(SM_CXSCREEN/SM_CYSCREEN)` — the only overlay implementation in this crate that actually reads `OverlayConfig.position` (Linux/macOS notifications don't have a configurable on-screen position).
- **outputs**: `(i32, i32, i32, i32)` (x, y, width, height)
- **calls**: `GetSystemMetrics`
- **called_by**: `create_overlay_window`, unit test `test_position_calculation`

### `WindowsOverlay::window_proc`
- **type**: function (unsafe extern "system")
- **file**: `crates/hsip-intercept/src/windows/overlay.rs`
- **purpose**: The window procedure for the overlay window. `WM_PAINT` fills a dark-gray background and draws the fixed placeholder text `"Send through HSIP instead?"` centered (a `TODO` notes real content rendering — title/message/recipient text from `OverlayContent` is not actually drawn, just this one hardcoded string) — regardless of the actual `OverlayContent` passed to `create_overlay_window`. `WM_LBUTTONDOWN` posts a `WM_CLOSE` with `wParam=1` (interpreted downstream as "Send Privately"); `WM_RBUTTONDOWN` posts `WM_CLOSE` with `wParam=0` ("Continue"). `WM_DESTROY` calls `PostQuitMessage`.
- **inputs**: `hwnd: HWND`, `msg: u32`, `wparam: WPARAM`, `lparam: LPARAM`
- **outputs**: `LRESULT`
- **calls**: `BeginPaint`/`EndPaint`, `CreateSolidBrush`/`FillRect`/`DeleteObject`, `DrawTextW`, `PostMessageW`, `PostQuitMessage`, `DefWindowProcW`
- **called_by**: the OS message dispatcher (registered as `WNDCLASSW.lpfnWndProc`)

### `WindowsOverlay::wait_for_choice`
- **type**: function (async, private)
- **file**: `crates/hsip-intercept/src/windows/overlay.rs`
- **purpose**: Spawns a dedicated OS thread (`std::thread::spawn`, deliberately *not* a tokio task, since it must run a native Win32 message loop with `GetMessageW`/`TranslateMessage`/`DispatchMessageW`) that blocks until it sees the `WM_CLOSE` message posted by `window_proc`, decodes the choice from `wParam`, and stores it in the shared `Arc<Mutex<Option<UserChoice>>>`. The calling async function meanwhile polls that mutex every 100ms with a hard timeout from `config.overlay.timeout_seconds`; on timeout it posts its own synthetic `WM_CLOSE`(wParam=0) to force the message-loop thread to exit and returns `Continue` immediately rather than waiting for that thread to actually process the synthetic message.
- **outputs**: `Result<UserChoice>`
- **calls**: `std::thread::spawn`, `GetMessageW`, `TranslateMessage`, `DispatchMessageW`, `PostMessageW`, `tokio::time::sleep`
- **called_by**: `show`
- **mutates**: `self.choice` (via the shared Mutex, from the spawned OS thread)

### `WindowsOverlay::show` / `hide` / `is_visible` (trait impl)
- **type**: function (async, trait impl of `InterceptOverlay`)
- **file**: `crates/hsip-intercept/src/windows/overlay.rs`
- **purpose**: `show` builds content, creates the overlay window in a scoped (non-`.await`-spanning) block and stores its `HWND` (wrapped in `SendableHwnd`) before awaiting `wait_for_choice`, then calls `hide` to tear the window down before returning the resolved choice. `hide` destroys the window (`DestroyWindow`) if one is currently tracked, clearing `self.hwnd`. `is_visible` reports whether `self.hwnd` currently holds a handle.
- **outputs**: `Result<UserChoice>` / `Result<()>` / `bool`
- **calls**: `OverlayContent::from_event`, `self.create_overlay_window`, `self.wait_for_choice`, `self.hide`, `DestroyWindow`
- **called_by**: `InterceptCoordinator::show_intercept_overlay`
- **mutates**: `self.hwnd`; creates/destroys an OS window

---

## `crates/hsip-common/src/lib.rs`

Crate root for `hsip-common`. Re-exports `quantum_physics` (see below) as the crate's entire public surface — there is no other functionality in this crate. The module doc comment maps each "quantum physics" name to the practical feature it actually implements: No-Cloning → single-use nonce/anti-replay, Decoherence → auto-expiring consent/sessions, Observer Effect → read receipts/access logging, Superposition → hidden message state until revealed, Entanglement → mutual/bidirectional consent linkage, Uncertainty → a privacy/performance tradeoff slider. The physics framing is a deliberate naming choice (per project history, to appeal to security researchers), not an indication of anything quantum-mechanical in the implementation — everything underneath is ordinary classical cryptography (BLAKE3 hashing, XOR/keyed-hash "encryption," `chrono` timestamps, `parking_lot` locks).

### `quantum_physics` (re-export)
- **type**: module re-export (`pub use quantum_physics::*`)
- **file**: `crates/hsip-common/src/lib.rs`
- **purpose**: Flattens all six `quantum_physics` submodules' public items into the crate root, so callers write `hsip_common::QuantumNonce` etc. instead of `hsip_common::quantum_physics::no_cloning::QuantumNonce`.
- **calls**: none
- **called_by**: `hsip-telemetry-guard` (`use hsip_common::quantum_physics::uncertainty::PrivacyLevel`, etc.)
- **mutates**: nothing

---

## `crates/hsip-common/src/quantum_physics/mod.rs`

Declares and re-exports the six "quantum physics" submodules. Purely organizational — no logic of its own.

### `decoherence` / `entanglement` / `no_cloning` / `observer_effect` / `superposition` / `uncertainty` (module declarations + re-exports)
- **type**: module declarations (`pub mod ...`) + wildcard re-exports (`pub use ...::*`)
- **file**: `crates/hsip-common/src/quantum_physics/mod.rs`
- **purpose**: Makes every submodule's public types directly reachable as `hsip_common::quantum_physics::<Type>` in addition to their fully-qualified submodule paths.
- **calls**: none
- **called_by**: `hsip-common/src/lib.rs`, `hsip-telemetry-guard` (imports `uncertainty::PrivacyLevel` and `observer_effect::{ObservationLog, ObservationType}` directly)
- **mutates**: nothing

---

## `crates/hsip-common/src/quantum_physics/decoherence.rs`

Real property implemented: **time-based automatic expiry** (consent TTLs, session idle timeouts, key-rotation intervals) — "decoherence" is just the name for "this state stops being valid after enough time/inactivity passes." Nothing here is cryptographic on its own; `DecoherenceState`/`DecayingConsent`/`DecayingSession` are plain `chrono`-timestamp bookkeeping structs that other code (e.g. `hsip-telemetry-guard`'s `ConsentGate`/`TelemetryConsent`, which is BLAKE3-signed) is expected to combine with real cryptographic enforcement. This module by itself does not stop anyone from ignoring `is_valid()` and using an "expired" value anyway — it only supplies the expiry computation and revocation-reason bookkeeping.

### `DecoherenceError`
- **type**: enum
- **file**: `crates/hsip-common/src/quantum_physics/decoherence.rs`
- **purpose**: Error variants for validation failures: `ConsentExpired` (used for both hard expiry and explicit revocation), `SessionIdle`, `KeyExpired` (declared, not currently produced by any method here), `InvalidConfig` (declared, unused).
- **called_by**: `DecoherenceState::validate`

### `DecoherenceConfig`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/decoherence.rs`
- **purpose**: Holds the three tunable lifetimes (consent, session-idle, key-rotation) plus a grace period field (declared but not read by any method in this file — clock-drift tolerance is not actually applied anywhere yet). `Default` derives from the module's `DEFAULT_*` constants (90-day consent, 24h idle, 30-day key rotation).
- **outputs**: `Self` (via `Default`/construction)
- **called_by**: `DecoherenceState::new_consent`, `DecayingConsent::new`, `DecayingSession::new`, `DecoherenceManager`

### `DecoherenceState`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/decoherence.rs`
- **purpose**: The core expiry primitive: `created_at`/`expires_at` (hard deadline) plus `last_activity`/`idle_timeout_secs` (soft, resettable-by-`touch()` deadline) plus an explicit `revoked`/`revocation_reason` flag. `is_valid()` = not revoked AND not past `expires_at` AND not idle past `last_activity + idle_timeout_secs`. `decoherence_factor()` gives a 0.0–1.0 "how expired is this" ratio (elapsed/total lifetime, clamped) purely for UI/display use — not a security check.
- **inputs**: `lifetime_secs: u64, idle_timeout_secs: u64` (`new`); `expires_at: DateTime<Utc>, idle_timeout_secs: u64` (`with_expiry`)
- **outputs**: `Self`; `bool` (`is_valid`/`is_expired`/`is_idle`); `Result<(), DecoherenceError>` (`validate`); `i64` (`remaining_lifetime_secs`/`idle_duration_secs`); `f64` (`decoherence_factor`)
- **calls**: `chrono::Utc::now()`
- **called_by**: `DecayingConsent`, `DecayingSession`, `DecoherenceManager::should_purge_*`

### `DecayingConsent`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/decoherence.rs`
- **purpose**: A consent record (grantor/grantee/purpose/scope list) wrapping a `DecoherenceState`. `allows(action)` checks both validity and scope membership (empty scope = allows everything valid). `revoke`/`renew` delegate straight to the inner state.
- **inputs**: `consent_id, grantor_id, grantee_id, purpose: String, lifetime_days: i64` (`new`)
- **outputs**: `Self`; `bool` (`allows`); `Result<ConsentStatus, DecoherenceError>` (`validate`)
- **calls**: `DecoherenceState::new`
- **called_by**: not consumed elsewhere in this workspace as of this writing (no call sites in `hsip-telemetry-guard`, which has its own parallel `TelemetryConsent` type instead) — available as a general-purpose primitive.
- **mutates**: `self.state`, `self.scope`

### `DecayingSession`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/decoherence.rs`
- **purpose**: A session wrapper around `DecoherenceState` that also tracks `message_count`; `record_message()` bumps the count and calls `touch()` to reset the idle clock on every message.
- **inputs**: `session_id, peer_id: String` (`new`)
- **outputs**: `Self`; `bool` (`is_alive`); `Result<(), DecoherenceError>` (`validate`)
- **calls**: `DecoherenceState::new`, `DecoherenceState::touch`
- **mutates**: `self.state`, `self.message_count`

### `DecoherenceManager`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/decoherence.rs`
- **purpose**: Stateless-except-for-config helper for bulk expiry checks: `should_purge_consent`/`should_purge_session` (pure boolean checks) and `filter_valid_consents` (retains only currently-valid consents from a `Vec`).
- **inputs**: `config: DecoherenceConfig` (`with_config`)
- **outputs**: `bool`; `Vec<DecayingConsent>`
- **calls**: `DecoherenceState::is_valid`
- **mutates**: nothing (all methods take `&self`, operate on caller-supplied data)

---

## `crates/hsip-common/src/quantum_physics/entanglement.rs`

Real property implemented: **bidirectionally-linked consent state between two (or more) parties**, i.e. a shared consent record both sides reference by the same ID rather than each tracking their own independent copy — "entanglement" names the fact that revoking on one side is meant to immediately affect the other because it's the *same* record, not two synced copies. The cryptography here (`Hasher::new_keyed(&self.shared_secret)`) produces a MAC-like `proof_hash` that lets a holder of `shared_secret` assert "this is the current state," but `shared_secret` is generated fresh per entanglement and never given to either party in this module — nothing in this file hands `shared_secret` to `party_a`/`party_b`, so `generate_proof`/`verify_proof` are currently only self-checks by whoever holds the `EntangledConsent`/`EntanglementManager` instance, not yet an inter-party protocol. `verify_history()` is a stub — it only checks the history vector is non-empty, it does not actually recompute and check the hash chain.

### `EntanglementError`
- **type**: enum
- **file**: `crates/hsip-common/src/quantum_physics/entanglement.rs`
- **purpose**: Error variants: `AlreadyEntangled`, `NotFound`, `BrokenByParty` (carries the revoking party's hex id), `InvalidProof` (used both for actual proof failures and for "wrong state to transition from" — e.g. calling `activate()` when not `Pending`), `Unauthorized`, `Expired`, `InsufficientParties`.
- **called_by**: `EntangledConsent`, `GroupEntanglement`, `EntanglementManager`

### `EntanglementState`
- **type**: enum
- **file**: `crates/hsip-common/src/quantum_physics/entanglement.rs`
- **purpose**: `Pending | Active | Revoked | Expired | Suspended` — the finite state machine both `EntangledConsent` and `GroupEntanglement` drive through.
- **called_by**: `EntangledConsent::transition_state`, `GroupEntanglement`

### `EntangledConsent`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/entanglement.rs`
- **purpose**: A pairwise consent link between `party_a`/`party_b` (32-byte identifiers — typically a public-key hash). `new()` generates a random `entanglement_id` and `shared_secret` and seeds a 1-entry `state_history`. `activate`/`revoke`/`suspend`/`resume` are guarded transitions (each rejects if not currently in the expected prior state) that all funnel through `transition_state`, which checks `expires_at` first (flipping to `Expired` and returning `Err` if past it) then appends a new BLAKE3 state-hash to `state_history`. `revoke` additionally checks the caller is one of the two parties.
- **inputs**: `party_a: [u8; 32], party_b: [u8; 32], expires_at: Option<DateTime<Utc>>` (`new`)
- **outputs**: `Self`; `Result<(), EntanglementError>` (transition methods); `bool` (`is_active`/`involves_party`/`verify_proof`/`verify_history`); `Option<[u8; 32]>` (`other_party`); `EntanglementProof` (`generate_proof`)
- **calls**: `rand::thread_rng().fill_bytes`, `blake3::Hasher::{new, new_keyed}`
- **called_by**: `EntanglementManager::create_pairwise`/`activate`/`revoke`
- **mutates**: `self.state`, `self.updated_at`, `self.state_history`

### `EntanglementProof`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/entanglement.rs`
- **purpose**: A snapshot (`entanglement_id`, `state`, `timestamp`, keyed-hash `proof_hash`) that `EntangledConsent::verify_proof` can check against the live object — proves "this proof was generated by someone holding `shared_secret` for this exact state/timestamp," not an inter-party signature.
- **called_by**: `EntangledConsent::{generate_proof, verify_proof}`

### `GroupEntanglement`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/entanglement.rs`
- **purpose**: Threshold multi-party consent — `parties` is the full member list, `threshold` is the minimum number who must call `add_consent` before `state` flips `Pending → Active`. `add_consent` returns `Ok(true)` exactly on the transition where the threshold is first reached (not on every subsequent call). `remove_consent` drops back to `Suspended` if the count falls below threshold after having been `Active`. Rejects non-member parties and expired groups.
- **inputs**: `parties: Vec<[u8;32]>, threshold: usize, expires_at: Option<DateTime<Utc>>` (`new`, errors if `threshold == 0` or `> parties.len()`)
- **outputs**: `Result<Self, EntanglementError>`; `Result<bool, EntanglementError>` (`add_consent`); `bool` (`is_active`/`has_consented`)
- **mutates**: `self.consenting`, `self.state`, `self.updated_at`

### `EntanglementManager`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/entanglement.rs`
- **purpose**: Thread-safe (`parking_lot::RwLock`-guarded) registry of both pairwise and group entanglements, plus a `party_index` for O(1)-ish lookup of "all entanglements involving this party." `create_pairwise` refuses to create a duplicate between the same two parties (`get_entanglement_between` scans the requesting party's indexed IDs). `cleanup_expired` removes any pairwise/group entanglement whose `expires_at` has passed.
- **outputs**: `Result<[u8; 32], EntanglementError>` (create methods, returns the new ID); `bool` (`are_entangled`/`is_group_active`); `usize` (`cleanup_expired`, count removed); `EntanglementStats`
- **calls**: `EntangledConsent::{new, activate, revoke}`, `GroupEntanglement::{new, add_consent, remove_consent}`
- **called_by**: not yet wired into any other crate in this workspace — a standalone primitive.
- **mutates**: `self.pairwise`, `self.groups`, `self.party_index`

### `EntanglementStats`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/entanglement.rs`
- **purpose**: Plain counts (`total_pairwise`, `active_pairwise`, `total_groups`, `active_groups`) returned by `EntanglementManager::stats()`.
- **called_by**: `EntanglementManager::stats`

---

## `crates/hsip-common/src/quantum_physics/no_cloning.rs`

Real property implemented: **anti-replay / single-use tokens** — the literal quantum no-cloning theorem ("you can't copy an unknown quantum state") is used only as a naming metaphor for "this token, once consumed, cannot be validly presented again." The actual mechanism is a sliding-window nonce cache (`AntiReplayGuard`) plus a payload-binding hash (`SingleUseToken`), both ordinary classical constructions — BLAKE3 hashing and a `HashSet` of seen-nonce hashes with time-based eviction.

### `QuantumNonce`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/no_cloning.rs`
- **purpose**: A 24-byte (`NONCE_SIZE`) cryptographically random value plus a creation timestamp. `generate()` uses `OsRng`. `is_within_window(window_secs)` is the freshness check consumers must apply before accepting a nonce — note `from_hex` deliberately cannot recover the true original `created_at_ms` (hex encoding only carries the 24 nonce bytes, not the timestamp), so it substitutes the current time; this makes a nonce reconstructed from hex always appear "fresh" regardless of when it was actually generated, which is a genuine sharp edge for any caller round-tripping nonces through hex and then checking their age.
- **outputs**: `Self` (`generate`); `bool` (`is_within_window`); `String` (`to_hex`); `Result<Self, NoClonError>` (`from_hex`)
- **calls**: `rand::rngs::OsRng::fill_bytes`, `std::time::SystemTime::now`
- **called_by**: `SingleUseToken::new`, `AntiReplayGuard`

### `NoClonError`
- **type**: enum
- **file**: `crates/hsip-common/src/quantum_physics/no_cloning.rs`
- **purpose**: `ReplayDetected`, `NonceExpired`, `InvalidNonce`, `SessionMismatch` (declared, not currently produced anywhere in this file), `VerificationFailed`.
- **called_by**: `AntiReplayGuard::check_and_mark`, `SingleUseToken::validate_and_consume`

### `AntiReplayGuard`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/no_cloning.rs`
- **purpose**: The actual anti-replay enforcement point — this is HSIP's genuine no-cloning-theorem implementation, and it is distinct from (and predates, in the workspace) `hsip-telemetry-guard::ConsentGate`'s own similar-looking `consumed_tokens` map. Tracks seen-nonce BLAKE3 hashes (optionally salted with a bound `session_id`, so two different sessions can independently accept the same raw nonce bytes) in a `HashSet` plus an ordered `Vec<NonceEntry>` for age-based eviction. `check_and_mark` double-checks for a replay both under the read lock and again after acquiring the write lock (closing the check-then-act race between the two lock acquisitions). Auto-evicts entries older than `window` on every check, and additionally prunes the oldest 10% if the cache exceeds `MAX_NONCE_CACHE_SIZE` (100,000).
- **inputs**: `window: Duration` (`with_window`); `nonce: &QuantumNonce` (`check_and_mark`/`would_accept`)
- **outputs**: `Self`; `Result<(), NoClonError>` (`check_and_mark`); `bool` (`would_accept`); `usize` (`tracked_count`)
- **calls**: `blake3::Hasher`, `parking_lot::RwLock`
- **called_by**: `SingleUseToken::validate_and_consume`, `new_shared_guard`
- **mutates**: `self.seen_nonces`, `self.nonce_order`

### `SingleUseToken`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/no_cloning.rs`
- **purpose**: Bundles an arbitrary `payload: Vec<u8>` (e.g. consent data) with a fresh `QuantumNonce` and a domain-separated (`"HSIP-NO-CLONE-BINDING-v1"`) BLAKE3 hash binding the two together, so a captured token's payload can't be swapped without invalidating `binding`. `validate_and_consume` checks `verify_integrity()` first, then delegates the actual replay check to the caller-supplied `AntiReplayGuard`.
- **inputs**: `payload: Vec<u8>` (`new`); `guard: &AntiReplayGuard` (`validate_and_consume`)
- **outputs**: `Self`; `bool` (`verify_integrity`); `Result<(), NoClonError>` (`validate_and_consume`)
- **calls**: `QuantumNonce::generate`, `AntiReplayGuard::check_and_mark`
- **mutates**: nothing itself (mutation happens inside the passed `guard`)

### `SharedAntiReplayGuard` / `new_shared_guard`
- **type**: type alias (`Arc<AntiReplayGuard>`) / function
- **file**: `crates/hsip-common/src/quantum_physics/no_cloning.rs`
- **purpose**: Convenience constructor for a thread-shareable guard.
- **outputs**: `SharedAntiReplayGuard`
- **calls**: `AntiReplayGuard::new`

---

## `crates/hsip-common/src/quantum_physics/observer_effect.rs`

Real property implemented: **tamper-evident access logging / read receipts** — every "observation" (read, decrypt, export, etc.) of a resource produces a cryptographically-bound, hash-chained record, so after the fact you can prove both that an access happened and that the log of accesses hasn't been reordered or edited. This is the same hash-chaining pattern `hsip-api`'s `audit_log.rs` uses for the HTTP audit trail (BLAKE3, `prev_hash`/`entry_hash` linkage) — this module is `hsip-telemetry-guard::AuditLog`'s and `hsip-telemetry-guard::ConsentGate`'s underlying receipt primitive (see `audit.rs` below, which wraps an `ObservationLog` internally).

### `ObserverError`
- **type**: enum
- **file**: `crates/hsip-common/src/quantum_physics/observer_effect.rs`
- **purpose**: `InvalidSignature`, `ChainIntegrityViolation(usize)` (carries the breaking index), `ObservationNotFound` (declared, unused in this file), `Unauthorized`, `ResourceNotFound` (declared, unused in this file).
- **called_by**: `ObservationLog::verify_chain`, `ResourceObserver::observe`

### `ObservationType`
- **type**: enum
- **file**: `crates/hsip-common/src/quantum_physics/observer_effect.rs`
- **purpose**: `Read | MetadataAccess | ConsentCheck | KeyDerivation | Decryption | Export | Forward`. `requires_explicit_log()` flags the three most sensitive kinds (`Decryption`, `Export`, `Forward`) as needing explicit logging — this is advisory metadata only; nothing in this file actually enforces that flag (no caller is forced to check it before proceeding).
- **called_by**: `ReadReceipt`, `ObservationLog::record_observation`, `hsip-telemetry-guard::audit.rs` (maps its own `DecisionType` onto these)

### `ReadReceipt`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/observer_effect.rs`
- **purpose**: The actual "read receipt": binds `resource_id`, `observer_id`, `observation_type`, `timestamp`, and an optional `prev_receipt_hash` (for chaining) under a keyed BLAKE3 `proof` computed from a caller-supplied `binding_secret`. `verify()` recomputes and compares; a wrong `binding_secret` or any tampered field fails verification.
- **inputs**: `resource_id, observer_id: [u8;32], observation_type: ObservationType, prev_receipt_hash: Option<[u8;32]>, binding_secret: &[u8]` (`new`)
- **outputs**: `Self`; `bool` (`verify`); `[u8; 32]` (`hash`, for chaining to the next receipt)
- **calls**: `blake3::Hasher::{new, new_keyed}`
- **called_by**: `ObservationLog::record_observation`

### `ObservationRecord`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/observer_effect.rs`
- **purpose**: The actual hash-chain link stored in `ObservationLog`: wraps a `ReadReceipt` plus an `index`, optional `encrypted_context`, `prev_hash`, and its own `record_hash` (BLAKE3 over index/receipt-id/receipt-proof/context/prev_hash). `verify_integrity()` recomputes and compares `record_hash`.
- **inputs**: `index: u64, receipt: ReadReceipt, encrypted_context: Option<Vec<u8>>, prev_hash: [u8;32]` (`new`)
- **outputs**: `Self`; `bool` (`verify_integrity`)
- **calls**: `blake3::Hasher::new`
- **called_by**: `ObservationLog::{record_observation, verify_chain}`

### `ObservationLog`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/observer_effect.rs`
- **purpose**: The append-only, hash-chained log itself, plus two secondary indices (`resource_index`, `observer_index`) for fast filtering. `record_observation` chains each new record to the previous one's `record_hash` (genesis is `[0u8;32]`) and returns the freshly created `ReadReceipt`. `verify_chain()` walks every record checking both per-record integrity and correct `prev_hash` linkage and receipt-proof validity — a single pass, `O(n)`, same "recompute the whole thing" pattern as `hsip-api`'s `GET /v1/audit/verify` (and shares its scaling caveat: no checkpointing, cost grows with log size).
- **inputs**: `binding_secret: [u8; 32]` (`new`)
- **outputs**: `Self`; `ReadReceipt` (`record_observation`); `Result<(), ObserverError>` (`verify_chain`); `Vec<ReadReceipt>` (`get_observations_for_resource`/`get_observations_by_observer`); `usize` (`len`); `Option<[u8;32]>` (`latest_hash`)
- **calls**: `ReadReceipt::new`, `ObservationRecord::new`
- **called_by**: `hsip-telemetry-guard::audit.rs::AuditLog` (embeds one as a secondary cryptographic-receipt log alongside its own primary `AuditEntry` chain), `ResourceObserver`
- **mutates**: `self.records`, `self.resource_index`, `self.observer_index`

### `ResourceObserver`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/observer_effect.rs`
- **purpose**: Per-resource authorization wrapper around a shared `ObservationLog`: maintains its own `authorized_observers` list and refuses to record an observation (`Err(ObserverError::Unauthorized)`) from anyone not on it, before delegating to `log.record_observation`.
- **inputs**: `log: SharedObservationLog, resource_id: [u8;32]` (`new`)
- **outputs**: `Self`; `Result<ReadReceipt, ObserverError>` (`observe`); `Vec<ReadReceipt>` (`get_receipts`)
- **calls**: `ObservationLog::record_observation`
- **mutates**: `self.authorized_observers`

---

## `crates/hsip-common/src/quantum_physics/superposition.rs`

Real property implemented: **state privacy via commitment/reveal** — a message's status (read/unread/deleted/etc.) is stored only as an encrypted+committed value ("in superposition") until explicitly decrypted ("collapsed"), so an observer holding the data structure alone (without the encryption/commitment keys) cannot tell what state it's in. `QuantumSealedEnvelope`'s "cover traffic" half additionally hides *whether a message exists at all* by making real and decoy envelopes byte-identical in shape. The "encryption" here is a single-byte XOR keyed off a BLAKE3-derived keystream byte — cryptographically trivial (not a real stream cipher; it only ever encrypts one byte, the `MessageState` discriminant) and should not be read as HSIP's actual message-content encryption (that's ChaCha20-Poly1305 elsewhere in `hsip-core`/`hsip-api`); this module's contribution is the state-hiding *protocol shape* (commit, then later reveal-and-verify), not production-grade encryption.

### `SuperpositionError`
- **type**: enum
- **file**: `crates/hsip-common/src/quantum_physics/superposition.rs`
- **purpose**: `AlreadyCollapsed` (also returned, not just as an error path — see `SuperpositionState::collapse`), `InvalidCollapseProof`, `Unauthorized` (declared, unused in this file), `StateNotFound`, `ImmutableState` (declared, unused).
- **called_by**: `SuperpositionState::collapse`, `SuperpositionManager`

### `MessageState`
- **type**: enum
- **file**: `crates/hsip-common/src/quantum_physics/superposition.rs`
- **purpose**: `Pending | Delivered | Read | Acknowledged | Deleted | Expired` — the hidden value being committed to/revealed. Cast to `u8` (0–5) for the XOR encoding in `SuperpositionState`.
- **called_by**: `StateCommitment`, `SuperpositionState`

### `StateCommitment`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/superposition.rs`
- **purpose**: A hiding commitment to a `MessageState`: `commitment = keyed_BLAKE3(secret, state_byte || nonce)`. `verify(state, secret)` recomputes and compares — this is what "collapsing" a superposition means concretely: proving a specific state matches a previously-published commitment without the commitment itself having revealed it.
- **inputs**: `state: MessageState, secret: &[u8; 32]` (`new`)
- **outputs**: `Self`; `bool` (`verify`)
- **calls**: `blake3::Hasher::new_keyed`
- **called_by**: `SuperpositionState::{new, collapse}`

### `SuperpositionState`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/superposition.rs`
- **purpose**: Combines an XOR-"encrypted" single-byte state with a `StateCommitment` and a `collapsed` flag. `collapse()` decrypts, matches the byte back to a `MessageState`, verifies it against the stored commitment, and — only once — records `revealed_state`/`collapsed_at`; a second call to `collapse()` (once already collapsed) short-circuits and returns the previously revealed value rather than re-deriving it, so it is idempotent rather than erroring, despite `SuperpositionError::AlreadyCollapsed` existing as a name (it only surfaces if `revealed_state` is somehow `None` on an already-collapsed instance, which the normal API can't produce). Derives `Zeroize`/`ZeroizeOnDrop` on `encrypted_state` implicitly (most fields are `#[zeroize(skip)]`, so only the raw encrypted byte vector is actually zeroized on drop).
- **inputs**: `entity_id: [u8;32], state: MessageState, encryption_key: &[u8;32], commitment_secret: &[u8;32]` (`new`); same two keys again (`collapse`)
- **outputs**: `Self`; `Result<MessageState, SuperpositionError>` (`collapse`); `bool` (`is_superposed`); `Option<MessageState>` (`revealed_state`)
- **calls**: `blake3::Hasher::new_keyed`, `StateCommitment::{new, verify}`
- **called_by**: `SuperpositionManager`
- **mutates**: `self.collapsed`, `self.collapsed_at`, `self.revealed_state`

### `SuperpositionManager`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/superposition.rs`
- **purpose**: Keyed store (`entity_id -> SuperpositionState`) holding one shared `encryption_key`/`commitment_secret` pair for all managed states. `transition_state` replaces an existing entry's state with a brand-new `SuperpositionState` (a fresh nonce/commitment each time) rather than mutating in place — each transition is its own independent commitment, so a `SuperpositionState`'s `collapsed` history doesn't carry across transitions.
- **inputs**: `encryption_key, commitment_secret: [u8;32]` (`new`)
- **outputs**: `StateCommitment` (`create_state`/`transition_state`, `Result` for the latter); `Result<MessageState, SuperpositionError>` (`collapse_state`); `Option<bool>` (`is_collapsed`)
- **calls**: `SuperpositionState::{new, collapse}`
- **mutates**: `self.states`

### `QuantumSealedEnvelope`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/superposition.rs`
- **purpose**: Existence-hiding cover traffic: `seal_real` pads a real payload to `max_size` and commits `is_real = true`; `seal_cover` fills `max_size` bytes with random noise and commits `is_real = false`. Both produce a struct of identical shape/size (`envelope_id`, `sealed_payload`, `batch_timestamp` all present either way), so an observer without `secret` cannot distinguish a real message from decoy traffic by inspecting the envelope alone (`test_envelope_indistinguishability` asserts exactly this). `open()` only returns `Some(payload)` if `verify_real` succeeds *and* `is_real` — otherwise `None`, whether because it's cover traffic or the commitment doesn't verify. Note: `open()`'s comment on removing padding is aspirational — the actual implementation returns the full padded buffer verbatim, it does not truncate back to the original payload length.
- **inputs**: `payload: &[u8], max_size: usize, batch_timestamp: DateTime<Utc>, secret: &[u8;32]` (`seal_real`); `max_size, batch_timestamp, secret` (`seal_cover`, no payload)
- **outputs**: `Self`; `bool` (`verify_real`); `Option<Vec<u8>>` (`open`)
- **calls**: `rand::thread_rng().fill_bytes`, `blake3::Hasher::new_keyed`

---

## `crates/hsip-common/src/quantum_physics/uncertainty.rs`

Real property implemented: **a discrete privacy/performance tradeoff configuration** — not cryptography at all, but a lookup table (`PrivacyLevel` 0–4 → `PrivacyFeatures`/`PerformanceImpact`/`EncryptionParams`) describing which mitigations (metadata hiding, cover traffic, multi-hop, padding, KDF iteration count, etc.) are conceptually "on" at each level, plus estimated (hardcoded, not measured) performance costs. "Uncertainty principle" names the tradeoff itself (more privacy protections ⇒ less performance, and vice versa) — nothing here actually measures or enforces performance; the multipliers/delays in `PerformanceImpact::for_level` are fixed illustrative constants, not derived from real benchmarking.

### `PrivacyLevel`
- **type**: enum (`#[repr(u8)]`, `Minimal=0 | Basic=1 | Balanced=2 (default) | Enhanced=3 | Maximum=4`)
- **file**: `crates/hsip-common/src/quantum_physics/uncertainty.rs`
- **purpose**: The discrete slider position everything else in this file is keyed off. `value()`/`from_value()` round-trip to `u8`; `normalized()` maps to 0.0–1.0.
- **outputs**: `u8`/`Option<Self>`/`&'static str`/`f32`
- **called_by**: `PrivacyFeatures::for_level`, `PerformanceImpact::for_level`, `EncryptionParams::from_level`, `UncertaintyConfig`, `TradeoffSummary::for_level`, `SliderData`; consumed externally by `hsip-telemetry-guard::ConsentGate`/`PolicyEngine`/`TelemetryGuard` to gate/block traffic at `Maximum`.

### `PrivacyFeatures`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/uncertainty.rs`
- **purpose**: Boolean feature flags (content encryption always on; metadata hiding, traffic-analysis resistance, cover traffic, delayed delivery, multi-hop, receipt/typing hiding progressively enabled) for a given `PrivacyLevel` — a hardcoded lookup table in `for_level`, not computed from any live measurement.
- **outputs**: `Self` (`for_level`); `usize` (`enabled_count`)
- **called_by**: `UncertaintyConfig::new`, `TradeoffSummary::for_level`, `hsip-telemetry-guard::consent_gate.rs` reasoning about privacy level (indirectly, via `PrivacyLevel` itself rather than this struct directly)

### `PerformanceImpact` / `BatteryImpact`
- **type**: struct / enum
- **file**: `crates/hsip-common/src/quantum_physics/uncertainty.rs`
- **purpose**: Illustrative (hardcoded, not measured) latency/bandwidth/CPU multipliers, delivery delay, and a `BatteryImpact` severity enum per privacy level — for UI display, not derived from any real profiling.
- **outputs**: `Self` (`for_level`); `&'static str` (`BatteryImpact::description`)

### `UncertaintyConfig`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/uncertainty.rs`
- **purpose**: Bundles a `PrivacyLevel` with its derived `PrivacyFeatures`/`PerformanceImpact` and an optional `custom_overrides: HashMap<String,bool>` map letting a caller flip individual named features away from the level's default (`effective_feature` checks the override map first, falls back to the level default). Also computes concrete operational parameters from the level: `padding_size(message_size)` (0 / 16B / pad-to-256B / pad-to-1KB / pad-to-fixed-4KB depending on level), `cover_traffic_interval_ms()`, `batch_delay_ms()`.
- **inputs**: `level: PrivacyLevel` (`new`); `level, overrides: HashMap<String,bool>` (`with_overrides`)
- **outputs**: `Self`; `Option<bool>` (`effective_feature`); `usize` (`padding_size`); `Option<u64>` (`cover_traffic_interval_ms`/`batch_delay_ms`)
- **calls**: `PrivacyFeatures::for_level`, `PerformanceImpact::for_level`

### `EncryptionParams`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/uncertainty.rs`
- **purpose**: Per-level suggested encryption parameters — number of layers (1–3), KDF iteration count (10k–500k), whether to authenticate, whether to encrypt the timestamp itself. Advisory only: nothing in this file (or, as far as this sweep found, elsewhere in the workspace) actually consumes `EncryptionParams` to configure a real cipher — it's a design/UI artifact describing intended tradeoffs, not wired into `hsip-core`'s actual ChaCha20-Poly1305 usage.
- **outputs**: `Self` (`from_level`)

### `TradeoffSummary`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/uncertainty.rs`
- **purpose**: Human-readable summary for a given level — `privacy_score`/`performance_score` (both linear in `level.value()`, deliberately inverse of each other), plus `protected`/`exposed`/`warnings` string lists built by checking each `PrivacyFeatures` flag.
- **outputs**: `Self` (`for_level`)
- **calls**: `PrivacyFeatures::for_level`

### `SliderData`
- **type**: struct
- **file**: `crates/hsip-common/src/quantum_physics/uncertainty.rs`
- **purpose**: UI-facing wrapper: current 0–4 `position`, fixed 5 display labels ("Speed"…"Maximum"), and the current `TradeoffSummary`. `increase`/`decrease` step the position by 1 (clamped to [0,4]) and recompute the summary.
- **inputs**: `level: PrivacyLevel` (`new`)
- **outputs**: `Self`
- **calls**: `TradeoffSummary::for_level`
- **mutates**: `self.position`, `self.summary`

---

## `crates/hsip-telemetry-guard/src/lib.rs`

Crate root. Declares the always-on modules (`audit`, `consent_gate`, `decisions`, `flow_meta`, `guard`, `known_endpoints`, `policy`, `quarantine`) plus three feature-gated ones (`audit_postgres` behind `postgres`, `ntp_sync` behind `ntp-sync`, `geolocation` behind `geolocation`) and wildcard-re-exports all of them, so the whole crate's public surface is reachable as `hsip_telemetry_guard::<Type>` without submodule paths. The module doc's ASCII architecture diagram is accurate to the actual pipeline in `guard.rs::TelemetryGuard::evaluate`: Flow Meta → (Consent Gate, then Policy Engine) → Decision → Audit Trail.

### `TelemetryGuardError`
- **type**: enum
- **file**: `crates/hsip-telemetry-guard/src/lib.rs`
- **purpose**: Crate-wide error type. Most variants (`NoConsent`, `ConsentExpired`, `ConsentRevoked`, `InvalidConsent`, `PolicyViolation`, `UnknownCategory`, `PatternError`) are declared but as of this reading are not actually constructed anywhere in the crate's non-test code — the crate's real runtime error paths mostly return `Result<_, TelemetryGuardError>` via `Ok`/direct construction of `InvalidConsent` (`ConsentGate::grant`) and `NoConsent` (`TelemetryGuard::approve_quarantine`/`reject_quarantine`, repurposed to carry a hex entry-id rather than a domain). `IoError` is populated via the `From<std::io::Error>` impl and by `QuarantineStorage::export_json`'s JSON-serialization failure path.
- **calls**: none
- **called_by**: every module in this crate via the `Result<T> = std::result::Result<T, TelemetryGuardError>` alias

### `Result<T>` (alias)
- **type**: type alias
- **file**: `crates/hsip-telemetry-guard/src/lib.rs`
- **purpose**: Crate-standard `Result` alias so every function signature in this crate reads `Result<X>` instead of the fully-qualified form.
- **called_by**: all public functions in this crate that can fail

---

## `crates/hsip-telemetry-guard/src/audit.rs`

Cryptographic logging of telemetry decisions, integrating `hsip_common::quantum_physics::observer_effect::ObservationLog` (see above) as a secondary receipt log alongside this file's own primary hash chain. This is the crate's own audit trail, independent of (and predates any integration with) `hsip-api`'s separate `audit_log.rs`/`audit_entries` table — the two are not connected; this module operates entirely in-memory (bounded `VecDeque`, `MAX_AUDIT_ENTRIES` = 50,000, oldest entries silently evicted once full).

### `AuditEntry`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/audit.rs`
- **purpose**: One hash-chained record of a `Decision`: `entry_id` (BLAKE3 of timestamp+flow-id-prefix), `entry_hash` (BLAKE3 of entry_id + decision-type byte + destination + `prev_hash`) — note the hash formula does *not* include `timestamp`, `intent`, or `reason` even though they're stored fields, so tampering with those specific fields after the fact would not be caught by `verify()`. `verify()` recomputes and compares `entry_hash`.
- **inputs**: `decision: &Decision, prev_hash: [u8;32]` (`from_decision`)
- **outputs**: `Self`; `bool` (`verify`)
- **calls**: `blake3::Hasher::new`
- **called_by**: `AuditLog::log`

### `AuditLog`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/audit.rs`
- **purpose**: In-memory, capacity-bounded (`VecDeque`, FIFO eviction at `max_entries`) hash chain of `AuditEntry` plus an embedded `ObservationLog` (fixed, hardcoded `binding_secret` `[0xA, 0xD1, 0x17, 0, 0, ...]` — **not derived from any real key material**, effectively a constant known to anyone reading this source, so the embedded `ObservationLog`'s receipts are only tamper-evident against someone who doesn't have the source code, not a cryptographic secret in the normal sense). `log()` appends an entry, recording `genesis_hash` on the very first insert, and separately records an `ObservationLog` observation mapping decision type to an `ObservationType` (Allow/AllowOnce→Read, Block→ConsentCheck, Quarantine→Export, else→MetadataAccess). `verify_chain()` walks all entries checking both per-entry `verify()` and chain linkage. `export_json()`/`export_verification_hash()` support external tamper-checking of an exported copy; `export_counter` increments every export specifically so repeated/selective exports are themselves detectable (an auditor can ask "how many times has this log been exported" and compare).
- **outputs**: `Self` (`new`); `[u8;32]` (`log`, the entry_id); `bool` (`verify_chain`); `Vec<AuditEntry>` (`recent`/`for_destination`/`by_decision`); `String` (`export_json`); `[u8;32]` (`export_verification_hash`)
- **calls**: `AuditEntry::from_decision`, `ObservationLog::{new, record_observation}`
- **called_by**: `hsip-telemetry-guard::guard.rs::TelemetryGuard` (holds one `Arc<AuditLog>`)
- **mutates**: `self.entries`, `self.stats`, `self.genesis_hash`, `self.export_counter`

### `AuditStats`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/audit.rs`
- **purpose**: Running counters (`total_logged`, `allowed`, `blocked`, `quarantined`) plus chain-verification bookkeeping (`chain_valid`, `last_verified`) and hex-encoded `genesis_hash`/`head_hash`/`export_count` for display.
- **called_by**: `AuditLog::{log, verify_chain, export_json}`

---

## `crates/hsip-telemetry-guard/src/audit_postgres.rs`

A PostgreSQL-backed alternative to `audit.rs`'s in-memory `AuditLog`, gated entirely behind the `postgres` cargo feature (via `#[cfg(feature = "postgres")]` on every real method; a stub `PostgresAuditLog` with an `init()` that always returns `Err(...)` exists for the non-feature build so the type name is always available). Implements the identical hash-chain formula as `AuditLog::log`/`AuditEntry::from_decision` (same field ordering into the BLAKE3 hasher) but persists rows to a real `hsip_audit_log` table instead of an in-memory `VecDeque`, and enforces write-once at the database layer via a Postgres trigger rather than only in application logic.

### `PgAuditEntry`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/audit_postgres.rs`
- **purpose**: Row-shape mirror of `AuditEntry` but with `Vec<u8>` fields instead of fixed-size arrays (Postgres `BYTEA` maps naturally to `Vec<u8>` via `tokio_postgres`).
- **called_by**: `PostgresAuditLog::recent`

### `PostgresAuditLog`
- **type**: struct (feature-gated; a zero-field stub exists when the feature is off)
- **file**: `crates/hsip-telemetry-guard/src/audit_postgres.rs`
- **purpose**: `init()` connects via `tokio_postgres` (spawning the connection driver as a background task, `NoTls` — no TLS to the database), then calls `create_schema` which `CREATE TABLE IF NOT EXISTS hsip_audit_log (...)` plus a `prevent_audit_modification()` trigger function that `RAISE EXCEPTION`s on any `UPDATE`/`DELETE` against the table — the actual write-once enforcement lives in the database itself, not merely in this Rust code, so even a bug or a raw `psql` session can't quietly edit/delete a row. `log()` reads the previous chain hash from the DB (`get_latest_hash`, defaulting to 32 zero bytes if the table is empty) and computes the identical hash formula as the in-memory `AuditEntry` before inserting. `verify_chain()` re-derives and checks every row's hash and linkage server-side.
- **inputs**: `connection_string: String` (`new`); `decision: &Decision` (`log`)
- **outputs**: `Self`; `Result<(), String>` (`init`); `Result<Vec<u8>, String>` (`log`, the entry_id); `Result<bool, String>` (`verify_chain`); `Result<Vec<PgAuditEntry>, String>` (`recent`); `Result<usize, String>` (`len`); `Result<String, String>` (`export_json`)
- **calls**: `tokio_postgres::Config::connect`, `client.execute`/`client.query`/`client.query_opt`
- **mutates**: the `hsip_audit_log` Postgres table

---

## `crates/hsip-telemetry-guard/src/consent_gate.rs`

The crate's actual consent-enforcement gate. Module doc explicitly claims integration with all four other quantum-physics modules (Decoherence for auto-expiry, No-Cloning for anti-replay, Entanglement for mutual consent, Uncertainty for privacy-level integration) — in the current code, only **Decoherence** (conceptually — expiry is via `expires_at`/`is_valid()`, not literally `hsip_common::decoherence` types) and **Uncertainty** (`use hsip_common::quantum_physics::uncertainty::PrivacyLevel`, actually imported and used to gate `Maximum`-privacy traffic in `evaluate()`) are genuinely wired in; No-Cloning and Entanglement are *not* imported here — this module's own `consumed_tokens: RwLock<HashMap<[u8;32], DateTime<Utc>>>` is a separate, independently-implemented single-use-token mechanism, not a reuse of `hsip_common::quantum_physics::no_cloning::AntiReplayGuard`. Per the CLAUDE.md note this task was asked to confirm: `ConsentGate` currently has **no** `anti_replay` field — its struct fields are exactly `consents`, `signing_key`, `consumed_tokens`, `privacy_level`; the anti-replay/single-use behavior lives entirely in `consumed_tokens`/`consume()` as documented, confirming that field was indeed removed as dead weight rather than functionality being lost.

### `ConsentScope`
- **type**: enum
- **file**: `crates/hsip-telemetry-guard/src/consent_gate.rs`
- **purpose**: What a `TelemetryConsent` covers: exact `Domain`, wildcard `DomainPattern` (`*.example.com`), `Vendor`, `Intent` (a `TelemetryIntent`), `Application` (substring match on process name), or `Global`. `matches()` implements the actual predicate against a `FlowMeta` + optional vendor string; `scope_id()` gives a deterministic BLAKE3-based key used to index `ConsentGate.consents`.
- **outputs**: `bool` (`matches`); `[u8;32]` (`scope_id`)
- **calls**: `blake3::Hasher::new`
- **called_by**: `TelemetryConsent::new`, `ConsentGate::{grant, revoke, check_consent}`

### `TelemetryConsent`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/consent_gate.rs`
- **purpose**: A signed, time-bounded consent token. `signature_hex` is a keyed-BLAKE3 MAC (`Hasher::new_keyed(signing_key)` over `consent_id`+`expires_at`) concatenated with the raw `consent_id` bytes, hex-encoded — this is a shared-secret MAC scheme, not an asymmetric signature, so `verify()` requires the same `signing_key` used to create it (typically held only by the `ConsentGate` itself). `standard()` = 90-day, `one_time()` = 24h single-use. `renew()` extends `expires_at` from *now*, not from the old expiry.
- **inputs**: `scope: ConsentScope, grantor: [u8;32], duration: Duration, single_use: bool, signing_key: &[u8;32]` (`new`)
- **outputs**: `Self`; `bool` (`is_valid`/`verify`); `Option<Duration>` (`remaining`)
- **calls**: `blake3::Hasher::{new, new_keyed}`
- **called_by**: `ConsentGate::{grant, grant_for_scope}`
- **mutates**: `self.expires_at` (via `renew`)

### `DataMinimization`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/consent_gate.rs`
- **purpose**: Optional per-consent minimization flags (strip identifiers/IP/device-info, round timestamps, cap payload size) attachable via `TelemetryConsent::with_minimization`. Declared/stored but as of this reading not read or enforced anywhere else in the crate — no code path actually strips/rounds/caps based on these flags; it's a data-carrying struct only.
- **outputs**: `Self` (`default`/`strict`)

### `ConsentGate`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/consent_gate.rs`
- **purpose**: The real gate. `grant()` verifies the consent's own MAC before storing it (rejects a forged/tampered consent up front) and indexes by `scope.scope_id()` — note this means granting a *second* consent for the same scope silently overwrites the first (a `HashMap` insert), there's no "already exists" check. `check_consent()` linear-scans all consents for a scope+vendor match, skipping any single-use consent already present in `consumed_tokens`. `consume()` marks a single-use token's ID as consumed (non-single-use tokens are treated as always "successfully consumed" — a no-op true). `evaluate()` is the actual decision entrypoint: at `PrivacyLevel::Maximum`, blocks everything except `CrashReport` intent regardless of consent; otherwise checks consent — single-use consents yield `AllowOnce` (and are immediately consumed) while standard ones yield `Allow` with a TTL equal to remaining consent lifetime; no consent at all yields `Block` (`DecisionReason::NoConsent`).
- **inputs**: `signing_key: [u8;32]` (`new`)
- **outputs**: `Result<[u8;32], TelemetryGuardError>` (`grant`/`grant_for_scope`, the consent ID); `bool` (`revoke`/`consume`); `Option<TelemetryConsent>` (`check_consent`); `Decision` (`evaluate`); `Vec<TelemetryConsent>` (`active_consents`); `usize` (`consent_count`/`cleanup_expired`/`cleanup_consumed`)
- **calls**: `TelemetryConsent::{verify, is_valid, remaining}`, `Decision::{block, allow, allow_once}`
- **called_by**: `hsip-telemetry-guard::guard.rs::TelemetryGuard` (holds one `Arc<ConsentGate>`, calls `evaluate`/`grant_for_scope`/`revoke`/`cleanup_expired`/`cleanup_consumed`)
- **mutates**: `self.consents`, `self.consumed_tokens`, `self.privacy_level`

---

## `crates/hsip-telemetry-guard/src/decisions.rs`

Pure data-model file — no I/O, no locking, just the `Decision`/`DecisionType`/`DecisionReason` types the rest of the crate (`consent_gate.rs`, `policy.rs`, `guard.rs`, `audit.rs`) all produce and consume, plus `DecisionStats` for aggregate counting.

### `DecisionType`
- **type**: enum
- **file**: `crates/hsip-telemetry-guard/src/decisions.rs`
- **purpose**: `Allow | AllowOnce | Block | Quarantine | Pending`. `allows_traffic()` is true only for `Allow`/`AllowOnce` — the single predicate every caller in this crate uses to decide whether to actually let a flow through.
- **outputs**: `bool` (`allows_traffic`); `&'static str` (`emoji`/`action_text`)
- **called_by**: `Decision`, `AuditEntry`, `TelemetryGuard::evaluate_with_quarantine`

### `DecisionReason`
- **type**: enum
- **file**: `crates/hsip-telemetry-guard/src/decisions.rs`
- **purpose**: The "why" behind a decision — carries structured payloads where relevant (`consent_id`, `rule_id`, risk `level`, tracker `vendor`, privacy `level`, match `pattern`). `description()` renders each variant to a human-readable string, used both for display and (via `Decision::display`) for `AuditEntry.reason`.
- **outputs**: `String` (`description`)
- **called_by**: `Decision::{allow, allow_once, block, quarantine, pending}`, `ConsentGate::evaluate`, `PolicyEngine` (all evaluate_* methods)

### `Decision`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/decisions.rs`
- **purpose**: The complete outcome of evaluating one `FlowMeta`: type, primary + contributing reasons, timestamp, optional `ttl` (for caching a decision so it needn't be re-evaluated on every packet), a privacy-safe `DecisionFlowSummary` (never the full `FlowMeta`, so decisions/audit entries derived from them can't leak full flow detail), and a `confidence` score. `is_valid()`/`allows_traffic()` both check `ttl` expiry against `timestamp` — an expired `Allow` decision no longer allows traffic even though `decision_type` itself hasn't changed.
- **inputs**: `flow: &FlowMeta, reason: DecisionReason, ttl: Option<Duration>` (constructors vary per type)
- **outputs**: `Self`; `bool` (`is_valid`/`allows_traffic`); `String` (`display`)
- **calls**: `FlowMeta::{effective_hostname, flow_id}` (via `summarize_flow`)
- **called_by**: `ConsentGate::evaluate`, `PolicyEngine` (all `evaluate_*` methods), `TelemetryGuard::evaluate`, `AuditLog::log`, `AuditEntry::from_decision`

### `DecisionFlowSummary`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/decisions.rs`
- **purpose**: Privacy-safe subset of `FlowMeta` embedded in a `Decision` — 8-byte hex flow-ID prefix, destination hostname, intent, risk level. Deliberately excludes source IP, full flow ID, request path, device fingerprint, etc.
- **called_by**: `Decision::summarize_flow`, `AuditEntry::from_decision`

### `DecisionStats`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/decisions.rs`
- **purpose**: Aggregate counters by decision type plus breakdowns `by_intent`/`by_vendor` (both keyed by `Debug`-formatted strings — same unbounded-cardinality-label caution `hsip-api`'s CLAUDE.md documents for Prometheus metrics applies conceptually here too, though this is an in-process `HashMap`, not an exposed metrics endpoint). `top_blocked_vendors(limit)` sorts descending by block count.
- **outputs**: `f32` (`block_rate`); `Vec<(String,u64)>` (`top_blocked_vendors`)
- **called_by**: `TelemetryGuard::evaluate`/`evaluate_with_quarantine` (records into its own `stats: RwLock<DecisionStats>`)
- **mutates**: `self.total`/`allowed`/etc., `self.by_intent`, `self.by_vendor` (via `record`)

---

## `crates/hsip-telemetry-guard/src/flow_meta.rs`

Pure data-model file describing an observed network flow, with no cryptography — this is the "who/what" half of the pipeline diagram in `lib.rs`'s module doc, feeding both `ConsentGate` and `PolicyEngine`.

### `FlowProtocol`
- **type**: enum
- **file**: `crates/hsip-telemetry-guard/src/flow_meta.rs`
- **purpose**: `Http | Https | Http2 | Http3 | WebSocket | WebSocketSecure | Grpc | Tcp | Udp | Dns | Unknown`. `is_encrypted()` flags the TLS-carried variants.
- **outputs**: `bool` (`is_encrypted`)

### `TelemetryIntent`
- **type**: enum
- **file**: `crates/hsip-telemetry-guard/src/flow_meta.rs`
- **purpose**: The inferred purpose of a flow (`CrashReport`, `UsageAnalytics`, `Diagnostics`, `Advertising`, `FeatureFlags`, `LicenseCheck`, `Heartbeat`, `BehaviorTracking`, `Performance`, `Security`, `Unknown`) — the central classification every other module in this crate branches on. `is_invasive()` flags `Advertising`/`BehaviorTracking`/`UsageAnalytics` as privacy-invasive by default.
- **outputs**: `bool` (`is_invasive`); `&'static str` (`description`)
- **called_by**: `ConsentScope::Intent`, `EndpointEntry`, `PolicyEngine::evaluate_privacy_level`, `DecisionFlowSummary`

### `RiskLevel`
- **type**: enum (`None=0 | Low=1 | Medium=2 | High=3 | Critical=4`, ordered)
- **file**: `crates/hsip-telemetry-guard/src/flow_meta.rs`
- **purpose**: Ordinal risk score used with `>=` comparisons throughout `policy.rs` (e.g. "auto-block anything at or above this level").
- **called_by**: `PolicyConfig::auto_block_risk_level`, `PolicyEngine::{evaluate_known_endpoint, evaluate_auto_rules}`, `QuarantineStorage::get_by_risk`

### `DeviceFingerprint`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/flow_meta.rs`
- **purpose**: Best-effort browser/OS/device metadata. `parse_user_agent` does simple substring matching (not a real UA-parser library — the comment says as much) to fill `os`/`browser`. `fingerprint_hash()` BLAKE3-hashes the available UA/language/encoding/timezone fields into a stable identifier — a genuine (if simple) fingerprinting primitive, notable because it's the one piece of this crate that computes something identifying about the *client*, not the destination.
- **outputs**: `String` (`fingerprint_hash`)
- **calls**: `blake3::Hasher::new`
- **mutates**: `self.os`, `self.browser`, `self.user_agent` (via `parse_user_agent`)

### `FlowMeta`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/flow_meta.rs`
- **purpose**: The full observed-flow record: addresses, hostname/SNI/cert fingerprint, HTTP method/path/content-type/user-agent, process id/name, inferred intent + risk, optional geolocation, device fingerprint. `flow_id` is a BLAKE3 hash of source/dest addr+port plus current nanosecond timestamp (so two otherwise-identical flows still get distinct IDs). `with_intent()` also recomputes `risk_level` from a fixed intent→risk mapping (e.g. `Advertising`/`BehaviorTracking` → `Critical`). `effective_hostname()` prefers SNI, then hostname, then falls back to the raw destination IP string — this is the hostname every consent-scope/policy-rule match in the crate actually compares against. `path_suggests_telemetry()` does substring matching against ~20 known telemetry-ish path fragments (`/collect`, `/beacon`, `/track`, etc.) as a heuristic classifier.
- **inputs**: `source, destination: SocketAddr` (`new`); plus `hostname, method, path: &str` (`from_http`)
- **outputs**: `Self`; `bool` (`path_suggests_telemetry`); `String` (`effective_hostname`); `FlowSummary` (`privacy_summary`)
- **calls**: `blake3::Hasher::new`
- **called_by**: virtually every other module in this crate (`ConsentGate`, `PolicyEngine`, `EndpointDatabase::lookup`, `Decision::*`, `QuarantineStorage::quarantine`)

### `FlowSummary`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/flow_meta.rs`
- **purpose**: A privacy-safe, no-PII rendering of a `FlowMeta` (truncated flow-ID prefix, destination domain, protocol, intent, risk, size) suitable for logging — distinct from, but structurally similar to, `decisions.rs::DecisionFlowSummary`.
- **called_by**: `FlowMeta::privacy_summary`

---

## `crates/hsip-telemetry-guard/src/geolocation.rs`

IP→location lookup via MaxMind's GeoLite2 database, entirely behind the `geolocation` cargo feature. When the feature is off, a stub `GeoLocator` exists whose `new`/`lookup` both always return `Err(...)`, so the type is always nameable but never functional without the feature.

### `GeoLocation`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/geolocation.rs`
- **purpose**: Plain result struct (country/country_code/city/lat/long/timezone/continent, all `Option`). Manually implements `Default` (all `None`) rather than deriving it — functionally identical to a derive, no special logic.
- **called_by**: `FlowMeta.geolocation` field (when the `geolocation` feature is on), `GeoLocator::lookup`

### `GeoLocator`
- **type**: struct (feature-gated; stub otherwise)
- **file**: `crates/hsip-telemetry-guard/src/geolocation.rs`
- **purpose**: Wraps a `maxminddb::Reader` opened from a `.mmdb` file path. `lookup()` decodes a `geoip2::City` record and maps its fields into `GeoLocation`. `lookup_batch()` is a simple per-IP map, silently discarding per-IP lookup errors (`.ok()`) rather than surfacing them.
- **inputs**: `db_path: PathBuf` (`new`); `ip: IpAddr` (`lookup`)
- **outputs**: `Result<Self, String>`; `Result<GeoLocation, String>` (`lookup`); `Vec<(IpAddr, Option<GeoLocation>)>` (`lookup_batch`)
- **calls**: `maxminddb::Reader::open_readfile`, `reader.lookup`

### `download` (submodule)
- **type**: module (functions `instructions`, `default_path`, `database_exists`)
- **file**: `crates/hsip-telemetry-guard/src/geolocation.rs`
- **purpose**: Operator-facing helper text and platform-aware default `.mmdb` path resolution (checks `HSIP_GEOIP_DB` env var first, falls back to a per-OS default path) plus an existence check. Pure convenience/diagnostics, no network calls (does not itself download anything despite the module name — it only prints instructions for a human to do so manually via MaxMind's site).
- **outputs**: `&'static str` (`instructions`); `String` (`default_path`); `bool` (`database_exists`)

---

## `crates/hsip-telemetry-guard/src/guard.rs`

The crate's top-level façade — combines `EndpointDatabase`, `PolicyEngine`, `ConsentGate`, `QuarantineStorage`, and `AuditLog` into one `TelemetryGuard` object implementing the full pipeline described in `lib.rs`'s module doc.

### `TelemetryGuard`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/guard.rs`
- **purpose**: `new()`/`with_config()` construct all five sub-components, using **hardcoded, non-secret placeholder keys** for both the consent-gate signing key and the quarantine encryption key (e.g. `signing_key[0]=0x53 /* 'S' */, [1]=0x49 /* 'I' */, ...` spelling "SIGN"/"ENC" in ASCII, the remaining 28+ bytes left as zero) — this is clearly a development/demo default, not a value safe for any real deployment; `TelemetryGuardBuilder` is the sanctioned way to supply real keys. `evaluate()` is the actual decision pipeline: if disabled, unconditionally allows; otherwise looks up the endpoint database for vendor context, checks the consent gate first (an explicit consent that allows traffic short-circuits straight to logging+return without ever reaching the policy engine), and only falls through to `PolicyEngine::evaluate` if consent didn't allow it — meaning a consent-based `Allow` decision never has policy-engine reasoning layered under it, while a policy-based decision has no possibility of consent-based override once it's reached (block-first design: consent can approve early, but if it doesn't, policy has the final say). `evaluate_with_quarantine()` additionally captures the payload into `QuarantineStorage` when the decision type is `Quarantine`. `approve_quarantine`/`reject_quarantine` are the human-review loop: approving grants a domain-scoped consent and marks the entry `Approved`; rejecting adds a permanent block `PolicyRule` and marks it `Rejected`.
- **inputs**: `flow: &FlowMeta` (`evaluate`); `flow, payload: Option<&[u8]>` (`evaluate_with_quarantine`)
- **outputs**: `Self`; `Decision` (`evaluate`/`evaluate_with_quarantine`); `Result<[u8;32], TelemetryGuardError>` (`grant_consent`/`grant_custom_consent`); `Result<(), TelemetryGuardError>` (`approve_quarantine`/`reject_quarantine`); `DecisionStats`/`QuarantineStats` (stats getters); `CleanupResult` (`cleanup`); `String` (`export_all`, raw JSON)
- **calls**: `EndpointDatabase::lookup`, `ConsentGate::evaluate`, `PolicyEngine::evaluate`, `AuditLog::log`, `QuarantineStorage::{quarantine, get, set_status}`
- **called_by**: not yet integrated into any other crate in this workspace (per CLAUDE.md, `hsip-telemetry-guard` is a supporting, not-actively-integrated crate) — consumed only by this crate's own tests.
- **mutates**: `self.stats` (via inner `RwLock`), plus whatever the delegated sub-component mutates

### `CleanupResult`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/guard.rs`
- **purpose**: Counts of expired consents and consumed-token entries removed by `TelemetryGuard::cleanup()` — a thin aggregation of `ConsentGate::cleanup_expired`/`cleanup_consumed` (the latter hardcoded to a 7-day retention window for consumed single-use tokens).
- **called_by**: `TelemetryGuard::cleanup`

### `TelemetryGuardBuilder`
- **type**: struct (builder pattern)
- **file**: `crates/hsip-telemetry-guard/src/guard.rs`
- **purpose**: The proper way to construct a `TelemetryGuard` with real keys/config/custom endpoints/initial rules instead of `new()`'s placeholder keys — still falls back to the same weak placeholder bytes (`0x53`/`0x45`, this time with only the *first* byte set rather than "SIGN"/"ENC" spelled out) if the caller never calls `.signing_key()`/`.encryption_key()`, so it does not itself force a caller to supply real key material.
- **outputs**: `Self` (`new`); `TelemetryGuard` (`build`)
- **calls**: `EndpointDatabase::{new, add_custom_rule}`, `PolicyEngine::{with_config, add_rule}`

---

## `crates/hsip-telemetry-guard/src/known_endpoints.rs`

A curated, hardcoded database (no external network calls, no dynamic updates) of ~40 known telemetry/tracking/advertising domain patterns across Google, Meta, Microsoft, Apple, Amazon, third-party analytics (Mixpanel, Amplitude, Segment, Hotjar, FullStory, Heap, New Relic), crash reporters (Sentry, Bugsnag, Raygun), ad networks (Criteo, Taboola, Outbrain, The Trade Desk, Xandr, Rubicon, PubMatic), A/B testing (Optimizely, LaunchDarkly, Split), spyware-grade trackers (Comscore, Quantcast), attribution (AppsFlyer, Adjust, Branch), and social widgets (Twitter/X, LinkedIn, TikTok, Snapchat). Each entry pre-classifies intent, risk level, vendor, and whether it's `safe_to_block` (crash reporters and some diagnostics are marked unsafe to block by default, since blocking them may break legitimate functionality).

### `EndpointEntry`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/known_endpoints.rs`
- **purpose**: One database row — domain pattern (wildcard-capable), optional path patterns (declared, stored, but **not actually checked** by `EndpointDatabase::lookup`, which only ever matches on hostname — path narrowing is not enforced despite being modeled), category, intent, risk, vendor, description, `safe_to_block`.
- **called_by**: `EndpointDatabase`, `PolicyEngine::evaluate_known_endpoint`, `TelemetryGuard`

### `EndpointCategory`
- **type**: enum
- **file**: `crates/hsip-telemetry-guard/src/known_endpoints.rs`
- **purpose**: Vendor/category classification (`Google | Meta | Microsoft | Apple | Amazon | AdNetwork | Analytics | CrashReporting | ABTesting | CDN | Social | Spyware | Enterprise | Gaming | IoT | Unknown`). `should_block_by_default()` flags `AdNetwork`/`Spyware`/`Social` — note this method is not actually called from `PolicyEngine`'s auto-block logic, which instead checks `intent`/`risk_level` directly; this predicate appears currently unused outside its own definition.
- **outputs**: `bool` (`should_block_by_default`)

### `EndpointDatabase`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/known_endpoints.rs`
- **purpose**: Holds built-in entries (indexed by base domain, extracted by stripping a leading `*.`/`www.`) plus a separate `custom_rules` list checked *first* on every lookup (so a caller's own rule always overrides a built-in one for the same hostname). `lookup()` tries an exact hostname match, then falls through to a linear suffix-match scan over all indexed base domains — `O(n)` in the number of distinct base domains, not indexed by suffix trie, but the list is small (~40 entries) so this is a non-issue at current scale.
- **outputs**: `Self` (`new`); `Option<EndpointEntry>` (`lookup`); `Vec<EndpointEntry>` (`get_by_category`); `usize` (`len`)
- **calls**: none external (builds `builtin_entries()` from a hardcoded `Vec` literal)
- **called_by**: `PolicyEngine::evaluate` (via `endpoints.lookup`), `TelemetryGuard::{evaluate, lookup_endpoint, endpoints_by_category, add_endpoint}`
- **mutates**: `self.entries` (at construction only, via `load_builtin_entries`), `self.custom_rules` (via `add_custom_rule`)

---

## `crates/hsip-telemetry-guard/src/ntp_sync.rs`

NTP-based clock-offset tracking, gated behind the `ntp-sync` feature. **The actual offset computation is a stub** — `sync_internal`'s comments say plainly that the real millisecond offset extraction from the `rsntp` crate's response is not yet implemented (`let offset_ms = 0i64; // Placeholder - actual sync occurs`), so even with the feature enabled and a successful NTP round-trip, `TimeOffset.offset_ms` is always hardcoded to zero — a real NTP query does happen (`client.synchronize`), but its result is discarded before ever being used, meaning `now()`'s "correction" is currently always a no-op and `is_synced()` will always report true (0 ≤ max_offset_ms) once any sync attempt has completed, regardless of the real clock's actual drift.

### `TimeOffset`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/ntp_sync.rs`
- **purpose**: Measured (nominally) offset in milliseconds plus when it was measured. As noted above, `offset_ms` is currently always 0 in practice.
- **called_by**: `NtpSync`

### `NtpSync`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/ntp_sync.rs`
- **purpose**: `init()` performs an initial sync then (feature-gated) spawns a background task re-syncing every 5 minutes. `now()` applies the tracked offset (currently always zero, per above) to `Utc::now()`, falling back to plain system time if never synced. `is_synced()` checks the tracked offset is within `max_offset_ms` (hardcoded 2000ms / ±2s, described as a "DFF requirement" in comments). Non-feature build's `init()` always returns `Err("NTP sync not enabled...")`.
- **inputs**: `server: String` (`new`)
- **outputs**: `Self`; `Result<(), String>` (`init`); `DateTime<Utc>` (`now`); `Option<TimeOffset>` (`get_offset`); `bool` (`is_synced`); `String` (`status`)
- **calls**: `rsntp::SntpClient::synchronize` (feature-gated)
- **called_by**: not consumed elsewhere in this crate's non-test code as of this reading — a standalone utility awaiting integration.
- **mutates**: `self.offset`

---

## `crates/hsip-telemetry-guard/src/policy.rs`

The rule-based decision engine — the "Policy Engine" box in `lib.rs`'s architecture diagram. Confirmed, per the CLAUDE.md note this task was asked to verify: `PolicyEngine`'s struct fields are exactly `config`, `rules`, `endpoints` — there is **no** `regex_cache` field; every `Regex::new(pattern)` call in `RuleCondition::matches`/`match_wildcard_pattern` recompiles the pattern from scratch on every single evaluation rather than caching a compiled `Regex`, which is a real (if likely minor at current rule-list sizes) performance cost, not just a naming artifact — consistent with a cache field having existed and been removed as genuinely unused (nothing in the current code reads back a cached compiled pattern).

### `PolicyRule`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/policy.rs`
- **purpose**: A user-configurable rule: id/name/description, `enabled` flag, `priority` (rules are kept sorted descending by priority — see `PolicyEngine::add_rule`), a list of `RuleCondition`s that must **all** match (implicit AND across the top-level `conditions` vec), and the `RuleAction` to take.
- **called_by**: `PolicyEngine::{add_rule, evaluate_custom_rules}`

### `RuleCondition`
- **type**: enum
- **file**: `crates/hsip-telemetry-guard/src/policy.rs`
- **purpose**: A small predicate DSL over a flow+optional endpoint: domain (pattern/suffix/exact), path (regex/prefix), intent, min risk level, vendor, category, min request size, protocol, process name, hostname regex, plus logical combinators `Not`/`And`/`Or` for nesting. `matches()` is the evaluator; both `PathRegex` and `HostnameRegex` silently treat an invalid regex pattern as a non-match (`if let Ok(re) = Regex::new(...)` — no error surfaced) rather than rejecting the rule at add-time.
- **outputs**: `bool` (`matches`)
- **calls**: `regex::Regex::new`/`is_match`
- **called_by**: `PolicyEngine::evaluate_custom_rules`

### `RuleAction`
- **type**: enum
- **file**: `crates/hsip-telemetry-guard/src/policy.rs`
- **purpose**: `Allow | AllowOnce | Block | Quarantine | Prompt | Continue`. `Continue` is special-cased in `evaluate_custom_rules`'s match arm as a bare `continue` (skip to the next rule) rather than producing a `Decision` — meaning a rule matching with action `Continue` effectively behaves as if it hadn't matched, deferring to lower-priority rules.
- **called_by**: `PolicyRule`, `PolicyConfig::default_action`

### `PolicyConfig`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/policy.rs`
- **purpose**: Engine-wide tunables: default action when nothing else matches, whether to block-by-default, auto-block trackers/ads, the risk-level threshold for auto-blocking, whether to allow crash reports, a `privacy_level` (0–4, mirrors `hsip_common`'s `PrivacyLevel` as a raw `u8` rather than the enum itself — kept in sync manually by `TelemetryGuard::set_privacy_level`, which writes `level.value()` into this field), and whether to quarantine unknown telemetry. `strict()`/`permissive()` are two preset configurations at opposite ends of the tradeoff.
- **outputs**: `Self` (`default`/`strict`/`permissive`)

### `PolicyEngine`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/policy.rs`
- **purpose**: `evaluate()` runs a fixed 5-stage waterfall, returning on the first stage that produces a decision: (1) custom user rules, sorted by priority, first full-condition-match wins; (2) known-endpoint auto-rules (`evaluate_known_endpoint` — auto-block ads/behavior-tracking, allow crash reports, block by risk threshold, in that checked order, so an endpoint that is *both* advertising *and* otherwise-high-risk is blocked for the advertising reason first); (3) generic auto-rules on the flow itself (risk threshold again, plus telemetry-looking paths); (4) privacy-level rules (`evaluate_privacy_level` — a level-dependent ladder blocking analytics at 4+, diagnostics at 3+, behavior tracking at 2+, and advertising unconditionally at any level); (5) the configured default action. Before any of this, `evaluate()` enriches a *cloned* copy of the input flow with the looked-up endpoint's intent/risk (the caller's original `FlowMeta` is never mutated).
- **inputs**: `endpoints: Arc<EndpointDatabase>` (`new`); `flow: &FlowMeta` (`evaluate`)
- **outputs**: `Self`; `Decision` (`evaluate`); `Vec<PolicyRule>` (`rules`)
- **calls**: `EndpointDatabase::lookup`, `RuleCondition::matches`, `Decision::{allow, allow_once, block, quarantine, pending}`
- **called_by**: `TelemetryGuard::evaluate` (fallback path when consent doesn't allow), `TelemetryGuard::{add_rule, remove_rule, rules, set_policy_config, policy_config}`
- **mutates**: `self.config` (via `set_config`), `self.rules` (via `add_rule`/`remove_rule`/`clear_rules`, keeping the list sorted by priority)

---

## `crates/hsip-telemetry-guard/src/quarantine.rs`

Capture-without-sending storage for telemetry payloads flagged for review — the "🧊 QUARANTINE" outcome in `lib.rs`'s pipeline diagram. Explicitly designed for security-analysis/OWASP-testing use per the file's module doc, not as a production data-retention feature.

### `QuarantinedPayload`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/quarantine.rs`
- **purpose**: One captured entry: entry ID, a privacy-safe `QuarantineFlowMeta`, capture timestamp, a BLAKE3 hash of the *full* payload (for later integrity verification even though only a sample is stored), payload size, an `encrypted_sample` (see `QuarantineStorage::encrypt_sample` — XOR keystream, not a real cipher), why it was quarantined, review status, optional analysis results, and free-form tags.
- **called_by**: `QuarantineStorage`

### `QuarantineFlowMeta` (+ `From<&FlowMeta>`)
- **type**: struct + trait impl
- **file**: `crates/hsip-telemetry-guard/src/quarantine.rs`
- **purpose**: A reduced, privacy-safe projection of `FlowMeta` (destination/port/protocol/method/path/intent/risk/process — no source IP, no cert fingerprint, no device fingerprint) stored alongside a quarantined payload instead of the full flow.
- **calls**: `FlowMeta::{effective_hostname, destination_port}`
- **called_by**: `QuarantineStorage::quarantine`

### `QuarantineReason`
- **type**: enum
- **file**: `crates/hsip-telemetry-guard/src/quarantine.rs`
- **purpose**: Why a payload was captured (`UserRequest | UnknownEndpoint | SuspiciousPattern | HighRisk | LargePayload | PolicyRule | NewEndpoint | Anomaly`) — several variants (`UnknownEndpoint`, `HighRisk`, `LargePayload`, `NewEndpoint`, `Anomaly`) are declared but, as of this reading, not actually constructed anywhere in the crate's non-test code; only `PolicyRule` (from `TelemetryGuard::evaluate_with_quarantine`) is currently produced by real code paths.
- **called_by**: `QuarantinedPayload`, `QuarantineStorage::quarantine`

### `ReviewStatus`
- **type**: enum
- **file**: `crates/hsip-telemetry-guard/src/quarantine.rs`
- **purpose**: `Pending | Approved | Rejected | Flagged | Archived` — the human-review lifecycle state. `Flagged`/`Archived` are declared but not set by any code in this crate outside tests as of this reading (only `Pending` at creation, and `Approved`/`Rejected` via `TelemetryGuard::approve_quarantine`/`reject_quarantine`, are currently reachable).
- **called_by**: `QuarantinedPayload`, `QuarantineStorage::{set_status, get_by_status, get_pending}`

### `PayloadAnalysis` / `DetectedDataType` / `PrivacyConcern`
- **type**: struct / enum / struct
- **file**: `crates/hsip-telemetry-guard/src/quarantine.rs`
- **purpose**: A rich schema for recording what a human (or, eventually, an automated analyzer) found upon inspecting a quarantined payload — detected data types (device IDs, location, PII categories, biometric/financial/health data, etc.) and privacy concerns with severity/remediation. Purely a data model: no code in this file (or elsewhere in the crate) actually performs the detection — `QuarantineStorage::set_analysis` only stores an already-computed `PayloadAnalysis` supplied by the caller.
- **called_by**: `QuarantinedPayload`, `QuarantineStorage::set_analysis`

### `QuarantineStats`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/quarantine.rs`
- **purpose**: Aggregate counts by status/intent/risk (again `Debug`-formatted-string-keyed `HashMap`s), total bytes, oldest/newest timestamps — recomputed from scratch on every mutating operation via `update_stats()` (full `O(n)` rescan, not incrementally maintained).
- **called_by**: `QuarantineStorage::{quarantine, delete, clear, stats}`

### `QuarantineStorage`
- **type**: struct
- **file**: `crates/hsip-telemetry-guard/src/quarantine.rs`
- **purpose**: FIFO-capacity-bounded (`VecDeque` + a `HashMap<[u8;32], usize>` position index) store. `quarantine()` computes a full-payload BLAKE3 hash for integrity but only samples/encrypts up to `MAX_SAMPLE_SIZE` (4096) bytes for storage — larger payloads are truncated before encryption, so `decrypt_sample` can never recover more than the first 4KB of an oversized original payload even though `payload_hash` covers the whole thing. `encrypt_sample`/`decrypt_sample` use a symmetric keyed-BLAKE3-keystream XOR (`key_stream = keyed_BLAKE3(encryption_key, nonce)`, then XOR each byte cyclically against the 32-byte digest) — the same "not a real stream cipher, just keystream reuse via a hash" pattern as `superposition.rs`, and notably the *nonce* used to derive the keystream is the entry's own `entry_id` (public, stored alongside the sample), so the "encryption" provides no confidentiality against anyone who can see both the stored entry and its ID — which is everyone with read access to the struct, since both fields live in the same record. `delete()` fully rebuilds the position index after removal (an `O(n)` operation) since removing from the middle of the `VecDeque` shifts every later position.
- **inputs**: `encryption_key: [u8;32]` (`new`/`with_capacity`); `flow: &FlowMeta, payload: &[u8], reason: QuarantineReason` (`quarantine`)
- **outputs**: `Result<[u8;32], TelemetryGuardError>` (`quarantine`, the entry ID); `Option<Vec<u8>>` (`decrypt_sample`); `Option<QuarantinedPayload>` (`get`); `bool` (`set_status`/`set_analysis`/`add_tag`/`delete`); `Vec<QuarantinedPayload>` (`get_by_status`/`get_pending`/`get_by_destination`/`get_by_risk`); `QuarantineStats` (`stats`); `Result<String>` (`export_json`)
- **calls**: `blake3::{Hasher, hash}`
- **called_by**: `TelemetryGuard::{evaluate_with_quarantine, approve_quarantine, reject_quarantine, pending_quarantine, quarantine_stats}`
- **mutates**: `self.entries`, `self.index`, `self.stats`

---
## `crates/hsip-gateway/src/main.rs`

Entry point for the `hsip-gateway` binary — a standalone MITM-style HTTP/HTTPS forward proxy (distinct from `hsip-api`'s `routes/proxy.rs` traffic monitor). Builds a `proxy::Config` from env vars, writes OS-specific auto-config helper files (a PAC file always, Windows registry enable/disable scripts on Windows), then runs the blocking proxy loop forever.

### `main` (gateway)
- **type**: function
- **file**: `crates/hsip-gateway/src/main.rs`
- **purpose**: Binary entry point. Builds the gateway config from environment variables, prints it, best-effort generates PAC/registry config files (a failure here is only a warning, not fatal), then hands off to `proxy::run_proxy` which blocks forever accepting connections.
- **inputs**: none
- **outputs**: `Result<()>` (only returns on `run_proxy`'s own I/O errors, e.g. failing to bind the listener)
- **calls**: `build_gateway_configuration`, `generate_proxy_config_files`, `proxy::run_proxy`
- **called_by**: Rust runtime (binary entry point)
- **mutates**: stdout/stderr, filesystem (via `generate_proxy_config_files`)

### `generate_proxy_config_files`
- **type**: function
- **file**: `crates/hsip-gateway/src/main.rs`
- **purpose**: Writes a browser-consumable PAC (Proxy Auto-Config) file that routes all non-local traffic through the gateway's `listen_addr` and, on Windows only, `.bat` scripts that flip the OS-level HTTP proxy registry keys on/off — a convenience so a user doesn't have to hand-edit browser/OS proxy settings to try the gateway.
- **inputs**: `listen_addr: &str`
- **outputs**: `Result<()>`
- **calls**: `get_config_directory`, `fs::create_dir_all`, `fs::write`
- **called_by**: `main`
- **mutates**: filesystem — creates `<config_dir>/proxy.pac` and, on Windows, `enable-proxy.bat`/`disable-proxy.bat`

### `get_config_directory`
- **type**: function
- **file**: `crates/hsip-gateway/src/main.rs`
- **purpose**: Resolves the platform-specific directory for gateway config output: `%LOCALAPPDATA%\HSIP\gateway` on Windows, `~/.config/hsip/gateway` on Unix-likes. Does not create the directory itself — the caller does that.
- **inputs**: none
- **outputs**: `Result<PathBuf>`
- **calls**: `std::env::var`
- **called_by**: `generate_proxy_config_files`
- **mutates**: nothing

### `build_gateway_configuration`
- **type**: function
- **file**: `crates/hsip-gateway/src/main.rs`
- **purpose**: Assembles the `proxy::Config` the gateway will run with, by reading listen address and connect-timeout from environment variables.
- **inputs**: none
- **outputs**: `Config`
- **calls**: `read_listen_address`, `read_timeout_configuration`
- **called_by**: `main`
- **mutates**: nothing

### `read_listen_address`
- **type**: function
- **file**: `crates/hsip-gateway/src/main.rs`
- **purpose**: Reads `HSIP_GATEWAY_LISTEN`, defaulting to `127.0.0.1:8080` if unset.
- **inputs**: none
- **outputs**: `String`
- **calls**: `std::env::var`
- **called_by**: `build_gateway_configuration`
- **mutates**: nothing

### `read_timeout_configuration`
- **type**: function
- **file**: `crates/hsip-gateway/src/main.rs`
- **purpose**: Reads `HSIP_GATEWAY_TIMEOUT_MS` (parsed as `u64`), defaulting to `5000` if unset or unparseable.
- **inputs**: none
- **outputs**: `u64`
- **calls**: `std::env::var`
- **called_by**: `build_gateway_configuration`
- **mutates**: nothing

---

## `crates/hsip-gateway/src/classify.rs`

Phase-2.0 (MVP, per the module doc comment) traffic classifier: loads a plaintext tracker-domain blocklist once from `~/.hsip/tracker_blocklist.txt` and decides allow/block for each request the proxy sees. Everything not on the list is allowed — no phishing/malware/ASN heuristics yet.

### `ProtoKind`
- **type**: enum
- **file**: `crates/hsip-gateway/src/classify.rs`
- **purpose**: Distinguishes HTTP vs HTTPS for a classified request, for future protocol-specific handling (not yet used to vary the actual decision logic).
- **called_by**: `RequestInfo`

### `RequestInfo`
- **type**: struct
- **file**: `crates/hsip-gateway/src/classify.rs`
- **purpose**: Minimal view of an inbound request handed to the classifier: host, port, path, protocol kind.
- **called_by**: `classify` callers (intended integration point for `proxy.rs`, though `proxy.rs`'s own `is_blocked_host` currently duplicates the blocking logic independently rather than calling into this module)

### `DecisionKind`
- **type**: enum
- **file**: `crates/hsip-gateway/src/classify.rs`
- **purpose**: `Allow` or `Block` — the coarse outcome of `classify`.
- **called_by**: `Decision`

### `Decision`
- **type**: struct
- **file**: `crates/hsip-gateway/src/classify.rs`
- **purpose**: A classification result: `kind` plus an optional human-readable `reason` (only set for blocks).
- **called_by**: `classify`

### `Decision::allow`
- **type**: function
- **file**: `crates/hsip-gateway/src/classify.rs`
- **purpose**: Constructs an allow decision with no reason.
- **outputs**: `Self`
- **called_by**: `classify`
- **mutates**: nothing

### `Decision::block`
- **type**: function
- **file**: `crates/hsip-gateway/src/classify.rs`
- **purpose**: Constructs a block decision carrying the given reason string.
- **inputs**: `reason: impl Into<String>`
- **outputs**: `Self`
- **called_by**: `classify`
- **mutates**: nothing

### `classify`
- **type**: function
- **file**: `crates/hsip-gateway/src/classify.rs`
- **purpose**: Public entry point: checks the request's host against the loaded tracker blocklist. On a match it records the block in gateway metrics (`metrics::record_tracker_block`) as a side effect of classifying, not just returning a decision — so simply calling `classify` on a tracker host updates the persisted metrics file even if the caller ignores the returned `Decision`.
- **inputs**: `req: &RequestInfo`
- **outputs**: `Decision`
- **calls**: `is_tracker_domain`, `metrics::record_tracker_block`, `Decision::block`, `Decision::allow`
- **called_by**: intended gateway request path (not currently wired into `proxy.rs`, which has its own separate `is_blocked_host` denylist)
- **mutates**: gateway metrics (via `record_tracker_block`)

### `TRACKERS`
- **type**: variable (static, `OnceLock<HashSet<String>>`)
- **file**: `crates/hsip-gateway/src/classify.rs`
- **purpose**: Process-lifetime cache of the lowercased tracker blocklist, loaded once on first use.
- **called_by**: `load_trackers`
- **mutates**: nothing after first initialization

### `tracker_blocklist_path`
- **type**: function
- **file**: `crates/hsip-gateway/src/classify.rs`
- **purpose**: Resolves the blocklist file path: `~/.hsip/tracker_blocklist.txt` (falls back to `.` if home dir can't be resolved).
- **outputs**: `PathBuf`
- **called_by**: `load_trackers`
- **mutates**: nothing

### `load_trackers`
- **type**: function
- **file**: `crates/hsip-gateway/src/classify.rs`
- **purpose**: Reads and parses the blocklist file (one domain per line, `#`-prefixed lines and blanks skipped, lowercased), caching the result in `TRACKERS`. A missing file is treated as an empty list (`unwrap_or_default`), not an error — the gateway still runs, just blocking nothing.
- **outputs**: `&'static HashSet<String>`
- **calls**: `tracker_blocklist_path`, `fs::read_to_string`, `TRACKERS.get_or_init`
- **called_by**: `is_tracker_domain`
- **mutates**: `TRACKERS` (first call only), stderr (logs count loaded)

### `is_tracker_domain`
- **type**: function
- **file**: `crates/hsip-gateway/src/classify.rs`
- **purpose**: Checks whether `host` exactly matches or is a subdomain of any entry in the loaded blocklist (case-insensitive).
- **inputs**: `host: &str`
- **outputs**: `bool`
- **calls**: `load_trackers`
- **called_by**: `classify`
- **mutates**: nothing

---

## `crates/hsip-gateway/src/metrics.rs`

In-process + on-disk counter of trackers the gateway has blocked, read back by `hsip-cli`'s daemon (`daemon/mod.rs::read_blocked_trackers`) and the tray app to color their status indicator.

### `GatewayMetrics`
- **type**: struct
- **file**: `crates/hsip-gateway/src/metrics.rs`
- **purpose**: Serializable snapshot of gateway blocking activity: running `blocked_trackers` count, the last blocked host/reason, and a last-updated millisecond timestamp. Persisted verbatim as JSON to `~/.hsip/gateway_metrics.json`.
- **called_by**: `record_tracker_block`, `daemon/mod.rs::GatewayMetricsFile` (a separate, read-only mirror struct in the CLI crate that only deserializes `blocked_trackers`)

### `METRICS`
- **type**: variable (static, `OnceLock<Mutex<GatewayMetrics>>`)
- **file**: `crates/hsip-gateway/src/metrics.rs`
- **purpose**: Process-wide in-memory metrics state, lazily initialized to `GatewayMetrics::default()`.
- **called_by**: `global`
- **mutates**: itself on first init

### `global`
- **type**: function
- **file**: `crates/hsip-gateway/src/metrics.rs`
- **purpose**: Accessor for the shared `METRICS` mutex, initializing it on first call.
- **outputs**: `&'static Mutex<GatewayMetrics>`
- **calls**: `METRICS.get_or_init`
- **called_by**: `record_tracker_block`
- **mutates**: nothing (beyond first-call init)

### `metrics_path`
- **type**: function
- **file**: `crates/hsip-gateway/src/metrics.rs`
- **purpose**: Resolves the on-disk metrics file path: `~/.hsip/gateway_metrics.json`.
- **outputs**: `PathBuf`
- **called_by**: `record_tracker_block`
- **mutates**: nothing

### `now_ms`
- **type**: function
- **file**: `crates/hsip-gateway/src/metrics.rs`
- **purpose**: Current Unix time in milliseconds, clamped to 0 on a clock error rather than panicking.
- **outputs**: `u64`
- **called_by**: `record_tracker_block`
- **mutates**: nothing

### `record_tracker_block`
- **type**: function
- **file**: `crates/hsip-gateway/src/metrics.rs`
- **purpose**: Increments the in-memory blocked-tracker counter, records the host/reason, and best-effort persists the whole struct to `gateway_metrics.json` — write failures are logged to stderr and swallowed, never propagated, since a metrics-write failure shouldn't affect the gateway's actual blocking behavior. Handles a poisoned mutex by recovering the inner guard rather than panicking (`poisoned.into_inner()`).
- **inputs**: `host: &str`, `reason: &str`
- **outputs**: none
- **calls**: `global`, `metrics_path`, `serde_json::to_string`, `fs::write`
- **called_by**: `classify::classify` (on a tracker-domain block)
- **mutates**: `METRICS` (in-memory), `~/.hsip/gateway_metrics.json` (filesystem)

---

## `crates/hsip-gateway/src/proxy.rs`

The actual blocking forward proxy implementation: a plain `TcpListener` accept loop, one thread per connection, handling both `CONNECT` (HTTPS tunneling) and plain HTTP forwarding. Has its own small, independent host denylist (`is_blocked_host`) rather than calling into `classify.rs`'s tracker-blocklist-file-backed classifier.

### `Config` (gateway proxy)
- **type**: struct
- **file**: `crates/hsip-gateway/src/proxy.rs`
- **purpose**: Runtime configuration for the proxy: the listen address and the upstream connect timeout in milliseconds.
- **called_by**: `main.rs::build_gateway_configuration`, `run_proxy`, `handle_client`, `handle_connect`, `handle_plain_http`

### `Config::default` (gateway proxy)
- **type**: function
- **file**: `crates/hsip-gateway/src/proxy.rs`
- **purpose**: Default config: `127.0.0.1:8080`, 5000ms connect timeout.
- **outputs**: `Self`
- **mutates**: nothing

### `run_proxy`
- **type**: function
- **file**: `crates/hsip-gateway/src/proxy.rs`
- **purpose**: Binds the listener and loops forever accepting connections, spawning a new OS thread per client to run `handle_client`. A per-client error is logged to stderr, not propagated — one bad client can't take down the whole proxy.
- **inputs**: `cfg: Config`
- **outputs**: `Result<()>` (only errors on the initial `bind`)
- **calls**: `TcpListener::bind`, `listener.accept`, `std::thread::spawn`, `handle_client`
- **called_by**: `main::main`
- **mutates**: spawns OS threads; binds a TCP listener

### `is_blocked_host`
- **type**: function
- **file**: `crates/hsip-gateway/src/proxy.rs`
- **purpose**: Hardcoded, in-source denylist (`neverssl.com`, `doubleclick.net`, `google-analytics.com`, `ads.google.com`, `tracking.test`) with exact-match or subdomain matching — a small "starter list so you can see blocks" per its own comment, independent of and not synced with `classify.rs`'s file-backed tracker list.
- **inputs**: `host: &str`
- **outputs**: `bool`
- **called_by**: `handle_client`
- **mutates**: nothing

### `send_blocked_response`
- **type**: function
- **file**: `crates/hsip-gateway/src/proxy.rs`
- **purpose**: Writes a minimal HTTP 403 HTML response back to the client explaining the destination is blocked by the gateway.
- **inputs**: `client: &mut TcpStream`, `host: &str`
- **outputs**: `Result<()>`
- **calls**: `client.write_all`, `client.flush`
- **called_by**: `handle_client`
- **mutates**: writes to the client TCP stream

### `extract_host_for_block`
- **type**: function
- **file**: `crates/hsip-gateway/src/proxy.rs`
- **purpose**: Extracts the target hostname to check against the denylist: for `CONNECT` requests, splits `host:port` from the target; for plain HTTP, reads the `Host:` header from the raw request text.
- **inputs**: `method: &str`, `target: &str`, `req_str: &str`
- **outputs**: `Option<String>`
- **called_by**: `handle_client`
- **mutates**: nothing

### `handle_client`
- **type**: function
- **file**: `crates/hsip-gateway/src/proxy.rs`
- **purpose**: Per-connection driver: sets 2s read/write timeouts, reads the request until the header terminator (`\r\n\r\n`) or a 64KB cap (guards against unbounded memory growth from a client that never sends the terminator), parses the request line, checks the blocklist, then routes to `handle_connect` (HTTPS tunnel) or `handle_plain_http` (plain forwarding) based on the method.
- **inputs**: `client: TcpStream`, `addr: SocketAddr`, `cfg: &Config`
- **outputs**: `Result<()>`
- **calls**: `client.read`, `parse_request_line`, `extract_host_for_block`, `is_blocked_host`, `send_blocked_response`, `handle_connect`, `handle_plain_http`
- **called_by**: `run_proxy` (spawned per accepted connection)
- **mutates**: the client TCP stream

### `parse_request_line`
- **type**: function
- **file**: `crates/hsip-gateway/src/proxy.rs`
- **purpose**: Splits an HTTP request line (`"METHOD target HTTP/x.y"`) into its three whitespace-separated parts.
- **inputs**: `line: &str`
- **outputs**: `Result<(String, String, String)>`
- **called_by**: `handle_client`
- **mutates**: nothing

### `handle_connect`
- **type**: function
- **file**: `crates/hsip-gateway/src/proxy.rs`
- **purpose**: Handles an HTTPS `CONNECT` tunnel: resolves and connects to the target `host:port`, replies `200 Connection Established`, then relays raw bytes bidirectionally — one direction on a spawned thread (`io::copy` client→server), the other on the current thread (server→client). Never inspects TLS content — it's a pure byte tunnel once established.
- **inputs**: `_method: String`, `target: String`, `_version: String`, `_req: Vec<u8>`, `client: TcpStream`, `cfg: &Config`
- **outputs**: `Result<()>`
- **calls**: `resolve_target`, `TcpStream::connect_timeout`, `client.write_all`, `std::thread::spawn`, `std::io::copy`
- **called_by**: `handle_client`
- **mutates**: opens an upstream TCP connection, spawns a relay thread, writes to both streams

### `handle_plain_http`
- **type**: function
- **file**: `crates/hsip-gateway/src/proxy.rs`
- **purpose**: Handles plain (non-TLS) HTTP forwarding: extracts the target host from the `Host:` header, connects to it on port 80, forwards the original raw request bytes verbatim, then copies the upstream's response back to the client.
- **inputs**: `_method: String`, `_target: String`, `_version: String`, `req: Vec<u8>`, `client: TcpStream`, `cfg: &Config`
- **outputs**: `Result<()>`
- **calls**: `extract_host_from_request`, `resolve_target`, `TcpStream::connect_timeout`, `server.write_all`, `std::io::copy`
- **called_by**: `handle_client`
- **mutates**: opens an upstream TCP connection, writes to both streams

### `extract_host_from_request`
- **type**: function
- **file**: `crates/hsip-gateway/src/proxy.rs`
- **purpose**: Pulls the `Host:` header value out of a raw HTTP request buffer.
- **inputs**: `req: &[u8]`
- **outputs**: `Result<String>`
- **called_by**: `handle_plain_http`
- **mutates**: nothing

### `resolve_target`
- **type**: function
- **file**: `crates/hsip-gateway/src/proxy.rs`
- **purpose**: DNS-resolves a `host:port` string to a `SocketAddr`, taking the first result.
- **inputs**: `target: &str`
- **outputs**: `Result<SocketAddr>`
- **calls**: `ToSocketAddrs::to_socket_addrs`
- **called_by**: `handle_connect`, `handle_plain_http`
- **mutates**: nothing (performs DNS I/O)

---

## `crates/hsip-reputation/src/lib.rs`

Crate root for `hsip-reputation` — its only job is declaring the `store` module and re-exporting `DecisionType`/`Event`/`Evidence`/`Store` so downstream crates can `use hsip_reputation::Store;` without reaching into `hsip_reputation::store::Store`. Also carries `#![allow(non_camel_case_types)]` so `store.rs`'s `SCREAMING_SNAKE_CASE`-styled `DecisionType` variants (`TRUSTED`, `VERIFIED_ID`, etc. — chosen to match the wire/JSON vocabulary, not Rust naming convention) don't trigger a lint warning.

### `store` (module declaration)
- **type**: variable (module declaration)
- **file**: `crates/hsip-reputation/src/lib.rs`
- **purpose**: Declares and re-exports the crate's sole substantive module — all real logic lives in `store.rs`.
- **calls**: none
- **called_by**: downstream crates via `hsip_reputation::{DecisionType, Event, Evidence, Store}`
- **mutates**: nothing

---

## `crates/hsip-reputation/src/store.rs`

A local, append-only, hash-chained, Ed25519-signed reputation event log — one flat JSON-lines file, file-locked for concurrent-safe appends (`fs2::FileExt`), no database. Each line's `prev_hash` is the SHA-256 of the previous raw line (self-verifying chain, same shape as `hsip-api`'s `audit_log.rs` BLAKE3 chain but SHA-256 here and file-based rather than DB-based). A subject's reputation "score" is just the sum of signed event weights recorded against it.

### `DecisionType`
- **type**: enum
- **file**: `crates/hsip-reputation/src/store.rs`
- **purpose**: The kind of reputation-affecting decision an event records — positive (`TRUSTED`, `VERIFIED_ID`, `GOOD_BEHAVIOR`), neutral/administrative (`NOTE`, `APPEAL`, `REVERSAL` — always weight 0), and negative (`SPAM`, `MALFORMED`, `TIMEOUT`, `MISBEHAVIOR`, `REPLAY`, `INVALID_SIG`). Serialized as `SCREAMING_SNAKE_CASE` to match the variant names literally.
- **called_by**: `Event`, `Store::weight_for`, `Store::append`

### `Evidence` (reputation)
- **type**: struct
- **file**: `crates/hsip-reputation/src/store.rs`
- **purpose**: A `{kind, value}` pair attached to an event as supporting evidence, e.g. `{"kind": "pcap_hash", "value": "sha256:..."}`.
- **called_by**: `Event`, `Store::append`

### `Event`
- **type**: struct
- **file**: `crates/hsip-reputation/src/store.rs`
- **purpose**: One line of the reputation log: a UUID event id, RFC-ish timestamp, actor/subject peer ids, decision type + severity (0-3) + computed weight, human/machine reason fields, evidence list, optional TTL, the chain-link `prev_hash`, and the event's own Ed25519 `sig` (hex) over its own canonical JSON with `sig` blanked.
- **called_by**: `Store::append`, `Store::verify`, `Store::compute_score`

### `to_canonical_json`
- **type**: function
- **file**: `crates/hsip-reputation/src/store.rs`
- **purpose**: Serializes any `Serialize` value to plain `serde_json::to_string` bytes — the "canonical" form signatures are computed over. Note this is ordinary serde JSON serialization, not RFC 8785 JCS (unlike `hsip-core::canonical`'s decision-attestation canonicalization) — field order follows struct declaration order, which is stable but not a standardized canonical form.
- **inputs**: `v: &T`
- **outputs**: `anyhow::Result<Vec<u8>>`
- **called_by**: `Store::append`, `Store::verify`
- **mutates**: nothing

### `now_rfc3339`
- **type**: function
- **file**: `crates/hsip-reputation/src/store.rs`
- **purpose**: Despite the name, returns the current Unix seconds as a string suffixed `"s"` (e.g. `"1738000000s"`), not an actual RFC 3339 datetime string — clamps to 0 rather than panicking if the system clock reads before the Unix epoch.
- **outputs**: `String`
- **called_by**: `Store::append`
- **mutates**: nothing

### `Store`
- **type**: struct
- **file**: `crates/hsip-reputation/src/store.rs`
- **purpose**: Handle to a single reputation log file, identified by its path. All operations reopen the file per call rather than holding a persistent handle.
- **called_by**: `hsip-cli`/`hsip-net` consumers of the reputation feature

### `Store::open`
- **type**: function
- **file**: `crates/hsip-reputation/src/store.rs`
- **purpose**: Opens (creating if missing, never truncating) the store file at `path`, creating parent directories as needed. On Unix, a newly-created file gets `0o600` permissions via `OpenOptionsExt::mode` — the same "don't leave secrets/sensitive logs world-readable" discipline `hsip-api`'s master-key files follow.
- **inputs**: `path: P where P: AsRef<Path>`
- **outputs**: `anyhow::Result<Self>`
- **calls**: `std::fs::create_dir_all`, `OpenOptions::new().create(true).append(true)[.mode(0o600)]`
- **called_by**: store consumers constructing a `Store`
- **mutates**: filesystem (creates directory/file)

### `Store::last_line_and_hash`
- **type**: function
- **file**: `crates/hsip-reputation/src/store.rs`
- **purpose**: Reads the file line-by-line to find the last non-blank line and computes its SHA-256 hex (or `"0"*64` genesis hash if the file is empty). Marked `#[allow(dead_code)]` — superseded in practice by the inline equivalent logic duplicated inside `append` (which needs the file handle it already holds locked, rather than reopening).
- **outputs**: `anyhow::Result<(Option<String>, String)>`
- **calls**: `BufReader::read_line`, `Sha256::update`/`finalize`
- **called_by**: nothing currently (dead code, kept for potential external/test use)
- **mutates**: nothing

### `Store::weight_for`
- **type**: function
- **file**: `crates/hsip-reputation/src/store.rs`
- **purpose**: Maps a `(decision_type, severity)` pair to an integer reputation weight via per-type severity-indexed tables (severity clamped to 0..3). Positive types produce positive weights, negative types negative, `NOTE`/`APPEAL`/`REVERSAL` are always 0 (administrative, not reputation-affecting).
- **inputs**: `decision_type: &DecisionType`, `severity: u8`
- **outputs**: `i32`
- **called_by**: `Store::append`
- **mutates**: nothing

### `Store::append`
- **type**: function
- **file**: `crates/hsip-reputation/src/store.rs`
- **purpose**: Appends one new signed, chained event. Opens the file, takes an exclusive cross-platform lock (`fs2::FileExt::lock_exclusive` — the mechanism that makes concurrent-process appends safe, called out in the doc comment as "Windows-safe"), re-reads the last line under that lock to compute `prev_hash`, builds the `Event` with a computed `weight`, signs its canonical JSON (with `sig` left empty during signing) with the caller-supplied `signing_key`, writes the final line, `sync_all()`s, then unlocks.
- **inputs**: `signing_key: &SigningKey`, `actor_peer_id: &str`, `subject_peer_id: &str`, `decision_type: DecisionType`, `severity: u8`, `reason_code: &str`, `reason_text: &str`, `evidence: Vec<Evidence>`, `ttl: Option<String>`
- **outputs**: `anyhow::Result<Event>`
- **calls**: `file.lock_exclusive`, `Store::weight_for`, `to_canonical_json`, `signing_key.sign`, `file.write_all`, `file.sync_all`, `fs2::FileExt::unlock`
- **called_by**: reputation-event producers (e.g. `hsip-net`/`hsip-cli` reputation commands)
- **mutates**: appends a line to the store file on disk

### `Store::verify`
- **type**: function
- **file**: `crates/hsip-reputation/src/store.rs`
- **purpose**: Re-reads the whole file, recomputing and checking the `prev_hash` chain link-by-link and verifying each line's Ed25519 signature (against the caller-supplied `verifying_key`, over the line's canonical JSON with `sig` blanked). Returns as soon as any line fails via `anyhow::bail!`/`?` rather than continuing past a break.
- **inputs**: `verifying_key: &VerifyingKey`
- **outputs**: `anyhow::Result<(bool, usize)>` — `(true, count)` on full success; an `Err` (not a `false`) on any broken link or bad signature
- **calls**: `Sha256::update`/`finalize`, `serde_json::from_str`, `verifying_key.verify`
- **called_by**: reputation-log auditors
- **mutates**: nothing

### `Store::compute_score`
- **type**: function
- **file**: `crates/hsip-reputation/src/store.rs`
- **purpose**: Sums the `weight` field of every event in the log whose `subject_peer_id` matches, giving that peer's current reputation score. No caching — recomputes from the whole file on every call.
- **inputs**: `subject_peer_id: &str`
- **outputs**: `anyhow::Result<i32>`
- **calls**: `serde_json::from_str`
- **called_by**: reputation-score lookups
- **mutates**: nothing

---

## `crates/hsip-session/src/lib.rs`

Ephemeral session establishment and AEAD sealing for HSIP's peer-to-peer transport: X25519 (or, when `hsip-core`'s `pqc` feature combines it with ML-KEM-768) ephemeral key exchange → HKDF-SHA256 → ChaCha20-Poly1305, with a direction-split 96-bit nonce (4-byte random prefix + 8-byte monotonic counter) enforcing per-direction replay protection.

### `HybridSharedSecret`
- **type**: struct
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: Wraps the 32-byte HKDF output of `(X25519_shared || ML-KEM-768_shared)` produced by a caller's own hybrid PQC handshake (via `hsip-core::pqc` encapsulate/decapsulate helpers, not this crate) before it's fed into `Session::from_hybrid_handshake`. Zeroizes its bytes on drop.
- **called_by**: `Session::from_hybrid_handshake`, external PQC handshake callers

### `DEFAULT_INFO`
- **type**: variable (constant)
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: HKDF domain-separation info string (`b"HSIP v1 session key"`) used when a caller derives a session without supplying their own `PeerLabel`.
- **called_by**: `Session::from_shared_secret`

### `PeerLabel`
- **type**: struct
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: Optional caller-supplied bytes (e.g. `b"CONSENTv1|peerA->peerB"`) mixed into HKDF as the `info` parameter, binding the derived session key to a specific protocol/peer-pair context so two different logical exchanges over the same raw shared secret can't be confused.
- **called_by**: `Session::from_shared_secret` and everything that forwards a label to it

### `SessionError`
- **type**: enum
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: `Consumed` (an `Ephemeral`'s one-shot secret was already used) or `KdfExpand` (HKDF expansion failed). Implements `Display`/`std::error::Error`.
- **called_by**: `Ephemeral::into_shared`, `Session::from_shared_secret` and its callers

### `SealError`
- **type**: enum
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: `Encrypt` — the single failure mode of `Session::seal` (AEAD encryption failed).
- **called_by**: `Session::seal`

### `OpenError`
- **type**: enum
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: Failure modes of `Session::open`: `Truncated` (shorter than nonce+tag), `BadNonce` (rx prefix mismatch or malformed nonce bytes), `Replayed` (stale/non-monotonic counter), `AuthFailed` (AEAD tag verification failed).
- **called_by**: `Session::open`

### `Ephemeral`
- **type**: struct
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: A one-shot X25519 keypair — `secret` is `Option<EphemeralSecret>` specifically so it can be `take()`n and consumed exactly once, making key reuse a compile-time-adjacent, runtime-checked impossibility (`SessionError::Consumed` on a second attempt) rather than a silent bug.
- **called_by**: `Session::from_handshake`, `demo_pair`

### `Ephemeral::generate`
- **type**: function
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: Generates a fresh ephemeral X25519 keypair from `OsRng`.
- **outputs**: `Self`
- **calls**: `EphemeralSecret::random_from_rng`
- **called_by**: `demo_pair`, session-handshake initiators
- **mutates**: nothing (beyond consuming OS entropy)

### `Ephemeral::public`
- **type**: function
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: Returns the (copyable) public key half, without consuming the ephemeral.
- **outputs**: `PublicKey`
- **called_by**: `demo_pair`
- **mutates**: nothing

### `Ephemeral::into_shared`
- **type**: function
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: Consumes `self`'s secret to perform X25519 Diffie-Hellman against `their_pub`, producing the raw 32-byte shared secret. Errors instead of panicking if called twice on the same value (the secret was already `take()`n).
- **inputs**: `self` (by value), `their_pub: &PublicKey`
- **outputs**: `Result<[u8; 32], SessionError>`
- **calls**: `EphemeralSecret::diffie_hellman`
- **called_by**: `Session::from_handshake`, `demo_pair`, test rekey flow
- **mutates**: consumes/drops the ephemeral secret

### `Session`
- **type**: struct
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: A live AEAD session: the derived `Key`/`ChaCha20Poly1305` cipher, a random `tx_prefix` + monotonic `tx_counter` for outgoing nonces, and a learned-on-first-receive `rx_prefix` + last-seen `rx_counter` for incoming replay detection. `Drop` zeroizes the key and counters/prefixes rather than relying on the field types alone.
- **called_by**: `demo_pair`, all handshake/seal/open call sites

### `Session::from_shared_secret`
- **type**: function
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: Core session derivation: HKDF-expands the raw 32-byte shared secret (with `label` or, absent one, `DEFAULT_INFO` as the HKDF info) into a 32-byte AEAD key, builds the cipher, zeroizes the intermediate key-material buffer, and randomizes a fresh `tx_prefix`.
- **inputs**: `shared: [u8; 32]`, `label: Option<&PeerLabel>`
- **outputs**: `Result<Self, SessionError>`
- **calls**: `Hkdf::<Sha256>::new`/`expand`, `ChaCha20Poly1305::new`, `OsRng.fill_bytes`
- **called_by**: `Session::from_handshake`, `Session::from_hybrid_handshake`, `Session::rekey_from_shared`
- **mutates**: nothing external (zeroizes a local buffer)

### `Session::from_handshake`
- **type**: function
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: Convenience wrapper: consumes `our_eph` against `their_pub` to get the shared secret, then derives a `Session` from it.
- **inputs**: `our_eph: Ephemeral`, `their_pub: &PublicKey`, `label: Option<&PeerLabel>`
- **outputs**: `Result<Self, SessionError>`
- **calls**: `Ephemeral::into_shared`, `Session::from_shared_secret`
- **called_by**: `demo_pair`
- **mutates**: consumes the ephemeral

### `Session::from_hybrid_handshake`
- **type**: function
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: Phase-2 PQC entry point: derives a `Session` from a caller-provided `HybridSharedSecret` (already combining X25519 + ML-KEM-768 outputs via HKDF, performed elsewhere in `hsip-core`). The resulting session's AEAD/nonce mechanics are identical to a classical session — the only security difference is how the input shared-secret bytes were produced.
- **inputs**: `hybrid_secret: HybridSharedSecret`, `label: Option<&PeerLabel>`
- **outputs**: `Result<Self, SessionError>`
- **calls**: `Session::from_shared_secret`
- **called_by**: PQC-hybrid handshake integrators
- **mutates**: nothing

### `Session::rekey_from_shared`
- **type**: function
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: Replaces `self` entirely with a freshly derived session from a new shared secret — resets both tx and rx state (new prefixes, counters back to 0), for periodic session rekeying after exchanging new ephemerals.
- **inputs**: `&mut self`, `new_shared: [u8; 32]`, `label: Option<&PeerLabel>`
- **outputs**: `Result<(), SessionError>`
- **calls**: `Session::from_shared_secret`
- **called_by**: session-rekey flows, tests
- **mutates**: `self` (full replacement)

### `Session::seal`
- **type**: function
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: AEAD-encrypts `plaintext` with `aad` as associated data, building the 12-byte nonce from `tx_prefix || tx_counter (big-endian)`, then increments `tx_counter` (wrapping, not checked — an extremely long-lived session could in principle wrap the counter back to 0, silently reusing a nonce; not guarded against here). Returns `nonce || ciphertext+tag`.
- **inputs**: `&mut self`, `aad: &[u8]`, `plaintext: &[u8]`
- **outputs**: `Result<Vec<u8>, SealError>`
- **calls**: `ChaCha20Poly1305::encrypt`
- **called_by**: session senders (tests, transport integrators)
- **mutates**: `self.tx_counter`

### `Session::open`
- **type**: function
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: AEAD-decrypts a `nonce || ciphertext+tag` frame. On the very first call, learns and stores the peer's `rx_prefix` from the incoming nonce; on every subsequent call, requires the prefix to match exactly (constant-time compare via `subtle::ConstantTimeEq`) and the embedded counter to be `>= rx_counter` (monotonic — replays and reordering-below-last-seen are rejected as `OpenError::Replayed`), then updates `rx_counter = rx_ctr + 1` **before** attempting decryption — so even a call that ultimately fails `AuthFailed` still advances the replay counter for that value, meaning a legitimately-late but valid frame can no longer be replayed after a forged one at the same counter was rejected.
- **inputs**: `&mut self`, `aad: &[u8]`, `nonce_and_ct: &[u8]`
- **outputs**: `Result<Vec<u8>, OpenError>`
- **calls**: `ChaCha20Poly1305::decrypt`
- **called_by**: session receivers (tests, transport integrators)
- **mutates**: `self.rx_prefix` (first call only), `self.rx_counter`

### `demo_pair`
- **type**: function
- **file**: `crates/hsip-session/src/lib.rs`
- **purpose**: Test/demo helper that generates two ephemeral keypairs, performs the handshake both directions, and returns both parties' public keys and live `Session`s — used to set up a working client/server pair in one call for tests.
- **inputs**: `label: Option<&PeerLabel>`
- **outputs**: `Result<(PublicKey, PublicKey, Session, Session), SessionError>`
- **calls**: `Ephemeral::generate`, `Session::from_handshake`
- **called_by**: this file's own `#[cfg(test)]` module
- **mutates**: nothing external

---

## `crates/hsip-session/src/persistence.rs`

Small filesystem persistence helpers shared by `hsip-cli`/`hsip-net` for storing resume tokens, last-seen data, and other small session-related artifacts under a common state directory — separate from `rate_limit_persistence.rs` in `hsip-api` (that's DB-table snapshotting; this is plain files). Every write is atomic (write to `.tmp`, `fsync` on Unix, then `rename`).

### `state_dir`
- **type**: function
- **file**: `crates/hsip-session/src/persistence.rs`
- **purpose**: Resolves the base directory for all state files: `$HSIP_HOME/state/` if `HSIP_HOME` is set, else `~/.hsip/state/` (falling back to `.` if the home directory can't be resolved).
- **outputs**: `PathBuf`
- **calls**: `std::env::var`, `dirs::home_dir`
- **called_by**: `path_for`, `ensure_dir`
- **mutates**: nothing

### `path_for`
- **type**: function
- **file**: `crates/hsip-session/src/persistence.rs`
- **purpose**: Joins a logical file name onto `state_dir()`.
- **inputs**: `name: &str`
- **outputs**: `PathBuf`
- **calls**: `state_dir`
- **called_by**: `read_json`, `remove`, `load_blob`
- **mutates**: nothing

### `ensure_dir`
- **type**: function
- **file**: `crates/hsip-session/src/persistence.rs`
- **purpose**: Idempotently creates the state directory if it doesn't exist yet, returning its path.
- **outputs**: `io::Result<PathBuf>`
- **calls**: `state_dir`, `fs::create_dir_all`
- **called_by**: `write_json`, `save_blob`, this file's own tests
- **mutates**: filesystem (creates directory)

### `write_json`
- **type**: function
- **file**: `crates/hsip-session/src/persistence.rs`
- **purpose**: Atomically writes pretty-printed JSON to `state_dir()/name`: writes to `name.tmp`, best-effort `fsync`s the file and (on Unix) the containing directory for durability, then `rename`s into place — so a crash mid-write never leaves a half-written file at the real path.
- **inputs**: `name: &str`, `value: &T where T: Serialize`
- **outputs**: `io::Result<()>`
- **calls**: `fs::File::create`, `serde_json::to_string_pretty`, `f.sync_all`, `fs::rename`
- **called_by**: session-state persisters (`hsip-cli` session save commands), this file's own tests
- **mutates**: filesystem — writes `<name>.tmp` then renames to `<name>`

### `read_json`
- **type**: function
- **file**: `crates/hsip-session/src/persistence.rs`
- **purpose**: Reads and deserializes JSON from `state_dir()/name`, returning `None` (not an error) on any failure — missing file, read error, or parse error are all treated identically as "nothing to load."
- **inputs**: `name: &str`
- **outputs**: `Option<T> where T: DeserializeOwned`
- **calls**: `fs::File::open`, `serde_json::from_str`
- **called_by**: session-state loaders, this file's own tests
- **mutates**: nothing

### `remove`
- **type**: function
- **file**: `crates/hsip-session/src/persistence.rs`
- **purpose**: Best-effort deletes `state_dir()/name` if it exists; a no-op (not an error) if it doesn't.
- **inputs**: `name: &str`
- **outputs**: `io::Result<()>`
- **calls**: `path.exists`, `fs::remove_file`
- **called_by**: session-state cleanup, this file's own tests
- **mutates**: filesystem (deletes a file)

### `save_blob`
- **type**: function
- **file**: `crates/hsip-session/src/persistence.rs`
- **purpose**: Same atomic tmp-write-then-rename pattern as `write_json`, but for raw bytes rather than a serializable value — used by `hsip-cli`'s `SessionSave`/`SessionLoad` commands.
- **inputs**: `name: &str`, `data: &[u8]`
- **outputs**: `io::Result<()>`
- **calls**: `fs::File::create`, `f.sync_all`, `fs::rename`
- **called_by**: `hsip-cli` session save commands, this file's own tests
- **mutates**: filesystem — writes `<name>.tmp` then renames to `<name>`

### `load_blob`
- **type**: function
- **file**: `crates/hsip-session/src/persistence.rs`
- **purpose**: Reads raw bytes from `state_dir()/name`.
- **inputs**: `name: &str`
- **outputs**: `io::Result<Vec<u8>>`
- **calls**: `fs::read`
- **called_by**: `hsip-cli` session load commands, this file's own tests
- **mutates**: nothing

---

## `crates/hsip-regenerative/src/lib.rs`

Distributed identity recovery via Shamir Secret Sharing (using the `ssss` crate, which the code's own comment notes avoids a polynomial-bias vulnerability some naive implementations have): an identity secret is split into `total_shards` pieces such that any `threshold` of them reconstruct it, while any single shard reveals nothing. Phase-1 defaults are a 3-of-5 scheme with a 1-year expiration and a fixed local/trusted-contact/paper storage plan.

### `DEFAULT_THRESHOLD`
- **type**: variable (constant)
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Phase-1 default minimum shards needed to recover: `3`.
- **called_by**: `ShardingConfig::default`

### `DEFAULT_TOTAL_SHARDS`
- **type**: variable (constant)
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Phase-1 default total shards created: `5`.
- **called_by**: `ShardingConfig::default`

### `MAX_SECRET_SIZE`
- **type**: variable (constant)
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Maximum shardable secret size in bytes: `32` (a 256-bit key, e.g. an Ed25519/X25519 seed).
- **called_by**: `IdentityRegenerator::shard_identity`

### `RegenerativeError`
- **type**: enum
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: All failure modes for sharding/recovery: bad threshold, too few shards, secret too large, shard verification/expiration/format/index failures, and a generic `RecoveryFailed(String)` wrapping the underlying `ssss` error's `Debug` output.
- **called_by**: every fallible method in this crate

### `ShardStorageType`
- **type**: enum
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Where a shard is meant to be stored — `LocalDevice`, `TrustedContact`, `CloudBackup`, `HardwareToken`, `PaperBackup` — purely descriptive metadata, not enforced by this crate.
- **called_by**: `ShardMetadata`, `ShardingConfig`

### `ShardMetadata`
- **type**: struct
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Non-sensitive, safely-shareable metadata about a shard: its id (BLAKE3 of the share data + identity fingerprint), index, creation/expiry timestamps, storage type, an integrity `verification_hash`, a human label, and the 8-byte `identity_fingerprint` (not the full identity) so shards from different identities can't be confused. Deliberately reveals nothing about the secret itself.
- **called_by**: `IdentityShard`, `RecoveryProgress`

### `IdentityShard`
- **type**: struct
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: One recoverable piece of a sharded identity secret: public `metadata` plus the actual `shard_data` bytes (Base62-encoded `ssss` share, stored as its UTF-8 bytes) and an optional passphrase salt. Derives `Zeroize`/`ZeroizeOnDrop` — `metadata` and `passphrase_salt` are `#[zeroize(skip)]`'d since they're non-secret, only `shard_data` needs zeroizing.
- **called_by**: `IdentityRegenerator::shard_identity`/`recover_identity`, `RecoveryProgress::add_shard`

### `IdentityShard::from_share_data`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Builds one `IdentityShard` from a raw `ssss` share plus its index/fingerprint/storage assignment: computes `shard_id` as BLAKE3(share_data || fingerprint) and `verification_hash` as BLAKE3(shard_id || share_data), both used later to detect tampering.
- **inputs**: `share_data: Vec<u8>`, `index: u8`, `identity_fingerprint: [u8; 8]`, `storage_type: ShardStorageType`, `label: String`, `expires_at: Option<DateTime<Utc>>`
- **outputs**: `Self`
- **calls**: `Hasher::new`/`update`/`finalize` (BLAKE3)
- **called_by**: `IdentityRegenerator::shard_identity`
- **mutates**: nothing

### `IdentityShard::verify`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Recomputes BLAKE3(shard_id || shard_data) and checks it matches the stored `verification_hash` — detects a corrupted or tampered shard before it's used in recovery.
- **outputs**: `bool`
- **calls**: `Hasher::new`/`update`/`finalize`
- **called_by**: `IdentityRegenerator::recover_identity`, tests
- **mutates**: nothing

### `IdentityShard::is_expired`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Checks whether `metadata.expires_at` (if set) is in the past.
- **outputs**: `bool`
- **called_by**: `IdentityRegenerator::recover_identity`, tests
- **mutates**: nothing

### `IdentityShard::shard_bytes`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Accessor for the raw shard data bytes.
- **outputs**: `&[u8]`
- **called_by**: `IdentityRegenerator::recover_identity`
- **mutates**: nothing

### `IdentityShard::index`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Accessor for the shard's 1-indexed position in the sharing scheme.
- **outputs**: `u8`
- **called_by**: external consumers of `IdentityShard`
- **mutates**: nothing

### `ShardingConfig`
- **type**: struct
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Parameters for a sharding operation: `threshold`, `total_shards`, optional `expiration` duration, and a `storage_plan` (ordered list of `(ShardStorageType, label)` assigned to each shard index in order — index `i` beyond the plan's length falls back to `LocalDevice`/`"Shard {i+1}"`, see `shard_identity`).
- **called_by**: `IdentityRegenerator`

### `ShardingConfig::default`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Phase-1 default: 3-of-5, 1-year expiration, a 5-entry storage plan (2 local devices, 2 trusted contacts, 1 paper backup).
- **outputs**: `Self`
- **called_by**: `IdentityRegenerator::new`
- **mutates**: nothing

### `ShardingConfig::validate`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Rejects a config whose `threshold` is below 2 or above `total_shards`.
- **outputs**: `Result<(), RegenerativeError>`
- **called_by**: `IdentityRegenerator::shard_identity`, tests
- **mutates**: nothing

### `IdentityRegenerator`
- **type**: struct
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: The sharding/recovery engine, parameterized by a `ShardingConfig`.
- **called_by**: identity-backup/recovery flows in `hsip-cli`/`hsip-net` (or future callers)

### `IdentityRegenerator::new`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Constructs with the Phase-1 default config.
- **outputs**: `Self`
- **calls**: `Self::with_config`, `ShardingConfig::default`
- **called_by**: callers wanting default 3-of-5 behavior, tests
- **mutates**: nothing

### `IdentityRegenerator::with_config`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Constructs with a caller-supplied config (custom threshold/total/expiration/storage plan).
- **inputs**: `config: ShardingConfig`
- **outputs**: `Self`
- **called_by**: `IdentityRegenerator::new`, custom-config callers, tests
- **mutates**: nothing

### `IdentityRegenerator::shard_identity`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Splits `secret` into `total_shards` `IdentityShard`s. Validates the secret isn't larger than `MAX_SECRET_SIZE` and the config itself, computes an 8-byte BLAKE3-derived identity fingerprint (shared by all resulting shards so they can be recognized as belonging to the same identity), then calls `ssss::gen_shares` and wraps each resulting Base62-string share (stored as UTF-8 bytes) in an `IdentityShard` via `from_share_data`, assigning storage type/label from `storage_plan` by index.
- **inputs**: `&self`, `secret: &[u8]`
- **outputs**: `Result<Vec<IdentityShard>, RegenerativeError>`
- **calls**: `Hasher` (BLAKE3), `ShardingConfig::validate`, `ssss::SsssConfig::builder`, `ssss::gen_shares`, `IdentityShard::from_share_data`
- **called_by**: identity backup flows, tests
- **mutates**: nothing (pure computation over its inputs)

### `IdentityRegenerator::recover_identity`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Reconstructs the original secret from a slice of shards: requires at least `threshold` shards, verifies each one's integrity and non-expiry first (failing fast on the first bad/expired shard rather than attempting recovery with tainted input), converts each shard's bytes back to its Base62 string form, then calls `ssss::unlock`.
- **inputs**: `&self`, `shards: &[IdentityShard]`
- **outputs**: `Result<Vec<u8>, RegenerativeError>`
- **calls**: `IdentityShard::verify`, `IdentityShard::is_expired`, `String::from_utf8`, `ssss::unlock`
- **called_by**: identity recovery flows, tests
- **mutates**: nothing

### `IdentityRegenerator::threshold`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Accessor for the configured recovery threshold.
- **outputs**: `u8`
- **called_by**: external consumers
- **mutates**: nothing

### `IdentityRegenerator::total_shards`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Accessor for the configured total shard count.
- **outputs**: `u8`
- **called_by**: external consumers
- **mutates**: nothing

### `RecoveryProgress`
- **type**: struct
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Tracks an in-progress, time-boxed recovery attempt for one identity fingerprint: which shard indices have been collected so far (as their `ShardMetadata`, not the sensitive share bytes), and when the attempt itself expires.
- **called_by**: recovery-flow orchestration (e.g. a wizard collecting shards from multiple contacts)

### `RecoveryProgress::start`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Begins tracking a new recovery attempt for `fingerprint`, expiring after `timeout` from now.
- **inputs**: `fingerprint: [u8; 8]`, `threshold: u8`, `timeout: Duration`
- **outputs**: `Self`
- **called_by**: recovery-flow initiators, tests
- **mutates**: nothing (constructs a new value)

### `RecoveryProgress::add_shard`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Records a collected shard's metadata if (and only if) its `identity_fingerprint` matches the one this recovery attempt is for — silently rejects (returns `false`) a shard belonging to a different identity, guarding against accidentally mixing shards from unrelated identities.
- **inputs**: `&mut self`, `shard: &IdentityShard`
- **outputs**: `bool`
- **called_by**: recovery-flow orchestration, tests
- **mutates**: `self.collected`

### `RecoveryProgress::can_recover`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Whether enough distinct shard indices have been collected to meet `threshold`.
- **outputs**: `bool`
- **called_by**: recovery-flow orchestration, tests
- **mutates**: nothing

### `RecoveryProgress::is_expired`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Whether the recovery attempt's own timeout has passed.
- **outputs**: `bool`
- **called_by**: recovery-flow orchestration
- **mutates**: nothing

### `RecoveryProgress::progress_percent`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: `collected / threshold` as a percentage, capped at 100.
- **outputs**: `u8`
- **called_by**: UI/progress-reporting callers, tests
- **mutates**: nothing

### `RecoveryProgress::remaining`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: How many more distinct shards are needed to reach `threshold` (saturating at 0).
- **outputs**: `usize`
- **called_by**: UI/progress-reporting callers, tests
- **mutates**: nothing

### `ShardRotation`
- **type**: struct
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Tracks when a set of shards was last (re)generated and whether it's due for rotation, for the "future premium" periodic-rotation feature the module doc mentions. Not wired to any actual re-sharding logic in this crate — it only tracks timing, the caller is responsible for actually calling `shard_identity` again.
- **called_by**: future shard-rotation schedulers

### `ShardRotation::new`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Starts a rotation tracker at generation 0, "created now".
- **inputs**: `rotation_interval: Duration`
- **outputs**: `Self`
- **called_by**: rotation schedulers, tests
- **mutates**: nothing

### `ShardRotation::needs_rotation`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Whether `created_at + rotation_interval` has passed.
- **outputs**: `bool`
- **called_by**: rotation schedulers, tests
- **mutates**: nothing

### `ShardRotation::rotated`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Marks a rotation as having just occurred: increments `generation`, resets `created_at` to now.
- **inputs**: `&mut self`
- **outputs**: none
- **called_by**: rotation schedulers, tests
- **mutates**: `self.generation`, `self.created_at`

### `ShardRotation::time_until_rotation`
- **type**: function
- **file**: `crates/hsip-regenerative/src/lib.rs`
- **purpose**: Time remaining until rotation is due, or `None` if already overdue.
- **outputs**: `Option<Duration>`
- **called_by**: rotation schedulers
- **mutates**: nothing

---

## `crates/hsip-integration-sdk/src/lib.rs`

A pure-types-and-traits crate — "stable extension points for third-party HSIP integrations," per its own module doc. Defines three extension traits (`PolicyHook`, `AuditSink`, `CapabilityProvider`) plus their supporting data types and a trivial no-op implementation of each. Nothing in this crate does any I/O or holds any state itself; it's an interface contract other crates/binaries are meant to implement. The doc comments are explicit that implementations must stay protocol-observable only — no platform-specific identity, no unverifiable claims, and must preserve HSIP's hash-chained audit trail properties (litigation/evidence-readiness is called out directly).

### `PolicyDecision`
- **type**: enum
- **file**: `crates/hsip-integration-sdk/src/lib.rs`
- **purpose**: The four possible outcomes a `PolicyHook` can force for a consent request: `AutoDeny`, `QueueForReview`, `AutoAccept`, `SilentReject` (malformed/suspicious traffic, deliberately unlogged).
- **called_by**: `PolicyHook::evaluate`

### `PolicyReason`
- **type**: enum
- **file**: `crates/hsip-integration-sdk/src/lib.rs`
- **purpose**: Machine-readable reason codes logged alongside a `PolicyDecision` — includes a `CustomPolicyRule { rule_id, reason }` variant for hook-specific reasons and `TooManyAttempts { count }` carrying a count.
- **called_by**: `PolicyHook::evaluate`

### `ConsentRequestContext`
- **type**: struct
- **file**: `crates/hsip-integration-sdk/src/lib.rs`
- **purpose**: The protocol-observable-only view of a consent request handed to a `PolicyHook`: verified peer id, claimed (unverified) purpose string, timestamp, and several booleans/counters (`unknown_peer`, `denied_before`, `failed_attempts`, `rate_limited`, `suspicious`) all derived from cryptographic verification or protocol history — explicitly not from unverifiable claims.
- **called_by**: `PolicyHook::evaluate`

### `PolicyHook`
- **type**: trait
- **file**: `crates/hsip-integration-sdk/src/lib.rs`
- **purpose**: Extension point letting an integration inject custom consent-policy logic. `evaluate` returns `Some((decision, reason))` to override HSIP's default policy for this request, or `None` to fall through to default behavior — an opt-out-by-omission design so a hook only needs to handle the cases it cares about.
- **inputs**: `&self`, `ctx: &ConsentRequestContext`
- **outputs**: `Option<(PolicyDecision, PolicyReason)>`
- **called_by**: HSIP's consent evaluation path (integration point, not called anywhere inside this crate itself beyond `NoOpPolicyHook`/tests)

### `AuditEvent`
- **type**: struct
- **file**: `crates/hsip-integration-sdk/src/lib.rs`
- **purpose**: A hash-chained, Ed25519-signed audit record shape for `AuditSink` implementations to log — structurally similar to `hsip-reputation::store::Event` (actor/subject peer ids, decision type, severity, reason codes, evidence, `prev_hash` chain link, hex signature) but this is a standalone SDK type, not literally shared code with that crate.
- **called_by**: `AuditSink` trait methods

### `Evidence` (integration-sdk)
- **type**: struct
- **file**: `crates/hsip-integration-sdk/src/lib.rs`
- **purpose**: A `{kind, value}` evidence pair attached to an `AuditEvent`, e.g. `{"kind": "signature_hash", "value": "sha256:..."}`. Distinct type from (but structurally identical to) `hsip-reputation::store::Evidence`.
- **called_by**: `AuditEvent`

### `AuditSink`
- **type**: trait
- **file**: `crates/hsip-integration-sdk/src/lib.rs`
- **purpose**: Extension point for exporting audit events to external storage while preserving tamper-evidence: `log_event` (must preserve hash chain/signatures, should be idempotent by `event_id`), `verify_chain` (must check `prev_hash` linkage and signatures), `export` (must include genesis/head hashes and a monotonic export counter to detect selective/rolled-back exports).
- **called_by**: HSIP's audit export path (integration point); `NoOpAuditSink` is the only in-crate implementer

### `AuditExport`
- **type**: struct
- **file**: `crates/hsip-integration-sdk/src/lib.rs`
- **purpose**: The tamper-detection-annotated result of `AuditSink::export`: the full event list plus `genesis_hash`/`head_hash`/`export_counter` and an HMAC-style `verification_hash` binding all three together — designed so a selective export, a modified event, or a rolled-back log all become independently detectable.
- **called_by**: `AuditSink::export`

### `Capability`
- **type**: struct
- **file**: `crates/hsip-integration-sdk/src/lib.rs`
- **purpose**: An opaque, expiring capability token (`capability_id`, opaque `token` bytes, `expires_ms`) — deliberately generic (e.g. `"file_transfer"`, `"video_call"`) so it can't encode platform-specific identity.
- **called_by**: `CapabilityProvider`

### `CapabilityProvider`
- **type**: trait
- **file**: `crates/hsip-integration-sdk/src/lib.rs`
- **purpose**: Extension point for issuing/verifying opaque capability tokens per peer. `capabilities_for_peer` must base decisions only on protocol-observable state and should return an empty vec by default; `verify_capability` checks a token's validity for a given peer/capability.
- **called_by**: HSIP's capability-issuance path (integration point); `NoOpCapabilityProvider` is the only in-crate implementer

### `NoOpPolicyHook`
- **type**: struct
- **file**: `crates/hsip-integration-sdk/src/lib.rs`
- **purpose**: Default `PolicyHook` implementation that always returns `None` — falls through to HSIP's built-in default policy every time, i.e. "no custom policy configured" behaves identically to not having this extension point at all.
- **called_by**: default-configuration call sites, this file's own tests

### `NoOpAuditSink`
- **type**: struct
- **file**: `crates/hsip-integration-sdk/src/lib.rs`
- **purpose**: Default `AuditSink` implementation that discards every event: `log_event` always succeeds without storing anything, `verify_chain` always reports `true` (vacuously, since there's nothing to check), `export` returns an empty export with all-zero genesis/head/verification hashes and counter 0.
- **called_by**: default-configuration call sites, this file's own tests

### `NoOpCapabilityProvider`
- **type**: struct
- **file**: `crates/hsip-integration-sdk/src/lib.rs`
- **purpose**: Default `CapabilityProvider` implementation that grants nothing: `capabilities_for_peer` always returns an empty vec, `verify_capability` always returns `Ok(false)`.
- **called_by**: default-configuration call sites, this file's own tests

---

## `crates/hsip-api/src/lib.rs`

The library-target crate root for `hsip-api`. Its sole content is `pub mod` declarations for every top-level source module (`anchor`, `anchor_job`, `audit_log`, `auth`, `config`, `db`, `errors`, `key_encryption`, `metrics`, `mtls`, `rate_limit_persistence`, `routes`, `state`, `system_health`) — no functions or types of its own. This is the "lib" half of the two-target split CLAUDE.md documents at length: `src/main.rs` independently re-declares its own private `mod` tree over the same source files for the `bin "hsip-api"` target, so `lib.rs` exists purely so `tests/integration.rs` (an external test crate, which can only see a library's public API) and other modules' own `#[cfg(test)]` blocks can reach `crate::db`, `crate::state`, etc. Because the two targets compile these files independently, dead-code analysis (and thus `cargo clippy -D warnings`) runs twice and can disagree — a function used only by `lib.rs`-side test code (e.g. `db::init`) can be genuinely dead in the `bin` target's own compilation, hence paired `#[allow(dead_code)]` annotations with explanatory comments at those sites rather than deleting the function.

### `hsip_api` (crate root module declarations)
- **type**: variable (module declarations)
- **file**: `crates/hsip-api/src/lib.rs`
- **purpose**: Re-exports every top-level module as `pub`, giving `tests/integration.rs` and any other external consumer of the `hsip_api` library crate access to `crate::{anchor, anchor_job, audit_log, auth, config, db, errors, key_encryption, metrics, mtls, rate_limit_persistence, routes, state, system_health}`.
- **calls**: none
- **called_by**: `tests/integration.rs`, `rate_limit_persistence.rs`'s own `#[cfg(test)]` tests (via `crate::db::init`), any other in-crate `#[cfg(test)]` code compiled as part of the lib target
- **mutates**: nothing

---

## `crates/hsip-cli/src/commands/mod.rs`

Pure module-declaration file for the CLI's `commands/` directory — no logic of its own.

### `commands` (module declarations)
- **type**: variable (module declarations)
- **file**: `crates/hsip-cli/src/commands/mod.rs`
- **purpose**: Declares every command submodule (`agent`, `diag`, `handshake`, `keys`, `receipts`, `trust`, `up`, `util`) as `pub mod`, making them addressable as `commands::agent`, `commands::util`, etc. from `main.rs` and from each other.
- **calls**: none
- **called_by**: `crates/hsip-cli/src/main.rs`
- **mutates**: nothing

---

## `crates/hsip-cli/src/daemon/mod.rs`

A separate lightweight HTTP status/consent daemon for `hsip-cli` (listens on `127.0.0.1:8787` per `hsip-tray.rs`'s hardcoded client address) distinct from the main `hsip-api` server — mostly stubbed/`TODO`-marked placeholder logic wired up to real gateway metrics for the one field (`blocked_trackers`) that has a real backing file. Every response is wrapped in an HMAC-SHA256 signature for basic integrity, though the signing key is a hardcoded placeholder string flagged in its own comment as "CHANGE IN PRODUCTION."

### `AppState` (daemon)
- **type**: struct
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: Axum shared state for the daemon: an `Arc<Mutex<Status>>` holding the current snapshot. Comment notes future intent to wire in real `sessions`/`reputation` managers, not yet done.
- **called_by**: `http::serve`, all `http::get_*`/`post_*` handlers via `State<AppState>`

### `Status` (daemon)
- **type**: struct
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: The daemon's reported protection status: `protected` flag, session count, egress peer, cipher name, `since` timestamp, byte counters, a routing `path`, and HSIP-shield block counters (`blocked_connections`, `blocked_ips`, `blocked_trackers`).
- **called_by**: `AppState`, `snapshot_status`, `http::get_status`, `hsip-tray.rs::Status` (a separately-defined deserialization mirror, not a shared type)

### `Status::default` (daemon)
- **type**: function
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: Zeroed/placeholder default status: `protected: false`, `cipher: "ChaCha20-Poly1305"`, `path: ["Local"]`, everything else 0/empty.
- **outputs**: `Self`
- **called_by**: `AppState`'s derived `Default`
- **mutates**: nothing

### `GatewayMetricsFile`
- **type**: struct
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: Minimal deserialization target for `hsip-gateway`'s persisted `~/.hsip/gateway_metrics.json` — only reads the `blocked_trackers` field (defaulted to 0 if absent), ignoring the rest of `hsip_gateway::metrics::GatewayMetrics`'s shape. A separate, independently-defined struct rather than a shared dependency on the gateway crate's own type.
- **called_by**: `read_blocked_trackers`

### `gateway_metrics_path`
- **type**: function
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: Resolves `~/.hsip/gateway_metrics.json` — the same path `hsip-gateway::metrics::metrics_path` computes independently (duplicated logic, not a shared constant).
- **outputs**: `PathBuf`
- **called_by**: `read_blocked_trackers`
- **mutates**: nothing

### `read_blocked_trackers`
- **type**: function
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: Reads and parses the gateway's metrics file to get the real `blocked_trackers` count; returns `0` on any read/parse failure (a missing file is silent, a malformed one logs to stderr first).
- **outputs**: `u64`
- **calls**: `gateway_metrics_path`, `fs::read_to_string`, `serde_json::from_str`
- **called_by**: `snapshot_status`
- **mutates**: nothing

### `snapshot_status`
- **type**: function
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: Builds a `Status` snapshot — marked with a `TODO: wire to real session metrics later` comment, since `protected`/`active_sessions`/`egress_peer`/byte counters/`path` are all hardcoded placeholder values. The one real field is `blocked_trackers`, sourced from the actual gateway metrics file.
- **outputs**: `Status`
- **calls**: `chrono::Utc::now`, `read_blocked_trackers`
- **called_by**: `http::serve` (initial state), `http::get_status` is not itself calling this — it reads from `AppState`'s already-set snapshot
- **mutates**: nothing

### `http::RESPONSE_HMAC_KEY`
- **type**: variable (constant)
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: Hardcoded HMAC-SHA256 key used to sign every daemon HTTP response for basic integrity — the literal value ends in `"-CHANGE-IN-PRODUCTION"`, i.e. explicitly a placeholder, not meant to be trusted in real deployments.
- **called_by**: `http::sign_response`

### `http::SignedResponse`
- **type**: struct
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: Generic response envelope: the actual `data`, an HMAC-SHA256 `signature` (hex), and a fixed `sig_alg` label — wraps every daemon HTTP response.
- **called_by**: `http::create_signed_response`

### `http::sign_response`
- **type**: function
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: Serializes `data` to JSON and computes an HMAC-SHA256 over those bytes keyed by `RESPONSE_HMAC_KEY`, hex-encoded.
- **inputs**: `data: &T where T: Serialize`
- **outputs**: `Result<String, String>`
- **calls**: `serde_json::to_vec`, `HmacSha256::new_from_slice`/`update`/`finalize`, `hex::encode`
- **called_by**: `http::create_signed_response`
- **mutates**: nothing

### `http::create_signed_response`
- **type**: function
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: Wraps any serializable `data` into a `SignedResponse` and turns it into an Axum response; on a signing failure, returns a `500` with an `{"error":"signature_failed"}` body instead of panicking.
- **inputs**: `data: T where T: Serialize`
- **outputs**: `axum::response::Response`
- **calls**: `sign_response`, `Json::into_response`
- **called_by**: every `http::get_*`/`post_*` route handler in this module
- **mutates**: nothing

### `http::GrantRequest` / `http::GrantResponse` / `http::RevokeRequest` / `http::ReputationResponse` / `http::SessionView`
- **type**: struct
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: Request/response body shapes for the daemon's stub consent-grant/revoke, reputation, and session-listing endpoints — most are placeholders backing `TODO`-marked handler logic, not real functionality yet.
- **called_by**: their respective `http::post_*`/`get_*` handlers

### `http::serve`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: Builds the daemon's `AppState` (seeding it with `snapshot_status()`), assembles the Axum router (`/status`, `/sessions`, `/consent/grant`, `/consent/revoke`, `/reputation/:peer_id`, `/.well-known/hsip-public-key.txt`), binds `addr`, and serves forever.
- **inputs**: `addr: SocketAddr`
- **outputs**: `anyhow::Result<()>`
- **calls**: `AppState::default`, `snapshot_status`, `Router::new`/`.route`/`.with_state`, `TcpListener::bind`, `axum::serve`
- **called_by**: `hsip-cli`'s daemon-launching command (the process that runs this listens on `127.0.0.1:8787`, matching `hsip-tray.rs`'s hardcoded client address)
- **mutates**: binds a TCP listener; sets `AppState.inner` to the initial snapshot

### `http::get_status`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: Returns the current `Status` snapshot from `AppState`, HMAC-signed.
- **inputs**: `State(state): State<AppState>`
- **outputs**: `impl IntoResponse`
- **calls**: `create_signed_response`
- **called_by**: Axum router (`GET /status`) — this is what `hsip-tray.rs::get_status` polls every 3 seconds
- **mutates**: nothing

### `http::get_sessions`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: Returns a hardcoded single-entry fake session list — placeholder, not backed by any real session tracking.
- **outputs**: `impl IntoResponse`
- **calls**: `create_signed_response`
- **called_by**: Axum router (`GET /sessions`)
- **mutates**: nothing

### `http::post_consent_grant`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: Stub consent-grant handler — per its own `TODO`, fabricates a fake capability token string (`"cap::{grantee}/{purpose}::{expires_ms}"`) rather than calling a real token issuer.
- **inputs**: `Json(req): Json<GrantRequest>`
- **outputs**: `impl IntoResponse`
- **calls**: `create_signed_response`
- **called_by**: Axum router (`POST /consent/grant`)
- **mutates**: nothing

### `http::post_consent_revoke`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: Stub consent-revoke handler — per its own `TODO`, doesn't actually kill any session; just echoes back `{"ok": true, "revoked_for": peer_id}`.
- **inputs**: `Json(req): Json<RevokeRequest>`
- **outputs**: `impl IntoResponse`
- **calls**: `create_signed_response`
- **called_by**: Axum router (`POST /consent/revoke`)
- **mutates**: nothing

### `http::get_reputation`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: Stub reputation lookup — per its own `TODO`, always returns `score: 0` regardless of `peer_id`, not backed by `hsip-reputation::Store`.
- **inputs**: `Path(peer_id): Path<String>`
- **outputs**: `impl IntoResponse`
- **calls**: `create_signed_response`
- **called_by**: Axum router (`GET /reputation/:peer_id`)
- **mutates**: nothing

### `http::get_public_key`
- **type**: function (async)
- **file**: `crates/hsip-cli/src/daemon/mod.rs`
- **purpose**: RFC 8615 `.well-known` endpoint serving the local identity's public key from `~/.hsip/identity.pub` as plain text; if the file doesn't exist yet, returns a helpful placeholder body explaining how to generate one via `hsip-cli keygen`, still as a `200` rather than an error. Unlike every other handler in this module, its response is **not** HMAC-wrapped via `create_signed_response` — it returns raw plain text directly.
- **outputs**: `impl IntoResponse`
- **calls**: `fs::read_to_string`
- **called_by**: Axum router (`GET /.well-known/hsip-public-key.txt`)
- **mutates**: nothing (reads the identity public-key file)

---

## `crates/hsip-cli/src/bin/hsip-tray.rs`

A separate small binary (`hsip-tray`) implementing a system-tray icon that polls the `daemon/mod.rs` HTTP status endpoint every 3 seconds over a raw `TcpStream` (no `reqwest`/HTTP-client crate) and colors itself red/yellow/green based on protection and blocking state.

### `Status` (tray)
- **type**: struct
- **file**: `crates/hsip-cli/src/bin/hsip-tray.rs`
- **purpose**: Deserialization target for the daemon's `/status` JSON body — an independently-defined mirror of `daemon::Status`, not a shared type (this binary doesn't depend on the daemon module's own struct). Several fields are `#[allow(dead_code)]`/unused beyond deserialization.
- **called_by**: `get_status`

### `solid_icon`
- **type**: function
- **file**: `crates/hsip-cli/src/bin/hsip-tray.rs`
- **purpose**: Builds a flat-color square RGBA icon of the given size/color for the tray (used to build the red/green/yellow status icons once at startup).
- **inputs**: `width: u32`, `height: u32`, `rgba: [u8; 4]`
- **outputs**: `tray_icon::Icon`
- **calls**: `tray_icon::Icon::from_rgba`
- **called_by**: `main` (tray)
- **mutates**: nothing

### `get_status` (tray)
- **type**: function
- **file**: `crates/hsip-cli/src/bin/hsip-tray.rs`
- **purpose**: Connects to the daemon at hardcoded `127.0.0.1:8787`, sends a raw `GET /status HTTP/1.1` request with `Connection: close`, reads the full response, splits off the body after the header terminator, and parses it as `Status` JSON. A raw hand-rolled HTTP client rather than using an HTTP library — brittle (assumes `Connection: close` and reads to EOF) but adequate for this local-only, single-purpose polling use.
- **outputs**: `Result<Status>`
- **calls**: `TcpStream::connect`, `stream.write_all`, `stream.read_to_string`, `serde_json::from_str`
- **called_by**: `main` (tray, polling loop)
- **mutates**: opens a TCP connection

### `main` (tray)
- **type**: function
- **file**: `crates/hsip-cli/src/bin/hsip-tray.rs`
- **purpose**: Builds the tray icon (starting red/"starting…"), then loops forever every 3 seconds: on a successful status fetch, picks red (`!protected`), yellow (any of `blocked_connections`/`blocked_ips`/`blocked_trackers` > 0 — active threats being blocked), or green (protected, nothing currently blocked) and updates the tray icon/tooltip accordingly; on a connection failure, shows red "OFFLINE - Daemon not running." Runs forever — this binary has no exit path other than process termination.
- **outputs**: `Result<()>` (never actually returns under normal operation — the loop is infinite)
- **calls**: `solid_icon`, `TrayIconBuilder::new`/`.build`, `get_status`, `tray.set_icon`/`set_tooltip`, `thread::sleep`
- **called_by**: Rust runtime (binary entry point)
- **mutates**: the tray icon/tooltip; blocks the thread in a poll loop

### `run_tray_ui`
- **type**: function
- **file**: `crates/hsip-cli/src/bin/hsip-tray.rs`
- **purpose**: Dead placeholder (`#[allow(dead_code)]`) — its own comment says "move your existing tray setup/start code here"; currently just sleeps in an hour-long loop forever and is never called from `main`.
- **outputs**: `anyhow::Result<()>` (never returns under normal operation)
- **calls**: `std::thread::sleep`
- **called_by**: nothing (dead code)
- **mutates**: blocks the thread

---
