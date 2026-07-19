//! Node-level administrative operations that are not scoped to a single
//! tenant — currently just master key rotation. Distinct from every other
//! route in this file tree, which all operate within one tenant's data.
//!
//! HSIP does not have a first-class "root admin" concept distinct from
//! "the first tenant's key named admin" (see `main.rs::bootstrap_admin`).
//! `require_root_admin` below is deliberately stricter than "any key named
//! admin" — it also requires the calling tenant to be the very first tenant
//! ever created, so a tenant in a multi-tenant deployment can't grant
//! itself global authority just by naming one of its own keys "admin".
//! This is a known limitation of HSIP's current admin model, not a full
//! superuser system — documented here and in THREAT_MODEL.md.

use axum::{extract::State, http::HeaderMap, Json};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::Row;

use crate::{
    auth::{hash_key, TenantId},
    db::now_ms,
    errors::{ApiError, ApiResult},
    key_encryption::{decrypt_signing_key, encrypt_signing_key},
    metrics,
    state::AppState,
};

async fn require_root_admin(
    db: &crate::db::Db,
    headers: &HeaderMap,
    tenant_id: &str,
) -> ApiResult<()> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .ok_or_else(|| ApiError::Unauthorized("Missing Authorization: Bearer <key>".into()))?;
    let key_hash = hash_key(token);

    let key_row = sqlx::query("SELECT name FROM api_keys WHERE key_hash = ? AND tenant_id = ?")
        .bind(&key_hash)
        .bind(tenant_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ApiError::Internal("authenticated key vanished mid-request".into()))?;
    let key_name: String = key_row.try_get(0)?;

    let root_tenant_row = sqlx::query("SELECT id FROM tenants ORDER BY created_at ASC LIMIT 1")
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ApiError::Internal("no tenants exist".into()))?;
    let root_tenant_id: String = root_tenant_row.try_get(0)?;

    if key_name != "admin" || tenant_id != root_tenant_id {
        return Err(ApiError::Unauthorized(
            "This operation is restricted to the bootstrap admin key.".into(),
        ));
    }
    Ok(())
}

fn fingerprint(key_bytes: &[u8]) -> String {
    let digest = Sha256::digest(key_bytes);
    hex::encode(&digest[..8])
}

#[derive(Serialize)]
pub struct MasterKeyFingerprintResponse {
    pub fingerprint: String,
    /// `None` when the key is sourced from `HSIP_MASTER_KEY` rather than a
    /// file — matches `RotateMasterKeyResponse` (and rotation's own
    /// refusal) in treating that as "no file this process owns."
    pub master_key_path: Option<String>,
}

/// `GET /v1/admin/master-key/fingerprint` — read-only. Returns the SHA-256
/// fingerprint of the master key currently in use, without touching or
/// rotating anything. Closes a real gap: before this existed, the *only*
/// way to see a fingerprint was in the startup log or in a rotation
/// response — there was no way for an operator to confirm "does my backup
/// file actually match what's running right now" without either grepping
/// server logs or triggering an actual rotation (which changes the key).
pub async fn master_key_fingerprint(
    State(state): State<AppState>,
    tenant: TenantId,
    headers: HeaderMap,
) -> ApiResult<Json<MasterKeyFingerprintResponse>> {
    require_root_admin(&state.db, &headers, &tenant.0).await?;

    let key = state.master_key.read().await;
    Ok(Json(MasterKeyFingerprintResponse {
        fingerprint: fingerprint(&key),
        master_key_path: state.master_key_path.as_ref().map(|p| p.to_string()),
    }))
}

#[derive(Serialize)]
pub struct RotateMasterKeyResponse {
    pub identities_reencrypted: u64,
    pub anchor_identity_reencrypted: bool,
    pub old_key_fingerprint: String,
    pub new_key_fingerprint: String,
    pub master_key_path: String,
    pub note: String,
}

/// `POST /v1/admin/master-key/rotate` — generates a new master key,
/// re-encrypts every tenant's `identities.signing_key_b64` and the
/// singleton `anchor_identity` row under it in one DB transaction, then
/// durably swaps the key file (staging file + atomic rename, so a crash
/// mid-write leaves either the old file untouched or the new file fully
/// written — never a half-written key on disk) and finally the in-memory
/// key used by every subsequent request.
///
/// A `master_key.write().await` guard is held for the entire operation
/// (not just the swap at the end): without it, a concurrent
/// `identity::create_or_get` (or any other handler that encrypts a new row
/// under the *old* key while this function's SELECT has already run) could
/// commit its write after this transaction reads the table but before the
/// in-memory key flips — leaving that one row encrypted under a key this
/// process no longer holds. Holding the write lock for the whole operation
/// makes rotation a brief, deliberate stop-the-world instead of a narrow,
/// hard-to-reproduce corruption window. Rotation is rare and admin-only;
/// this cost is the right tradeoff.
pub async fn rotate_master_key(
    State(state): State<AppState>,
    tenant: TenantId,
    headers: HeaderMap,
) -> ApiResult<Json<RotateMasterKeyResponse>> {
    require_root_admin(&state.db, &headers, &tenant.0).await?;

    let Some(master_key_path) = state.master_key_path.as_ref() else {
        return Err(ApiError::BadRequest(
            "Master key is sourced from HSIP_MASTER_KEY, not a file this process can rewrite. \
             Rotate it wherever that value is managed (e.g. your secrets manager), then \
             restart HSIP with the new value."
                .into(),
        ));
    };

    let mut master_key_guard = state.master_key.write().await;
    let old_key: Vec<u8> = master_key_guard.clone();

    let mut new_key = vec![0u8; 32];
    {
        use rand::RngCore;
        rand::rngs::OsRng.fill_bytes(&mut new_key);
    }

    let mut tx = state.db.begin().await?;

    let identity_rows = sqlx::query("SELECT tenant_id, signing_key_b64 FROM identities")
        .fetch_all(&mut *tx)
        .await?;

    let mut identities_reencrypted = 0u64;
    for row in &identity_rows {
        let row_tenant_id: String = row.try_get(0)?;
        let encrypted_b64: String = row.try_get(1)?;

        let key_bytes = decrypt_signing_key(&encrypted_b64, &old_key).map_err(|e| {
            ApiError::Internal(format!(
                "master key rotation aborted: could not decrypt identity for tenant {row_tenant_id} \
                 under the current master key — {e}. No changes were made."
            ))
        })?;
        let re_encrypted = encrypt_signing_key(&key_bytes, &new_key);

        sqlx::query("UPDATE identities SET signing_key_b64 = ? WHERE tenant_id = ?")
            .bind(&re_encrypted)
            .bind(&row_tenant_id)
            .execute(&mut *tx)
            .await?;
        identities_reencrypted += 1;
    }

    let anchor_row = sqlx::query("SELECT signing_key_b64 FROM anchor_identity WHERE id = 1")
        .fetch_optional(&mut *tx)
        .await?;
    let anchor_identity_reencrypted = if let Some(row) = anchor_row {
        let encrypted_b64: String = row.try_get(0)?;
        let key_bytes = decrypt_signing_key(&encrypted_b64, &old_key).map_err(|e| {
            ApiError::Internal(format!(
                "master key rotation aborted: could not decrypt anchor_identity under the \
                 current master key — {e}. No changes were made."
            ))
        })?;
        let re_encrypted = encrypt_signing_key(&key_bytes, &new_key);
        sqlx::query("UPDATE anchor_identity SET signing_key_b64 = ? WHERE id = 1")
            .bind(&re_encrypted)
            .execute(&mut *tx)
            .await?;
        true
    } else {
        false
    };

    // Write the new key to a staging file on the same filesystem as the
    // real path *before* committing the DB transaction. If this fails,
    // returning here drops `tx` unconsumed, which rolls it back — nothing
    // changed. If it succeeds but the process crashes before the commit
    // below, the DB is still under the old key and the real key file is
    // untouched (only the staging file has the new key) — safe, just
    // requires re-running rotation.
    let staging_path = format!("{master_key_path}.rotating");
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&staging_path)
            .map_err(|e| ApiError::Internal(format!("failed to write staging key file: {e}")))?;
        f.write_all(hex::encode(&new_key).as_bytes())
            .and_then(|_| f.sync_all())
            .map_err(|e| ApiError::Internal(format!("failed to write staging key file: {e}")))?;
    }

    tx.commit().await?;

    // Narrowest possible risk window: if the process dies between the
    // commit above and the rename below, the DB now holds ciphertext under
    // the new key but the real key file still has the old one. The staging
    // file is left in place specifically so that window is recoverable —
    // an operator can manually move it into place — rather than silent
    // data loss. See the module doc and THREAT_MODEL.md for this residual
    // risk, in the same spirit as the signing-to-anchoring gap already
    // documented for decision attestations.
    if let Err(e) = std::fs::rename(&staging_path, master_key_path.as_str()) {
        tracing::error!(
            staging_path = %staging_path,
            master_key_path = %master_key_path.as_str(),
            error = %e,
            "MASTER KEY ROTATION: DB committed under the new key but renaming the staging \
             file to the real master key path failed. The new key is at {staging_path} — \
             move it to {master_key_path} manually before restarting HSIP, or it will boot \
             with the old key and be unable to decrypt any identity."
        );
        return Err(ApiError::Internal(format!(
            "DB rotation committed, but persisting the new key file failed: {e}. \
             Manual recovery required — see server logs."
        )));
    }

    *master_key_guard = new_key.clone();
    drop(master_key_guard);

    let now = now_ms();
    for row in &identity_rows {
        let row_tenant_id: String = row.try_get(0)?;
        let _ = crate::audit_log::record(
            &state.db,
            &row_tenant_id,
            "master_key.rotated",
            None,
            Some(&format!(
                "old={} new={}",
                fingerprint(&old_key),
                fingerprint(&new_key)
            )),
            now,
        )
        .await;
    }

    metrics::MASTER_KEY_ROTATIONS.inc();
    tracing::warn!(
        old_fingerprint = %fingerprint(&old_key),
        new_fingerprint = %fingerprint(&new_key),
        identities_reencrypted,
        anchor_identity_reencrypted,
        "Master key rotated"
    );

    Ok(Json(RotateMasterKeyResponse {
        identities_reencrypted,
        anchor_identity_reencrypted,
        old_key_fingerprint: fingerprint(&old_key),
        new_key_fingerprint: fingerprint(&new_key),
        master_key_path: master_key_path.to_string(),
        note: "Back up the new master key file now. Any other process or secrets manager \
               entry holding the old key (e.g. HSIP_MASTER_KEY elsewhere) is now stale."
            .to_string(),
    }))
}
