use axum::{extract::State, http::HeaderMap, Json};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Serialize;
use std::sync::atomic::Ordering;
use uuid::Uuid;

use crate::{
    auth::hash_key,
    db::{now_ms, Db},
    errors::{ApiError, ApiResult},
    state::{AppState, RateWindow},
};

const PROVISIONS_PER_HOUR: u64 = 5;
const HOUR_MS: i64 = 3_600_000;
const TRIAL_DURATION_MS: i64 = 86_400_000; // 24 h
/// Hard cap on simultaneously-live sandbox tenants. Prevents DB/memory bloat if
/// many people hit the demo at the same time.
const MAX_ACTIVE_SANDBOXES: i64 = 300;

#[derive(Serialize)]
pub struct ProvisionResponse {
    pub api_key: String,
    pub expires_at: String,
    pub expires_at_ms: i64,
    pub base_url: String,
    pub note: String,
    pub quickstart: Quickstart,
}

#[derive(Serialize)]
pub struct Quickstart {
    pub step1_sign_message: String,
    pub step2_get_identity: String,
    pub step3_view_audit_trail: String,
    pub step4_grant_consent: String,
    pub step5_agent_capabilities: String,
}

/// POST /v1/sandbox/provision
///
/// No auth required. Creates an isolated tenant + 24-hour trial API key.
/// Only active when HSIP_SANDBOX=true env var is set.
/// Rate-limited to PROVISIONS_PER_HOUR per source IP.
pub async fn provision(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<ProvisionResponse>> {
    if std::env::var("HSIP_SANDBOX").as_deref() != Ok("true") {
        return Err(ApiError::NotFound(
            "Sandbox mode is not enabled on this server.".into(),
        ));
    }

    let ip = client_ip(&headers);
    check_provision_rate(&ip, &state)?;

    // Run expired-sandbox cleanup in the background so it doesn't slow the response.
    let db_clone = state.db.clone();
    tokio::spawn(async move {
        cleanup_expired_sandboxes(&db_clone).await;
    });

    // Enforce active-tenant cap after cleanup has run (best-effort; races are fine).
    let active = count_active_sandboxes(&state.db).await;
    if active >= MAX_ACTIVE_SANDBOXES {
        return Err(ApiError::TooManyRequests(
            "Demo capacity temporarily full. Try again in a few minutes.".into(),
        ));
    }

    let now = now_ms();
    let expires_at_ms = now + TRIAL_DURATION_MS;

    // Isolated tenant per trial user.
    let tenant_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO tenants (id, name, created_at) VALUES (?, ?, ?)")
        .bind(&tenant_id)
        .bind(format!("sandbox-{}", &tenant_id[..8]))
        .bind(now)
        .execute(&state.db)
        .await?;

    // Trial key — expires in 24 hours.
    let mut raw_bytes = [0u8; 32];
    OsRng.fill_bytes(&mut raw_bytes);
    let raw_key = format!("hsip_{}", hex::encode(raw_bytes));
    let key_hash = hash_key(&raw_key);
    let key_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO api_keys
         (id, tenant_id, key_hash, name, agent_type, created_at, expires_at, active)
         VALUES (?, ?, ?, 'sandbox-trial', 'human', ?, ?, 1)",
    )
    .bind(&key_id)
    .bind(&tenant_id)
    .bind(&key_hash)
    .bind(now)
    .bind(expires_at_ms)
    .execute(&state.db)
    .await?;

    // Audit trail entry for this provision event.
    crate::audit_log::record(
        &state.db,
        &tenant_id,
        "sandbox.provision",
        None,
        Some(&format!("trial_key_issued ip={ip}")),
        now,
    )
    .await?;

    let base_url =
        std::env::var("HSIP_PUBLIC_URL").unwrap_or_else(|_| "https://demo.hsip.io".to_string());

    let key = raw_key.clone();
    let qs = Quickstart {
        step1_sign_message: format!(
            "curl -X POST {base_url}/v1/messages/sign \\\n\
             \x20 -H \"Authorization: Bearer {key}\" \\\n\
             \x20 -H \"Content-Type: application/json\" \\\n\
             \x20 -d '{{\"content\": \"I authorize this action.\"}}'"
        ),
        step2_get_identity: format!(
            "curl -X POST {base_url}/v1/identity \\\n\
             \x20 -H \"Authorization: Bearer {key}\" \\\n\
             \x20 -H \"Content-Type: application/json\" \\\n\
             \x20 -d '{{}}'"
        ),
        step3_view_audit_trail: format!(
            "curl \"{base_url}/v1/audit\" \\\n\
             \x20 -H \"Authorization: Bearer {key}\""
        ),
        step4_grant_consent: format!(
            "curl -X POST {base_url}/v1/consent/grant \\\n\
             \x20 -H \"Authorization: Bearer {key}\" \\\n\
             \x20 -H \"Content-Type: application/json\" \\\n\
             \x20 -d '{{\"peer_verify_key\":\"<counterparty_pubkey>\",\"scope\":\"contact\",\"expires_in_seconds\":3600}}'"
        ),
        step5_agent_capabilities: format!(
            "curl \"{base_url}/v1/agent/capabilities\" \\\n\
             \x20 -H \"Authorization: Bearer {key}\""
        ),
    };

    Ok(Json(ProvisionResponse {
        api_key: raw_key,
        expires_at: ms_to_iso(expires_at_ms),
        expires_at_ms,
        base_url,
        note: "Trial key expires in 24 hours. Each visitor gets their own isolated environment."
            .into(),
        quickstart: qs,
    }))
}

// ── Cleanup ───────────────────────────────────────────────────────────────────

/// Delete all sandbox tenants whose trial key has expired. Runs on every
/// provision call (in the background) so the DB never accumulates stale rows.
async fn cleanup_expired_sandboxes(db: &Db) {
    let now = now_ms();

    // Collect expired sandbox tenant IDs first so we can safely delete api_keys
    // without the subquery reference breaking mid-transaction.
    let rows = sqlx::query(
        "SELECT t.id FROM tenants t
         WHERE t.name LIKE 'sandbox-%'
         AND NOT EXISTS (
           SELECT 1 FROM api_keys k
           WHERE k.tenant_id = t.id
           AND k.active = 1
           AND (k.expires_at IS NULL OR k.expires_at > ?)
         )",
    )
    .bind(now)
    .fetch_all(db)
    .await;

    let ids: Vec<String> = match rows {
        Ok(r) => r
            .iter()
            .filter_map(|row| {
                use sqlx::Row;
                row.try_get::<String, _>(0).ok()
            })
            .collect(),
        Err(_) => return,
    };

    if ids.is_empty() {
        return;
    }

    // Delete all tenant data in dependency order. Errors are silently ignored
    // (cleanup is best-effort; the next provision call will retry).
    for id in &ids {
        let _ = sqlx::query("DELETE FROM messages      WHERE tenant_id = ?")
            .bind(id)
            .execute(db)
            .await;
        let _ = sqlx::query("DELETE FROM consents      WHERE tenant_id = ?")
            .bind(id)
            .execute(db)
            .await;
        let _ = sqlx::query("DELETE FROM identities    WHERE tenant_id = ?")
            .bind(id)
            .execute(db)
            .await;
        let _ = sqlx::query("DELETE FROM contacts      WHERE tenant_id = ?")
            .bind(id)
            .execute(db)
            .await;
        let _ = sqlx::query("DELETE FROM credentials   WHERE tenant_id = ?")
            .bind(id)
            .execute(db)
            .await;
        let _ = sqlx::query("DELETE FROM audit_entries WHERE tenant_id = ?")
            .bind(id)
            .execute(db)
            .await;
        let _ = sqlx::query("DELETE FROM trusted_peers WHERE tenant_id = ?")
            .bind(id)
            .execute(db)
            .await;
        let _ = sqlx::query("DELETE FROM uploads       WHERE tenant_id = ?")
            .bind(id)
            .execute(db)
            .await;
        let _ = sqlx::query("DELETE FROM api_keys      WHERE tenant_id = ?")
            .bind(id)
            .execute(db)
            .await;
        let _ = sqlx::query("DELETE FROM tenants       WHERE id = ?")
            .bind(id)
            .execute(db)
            .await;
    }
}

/// Count sandbox tenants that still have an active, non-expired trial key.
async fn count_active_sandboxes(db: &Db) -> i64 {
    use sqlx::Row;
    let now = now_ms();
    sqlx::query(
        "SELECT COUNT(DISTINCT t.id) FROM tenants t
         JOIN api_keys k ON k.tenant_id = t.id
         WHERE t.name LIKE 'sandbox-%'
         AND k.active = 1
         AND k.expires_at > ?",
    )
    .bind(now)
    .fetch_one(db)
    .await
    .and_then(|r| r.try_get::<i64, _>(0))
    .unwrap_or(0)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn check_provision_rate(ip: &str, state: &AppState) -> Result<(), ApiError> {
    let now = now_ms();
    let limiter = &state.sandbox_rate;

    let count = if let Some(win) = limiter.get(ip) {
        let ws = win.window_start_ms.load(Ordering::SeqCst);
        if now - ws > HOUR_MS {
            win.window_start_ms.store(now, Ordering::SeqCst);
            win.count.store(1, Ordering::SeqCst);
            1u64
        } else {
            win.count.fetch_add(1, Ordering::SeqCst) + 1
        }
    } else {
        limiter.insert(ip.to_string(), RateWindow::new(now));
        1u64
    };

    if count > PROVISIONS_PER_HOUR {
        return Err(ApiError::TooManyRequests(format!(
            "Sandbox limit: {PROVISIONS_PER_HOUR} trial keys per IP per hour. Try again later."
        )));
    }
    Ok(())
}

/// Convert a Unix millisecond timestamp to ISO 8601 UTC string without chrono.
pub(crate) fn ms_to_iso(ms: i64) -> String {
    let secs = (ms / 1000) as u64;
    let h = (secs % 86400) / 3600;
    let mi = (secs % 3600) / 60;
    let s = secs % 60;
    let mut days = secs / 86400;
    let mut year = 1970u32;
    loop {
        let dy: u64 = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
            366
        } else {
            365
        };
        if days < dy {
            break;
        }
        days -= dy;
        year += 1;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u32;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    format!(
        "{year:04}-{month:02}-{:02}T{h:02}:{mi:02}:{s:02}Z",
        days + 1
    )
}
