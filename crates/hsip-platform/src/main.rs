use anyhow::Result;
use hsip_platform::{anchor, app, config::Config, state::AppState};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cfg = Config::from_env();
    let bind = cfg.bind.clone();
    let public_url = cfg.public_url.clone();
    let signup = cfg.signup_open;
    let ots = cfg.ots_enabled;
    let operator = cfg.operator_key.is_some();
    let state = AppState::new(cfg).await?;
    anchor::spawn(state.clone());

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!("HSIP Prove listening on {bind} (public URL {public_url})");
    tracing::info!(
        "sign-up: {} | OpenTimestamps: {} | operator console: {}",
        if signup { "open" } else { "closed" },
        if ots {
            "on"
        } else {
            "off (set HSP_OTS=1 to enable)"
        },
        if operator {
            "enabled"
        } else {
            "disabled (set HSP_OPERATOR_KEY)"
        }
    );
    axum::serve(listener, app(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
