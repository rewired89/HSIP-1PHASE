//! API Server

use crate::handlers::{self, ApiState};
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

/// HSIP Enterprise API Server
pub struct ApiServer {
    state: Arc<ApiState>,
}

impl ApiServer {
    pub fn new() -> Self {
        let state = Arc::new(ApiState {
            version: "2.0.0".to_string(),
        });

        Self { state }
    }

    /// Build the router
    pub fn router(&self) -> Router {
        Router::new()
            // Health check
            .route("/health", get(handlers::health_check))

            // Enterprise message operations
            .route("/api/v2/messages/send", post(handlers::send_message))
            .route("/api/v2/messages/verify", post(handlers::verify_message))

            // Consent management
            .route("/api/v2/consent/grant", post(handlers::grant_consent))
            .route("/api/v2/consent/revoke", post(handlers::revoke_consent))

            // Audit export
            .route("/api/v2/audit/export", post(handlers::export_audit))

            // Add state and middleware
            .with_state(self.state.clone())
            .layer(TraceLayer::new_for_http())
    }

    /// Run the server
    pub async fn run(self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let listener = tokio::net::TcpListener::bind(addr).await?;

        tracing::info!("HSIP Enterprise API listening on {}", addr);

        axum::serve(listener, self.router()).await?;

        Ok(())
    }
}

impl Default for ApiServer {
    fn default() -> Self {
        Self::new()
    }
}
