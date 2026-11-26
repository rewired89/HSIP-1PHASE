//! Signed stream checkpoints. Each time a batch is anchored, every stream that
//! gained records gets a checkpoint: the platform signs "this stream is at
//! position N with this head" at a time. Customers keep copies; the offline
//! verifier (`hsip_proof::check_checkpoints`) then catches a history that
//! disagrees with one. Best effort: a failure is logged and never stops anchoring.

use std::collections::BTreeMap;

use hsip_proof::{self as hp, Checkpoint, CheckpointBody, CHECKPOINT_FORMAT};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    db::{ms_to_iso, now_ms},
    errors::ApiResult,
    state::AppState,
};

/// `(org_id, stream_id)` to `(head seq, head event hash)`.
pub type Heads = BTreeMap<(String, String), (i64, String)>;

/// Writes one checkpoint per stream whose head moved past its last checkpoint.
pub async fn record_heads(state: &AppState, heads: &Heads) {
    for ((org, stream), (seq, head)) in heads {
        if let Err(e) = record_one(state, org, stream, *seq, head).await {
            tracing::error!(org = %org, stream = %stream, error = ?e, "checkpoint not written");
        }
    }
}

async fn record_one(
    state: &AppState,
    org: &str,
    stream: &str,
    seq: i64,
    head: &str,
) -> anyhow::Result<()> {
    let last = sqlx::query(
        "SELECT seq, checkpoint_hash FROM checkpoints WHERE stream_id = $1 ORDER BY seq DESC LIMIT 1",
    )
    .bind(stream)
    .fetch_optional(&state.db)
    .await?;
    let prev = match last {
        Some(r) => {
            if r.try_get::<i64, _>(0)? >= seq {
                return Ok(());
            }
            r.try_get::<String, _>(1)?
        }
        None => String::new(),
    };
    let key = state.org_signing_key(org).await?;
    let now = now_ms();
    let cp = hp::sign_checkpoint(
        CheckpointBody {
            format: CHECKPOINT_FORMAT.into(),
            org_id: org.into(),
            stream_id: stream.into(),
            seq: seq.to_string(),
            head_hash: head.into(),
            timestamp_ms: now.to_string(),
            timestamp_iso: ms_to_iso(now),
            prev_checkpoint_hash: prev,
            signer_key: hp::verify_key_b64(&key),
        },
        &key,
    )
    .map_err(anyhow::Error::msg)?;
    // Two nodes racing on one stream: the unique (stream, seq) keeps the first.
    let res = sqlx::query("INSERT INTO checkpoints (id, org_id, stream_id, seq, head_hash, checkpoint_hash, checkpoint_json, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(Uuid::new_v4().to_string())
        .bind(org)
        .bind(stream)
        .bind(seq)
        .bind(head)
        .bind(&cp.checkpoint_hash)
        .bind(serde_json::to_string(&cp)?)
        .bind(now)
        .execute(&state.db)
        .await;
    match res {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Checkpoints of one stream after `after_seq`, oldest first, at most `limit`.
pub async fn list(
    state: &AppState,
    org: &str,
    stream: &str,
    after_seq: i64,
    limit: i64,
) -> ApiResult<Vec<Checkpoint>> {
    let rows = sqlx::query("SELECT checkpoint_json FROM checkpoints WHERE org_id = $1 AND stream_id = $2 AND seq > $3 ORDER BY seq ASC LIMIT $4")
        .bind(org)
        .bind(stream)
        .bind(after_seq)
        .bind(limit.clamp(1, 200))
        .fetch_all(&state.db)
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in &rows {
        out.push(serde_json::from_str(&r.try_get::<String, _>(0)?)?);
    }
    Ok(out)
}
