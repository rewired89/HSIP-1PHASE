//! BLAKE3 hash-chained writes to the `audit_entries` table.
//!
//! Closes the gap documented in `THREAT_MODEL.md` §4.8: the audit log was
//! append-only by *application policy* only, so an attacker with direct
//! database write access (OS-level compromise) could alter or delete rows
//! without leaving a detectable fingerprint. Every entry now links to the
//! previous one via `entry_hash = BLAKE3(prev_hash || id || tenant_id ||
//! action || peer_verify_key || details || timestamp)`, so tampering with
//! or removing a row breaks the chain for every entry after it — detectable
//! via `GET /v1/audit/verify` without trusting the database's own account
//! of what happened.
//!
//! Scope: this chain starts at the tenant's first write after this
//! migration. Rows written before it have NULL `prev_hash`/`entry_hash` and
//! are not covered — there is no retroactive integrity proof for history
//! that predates the hash chain existing.
//!
//! The chain above is self-verifiable but, on its own, only proves internal
//! consistency — it can't prove the whole chain wasn't deleted and
//! recreated by whoever controls this database. `anchor_job.rs`'s
//! `run_audit_anchor_cycle` closes that gap the same way decision
//! attestations are anchored: batches of entries (by `entry_hash`) are
//! folded into an RFC 6962 Merkle tree, the root is signed by the
//! node-level anchor identity and submitted to OpenTimestamps. See
//! `routes::audit::proof`/`verify_proof`.
//!
//! Uses the same `UNIQUE(tenant_id, prev_hash)` optimistic-retry pattern as
//! `routes::decisions::record`: a conflict means another request extended
//! this tenant's chain first, not a real error.

use rand::Rng;
use sqlx::Row;
use uuid::Uuid;

use crate::db::Db;

const MAX_ATTEMPTS: u32 = 5;
const GENESIS_PREV_HASH: &str = "";

/// Small randomized backoff between chain-write retry attempts. Without
/// this, concurrent writers hitting the same tenant's chain would retry in
/// a tight loop with no delay — fine at low contention, but at scale (a
/// busy agent writing many entries/sec) that's a self-inflicted thundering
/// herd instead of the DB naturally serializing the writers. Grows with
/// attempt number; capped low because this blocks a live HTTP request.
pub(crate) async fn chain_retry_backoff(attempt: u32) {
    let base_ms = 2u64.saturating_mul(attempt as u64);
    let jitter_ms = rand::thread_rng().gen_range(0..5);
    tokio::time::sleep(std::time::Duration::from_millis(base_ms + jitter_ms)).await;
}

/// Appends one entry to `tenant_id`'s audit hash chain and returns its id.
pub async fn record(
    db: &Db,
    tenant_id: &str,
    action: &str,
    peer_verify_key: Option<&str>,
    details: Option<&str>,
    timestamp: i64,
) -> Result<String, sqlx::Error> {
    for attempt in 1..=MAX_ATTEMPTS {
        let prev_row = sqlx::query(
            "SELECT entry_hash FROM audit_entries
             WHERE tenant_id = $1 AND entry_hash IS NOT NULL
             ORDER BY timestamp DESC LIMIT 1",
        )
        .bind(tenant_id)
        .fetch_optional(db)
        .await?;
        let prev_hash: String = match prev_row {
            Some(r) => r.try_get::<String, _>(0)?,
            None => GENESIS_PREV_HASH.to_string(),
        };

        let id = Uuid::new_v4().to_string();
        let entry_hash = compute_entry_hash(
            &prev_hash,
            &id,
            tenant_id,
            action,
            peer_verify_key,
            details,
            timestamp,
        );

        let result = sqlx::query(
            "INSERT INTO audit_entries
             (id, tenant_id, action, peer_verify_key, details, timestamp, prev_hash, entry_hash)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(action)
        .bind(peer_verify_key)
        .bind(details)
        .bind(timestamp)
        .bind(&prev_hash)
        .bind(&entry_hash)
        .execute(db)
        .await;

        match result {
            Ok(_) => return Ok(id),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                crate::metrics::CHAIN_WRITE_RETRIES
                    .with_label_values(&["audit"])
                    .inc();
                if attempt == MAX_ATTEMPTS {
                    return Err(sqlx::Error::Database(db_err));
                }
                chain_retry_backoff(attempt).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!("loop always returns or errors within MAX_ATTEMPTS")
}

/// Exposed `pub(crate)` (not private) so `routes::audit::verify_proof` can
/// recompute an entry's hash from caller-supplied fields without a DB call
/// — the same "pure function, no trust in this server's database" pattern
/// as `routes::decisions::verify`.
pub(crate) fn compute_entry_hash(
    prev_hash: &str,
    id: &str,
    tenant_id: &str,
    action: &str,
    peer_verify_key: Option<&str>,
    details: Option<&str>,
    timestamp: i64,
) -> String {
    let mut hasher = blake3::Hasher::new();
    // 0x00 separator: none of these fields can otherwise be delimited
    // unambiguously (e.g. "ab"+"c" colliding with "a"+"bc").
    hasher.update(prev_hash.as_bytes());
    hasher.update(&[0u8]);
    hasher.update(id.as_bytes());
    hasher.update(&[0u8]);
    hasher.update(tenant_id.as_bytes());
    hasher.update(&[0u8]);
    hasher.update(action.as_bytes());
    hasher.update(&[0u8]);
    hasher.update(peer_verify_key.unwrap_or("").as_bytes());
    hasher.update(&[0u8]);
    hasher.update(details.unwrap_or("").as_bytes());
    hasher.update(&[0u8]);
    hasher.update(&timestamp.to_le_bytes());
    hex::encode(hasher.finalize().as_bytes())
}

/// One row as read back from the database for chain verification.
pub struct ChainRow {
    pub id: String,
    pub tenant_id: String,
    pub action: String,
    pub peer_verify_key: Option<String>,
    pub details: Option<String>,
    pub timestamp: i64,
    pub prev_hash: Option<String>,
    pub entry_hash: Option<String>,
}

/// Result of walking one tenant's chain in insertion order (oldest first).
pub struct VerifyResult {
    pub valid: bool,
    pub checked: usize,
    pub unchained: usize,
    pub first_break_id: Option<String>,
}

/// Recomputes and checks the hash chain over `rows`, which must already be
/// pre-migration-first, chain-order (i.e. `ORDER BY timestamp ASC`). Rows
/// with NULL `entry_hash` (written before this migration) are counted in
/// `unchained` and skipped rather than treated as breaks.
pub fn verify_chain(rows: &[ChainRow]) -> VerifyResult {
    let mut expected_prev = GENESIS_PREV_HASH.to_string();
    let mut checked = 0usize;
    let mut unchained = 0usize;

    for row in rows {
        let (Some(prev_hash), Some(entry_hash)) = (&row.prev_hash, &row.entry_hash) else {
            unchained += 1;
            continue;
        };

        if *prev_hash != expected_prev {
            return VerifyResult {
                valid: false,
                checked,
                unchained,
                first_break_id: Some(row.id.clone()),
            };
        }

        let recomputed = compute_entry_hash(
            prev_hash,
            &row.id,
            &row.tenant_id,
            &row.action,
            row.peer_verify_key.as_deref(),
            row.details.as_deref(),
            row.timestamp,
        );
        if recomputed != *entry_hash {
            return VerifyResult {
                valid: false,
                checked,
                unchained,
                first_break_id: Some(row.id.clone()),
            };
        }

        checked += 1;
        expected_prev = entry_hash.clone();
    }

    VerifyResult {
        valid: true,
        checked,
        unchained,
        first_break_id: None,
    }
}
