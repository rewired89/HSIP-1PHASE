mod common;
use axum::{body::Body, http::Request};
use common::*;
use hmac::{Hmac, Mac};
use hsip_platform::config::{Config, EmailCfg, StripeCfg};
use serde_json::{json, Value};
use sha2::Sha256;
use sqlx::Row;
use tower::ServiceExt;
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

const SECRET: &str = "whsec_test_secret";

fn cfg(url: &str) -> Config {
    let mut c = Config::for_tests();
    c.stripe = Some(StripeCfg {
        api_url: url.into(),
        secret_key: "sk_test_123".into(),
        webhook_secret: SECRET.into(),
        price_team: Some("price_team".into()),
        price_business: Some("price_biz".into()),
    });
    c.email = Some(EmailCfg {
        api_url: url.into(),
        api_key: "re_key".into(),
        from: "HSIP <noreply@example.com>".into(),
    });
    c
}

fn sign(payload: &str, t: i64, secret: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(format!("{t}.{payload}").as_bytes());
    format!("t={t},v1={}", hex::encode(mac.finalize().into_bytes()))
}

async fn hook(e: &Env, payload: &Value, sig: Option<String>) -> u16 {
    let body = payload.to_string();
    let mut req = Request::builder().method("POST").uri("/v1/stripe/webhook");
    if let Some(s) = sig {
        req = req.header("stripe-signature", s);
    }
    let r = e
        .app
        .clone()
        .oneshot(req.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    r.status().as_u16()
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

async fn plan(e: &Env, org: &str) -> String {
    sqlx::query("SELECT plan FROM organizations WHERE id = $1")
        .bind(org)
        .fetch_one(&e.state.db)
        .await
        .unwrap()
        .try_get(0)
        .unwrap()
}

#[tokio::test]
async fn checkout_then_signed_webhook_upgrades_and_cancel_downgrades() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/checkout/sessions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"url": "https://checkout.stripe.test/c1"})),
        )
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path("/emails"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "e1"})))
        .mount(&mock)
        .await;
    let e = env_with(cfg(&mock.uri())).await;
    let t = signup(&e, "owner@example.com", "Acme").await;

    let (st, v) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/billing/checkout", t.org),
        Some(&t.token),
        Some(json!({"plan": "team"})),
    )
    .await;
    assert_eq!(st, 200, "{v}");
    assert_eq!(v["url"], "https://checkout.stripe.test/c1");
    let (st, _) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/billing/checkout", t.org),
        Some(&t.token),
        Some(json!({"plan": "enterprise"})),
    )
    .await;
    assert_eq!(st, 400);

    let ev = json!({"type": "checkout.session.completed", "data": {"object": {
        "client_reference_id": t.org, "customer": "cus_1", "subscription": "sub_1", "metadata": {"plan": "team"}}}});
    assert_eq!(hook(&e, &ev, None).await, 400);
    assert_eq!(
        hook(&e, &ev, Some(sign(&ev.to_string(), now(), "wrong"))).await,
        400
    );
    assert_eq!(
        hook(&e, &ev, Some(sign(&ev.to_string(), now() - 1000, SECRET))).await,
        400
    );
    assert_eq!(plan(&e, &t.org).await, "free");
    assert_eq!(
        hook(&e, &ev, Some(sign(&ev.to_string(), now(), SECRET))).await,
        200
    );
    assert_eq!(plan(&e, &t.org).await, "team");

    let up = json!({"type": "customer.subscription.updated", "data": {"object": {
        "id": "sub_1", "status": "active", "items": {"data": [{"price": {"id": "price_biz"}}]}}}});
    assert_eq!(
        hook(&e, &up, Some(sign(&up.to_string(), now(), SECRET))).await,
        200
    );
    assert_eq!(plan(&e, &t.org).await, "business");

    let del = json!({"type": "customer.subscription.deleted", "data": {"object": {"id": "sub_1"}}});
    assert_eq!(
        hook(&e, &del, Some(sign(&del.to_string(), now(), SECRET))).await,
        200
    );
    assert_eq!(plan(&e, &t.org).await, "free");

    // the owner got an upgrade email
    let reqs = mock.received_requests().await.unwrap();
    assert!(reqs.iter().any(|r| r.url.path() == "/emails"));
}

#[tokio::test]
async fn billing_is_off_without_config_and_invites_report_email_state() {
    let e = env().await;
    let t = signup(&e, "o@example.com", "Acme").await;
    let (st, _) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/billing/checkout", t.org),
        Some(&t.token),
        Some(json!({"plan": "team"})),
    )
    .await;
    assert_eq!(st, 400);
    assert_eq!(hook(&e, &json!({}), None).await, 400);
    let (_, m) = call(&e.app, "GET", "/v1/meta", None, None).await;
    assert_eq!(m["billing_enabled"], false);
    let (st, v) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/invites", t.org),
        Some(&t.token),
        Some(json!({"email": "new@example.com", "role": "developer"})),
    )
    .await;
    assert_eq!(st, 200, "{v}");
    assert_eq!(v["emailed"], false);
}

#[tokio::test]
async fn invite_sends_email_when_configured() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/emails"))
        .and(header("authorization", "Bearer re_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "e1"})))
        .expect(1)
        .mount(&mock)
        .await;
    let e = env_with(cfg(&mock.uri())).await;
    let t = signup(&e, "o@example.com", "Acme").await;
    let (st, v) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/invites", t.org),
        Some(&t.token),
        Some(json!({"email": "new@example.com", "role": "developer"})),
    )
    .await;
    assert_eq!(st, 200, "{v}");
    assert_eq!(v["emailed"], true);
}
