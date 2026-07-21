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
use sqlx::any::AnyRow;
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
         SELECT 1, $1, $2, $3
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
         ORDER BY created_at ASC LIMIT $1",
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
            metrics::ANCHOR_CALENDAR_UNREACHABLE.inc();
            (None, "calendar_unreachable".to_string())
        }
    };

    let anchor_id = Uuid::new_v4().to_string();
    let now = now_ms();

    sqlx::query(
        "INSERT INTO decision_anchors
         (id, merkle_root, leaf_count, anchor_signature, anchor_verify_key, ots_proof, ots_status, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
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
        sqlx::query("UPDATE decisions SET anchor_id = $1, merkle_index = $2 WHERE id = $3")
            .bind(&anchor_id)
            .bind(index as i64)
            .bind(id)
            .execute(db)
            .await?;
    }

    // One audit entry per distinct tenant touched by this batch — anchoring
    // is a system-level operation, but audit_entries is scoped per tenant.
    let tenant_rows = sqlx::query("SELECT DISTINCT tenant_id FROM decisions WHERE anchor_id = $1")
        .bind(&anchor_id)
        .fetch_all(db)
        .await?;
    for row in &tenant_rows {
        let tenant_id: String = row.try_get(0)?;
        crate::audit_log::record(
            db,
            &tenant_id,
            "decision.anchored",
            None,
            Some(&anchor_id),
            now,
        )
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
                match sqlx::query(
                    "UPDATE decision_anchors SET ots_proof = $1, ots_status = 'pending' WHERE id = $2",
                )
                .bind(&receipt.response_bytes)
                .bind(&anchor_id)
                .execute(db)
                .await
                {
                    Ok(result) if result.rows_affected() > 0 => {
                        tracing::info!(anchor_id = %anchor_id, "OpenTimestamps retry succeeded");
                    }
                    Ok(_) => {
                        tracing::warn!(
                            anchor_id = %anchor_id,
                            "OpenTimestamps retry got a calendar response but the DB update \
                             affected zero rows (row deleted concurrently?) — not counted as success"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            anchor_id = %anchor_id,
                            error = %e,
                            "OpenTimestamps retry got a calendar response but failed to persist \
                             it — will retry again next cycle, row is still 'calendar_unreachable'"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::debug!(anchor_id = %anchor_id, error = %e, "OpenTimestamps retry still failing");
                metrics::ANCHOR_CALENDAR_UNREACHABLE.inc();
            }
        }
    }
}

/// Run one audit-log anchor cycle against the default public OpenTimestamps
/// calendars. Same shape as [`run_anchor_cycle`], batching `audit_entries`
/// instead of `decisions` — see the module-level comparison in
/// `audit_log.rs` for why this exists (THREAT_MODEL.md §4.8's "chain isn't
/// anchored outside this database" gap).
pub async fn run_audit_anchor_cycle(
    db: &Db,
    master_key: &[u8],
) -> anyhow::Result<Option<AnchorSummary>> {
    run_audit_anchor_cycle_with_calendars(db, master_key, anchor::DEFAULT_CALENDARS).await
}

/// Run one audit-log anchor cycle: retry any previously-unreachable OTS
/// submissions for audit batches, then check whether a new batch is due.
///
/// Only entries that already participate in the BLAKE3 hash chain
/// (`entry_hash IS NOT NULL`) are eligible — rows written before that chain
/// existed have nothing to commit to a Merkle leaf and are excluded, the
/// same way `audit_log::verify_chain` counts them as `unchained` rather
/// than treating them as a break.
///
/// `calendars` is threaded through explicitly for the same testing reason
/// as [`run_anchor_cycle_with_calendars`].
pub async fn run_audit_anchor_cycle_with_calendars(
    db: &Db,
    master_key: &[u8],
    calendars: &[&str],
) -> anyhow::Result<Option<AnchorSummary>> {
    retry_pending_audit_ots_submissions(db, calendars).await;

    let unanchored_count: i64 = sqlx::query(
        "SELECT COUNT(*) FROM audit_entries WHERE anchor_id IS NULL AND entry_hash IS NOT NULL",
    )
    .fetch_one(db)
    .await?
    .try_get(0)?;

    if unanchored_count == 0 {
        return Ok(None);
    }

    let last_anchor_ms: i64 = sqlx::query("SELECT COALESCE(MAX(created_at), 0) FROM audit_anchors")
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
        "SELECT id, entry_hash FROM audit_entries WHERE anchor_id IS NULL AND entry_hash IS NOT NULL
         ORDER BY timestamp ASC LIMIT $1",
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
        let entry_hash_hex: String = row.try_get(1)?;
        let leaf = hex::decode(&entry_hash_hex)
            .map_err(|_| anyhow::anyhow!("corrupt entry_hash in DB for audit entry {id}"))?;
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
            tracing::debug!(
                calendar = %receipt.calendar_url,
                "OpenTimestamps submission accepted (audit-log batch)"
            );
            (Some(receipt.response_bytes), "pending".to_string())
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "OpenTimestamps submission failed for this audit-log anchor batch; \
                 local Merkle anchoring proceeds, external anchoring will retry next cycle"
            );
            metrics::ANCHOR_CALENDAR_UNREACHABLE.inc();
            (None, "calendar_unreachable".to_string())
        }
    };

    let anchor_id = Uuid::new_v4().to_string();
    let now = now_ms();

    sqlx::query(
        "INSERT INTO audit_anchors
         (id, merkle_root, leaf_count, anchor_signature, anchor_verify_key, ots_proof, ots_status, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
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
        sqlx::query("UPDATE audit_entries SET anchor_id = $1, merkle_index = $2 WHERE id = $3")
            .bind(&anchor_id)
            .bind(index as i64)
            .bind(id)
            .execute(db)
            .await?;
    }

    // One audit entry per distinct tenant touched by this batch — mirrors
    // decisions.anchored below. Naturally becomes part of a future audit
    // anchor batch itself; this does not recurse within the same cycle
    // since the UPDATE above already ran before this INSERT.
    let tenant_rows =
        sqlx::query("SELECT DISTINCT tenant_id FROM audit_entries WHERE anchor_id = $1")
            .bind(&anchor_id)
            .fetch_all(db)
            .await?;
    for row in &tenant_rows {
        let tenant_id: String = row.try_get(0)?;
        crate::audit_log::record(
            db,
            &tenant_id,
            "audit.anchored",
            None,
            Some(&anchor_id),
            now,
        )
        .await?;
    }

    metrics::AUDIT_ANCHORED
        .with_label_values(&[&ots_status])
        .inc();

    Ok(Some(AnchorSummary {
        anchor_id,
        leaf_count: ids.len(),
        ots_status,
    }))
}

/// Retry OpenTimestamps submission for audit-log anchors whose local Merkle
/// root was already committed but whose external calendar submission
/// previously failed. Best-effort — logs and moves on if a retry fails
/// again. Twin of [`retry_pending_ots_submissions`] against `audit_anchors`
/// instead of `decision_anchors`.
async fn retry_pending_audit_ots_submissions(db: &Db, calendars: &[&str]) {
    let rows = match sqlx::query(
        "SELECT id, merkle_root FROM audit_anchors WHERE ots_status = 'calendar_unreachable'",
    )
    .fetch_all(db)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "failed to query audit anchors pending OTS retry");
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
                match sqlx::query(
                    "UPDATE audit_anchors SET ots_proof = $1, ots_status = 'pending' WHERE id = $2",
                )
                .bind(&receipt.response_bytes)
                .bind(&anchor_id)
                .execute(db)
                .await
                {
                    Ok(result) if result.rows_affected() > 0 => {
                        tracing::info!(anchor_id = %anchor_id, "OpenTimestamps retry succeeded (audit-log batch)");
                    }
                    Ok(_) => {
                        tracing::warn!(
                            anchor_id = %anchor_id,
                            "OpenTimestamps retry (audit-log batch) got a calendar response but \
                             the DB update affected zero rows — not counted as success"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            anchor_id = %anchor_id,
                            error = %e,
                            "OpenTimestamps retry (audit-log batch) got a calendar response but \
                             failed to persist it — will retry again next cycle"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::debug!(anchor_id = %anchor_id, error = %e, "OpenTimestamps retry still failing (audit-log batch)");
                metrics::ANCHOR_CALENDAR_UNREACHABLE.inc();
            }
        }
    }
}

/// Per-cycle cap on how many still-pending anchor rows to check for an
/// upgrade. Bounds one cycle's worst-case total network time (each check
/// has its own 15s timeout, so an unlucky cycle where every check times out
/// would otherwise scale with however many pending rows exist) safely under
/// the 15-minute gap between cycles, and lets a large backlog work through
/// gradually across multiple cycles (oldest first) rather than one cycle
/// trying to check everything at once.
const MAX_UPGRADE_CHECKS_PER_CYCLE: i64 = 25;

/// How long to keep actively polling a still-pending anchor before giving
/// up automatic upgrade checks for it. Real OpenTimestamps confirmations
/// normally land within hours under normal calendar operation; this is a
/// generous outer bound so a batch isn't checked, every 15 minutes, for the
/// rest of this server's operational lifetime if a calendar goes
/// permanently dark. A batch past this age is still fully intact — its
/// signature and Merkle proof still verify — it simply stops being
/// auto-upgraded; an operator can still check it against the calendar
/// directly if needed.
pub(crate) const MAX_PENDING_UPGRADE_AGE_MS: i64 = 7 * 24 * 60 * 60 * 1000; // 7 days

/// Poll calendars for anchor batches (both decisions and the audit log)
/// still sitting at `ots_status = 'pending'`, and upgrade any that have
/// since been confirmed by a mined Bitcoin block to `ots_status =
/// 'confirmed'`. Meant to run on its own, much slower timer than
/// `run_anchor_cycle`/`run_audit_anchor_cycle` — Bitcoin blocks land
/// roughly every 10 minutes on average, so polling every 10s like the
/// submission loop does would just hammer the calendars for no benefit.
/// See `main.rs` for the actual interval.
pub async fn run_upgrade_cycle(db: &Db) {
    upgrade_pending_decision_anchors(db).await;
    upgrade_pending_audit_anchors(db).await;
}

/// Check up to [`MAX_UPGRADE_CHECKS_PER_CYCLE`] `decision_anchors` rows
/// still at `ots_status = 'pending'` (oldest first) against the calendar
/// that originally accepted each one (read back out of the stored
/// `ots_proof` via `anchor::extract_pending_calendar_uri` — there's no
/// separate calendar-URL column, and that calendar is the only one with any
/// record of this submission to check). Best-effort, same shape as
/// `retry_pending_ots_submissions` above — a batch that's still pending, an
/// unparseable stored proof, or a calendar that's temporarily unreachable
/// all just mean "try again next cycle," not an error.
async fn upgrade_pending_decision_anchors(db: &Db) {
    let rows = match sqlx::query(
        "SELECT id, merkle_root, ots_proof, created_at FROM decision_anchors
         WHERE ots_status = 'pending' ORDER BY created_at ASC LIMIT $1",
    )
    .bind(MAX_UPGRADE_CHECKS_PER_CYCLE)
    .fetch_all(db)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "failed to query decision anchors pending OTS upgrade");
            return;
        }
    };

    for row in &rows {
        upgrade_one_anchor(db, "decision_anchors", row).await;
    }
}

/// Twin of [`upgrade_pending_decision_anchors`] against `audit_anchors`
/// instead of `decision_anchors`.
async fn upgrade_pending_audit_anchors(db: &Db) {
    let rows = match sqlx::query(
        "SELECT id, merkle_root, ots_proof, created_at FROM audit_anchors
         WHERE ots_status = 'pending' ORDER BY created_at ASC LIMIT $1",
    )
    .bind(MAX_UPGRADE_CHECKS_PER_CYCLE)
    .fetch_all(db)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "failed to query audit anchors pending OTS upgrade");
            return;
        }
    };

    for row in &rows {
        upgrade_one_anchor(db, "audit_anchors", row).await;
    }
}

/// Shared upgrade-check logic for one anchor row, used by both
/// [`upgrade_pending_decision_anchors`] and [`upgrade_pending_audit_anchors`].
/// `table` selects which table's row gets the `UPDATE` — always a hardcoded
/// literal from this module, never external input, so building the `UPDATE`
/// string with it carries no injection risk (same reasoning already applied
/// to `bin/hsip_migrate.rs`'s table-driven copy, just inline here instead of
/// via a second duplicated function per table).
async fn upgrade_one_anchor(db: &Db, table: &'static str, row: &AnyRow) {
    let Ok(anchor_id) = row.try_get::<String, _>(0) else {
        return;
    };
    let Ok(created_at) = row.try_get::<i64, _>(3) else {
        return;
    };
    if now_ms() - created_at > MAX_PENDING_UPGRADE_AGE_MS {
        tracing::debug!(
            anchor_id = %anchor_id,
            table,
            "anchor has been pending upgrade longer than MAX_PENDING_UPGRADE_AGE_MS; \
             no longer auto-checking it (still fully valid, just not auto-upgraded further)"
        );
        metrics::ANCHOR_UPGRADE_STALE.inc();
        return;
    }

    let Ok(root_hex) = row.try_get::<String, _>(1) else {
        return;
    };
    let Ok(root_bytes) = hex::decode(&root_hex) else {
        return;
    };
    let Ok(root): Result<[u8; 32], _> = root_bytes.try_into() else {
        return;
    };
    let Ok(Some(existing_proof)) = row.try_get::<Option<Vec<u8>>, _>(2) else {
        return;
    };
    let Some(calendar_url) = anchor::extract_pending_calendar_uri(&existing_proof) else {
        tracing::debug!(
            anchor_id = %anchor_id,
            table,
            "stored OTS proof has no recognizable calendar URI; skipping upgrade check"
        );
        return;
    };

    match anchor::check_for_upgrade(&calendar_url, &root).await {
        Ok(Some(proof_bytes)) if anchor::contains_bitcoin_attestation(&proof_bytes) => {
            let update_sql = format!(
                "UPDATE {table} SET ots_proof = $1, ots_status = 'confirmed' WHERE id = $2"
            );
            match sqlx::query(&update_sql)
                .bind(&proof_bytes)
                .bind(&anchor_id)
                .execute(db)
                .await
            {
                Ok(result) if result.rows_affected() > 0 => {
                    tracing::info!(
                        anchor_id = %anchor_id,
                        table,
                        "OpenTimestamps proof upgraded to Bitcoin-confirmed"
                    );
                    metrics::ANCHOR_UPGRADED_TO_CONFIRMED.inc();
                }
                Ok(_) => {
                    tracing::warn!(
                        anchor_id = %anchor_id,
                        table,
                        "calendar confirmed Bitcoin attestation but the DB update affected zero \
                         rows (row deleted concurrently?) — not counted as upgraded; will be \
                         re-checked next cycle if the row still exists and is still pending"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        anchor_id = %anchor_id,
                        table,
                        error = %e,
                        "calendar confirmed Bitcoin attestation but the DB update failed — not \
                         counted as upgraded, row is still 'pending' and will be re-checked next cycle"
                    );
                }
            }
        }
        Ok(Some(_)) => {
            tracing::debug!(
                anchor_id = %anchor_id,
                table,
                "calendar returned an update, but it isn't Bitcoin-confirmed yet"
            );
        }
        Ok(None) => {
            // Normal, common case: calendar has nothing new for this digest
            // yet — Bitcoin confirmation just hasn't happened, no logging
            // needed on every quiet cycle.
        }
        Err(e) => {
            tracing::debug!(
                anchor_id = %anchor_id,
                table,
                calendar = %calendar_url,
                error = %e,
                "OpenTimestamps upgrade check failed, will retry next cycle"
            );
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

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_db() -> Db {
        let db_url = format!(
            "sqlite:file:{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        sqlx::any::install_default_drivers();
        crate::db::init(&db_url).await.expect("db init")
    }

    fn build_pending_proof(calendar_uri: &str) -> Vec<u8> {
        let uri_bytes = calendar_uri.as_bytes();
        let mut proof = vec![0xAA, 0xBB, 0xCC];
        proof.extend_from_slice(&[0x83, 0xdf, 0xe3, 0x0d, 0x2e, 0xf9, 0x0c, 0x8e]); // PendingAttestation tag
        proof.push((uri_bytes.len() + 1) as u8);
        proof.push(uri_bytes.len() as u8);
        proof.extend_from_slice(uri_bytes);
        proof
    }

    /// A row that gets deleted out from under `upgrade_one_anchor` between
    /// being fetched and the `UPDATE` (simulating a concurrent delete, or
    /// standing in for any reason the `UPDATE` might legitimately affect
    /// zero rows) must NOT be counted as a successful upgrade — this is the
    /// exact bug found during a QA review: the original code discarded the
    /// `UPDATE`'s result with `let _ =` and logged/counted success
    /// unconditionally.
    #[tokio::test]
    async fn zero_rows_affected_is_not_counted_as_a_successful_upgrade() {
        let db = test_db().await;

        let mock_calendar = wiremock::MockServer::start().await;
        let calendar_uri = mock_calendar.uri();
        let pending_proof = build_pending_proof(&calendar_uri);

        let mut confirmed_proof = vec![0xAA, 0xBB, 0xCC];
        confirmed_proof.extend_from_slice(&[0x05, 0x88, 0x96, 0x0d, 0x73, 0xd7, 0x19, 0x01]); // BitcoinBlockHeaderAttestation tag
        confirmed_proof.extend_from_slice(b"...stand-in for real block header bytes...");

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path_regex(r"^/timestamp/.*"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_bytes(confirmed_proof))
            .mount(&mock_calendar)
            .await;

        let anchor_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO decision_anchors
             (id, merkle_root, leaf_count, anchor_signature, anchor_verify_key, ots_proof, ots_status, created_at)
             VALUES ($1, $2, 1, 'sig', 'verify_key', $3, 'pending', $4)",
        )
        .bind(&anchor_id)
        .bind(hex::encode([0x22u8; 32]))
        .bind(&pending_proof)
        .bind(now_ms())
        .execute(&db)
        .await
        .unwrap();

        // Fetch the row exactly like upgrade_pending_decision_anchors does,
        // then delete it — the in-memory `row` stays valid, but the later
        // UPDATE inside upgrade_one_anchor will now match zero rows.
        let row = sqlx::query(
            "SELECT id, merkle_root, ots_proof, created_at FROM decision_anchors WHERE id = $1",
        )
        .bind(&anchor_id)
        .fetch_one(&db)
        .await
        .unwrap();

        sqlx::query("DELETE FROM decision_anchors WHERE id = $1")
            .bind(&anchor_id)
            .execute(&db)
            .await
            .unwrap();

        let before = metrics::ANCHOR_UPGRADED_TO_CONFIRMED.get();
        upgrade_one_anchor(&db, "decision_anchors", &row).await;
        let after = metrics::ANCHOR_UPGRADED_TO_CONFIRMED.get();

        assert_eq!(
            before, after,
            "a 0-rows-affected UPDATE must not increment the upgraded-to-confirmed metric"
        );
    }
}
