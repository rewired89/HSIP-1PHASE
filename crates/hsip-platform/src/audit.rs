//! The platform records its own sensitive actions as proofs in each
//! organization's `system` stream: the audit trail is signed, chained and
//! anchored by the same machinery customers are buying.

use std::collections::BTreeMap;

use sqlx::Row;

use crate::{engine, state::AppState};

/// Best effort: a failed audit write is logged, never allowed to undo or
/// fail an action that already happened.
pub async fn log(
    state: &AppState,
    org_id: &str,
    actor: &str,
    action: &str,
    subject: &str,
    detail: &str,
) {
    let stream = match sqlx::query("SELECT id FROM streams WHERE org_id = $1 AND kind = 'system'")
        .bind(org_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(r)) => match r.try_get::<String, _>(0) {
            Ok(s) => s,
            Err(_) => return,
        },
        _ => {
            tracing::error!(org = org_id, action, "audit: no system stream");
            return;
        }
    };
    let mut metadata = BTreeMap::new();
    if !detail.is_empty() {
        metadata.insert("detail".to_string(), detail.chars().take(250).collect());
    }
    let payload_hash =
        hsip_proof::sha256_hex(format!("{org_id}|{actor}|{action}|{subject}|{detail}").as_bytes());
    let np = engine::NewProof {
        org_id,
        stream_id: &stream,
        key_id: None,
        actor,
        action,
        subject,
        payload_hash: &payload_hash,
        metadata,
    };
    if let Err(e) = engine::record(state, &np).await {
        tracing::error!(org = org_id, action, error = ?e, "audit write failed");
    }
}
