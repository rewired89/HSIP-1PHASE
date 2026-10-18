//! Batch anchoring. Unanchored proofs from every organization are folded
//! into one RFC 6962 Merkle tree; the root is signed by the node's anchor
//! key and, when enabled, committed to Bitcoin through OpenTimestamps
//! calendars. A proof then carries evidence that does not depend on this
//! database staying honest.
//!
//! The OpenTimestamps client below is ported from the retired Phase 1 repo and was
//! verified there against the real public calendars.

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ed25519_dalek::Signer;
use hsip_proof::MerkleTree;
use sqlx::Row;
use uuid::Uuid;

use crate::{db::now_ms, state::AppState};

// ------------------------------------------------------------ OpenTimestamps

const PENDING_ATTESTATION_TAG: [u8; 8] = [0x83, 0xdf, 0xe3, 0x0d, 0x2e, 0xf9, 0x0c, 0x8e];
pub const BITCOIN_ATTESTATION_TAG: [u8; 8] = [0x05, 0x88, 0x96, 0x0d, 0x73, 0xd7, 0x19, 0x01];

pub const DEFAULT_CALENDARS: &[&str] = &[
    "https://alice.btc.calendar.opentimestamps.org",
    "https://bob.btc.calendar.opentimestamps.org",
    "https://finney.calendar.eternitywall.com",
];

/// Tag presence only: trusts the calendar the same way the initial
/// "pending" submission does. Full `.ots` path verification is left to
/// the OpenTimestamps client, using the proof carried in the bundle.
#[must_use]
pub fn contains_bitcoin_attestation(proof: &[u8]) -> bool {
    proof.windows(8).any(|w| w == BITCOIN_ATTESTATION_TAG)
}

fn read_varuint(bytes: &[u8], pos: &mut usize) -> Option<u64> {
    let (mut result, mut shift) = (0u64, 0u32);
    loop {
        let byte = *bytes.get(*pos)?;
        *pos += 1;
        result |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(result);
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
}

#[must_use]
pub fn extract_pending_calendar_uri(proof: &[u8]) -> Option<String> {
    let tag_pos = proof
        .windows(8)
        .position(|w| w == PENDING_ATTESTATION_TAG)?;
    let mut pos = tag_pos + 8;
    let _outer = read_varuint(proof, &mut pos)?;
    let inner = usize::try_from(read_varuint(proof, &mut pos)?).ok()?;
    String::from_utf8(proof.get(pos..pos.checked_add(inner)?)?.to_vec()).ok()
}

fn http() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent(concat!("hsip-platform/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("failed to build HTTP client")
}

pub struct CalendarReceipt {
    pub calendar_url: String,
    pub response_bytes: Vec<u8>,
}

pub async fn submit_digest_to(calendars: &[&str], digest: &[u8; 32]) -> Result<CalendarReceipt> {
    let client = http()?;
    let mut last = String::new();
    for cal in calendars {
        match client
            .post(format!("{cal}/digest"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(digest.to_vec())
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => match r.bytes().await {
                Ok(b) => {
                    return Ok(CalendarReceipt {
                        calendar_url: (*cal).into(),
                        response_bytes: b.to_vec(),
                    })
                }
                Err(e) => last = format!("{cal}: body: {e}"),
            },
            Ok(r) => last = format!("{cal}: HTTP {}", r.status()),
            Err(e) => last = format!("{cal}: {e}"),
        }
    }
    bail!("all OpenTimestamps calendars failed: {last}")
}

pub async fn check_for_upgrade(calendar: &str, digest: &[u8; 32]) -> Result<Option<Vec<u8>>> {
    let r = http()?
        .get(format!("{calendar}/timestamp/{}", hex::encode(digest)))
        .send()
        .await?;
    if !r.status().is_success() {
        return Ok(None);
    }
    Ok(Some(r.bytes().await?.to_vec()))
}

// ------------------------------------------------------------------- batching

const MAX_BATCH: i64 = 1000;
/// Stop polling calendars for a commitment after this long.
const MAX_PENDING_AGE_MS: i64 = 7 * 24 * 3600 * 1000;

/// Anchors whatever is waiting. Returns the number of proofs anchored.
pub async fn run_cycle(state: &AppState, calendars: &[&str]) -> Result<usize> {
    let rows = sqlx::query("SELECT id, event_hash FROM proofs WHERE anchor_id IS NULL ORDER BY created_at ASC, seq ASC LIMIT $1")
        .bind(MAX_BATCH)
        .fetch_all(&state.db)
        .await?;
    if rows.is_empty() {
        return Ok(0);
    }
    let mut ids = Vec::with_capacity(rows.len());
    let mut leaves: Vec<Vec<u8>> = Vec::with_capacity(rows.len());
    for r in &rows {
        ids.push(r.try_get::<String, _>(0)?);
        leaves.push(hex::decode(r.try_get::<String, _>(1)?)?);
    }
    let tree = MerkleTree::from_leaves(&leaves);
    let root = tree.root();
    let signature = B64.encode(state.anchor_key.sign(&root).to_bytes());
    let verify_key = B64.encode(state.anchor_key.verifying_key().to_bytes());

    let (ots_status, ots_proof) = if state.cfg.ots_enabled {
        match submit_digest_to(calendars, &root).await {
            Ok(r) => ("pending", Some(r.response_bytes)),
            Err(e) => {
                tracing::warn!(error = %e, "OpenTimestamps calendars unreachable; will retry");
                ("calendar_unreachable", None)
            }
        }
    } else {
        ("disabled", None)
    };

    let anchor_id = Uuid::new_v4().to_string();
    let mut tx = state.db.begin().await?;
    sqlx::query("INSERT INTO anchors (id, merkle_root, leaf_count, anchor_signature, anchor_verify_key, ots_proof, ots_status, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(&anchor_id)
        .bind(hex::encode(root))
        .bind(ids.len() as i64)
        .bind(&signature)
        .bind(&verify_key)
        .bind(ots_proof)
        .bind(ots_status)
        .bind(now_ms())
        .execute(&mut *tx)
        .await?;
    for (i, id) in ids.iter().enumerate() {
        let res = sqlx::query("UPDATE proofs SET anchor_id = $1, merkle_index = $2 WHERE id = $3 AND anchor_id IS NULL")
            .bind(&anchor_id)
            .bind(i as i64)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        if res.rows_affected() != 1 {
            // Another node anchored this proof first; abandon this batch.
            tx.rollback().await?;
            tracing::info!("anchor batch lost a race with another node; skipping");
            return Ok(0);
        }
    }
    tx.commit().await?;
    tracing::info!(anchor = %anchor_id, proofs = ids.len(), ots = ots_status, "anchored batch");
    Ok(ids.len())
}

/// Re-submits batches whose calendars were unreachable at anchor time.
pub async fn retry_unreachable(state: &AppState, calendars: &[&str]) -> Result<()> {
    let rows = sqlx::query("SELECT id, merkle_root FROM anchors WHERE ots_status = 'calendar_unreachable' ORDER BY created_at ASC LIMIT 25")
        .fetch_all(&state.db)
        .await?;
    for r in &rows {
        let id: String = r.try_get(0)?;
        let root: [u8; 32] = hex::decode(r.try_get::<String, _>(1)?)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("bad root"))?;
        if let Ok(rc) = submit_digest_to(calendars, &root).await {
            let res = sqlx::query("UPDATE anchors SET ots_proof = $1, ots_status = 'pending' WHERE id = $2 AND ots_status = 'calendar_unreachable'")
                .bind(rc.response_bytes)
                .bind(&id)
                .execute(&state.db)
                .await?;
            if res.rows_affected() == 0 {
                tracing::warn!(anchor = %id, "retry succeeded but status row did not update");
            }
        }
    }
    Ok(())
}

/// Promotes `pending` batches to `confirmed` once a calendar reports a
/// Bitcoin block attestation.
pub async fn run_upgrade_cycle(state: &AppState) -> Result<usize> {
    let cutoff = now_ms() - MAX_PENDING_AGE_MS;
    let rows = sqlx::query("SELECT id, merkle_root, ots_proof FROM anchors WHERE ots_status = 'pending' AND created_at > $1 ORDER BY created_at ASC LIMIT 25")
        .bind(cutoff)
        .fetch_all(&state.db)
        .await?;
    let mut confirmed = 0;
    for r in &rows {
        let id: String = r.try_get(0)?;
        let root: [u8; 32] = hex::decode(r.try_get::<String, _>(1)?)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("bad root"))?;
        let proof: Option<Vec<u8>> = r.try_get(2)?;
        let Some(cal) = proof.as_deref().and_then(extract_pending_calendar_uri) else {
            continue;
        };
        if let Ok(Some(body)) = check_for_upgrade(&cal, &root).await {
            if contains_bitcoin_attestation(&body) {
                let res = sqlx::query("UPDATE anchors SET ots_proof = $1, ots_status = 'confirmed' WHERE id = $2 AND ots_status = 'pending'")
                    .bind(body)
                    .bind(&id)
                    .execute(&state.db)
                    .await?;
                if res.rows_affected() == 1 {
                    confirmed += 1;
                } else {
                    tracing::warn!(anchor = %id, "confirmation seen but status row did not update");
                }
            }
        }
    }
    Ok(confirmed)
}

/// Background loops: batch when the trigger count is reached or the oldest
/// waiting proof is older than the interval; poll for confirmations slowly
/// (Bitcoin blocks land about every ten minutes).
pub fn spawn(state: AppState) {
    let s1 = state.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            tick.tick().await;
            let due = async {
                let r = sqlx::query(
                    "SELECT COUNT(*), MIN(created_at) FROM proofs WHERE anchor_id IS NULL",
                )
                .fetch_one(&s1.db)
                .await?;
                let n: i64 = r.try_get(0)?;
                let oldest: Option<i64> = r.try_get(1)?;
                Ok::<bool, anyhow::Error>(match oldest {
                    Some(o) => {
                        n >= s1.cfg.anchor_batch_trigger
                            || now_ms() - o >= (s1.cfg.anchor_interval_secs as i64) * 1000
                    }
                    None => false,
                })
            }
            .await;
            match due {
                Ok(true) => {
                    if let Err(e) = run_cycle(&s1, DEFAULT_CALENDARS).await {
                        tracing::error!(error = %e, "anchor cycle failed");
                    }
                    if s1.cfg.ots_enabled {
                        if let Err(e) = retry_unreachable(&s1, DEFAULT_CALENDARS).await {
                            tracing::error!(error = %e, "anchor retry failed");
                        }
                    }
                }
                Ok(false) => {}
                Err(e) => tracing::error!(error = %e, "anchor due-check failed"),
            }
            s1.sweep_throttle_if_due();
        }
    });
    if state.cfg.ots_enabled {
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(15 * 60));
            loop {
                tick.tick().await;
                match run_upgrade_cycle(&state).await {
                    Ok(n) if n > 0 => {
                        tracing::info!(confirmed = n, "OpenTimestamps batches confirmed on Bitcoin")
                    }
                    Ok(_) => {}
                    Err(e) => tracing::error!(error = %e, "upgrade cycle failed"),
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn falls_over_to_the_next_calendar() {
        let dead = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/digest"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&dead)
            .await;
        let alive = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/digest"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"ok".to_vec()))
            .mount(&alive)
            .await;
        let r = submit_digest_to(&[&dead.uri(), &alive.uri()], &[7u8; 32])
            .await
            .unwrap();
        assert_eq!(r.calendar_url, alive.uri());
        let reqs = alive.received_requests().await.unwrap();
        assert_eq!(reqs[0].body, vec![7u8; 32]);
    }

    #[tokio::test]
    async fn all_calendars_failing_is_an_error() {
        let dead = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/digest"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&dead)
            .await;
        assert!(submit_digest_to(&[&dead.uri()], &[1u8; 32]).await.is_err());
    }

    #[test]
    fn bitcoin_tag_detection() {
        let mut p = vec![1, 2, 3];
        assert!(!contains_bitcoin_attestation(&p));
        p.extend_from_slice(&BITCOIN_ATTESTATION_TAG);
        assert!(contains_bitcoin_attestation(&p));
    }
}
