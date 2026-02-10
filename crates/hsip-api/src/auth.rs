use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use sha2::{Sha256, Digest};
use crate::{errors::ApiError, state::AppState};

#[derive(Clone, Debug)]
pub struct TenantId(pub String);

#[axum::async_trait]
impl FromRequestParts<AppState> for TenantId {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts.headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(str::trim)
            .ok_or_else(|| ApiError::Unauthorized("Missing Authorization: Bearer <key>".into()))?
            .to_string();

        let key_hash = hash_key(&token);
        let db       = state.db.clone();

        let tenant_id = tokio::task::spawn_blocking(move || {
            let conn = db.lock().map_err(|_| ApiError::Internal("db lock poisoned".into()))?;
            match conn.query_row(
                "SELECT tenant_id FROM api_keys WHERE key_hash = ? AND active = 1",
                rusqlite::params![key_hash],
                |row| row.get::<_, String>(0),
            ) {
                Ok(id) => Ok(id),
                Err(rusqlite::Error::QueryReturnedNoRows) =>
                    Err(ApiError::Unauthorized("Invalid API key".into())),
                Err(e) => Err(ApiError::Internal(e.to_string())),
            }
        })
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))??;

        Ok(TenantId(tenant_id))
    }
}

pub fn hash_key(key: &str) -> String {
    let mut h = Sha256::new();
    h.update(key.as_bytes());
    hex::encode(h.finalize())
}
