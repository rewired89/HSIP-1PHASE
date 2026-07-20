use axum::{
    extract::{Path, Query, State},
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use hsip_core::merkle::{self, MerkleTree, ProofStep};

use super::decisions::ProofStepDto;
use crate::{
    audit_log::ChainRow,
    auth::TenantId,
    errors::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Deserialize)]
pub struct AuditQuery {
    pub limit: Option<i64>,
    pub action: Option<String>,
}

#[derive(Serialize)]
pub struct AuditEntry {
    pub id: String,
    pub action: String,
    pub peer_verify_key: Option<String>,
    pub details: Option<String>,
    pub timestamp: i64,
    /// BLAKE3 chain fields — `None` for entries written before the audit
    /// hash chain existed (see `audit_log` module docs).
    pub prev_hash: Option<String>,
    pub entry_hash: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
    Query(params): Query<AuditQuery>,
) -> ApiResult<Json<Vec<AuditEntry>>> {
    let limit = params.limit.unwrap_or(50).min(500);
    let action = params.action.clone();

    let rows = if let Some(act) = action {
        let pattern = format!("%{act}%");
        sqlx::query(
            "SELECT id, action, peer_verify_key, details, timestamp, prev_hash, entry_hash
             FROM audit_entries WHERE tenant_id=? AND action LIKE ?
             ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(&tenant.0)
        .bind(&pattern)
        .bind(limit)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query(
            "SELECT id, action, peer_verify_key, details, timestamp, prev_hash, entry_hash
             FROM audit_entries WHERE tenant_id=?
             ORDER BY timestamp DESC LIMIT ?",
        )
        .bind(&tenant.0)
        .bind(limit)
        .fetch_all(&state.db)
        .await?
    };

    let entries = rows
        .iter()
        .map(|r| -> Result<AuditEntry, sqlx::Error> {
            Ok(AuditEntry {
                id: r.try_get(0)?,
                action: r.try_get(1)?,
                peer_verify_key: r.try_get(2)?,
                details: r.try_get(3)?,
                timestamp: r.try_get(4)?,
                prev_hash: r.try_get(5)?,
                entry_hash: r.try_get(6)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(entries))
}

#[derive(Serialize)]
pub struct AuditVerifyResponse {
    /// True if every chained entry's hash matches its recomputed value and
    /// every `prev_hash` correctly links to the entry before it.
    pub valid: bool,
    /// Number of chained entries (non-NULL entry_hash) checked.
    pub checked: usize,
    /// Number of entries written before the hash chain existed — not
    /// covered by this check.
    pub unchained: usize,
    /// id of the first entry where the chain breaks, if any.
    pub first_break_id: Option<String>,
}

/// `GET /v1/audit/verify` — recomputes this tenant's BLAKE3 audit hash
/// chain server-side and reports whether it's intact. A `valid: false`
/// result means a row was altered, deleted, or reordered after being
/// written — evidence of database-level tampering that application-level
/// access controls alone cannot detect.
pub async fn verify_chain(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<AuditVerifyResponse>> {
    let rows = sqlx::query(
        "SELECT id, tenant_id, action, peer_verify_key, details, timestamp, prev_hash, entry_hash
         FROM audit_entries WHERE tenant_id=?
         ORDER BY timestamp ASC",
    )
    .bind(&tenant.0)
    .fetch_all(&state.db)
    .await?;

    let chain_rows = rows
        .iter()
        .map(|r| -> Result<ChainRow, sqlx::Error> {
            Ok(ChainRow {
                id: r.try_get(0)?,
                tenant_id: r.try_get(1)?,
                action: r.try_get(2)?,
                peer_verify_key: r.try_get(3)?,
                details: r.try_get(4)?,
                timestamp: r.try_get(5)?,
                prev_hash: r.try_get(6)?,
                entry_hash: r.try_get(7)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let result = crate::audit_log::verify_chain(&chain_rows);

    Ok(Json(AuditVerifyResponse {
        valid: result.valid,
        checked: result.checked,
        unchained: result.unchained,
        first_break_id: result.first_break_id,
    }))
}

#[derive(Serialize)]
pub struct AuditProofBundle {
    pub id: String,
    pub tenant_id: String,
    pub action: String,
    pub peer_verify_key: Option<String>,
    pub details: Option<String>,
    pub timestamp: i64,
    pub prev_hash: Option<String>,
    pub entry_hash: Option<String>,
    pub anchored: bool,
    pub merkle_root: Option<String>,
    pub merkle_index: Option<i64>,
    pub inclusion_proof: Option<Vec<ProofStepDto>>,
    pub anchor_signature: Option<String>,
    pub anchor_verify_key: Option<String>,
    pub ots_status: Option<String>,
    pub ots_proof: Option<String>,
}

/// `GET /v1/audit/:id/proof` — the full self-contained verification bundle
/// for one audit entry, the same shape as
/// `routes::decisions::proof`/`DecisionProofBundle`. If the entry predates
/// the hash chain (`entry_hash` is `None`) or hasn't been picked up by an
/// anchor cycle yet, `anchored` is `false` and the Merkle/anchor fields are
/// absent — call back later once a batch anchors.
pub async fn proof(
    State(state): State<AppState>,
    tenant: TenantId,
    Path(id): Path<String>,
) -> ApiResult<Json<AuditProofBundle>> {
    let row = sqlx::query(
        "SELECT action, peer_verify_key, details, timestamp, prev_hash, entry_hash, anchor_id, merkle_index
         FROM audit_entries WHERE id = ? AND tenant_id = ?",
    )
    .bind(&id)
    .bind(&tenant.0)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Audit entry not found".into()))?;

    let action: String = row.try_get(0)?;
    let peer_verify_key: Option<String> = row.try_get(1)?;
    let details: Option<String> = row.try_get(2)?;
    let timestamp: i64 = row.try_get(3)?;
    let prev_hash: Option<String> = row.try_get(4)?;
    let entry_hash: Option<String> = row.try_get(5)?;
    let anchor_id: Option<String> = row.try_get(6)?;
    let merkle_index: Option<i64> = row.try_get(7)?;

    let unanchored_bundle = |entry_hash: Option<String>| AuditProofBundle {
        id: id.clone(),
        tenant_id: tenant.0.clone(),
        action: action.clone(),
        peer_verify_key: peer_verify_key.clone(),
        details: details.clone(),
        timestamp,
        prev_hash: prev_hash.clone(),
        entry_hash,
        anchored: false,
        merkle_root: None,
        merkle_index: None,
        inclusion_proof: None,
        anchor_signature: None,
        anchor_verify_key: None,
        ots_status: None,
        ots_proof: None,
    };

    let Some(entry_hash_hex) = entry_hash else {
        // Pre-chain-migration row — nothing to anchor or prove beyond the
        // raw fields themselves.
        return Ok(Json(unanchored_bundle(None)));
    };

    let Some(anchor_id) = anchor_id else {
        return Ok(Json(unanchored_bundle(Some(entry_hash_hex))));
    };

    let anchor_row = sqlx::query(
        "SELECT merkle_root, anchor_signature, anchor_verify_key, ots_proof, ots_status
         FROM audit_anchors WHERE id = ?",
    )
    .bind(&anchor_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| {
        ApiError::Internal("anchor batch referenced by audit entry is missing".into())
    })?;
    let merkle_root: String = anchor_row.try_get(0)?;
    let anchor_signature: String = anchor_row.try_get(1)?;
    let anchor_verify_key: String = anchor_row.try_get(2)?;
    let ots_proof: Option<Vec<u8>> = anchor_row.try_get(3)?;
    let ots_status: String = anchor_row.try_get(4)?;

    let leaf_rows = sqlx::query(
        "SELECT entry_hash FROM audit_entries WHERE anchor_id = ? ORDER BY merkle_index ASC",
    )
    .bind(&anchor_id)
    .fetch_all(&state.db)
    .await?;

    let mut leaves = Vec::with_capacity(leaf_rows.len());
    for r in &leaf_rows {
        let hex_hash: String = r.try_get(0)?;
        let bytes = hex::decode(&hex_hash)
            .map_err(|_| ApiError::Internal("corrupt entry_hash in DB".into()))?;
        leaves.push(bytes);
    }

    let tree = MerkleTree::from_leaves(&leaves);
    if hex::encode(tree.root()) != merkle_root {
        return Err(ApiError::Internal(
            "recomputed Merkle root does not match stored anchor root".into(),
        ));
    }

    let index = merkle_index
        .ok_or_else(|| ApiError::Internal("anchored audit entry missing merkle_index".into()))?
        as usize;
    let inclusion_proof: Vec<ProofStepDto> = tree
        .inclusion_proof(index)
        .iter()
        .map(ProofStepDto::from)
        .collect();

    Ok(Json(AuditProofBundle {
        id,
        tenant_id: tenant.0,
        action,
        peer_verify_key,
        details,
        timestamp,
        prev_hash,
        entry_hash: Some(entry_hash_hex),
        anchored: true,
        merkle_root: Some(merkle_root),
        merkle_index: Some(index as i64),
        inclusion_proof: Some(inclusion_proof),
        anchor_signature: Some(anchor_signature),
        anchor_verify_key: Some(anchor_verify_key),
        ots_status: Some(ots_status),
        ots_proof: ots_proof.map(|b| BASE64.encode(b)),
    }))
}

#[derive(Deserialize)]
pub struct VerifyAuditProofRequest {
    pub id: String,
    pub tenant_id: String,
    pub action: String,
    #[serde(default)]
    pub peer_verify_key: Option<String>,
    #[serde(default)]
    pub details: Option<String>,
    pub timestamp: i64,
    pub prev_hash: String,
    pub entry_hash: String,
    #[serde(default)]
    pub merkle_root: Option<String>,
    #[serde(default)]
    pub inclusion_proof: Option<Vec<ProofStepDto>>,
    #[serde(default)]
    pub anchor_signature: Option<String>,
    #[serde(default)]
    pub anchor_verify_key: Option<String>,
}

#[derive(Serialize)]
pub struct VerifyAuditProofResponse {
    pub valid: bool,
    pub entry_hash_matches: bool,
    pub merkle_inclusion_valid: Option<bool>,
    pub anchor_signature_valid: Option<bool>,
    pub reason: Option<String>,
}

/// `POST /v1/audit/verify-proof` — pure verification of an
/// `AuditProofBundle`-shaped request. Deliberately takes no `TenantId`, no
/// `State`, makes no database call — the same "runnable by a third party
/// with zero trust in this server" design as
/// `routes::decisions::verify`, applied to the audit log instead of
/// decisions.
pub async fn verify_proof(
    Json(req): Json<VerifyAuditProofRequest>,
) -> Json<VerifyAuditProofResponse> {
    let recomputed_hex = crate::audit_log::compute_entry_hash(
        &req.prev_hash,
        &req.id,
        &req.tenant_id,
        &req.action,
        req.peer_verify_key.as_deref(),
        req.details.as_deref(),
        req.timestamp,
    );
    let entry_hash_matches = recomputed_hex == req.entry_hash;

    let claimed_hash_bytes = match hex::decode(&req.entry_hash) {
        Ok(b) if b.len() == 32 => b,
        _ => {
            return Json(VerifyAuditProofResponse {
                valid: false,
                entry_hash_matches: false,
                merkle_inclusion_valid: None,
                anchor_signature_valid: None,
                reason: Some("entry_hash must be 32-byte hex".into()),
            });
        }
    };

    let merkle_inclusion_valid = match (&req.merkle_root, &req.inclusion_proof) {
        (Some(root_hex), Some(proof_dtos)) => Some(
            (|| -> Option<bool> {
                let root_bytes: [u8; 32] = hex::decode(root_hex).ok()?.try_into().ok()?;
                let steps: Vec<ProofStep> = proof_dtos
                    .iter()
                    .map(ProofStep::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .ok()?;
                Some(merkle::verify_inclusion(
                    &claimed_hash_bytes,
                    &steps,
                    &root_bytes,
                ))
            })()
            .unwrap_or(false),
        ),
        _ => None,
    };

    let anchor_signature_valid = match (
        &req.merkle_root,
        &req.anchor_signature,
        &req.anchor_verify_key,
    ) {
        (Some(root_hex), Some(sig_b64), Some(vk_b64)) => Some(
            (|| -> Option<bool> {
                let root_bytes: [u8; 32] = hex::decode(root_hex).ok()?.try_into().ok()?;
                let vk_bytes: [u8; 32] = BASE64.decode(vk_b64).ok()?.try_into().ok()?;
                let sig_bytes: [u8; 64] = BASE64.decode(sig_b64).ok()?.try_into().ok()?;
                Some(crate::anchor_job::verify_anchor_signature(
                    &root_bytes,
                    &sig_bytes,
                    &vk_bytes,
                ))
            })()
            .unwrap_or(false),
        ),
        _ => None,
    };

    let valid = entry_hash_matches
        && merkle_inclusion_valid.unwrap_or(true)
        && anchor_signature_valid.unwrap_or(true);

    let reason = if !entry_hash_matches {
        Some(
            "recomputed entry_hash does not match — fields (or prev_hash) were tampered with"
                .to_string(),
        )
    } else if merkle_inclusion_valid == Some(false) {
        Some("inclusion proof does not verify against merkle_root".to_string())
    } else if anchor_signature_valid == Some(false) {
        Some("anchor signature does not verify against anchor_verify_key".to_string())
    } else if merkle_inclusion_valid.is_none() {
        Some(
            "entry_hash verified; not yet anchored, so timing and deletion-resistance \
             cannot be externally verified yet"
                .to_string(),
        )
    } else {
        None
    };

    Json(VerifyAuditProofResponse {
        valid,
        entry_hash_matches,
        merkle_inclusion_valid,
        anchor_signature_valid,
        reason,
    })
}
