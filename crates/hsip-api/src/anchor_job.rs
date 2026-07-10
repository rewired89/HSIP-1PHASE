//! Batches unanchored decisions into an RFC 6962 Merkle tree, signs the
//! root with HSIP's node-level anchor identity, and submits it to
//! OpenTimestamps. Runs on a "whichever comes first" cadence: every batch
//! that reaches `BATCH_SIZE_TRIGGER` decisions, or every `INTERVAL_TRIGGER`
//! with at least one unanchored decision waiting — whichever happens
//! sooner. That cadence bounds how long a signed-but-unanchored decision
//! can be silently deleted or reordered by whoever controls this HSIP
//! instance, without needing a real-time anchor per decision.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    anchor,
    db::{now_ms, Db},
    key_encryption::{decrypt_signing_key, encrypt_signing_key},
    metrics,
};
use hsip_core::merkle::MerkleTree;

/// At least this many unanchored decisions triggers an immediate anchor
/// cycle regardless of how long it's been since the last one.
pub const BATCH_SIZE_TRIGGER: i64 = 50;
/// At least this long since the last anchor, with at least one unanchored
/// decision waiting, also triggers a cycle.
pub const INTERVAL_TRIGGER_MS: i64 = 5 * 60 * 1000;
/// Upper bound on decisions folded into a single tree, so one very bursty
/// tenant can't make a batch grow without bound.
const MAX_BATCH_SIZE: i64 = 2000;

/// Load the node-level anchor identity, creating it on first use. Distinct
/// from any tenant's identity — see the `anchor_identity` migration in
/// `db.rs` for why anchoring can't be attributed to one tenant's key.
async fn load_or_create_anchor_identity(db: &Db, master_key: &[u8]) -> anyhow::Result<SigningKey> {
    if let Some(row) = sqlx::query("SELECT signing_key_b64 FROM anchor_identity WHERE id = 1")
        .fetch_optional(db)
        .await?
    {
        let encrypted_b64: String = row.try_get(0)?;
        let key_bytes = decrypt_signing_key(&encrypted_b64, master_key)?;
        return Ok(SigningKey::from_bytes(&key_bytes));
    }

    let signing_key = SigningKey::generate(&mut OsRng);
    let verify_key = signing_key.verifying_key();
    let encrypted_b64 = encrypt_signing_key(&signing_key.to_bytes(), master_key);
    let verify_b64 = BASE64.encode(verify_key.to_bytes());
    let now = now_ms();

    // Two anchor cycles racing to create the row both attempt this insert;
    // only one may win. Whoever loses re-reads the winner's row instead of
    // failing the whole cycle.
    let inserted = sqlx::query(
        "INSERT INTO anchor_identity (id, signing_key_b64, verify_key_b64, created_at)
         SELECT 1, ?, ?, ?
         WHERE NOT EXISTS (SELECT 1 FROM anchor_identity WHERE id = 1)",
    )
    .bind(&encrypted_b64)
    .bind(&verify_b64)
    .bind(now)
    .execute(db)
    .await?;

    if inserted.rows_affected() == 1 {
        return Ok(signing_key);
    }

    let row = sqlx::query("SELECT signing_key_b64 FROM anchor_identity WHERE id = 1")
        .fetch_one(db)
        .await?;
    let encrypted_b64: String = row.try_get(0)?;
    let key_bytes = decrypt_signing_key(&encrypted_b64, master_key)?;
    Ok(SigningKey::from_bytes(&key_bytes))
}

/// Summary of one anchor cycle, for logging.
#[derive(Debug)]
pub struct AnchorSummary {
    pub anchor_id: String,
    pub leaf_count: usize,
    pub ots_status: String,
}

/// Run one anchor cycle against the default public OpenTimestamps calendars.
pub async fn run_anchor_cycle(db: &Db, master_key: &[u8]) -> anyhow::Result<Option<AnchorSummary>> {
    run_anchor_cycle_with_calendars(db, master_key, anchor::DEFAULT_CALENDARS).await
}

/// Run one anchor cycle: retry any previously-unreachable OTS submissions,
/// then check whether a new batch is due and anchor it if so.
///
/// Returns `Ok(None)` when there was nothing to do (no unanchored
/// decisions, or the cadence hasn't elapsed and the batch isn't big enough
/// yet) — that is the common case on every tick, not an error.
///
/// `calendars` is threaded through explicitly (rather than always using
/// [`anchor::DEFAULT_CALENDARS`]) so tests can point this at a local mock
/// server instead of making a real, possibly network-policy-blocked call to
/// the public OpenTimestamps calendars on every test run.
pub async fn run_anchor_cycle_with_calendars(
    db: &Db,
    master_key: &[u8],
    calendars: &[&str],
) -> anyhow::Result<Option<AnchorSummary>> {
    retry_pending_ots_submissions(db, calendars).await;

    let unanchored_count: i64 =
        sqlx::query("SELECT COUNT(*) FROM decisions WHERE anchor_id IS NULL")
            .fetch_one(db)
            .await?
            .try_get(0)?;

    if unanchored_count == 0 {
        return Ok(None);
    }

    let last_anchor_ms: i64 =
        sqlx::query("SELECT COALESCE(MAX(created_at), 0) FROM decision_anchors")
            .fetch_one(db)
            .await?
            .try_get(0)?;
    let now = now_ms();
    let due_by_size = unanchored_count >= BATCH_SIZE_TRIGGER;
    let due_by_time = now - last_anchor_ms >= INTERVAL_TRIGGER_MS;

    if !due_by_size && !due_by_time {
        return Ok(None);
    }

    let rows = sqlx::query(
        "SELECT id, event_hash FROM decisions WHERE anchor_id IS NULL
         ORDER BY created_at ASC LIMIT ?",
    )
    .bind(MAX_BATCH_SIZE)
    .fetch_all(db)
    .await?;

    if rows.is_empty() {
        return Ok(None);
    }

    let mut ids = Vec::with_capacity(rows.len());
    let mut leaves = Vec::with_capacity(rows.len());
    for row in &rows {
        let id: String = row.try_get(0)?;
        let event_hash_hex: String = row.try_get(1)?;
        let leaf = hex::decode(&event_hash_hex)
            .map_err(|_| anyhow::anyhow!("corrupt event_hash in DB for decision {id}"))?;
        ids.push(id);
        leaves.push(leaf);
    }

    let tree = MerkleTree::from_leaves(&leaves);
    let root = tree.root();

    let anchor_signing_key = load_or_create_anchor_identity(db, master_key).await?;
    let anchor_verify_b64 = BASE64.encode(anchor_signing_key.verifying_key().to_bytes());
    let anchor_signature = anchor_signing_key.sign(&root);
    let anchor_signature_b64 = BASE64.encode(anchor_signature.to_bytes());

    let (ots_proof, ots_status) = match anchor::submit_digest_to(calendars, &root).await {
        Ok(receipt) => {
            tracing::debug!(calendar = %receipt.calendar_url, "OpenTimestamps submission accepted");
            (Some(receipt.response_bytes), "pending".to_string())
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "OpenTimestamps submission failed for this anchor batch; \
                 local Merkle anchoring proceeds, external anchoring will retry next cycle"
            );
            (None, "calendar_unreachable".to_string())
        }
    };

    let anchor_id = Uuid::new_v4().to_string();
    let now = now_ms();

    sqlx::query(
        "INSERT INTO decision_anchors
         (id, merkle_root, leaf_count, anchor_signature, anchor_verify_key, ots_proof, ots_status, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&anchor_id)
    .bind(hex::encode(root))
    .bind(ids.len() as i64)
    .bind(&anchor_signature_b64)
    .bind(&anchor_verify_b64)
    .bind(&ots_proof)
    .bind(&ots_status)
    .bind(now)
    .execute(db)
    .await?;

    for (index, id) in ids.iter().enumerate() {
        sqlx::query("UPDATE decisions SET anchor_id = ?, merkle_index = ? WHERE id = ?")
            .bind(&anchor_id)
            .bind(index as i64)
            .bind(id)
            .execute(db)
            .await?;
    }

    // One audit entry per distinct tenant touched by this batch — anchoring
    // is a system-level operation, but audit_entries is scoped per tenant.
    let tenant_rows = sqlx::query("SELECT DISTINCT tenant_id FROM decisions WHERE anchor_id = ?")
        .bind(&anchor_id)
        .fetch_all(db)
        .await?;
    for row in &tenant_rows {
        let tenant_id: String = row.try_get(0)?;
        let aid = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO audit_entries (id, tenant_id, action, details, timestamp)
             VALUES (?, ?, 'decision.anchored', ?, ?)",
        )
        .bind(&aid)
        .bind(&tenant_id)
        .bind(&anchor_id)
        .bind(now)
        .execute(db)
        .await?;
    }

    metrics::DECISIONS_ANCHORED
        .with_label_values(&[&ots_status])
        .inc();

    Ok(Some(AnchorSummary {
        anchor_id,
        leaf_count: ids.len(),
        ots_status,
    }))
}

/// Retry OpenTimestamps submission for anchors whose local Merkle root was
/// already committed but whose external calendar submission previously
/// failed. Best-effort — logs and moves on if a retry fails again.
async fn retry_pending_ots_submissions(db: &Db, calendars: &[&str]) {
    let rows = match sqlx::query(
        "SELECT id, merkle_root FROM decision_anchors WHERE ots_status = 'calendar_unreachable'",
    )
    .fetch_all(db)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "failed to query anchors pending OTS retry");
            return;
        }
    };

    for row in &rows {
        let Ok(anchor_id) = row.try_get::<String, _>(0) else {
            continue;
        };
        let Ok(root_hex) = row.try_get::<String, _>(1) else {
            continue;
        };
        let Ok(root_bytes) = hex::decode(&root_hex) else {
            continue;
        };
        let Ok(root): Result<[u8; 32], _> = root_bytes.try_into() else {
            continue;
        };

        match anchor::submit_digest_to(calendars, &root).await {
            Ok(receipt) => {
                let _ = sqlx::query(
                    "UPDATE decision_anchors SET ots_proof = ?, ots_status = 'pending' WHERE id = ?",
                )
                .bind(&receipt.response_bytes)
                .bind(&anchor_id)
                .execute(db)
                .await;
                tracing::info!(anchor_id = %anchor_id, "OpenTimestamps retry succeeded");
            }
            Err(e) => {
                tracing::debug!(anchor_id = %anchor_id, error = %e, "OpenTimestamps retry still failing");
            }
        }
    }
}

/// Verify a node-level anchor signature over a Merkle root — used by
/// `routes/decisions.rs::verify` and available for anyone re-implementing
/// verification independently.
#[must_use]
pub fn verify_anchor_signature(
    root: &[u8; 32],
    signature: &[u8; 64],
    verify_key: &[u8; 32],
) -> bool {
    let Ok(vk) = VerifyingKey::from_bytes(verify_key) else {
        return false;
    };
    let sig = Signature::from_bytes(signature);
    vk.verify(root, &sig).is_ok()
}
