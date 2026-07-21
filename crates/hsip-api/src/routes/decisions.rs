//! AI-agent decision attestations — sign, chain, anchor, and independently
//! verify a record of "this identity made this decision," without
//! requiring anyone to trust HSIP's own database for the parts that
//! matter: authorship (Ed25519 signature), tamper-evidence within a batch
//! (RFC 6962 Merkle inclusion), and tamper-evidence of the batch itself
//! over time (OpenTimestamps anchor — see `anchor_job.rs`).
//!
//! Two-tier record by design: `model_version`/`strategy_id`/
//! `accountable_key` are clear accountability metadata (HSIP's own draft
//! ahead of VCP-GOV, tagged `hsip_gov_ext`); `payload_hash` is opaque — the
//! actual decision content (trade parameters, etc.) is never sent to or
//! stored by HSIP, only its SHA-256 hash. Disclosure of the preimage, if
//! ever needed, happens entirely on the caller's side.
//!
//! `verify` is deliberately the one handler in this file that takes no
//! `TenantId` and no `State` — it is a pure function of its request body,
//! checkable by anyone (a regulator, an acquirer's engineering review,
//! Predicta itself) with zero calls back into this database. That's the
//! whole point of the feature.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use hsip_core::canonical::{
    accountable_proof_preimage_hash, event_hash, DecisionEnvelope, HSIP_GOV_EXT_VERSION,
};
use hsip_core::merkle::{self, MerkleTree, ProofStep, Side};

use super::identity::load_signing_key;
use super::sandbox::ms_to_iso;
use crate::{
    auth::{hash_key, TenantId},
    db::now_ms,
    errors::{ApiError, ApiResult},
    metrics,
    state::AppState,
};

const MAX_ATTEMPTS: u32 = 5;
const MAX_FIELD_LEN: usize = 128;
const MAX_DECISION_TYPE_LEN: usize = 64;

#[derive(Deserialize)]
pub struct RecordDecisionRequest {
    /// Verify key (base64) of whoever is accountable for this agent's
    /// decisions — usually the tenant's own identity, but may name a
    /// different registered human/tenant key when responsibility is split.
    pub accountable_key: String,
    pub model_version: String,
    pub strategy_id: String,
    pub decision_type: String,
    /// Hex-encoded SHA-256 of the caller's actual (undisclosed) decision payload.
    pub payload_hash: String,
    /// Optional base64 Ed25519 signature by `accountable_key`'s own private
    /// key, proving whoever is submitting this decision actually holds that
    /// key rather than merely naming it. Signs
    /// `hsip_core::canonical::accountable_proof_preimage_hash(accountable_key,
    /// tenant_id, model_version, strategy_id, decision_type, payload_hash)`
    /// — the caller can compute this and sign it entirely client-side
    /// before submitting, since none of those fields are server-assigned.
    /// Omitting this field keeps `accountable_key` exactly as
    /// caller-asserted metadata as it always was — this is additive, not a
    /// breaking requirement on existing callers.
    #[serde(default)]
    pub accountable_key_signature: Option<String>,
}

#[derive(Serialize)]
pub struct RecordDecisionResponse {
    pub decision_id: String,
    pub envelope: DecisionEnvelope,
    pub event_hash: String,
    pub signature: String,
    pub sign_algo: String,
    pub issuer_verify_key: String,
    /// Whether `accountable_key_signature` was supplied and verified. When
    /// `false`, `accountable_key` remains purely caller-asserted — same
    /// trust level as before this feature existed.
    pub accountable_key_verified: bool,
}

#[derive(Serialize)]
pub struct DecisionSummary {
    pub id: String,
    pub decision_type: String,
    pub model_version: String,
    pub strategy_id: String,
    pub event_hash: String,
    pub prev_hash: String,
    pub timestamp_iso: String,
    pub anchored: bool,
    pub anchor_id: Option<String>,
    pub merkle_index: Option<i64>,
    pub accountable_key_verified: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ProofStepDto {
    pub hash: String,
    pub position: String,
}

impl From<&ProofStep> for ProofStepDto {
    fn from(step: &ProofStep) -> Self {
        ProofStepDto {
            hash: hex::encode(step.hash),
            position: match step.side {
                Side::Left => "left".to_string(),
                Side::Right => "right".to_string(),
            },
        }
    }
}

impl TryFrom<&ProofStepDto> for ProofStep {
    type Error = ();
    fn try_from(dto: &ProofStepDto) -> Result<Self, ()> {
        let hash: [u8; 32] = hex::decode(&dto.hash)
            .map_err(|_| ())?
            .try_into()
            .map_err(|_| ())?;
        let side = match dto.position.as_str() {
            "left" => Side::Left,
            "right" => Side::Right,
            _ => return Err(()),
        };
        Ok(ProofStep { hash, side })
    }
}

#[derive(Serialize)]
pub struct DecisionProofBundle {
    pub envelope: DecisionEnvelope,
    pub event_hash: String,
    pub signature: String,
    pub sign_algo: String,
    pub issuer_verify_key: String,
    pub anchored: bool,
    pub merkle_root: Option<String>,
    pub merkle_index: Option<i64>,
    pub inclusion_proof: Option<Vec<ProofStepDto>>,
    pub anchor_signature: Option<String>,
    pub anchor_verify_key: Option<String>,
    pub ots_status: Option<String>,
    pub ots_proof: Option<String>,
    /// Whether `envelope.accountable_key_signature` verifies against
    /// `envelope.accountable_key` over `accountable_proof_preimage_hash`.
    /// `false` when no proof was ever supplied for this decision — same as
    /// `envelope.accountable_key_signature` being empty.
    pub accountable_key_verified: bool,
}

#[derive(Deserialize)]
pub struct VerifyDecisionRequest {
    pub envelope: DecisionEnvelope,
    pub event_hash: String,
    pub signature: String,
    pub issuer_verify_key: String,
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
pub struct VerifyDecisionResponse {
    pub valid: bool,
    pub event_hash_matches: bool,
    pub signature_valid: bool,
    pub merkle_inclusion_valid: Option<bool>,
    pub anchor_signature_valid: Option<bool>,
    /// `None` when `envelope.accountable_key_signature` is empty — no
    /// proof-of-possession was ever claimed for this decision, so there is
    /// nothing to check (this does not invalidate the bundle). `Some(false)`
    /// means a proof *was* claimed but does not verify — that does
    /// invalidate the bundle, same as a failed Merkle-inclusion or
    /// anchor-signature check.
    pub accountable_key_verified: Option<bool>,
    pub reason: Option<String>,
}

fn check_len(name: &str, value: &str, max: usize) -> ApiResult<()> {
    if value.is_empty() || value.len() > max {
        return Err(ApiError::BadRequest(format!(
            "{name} must be 1-{max} characters"
        )));
    }
    Ok(())
}

/// Checks `accountable_key_signature` against `accountable_key` over
/// `accountable_proof_preimage_hash`. Returns `None` when no signature was
/// supplied at all (empty string — nothing claimed, nothing to check) and
/// `Some(bool)` otherwise, including `Some(false)` for a malformed
/// (non-base64, wrong length) signature or key — a claimed-but-garbage
/// proof is a real verification failure, not "nothing to check." Shared by
/// `record` (verifying before persisting), `proof` (re-deriving for the
/// bundle), and `verify` (the independent, DB-free re-check) — a single
/// source of truth for this formula, same reasoning as
/// `audit_log::compute_entry_hash`.
fn verify_accountable_proof(
    accountable_key_b64: &str,
    accountable_key_signature: &str,
    tenant_id: &str,
    model_version: &str,
    strategy_id: &str,
    decision_type: &str,
    payload_hash: &str,
) -> Option<bool> {
    if accountable_key_signature.is_empty() {
        return None;
    }
    Some(
        (|| -> Option<bool> {
            let vk_bytes: [u8; 32] = BASE64.decode(accountable_key_b64).ok()?.try_into().ok()?;
            let vk = VerifyingKey::from_bytes(&vk_bytes).ok()?;
            let sig_bytes: [u8; 64] = BASE64
                .decode(accountable_key_signature)
                .ok()?
                .try_into()
                .ok()?;
            let sig = Signature::from_bytes(&sig_bytes);
            let digest = accountable_proof_preimage_hash(
                accountable_key_b64,
                tenant_id,
                model_version,
                strategy_id,
                decision_type,
                payload_hash,
            )
            .ok()?;
            Some(vk.verify(&digest, &sig).is_ok())
        })()
        .unwrap_or(false),
    )
}

fn validate_verify_key(name: &str, b64: &str) -> ApiResult<()> {
    let bytes: [u8; 32] = BASE64
        .decode(b64)
        .map_err(|_| ApiError::BadRequest(format!("{name} must be valid base64")))?
        .try_into()
        .map_err(|_| ApiError::BadRequest(format!("{name} must decode to 32 bytes")))?;
    VerifyingKey::from_bytes(&bytes)
        .map_err(|_| ApiError::BadRequest(format!("{name} is not a valid Ed25519 verify key")))?;
    Ok(())
}

/// `POST /v1/decisions` — sign and chain one AI-agent decision attestation.
///
/// Retries on a `UNIQUE(tenant_id, prev_hash)` conflict: that constraint is
/// what serializes each tenant's hash chain against concurrent requests,
/// so a conflict here means another request extended the chain first, not
/// a real error — re-reading the new tip and re-signing is the correct
/// response, up to `MAX_ATTEMPTS`.
pub async fn record(
    State(state): State<AppState>,
    tenant: TenantId,
    headers: HeaderMap,
    Json(req): Json<RecordDecisionRequest>,
) -> ApiResult<Json<RecordDecisionResponse>> {
    check_len("model_version", &req.model_version, MAX_FIELD_LEN)?;
    check_len("strategy_id", &req.strategy_id, MAX_FIELD_LEN)?;
    check_len("decision_type", &req.decision_type, MAX_DECISION_TYPE_LEN)?;
    validate_verify_key("accountable_key", &req.accountable_key)?;

    if req.payload_hash.len() != 64 || !req.payload_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ApiError::BadRequest(
            "payload_hash must be a 64-character hex-encoded SHA-256 digest".into(),
        ));
    }

    // Optional proof-of-possession: if the caller claims a signature,
    // reject up front (before touching the DB) rather than silently
    // recording accountable_key as verified when it isn't.
    let accountable_key_signature = req
        .accountable_key_signature
        .clone()
        .filter(|s| !s.is_empty());
    if let Some(sig_b64) = &accountable_key_signature {
        match verify_accountable_proof(
            &req.accountable_key,
            sig_b64,
            &tenant.0,
            &req.model_version,
            &req.strategy_id,
            &req.decision_type,
            &req.payload_hash,
        ) {
            Some(true) => {}
            _ => {
                return Err(ApiError::BadRequest(
                    "accountable_key_signature does not verify against accountable_key over \
                     this decision's content — proof-of-possession failed"
                        .into(),
                ));
            }
        }
    }
    let accountable_key_verified = accountable_key_signature.is_some();

    // Resolve which api_keys row actually authenticated this request — the
    // caller cannot claim to be a different agent than the one whose
    // credential is on this request.
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .ok_or_else(|| ApiError::Unauthorized("Missing Authorization: Bearer <key>".into()))?;
    let key_hash = hash_key(token);
    let key_row = sqlx::query("SELECT id FROM api_keys WHERE key_hash = $1 AND tenant_id = $2")
        .bind(&key_hash)
        .bind(&tenant.0)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::Internal("authenticated key vanished mid-request".into()))?;
    let agent_key_id: String = key_row.try_get(0)?;

    let signing_key = {
        let master_key = state.master_key.read().await;
        load_signing_key(&state.db, &tenant.0, &master_key).await?
    };
    let issuer_verify_b64 = BASE64.encode(signing_key.verifying_key().to_bytes());

    for attempt in 1..=MAX_ATTEMPTS {
        let now = now_ms();
        let prev_row = sqlx::query(
            "SELECT event_hash FROM decisions WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(&tenant.0)
        .fetch_optional(&state.db)
        .await?;
        let prev_hash: String = match prev_row {
            Some(r) => r.try_get(0)?,
            None => String::new(),
        };

        let decision_id = Uuid::new_v4().to_string();
        let envelope = DecisionEnvelope {
            decision_id: decision_id.clone(),
            tenant_id: tenant.0.clone(),
            agent_key_id: agent_key_id.clone(),
            accountable_key: req.accountable_key.clone(),
            model_version: req.model_version.clone(),
            strategy_id: req.strategy_id.clone(),
            decision_type: req.decision_type.clone(),
            payload_hash: req.payload_hash.clone(),
            prev_hash: prev_hash.clone(),
            timestamp_iso: ms_to_iso(now),
            // HSIP's clock resolution is milliseconds; scaled to nanoseconds
            // only to match VCP's dual timestamp format, not a claim of
            // nanosecond-precision measurement.
            timestamp_int: (now as i128 * 1_000_000).to_string(),
            hsip_gov_ext: HSIP_GOV_EXT_VERSION.to_string(),
            accountable_key_signature: accountable_key_signature.clone().unwrap_or_default(),
        };

        let event_hash_bytes = event_hash(&envelope).map_err(|e| {
            tracing::error!(error = %e, "canonicalization failed");
            ApiError::Internal("internal server error".into())
        })?;
        let event_hash_hex = hex::encode(event_hash_bytes);
        let signature = signing_key.sign(&event_hash_bytes);
        let sig_b64 = BASE64.encode(signature.to_bytes());

        let insert_result = sqlx::query(
            "INSERT INTO decisions
             (id, tenant_id, agent_key_id, accountable_key, model_version, strategy_id, decision_type,
              payload_hash, prev_hash, event_hash, signature, sign_algo, timestamp_iso, timestamp_int,
              hsip_gov_ext, anchor_id, merkle_index, created_at, accountable_key_signature)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'ED25519', $12, $13, $14, NULL, NULL, $15, $16)",
        )
        .bind(&decision_id)
        .bind(&tenant.0)
        .bind(&agent_key_id)
        .bind(&req.accountable_key)
        .bind(&req.model_version)
        .bind(&req.strategy_id)
        .bind(&req.decision_type)
        .bind(&req.payload_hash)
        .bind(&prev_hash)
        .bind(&event_hash_hex)
        .bind(&sig_b64)
        .bind(&envelope.timestamp_iso)
        .bind(&envelope.timestamp_int)
        .bind(&envelope.hsip_gov_ext)
        .bind(now)
        .bind(&envelope.accountable_key_signature)
        .execute(&state.db)
        .await;

        match insert_result {
            Ok(_) => {
                crate::audit_log::record_best_effort(
                    &state.db,
                    &tenant.0,
                    "decision.recorded",
                    None,
                    Some(&decision_id),
                    now,
                )
                .await;

                metrics::DECISIONS_RECORDED.inc();

                return Ok(Json(RecordDecisionResponse {
                    decision_id,
                    envelope,
                    event_hash: event_hash_hex,
                    signature: sig_b64,
                    sign_algo: "ED25519".to_string(),
                    issuer_verify_key: issuer_verify_b64,
                    accountable_key_verified,
                }));
            }
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                metrics::CHAIN_WRITE_RETRIES
                    .with_label_values(&["decisions"])
                    .inc();
                if attempt == MAX_ATTEMPTS {
                    return Err(ApiError::Conflict(
                        "could not extend decision chain after several attempts — high contention on this tenant's decision log".into(),
                    ));
                }
                crate::audit_log::chain_retry_backoff(attempt).await;
                continue;
            }
            Err(e) => return Err(e.into()),
        }
    }

    unreachable!("loop always returns or errors within MAX_ATTEMPTS");
}

/// `GET /v1/decisions` — list this tenant's decisions, newest first.
pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<Vec<DecisionSummary>>> {
    let rows = sqlx::query(
        "SELECT id, decision_type, model_version, strategy_id, event_hash, prev_hash,
                timestamp_iso, anchor_id, merkle_index, accountable_key_signature
         FROM decisions WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT 200",
    )
    .bind(&tenant.0)
    .fetch_all(&state.db)
    .await?;

    let records = rows
        .iter()
        .map(|r| -> Result<DecisionSummary, sqlx::Error> {
            let anchor_id: Option<String> = r.try_get(7)?;
            let accountable_key_signature: Option<String> = r.try_get(9)?;
            Ok(DecisionSummary {
                id: r.try_get(0)?,
                decision_type: r.try_get(1)?,
                model_version: r.try_get(2)?,
                strategy_id: r.try_get(3)?,
                event_hash: r.try_get(4)?,
                prev_hash: r.try_get(5)?,
                timestamp_iso: r.try_get(6)?,
                anchored: anchor_id.is_some(),
                anchor_id,
                merkle_index: r.try_get(8)?,
                accountable_key_verified: accountable_key_signature
                    .map(|s| !s.is_empty())
                    .unwrap_or(false),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(records))
}

/// `GET /v1/decisions/:id/proof` — the full self-contained verification
/// bundle for one decision. If it hasn't been anchored yet, the bundle
/// still proves authorship (signature) but `anchored` is `false` and the
/// Merkle/anchor fields are absent — call back later once a batch anchors.
pub async fn proof(
    State(state): State<AppState>,
    tenant: TenantId,
    Path(id): Path<String>,
) -> ApiResult<Json<DecisionProofBundle>> {
    let row = sqlx::query(
        "SELECT agent_key_id, accountable_key, model_version, strategy_id, decision_type,
                payload_hash, prev_hash, event_hash, signature, sign_algo, timestamp_iso,
                timestamp_int, hsip_gov_ext, anchor_id, merkle_index, accountable_key_signature
         FROM decisions WHERE id = $1 AND tenant_id = $2",
    )
    .bind(&id)
    .bind(&tenant.0)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Decision not found".into()))?;

    let accountable_key_signature: Option<String> = row.try_get(15)?;
    let envelope = DecisionEnvelope {
        decision_id: id.clone(),
        tenant_id: tenant.0.clone(),
        agent_key_id: row.try_get(0)?,
        accountable_key: row.try_get(1)?,
        model_version: row.try_get(2)?,
        strategy_id: row.try_get(3)?,
        decision_type: row.try_get(4)?,
        payload_hash: row.try_get(5)?,
        prev_hash: row.try_get(6)?,
        timestamp_iso: row.try_get(10)?,
        timestamp_int: row.try_get(11)?,
        hsip_gov_ext: row.try_get(12)?,
        accountable_key_signature: accountable_key_signature.unwrap_or_default(),
    };
    let event_hash_hex: String = row.try_get(7)?;
    let signature: String = row.try_get(8)?;
    let sign_algo: String = row.try_get(9)?;
    let anchor_id: Option<String> = row.try_get(13)?;
    let merkle_index: Option<i64> = row.try_get(14)?;
    let accountable_key_verified = verify_accountable_proof(
        &envelope.accountable_key,
        &envelope.accountable_key_signature,
        &tenant.0,
        &envelope.model_version,
        &envelope.strategy_id,
        &envelope.decision_type,
        &envelope.payload_hash,
    )
    .unwrap_or(false);

    let ident_row = sqlx::query("SELECT verify_key_b64 FROM identities WHERE tenant_id = $1")
        .bind(&tenant.0)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| {
            ApiError::Internal("identity missing for tenant with recorded decisions".into())
        })?;
    let issuer_verify_key: String = ident_row.try_get(0)?;

    let Some(anchor_id) = anchor_id else {
        return Ok(Json(DecisionProofBundle {
            envelope,
            event_hash: event_hash_hex,
            signature,
            sign_algo,
            issuer_verify_key,
            anchored: false,
            merkle_root: None,
            merkle_index: None,
            inclusion_proof: None,
            anchor_signature: None,
            anchor_verify_key: None,
            ots_status: None,
            ots_proof: None,
            accountable_key_verified,
        }));
    };

    let anchor_row = sqlx::query(
        "SELECT merkle_root, anchor_signature, anchor_verify_key, ots_proof, ots_status
         FROM decision_anchors WHERE id = $1",
    )
    .bind(&anchor_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::Internal("anchor batch referenced by decision is missing".into()))?;
    let merkle_root: String = anchor_row.try_get(0)?;
    let anchor_signature: String = anchor_row.try_get(1)?;
    let anchor_verify_key: String = anchor_row.try_get(2)?;
    let ots_proof: Option<Vec<u8>> = anchor_row.try_get(3)?;
    let ots_status: String = anchor_row.try_get(4)?;

    let leaf_rows = sqlx::query(
        "SELECT event_hash FROM decisions WHERE anchor_id = $1 ORDER BY merkle_index ASC",
    )
    .bind(&anchor_id)
    .fetch_all(&state.db)
    .await?;

    let mut leaves = Vec::with_capacity(leaf_rows.len());
    for r in &leaf_rows {
        let hex_hash: String = r.try_get(0)?;
        let bytes = hex::decode(&hex_hash)
            .map_err(|_| ApiError::Internal("corrupt event_hash in DB".into()))?;
        leaves.push(bytes);
    }

    let tree = MerkleTree::from_leaves(&leaves);
    if hex::encode(tree.root()) != merkle_root {
        return Err(ApiError::Internal(
            "recomputed Merkle root does not match stored anchor root".into(),
        ));
    }

    let index = merkle_index
        .ok_or_else(|| ApiError::Internal("anchored decision missing merkle_index".into()))?
        as usize;
    let inclusion_proof: Vec<ProofStepDto> = tree
        .inclusion_proof(index)
        .iter()
        .map(ProofStepDto::from)
        .collect();

    Ok(Json(DecisionProofBundle {
        envelope,
        event_hash: event_hash_hex,
        signature,
        sign_algo,
        issuer_verify_key,
        anchored: true,
        merkle_root: Some(merkle_root),
        merkle_index: Some(index as i64),
        inclusion_proof: Some(inclusion_proof),
        anchor_signature: Some(anchor_signature),
        anchor_verify_key: Some(anchor_verify_key),
        ots_status: Some(ots_status),
        ots_proof: ots_proof.map(|b| BASE64.encode(b)),
        accountable_key_verified,
    }))
}

/// `POST /v1/decisions/verify` — pure verification of a self-contained
/// proof bundle. Deliberately takes no `TenantId`, no `State`, makes no
/// database call: this is the function a third party runs, not a
/// convenience wrapper around trusting this server.
pub async fn verify(Json(req): Json<VerifyDecisionRequest>) -> Json<VerifyDecisionResponse> {
    let recomputed = match event_hash(&req.envelope) {
        Ok(h) => h,
        Err(_) => {
            return Json(VerifyDecisionResponse {
                valid: false,
                event_hash_matches: false,
                signature_valid: false,
                merkle_inclusion_valid: None,
                anchor_signature_valid: None,
                accountable_key_verified: None,
                reason: Some("envelope failed to canonicalize".into()),
            });
        }
    };

    let claimed_hash_bytes = match hex::decode(&req.event_hash) {
        Ok(b) if b.len() == 32 => b,
        _ => {
            return Json(VerifyDecisionResponse {
                valid: false,
                event_hash_matches: false,
                signature_valid: false,
                merkle_inclusion_valid: None,
                anchor_signature_valid: None,
                accountable_key_verified: None,
                reason: Some("event_hash must be 32-byte hex".into()),
            });
        }
    };
    let event_hash_matches = recomputed.as_slice() == claimed_hash_bytes.as_slice();

    let signature_valid = (|| -> Option<bool> {
        let vk_bytes: [u8; 32] = BASE64
            .decode(&req.issuer_verify_key)
            .ok()?
            .try_into()
            .ok()?;
        let vk = VerifyingKey::from_bytes(&vk_bytes).ok()?;
        let sig_bytes: [u8; 64] = BASE64.decode(&req.signature).ok()?.try_into().ok()?;
        let sig = Signature::from_bytes(&sig_bytes);
        Some(vk.verify(&recomputed, &sig).is_ok())
    })()
    .unwrap_or(false);

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

    let accountable_key_verified = verify_accountable_proof(
        &req.envelope.accountable_key,
        &req.envelope.accountable_key_signature,
        &req.envelope.tenant_id,
        &req.envelope.model_version,
        &req.envelope.strategy_id,
        &req.envelope.decision_type,
        &req.envelope.payload_hash,
    );

    let valid = event_hash_matches
        && signature_valid
        && merkle_inclusion_valid.unwrap_or(true)
        && anchor_signature_valid.unwrap_or(true)
        && accountable_key_verified.unwrap_or(true);

    let reason = if !event_hash_matches {
        Some("envelope does not match claimed event_hash — payload was tampered with".to_string())
    } else if !signature_valid {
        Some("Ed25519 signature does not verify against issuer_verify_key".to_string())
    } else if merkle_inclusion_valid == Some(false) {
        Some("inclusion proof does not verify against merkle_root".to_string())
    } else if anchor_signature_valid == Some(false) {
        Some("anchor signature does not verify against anchor_verify_key".to_string())
    } else if accountable_key_verified == Some(false) {
        Some(
            "accountable_key_signature does not verify against accountable_key — \
             proof-of-possession failed"
                .to_string(),
        )
    } else if merkle_inclusion_valid.is_none() {
        Some(
            "signature and envelope verified; not yet anchored, so timing and \
             deletion-resistance cannot be externally verified yet"
                .to_string(),
        )
    } else {
        None
    };

    metrics::DECISIONS_VERIFIED
        .with_label_values(&[if valid { "valid" } else { "invalid" }])
        .inc();

    Json(VerifyDecisionResponse {
        valid,
        event_hash_matches,
        signature_valid,
        merkle_inclusion_valid,
        anchor_signature_valid,
        accountable_key_verified,
        reason,
    })
}
