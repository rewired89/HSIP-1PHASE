//! Outbound email through an HTTP API compatible with Resend
//! (`POST {HSP_EMAIL_API_URL}/emails`, bearer key). Best effort: a failed or
//! unconfigured send returns `false` and never fails the user action.

use std::time::Duration;

use serde_json::json;

use crate::state::AppState;

/// Plain-text email. Returns whether the provider accepted it.
pub async fn send(state: &AppState, to: &str, subject: &str, text: &str) -> bool {
    let Some(cfg) = &state.cfg.email else {
        return false;
    };
    let res = reqwest::Client::new()
        .post(format!("{}/emails", cfg.api_url))
        .bearer_auth(&cfg.api_key)
        .timeout(Duration::from_secs(8))
        .header("content-type", "application/json")
        .body(json!({ "from": cfg.from, "to": [to], "subject": subject, "text": text }).to_string())
        .send()
        .await;
    match res {
        Ok(r) if r.status().is_success() => true,
        Ok(r) => {
            tracing::warn!(status = %r.status(), "email provider rejected a message");
            false
        }
        Err(e) => {
            tracing::warn!(error = %e, "email send failed");
            false
        }
    }
}
