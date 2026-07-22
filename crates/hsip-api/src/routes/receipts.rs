//! Receipt collection — lets a business run HSIP purely locally on every
//! employee's/agent's own machine, and still get one centralized audit
//! trail, *without* running a shared database that holds everyone's raw
//! operational data.
//!
//! A "collector" is just an ordinary HSIP instance whose owner accepts
//! `POST /v1/receipts/submit` calls from other, independent HSIP instances.
//! Each submission is one self-contained proof bundle — exactly what
//! `GET /v1/decisions/:id/proof` or `GET /v1/audit/:id/proof` already
//! return on the *submitting* instance: hashes, an Ed25519 signature, a
//! public verify key, and (once anchored) a Merkle inclusion proof and
//! anchor signature. It never contains the actual decision payload (HSIP
//! never receives that anywhere, on any instance) or any private key
//! material. The collector independently re-verifies every submission
//! using the exact same DB-free verification logic a third party would
//! run (`routes::decisions::verify` / `routes::audit::verify_proof`)
//! before ever storing it — a bundle that doesn't verify is rejected, not
//! recorded.
//!
//! The result: the one shared, network-reachable component in this
//! design holds a small, append-only log of already-verified receipts,
//! not a full copy of every submitting instance's messages, credentials,
//! consents, or identities. Compromising the collector discloses which
//! decisions were made and by which (already-public) verify keys — not
//! the sensitive operational data those decisions were about, none of
//! which the collector ever had.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::audit::VerifyAuditProofRequest;
use super::decisions::VerifyDecisionRequest;
use crate::{
    auth::TenantId,
    db::now_ms,
    errors::{ApiError, ApiResult},
    state::AppState,
};

const MAX_LABEL_LEN: usize = 128;

#[derive(Deserialize)]
pub struct SubmitReceiptRequest {
    /// Human-readable, caller-supplied label for whichever local instance
    /// is submitting (e.g. "alice-laptop", "trading-bot-3") — informational
    /// only, not a cryptographic identity claim. `accountable_key` inside
    /// the bundle itself (for decision receipts) is what's actually
    /// verified.
    pub submitter_label: String,
    /// "decision" or "audit" — which proof-bundle shape `bundle` is.
    pub receipt_type: String,
    /// The exact JSON body returned by the submitting instance's own
    /// `GET /v1/decisions/:id/proof` (for `receipt_type: "decision"`) or
    /// `GET /v1/audit/:id/proof` (for `receipt_type: "audit"`).
    pub bundle: serde_json::Value,
}

#[derive(Serialize)]
pub struct SubmitReceiptResponse {
    pub id: String,
    pub valid: bool,
    pub source_tenant_id: String,
    pub source_record_id: String,
}

#[derive(Serialize)]
pub struct ReceiptSummary {
    pub id: String,
    pub submitter_label: String,
    pub receipt_type: String,
    pub source_tenant_id: String,
    pub source_record_id: String,
    pub valid: bool,
    pub submitted_at: i64,
}

#[derive(Serialize)]
pub struct ReceiptDetail {
    pub id: String,
    pub submitter_label: String,
    pub receipt_type: String,
    pub source_tenant_id: String,
    pub source_record_id: String,
    pub valid: bool,
    pub submitted_at: i64,
    pub bundle: serde_json::Value,
}

/// `POST /v1/receipts/submit` — verify and store one proof bundle from
/// another, independent HSIP instance. Re-verification reuses the exact
/// same DB-free logic a third party would run, so a tampered or garbage
/// bundle is rejected (`400`) before it ever reaches storage — this table
/// is meant to be a trustworthy log of genuinely-verified receipts, not a
/// raw inbox of caller claims.
pub async fn submit(
    State(state): State<AppState>,
    tenant: TenantId,
    Json(req): Json<SubmitReceiptRequest>,
) -> ApiResult<Json<SubmitReceiptResponse>> {
    if req.submitter_label.is_empty() || req.submitter_label.len() > MAX_LABEL_LEN {
        return Err(ApiError::BadRequest(format!(
            "submitter_label must be 1-{MAX_LABEL_LEN} characters"
        )));
    }

    let (valid, source_tenant_id, source_record_id) = match req.receipt_type.as_str() {
        "decision" => {
            let verify_req: VerifyDecisionRequest = serde_json::from_value(req.bundle.clone())
                .map_err(|e| {
                    ApiError::BadRequest(format!(
                        "bundle is not a valid decision proof bundle: {e}"
                    ))
                })?;
            let source_tenant_id = verify_req.envelope.tenant_id.clone();
            let source_record_id = verify_req.envelope.decision_id.clone();
            let result = super::decisions::verify(Json(verify_req)).await;
            (result.0.valid, source_tenant_id, source_record_id)
        }
        "audit" => {
            let verify_req: VerifyAuditProofRequest = serde_json::from_value(req.bundle.clone())
                .map_err(|e| {
                    ApiError::BadRequest(format!("bundle is not a valid audit proof bundle: {e}"))
                })?;
            let source_tenant_id = verify_req.tenant_id.clone();
            let source_record_id = verify_req.id.clone();
            let result = super::audit::verify_proof(Json(verify_req)).await;
            (result.0.valid, source_tenant_id, source_record_id)
        }
        other => {
            return Err(ApiError::BadRequest(format!(
                "receipt_type must be \"decision\" or \"audit\", got \"{other}\""
            )));
        }
    };

    if !valid {
        return Err(ApiError::BadRequest(
            "submitted bundle does not verify — rejected, not stored".into(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let now = now_ms();
    let bundle_json = req.bundle.to_string();

    let insert_result = sqlx::query(
        "INSERT INTO submitted_receipts
         (id, collector_tenant_id, submitter_label, receipt_type, source_tenant_id, source_record_id, bundle_json, valid, submitted_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(&id)
    .bind(&tenant.0)
    .bind(&req.submitter_label)
    .bind(&req.receipt_type)
    .bind(&source_tenant_id)
    .bind(&source_record_id)
    .bind(&bundle_json)
    .bind(1i64)
    .bind(now)
    .execute(&state.db)
    .await;

    match insert_result {
        Ok(_) => {
            crate::audit_log::record_best_effort(
                &state.db,
                &tenant.0,
                "receipt.submitted",
                None,
                Some(&id),
                now,
            )
            .await;

            Ok(Json(SubmitReceiptResponse {
                id,
                valid: true,
                source_tenant_id,
                source_record_id,
            }))
        }
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => Err(
            ApiError::Conflict("this receipt has already been submitted to this collector".into()),
        ),
        Err(e) => Err(e.into()),
    }
}

/// `GET /v1/receipts` — list receipts submitted to this collector tenant,
/// newest first. Summary only (no bundle body) — use `GET /v1/receipts/:id`
/// for the full proof.
pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<Vec<ReceiptSummary>>> {
    let rows = sqlx::query(
        "SELECT id, submitter_label, receipt_type, source_tenant_id, source_record_id, valid, submitted_at
         FROM submitted_receipts WHERE collector_tenant_id = $1
         ORDER BY submitted_at DESC LIMIT 500",
    )
    .bind(&tenant.0)
    .fetch_all(&state.db)
    .await?;

    let records = rows
        .iter()
        .map(|r| -> Result<ReceiptSummary, sqlx::Error> {
            Ok(ReceiptSummary {
                id: r.try_get(0)?,
                submitter_label: r.try_get(1)?,
                receipt_type: r.try_get(2)?,
                source_tenant_id: r.try_get(3)?,
                source_record_id: r.try_get(4)?,
                valid: r.try_get::<i64, _>(5)? != 0,
                submitted_at: r.try_get(6)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(records))
}

/// `GET /v1/receipts/:id` — full detail for one submitted receipt,
/// including the original bundle, for a deeper audit.
pub async fn get_one(
    State(state): State<AppState>,
    tenant: TenantId,
    Path(id): Path<String>,
) -> ApiResult<Json<ReceiptDetail>> {
    let row = sqlx::query(
        "SELECT id, submitter_label, receipt_type, source_tenant_id, source_record_id, bundle_json, valid, submitted_at
         FROM submitted_receipts WHERE id = $1 AND collector_tenant_id = $2",
    )
    .bind(&id)
    .bind(&tenant.0)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Receipt not found".into()))?;

    let bundle_json: String = row.try_get(5)?;
    let bundle: serde_json::Value = serde_json::from_str(&bundle_json).map_err(|e| {
        tracing::error!(error = %e, "corrupt bundle_json in submitted_receipts");
        ApiError::Internal("internal server error".into())
    })?;

    Ok(Json(ReceiptDetail {
        id: row.try_get(0)?,
        submitter_label: row.try_get(1)?,
        receipt_type: row.try_get(2)?,
        source_tenant_id: row.try_get(3)?,
        source_record_id: row.try_get(4)?,
        valid: row.try_get::<i64, _>(6)? != 0,
        submitted_at: row.try_get(7)?,
        bundle,
    }))
}
