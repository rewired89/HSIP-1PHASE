use axum::Router;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

mod auth;
mod db;
mod errors;
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

    let db_path = std::env::var("HSIP_DB_PATH")
        .unwrap_or_else(|_| "hsip_api.db".to_string());

    let db = db::init(&db_path)?;
    bootstrap_admin(&db)?;

    let state = AppState { db };
    let app   = Router::new()
        .merge(routes::router())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");

    tracing::info!("HSIP API listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn bootstrap_admin(db: &db::Db) -> anyhow::Result<()> {
    use auth::hash_key;
    use db::now_ms;
    use rand::Rng;

    let conn = db.lock().expect("db lock");

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tenants", [], |r| r.get(0)
    )?;

    if count > 0 {
        return Ok(());
    }

    let tenant_id = Uuid::new_v4().to_string();
    let now       = now_ms();

    conn.execute(
        "INSERT INTO tenants (id, name, created_at) VALUES (?1, 'default', ?2)",
        rusqlite::params![tenant_id, now],
    )?;

    let raw_bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen::<u8>()).collect();
    let raw_key   = format!("hsip_{}", hex::encode(&raw_bytes));
    let key_hash  = hash_key(&raw_key);
    let key_id    = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO api_keys (id, tenant_id, key_hash, name, created_at, active)
         VALUES (?1, ?2, ?3, 'admin', ?4, 1)",
        rusqlite::params![key_id, tenant_id, key_hash, now],
    )?;

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

    // Write key to file so it is never lost
    std::fs::write("hsip_admin_key.txt", &raw_key)
        .unwrap_or_else(|e| eprintln!("Warning: could not write key file: {e}"));

    Ok(())
}
