//! Stripe billing: Checkout subscriptions, the customer portal and the
//! webhook that applies plan changes. Talks to Stripe's REST API directly
//! (no SDK). The webhook is the only thing that changes a plan, and only after
//! its `Stripe-Signature` verifies; the redirect back from Checkout grants
//! nothing.

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::Sha256;
use sqlx::Row;

use crate::{
    audit, auth,
    config::StripeCfg,
    db::now_ms,
    errors::{ApiError, ApiResult},
    plans,
    state::AppState,
};

const TOLERANCE_SECS: i64 = 300;

fn cfg(s: &AppState) -> ApiResult<&StripeCfg> {
    s.cfg
        .stripe
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("online billing is not configured".into()))
}

fn price_for<'a>(c: &'a StripeCfg, plan: &str) -> Option<&'a str> {
    match plan {
        "team" => c.price_team.as_deref(),
        "business" => c.price_business.as_deref(),
        _ => None,
    }
}

fn plan_for_price(c: &StripeCfg, price: &str) -> Option<&'static str> {
    if c.price_team.as_deref() == Some(price) {
        Some("team")
    } else if c.price_business.as_deref() == Some(price) {
        Some("business")
    } else {
        None
    }
}

async fn stripe_post(c: &StripeCfg, path: &str, form: &[(&str, String)]) -> ApiResult<Value> {
    let res = reqwest::Client::new()
        .post(format!("{}{}", c.api_url, path))
        .basic_auth(&c.secret_key, Some(""))
        .timeout(std::time::Duration::from_secs(15))
        .form(form)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "stripe request failed");
            ApiError::internal()
        })?;
    let ok = res.status().is_success();
    let body = res.text().await.unwrap_or_default();
    if !ok {
        tracing::error!(body = %body, "stripe returned an error");
        return Err(ApiError::Internal(
            "the payment provider is unavailable".into(),
        ));
    }
    serde_json::from_str(&body).map_err(|_| ApiError::internal())
}

#[derive(Deserialize)]
pub struct CheckoutReq {
    pub plan: String,
}

/// Start a Stripe Checkout subscription for `team` or `business`.
pub async fn checkout(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
    Json(r): Json<CheckoutReq>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    m.require_owner()?;
    let c = cfg(&s)?;
    let price = price_for(c, &r.plan).ok_or_else(|| {
        ApiError::BadRequest("that plan is not available for online checkout".into())
    })?;
    let row = sqlx::query("SELECT stripe_customer_id FROM organizations WHERE id = $1")
        .bind(&org)
        .fetch_one(&s.db)
        .await?;
    let customer: Option<String> = row.try_get(0).ok().flatten();
    let back = format!("{}/app/{}/billing", s.cfg.public_url, org);
    let mut form = vec![
        ("mode", "subscription".to_string()),
        ("line_items[0][price]", price.to_string()),
        ("line_items[0][quantity]", "1".into()),
        ("client_reference_id", org.clone()),
        ("metadata[org_id]", org.clone()),
        ("metadata[plan]", r.plan.clone()),
        ("subscription_data[metadata][org_id]", org.clone()),
        ("success_url", format!("{back}?checkout=success")),
        ("cancel_url", format!("{back}?checkout=cancelled")),
    ];
    match customer {
        Some(cu) => form.push(("customer", cu)),
        None => form.push(("customer_email", m.user.email.clone())),
    }
    let v = stripe_post(c, "/v1/checkout/sessions", &form).await?;
    let url = v["url"].as_str().ok_or_else(ApiError::internal)?;
    audit::log(
        &s,
        &org,
        &m.actor(),
        "billing.checkout_started",
        &r.plan,
        "",
    )
    .await;
    Ok(Json(json!({ "url": url })))
}

/// Stripe customer portal: change card, switch plan, cancel.
pub async fn portal(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(org): Path<String>,
) -> ApiResult<Json<Value>> {
    let m = auth::member(&s, &headers, &org).await?;
    m.require_owner()?;
    let c = cfg(&s)?;
    let row = sqlx::query("SELECT stripe_customer_id FROM organizations WHERE id = $1")
        .bind(&org)
        .fetch_one(&s.db)
        .await?;
    let customer: Option<String> = row.try_get(0).ok().flatten();
    let customer =
        customer.ok_or_else(|| ApiError::BadRequest("no subscription on file yet".into()))?;
    let form = [
        ("customer", customer),
        (
            "return_url",
            format!("{}/app/{}/billing", s.cfg.public_url, org),
        ),
    ];
    let v = stripe_post(c, "/v1/billing_portal/sessions", &form).await?;
    let url = v["url"].as_str().ok_or_else(ApiError::internal)?;
    Ok(Json(json!({ "url": url })))
}

/// Verify a `Stripe-Signature` header (`t=...,v1=...`) over `{t}.{payload}`.
pub fn verify_signature(secret: &str, header: &str, payload: &[u8], now_secs: i64) -> bool {
    let mut t: Option<i64> = None;
    let mut sigs: Vec<Vec<u8>> = Vec::new();
    for part in header.split(',') {
        match part.trim().split_once('=') {
            Some(("t", v)) => t = v.parse().ok(),
            Some(("v1", v)) => {
                if let Ok(b) = hex::decode(v) {
                    sigs.push(b)
                }
            }
            _ => {}
        }
    }
    let Some(t) = t else { return false };
    if (now_secs - t).abs() > TOLERANCE_SECS {
        return false;
    }
    sigs.iter().any(|sig| {
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
            return false;
        };
        mac.update(t.to_string().as_bytes());
        mac.update(b".");
        mac.update(payload);
        mac.verify_slice(sig).is_ok()
    })
}

async fn owner_email(s: &AppState, org: &str) -> Option<String> {
    sqlx::query("SELECT u.email FROM memberships m JOIN users u ON u.id = m.user_id WHERE m.org_id = $1 AND m.role = 'owner' ORDER BY m.created_at")
        .bind(org)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
        .and_then(|r| r.try_get(0).ok())
}

/// `POST /v1/stripe/webhook`. Unauthenticated by design; trust comes only
/// from the signature.
pub async fn webhook(
    State(s): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<Value>> {
    let c = cfg(&s)?;
    let sig = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !verify_signature(&c.webhook_secret, sig, &body, now_ms() / 1000) {
        tracing::warn!(ip = ?auth::client_ip(&headers), "stripe webhook signature rejected");
        return Err(ApiError::BadRequest("invalid signature".into()));
    }
    let ev: Value =
        serde_json::from_slice(&body).map_err(|_| ApiError::BadRequest("invalid event".into()))?;
    let obj = &ev["data"]["object"];
    match ev["type"].as_str().unwrap_or("") {
        "checkout.session.completed" => {
            let org = obj["client_reference_id"].as_str().unwrap_or("");
            let plan = obj["metadata"]["plan"].as_str().unwrap_or("");
            if org.is_empty() || plan == "free" || plan == "enterprise" || !plans::exists(plan) {
                return Ok(Json(json!({ "received": true, "ignored": true })));
            }
            let res = sqlx::query("UPDATE organizations SET plan = $1, stripe_customer_id = $2, stripe_subscription_id = $3 WHERE id = $4")
                .bind(plan)
                .bind(obj["customer"].as_str().map(str::to_string))
                .bind(obj["subscription"].as_str().map(str::to_string))
                .bind(org)
                .execute(&s.db)
                .await?;
            if res.rows_affected() > 0 {
                audit::log(&s, org, "stripe", "billing.plan_changed", plan, "checkout").await;
                if let Some(to) = owner_email(&s, org).await {
                    crate::email::send(
                        &s,
                        &to,
                        "Your HSIP plan is active",
                        &format!(
                            "Thanks. Your organization is now on the {} plan.",
                            plans::get(plan).name
                        ),
                    )
                    .await;
                }
            }
        }
        "customer.subscription.updated" => {
            let sub = obj["id"].as_str().unwrap_or("");
            let status = obj["status"].as_str().unwrap_or("");
            let price = obj["items"]["data"][0]["price"]["id"]
                .as_str()
                .unwrap_or("");
            let new_plan = match status {
                "active" | "trialing" => plan_for_price(c, price),
                "canceled" | "unpaid" | "incomplete_expired" => Some("free"),
                _ => None, // past_due: keep the plan while Stripe retries
            };
            if let Some(p) = new_plan {
                set_by_subscription(&s, sub, p).await?;
            }
        }
        "customer.subscription.deleted" => {
            set_by_subscription(&s, obj["id"].as_str().unwrap_or(""), "free").await?;
        }
        _ => {}
    }
    Ok(Json(json!({ "received": true })))
}

async fn set_by_subscription(s: &AppState, sub: &str, plan: &str) -> ApiResult<()> {
    if sub.is_empty() {
        return Ok(());
    }
    let row = sqlx::query("SELECT id, plan FROM organizations WHERE stripe_subscription_id = $1")
        .bind(sub)
        .fetch_optional(&s.db)
        .await?;
    let Some(row) = row else { return Ok(()) };
    let org: String = row.try_get(0)?;
    let current: String = row.try_get(1)?;
    if current == plan || current == "enterprise" {
        return Ok(());
    }
    sqlx::query("UPDATE organizations SET plan = $1 WHERE id = $2")
        .bind(plan)
        .bind(&org)
        .execute(&s.db)
        .await?;
    audit::log(
        s,
        &org,
        "stripe",
        "billing.plan_changed",
        plan,
        "subscription",
    )
    .await;
    Ok(())
}
