// On Windows release builds, suppress the console window so the app
// runs silently in the background (browser opens automatically).
#![cfg_attr(
    all(windows, feature = "embed-dashboard"),
    windows_subsystem = "windows"
)]

use anyhow::{Context, Result};
use axum::http::header::HeaderName;
use axum::{
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

mod anchor;
mod anchor_job;
mod audit_log;
mod auth;
mod config;
mod db;
mod errors;
mod key_encryption;
mod metrics;
mod mtls;
mod rate_limit_persistence;
mod routes;
mod state;
mod static_files;
mod system_health;

use config::Config;
use state::AppState;

/// On Windows the terminal window vanishes on exit — pause so the user can
/// read the error message regardless of how the binary was compiled.
fn fatal(msg: &str) -> ! {
    eprintln!("\n{}", msg);
    #[cfg(windows)]
    {
        use std::io::BufRead;
        eprintln!("\nPress Enter to close...");
        let mut buf = String::new();
        let _ = std::io::stdin().lock().read_line(&mut buf);
    }
    std::process::exit(1);
}

#[tokio::main]
async fn main() {
    // Must happen before any rustls ServerConfig/ClientConfig is built
    // (TLS server binding below, or reqwest's TLS client used elsewhere in
    // this process for OpenTimestamps submission). Both `axum-server`
    // (server TLS) and `reqwest` (HTTP client TLS) enable a rustls crypto
    // provider feature — `aws-lc-rs` and `ring` respectively — so more than
    // one is compiled into this binary and rustls can't auto-select a
    // default; without this, the first TLS operation anywhere in the
    // process panics with "Could not automatically determine the
    // process-level CryptoProvider". Ignore the Err: it only means some
    // other call already won the race to install one, which is fine.
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // On Windows desktop builds: self-install on first run (creates shortcuts,
    // copies exe to %LOCALAPPDATA%\HSIP, then re-launches from there).
    #[cfg(all(windows, feature = "embed-dashboard"))]
    maybe_self_install(); // may exit this process

    if let Err(e) = run().await {
        let msg = format!("❌ {:#}", e);
        write_error_log(&msg);
        fatal(&msg);
    }
}

/// Append a fatal error to the HSIP log file (best-effort, never panics).
fn write_error_log(msg: &str) {
    use std::io::Write;
    let log_path = config::hsip_data_dir().join("hsip.log");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(f, "[{}] {}", ts, msg);
        eprintln!("\n(Error written to {})", log_path.display());
    }
}

async fn run() -> Result<()> {
    // Desktop release binary (embed-dashboard): always use zero-config desktop
    // defaults so non-tech users never have to touch a config file.
    // The HSIP_CONFIG env var can still override this for power users.
    #[cfg(feature = "embed-dashboard")]
    let (config, config_source) = {
        if let Ok(path) = std::env::var("HSIP_CONFIG") {
            let cfg = Config::load(&path)
                .unwrap_or_else(|e| fatal(&format!("❌ Configuration error ({}): {}", path, e)));
            (cfg, path)
        } else {
            let cfg = config::Config::desktop_defaults().unwrap_or_else(|e| {
                fatal(&format!(
                    "❌ Failed to initialise HSIP data directory: {}",
                    e
                ))
            });
            (
                cfg,
                format!("desktop defaults ({})", config::hsip_data_dir().display()),
            )
        }
    };

    // Dev / server binary: load config.toml from current directory, fall back
    // to desktop defaults if no file is found.
    #[cfg(not(feature = "embed-dashboard"))]
    let (config, config_source) = {
        let config_path =
            std::env::var("HSIP_CONFIG").unwrap_or_else(|_| "config.toml".to_string());

        match Config::load(&config_path) {
            Ok(cfg) => (cfg, config_path),
            Err(_) if config_path == "config.toml" => {
                let cfg = config::Config::desktop_defaults().unwrap_or_else(|e| {
                    fatal(&format!(
                        "❌ Failed to initialise HSIP data directory: {}",
                        e
                    ))
                });
                (
                    cfg,
                    format!("desktop defaults ({})", config::hsip_data_dir().display()),
                )
            }
            Err(e) => {
                fatal(&format!(
                    "❌ Configuration error: {}\n\nSet HSIP_CONFIG=/path/to/config.toml or run from a directory with config.toml",
                    e
                ));
            }
        }
    };

    // Validate config (skip in desktop-defaults mode where key files are
    // freshly created placeholders that bootstrap_admin will fill).
    if !config_source.starts_with("desktop defaults") {
        if let Err(e) = config.validate() {
            fatal(&format!("❌ Configuration validation failed: {}", e));
        }
    }

    // Initialize logging based on config
    init_logging(&config);

    tracing::info!("Starting HSIP API v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Configuration: {}", config_source);

    // Initialize metrics
    metrics::init();

    // HSIP_SANDBOX=true opens POST /v1/sandbox/provision — the one endpoint
    // in this API that requires no bearer key at all. It's rate-limited and
    // capped, but flip this on somewhere it shouldn't be and it's an open
    // tenant/API-key mint. Make that impossible to miss at boot.
    if std::env::var("HSIP_SANDBOX").as_deref() == Ok("true") {
        tracing::warn!("╔══════════════════════════════════════════════════════════╗");
        tracing::warn!("║  HSIP_SANDBOX=true — UNAUTHENTICATED PROVISIONING IS ON  ║");
        tracing::warn!("║  POST /v1/sandbox/provision requires NO Authorization    ║");
        tracing::warn!("║  header and mints a 24h trial tenant + API key for any   ║");
        tracing::warn!("║  caller. Intended for a public demo deployment only.     ║");
        tracing::warn!("║  Unset HSIP_SANDBOX if this is not that.                 ║");
        tracing::warn!("╚══════════════════════════════════════════════════════════╝");
    }

    // Load master encryption key
    let (master_key, master_key_path) = load_master_key(&config.security.master_key_path)?;
    tracing::info!("✓ Master encryption key loaded");

    // Initialize database
    let db = db::init_with_config(&config.database).await?;
    tracing::info!(
        "✓ Database initialized: {}",
        if config.database.url.contains("postgres") {
            "PostgreSQL"
        } else {
            "SQLite"
        }
    );

    // Bootstrap admin tenant and key
    bootstrap_admin(&db, &config.security.admin_key_path).await?;

    let state = AppState::new_with_master_key_path(db, master_key, master_key_path);

    // Restore rate-limit/AI-agent-velocity counters saved by the last
    // snapshot before serving any traffic — best-effort, a failure here
    // must not block startup (worst case: counters start fresh, same as
    // before this feature existed).
    if let Err(e) = rate_limit_persistence::load(&state.db, &state).await {
        tracing::warn!(error = %e, "failed to restore rate-limit state from last snapshot");
    }

    // Periodic snapshot of the in-memory rate-limit / AI-agent-velocity
    // DashMaps so a restart doesn't silently reset abuse-detection
    // counters. See rate_limit_persistence.rs for why this is a periodic
    // snapshot rather than a write-through on every request.
    {
        let snapshot_db = state.db.clone();
        let snapshot_state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                rate_limit_persistence::SNAPSHOT_INTERVAL_SECS,
            ));
            loop {
                interval.tick().await;
                if let Err(e) =
                    rate_limit_persistence::snapshot(&snapshot_db, &snapshot_state).await
                {
                    tracing::warn!(error = %e, "rate-limit state snapshot failed");
                }
            }
        });
    }

    // Periodic anchoring cycle: batches unanchored decisions, and
    // separately unanchored audit-log entries, into RFC 6962 Merkle trees
    // and submits each root to OpenTimestamps on a "whichever comes first"
    // cadence (see anchor_job::BATCH_SIZE_TRIGGER / INTERVAL_TRIGGER_MS).
    // The 10s poll interval just checks whether a cycle is due — most ticks
    // are a no-op for both.
    {
        let anchor_db = state.db.clone();
        let anchor_master_key_lock = state.master_key.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                // Snapshot the key rather than holding the lock for the
                // cycle's duration (which includes network I/O to
                // OpenTimestamps) — a rotation in progress would otherwise
                // block behind a live anchor cycle for no reason.
                let anchor_master_key = anchor_master_key_lock.read().await.clone();
                match anchor_job::run_anchor_cycle(&anchor_db, &anchor_master_key).await {
                    Ok(Some(summary)) => {
                        tracing::info!(
                            anchor_id = %summary.anchor_id,
                            leaf_count = summary.leaf_count,
                            ots_status = %summary.ots_status,
                            "anchored decision batch"
                        );
                    }
                    Ok(None) => {}
                    Err(e) => tracing::warn!(error = %e, "decision anchor cycle failed"),
                }
                match anchor_job::run_audit_anchor_cycle(&anchor_db, &anchor_master_key).await {
                    Ok(Some(summary)) => {
                        tracing::info!(
                            anchor_id = %summary.anchor_id,
                            leaf_count = summary.leaf_count,
                            ots_status = %summary.ots_status,
                            "anchored audit-log batch"
                        );
                    }
                    Ok(None) => {}
                    Err(e) => tracing::warn!(error = %e, "audit-log anchor cycle failed"),
                }
            }
        });
    }

    // Poll calendars for anchor batches still sitting at ots_status =
    // 'pending' and upgrade any that have since been confirmed by a mined
    // Bitcoin block (see anchor_job::run_upgrade_cycle). A much slower
    // interval than the 10s anchor-submission loop above — Bitcoin blocks
    // land roughly every 10 minutes on average, so checking every 10s would
    // just hammer the calendars for no benefit. No master key needed: this
    // only reads/updates ots_proof/ots_status, it doesn't sign anything.
    {
        let upgrade_db = state.db.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(15 * 60));
            loop {
                interval.tick().await;
                anchor_job::run_upgrade_cycle(&upgrade_db).await;
            }
        });
    }

    // Periodically refresh metrics::SYSTEM_HEALTH_ISSUES (see
    // system_health.rs) so conditions needing operator attention — an
    // incomplete master key rotation, zero root-admin keys, abandoned OTS
    // anchors — show up on /metrics even if nobody's polling
    // GET /v1/admin/system-health themselves. This is the actual answer to
    // "how would an operator find out something needs manual intervention":
    // a business running real Prometheus alerting can fire on
    // hsip_system_health_issues{severity="critical"} > 0 without ever
    // touching HSIP's own API. Cheap (a filesystem stat plus a couple of
    // COUNT(*) queries), so a 5-minute interval is plenty responsive without
    // being wasteful.
    {
        let health_db = state.db.clone();
        let health_master_key_path = state.master_key_path.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5 * 60));
            loop {
                interval.tick().await;
                let path = health_master_key_path.as_deref().map(|p| p.as_str());
                system_health::check_and_update_metrics(&health_db, path).await;
            }
        });
    }

    // Sweep expired replay-protection nonces so opt-in callers using
    // x-hsip-timestamp/x-hsip-nonce (see auth.rs::check_replay_protection)
    // don't leave the tracker growing unbounded. No-op for deployments where
    // nobody sends those headers — the map just stays empty.
    {
        let replay_nonces = state.replay_nonces.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let now = db::now_ms();
                replay_nonces.retain(|_, expiry_ms| *expiry_ms > now);
            }
        });
    }

    // Build CORS layer from config
    let cors = build_cors_layer(&config.cors);

    // Request ID header
    let x_request_id = HeaderName::from_static("x-request-id");

    #[allow(unused_mut)]
    let mut app = Router::new()
        .merge(routes::router())
        .route("/metrics", get(metrics_handler))
        .route("/health", get(health_handler))
        .route("/openapi.json", get(openapi_handler))
        .route("/docs", get(docs_handler));

    // Serve the embedded React dashboard on all non-API paths (release builds only)
    #[cfg(feature = "embed-dashboard")]
    {
        app = app.fallback(static_files::serve);
    }

    let app = app
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
        .layer(SetRequestIdLayer::new(
            x_request_id.clone(),
            MakeRequestUuid,
        ))
        .layer(PropagateRequestIdLayer::new(x_request_id))
        .with_state(state);

    let addr = format!("{}:{}", config.server.host, config.server.port);

    // Start server with TLS if configured
    if let Some(ref tls_config) = config.server.tls {
        tracing::info!("🔒 TLS enabled");
        tracing::info!("   Certificate: {}", tls_config.cert_path);
        tracing::info!("   Private key: {}", tls_config.key_path);
        if let Some(ref ca_path) = tls_config.client_ca_path {
            tracing::info!("🔐 Mutual TLS enabled — client certificate required");
            tracing::info!("   Client CA: {}", ca_path);
        }

        let rustls_config = mtls::build_rustls_config(
            &tls_config.cert_path,
            &tls_config.key_path,
            tls_config.client_ca_path.as_deref(),
        )
        .await
        .context("Failed to load TLS certificates")?;

        tracing::info!("🚀 HSIP API listening on https://{}", addr);
        tracing::info!("   Docs:    https://{}/docs", addr);
        tracing::info!("   Metrics: https://{}/metrics", addr);
        tracing::info!("   Health:  https://{}/health", addr);

        let bind_addr: std::net::SocketAddr = addr.parse()?;
        if tls_config.client_ca_path.is_some() {
            // mTLS is configured: use ClientCertAcceptor so every request's
            // extensions carry the connection's client-cert fingerprint,
            // for auth.rs's per-key binding check
            // (api_keys.bound_client_cert_fingerprint). The plain-TLS
            // branch below is untouched — this only runs when an operator
            // has already opted into client_ca_path.
            axum_server::bind(bind_addr)
                .acceptor(mtls::ClientCertAcceptor::new(rustls_config))
                .serve(app.into_make_service())
                .await?;
        } else {
            axum_server::bind_rustls(bind_addr, rustls_config)
                .serve(app.into_make_service())
                .await?;
        }
    } else {
        tracing::warn!("⚠️  TLS is DISABLED - this is insecure for production!");
        tracing::warn!("   Configure [server.tls] in config.toml to enable HTTPS");

        let url = format!("http://{}", addr);

        // Check whether HSIP is already running on this port before attempting to
        // bind.  This applies to both desktop (embed-dashboard) and dev builds so
        // that every build mode gives a clear, actionable message instead of a
        // confusing "already in use" error after the "listening on …" log line.
        {
            use std::net::TcpStream;
            if TcpStream::connect(&addr as &str).is_ok() {
                #[cfg(feature = "embed-dashboard")]
                {
                    tracing::info!("HSIP already running — opening browser to existing session");
                    let _ = webbrowser::open(&url);
                    return Ok(());
                }
                #[cfg(not(feature = "embed-dashboard"))]
                return Err(anyhow::anyhow!(
                    "Port {} is already in use. \
                     An HSIP instance may already be running — check Task Manager / ps. \
                     Stop the other process or change the port in config.toml.",
                    config.server.port
                ));
            }
        }

        let listener = match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => l,
            // Race condition: something grabbed the port between the check above and bind.
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                #[cfg(feature = "embed-dashboard")]
                {
                    tracing::info!("Port taken — opening browser to existing HSIP session");
                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                    let _ = webbrowser::open(&url);
                    return Ok(());
                }
                #[cfg(not(feature = "embed-dashboard"))]
                return Err(anyhow::anyhow!(
                    "Port {} is already in use. \
                     An HSIP instance may already be running — check Task Manager / ps. \
                     Stop the other process or change the port in config.toml.",
                    config.server.port
                ));
            }
            Err(e) => return Err(e.into()),
        };

        // Log *after* a successful bind so the "listening on" message is only
        // ever printed when the server is actually ready to accept connections.
        tracing::info!("🚀 HSIP API listening on {}", url);
        tracing::info!("   Docs:    http://{}/docs", addr);
        tracing::info!("   Metrics: http://{}/metrics", addr);
        tracing::info!("   Health:  http://{}/health", addr);

        // Auto-open the dashboard in the default browser (desktop/release builds)
        #[cfg(feature = "embed-dashboard")]
        {
            let open_url = url.clone();
            tokio::spawn(async move {
                // Small delay so the server finishes binding before the browser hits it
                tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                if let Err(e) = webbrowser::open(&open_url) {
                    tracing::warn!("Could not open browser automatically: {}", e);
                }
            });
        }

        axum::serve(listener, app).await?;
    }

    Ok(())
}

fn init_logging(config: &Config) {
    use tracing_subscriber::fmt::format::FmtSpan;

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.logging.level));

    match config.logging.format {
        config::LogFormat::Json => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_span_events(FmtSpan::CLOSE),
                )
                .init();
        }
        config::LogFormat::Pretty => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().with_span_events(FmtSpan::CLOSE))
                .init();
        }
    }
}

/// Loads the master encryption key. `HSIP_MASTER_KEY`, when set, takes
/// precedence over the file at `path` — this is what makes the
/// THREAT_MODEL.md-recommended mitigation for "master key lives on the
/// filesystem" ("point HSIP_MASTER_KEY at a secrets manager") actually
/// work. Previously nothing in the real startup path read that env var:
/// `key_encryption::load_master_key()` was the only code that did, and it
/// was `#[allow(dead_code)]` and never called.
///
/// Returns `(key_bytes, master_key_path)` — `master_key_path` is `Some`
/// only when the key came from the file, since that's the only source this
/// process can durably rewrite (used by the master-key rotation endpoint).
/// When sourced from `HSIP_MASTER_KEY`, rotation must happen wherever that
/// env var's value is managed (e.g. the secrets manager), not via this API.
fn load_master_key(path: &str) -> Result<(Vec<u8>, Option<String>)> {
    if let Ok(env_hex) = std::env::var("HSIP_MASTER_KEY") {
        let env_hex = env_hex.trim();
        if !env_hex.is_empty() {
            let key_bytes =
                hex::decode(env_hex).context("HSIP_MASTER_KEY must be valid hexadecimal")?;
            if key_bytes.len() != 32 {
                anyhow::bail!(
                    "HSIP_MASTER_KEY must be exactly 32 bytes (64 hex characters), got {} bytes",
                    key_bytes.len()
                );
            }
            tracing::info!(
                fingerprint = %master_key_fingerprint(&key_bytes),
                "✓ Master key loaded from HSIP_MASTER_KEY env var — {} is not read and \
                 cannot be rotated via the API while this is set",
                path
            );
            return Ok((key_bytes, None));
        }
    }

    let key_hex = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read master key from: {}", path))?;

    let key_hex = key_hex.trim();

    let key_bytes = hex::decode(key_hex).context("Master key must be valid hexadecimal")?;

    if key_bytes.len() != 32 {
        anyhow::bail!(
            "Master key must be exactly 32 bytes (64 hex characters), got {} bytes",
            key_bytes.len()
        );
    }

    tracing::info!(
        fingerprint = %master_key_fingerprint(&key_bytes),
        "Master key loaded from: {}", path
    );
    tracing::warn!(
        "⚠️  Back up {} now. If this file is lost, every tenant's encrypted \
         signing key becomes permanently unrecoverable — there is no other copy.",
        path
    );
    Ok((key_bytes, Some(path.to_string())))
}

/// SHA-256 fingerprint of the master key, safe to log — lets an operator
/// confirm a backup file matches the key actually in use without ever
/// printing or transmitting the key itself.
fn master_key_fingerprint(key_bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(key_bytes);
    hex::encode(&digest[..8]) // first 8 bytes — enough to compare, not enough to help brute force
}

fn build_cors_layer(cors_config: &config::CorsConfig) -> CorsLayer {
    if cors_config.allowed_origins.is_empty() {
        tracing::info!("CORS: deny all cross-origin requests (no allowed_origins configured)");
        return CorsLayer::new();
    }

    if cors_config.allowed_origins.iter().any(|o| o == "*") {
        tracing::info!("CORS: allowing all origins (CORS_ALLOW_ALL)");
        return CorsLayer::new()
            .allow_origin(AllowOrigin::any())
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::DELETE,
            ])
            .allow_headers([
                axum::http::header::AUTHORIZATION,
                axum::http::header::CONTENT_TYPE,
            ]);
    }

    let origins: Vec<axum::http::HeaderValue> = cors_config
        .allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    if origins.is_empty() {
        tracing::warn!("CORS: invalid origins configured, denying all");
        return CorsLayer::new();
    }

    tracing::info!("CORS: allowing {} origin(s)", origins.len());

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ])
}

async fn metrics_handler(headers: HeaderMap) -> impl IntoResponse {
    // Get metrics token from environment (can override config)
    let token_env = std::env::var("METRICS_TOKEN").ok();

    if let Some(expected_token) = token_env {
        if !expected_token.is_empty() {
            let provided = headers
                .get("Authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .unwrap_or("");

            if provided != expected_token.trim() {
                return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
            }
        }
    }

    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        metrics::render(),
    )
        .into_response()
}

async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn openapi_handler() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        include_str!("openapi.json"),
    )
}

async fn docs_handler() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/html")],
        r#"<!DOCTYPE html>
<html>
<head>
  <title>HSIP API Docs</title>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="stylesheet" type="text/css"
    href="https://unpkg.com/swagger-ui-dist@5.17.14/swagger-ui.css"
    integrity="sha384-RNPcjcpmuKyMRQicJJhbmjN9P5v4Mf62V/lJNBGgD2M9qAzPuUmIQQ8+zy8GUF6"
    crossorigin="anonymous">
</head>
<body>
  <div id="swagger-ui"></div>
  <script
    src="https://unpkg.com/swagger-ui-dist@5.17.14/swagger-ui-bundle.js"
    integrity="sha384-l73Hn5MbNGM0KmqKvPMKSFbFJXxhkRPHKZaSHbCBe7CL93iXCCkFuQFnWayC0eo"
    crossorigin="anonymous"></script>
  <script>
    window.onload = function() {
      SwaggerUIBundle({
        url: "/openapi.json",
        dom_id: '#swagger-ui',
        presets: [SwaggerUIBundle.presets.apis, SwaggerUIBundle.SwaggerUIStandalonePreset],
        layout: "BaseLayout"
      })
    }
  </script>
</body>
</html>"#,
    )
}

/// Write Desktop + Start Menu shortcuts pointing at `target_exe`.
/// Uses the `mslnk` crate to write the .lnk binary directly — no PowerShell,
/// no VBScript, no execution policy, no subprocess of any kind.
#[cfg(all(windows, feature = "embed-dashboard"))]
fn create_shortcuts(target_exe: &std::path::Path) {
    use mslnk::ShellLink;

    let target = target_exe.to_string_lossy();

    let mut folders: Vec<std::path::PathBuf> = Vec::new();

    // Desktop: dirs::desktop_dir() calls SHGetKnownFolderPath — the only
    // correct way to get Desktop when OneDrive has moved it.
    if let Some(d) = dirs::desktop_dir() {
        folders.push(d);
    } else if let Ok(p) = std::env::var("USERPROFILE") {
        folders.push(std::path::PathBuf::from(p).join("Desktop"));
    }

    // Start Menu → Programs (this path is stable across all Windows versions)
    if let Ok(appdata) = std::env::var("APPDATA") {
        folders.push(
            std::path::PathBuf::from(appdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }

    for folder in &folders {
        let _ = std::fs::create_dir_all(folder);
        let lnk = folder.join("HSIP.lnk");
        if let Ok(sl) = ShellLink::new(&*target) {
            let _ = sl.create_lnk(lnk.to_string_lossy().as_ref());
        }
    }
}

/// Windows-only, embed-dashboard builds.
///
/// If the binary is NOT already running from %LOCALAPPDATA%\HSIP\hsip.exe:
///   1. Creates %LOCALAPPDATA%\HSIP\
///   2. Copies itself there
///   3. Creates a Desktop shortcut and a Start Menu shortcut
///   4. Launches the installed copy
///   5. Exits this process
///
/// On subsequent launches the installed copy detects it's already in the
/// right place and skips straight to starting the server.
#[cfg(all(windows, feature = "embed-dashboard"))]
fn maybe_self_install() {
    use std::path::PathBuf;

    // Resolve install directory
    let local_appdata = match std::env::var("LOCALAPPDATA") {
        Ok(v) if !v.is_empty() => v,
        _ => return,
    };
    let install_dir = PathBuf::from(&local_appdata).join("HSIP");
    let install_exe = install_dir.join("hsip.exe");

    // Already running from the installed location — nothing to do
    let current_exe = match std::env::current_exe() {
        Ok(p) => match p.canonicalize() {
            Ok(c) => c,
            Err(_) => p,
        },
        Err(_) => return,
    };
    let install_canon = install_exe
        .canonicalize()
        .unwrap_or_else(|_| install_exe.clone());
    if current_exe == install_canon {
        return;
    }

    // ── Copy to install location ──────────────────────────────────────────
    if std::fs::create_dir_all(&install_dir).is_err() {
        return;
    }

    // Write an install log — tells us exactly what happened (useful for debugging)
    let log_path = install_dir.join("install.log");
    let copied = std::fs::copy(&current_exe, &install_exe).is_ok();
    let log = format!(
        "HSIP self-install\nfrom:   {}\nto:     {}\ncopy ok: {}\n",
        current_exe.display(),
        install_exe.display(),
        copied
    );
    let _ = std::fs::write(&log_path, log.as_bytes());

    // Whether or not the copy succeeded (the exe may be locked because
    // HSIP was already running), create/refresh shortcuts as long as the
    // installed exe is present.
    if install_exe.exists() {
        // mslnk writes the .lnk binary directly; dirs resolves Desktop path
        // via SHGetKnownFolderPath (handles OneDrive-moved Desktops correctly).
        create_shortcuts(&install_exe);
    } else {
        // Nothing to point a shortcut at — bail out entirely.
        return;
    }

    // ── Launch installed copy (only if we just freshly copied) and exit ──
    // If copy failed the installed copy is already running — don't spawn a
    // second server. Either way this process exits so the installed copy
    // is always the one that serves requests.
    if copied {
        let _ = std::process::Command::new(&install_exe).spawn();
    }
    std::process::exit(0);
}

async fn bootstrap_admin(db: &db::Db, admin_key_path: &str) -> Result<()> {
    use auth::hash_key;
    use db::now_ms;
    use rand::rngs::OsRng;
    use rand::RngCore;
    use sqlx::Row;

    let row = sqlx::query("SELECT COUNT(*) FROM tenants")
        .fetch_one(db)
        .await?;
    let count: i64 = row.try_get(0)?;

    if count > 0 {
        tracing::info!("✓ Admin tenant already exists");
        return Ok(());
    }

    tracing::info!("🔧 First-time setup: creating admin tenant and API key");

    let tenant_id = Uuid::new_v4().to_string();
    let now = now_ms();

    sqlx::query("INSERT INTO tenants (id, name, created_at) VALUES ($1, 'default', $2)")
        .bind(&tenant_id)
        .bind(now)
        .execute(db)
        .await?;

    // HSIP_ADMIN_KEY env var lets Railway/K8s deployments use a fixed key so the
    // admin password survives container restarts (ephemeral filesystem resets the DB).
    let raw_key = match std::env::var("HSIP_ADMIN_KEY") {
        Ok(k) if k.starts_with("hsip_") && k.len() >= 20 => {
            tracing::info!("Using admin key from HSIP_ADMIN_KEY env var");
            k
        }
        _ => {
            let mut raw_bytes = [0u8; 32];
            OsRng.fill_bytes(&mut raw_bytes);
            format!("hsip_{}", hex::encode(raw_bytes))
        }
    };
    let key_hash = hash_key(&raw_key);
    let key_id = Uuid::new_v4().to_string();

    // The bootstrap key is both this tenant's 'owner' (can manage other keys
    // in it) and a root admin (can rotate the master key, grant/revoke
    // other root admins) — this INSERT is what actually establishes both on
    // a fresh install; db.rs's migration backfill only covers upgrades of
    // an already-existing database, which doesn't apply here since this row
    // doesn't exist yet when migrations run.
    sqlx::query(
        "INSERT INTO api_keys (id, tenant_id, key_hash, name, agent_type, role, is_root_admin, created_at, active)
         VALUES ($1, $2, $3, 'admin', 'human', 'owner', 1, $4, 1)",
    )
    .bind(&key_id)
    .bind(&tenant_id)
    .bind(&key_hash)
    .bind(now)
    .execute(db)
    .await?;

    metrics::ACTIVE_TENANTS.inc();

    let from_env = std::env::var("HSIP_ADMIN_KEY").is_ok();
    println!();
    println!("╔══════════════════════════════════════════════════╗");
    println!("║          HSIP API — FIRST-TIME SETUP             ║");
    println!("╠══════════════════════════════════════════════════╣");
    if from_env {
        println!("║  Admin key loaded from HSIP_ADMIN_KEY env var    ║");
    } else {
        println!("║  Admin API Key (save this — shown only once):    ║");
        println!("║                                                  ║");
        println!("║  {:<48}  ║", raw_key);
        println!("║                                                  ║");
        println!("║  Key saved to: {:<33}  ║", admin_key_path);
    }
    println!("╚══════════════════════════════════════════════════╝");
    println!();

    std::fs::write(admin_key_path, &raw_key)
        .with_context(|| format!("Failed to write admin key to: {}", admin_key_path))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(admin_key_path, std::fs::Permissions::from_mode(0o600))?;
    }

    tracing::info!("✓ Admin tenant and key created");

    Ok(())
}
