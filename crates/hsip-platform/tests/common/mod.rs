#![allow(dead_code)]
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use hsip_platform::{app, config::Config, state::AppState};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

pub struct Env {
    pub state: AppState,
    pub app: Router,
}

pub async fn env() -> Env {
    env_with(Config::for_tests()).await
}

pub async fn env_with(cfg: Config) -> Env {
    let state = AppState::new(cfg).await.unwrap();
    Env {
        app: app(state.clone()),
        state,
    }
}

pub async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let (status, bytes) = call_raw(app, method, uri, token, body).await;
    let v = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, v)
}

pub async fn call_raw(
    app: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Vec<u8>) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    let req = match body {
        Some(b) => req
            .header("content-type", "application/json")
            .body(Body::from(b.to_string()))
            .unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    (
        status,
        resp.into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
}

pub struct Tenant {
    pub token: String,
    pub org: String,
}

pub async fn signup(e: &Env, email: &str, org_name: &str) -> Tenant {
    let (st, v) = call(&e.app, "POST", "/v1/auth/signup", None, Some(json!({
        "email": email, "password": "correct-horse-battery", "name": "Test User", "org_name": org_name
    }))).await;
    assert_eq!(st, StatusCode::OK, "{v}");
    Tenant {
        token: v["token"].as_str().unwrap().into(),
        org: v["org_id"].as_str().unwrap().into(),
    }
}

pub async fn stream(e: &Env, t: &Tenant, name: &str) -> String {
    let (st, v) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/streams", t.org),
        Some(&t.token),
        Some(json!({"name": name})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    v["id"].as_str().unwrap().into()
}

pub async fn api_key(
    e: &Env,
    t: &Tenant,
    scope: &str,
    stream_id: Option<&str>,
) -> (String, String) {
    let (st, v) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/api-keys", t.org),
        Some(&t.token),
        Some(json!({
            "name": "k", "scope": scope, "stream_id": stream_id
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    (
        v["token"].as_str().unwrap().into(),
        v["id"].as_str().unwrap().into(),
    )
}

pub fn hash_of(s: &str) -> String {
    hsip_proof::sha256_hex(s.as_bytes())
}

pub async fn ingest(e: &Env, key: &str, stream: &str, n: usize) -> (StatusCode, Value) {
    call(&e.app, "POST", "/v1/ingest", Some(key), Some(json!({
        "stream": stream, "actor": "loan-model-v3", "action": "loan.decision", "subject": format!("app-{n}"),
        "payload_hash": hash_of(&format!("record {n}")), "metadata": {"region": "eu"}
    }))).await
}
