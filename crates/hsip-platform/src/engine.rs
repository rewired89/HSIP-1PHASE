//! The proof engine: record a signed, hash-chained proof, and rebuild its
//! self-contained verification bundle. Verification itself lives in
//! `hsip-proof` so customers can run the same code without this server.

use std::collections::{BTreeMap, HashMap};

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use hsip_proof::{
    self as hp, AnchorBlock, Envelope, MerkleTree, ProofBundle, ProofStepDto, BUNDLE_FORMAT,
    ENVELOPE_FORMAT,
};
use serde::Serialize;
use sqlx::{any::AnyRow, AnyPool, Row};
use uuid::Uuid;

use crate::{
    db::{month_key, ms_to_iso, now_ms},
    errors::{ApiError, ApiResult},
    plans,
    state::AppState,
};

pub const PROOF_COLS: &str = "id, org_id, stream_id, seq, key_id, actor, action, subject, payload_hash, \
     metadata_json, prev_hash, event_hash, signature, timestamp_iso, created_at, anchor_id, merkle_index";

const MAX_ATTEMPTS: u32 = 8;

#[derive(Debug, Clone)]
pub struct ProofRow {
    pub id: String,
    pub org_id: String,
    pub stream_id: String,
    pub seq: i64,
    pub key_id: Option<String>,
    pub actor: String,
    pub action: String,
    pub subject: String,
    pub payload_hash: String,
    pub metadata_json: String,
    pub prev_hash: String,
    pub event_hash: String,
    pub signature: String,
    pub timestamp_iso: String,
    pub created_at: i64,
    pub anchor_id: Option<String>,
    pub merkle_index: Option<i64>,
}

impl ProofRow {
    pub fn from_row(r: &AnyRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: r.try_get(0)?,
            org_id: r.try_get(1)?,
            stream_id: r.try_get(2)?,
            seq: r.try_get(3)?,
            key_id: r.try_get(4)?,
            actor: r.try_get(5)?,
            action: r.try_get(6)?,
            subject: r.try_get(7)?,
            payload_hash: r.try_get(8)?,
            metadata_json: r.try_get(9)?,
            prev_hash: r.try_get(10)?,
            event_hash: r.try_get(11)?,
            signature: r.try_get(12)?,
            timestamp_iso: r.try_get(13)?,
            created_at: r.try_get(14)?,
            anchor_id: r.try_get(15)?,
            merkle_index: r.try_get(16)?,
        })
    }

    pub fn envelope(&self) -> Envelope {
        Envelope {
            format: ENVELOPE_FORMAT.into(),
            proof_id: self.id.clone(),
            org_id: self.org_id.clone(),
            stream_id: self.stream_id.clone(),
            seq: self.seq.to_string(),
            actor: self.actor.clone(),
            action: self.action.clone(),
            subject: self.subject.clone(),
            payload_hash: self.payload_hash.clone(),
            metadata: serde_json::from_str(&self.metadata_json).unwrap_or_default(),
            prev_hash: self.prev_hash.clone(),
            timestamp_iso: self.timestamp_iso.clone(),
            timestamp_ms: self.created_at.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProofSummary {
    pub id: String,
    pub stream_id: String,
    pub seq: i64,
    pub actor: String,
    pub action: String,
    pub subject: String,
    pub payload_hash: String,
    pub event_hash: String,
    pub timestamp_iso: String,
    pub created_at: i64,
    pub anchored: bool,
    pub metadata: BTreeMap<String, String>,
}

impl From<&ProofRow> for ProofSummary {
    fn from(r: &ProofRow) -> Self {
        Self {
            id: r.id.clone(),
            stream_id: r.stream_id.clone(),
            seq: r.seq,
            actor: r.actor.clone(),
            action: r.action.clone(),
            subject: r.subject.clone(),
            payload_hash: r.payload_hash.clone(),
            event_hash: r.event_hash.clone(),
            timestamp_iso: r.timestamp_iso.clone(),
            created_at: r.created_at,
            anchored: r.anchor_id.is_some(),
            metadata: serde_json::from_str(&r.metadata_json).unwrap_or_default(),
        }
    }
}

// ------------------------------------------------------------------ recording

pub struct NewProof<'a> {
    pub org_id: &'a str,
    pub stream_id: &'a str,
    pub key_id: Option<&'a str>,
    pub actor: &'a str,
    pub action: &'a str,
    pub subject: &'a str,
    pub payload_hash: &'a str,
    pub metadata: BTreeMap<String, String>,
}

fn bad(m: &str) -> ApiError {
    ApiError::BadRequest(m.into())
}

fn validate(np: &NewProof<'_>) -> ApiResult<()> {
    if np.actor.is_empty() || np.actor.len() > 128 {
        return Err(bad("actor must be 1-128 characters"));
    }
    if np.action.is_empty()
        || np.action.len() > 96
        || !np
            .action
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._:/-".contains(c))
    {
        return Err(bad(
            "action must be 1-96 characters of letters, digits and . _ : / -",
        ));
    }
    if np.subject.len() > 256 {
        return Err(bad("subject must be at most 256 characters"));
    }
    if np.payload_hash.len() != 64 || !np.payload_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(bad("payload_hash must be a 64-character hex SHA-256 digest (hash your record client-side; never send its content)"));
    }
    if np.metadata.len() > 16 {
        return Err(bad("metadata allows at most 16 entries"));
    }
    for (k, v) in &np.metadata {
        if k.is_empty() || k.len() > 64 || v.len() > 256 {
            return Err(bad(
                "metadata keys must be 1-64 and values at most 256 characters",
            ));
        }
    }
    Ok(())
}

pub struct OrgInfo {
    pub name: String,
    pub plan: String,
    pub status: String,
    pub verify_key: String,
}

pub async fn org_info(db: &AnyPool, org_id: &str) -> ApiResult<OrgInfo> {
    let r =
        sqlx::query("SELECT name, plan, status, verify_key_b64 FROM organizations WHERE id = $1")
            .bind(org_id)
            .fetch_one(db)
            .await?;
    Ok(OrgInfo {
        name: r.try_get(0)?,
        plan: r.try_get(1)?,
        status: r.try_get(2)?,
        verify_key: r.try_get(3)?,
    })
}

pub async fn month_usage(db: &AnyPool, org_id: &str, month: &str) -> ApiResult<i64> {
    let r = sqlx::query("SELECT proofs FROM usage_monthly WHERE org_id = $1 AND month = $2")
        .bind(org_id)
        .bind(month)
        .fetch_optional(db)
        .await?;
    Ok(match r {
        Some(r) => r.try_get(0)?,
        None => 0,
    })
}

/// Signs and chains one proof onto its stream. Concurrent writers to the
/// same stream race on `UNIQUE(stream_id, seq)`; the loser re-reads the tip
/// and retries, so the chain can never fork.
pub async fn record(state: &AppState, np: &NewProof<'_>) -> ApiResult<ProofBundle> {
    validate(np)?;
    let org = org_info(&state.db, np.org_id).await?;
    let stream = sqlx::query("SELECT kind, archived FROM streams WHERE id = $1 AND org_id = $2")
        .bind(np.stream_id)
        .bind(np.org_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("stream not found".into()))?;
    let kind: String = stream.try_get(0)?;
    let archived: i64 = stream.try_get(1)?;
    let is_system = kind == "system";
    if !is_system {
        if org.status != "active" {
            return Err(ApiError::Forbidden(
                "this organization is suspended; contact support".into(),
            ));
        }
        if archived != 0 {
            return Err(ApiError::Conflict("stream is archived".into()));
        }
        let plan = plans::get(&org.plan);
        let used = month_usage(&state.db, np.org_id, &month_key(now_ms())).await?;
        if used >= plan.proofs_per_month {
            return Err(ApiError::PaymentRequired(format!(
                "monthly proof limit of {} reached on the {} plan; upgrade to continue",
                plan.proofs_per_month, plan.name
            )));
        }
    }

    let key = state.org_signing_key(np.org_id).await.map_err(|e| {
        tracing::error!(error = %e, org = np.org_id, "cannot load org signing key");
        ApiError::internal()
    })?;
    let metadata_json = serde_json::to_string(&np.metadata)?;

    for attempt in 1..=MAX_ATTEMPTS {
        let tip = sqlx::query(
            "SELECT seq, event_hash FROM proofs WHERE stream_id = $1 ORDER BY seq DESC LIMIT 1",
        )
        .bind(np.stream_id)
        .fetch_optional(&state.db)
        .await?;
        let (seq, prev_hash): (i64, String) = match tip {
            Some(r) => (r.try_get::<i64, _>(0)? + 1, r.try_get(1)?),
            None => (1, String::new()),
        };
        let now = now_ms();
        let id = Uuid::new_v4().to_string();
        let envelope = Envelope {
            format: ENVELOPE_FORMAT.into(),
            proof_id: id.clone(),
            org_id: np.org_id.into(),
            stream_id: np.stream_id.into(),
            seq: seq.to_string(),
            actor: np.actor.into(),
            action: np.action.into(),
            subject: np.subject.into(),
            payload_hash: np.payload_hash.to_ascii_lowercase(),
            metadata: np.metadata.clone(),
            prev_hash: prev_hash.clone(),
            timestamp_iso: ms_to_iso(now),
            timestamp_ms: now.to_string(),
        };
        let (event_hash, signature) = hp::sign_envelope(&envelope, &key)?;

        let res = sqlx::query(
            "INSERT INTO proofs (id, org_id, stream_id, seq, key_id, actor, action, subject, payload_hash,
                metadata_json, prev_hash, event_hash, signature, timestamp_iso, created_at, anchor_id, merkle_index)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,NULL,NULL)",
        )
        .bind(&id)
        .bind(np.org_id)
        .bind(np.stream_id)
        .bind(seq)
        .bind(np.key_id.map(str::to_string))
        .bind(np.actor)
        .bind(np.action)
        .bind(np.subject)
        .bind(&envelope.payload_hash)
        .bind(&metadata_json)
        .bind(&prev_hash)
        .bind(&event_hash)
        .bind(&signature)
        .bind(&envelope.timestamp_iso)
        .bind(now)
        .execute(&state.db)
        .await;

        match res {
            Ok(_) => {
                if !is_system {
                    let up = sqlx::query(
                        "INSERT INTO usage_monthly (org_id, month, proofs) VALUES ($1, $2, 1)
                         ON CONFLICT (org_id, month) DO UPDATE SET proofs = usage_monthly.proofs + 1",
                    )
                    .bind(np.org_id)
                    .bind(month_key(now))
                    .execute(&state.db)
                    .await;
                    if let Err(e) = up {
                        // The proof is already durable; a missed meter tick must not fail the call.
                        tracing::error!(error = %e, org = np.org_id, "usage metering failed");
                    }
                }
                return Ok(ProofBundle {
                    format: BUNDLE_FORMAT.into(),
                    envelope,
                    event_hash,
                    signature,
                    sign_algo: "ED25519".into(),
                    org_verify_key: org.verify_key,
                    org_name: org.name,
                    anchor: None,
                });
            }
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                if attempt == MAX_ATTEMPTS {
                    return Err(ApiError::Conflict(
                        "stream is under heavy concurrent writes; retry, or spread load across streams".into(),
                    ));
                }
                tokio::time::sleep(std::time::Duration::from_millis(
                    5 * u64::from(attempt) * u64::from(attempt),
                ))
                .await;
            }
            Err(e) => return Err(e.into()),
        }
    }
    unreachable!("loop returns within MAX_ATTEMPTS")
}

// -------------------------------------------------------------------- bundles

struct AnchorEntry {
    root: String,
    signature: String,
    verify_key: String,
    created_at: i64,
    ots_status: String,
    ots_proof: Option<Vec<u8>>,
    leaf_count: i64,
    tree: MerkleTree,
}

/// Per-request cache so a pack of thousands of proofs loads each batch once.
#[derive(Default)]
pub struct AnchorCache(HashMap<String, AnchorEntry>);

async fn load_anchor(db: &AnyPool, anchor_id: &str) -> ApiResult<AnchorEntry> {
    let a = sqlx::query(
        "SELECT merkle_root, anchor_signature, anchor_verify_key, ots_proof, ots_status, created_at, leaf_count
         FROM anchors WHERE id = $1",
    )
    .bind(anchor_id)
    .fetch_one(db)
    .await?;
    let leaves =
        sqlx::query("SELECT event_hash FROM proofs WHERE anchor_id = $1 ORDER BY merkle_index ASC")
            .bind(anchor_id)
            .fetch_all(db)
            .await?;
    let raw: Vec<Vec<u8>> = leaves
        .iter()
        .map(|r| -> Result<Vec<u8>, ApiError> {
            let h: String = r.try_get(0)?;
            hex::decode(h).map_err(|_| ApiError::internal())
        })
        .collect::<Result<_, _>>()?;
    if raw.is_empty() {
        return Err(ApiError::internal());
    }
    Ok(AnchorEntry {
        root: a.try_get(0)?,
        signature: a.try_get(1)?,
        verify_key: a.try_get(2)?,
        ots_proof: a.try_get(3)?,
        ots_status: a.try_get(4)?,
        created_at: a.try_get(5)?,
        leaf_count: a.try_get(6)?,
        tree: MerkleTree::from_leaves(&raw),
    })
}

pub async fn bundle_for(
    db: &AnyPool,
    row: &ProofRow,
    org: &OrgInfo,
    cache: &mut AnchorCache,
) -> ApiResult<ProofBundle> {
    let anchor = match (&row.anchor_id, row.merkle_index) {
        (Some(aid), Some(idx)) => {
            if !cache.0.contains_key(aid) {
                let e = load_anchor(db, aid).await?;
                cache.0.insert(aid.clone(), e);
            }
            let e = &cache.0[aid];
            let idx_u = usize::try_from(idx).map_err(|_| ApiError::internal())?;
            if idx_u >= e.tree.len() {
                return Err(ApiError::internal());
            }
            Some(AnchorBlock {
                anchor_id: aid.clone(),
                merkle_root: e.root.clone(),
                merkle_index: idx,
                leaf_count: e.leaf_count,
                inclusion_proof: e
                    .tree
                    .inclusion_proof(idx_u)
                    .iter()
                    .map(ProofStepDto::from)
                    .collect(),
                anchor_signature: e.signature.clone(),
                anchor_verify_key: e.verify_key.clone(),
                anchored_at_iso: ms_to_iso(e.created_at),
                ots_status: e.ots_status.clone(),
                ots_proof: e.ots_proof.as_ref().map(|b| B64.encode(b)),
            })
        }
        _ => None,
    };
    Ok(ProofBundle {
        format: BUNDLE_FORMAT.into(),
        envelope: row.envelope(),
        event_hash: row.event_hash.clone(),
        signature: row.signature.clone(),
        sign_algo: "ED25519".into(),
        org_verify_key: org.verify_key.clone(),
        org_name: org.name.clone(),
        anchor,
    })
}

pub async fn fetch_proof(db: &AnyPool, org_id: &str, proof_id: &str) -> ApiResult<ProofRow> {
    let r = sqlx::query(&format!(
        "SELECT {PROOF_COLS} FROM proofs WHERE id = $1 AND org_id = $2"
    ))
    .bind(proof_id)
    .bind(org_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| ApiError::NotFound("proof not found".into()))?;
    Ok(ProofRow::from_row(&r)?)
}
