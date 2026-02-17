use axum::{Router, routing::get, response::IntoResponse, Json};
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

mod auth;
mod db;
mod errors;
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

    // DATABASE_URL takes priority; fall back to HSIP_DB_PATH; default to sqlite file
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        let path = std::env::var("HSIP_DB_PATH").unwrap_or_else(|_| "hsip_api.db".to_string());
        format!("sqlite:{path}")
    });

    let db = db::init(&database_url).await?;
    bootstrap_admin(&db).await?;

    let state = AppState::new(db);
    let app = Router::new()
        .merge(routes::router())
        .route("/metrics",    get(metrics_handler))
        .route("/health",     get(health_handler))
        .route("/openapi.json", get(openapi_handler))
        .route("/docs",       get(docs_handler))
        .layer(CorsLayer::permissive())
        .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");

    tracing::info!("HSIP API listening on http://{addr}");
    tracing::info!("Docs:    http://{addr}/docs");
    tracing::info!("Metrics: http://{addr}/metrics");
    tracing::info!("Health:  http://{addr}/health");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn metrics_handler() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        metrics::render(),
    )
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
  <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" >
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"> </script>
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
    use rand::Rng;
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

    let raw_bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen::<u8>()).collect();
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
    println!("║  Key also saved to: hsip_admin_key.txt           ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();

    std::fs::write("hsip_admin_key.txt", &raw_key)
        .unwrap_or_else(|e| eprintln!("Warning: could not write key file: {e}"));

    Ok(())
}
