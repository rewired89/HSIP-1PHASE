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

use config::Config;
use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config_path = std::env::var("HSIP_CONFIG")
        .unwrap_or_else(|_| "config.toml".to_string());

    let config = match Config::load(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("❌ Configuration error: {}", e);
            eprintln!("\nTo generate a sample config file, run:");
            eprintln!("  cp crates/hsip-api/config.toml.example config.toml");
            eprintln!("\nThen edit config.toml with your settings.");
            std::process::exit(1);
        }
    };

    // Validate configuration
    if let Err(e) = config.validate() {
        eprintln!("❌ Configuration validation failed: {}", e);
        std::process::exit(1);
    }

    // Initialize logging based on config
    init_logging(&config);

    tracing::info!("Starting HSIP API v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Loaded configuration from: {}", config_path);

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

    let app = Router::new()
        .merge(routes::router())
        .route("/metrics", get(metrics_handler))
        .route("/health",  get(health_handler))
        .route("/openapi.json", get(openapi_handler))
        .route("/docs",    get(docs_handler))
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

        tracing::info!("🚀 HSIP API listening on http://{}", addr);
        tracing::info!("   Docs:    http://{}/docs", addr);
        tracing::info!("   Metrics: http://{}/metrics", addr);
        tracing::info!("   Health:  http://{}/health", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
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
