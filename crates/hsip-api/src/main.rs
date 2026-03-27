// On Windows release builds, suppress the console window so the app
// runs silently in the background (browser opens automatically).
#![cfg_attr(all(windows, feature = "embed-dashboard"), windows_subsystem = "windows")]

use axum::{
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Router,
    Json,
    routing::get,
};
use tower_http::cors::{CorsLayer, AllowOrigin};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, SetRequestIdLayer, PropagateRequestIdLayer};
use axum::http::header::HeaderName;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;
use anyhow::{Context, Result};

mod auth;
mod config;
mod db;
mod errors;
mod key_encryption;
mod metrics;
mod routes;
mod state;
mod static_files;

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
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
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
            let cfg = Config::load(&path).unwrap_or_else(|e| {
                fatal(&format!("❌ Configuration error ({}): {}", path, e))
            });
            (cfg, path)
        } else {
            let cfg = config::Config::desktop_defaults().unwrap_or_else(|e| {
                fatal(&format!("❌ Failed to initialise HSIP data directory: {}", e))
            });
            (cfg, format!("desktop defaults ({})", config::hsip_data_dir().display()))
        }
    };

    // Dev / server binary: load config.toml from current directory, fall back
    // to desktop defaults if no file is found.
    #[cfg(not(feature = "embed-dashboard"))]
    let (config, config_source) = {
        let config_path = std::env::var("HSIP_CONFIG")
            .unwrap_or_else(|_| "config.toml".to_string());

        match Config::load(&config_path) {
            Ok(cfg) => (cfg, config_path),
            Err(_) if config_path == "config.toml" => {
                let cfg = config::Config::desktop_defaults().unwrap_or_else(|e| {
                    fatal(&format!("❌ Failed to initialise HSIP data directory: {}", e))
                });
                (cfg, format!("desktop defaults ({})", config::hsip_data_dir().display()))
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

    // Load master encryption key
    let master_key = load_master_key(&config.security.master_key_path)?;
    tracing::info!("✓ Master encryption key loaded");

    // Initialize database
    let db = db::init_with_config(&config.database).await?;
    tracing::info!("✓ Database initialized: {}",
        if config.database.url.contains("postgres") { "PostgreSQL" }
        else { "SQLite" }
    );

    // Bootstrap admin tenant and key
    bootstrap_admin(&db, &config.security.admin_key_path).await?;

    let state = AppState::new(db, master_key);

    // Build CORS layer from config
    let cors = build_cors_layer(&config.cors);

    // Request ID header
    let x_request_id = HeaderName::from_static("x-request-id");

    let mut app = Router::new()
        .merge(routes::router())
        .route("/metrics", get(metrics_handler))
        .route("/health",  get(health_handler))
        .route("/openapi.json", get(openapi_handler))
        .route("/docs",    get(docs_handler));

    // Serve the embedded React dashboard on all non-API paths (release builds only)
    #[cfg(feature = "embed-dashboard")]
    {
        app = app.fallback(static_files::serve);
    }

    let app = app
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
        .layer(SetRequestIdLayer::new(x_request_id.clone(), MakeRequestUuid))
        .layer(PropagateRequestIdLayer::new(x_request_id))
        .with_state(state);

    let addr = format!("{}:{}", config.server.host, config.server.port);

    // Start server with TLS if configured
    if let Some(ref tls_config) = config.server.tls {
        tracing::info!("🔒 TLS enabled");
        tracing::info!("   Certificate: {}", tls_config.cert_path);
        tracing::info!("   Private key: {}", tls_config.key_path);

        let rustls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
            &tls_config.cert_path,
            &tls_config.key_path,
        )
        .await
        .context("Failed to load TLS certificates")?;

        tracing::info!("🚀 HSIP API listening on https://{}", addr);
        tracing::info!("   Docs:    https://{}/docs", addr);
        tracing::info!("   Metrics: https://{}/metrics", addr);
        tracing::info!("   Health:  https://{}/health", addr);

        let bind_addr: std::net::SocketAddr = addr.parse()?;
        axum_server::bind_rustls(bind_addr, rustls_config)
            .serve(app.into_make_service())
            .await?;
    } else {
        tracing::warn!("⚠️  TLS is DISABLED - this is insecure for production!");
        tracing::warn!("   Configure [server.tls] in config.toml to enable HTTPS");

        let url = format!("http://{}", addr);
        tracing::info!("🚀 HSIP API listening on {}", url);
        tracing::info!("   Docs:    http://{}/docs", addr);
        tracing::info!("   Metrics: http://{}/metrics", addr);
        tracing::info!("   Health:  http://{}/health", addr);

        // Desktop builds: if HSIP is already running on this port, open the
        // existing session in the browser and exit cleanly instead of crashing.
        #[cfg(feature = "embed-dashboard")]
        {
            use std::net::TcpStream;
            if TcpStream::connect(&addr as &str).is_ok() {
                tracing::info!("HSIP already running — opening browser to existing session");
                let _ = webbrowser::open(&url);
                return Ok(());
            }
        }

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

        let listener = match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => l,
            // Race condition: something grabbed the port between check and bind.
            // Open browser to existing instance and exit cleanly.
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
                    "Port {} is already in use. Stop the other process or change the port in config.toml.",
                    config.server.port
                ));
            }
            Err(e) => return Err(e.into()),
        };
        axum::serve(listener, app).await?;
    }

    Ok(())
}

fn init_logging(config: &Config) {
    use tracing_subscriber::fmt::format::FmtSpan;

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new(&config.logging.level)
        });

    match config.logging.format {
        config::LogFormat::Json => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer()
                    .json()
                    .with_span_events(FmtSpan::CLOSE))
                .init();
        }
        config::LogFormat::Pretty => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer()
                    .with_span_events(FmtSpan::CLOSE))
                .init();
        }
    }
}

fn load_master_key(path: &str) -> Result<Vec<u8>> {
    let key_hex = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read master key from: {}", path))?;

    let key_hex = key_hex.trim();

    let key_bytes = hex::decode(key_hex)
        .context("Master key must be valid hexadecimal")?;

    if key_bytes.len() != 32 {
        anyhow::bail!("Master key must be exactly 32 bytes (64 hex characters), got {} bytes", key_bytes.len());
    }

    tracing::debug!("Master key loaded from: {}", path);
    Ok(key_bytes)
}

fn build_cors_layer(cors_config: &config::CorsConfig) -> CorsLayer {
    if cors_config.allowed_origins.is_empty() {
        tracing::info!("CORS: deny all cross-origin requests (no allowed_origins configured)");
        return CorsLayer::new();
    }

    let origins: Vec<axum::http::HeaderValue> = cors_config.allowed_origins
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
        .allow_headers([axum::http::header::AUTHORIZATION, axum::http::header::CONTENT_TYPE])
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
        [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        metrics::render(),
    ).into_response()
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
        Ok(p) => match p.canonicalize() { Ok(c) => c, Err(_) => p },
        Err(_) => return,
    };
    let install_canon = install_exe.canonicalize().unwrap_or_else(|_| install_exe.clone());
    if current_exe == install_canon { return; }

    // ── Copy to install location ──────────────────────────────────────────
    if std::fs::create_dir_all(&install_dir).is_err() { return; }
    if std::fs::copy(&current_exe, &install_exe).is_err() { return; }

    // ── Create Desktop + Start Menu shortcuts via PowerShell ──────────────
    // Write a real .ps1 file so there are zero quoting/escaping issues with
    // the -Command inline approach (which silently broke the old code).
    // [Environment]::GetFolderPath() handles OneDrive-moved Desktop paths.
    let exe_path = install_exe.to_string_lossy();
    let script = format!(
        "$exe = \"{exe}\"\r\n\
         $ws  = New-Object -COM WScript.Shell\r\n\
         $d   = [Environment]::GetFolderPath('Desktop')\r\n\
         $p   = [Environment]::GetFolderPath('Programs')\r\n\
         $s = $ws.CreateShortcut(\"$d\\HSIP.lnk\")\r\n\
         $s.TargetPath  = $exe\r\n\
         $s.Description = 'Open HSIP'\r\n\
         $s.Save()\r\n\
         $s = $ws.CreateShortcut(\"$p\\HSIP.lnk\")\r\n\
         $s.TargetPath  = $exe\r\n\
         $s.Description = 'Open HSIP'\r\n\
         $s.Save()\r\n",
        exe = exe_path.replace('"', "`\""),
    );
    let ps1_path = install_dir.join("_create_shortcuts.ps1");
    let _ = std::fs::write(&ps1_path, script.as_bytes());

    // Wait for shortcuts to finish before we exit
    let _ = std::process::Command::new("powershell")
        .args([
            "-WindowStyle",    "Hidden",
            "-NonInteractive",
            "-ExecutionPolicy", "Bypass",
            "-File",            &ps1_path.to_string_lossy(),
        ])
        .status();
    let _ = std::fs::remove_file(&ps1_path);

    // ── Launch installed copy and exit ────────────────────────────────────
    let _ = std::process::Command::new(&install_exe).spawn();
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
    let now       = now_ms();

    sqlx::query("INSERT INTO tenants (id, name, created_at) VALUES (?, 'default', ?)")
        .bind(&tenant_id)
        .bind(now)
        .execute(db)
        .await?;

    let mut raw_bytes = [0u8; 32];
    OsRng.fill_bytes(&mut raw_bytes);
    let raw_key  = format!("hsip_{}", hex::encode(&raw_bytes));
    let key_hash = hash_key(&raw_key);
    let key_id   = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO api_keys (id, tenant_id, key_hash, name, agent_type, created_at, active)
         VALUES (?, ?, ?, 'admin', 'human', ?, 1)",
    )
    .bind(&key_id)
    .bind(&tenant_id)
    .bind(&key_hash)
    .bind(now)
    .execute(db)
    .await?;

    metrics::ACTIVE_TENANTS.inc();

    println!();
    println!("╔══════════════════════════════════════════════════╗");
    println!("║          HSIP API — FIRST-TIME SETUP             ║");
    println!("╠══════════════════════════════════════════════════╣");
    println!("║  Admin API Key (save this — shown only once):    ║");
    println!("║                                                  ║");
    println!("║  {:<48}  ║", raw_key);
    println!("║                                                  ║");
    println!("║  Authorization: Bearer <key>                     ║");
    println!("║  Key saved to: {:<33}  ║", admin_key_path);
    println!("╚══════════════════════════════════════════════════╝");
    println!();

    std::fs::write(admin_key_path, &raw_key)
        .with_context(|| format!("Failed to write admin key to: {}", admin_key_path))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            admin_key_path,
            std::fs::Permissions::from_mode(0o600),
        )?;
    }

    tracing::info!("✓ Admin tenant and key created");

    Ok(())
}
