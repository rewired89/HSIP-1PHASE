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

mod auth;
mod db;
mod errors;
mod key_encryption;
mod metrics;
mod routes;
mod state;

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "hsip_api=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    metrics::init();

    // C1: load master key at startup — exits with clear error if not set
    let master_key = key_encryption::load_master_key();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        let path = std::env::var("HSIP_DB_PATH").unwrap_or_else(|_| "hsip_api.db".to_string());
        format!("sqlite:{path}")
    });

    let db = db::init(&database_url).await?;
    bootstrap_admin(&db).await?;

    let state = AppState::new(db, master_key);

    // H4: build CORS from CORS_ORIGINS env var, defaulting to restrictive
    let cors = build_cors_layer();

    // L2: request-id header name
    let x_request_id = HeaderName::from_static("x-request-id");

    let app = Router::new()
        .merge(routes::router())
        .route("/metrics", get(metrics_handler))
        .route("/health",  get(health_handler))
        .route("/openapi.json", get(openapi_handler))
        .route("/docs",    get(docs_handler))
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
        // L2: attach a unique request ID to every request and propagate to response
        .layer(SetRequestIdLayer::new(x_request_id.clone(), MakeRequestUuid))
        .layer(PropagateRequestIdLayer::new(x_request_id))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");

    tracing::info!("HSIP API listening on http://{addr}");
    tracing::info!("Docs:    http://{addr}/docs");
    tracing::info!("Metrics: http://{addr}/metrics  (set METRICS_TOKEN to protect)");
    tracing::info!("Health:  http://{addr}/health");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// H4: Build a CORS layer from CORS_ORIGINS env var.
/// Set CORS_ORIGINS=https://your-dashboard.com,https://app.example.com
/// Defaults to deny all cross-origin if not set.
fn build_cors_layer() -> CorsLayer {
    let origins_env = std::env::var("CORS_ORIGINS").unwrap_or_default();
    if origins_env.trim().is_empty() {
        // No CORS_ORIGINS set: deny all cross-origin requests
        return CorsLayer::new();
    }

    let origins: Vec<axum::http::HeaderValue> = origins_env
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|o| o.parse().ok())
        .collect();

    if origins.is_empty() {
        return CorsLayer::new();
    }

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::DELETE,
        ])
        .allow_headers([axum::http::header::AUTHORIZATION, axum::http::header::CONTENT_TYPE])
}

/// H7: Metrics endpoint protected by METRICS_TOKEN env var.
/// Set METRICS_TOKEN=<secret> and pass Authorization: Bearer <secret> to access.
async fn metrics_handler(
    headers: HeaderMap,
) -> impl IntoResponse {
    let token_env = std::env::var("METRICS_TOKEN").unwrap_or_default();
    if !token_env.is_empty() {
        let provided = headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or("");
        if provided != token_env.trim() {
            return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
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

/// M3: Swagger UI with pinned CDN version and SRI integrity hashes.
/// SRI hashes are for swagger-ui-dist@5.17.14 — verify with:
///   curl -s https://unpkg.com/swagger-ui-dist@5.17.14/swagger-ui-bundle.js | openssl dgst -sha384 -binary | openssl base64 -A
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

async fn bootstrap_admin(db: &db::Db) -> anyhow::Result<()> {
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
        return Ok(());
    }

    let tenant_id = Uuid::new_v4().to_string();
    let now       = now_ms();

    sqlx::query("INSERT INTO tenants (id, name, created_at) VALUES (?, 'default', ?)")
        .bind(&tenant_id)
        .bind(now)
        .execute(db)
        .await?;

    // L1: use OsRng explicitly for admin key generation
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
    println!("║  Key also saved to: hsip_admin_key.txt (0600)   ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();

    // C2: write key file with restricted permissions (owner read-only)
    std::fs::write("hsip_admin_key.txt", &raw_key)
        .unwrap_or_else(|e| eprintln!("Warning: could not write key file: {e}"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(
            "hsip_admin_key.txt",
            std::fs::Permissions::from_mode(0o600),
        );
    }

    Ok(())
}
