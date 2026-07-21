//! Node-level administrative operations that are not scoped to a single
//! tenant — master key rotation, and managing who else can do it. Distinct
//! from every other route in this file tree, which all operate within one
//! tenant's data.
//!
//! Root-admin is an explicit, grantable flag (`api_keys.is_root_admin`),
//! not tied to any particular tenant or key name. It replaces an earlier
//! model where the only root admin was "the key named `admin` in the first
//! tenant ever created" — a single hardcoded credential with no way to add
//! a second one short of editing the database by hand. The bootstrap admin
//! key still gets `is_root_admin=1` automatically (see
//! `main.rs::bootstrap_admin` and `db.rs`'s upgrade backfill), but any
//! existing root admin can now grant or revoke the flag on other keys via
//! `POST /v1/admin/root-admins/grant` / `.../revoke` below. This is still
//! not a full RBAC/permissions system — root-admin is a single flat
//! capability covering every node-level operation, not scoped grants — but
//! it removes the single-hardcoded-credential limitation. See
//! THREAT_MODEL.md for the residual "one flat capability, not scoped
//! roles" tradeoff.

use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};
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

async fn require_root_admin(db: &crate::db::Db, headers: &HeaderMap) -> ApiResult<()> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .ok_or_else(|| ApiError::Unauthorized("Missing Authorization: Bearer <key>".into()))?;
    let key_hash = hash_key(token);

    let key_row =
        sqlx::query("SELECT is_root_admin FROM api_keys WHERE key_hash = $1 AND active = 1")
            .bind(&key_hash)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| ApiError::Internal("authenticated key vanished mid-request".into()))?;
    let is_root_admin: i64 = key_row.try_get(0)?;

    if is_root_admin == 0 {
        return Err(ApiError::Unauthorized(
            "This operation is restricted to a root-admin key. An existing root admin can \
             grant this key access via POST /v1/admin/root-admins/grant."
                .into(),
        ));
    }
    Ok(())
}

fn fingerprint(key_bytes: &[u8]) -> String {
    let digest = Sha256::digest(key_bytes);
    hex::encode(&digest[..8])
}

/// Where a rotated key can be durably persisted. Resolved once at the start
/// of rotation so the whole operation either has somewhere to put the new
/// key or refuses before touching the database at all.
enum KeyPersistence {
    /// File this process owns and can rewrite (the common case).
    File(String),
    /// `HSIP_MASTER_KEY`-sourced deployments have no such file. If
    /// `HSIP_ROTATION_HOOK` names an executable, that script is invoked
    /// with the new key on stdin and is responsible for writing it
    /// wherever the operator's secrets manager actually lives (Vault, AWS
    /// Secrets Manager/KMS, ...). HSIP itself never holds credentials for
    /// any of those — the hook is the operator's own trusted tooling,
    /// using whatever auth it already has, not a new secret HSIP manages.
    Hook(String),
}

fn resolve_persistence(state: &AppState) -> Option<KeyPersistence> {
    if let Some(path) = state.master_key_path.as_ref() {
        return Some(KeyPersistence::File(path.to_string()));
    }
    let hook = std::env::var("HSIP_ROTATION_HOOK").ok()?;
    let hook = hook.trim();
    if hook.is_empty() {
        return None;
    }
    Some(KeyPersistence::Hook(hook.to_string()))
}

const ROTATION_HOOK_TIMEOUT_SECS: u64 = 30;

/// Invokes the configured rotation hook with the new key hex-encoded on
/// stdin (never as a CLI argument — those are visible in `ps`/process
/// listings on some systems). The hook's exit code is the only signal we
/// trust: non-zero, or a timeout, means the write did not happen and
/// rotation must not proceed. Old/new fingerprints (safe to expose — see
/// `fingerprint()`) are passed as env vars so the hook can tag/log the
/// secrets-manager entry without recomputing them.
async fn run_rotation_hook(hook_path: &str, old_key: &[u8], new_key: &[u8]) -> anyhow::Result<()> {
    use tokio::io::AsyncWriteExt;
    use tokio::process::Command;

    let mut child = Command::new(hook_path)
        .env("HSIP_ROTATION_OLD_FINGERPRINT", fingerprint(old_key))
        .env("HSIP_ROTATION_NEW_FINGERPRINT", fingerprint(new_key))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn rotation hook {hook_path}: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(hex::encode(new_key).as_bytes())
            .await
            .map_err(|e| anyhow::anyhow!("failed to write new key to rotation hook stdin: {e}"))?;
        // `stdin` drops here, closing the pipe so the hook sees EOF.
    }

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(ROTATION_HOOK_TIMEOUT_SECS),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "rotation hook {hook_path} did not finish within {ROTATION_HOOK_TIMEOUT_SECS}s"
        )
    })?
    .map_err(|e| anyhow::anyhow!("failed to run rotation hook {hook_path}: {e}"))?;

    if !output.status.success() {
        // Capped — a misbehaving hook's stderr should not be able to fill
        // logs or the HTTP error response unbounded.
        let stderr_tail: String = String::from_utf8_lossy(&output.stderr)
            .chars()
            .take(2000)
            .collect();
        anyhow::bail!(
            "rotation hook {hook_path} exited with {} — stderr: {stderr_tail}",
            output.status
        );
    }
    Ok(())
}

#[derive(Serialize)]
pub struct MasterKeyFingerprintResponse {
    pub fingerprint: String,
    /// `None` when the key is sourced from `HSIP_MASTER_KEY` rather than a
    /// file — matches `RotateMasterKeyResponse` (and rotation's own
    /// refusal) in treating that as "no file this process owns."
    pub master_key_path: Option<String>,
    /// Whether `POST /v1/admin/master-key/rotate` currently has anywhere
    /// to durably persist a new key — `true` when file-backed, or when
    /// `HSIP_ROTATION_HOOK` is set for an `HSIP_MASTER_KEY`-sourced
    /// deployment. `false` means rotation will refuse to run.
    pub rotation_available: bool,
}

/// `GET /v1/admin/system-health` — read-only, root-admin gated. Aggregates
/// conditions the rest of this codebase can detect but cannot fix itself —
/// see `system_health.rs` for what's checked and why. Exists because "can
/// this recover automatically?" has real "no" answers in this codebase (an
/// incomplete master key rotation, a permanently root-admin-less node), and
/// HSIP has no push-based alerting of its own — an operator, whether that's
/// one person on a desktop or a business running real monitoring, would
/// otherwise only discover these states by reading the database directly.
pub async fn system_health(
    State(state): State<AppState>,
    _tenant: TenantId,
    headers: HeaderMap,
) -> ApiResult<Json<crate::system_health::SystemHealth>> {
    require_root_admin(&state.db, &headers).await?;
    let master_key_path = state.master_key_path.as_ref().map(|p| p.as_str());
    Ok(Json(
        crate::system_health::check_and_update_metrics(&state.db, master_key_path).await,
    ))
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
    // Extractor still runs auth/rate-limit/replay checks even though the
    // resolved tenant_id itself is unused now that require_root_admin
    // checks the caller's is_root_admin flag directly instead.
    _tenant: TenantId,
    headers: HeaderMap,
) -> ApiResult<Json<MasterKeyFingerprintResponse>> {
    require_root_admin(&state.db, &headers).await?;

    let key = state.master_key.read().await;
    Ok(Json(MasterKeyFingerprintResponse {
        fingerprint: fingerprint(&key),
        master_key_path: state.master_key_path.as_ref().map(|p| p.to_string()),
        rotation_available: resolve_persistence(&state).is_some(),
    }))
}

#[derive(Serialize)]
pub struct RotateMasterKeyResponse {
    pub identities_reencrypted: u64,
    pub anchor_identity_reencrypted: bool,
    pub old_key_fingerprint: String,
    pub new_key_fingerprint: String,
    /// Set when the key is file-backed; `None` when persisted via a
    /// rotation hook instead.
    pub master_key_path: Option<String>,
    /// Set when persistence went through `HSIP_ROTATION_HOOK`; `None` when
    /// file-backed.
    pub rotation_hook: Option<String>,
    pub note: String,
}

/// `POST /v1/admin/master-key/rotate` — generates a new master key,
/// re-encrypts every tenant's `identities.signing_key_b64` and the
/// singleton `anchor_identity` row under it in one DB transaction, then
/// durably persists the new key and finally swaps it into memory for every
/// subsequent request.
///
/// Persistence is one of two modes, resolved once up front by
/// `resolve_persistence` (see `KeyPersistence`): a file-backed key gets a
/// staging file + atomic rename (a crash mid-write leaves either the old
/// file untouched or the new file fully written — never half-written); an
/// `HSIP_MASTER_KEY`-sourced key with `HSIP_ROTATION_HOOK` set instead hands
/// the new key to that script on stdin and trusts its exit code. Either way
/// the DB transaction only commits *after* persistence succeeds, so a
/// failure at that step (bad file permissions, hook exits non-zero, hook
/// times out) leaves the database completely untouched.
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
    _tenant: TenantId,
    headers: HeaderMap,
) -> ApiResult<Json<RotateMasterKeyResponse>> {
    require_root_admin(&state.db, &headers).await?;

    let Some(persistence) = resolve_persistence(&state) else {
        return Err(ApiError::BadRequest(
            "Master key is sourced from HSIP_MASTER_KEY, not a file this process can rewrite, \
             and no HSIP_ROTATION_HOOK is configured to hand the new key to your secrets \
             manager. Either set HSIP_ROTATION_HOOK to a script that writes the new key \
             (received hex-encoded on stdin) to wherever HSIP_MASTER_KEY is sourced from, or \
             rotate the value at its source manually and restart HSIP."
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

        sqlx::query("UPDATE identities SET signing_key_b64 = $1 WHERE tenant_id = $2")
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
        sqlx::query("UPDATE anchor_identity SET signing_key_b64 = $1 WHERE id = 1")
            .bind(&re_encrypted)
            .execute(&mut *tx)
            .await?;
        true
    } else {
        false
    };

    // Persist the new key *before* committing the DB transaction. If this
    // fails, returning here drops `tx` unconsumed, which rolls it back —
    // nothing changed. This is the step that differs by persistence mode:
    let staging_path = match &persistence {
        KeyPersistence::File(path) => {
            // Write to a staging file on the same filesystem as the real
            // path. If the process crashes after this succeeds but before
            // the commit below, the DB is still under the old key and the
            // real key file is untouched (only the staging file has the
            // new key) — safe, just requires re-running rotation.
            let staging_path = format!("{path}.rotating");
            use std::io::Write;
            let mut f = std::fs::File::create(&staging_path).map_err(|e| {
                ApiError::Internal(format!("failed to write staging key file: {e}"))
            })?;
            f.write_all(hex::encode(&new_key).as_bytes())
                .and_then(|_| f.sync_all())
                .map_err(|e| {
                    ApiError::Internal(format!("failed to write staging key file: {e}"))
                })?;
            // `File::create` leaves the staging file at whatever the
            // process umask allows (0644 — world-readable — on any
            // default Unix umask), and `rename()` below preserves the
            // *source* file's mode bits, not the destination's. Without
            // this, rotating the master key would silently downgrade its
            // on-disk permissions back to world-readable even if an
            // operator had correctly `chmod 600`'d the original file
            // themselves — see `config.rs::desktop_defaults`'s identical
            // fix for the initial-generation case.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                f.set_permissions(std::fs::Permissions::from_mode(0o600))
                    .map_err(|e| {
                        ApiError::Internal(format!(
                            "failed to restrict staging key file permissions: {e}"
                        ))
                    })?;
            }
            Some(staging_path)
        }
        KeyPersistence::Hook(hook_path) => {
            // The hook is the durable write itself (e.g. a `vault kv put`
            // call) — if it fails or times out, nothing has been written
            // anywhere durable yet, so aborting here (dropping `tx`
            // unconsumed) is fully safe.
            run_rotation_hook(hook_path, &old_key, &new_key)
                .await
                .map_err(|e| {
                    ApiError::Internal(format!("rotation hook failed, no changes made: {e}"))
                })?;
            None
        }
    };

    tx.commit().await?;

    // File mode only: narrowest possible risk window left. If the process
    // dies between the commit above and the rename below, the DB now holds
    // ciphertext under the new key but the real key file still has the old
    // one. The staging file is left in place specifically so that window
    // is recoverable — an operator can manually move it into place —
    // rather than silent data loss. Hook mode has no equivalent window:
    // the hook call above *was* the durable write, already done pre-commit.
    if let (KeyPersistence::File(path), Some(staging_path)) = (&persistence, &staging_path) {
        if let Err(e) = std::fs::rename(staging_path, path) {
            tracing::error!(
                staging_path = %staging_path,
                master_key_path = %path,
                error = %e,
                "MASTER KEY ROTATION: DB committed under the new key but renaming the staging \
                 file to the real master key path failed. The new key is at {staging_path} — \
                 move it to {path} manually before restarting HSIP, or it will boot with the \
                 old key and be unable to decrypt any identity."
            );
            return Err(ApiError::Internal(format!(
                "DB rotation committed, but persisting the new key file failed: {e}. \
                 Manual recovery required — see server logs."
            )));
        }
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

    let (master_key_path_out, rotation_hook_out, note) = match &persistence {
        KeyPersistence::File(path) => (
            Some(path.clone()),
            None,
            "Back up the new master key file now. Any other process or secrets manager \
             entry holding the old key (e.g. HSIP_MASTER_KEY elsewhere) is now stale."
                .to_string(),
        ),
        KeyPersistence::Hook(hook) => (
            None,
            Some(hook.clone()),
            format!(
                "The new key was handed to {hook}, which reported success. Confirm it landed \
                 wherever your secrets manager expects it — HSIP has no visibility past the \
                 hook's exit code. Any other process still reading the old HSIP_MASTER_KEY \
                 value is now stale."
            ),
        ),
    };

    Ok(Json(RotateMasterKeyResponse {
        identities_reencrypted,
        anchor_identity_reencrypted,
        old_key_fingerprint: fingerprint(&old_key),
        new_key_fingerprint: fingerprint(&new_key),
        master_key_path: master_key_path_out,
        rotation_hook: rotation_hook_out,
        note,
    }))
}

// ── Root-admin management ───────────────────────────────────────────────────
//
// is_root_admin is a node-wide flag, not scoped to any one tenant — the
// key being granted/revoked/listed can belong to any tenant, not just the
// caller's own. Every handler here still requires TenantId as an extractor
// (for auth/rate-limit/replay enforcement) even though the resolved
// tenant_id itself is unused.

#[derive(Deserialize)]
pub struct RootAdminKeyRequest {
    pub key_id: String,
}

#[derive(Serialize)]
pub struct RootAdminRecord {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub created_at: i64,
}

async fn root_admin_count(db: &crate::db::Db) -> ApiResult<i64> {
    let row = sqlx::query("SELECT COUNT(*) FROM api_keys WHERE is_root_admin = 1 AND active = 1")
        .fetch_one(db)
        .await?;
    Ok(row.try_get(0)?)
}

/// `GET /v1/admin/root-admins` — lists every active key currently holding
/// the root-admin flag, across all tenants. Root-admin-gated so this
/// doesn't leak "who holds node-level authority" to anyone but the people
/// who already have it.
pub async fn list_root_admins(
    State(state): State<AppState>,
    _tenant: TenantId,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<RootAdminRecord>>> {
    require_root_admin(&state.db, &headers).await?;

    let rows = sqlx::query(
        "SELECT id, tenant_id, name, created_at FROM api_keys
         WHERE is_root_admin = 1 AND active = 1 ORDER BY created_at ASC",
    )
    .fetch_all(&state.db)
    .await?;

    let out = rows
        .iter()
        .map(|r| -> Result<RootAdminRecord, sqlx::Error> {
            Ok(RootAdminRecord {
                id: r.try_get(0)?,
                tenant_id: r.try_get(1)?,
                name: r.try_get(2)?,
                created_at: r.try_get(3)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(out))
}

/// `POST /v1/admin/root-admins/grant` — an existing root admin grants the
/// flag to another active key (by id, any tenant). This is the mechanism
/// that makes it possible to have more than one root admin at all — before
/// this existed, the only root admin was whichever key `bootstrap_admin`
/// created on first boot, with no way to add a second short of editing the
/// database by hand.
pub async fn grant_root_admin(
    State(state): State<AppState>,
    _tenant: TenantId,
    headers: HeaderMap,
    Json(req): Json<RootAdminKeyRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    require_root_admin(&state.db, &headers).await?;

    let row = sqlx::query("SELECT tenant_id, active FROM api_keys WHERE id = $1")
        .bind(&req.key_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Key {} not found", req.key_id)))?;
    let target_tenant: String = row.try_get(0)?;
    let active: i64 = row.try_get(1)?;
    if active == 0 {
        return Err(ApiError::BadRequest(
            "Cannot grant root-admin to a revoked key.".into(),
        ));
    }

    sqlx::query("UPDATE api_keys SET is_root_admin = 1 WHERE id = $1")
        .bind(&req.key_id)
        .execute(&state.db)
        .await?;

    let now = now_ms();
    let _ = crate::audit_log::record(
        &state.db,
        &target_tenant,
        "admin.root_admin_granted",
        None,
        Some(&format!("key_id={}", req.key_id)),
        now,
    )
    .await;
    metrics::ROOT_ADMIN_CHANGES
        .with_label_values(&["granted"])
        .inc();

    Ok(Json(
        serde_json::json!({ "granted": req.key_id, "tenant_id": target_tenant }),
    ))
}

/// `POST /v1/admin/root-admins/revoke` — refuses if this would leave zero
/// root admins on the node (would lock every tenant out of master key
/// rotation with no way to recover except editing the database by hand —
/// the exact failure mode granting a second root admin exists to avoid).
pub async fn revoke_root_admin(
    State(state): State<AppState>,
    _tenant: TenantId,
    headers: HeaderMap,
    Json(req): Json<RootAdminKeyRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    require_root_admin(&state.db, &headers).await?;

    let row = sqlx::query("SELECT tenant_id, is_root_admin FROM api_keys WHERE id = $1")
        .bind(&req.key_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Key {} not found", req.key_id)))?;
    let target_tenant: String = row.try_get(0)?;
    let is_admin: i64 = row.try_get(1)?;
    if is_admin == 0 {
        return Err(ApiError::BadRequest("Key is not a root admin.".into()));
    }

    if root_admin_count(&state.db).await? <= 1 {
        return Err(ApiError::Conflict(
            "Cannot revoke the last root admin — this would lock every tenant out of \
             node-level operations (master key rotation) with no way to recover except \
             editing the database directly."
                .into(),
        ));
    }

    sqlx::query("UPDATE api_keys SET is_root_admin = 0 WHERE id = $1")
        .bind(&req.key_id)
        .execute(&state.db)
        .await?;

    let now = now_ms();
    let _ = crate::audit_log::record(
        &state.db,
        &target_tenant,
        "admin.root_admin_revoked",
        None,
        Some(&format!("key_id={}", req.key_id)),
        now,
    )
    .await;
    metrics::ROOT_ADMIN_CHANGES
        .with_label_values(&["revoked"])
        .inc();

    Ok(Json(
        serde_json::json!({ "revoked": req.key_id, "tenant_id": target_tenant }),
    ))
}
