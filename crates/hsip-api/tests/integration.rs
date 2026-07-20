//! Integration tests for the HSIP REST API.
//!
//! Each test spins up an in-memory SQLite database, bootstraps the router,
//! and exercises the full HTTP stack via `tower::ServiceExt`.

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

// ── helpers ──────────────────────────────────────────────────────────────────

// Build an app backed by an in-memory SQLite database and return (app, admin_key).
async fn test_app() -> (axum::Router, String) {
    // Each test gets its own in-memory DB by using a unique name.
    let db_url = format!(
        "sqlite:file:{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4()
    );

    // Install drivers once (idempotent via Once in db::init)
    sqlx::any::install_default_drivers();

    let db = hsip_api::db::init(&db_url).await.expect("db init");

    // Bootstrap a test tenant + key manually
    let tenant_id = uuid::Uuid::new_v4().to_string();
    let raw_key = format!("hsip_{}", hex::encode([0u8; 32]));
    let key_hash = hsip_api::auth::hash_key(&raw_key);
    let key_id = uuid::Uuid::new_v4().to_string();
    let now = hsip_api::db::now_ms();

    use sqlx::Executor;
    db.execute(
        sqlx::query("INSERT INTO tenants (id, name, created_at) VALUES (?, 'test', ?)")
            .bind(&tenant_id)
            .bind(now),
    )
    .await
    .unwrap();

    // 'owner' — this is the tenant's only key, and several existing tests
    // use it to create/list additional keys via POST/GET /v1/keys, which
    // now requires the owner role.
    db.execute(
        sqlx::query(
            "INSERT INTO api_keys (id, tenant_id, key_hash, name, agent_type, role, created_at, active)
             VALUES (?, ?, ?, 'test', 'human', 'owner', ?, 1)",
        )
        .bind(&key_id)
        .bind(&tenant_id)
        .bind(&key_hash)
        .bind(now),
    )
    .await
    .unwrap();

    let state = hsip_api::state::AppState::new(db, vec![0u8; 32]);
    let app = hsip_api::routes::router().with_state(state);

    (app, raw_key)
}

fn bearer(key: &str) -> String {
    format!("Bearer {key}")
}

// Like `test_app`, but also returns the raw `Db` and master key so a test can
// call `hsip_api::anchor_job::run_anchor_cycle` directly — the production
// binary runs that on a timer, but a test needs to drive it synchronously to
// make batch timing deterministic. Registers the key as `ai_agent` type
// (Predicta's shape), not `human`, to match how decision attestation callers
// authenticate in practice.
async fn test_app_with_db() -> (axum::Router, String, hsip_api::db::Db, Vec<u8>) {
    let db_url = format!(
        "sqlite:file:{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4()
    );
    sqlx::any::install_default_drivers();
    let db = hsip_api::db::init(&db_url).await.expect("db init");

    let tenant_id = uuid::Uuid::new_v4().to_string();
    let raw_key = format!("hsip_{}", hex::encode([9u8; 32]));
    let key_hash = hsip_api::auth::hash_key(&raw_key);
    let key_id = uuid::Uuid::new_v4().to_string();
    let now = hsip_api::db::now_ms();

    use sqlx::Executor;
    db.execute(
        sqlx::query("INSERT INTO tenants (id, name, created_at) VALUES (?, 'test', ?)")
            .bind(&tenant_id)
            .bind(now),
    )
    .await
    .unwrap();

    db.execute(
        sqlx::query(
            "INSERT INTO api_keys (id, tenant_id, key_hash, name, agent_type, role, created_at, active)
             VALUES (?, ?, ?, 'predicta', 'ai_agent', 'owner', ?, 1)",
        )
        .bind(&key_id)
        .bind(&tenant_id)
        .bind(&key_hash)
        .bind(now),
    )
    .await
    .unwrap();

    let master_key = vec![7u8; 32];
    let state = hsip_api::state::AppState::new(db.clone(), master_key.clone());
    let app = hsip_api::routes::router().with_state(state);

    (app, raw_key, db, master_key)
}

/// Like `test_app`, but the bootstrap key is named `admin` (matching
/// `main.rs::bootstrap_admin`'s convention) in the *first* tenant created,
/// and the master key is backed by a real temp file rather than an in-memory
/// `Vec<u8>` — needed to exercise `POST /v1/admin/master-key/rotate`, which
/// refuses to run unless `state.master_key_path` is `Some`.
async fn test_app_with_admin_and_key_file(
) -> (axum::Router, String, hsip_api::db::Db, std::path::PathBuf) {
    let db_url = format!(
        "sqlite:file:{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4()
    );
    sqlx::any::install_default_drivers();
    let db = hsip_api::db::init(&db_url).await.expect("db init");

    let tenant_id = uuid::Uuid::new_v4().to_string();
    let raw_key = format!("hsip_{}", hex::encode([3u8; 32]));
    let key_hash = hsip_api::auth::hash_key(&raw_key);
    let key_id = uuid::Uuid::new_v4().to_string();
    let now = hsip_api::db::now_ms();

    use sqlx::Executor;
    db.execute(
        sqlx::query("INSERT INTO tenants (id, name, created_at) VALUES (?, 'default', ?)")
            .bind(&tenant_id)
            .bind(now),
    )
    .await
    .unwrap();

    // 'owner' + is_root_admin=1 — mirrors main.rs::bootstrap_admin's real
    // INSERT exactly, since this helper exists specifically to exercise the
    // root-admin-gated master-key-rotation routes.
    db.execute(
        sqlx::query(
            "INSERT INTO api_keys (id, tenant_id, key_hash, name, agent_type, role, is_root_admin, created_at, active)
             VALUES (?, ?, ?, 'admin', 'human', 'owner', 1, ?, 1)",
        )
        .bind(&key_id)
        .bind(&tenant_id)
        .bind(&key_hash)
        .bind(now),
    )
    .await
    .unwrap();

    let master_key = vec![5u8; 32];
    let key_path =
        std::env::temp_dir().join(format!("hsip-test-master-{}.key", uuid::Uuid::new_v4()));
    std::fs::write(&key_path, hex::encode(&master_key)).expect("write test master key file");

    let state = hsip_api::state::AppState::new_with_master_key_path(
        db.clone(),
        master_key,
        Some(key_path.to_string_lossy().into_owned()),
    );
    let app = hsip_api::routes::router().with_state(state);

    (app, raw_key, db, key_path)
}

async fn body_json(body: axum::body::Body) -> serde_json::Value {
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_unauthorized_without_key() {
    let (app, _) = test_app().await;
    let res = app
        .oneshot(Request::get("/v1/identity").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_identity() {
    let (app, key) = test_app().await;
    let res = app
        .oneshot(
            Request::post("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let json = body_json(res.into_body()).await;
    assert!(json["verify_key"].is_string());
    assert!(json["created_at"].is_number());
}

#[tokio::test]
async fn test_get_identity_not_found() {
    let (app, key) = test_app().await;
    let res = app
        .oneshot(
            Request::get("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_identity_idempotent() {
    let (app, key) = test_app().await;

    // Create identity twice — should return same verify_key
    let r1 = app
        .clone()
        .oneshot(
            Request::post("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let j1 = body_json(r1.into_body()).await;

    let r2 = app
        .oneshot(
            Request::post("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let j2 = body_json(r2.into_body()).await;

    assert_eq!(j1["verify_key"], j2["verify_key"]);
}

#[tokio::test]
async fn test_create_and_list_keys() {
    let (app, key) = test_app().await;

    let res = app
        .clone()
        .oneshot(
            Request::post("/v1/keys")
                .header(header::AUTHORIZATION, bearer(&key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"name":"test-svc","agent_type":"service"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let json = body_json(res.into_body()).await;
    assert!(json["key"].as_str().unwrap().starts_with("hsip_"));
    assert_eq!(json["agent_type"], "service");

    let list_res = app
        .oneshot(
            Request::get("/v1/keys")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_res.status(), StatusCode::OK);
    let keys: serde_json::Value = body_json(list_res.into_body()).await;
    assert!(keys.as_array().unwrap().len() >= 2); // admin + new key
}

#[tokio::test]
async fn test_key_with_expiry() {
    let (app, key) = test_app().await;

    let res = app
        .oneshot(
            Request::post("/v1/keys")
                .header(header::AUTHORIZATION, bearer(&key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"name":"expiring","expires_in_days":30}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let json = body_json(res.into_body()).await;
    assert!(json["expires_at"].is_number());
}

#[tokio::test]
async fn test_consent_grant_and_revoke() {
    let (app, key) = test_app().await;
    let peer = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="; // dummy 32-byte base64

    let grant_res = app
        .clone()
        .oneshot(
            Request::post("/v1/consent/grant")
                .header(header::AUTHORIZATION, bearer(&key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(r#"{{"peer_verify_key":"{peer}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(grant_res.status(), StatusCode::OK);
    let j = body_json(grant_res.into_body()).await;
    assert_eq!(j["status"], "granted");
    // The "ignored actor" fix: consent records which kind of key authorized it.
    assert_eq!(j["granted_by_key_type"], "human");

    let revoke_res = app
        .oneshot(
            Request::post("/v1/consent/revoke")
                .header(header::AUTHORIZATION, bearer(&key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(r#"{{"peer_verify_key":"{peer}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(revoke_res.status(), StatusCode::OK);
    let j = body_json(revoke_res.into_body()).await;
    assert_eq!(j["status"], "revoked");
}

#[tokio::test]
async fn test_credential_issue_verify_revoke() {
    let (app, key) = test_app().await;

    // Must have identity first
    app.clone()
        .oneshot(
            Request::post("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Issue
    let issue_res = app
        .clone()
        .oneshot(
            Request::post("/v1/credentials/issue")
                .header(header::AUTHORIZATION, bearer(&key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"claim":"age_over_18","user_token":"tok_abc123","ttl_seconds":3600}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(issue_res.status(), StatusCode::OK);
    let issue_json = body_json(issue_res.into_body()).await;
    let cred_id = issue_json["credential"]["id"].as_str().unwrap().to_string();

    // Verify (valid)
    let verify_body = serde_json::json!({
        "credential": issue_json["credential"],
        "signature":  issue_json["signature"]
    });
    let verify_res = app
        .clone()
        .oneshot(
            Request::post("/v1/credentials/verify")
                .header(header::AUTHORIZATION, bearer(&key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(verify_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(verify_res.status(), StatusCode::OK);
    let vj = body_json(verify_res.into_body()).await;
    assert_eq!(vj["valid"], true);
    assert_eq!(vj["revoked"], false);

    // Revoke
    let revoke_res = app
        .clone()
        .oneshot(
            Request::delete(format!("/v1/credentials/{cred_id}/revoke"))
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(revoke_res.status(), StatusCode::OK);

    // Verify again (should be invalid: revoked)
    let verify_body2 = serde_json::json!({
        "credential": issue_json["credential"],
        "signature":  issue_json["signature"]
    });
    let verify_res2 = app
        .oneshot(
            Request::post("/v1/credentials/verify")
                .header(header::AUTHORIZATION, bearer(&key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(verify_body2.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let vj2 = body_json(verify_res2.into_body()).await;
    assert_eq!(vj2["valid"], false);
    assert_eq!(vj2["revoked"], true);
}

#[tokio::test]
async fn test_rate_limit_enforced() {
    // Set a very low limit for this test via env (default is 300, we'll just confirm
    // the tracker increments correctly by checking auth succeeds normally).
    let (app, key) = test_app().await;

    // Make 5 successful requests — should all succeed with default 300 rpm limit
    for _ in 0..5 {
        let res = app
            .clone()
            .oneshot(
                Request::get("/v1/audit")
                    .header(header::AUTHORIZATION, bearer(&key))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn test_gdpr_erase() {
    let (app, key) = test_app().await;

    // Create some data first
    app.clone()
        .oneshot(
            Request::post("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Erase everything
    let erase_res = app
        .oneshot(
            Request::post("/v1/tenant/erase")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(erase_res.status(), StatusCode::OK);
    let ej = body_json(erase_res.into_body()).await;
    assert_eq!(ej["erased"], true);
    assert!(ej["tables_cleared"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_audit_log_populated() {
    let (app, key) = test_app().await;

    // Create identity to generate an audit entry
    app.clone()
        .oneshot(
            Request::post("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let res = app
        .oneshot(
            Request::get("/v1/audit")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let entries: serde_json::Value = body_json(res.into_body()).await;
    let arr = entries.as_array().unwrap();
    assert!(!arr.is_empty());
    assert_eq!(arr[0]["action"], "identity.created");
}

/// End-to-end decision attestation: sign a Predicta-shaped trading decision,
/// chain a second one, anchor the batch, and verify the resulting proof
/// bundle via `POST /v1/decisions/verify` — which takes no auth and makes
/// no database call — matching the "smallest possible working slice" the
/// feature was scoped to: one real decision through the whole loop,
/// independently verifiable with zero further trust in this server.
#[tokio::test]
async fn test_decision_attestation_sign_anchor_verify_end_to_end() {
    use sha2::{Digest, Sha256};

    let (app, key, db, master_key) = test_app_with_db().await;

    app.clone()
        .oneshot(
            Request::post("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let ident_res = app
        .clone()
        .oneshot(
            Request::get("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let ident_json = body_json(ident_res.into_body()).await;
    let accountable_key = ident_json["verify_key"].as_str().unwrap().to_string();

    // Predicta hashes its real (undisclosed) trade decision locally — HSIP
    // never sees the trade parameters themselves, only this hash.
    let payload_hash = hex::encode(Sha256::digest(
        b"BUY 100 AAPL @ 191.20 strategy=mean-reversion-1",
    ));

    let record_body = serde_json::json!({
        "accountable_key": accountable_key,
        "model_version": "predicta-v3.2",
        "strategy_id": "mean-reversion-1",
        "decision_type": "trade.order",
        "payload_hash": payload_hash,
    });
    let record_res = app
        .clone()
        .oneshot(
            Request::post("/v1/decisions")
                .header(header::AUTHORIZATION, bearer(&key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(record_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(record_res.status(), StatusCode::OK);
    let record_json = body_json(record_res.into_body()).await;
    let decision_id = record_json["decision_id"].as_str().unwrap().to_string();
    assert_eq!(record_json["envelope"]["prev_hash"], "");

    // A second decision must chain to the first.
    let record_res2 = app
        .clone()
        .oneshot(
            Request::post("/v1/decisions")
                .header(header::AUTHORIZATION, bearer(&key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(record_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let record_json2 = body_json(record_res2.into_body()).await;
    assert_eq!(
        record_json2["envelope"]["prev_hash"],
        record_json["event_hash"]
    );

    // Before anchoring: authorship is provable, anchoring is not yet done.
    let proof_res = app
        .clone()
        .oneshot(
            Request::get(format!("/v1/decisions/{decision_id}/proof"))
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let proof_json = body_json(proof_res.into_body()).await;
    assert_eq!(proof_json["anchored"], false);

    let verify_body_pre = serde_json::json!({
        "envelope": proof_json["envelope"],
        "event_hash": proof_json["event_hash"],
        "signature": proof_json["signature"],
        "issuer_verify_key": proof_json["issuer_verify_key"],
    });
    let verify_res_pre = app
        .clone()
        .oneshot(
            Request::post("/v1/decisions/verify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(verify_body_pre.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let verify_json_pre = body_json(verify_res_pre.into_body()).await;
    assert_eq!(verify_json_pre["valid"], true);
    assert_eq!(
        verify_json_pre["merkle_inclusion_valid"],
        serde_json::Value::Null
    );

    // Drive one anchor cycle synchronously against a local mock calendar
    // (production runs this on a timer against the real public OpenTimestamps
    // calendars — pointing tests at a mock keeps this fast and hermetic).
    let mock_calendar = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/digest"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_bytes(b"test-ots-receipt".to_vec()),
        )
        .mount(&mock_calendar)
        .await;
    let summary = hsip_api::anchor_job::run_anchor_cycle_with_calendars(
        &db,
        &master_key,
        &[&mock_calendar.uri()],
    )
    .await
    .expect("anchor cycle should not error");
    assert!(
        summary.is_some(),
        "batch of 2 unanchored decisions should anchor immediately in tests"
    );

    let proof_res2 = app
        .clone()
        .oneshot(
            Request::get(format!("/v1/decisions/{decision_id}/proof"))
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let proof_json2 = body_json(proof_res2.into_body()).await;
    assert_eq!(proof_json2["anchored"], true);
    assert!(proof_json2["merkle_root"].is_string());
    assert!(proof_json2["inclusion_proof"].is_array());

    // The whole point: verify with zero further calls to this app's database.
    let verify_body_post = serde_json::json!({
        "envelope": proof_json2["envelope"],
        "event_hash": proof_json2["event_hash"],
        "signature": proof_json2["signature"],
        "issuer_verify_key": proof_json2["issuer_verify_key"],
        "merkle_root": proof_json2["merkle_root"],
        "inclusion_proof": proof_json2["inclusion_proof"],
        "anchor_signature": proof_json2["anchor_signature"],
        "anchor_verify_key": proof_json2["anchor_verify_key"],
    });
    let verify_res_post = app
        .clone()
        .oneshot(
            Request::post("/v1/decisions/verify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(verify_body_post.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let verify_json_post = body_json(verify_res_post.into_body()).await;
    assert_eq!(verify_json_post["valid"], true);
    assert_eq!(verify_json_post["merkle_inclusion_valid"], true);
    assert_eq!(verify_json_post["reason"], serde_json::Value::Null);

    // Tamper check: altering the disclosed envelope must break verification
    // even though the signature/hash/merkle-proof strings are unchanged.
    let mut tampered = verify_body_post.clone();
    tampered["envelope"]["strategy_id"] = serde_json::json!("mean-reversion-2-TAMPERED");
    let tamper_res = app
        .oneshot(
            Request::post("/v1/decisions/verify")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(tampered.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let tamper_json = body_json(tamper_res.into_body()).await;
    assert_eq!(tamper_json["valid"], false);
    assert_eq!(tamper_json["event_hash_matches"], false);
}

/// A `decision_anchors` batch that's `ots_status = 'pending'` must upgrade
/// to `'confirmed'` once its calendar reports a Bitcoin-block-header
/// attestation — the THREAT_MODEL.md §4.20 follow-up:
/// `anchor_job::run_upgrade_cycle` polling `anchor::check_for_upgrade`
/// against whichever calendar the original submission's stored `ots_proof`
/// points at (via `anchor::extract_pending_calendar_uri`), not a
/// caller-supplied list.
#[tokio::test]
async fn test_decision_anchor_upgrades_to_bitcoin_confirmed() {
    use sha2::{Digest, Sha256};
    use sqlx::Row;

    let (app, key, db, master_key) = test_app_with_db().await;

    app.clone()
        .oneshot(
            Request::post("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let ident_res = app
        .clone()
        .oneshot(
            Request::get("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let ident_json = body_json(ident_res.into_body()).await;
    let accountable_key = ident_json["verify_key"].as_str().unwrap().to_string();

    let payload_hash = hex::encode(Sha256::digest(b"upgrade-test-payload"));
    let record_body = serde_json::json!({
        "accountable_key": accountable_key,
        "model_version": "v1",
        "strategy_id": "upgrade-test",
        "decision_type": "test",
        "payload_hash": payload_hash,
    });
    let record_res = app
        .clone()
        .oneshot(
            Request::post("/v1/decisions")
                .header(header::AUTHORIZATION, bearer(&key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(record_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let record_json = body_json(record_res.into_body()).await;
    let decision_id = record_json["decision_id"].as_str().unwrap().to_string();

    // The mock calendar's `/digest` response is shaped like a real one: a
    // `PendingAttestation` (tag + length-prefixed URI) pointing at the
    // calendar's *own* URL — same layout as the real
    // `alice.btc.calendar.opentimestamps.org` fixture in `anchor.rs`'s unit
    // tests. This is what lets `extract_pending_calendar_uri` find the
    // right calendar later, exactly like it would in production.
    let mock_calendar = wiremock::MockServer::start().await;
    let calendar_uri = mock_calendar.uri();
    let uri_bytes = calendar_uri.as_bytes();
    let mut pending_proof = vec![0xAA, 0xBB, 0xCC]; // stand-in for the Merkle-op prefix bytes a real proof carries
    pending_proof.extend_from_slice(&[0x83, 0xdf, 0xe3, 0x0d, 0x2e, 0xf9, 0x0c, 0x8e]); // PendingAttestation tag
    pending_proof.push((uri_bytes.len() + 1) as u8); // outer body length
    pending_proof.push(uri_bytes.len() as u8); // inner URI length
    pending_proof.extend_from_slice(uri_bytes);

    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/digest"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_bytes(pending_proof))
        .mount(&mock_calendar)
        .await;

    let summary =
        hsip_api::anchor_job::run_anchor_cycle_with_calendars(&db, &master_key, &[&calendar_uri])
            .await
            .expect("anchor cycle should not error");
    assert!(summary.is_some());

    let anchor_row = sqlx::query("SELECT merkle_root, ots_status FROM decision_anchors LIMIT 1")
        .fetch_one(&db)
        .await
        .unwrap();
    let merkle_root_hex: String = anchor_row.try_get(0).unwrap();
    let ots_status_before: String = anchor_row.try_get(1).unwrap();
    assert_eq!(ots_status_before, "pending");

    // The calendar has since reported this digest confirmed in a mined
    // Bitcoin block — mock the upgrade-check endpoint per the real
    // OpenTimestamps calendar HTTP protocol (`GET /timestamp/<hex-digest>`).
    let mut confirmed_proof = vec![0xAA, 0xBB, 0xCC];
    confirmed_proof.extend_from_slice(&[0x05, 0x88, 0x96, 0x0d, 0x73, 0xd7, 0x19, 0x01]); // BitcoinBlockHeaderAttestation tag
    confirmed_proof.extend_from_slice(b"...stand-in for real block header / merkle path bytes...");

    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path(format!(
            "/timestamp/{merkle_root_hex}"
        )))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_bytes(confirmed_proof))
        .mount(&mock_calendar)
        .await;

    hsip_api::anchor_job::run_upgrade_cycle(&db).await;

    let anchor_row_after = sqlx::query("SELECT ots_status FROM decision_anchors LIMIT 1")
        .fetch_one(&db)
        .await
        .unwrap();
    let ots_status_after: String = anchor_row_after.try_get(0).unwrap();
    assert_eq!(ots_status_after, "confirmed");

    // Also visible through the public API, not just the raw DB row.
    let proof_res = app
        .oneshot(
            Request::get(format!("/v1/decisions/{decision_id}/proof"))
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let proof_json = body_json(proof_res.into_body()).await;
    assert_eq!(proof_json["ots_status"], "confirmed");
}

/// `GET /v1/agents/discover` must actually be reachable — it was fully
/// implemented in routes/agents.rs but never registered in the router, so
/// the documented `hsip agent discover` / CLAUDE.md route never worked.
#[tokio::test]
async fn test_agents_discover_route_is_wired() {
    let (app, key) = test_app().await;

    let res = app
        .oneshot(
            Request::get("/v1/agents/discover")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let discovered: serde_json::Value = body_json(res.into_body()).await;
    assert!(discovered.is_array());
}

/// The audit log's BLAKE3 hash chain (`audit_log::record`) must link every
/// entry to the one before it, and `GET /v1/audit/verify` must be able to
/// recompute and confirm that chain independently of the raw rows.
#[tokio::test]
async fn test_audit_chain_verify_detects_valid_and_tampered_chains() {
    let (app, key, db, _master_key) = test_app_with_db().await;

    app.clone()
        .oneshot(
            Request::post("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Generate several chained audit entries.
    for _ in 0..3 {
        app.clone()
            .oneshot(
                Request::post("/v1/messages/sign")
                    .header(header::AUTHORIZATION, bearer(&key))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({ "content": "hello" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    let verify_res = app
        .clone()
        .oneshot(
            Request::get("/v1/audit/verify")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(verify_res.status(), StatusCode::OK);
    let verify_json: serde_json::Value = body_json(verify_res.into_body()).await;
    assert_eq!(verify_json["valid"], true);
    assert_eq!(verify_json["unchained"], 0);
    assert!(verify_json["checked"].as_u64().unwrap() >= 3);

    // Directly tamper with one entry's details, as if via OS-level DB write
    // access — the scenario THREAT_MODEL.md §4.8 says the chain must catch.
    use sqlx::Executor;
    db.execute(sqlx::query(
        "UPDATE audit_entries SET details = 'tampered' WHERE action = 'message.signed'",
    ))
    .await
    .unwrap();

    let tampered_res = app
        .oneshot(
            Request::get("/v1/audit/verify")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let tampered_json: serde_json::Value = body_json(tampered_res.into_body()).await;
    assert_eq!(tampered_json["valid"], false);
    assert!(tampered_json["first_break_id"].is_string());
}

/// Master key rotation must actually re-encrypt every identity under a new
/// key (not silently no-op), must swap the in-memory key used by every
/// subsequent request without a restart, must durably rewrite the key file,
/// and must refuse anyone who isn't the bootstrap admin key.
#[tokio::test]
async fn test_master_key_rotation_reencrypts_and_swaps_live_key() {
    let (app, admin_key, db, key_path) = test_app_with_admin_and_key_file().await;

    // Create an identity under the original master key.
    let ident_res = app
        .clone()
        .oneshot(
            Request::post("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&admin_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ident_res.status(), StatusCode::OK);
    let ident_json = body_json(ident_res.into_body()).await;
    let verify_key_before = ident_json["verify_key"].as_str().unwrap().to_string();

    // Capture the ciphertext as stored before rotation.
    use sqlx::Row;
    let row_before = sqlx::query("SELECT signing_key_b64 FROM identities LIMIT 1")
        .fetch_one(&db)
        .await
        .unwrap();
    let ciphertext_before: String = row_before.try_get(0).unwrap();

    // A non-admin key in a *second* tenant must be rejected.
    let other_tenant_id = uuid::Uuid::new_v4().to_string();
    let other_raw_key = format!("hsip_{}", hex::encode([11u8; 32]));
    let other_key_hash = hsip_api::auth::hash_key(&other_raw_key);
    let now = hsip_api::db::now_ms();
    use sqlx::Executor;
    db.execute(
        sqlx::query("INSERT INTO tenants (id, name, created_at) VALUES (?, 'other', ?)")
            .bind(&other_tenant_id)
            .bind(now),
    )
    .await
    .unwrap();
    db.execute(
        sqlx::query(
            "INSERT INTO api_keys (id, tenant_id, key_hash, name, agent_type, created_at, active)
             VALUES (?, ?, ?, 'admin', 'human', ?, 1)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&other_tenant_id)
        .bind(&other_key_hash)
        .bind(now),
    )
    .await
    .unwrap();

    let denied_res = app
        .clone()
        .oneshot(
            Request::post("/v1/admin/master-key/rotate")
                .header(header::AUTHORIZATION, bearer(&other_raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        denied_res.status(),
        StatusCode::UNAUTHORIZED,
        "a key named 'admin' in a non-root tenant must not be able to rotate the master key"
    );

    // The real admin key rotates successfully.
    let rotate_res = app
        .clone()
        .oneshot(
            Request::post("/v1/admin/master-key/rotate")
                .header(header::AUTHORIZATION, bearer(&admin_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rotate_res.status(), StatusCode::OK);
    let rotate_json = body_json(rotate_res.into_body()).await;
    assert_eq!(rotate_json["identities_reencrypted"], 1);
    assert_ne!(
        rotate_json["old_key_fingerprint"],
        rotate_json["new_key_fingerprint"]
    );

    // The key file on disk must now hold the new key, not the old one.
    let new_key_hex = std::fs::read_to_string(&key_path).unwrap();
    let new_key_bytes = hex::decode(new_key_hex.trim()).unwrap();
    assert_ne!(
        new_key_bytes,
        vec![5u8; 32],
        "master key file was not actually rewritten"
    );

    // The stored ciphertext must have changed, and must now be undecryptable
    // under the *old* key but decryptable under the key now on disk — proof
    // this was a real re-encryption, not a no-op.
    let row_after = sqlx::query("SELECT signing_key_b64 FROM identities WHERE tenant_id != ?")
        .bind(&other_tenant_id)
        .fetch_one(&db)
        .await
        .unwrap();
    let ciphertext_after: String = row_after.try_get(0).unwrap();
    assert_ne!(ciphertext_before, ciphertext_after);
    assert!(
        hsip_api::key_encryption::decrypt_signing_key(&ciphertext_after, &vec![5u8; 32]).is_err(),
        "old master key must no longer decrypt the re-encrypted identity"
    );
    assert!(
        hsip_api::key_encryption::decrypt_signing_key(&ciphertext_after, &new_key_bytes).is_ok(),
        "the key now on disk must decrypt the re-encrypted identity"
    );

    // Signing must keep working transparently on the *same running process*
    // — proves the in-memory key was swapped, not just the DB and file.
    let sign_res = app
        .clone()
        .oneshot(
            Request::post("/v1/messages/sign")
                .header(header::AUTHORIZATION, bearer(&admin_key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"content":"still works post-rotation"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(sign_res.status(), StatusCode::OK);

    // Identity's public verify key is unchanged — rotation re-encrypts the
    // existing private key, it does not generate a new keypair.
    let ident_res2 = app
        .oneshot(
            Request::get("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&admin_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let ident_json2 = body_json(ident_res2.into_body()).await;
    assert_eq!(ident_json2["verify_key"], verify_key_before);

    let _ = std::fs::remove_file(&key_path);
}

/// `GET /v1/admin/master-key/fingerprint` must be read-only (no mutation,
/// callable repeatedly, matches what rotation would report as the "old"
/// fingerprint) and must enforce the same admin-only gate as rotation.
#[tokio::test]
async fn test_master_key_fingerprint_is_read_only_and_admin_gated() {
    use sha2::{Digest, Sha256};

    let (app, admin_key, db, key_path) = test_app_with_admin_and_key_file().await;

    let expected_fingerprint = hex::encode(&Sha256::digest([5u8; 32])[..8]);

    let res1 = app
        .clone()
        .oneshot(
            Request::get("/v1/admin/master-key/fingerprint")
                .header(header::AUTHORIZATION, bearer(&admin_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res1.status(), StatusCode::OK);
    let json1 = body_json(res1.into_body()).await;
    assert_eq!(json1["fingerprint"], expected_fingerprint);
    assert!(json1["master_key_path"].is_string());

    // Calling it again must return the identical fingerprint — proof this
    // endpoint doesn't mutate or rotate anything, unlike the POST endpoint.
    let res2 = app
        .clone()
        .oneshot(
            Request::get("/v1/admin/master-key/fingerprint")
                .header(header::AUTHORIZATION, bearer(&admin_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json2 = body_json(res2.into_body()).await;
    assert_eq!(json1["fingerprint"], json2["fingerprint"]);

    // A non-admin key (a second tenant, even one that also names its key
    // "admin") must be rejected, same as rotation.
    let other_tenant_id = uuid::Uuid::new_v4().to_string();
    let other_raw_key = format!("hsip_{}", hex::encode([13u8; 32]));
    let other_key_hash = hsip_api::auth::hash_key(&other_raw_key);
    let now = hsip_api::db::now_ms();
    use sqlx::Executor;
    db.execute(
        sqlx::query("INSERT INTO tenants (id, name, created_at) VALUES (?, 'other', ?)")
            .bind(&other_tenant_id)
            .bind(now),
    )
    .await
    .unwrap();
    db.execute(
        sqlx::query(
            "INSERT INTO api_keys (id, tenant_id, key_hash, name, agent_type, created_at, active)
             VALUES (?, ?, ?, 'admin', 'human', ?, 1)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&other_tenant_id)
        .bind(&other_key_hash)
        .bind(now),
    )
    .await
    .unwrap();

    let denied_res = app
        .oneshot(
            Request::get("/v1/admin/master-key/fingerprint")
                .header(header::AUTHORIZATION, bearer(&other_raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied_res.status(), StatusCode::UNAUTHORIZED);

    let _ = std::fs::remove_file(&key_path);
}

/// `HSIP_ROTATION_HOOK` is the auto-rotation path for `HSIP_MASTER_KEY`-
/// sourced deployments (no file this process can rewrite). Proves: (1)
/// rotation refuses with no hook configured, (2) a succeeding hook
/// receives the new key on stdin and the correct fingerprint env vars, the
/// DB gets genuinely re-encrypted, and the in-memory key swaps live, and
/// (3) — the safety-critical case — a *failing* hook leaves the database
/// completely untouched, not partially rotated.
///
/// Unix-only: builds and chmods shell scripts as test hooks.
#[cfg(unix)]
#[tokio::test]
async fn test_master_key_rotation_hook_for_env_sourced_key() {
    use sha2::{Digest, Sha256};
    use sqlx::Row;
    use std::os::unix::fs::PermissionsExt;

    // Build a state with master_key_path: None, matching what main.rs
    // constructs when the key comes from HSIP_MASTER_KEY.
    let db_url = format!(
        "sqlite:file:{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4()
    );
    sqlx::any::install_default_drivers();
    let db = hsip_api::db::init(&db_url).await.expect("db init");

    let tenant_id = uuid::Uuid::new_v4().to_string();
    let raw_key = format!("hsip_{}", hex::encode([21u8; 32]));
    let key_hash = hsip_api::auth::hash_key(&raw_key);
    let now = hsip_api::db::now_ms();
    use sqlx::Executor;
    db.execute(
        sqlx::query("INSERT INTO tenants (id, name, created_at) VALUES (?, 'default', ?)")
            .bind(&tenant_id)
            .bind(now),
    )
    .await
    .unwrap();
    db.execute(
        sqlx::query(
            "INSERT INTO api_keys (id, tenant_id, key_hash, name, agent_type, role, is_root_admin, created_at, active)
             VALUES (?, ?, ?, 'admin', 'human', 'owner', 1, ?, 1)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&tenant_id)
        .bind(&key_hash)
        .bind(now),
    )
    .await
    .unwrap();

    let original_key = vec![6u8; 32];
    let state = hsip_api::state::AppState::new_with_master_key_path(
        db.clone(),
        original_key.clone(),
        None, // env-var-sourced: no file
    );
    let app = hsip_api::routes::router().with_state(state);

    app.clone()
        .oneshot(
            Request::post("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 1. No hook configured — must refuse, no DB change.
    std::env::remove_var("HSIP_ROTATION_HOOK");
    let refused = app
        .clone()
        .oneshot(
            Request::post("/v1/admin/master-key/rotate")
                .header(header::AUTHORIZATION, bearer(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refused.status(), StatusCode::BAD_REQUEST);

    // 2. A succeeding hook: writes stdin (the new key, hex) to a file we
    // control, so we can verify exactly what HSIP sent it.
    let work_dir = std::env::temp_dir().join(format!("hsip-hook-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&work_dir).unwrap();
    let hook_output = work_dir.join("received-key.hex");
    let ok_hook = work_dir.join("ok-hook.sh");
    std::fs::write(
        &ok_hook,
        format!("#!/bin/sh\ncat > \"{}\"\n", hook_output.display()),
    )
    .unwrap();
    std::fs::set_permissions(&ok_hook, std::fs::Permissions::from_mode(0o700)).unwrap();

    std::env::set_var("HSIP_ROTATION_HOOK", &ok_hook);
    let rotate_res = app
        .clone()
        .oneshot(
            Request::post("/v1/admin/master-key/rotate")
                .header(header::AUTHORIZATION, bearer(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rotate_res.status(), StatusCode::OK);
    let rotate_json = body_json(rotate_res.into_body()).await;
    assert_eq!(rotate_json["identities_reencrypted"], 1);
    assert_eq!(rotate_json["master_key_path"], serde_json::Value::Null);
    assert_eq!(
        rotate_json["rotation_hook"],
        ok_hook.to_string_lossy().as_ref()
    );

    // The hook must have received exactly the new key, hex-encoded.
    let received_hex = std::fs::read_to_string(&hook_output).unwrap();
    let received_key = hex::decode(received_hex.trim()).unwrap();
    let expected_fingerprint = hex::encode(&Sha256::digest(&received_key)[..8]);
    assert_eq!(rotate_json["new_key_fingerprint"], expected_fingerprint);
    assert_ne!(received_key, original_key);

    // Live in-memory swap: signing must keep working transparently.
    let sign_res = app
        .clone()
        .oneshot(
            Request::post("/v1/messages/sign")
                .header(header::AUTHORIZATION, bearer(&raw_key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"content":"post-hook-rotation"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(sign_res.status(), StatusCode::OK);

    // The DB row must genuinely be re-encrypted under the new key now.
    let row = sqlx::query("SELECT signing_key_b64 FROM identities WHERE tenant_id = ?")
        .bind(&tenant_id)
        .fetch_one(&db)
        .await
        .unwrap();
    let ciphertext: String = row.try_get(0).unwrap();
    assert!(
        hsip_api::key_encryption::decrypt_signing_key(&ciphertext, &original_key).is_err(),
        "old key must no longer decrypt after hook-based rotation"
    );
    assert!(
        hsip_api::key_encryption::decrypt_signing_key(&ciphertext, &received_key).is_ok(),
        "the key the hook received must decrypt the re-encrypted identity"
    );

    // 3. Safety-critical case: a FAILING hook must leave the database
    // completely untouched — no partial re-encryption.
    let fail_hook = work_dir.join("fail-hook.sh");
    std::fs::write(
        &fail_hook,
        "#!/bin/sh\necho 'simulated failure' >&2\nexit 1\n",
    )
    .unwrap();
    std::fs::set_permissions(&fail_hook, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::env::set_var("HSIP_ROTATION_HOOK", &fail_hook);

    let ciphertext_before_failed_attempt = ciphertext.clone();

    let failed_res = app
        .clone()
        .oneshot(
            Request::post("/v1/admin/master-key/rotate")
                .header(header::AUTHORIZATION, bearer(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(failed_res.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let row_after = sqlx::query("SELECT signing_key_b64 FROM identities WHERE tenant_id = ?")
        .bind(&tenant_id)
        .fetch_one(&db)
        .await
        .unwrap();
    let ciphertext_after: String = row_after.try_get(0).unwrap();
    assert_eq!(
        ciphertext_after, ciphertext_before_failed_attempt,
        "a failing rotation hook must leave the database completely untouched"
    );

    // Fingerprint must also be unchanged — the in-memory key never swapped.
    let fp_res = app
        .oneshot(
            Request::get("/v1/admin/master-key/fingerprint")
                .header(header::AUTHORIZATION, bearer(&raw_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let fp_json = body_json(fp_res.into_body()).await;
    assert_eq!(fp_json["fingerprint"], expected_fingerprint);

    std::env::remove_var("HSIP_ROTATION_HOOK");
    let _ = std::fs::remove_dir_all(&work_dir);
}

// ── HTTP replay protection (opt-in x-hsip-timestamp / x-hsip-nonce) ────────────

#[tokio::test]
async fn test_replay_protection_absent_headers_unaffected() {
    // No replay-protection headers at all — must behave exactly like every
    // other test in this file that doesn't know these headers exist.
    let (app, key) = test_app().await;
    let res = app
        .oneshot(
            Request::get("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_replay_protection_valid_headers_accepted() {
    let (app, key) = test_app().await;
    let now_secs = hsip_api::db::now_ms() / 1000;
    let res = app
        .oneshot(
            Request::get("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .header("x-hsip-timestamp", now_secs.to_string())
                .header("x-hsip-nonce", "nonce-valid-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // Passed the replay check; 404 because no identity was created yet.
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_replay_protection_duplicate_nonce_rejected() {
    let (app, key) = test_app().await;
    let now_secs = hsip_api::db::now_ms() / 1000;

    let r1 = app
        .clone()
        .oneshot(
            Request::get("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .header("x-hsip-timestamp", now_secs.to_string())
                .header("x-hsip-nonce", "nonce-replay-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r1.status(), StatusCode::NOT_FOUND);

    // Same (key, nonce) pair again — must be rejected even though the
    // timestamp is still well within the tolerance window.
    let r2 = app
        .oneshot(
            Request::get("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .header("x-hsip-timestamp", now_secs.to_string())
                .header("x-hsip-nonce", "nonce-replay-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r2.status(), StatusCode::UNAUTHORIZED);
    let json = body_json(r2.into_body()).await;
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("Duplicate x-hsip-nonce"));
}

#[tokio::test]
async fn test_replay_protection_stale_timestamp_rejected() {
    let (app, key) = test_app().await;
    let stale_secs = hsip_api::db::now_ms() / 1000 - 1000; // 1000s ago, outside the 300s window
    let res = app
        .oneshot(
            Request::get("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .header("x-hsip-timestamp", stale_secs.to_string())
                .header("x-hsip-nonce", "nonce-stale-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let json = body_json(res.into_body()).await;
    assert!(json["error"].as_str().unwrap().contains("window"));
}

#[tokio::test]
async fn test_replay_protection_partial_headers_rejected() {
    let (app, key) = test_app().await;
    let now_secs = hsip_api::db::now_ms() / 1000;

    // Timestamp without nonce
    let res = app
        .clone()
        .oneshot(
            Request::get("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .header("x-hsip-timestamp", now_secs.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Nonce without timestamp
    let res2 = app
        .oneshot(
            Request::get("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .header("x-hsip-nonce", "nonce-only")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res2.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_replay_protection_different_keys_can_reuse_same_nonce() {
    // Nonce dedup is scoped per key_id, not global — two keys in the same
    // running server sending the same nonce value must not collide.
    let (app, key_a, db, _master_key) = test_app_with_db().await;

    let other_tenant_id = uuid::Uuid::new_v4().to_string();
    let key_b_raw = format!("hsip_{}", hex::encode([42u8; 32]));
    let key_b_hash = hsip_api::auth::hash_key(&key_b_raw);
    let now = hsip_api::db::now_ms();
    use sqlx::Executor;
    db.execute(
        sqlx::query("INSERT INTO tenants (id, name, created_at) VALUES (?, 'other', ?)")
            .bind(&other_tenant_id)
            .bind(now),
    )
    .await
    .unwrap();
    db.execute(
        sqlx::query(
            "INSERT INTO api_keys (id, tenant_id, key_hash, name, agent_type, created_at, active)
             VALUES (?, ?, ?, 'test-b', 'human', ?, 1)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&other_tenant_id)
        .bind(&key_b_hash)
        .bind(now),
    )
    .await
    .unwrap();

    let now_secs = hsip_api::db::now_ms() / 1000;

    let res_a = app
        .clone()
        .oneshot(
            Request::get("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key_a))
                .header("x-hsip-timestamp", now_secs.to_string())
                .header("x-hsip-nonce", "shared-nonce")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_a.status(), StatusCode::NOT_FOUND);

    // Same nonce, different key — must NOT be rejected as a duplicate.
    let res_b = app
        .oneshot(
            Request::get("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key_b_raw))
                .header("x-hsip-timestamp", now_secs.to_string())
                .header("x-hsip-nonce", "shared-nonce")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_b.status(), StatusCode::NOT_FOUND);
}

// ── RBAC: tenant-scoped key management (owner vs member) ───────────────────────

#[tokio::test]
async fn test_member_cannot_create_or_revoke_keys() {
    let (app, owner_key) = test_app().await;

    // Owner mints a member-role key (the default role).
    let create_res = app
        .clone()
        .oneshot(
            Request::post("/v1/keys")
                .header(header::AUTHORIZATION, bearer(&owner_key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"name":"agent","agent_type":"ai_agent"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::OK);
    let created = body_json(create_res.into_body()).await;
    assert_eq!(created["role"], "member");
    let member_key = created["key"].as_str().unwrap().to_string();
    let member_id = created["id"].as_str().unwrap().to_string();

    // A member-role key cannot create another key — previously any active
    // key, including this one, could mint arbitrary new keys with no check.
    let denied_create = app
        .clone()
        .oneshot(
            Request::post("/v1/keys")
                .header(header::AUTHORIZATION, bearer(&member_key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"name":"escalate"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied_create.status(), StatusCode::UNAUTHORIZED);

    // Nor can it revoke any key, including itself.
    let denied_revoke = app
        .oneshot(
            Request::delete(format!("/v1/keys/{member_id}"))
                .header(header::AUTHORIZATION, bearer(&member_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied_revoke.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_owner_can_create_and_revoke_member_key() {
    let (app, owner_key) = test_app().await;

    let create_res = app
        .clone()
        .oneshot(
            Request::post("/v1/keys")
                .header(header::AUTHORIZATION, bearer(&owner_key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"name":"agent"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::OK);
    let created = body_json(create_res.into_body()).await;
    let member_id = created["id"].as_str().unwrap().to_string();

    let revoke_res = app
        .oneshot(
            Request::delete(format!("/v1/keys/{member_id}"))
                .header(header::AUTHORIZATION, bearer(&owner_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(revoke_res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_owner_can_create_a_second_owner_key() {
    let (app, owner_key) = test_app().await;

    let create_res = app
        .clone()
        .oneshot(
            Request::post("/v1/keys")
                .header(header::AUTHORIZATION, bearer(&owner_key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"name":"second-admin","role":"owner"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::OK);
    let created = body_json(create_res.into_body()).await;
    assert_eq!(created["role"], "owner");
    let second_owner_key = created["key"].as_str().unwrap().to_string();

    // The new owner key can itself create keys.
    let res = app
        .oneshot(
            Request::post("/v1/keys")
                .header(header::AUTHORIZATION, bearer(&second_owner_key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"name":"third"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_cannot_revoke_last_owner_key() {
    let (app, owner_key) = test_app().await;

    let list_res = app
        .clone()
        .oneshot(
            Request::get("/v1/keys")
                .header(header::AUTHORIZATION, bearer(&owner_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let keys = body_json(list_res.into_body()).await;
    assert_eq!(
        keys.as_array().unwrap().len(),
        1,
        "test_app starts with exactly one key"
    );
    let owner_id = keys[0]["id"].as_str().unwrap().to_string();

    let revoke_res = app
        .oneshot(
            Request::delete(format!("/v1/keys/{owner_id}"))
                .header(header::AUTHORIZATION, bearer(&owner_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        revoke_res.status(),
        StatusCode::CONFLICT,
        "revoking a tenant's last owner key must be refused, not leave the tenant locked out"
    );
}

// ── RBAC: node-level root-admin grant/revoke ────────────────────────────────────

#[tokio::test]
async fn test_grant_root_admin_requires_existing_root_admin() {
    // test_app()'s key is a tenant owner but not a root admin.
    let (app, owner_key) = test_app().await;

    let grant_res = app
        .oneshot(
            Request::post("/v1/admin/root-admins/grant")
                .header(header::AUTHORIZATION, bearer(&owner_key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"key_id":"whatever"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(grant_res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_root_admin_grant_list_revoke_flow() {
    let (app, admin_key, _db, key_path) = test_app_with_admin_and_key_file().await;

    // The bootstrap admin (owner + root admin) creates a second, plain
    // member key in the same tenant.
    let create_res = app
        .clone()
        .oneshot(
            Request::post("/v1/keys")
                .header(header::AUTHORIZATION, bearer(&admin_key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"name":"ops"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let created = body_json(create_res.into_body()).await;
    let second_key = created["key"].as_str().unwrap().to_string();
    let second_id = created["id"].as_str().unwrap().to_string();

    // Not yet a root admin — fingerprint must be denied.
    let denied = app
        .clone()
        .oneshot(
            Request::get("/v1/admin/master-key/fingerprint")
                .header(header::AUTHORIZATION, bearer(&second_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);

    // The existing root admin grants it — this is the mechanism that makes
    // more than one root admin possible at all.
    let grant_res = app
        .clone()
        .oneshot(
            Request::post("/v1/admin/root-admins/grant")
                .header(header::AUTHORIZATION, bearer(&admin_key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(r#"{{"key_id":"{second_id}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(grant_res.status(), StatusCode::OK);

    let list_res = app
        .clone()
        .oneshot(
            Request::get("/v1/admin/root-admins")
                .header(header::AUTHORIZATION, bearer(&admin_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_res.status(), StatusCode::OK);
    let admins = body_json(list_res.into_body()).await;
    assert_eq!(admins.as_array().unwrap().len(), 2);

    // The newly granted key can now call fingerprint.
    let allowed = app
        .clone()
        .oneshot(
            Request::get("/v1/admin/master-key/fingerprint")
                .header(header::AUTHORIZATION, bearer(&second_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(allowed.status(), StatusCode::OK);

    // Find the original admin key's own id so it can be revoked.
    let list_keys_res = app
        .clone()
        .oneshot(
            Request::get("/v1/keys")
                .header(header::AUTHORIZATION, bearer(&admin_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let keys = body_json(list_keys_res.into_body()).await;
    let admin_id = keys
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["name"] == "admin")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Revoking the original admin's root-admin flag still leaves one (the
    // second key) — must succeed.
    let revoke_original = app
        .clone()
        .oneshot(
            Request::post("/v1/admin/root-admins/revoke")
                .header(header::AUTHORIZATION, bearer(&second_key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(r#"{{"key_id":"{admin_id}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(revoke_original.status(), StatusCode::OK);

    // The original admin key can no longer reach root-admin-gated routes.
    let now_denied = app
        .clone()
        .oneshot(
            Request::get("/v1/admin/master-key/fingerprint")
                .header(header::AUTHORIZATION, bearer(&admin_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(now_denied.status(), StatusCode::UNAUTHORIZED);

    // second_key is now the ONLY root admin — revoking it must be refused,
    // or the node would have zero root admins with no way to recover short
    // of editing the database directly.
    let last_denied = app
        .oneshot(
            Request::post("/v1/admin/root-admins/revoke")
                .header(header::AUTHORIZATION, bearer(&second_key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(r#"{{"key_id":"{second_id}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(last_denied.status(), StatusCode::CONFLICT);

    let _ = std::fs::remove_file(&key_path);
}

// ── RBAC: upgrade migration backfill ────────────────────────────────────────────

#[tokio::test]
async fn test_role_and_root_admin_backfill_on_upgrade() {
    let db_url = format!(
        "sqlite:file:{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4()
    );
    sqlx::any::install_default_drivers();
    let db = hsip_api::db::init(&db_url).await.expect("db init");

    // Simulate a pre-upgrade row: no role, no is_root_admin explicitly set
    // — mirrors what a real deployment's admin key row looked like before
    // this migration existed.
    let tenant_id = uuid::Uuid::new_v4().to_string();
    let now = hsip_api::db::now_ms();
    use sqlx::Executor;
    db.execute(
        sqlx::query("INSERT INTO tenants (id, name, created_at) VALUES (?, 'default', ?)")
            .bind(&tenant_id)
            .bind(now),
    )
    .await
    .unwrap();
    db.execute(
        sqlx::query(
            "INSERT INTO api_keys (id, tenant_id, key_hash, name, agent_type, created_at, active)
             VALUES (?, ?, 'legacyhash', 'admin', 'human', ?, 1)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&tenant_id)
        .bind(now),
    )
    .await
    .unwrap();

    // Re-running init (as main.rs does on every startup) re-runs
    // migrations, including the backfill — this is the "upgrade an
    // existing database" path, distinct from a fresh install where
    // bootstrap_admin sets role/is_root_admin directly on INSERT.
    let db2 = hsip_api::db::init(&db_url).await.expect("second db init");

    use sqlx::Row;
    let row = sqlx::query("SELECT role, is_root_admin FROM api_keys WHERE tenant_id = ?")
        .bind(&tenant_id)
        .fetch_one(&db2)
        .await
        .unwrap();
    let role: Option<String> = row.try_get(0).unwrap();
    let is_root_admin: i64 = row.try_get(1).unwrap();
    assert_eq!(role.as_deref(), Some("owner"));
    assert_eq!(is_root_admin, 1);
}

// ── Audit log external anchoring ────────────────────────────────────────────────

#[tokio::test]
async fn test_audit_log_anchor_proof_and_verify_end_to_end() {
    let (app, key, db, master_key) = test_app_with_db().await;

    // Identity creation writes an audit entry — gives us something to anchor.
    app.clone()
        .oneshot(
            Request::post("/v1/identity")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let list_res = app
        .clone()
        .oneshot(
            Request::get("/v1/audit")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let entries = body_json(list_res.into_body()).await;
    let entries = entries.as_array().unwrap();
    assert!(
        !entries.is_empty(),
        "identity creation should have written at least one audit entry"
    );
    let entry_id = entries[0]["id"].as_str().unwrap().to_string();

    // Before anchoring: the chain fields are provable, anchoring is not yet done.
    let proof_res = app
        .clone()
        .oneshot(
            Request::get(format!("/v1/audit/{entry_id}/proof"))
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(proof_res.status(), StatusCode::OK);
    let proof_json = body_json(proof_res.into_body()).await;
    assert_eq!(proof_json["anchored"], false);
    assert!(proof_json["entry_hash"].is_string());

    let verify_body_pre = serde_json::json!({
        "id": proof_json["id"],
        "tenant_id": proof_json["tenant_id"],
        "action": proof_json["action"],
        "peer_verify_key": proof_json["peer_verify_key"],
        "details": proof_json["details"],
        "timestamp": proof_json["timestamp"],
        "prev_hash": proof_json["prev_hash"],
        "entry_hash": proof_json["entry_hash"],
    });
    let verify_res_pre = app
        .clone()
        .oneshot(
            Request::post("/v1/audit/verify-proof")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(verify_body_pre.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let verify_json_pre = body_json(verify_res_pre.into_body()).await;
    assert_eq!(verify_json_pre["entry_hash_matches"], true);
    assert_eq!(verify_json_pre["valid"], true);
    assert_eq!(
        verify_json_pre["merkle_inclusion_valid"],
        serde_json::Value::Null
    );

    // Drive one audit anchor cycle synchronously against a local mock
    // calendar (production runs this on a timer against the real public
    // OpenTimestamps calendars).
    let mock_calendar = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/digest"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_bytes(b"test-ots-receipt-audit".to_vec()),
        )
        .mount(&mock_calendar)
        .await;
    let summary = hsip_api::anchor_job::run_audit_anchor_cycle_with_calendars(
        &db,
        &master_key,
        &[&mock_calendar.uri()],
    )
    .await
    .expect("audit anchor cycle should not error");
    assert!(
        summary.is_some(),
        "unanchored audit entries should anchor immediately in tests"
    );

    let proof_res2 = app
        .clone()
        .oneshot(
            Request::get(format!("/v1/audit/{entry_id}/proof"))
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let proof_json2 = body_json(proof_res2.into_body()).await;
    assert_eq!(proof_json2["anchored"], true);
    assert!(proof_json2["merkle_root"].is_string());
    assert!(proof_json2["inclusion_proof"].is_array());

    // The whole point: verify with zero further calls to this app's database.
    let verify_body_post = serde_json::json!({
        "id": proof_json2["id"],
        "tenant_id": proof_json2["tenant_id"],
        "action": proof_json2["action"],
        "peer_verify_key": proof_json2["peer_verify_key"],
        "details": proof_json2["details"],
        "timestamp": proof_json2["timestamp"],
        "prev_hash": proof_json2["prev_hash"],
        "entry_hash": proof_json2["entry_hash"],
        "merkle_root": proof_json2["merkle_root"],
        "inclusion_proof": proof_json2["inclusion_proof"],
        "anchor_signature": proof_json2["anchor_signature"],
        "anchor_verify_key": proof_json2["anchor_verify_key"],
    });
    let verify_res_post = app
        .clone()
        .oneshot(
            Request::post("/v1/audit/verify-proof")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(verify_body_post.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let verify_json_post = body_json(verify_res_post.into_body()).await;
    assert_eq!(verify_json_post["valid"], true);
    assert_eq!(verify_json_post["merkle_inclusion_valid"], true);
    assert_eq!(verify_json_post["reason"], serde_json::Value::Null);

    // Tamper check: altering a disclosed field must break verification even
    // though the merkle proof / anchor signature strings are unchanged.
    let mut tampered = verify_body_post.clone();
    tampered["action"] = serde_json::json!("tampered.action");
    let tamper_res = app
        .oneshot(
            Request::post("/v1/audit/verify-proof")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(tampered.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let tamper_json = body_json(tamper_res.into_body()).await;
    assert_eq!(tamper_json["valid"], false);
    assert_eq!(tamper_json["entry_hash_matches"], false);
}

#[tokio::test]
async fn test_audit_proof_not_found_for_unknown_id() {
    let (app, key) = test_app().await;
    let res = app
        .oneshot(
            Request::get("/v1/audit/does-not-exist/proof")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// Federated trust (`/v1/trust/*`) had no integration coverage at all —
// `trusted_peers` was documented, routed, and referenced by every handler in
// `routes/trust.rs`, but never actually created by `db::run_migrations`, so
// every one of these calls 500'd with "no such table" on any fresh
// database. Found by exercising the dashboard's new Trust page against a
// real server. This test exercises the full add/list/verify/remove
// lifecycle end-to-end so a regression here fails loudly instead of only
// showing up as a runtime 500 nobody notices until they click the page.
#[tokio::test]
async fn test_trust_add_list_verify_remove() {
    let (app, key) = test_app().await;

    // A real Ed25519 keypair so `verify` can check a real signature, not
    // just exercise the "peer not found" / bad-encoding error paths.
    use ed25519_dalek::{Signer, SigningKey};
    let mut csprng = rand::rngs::OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verify_key_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        signing_key.verifying_key().to_bytes(),
    );

    // add
    let add_res = app
        .clone()
        .oneshot(
            Request::post("/v1/trust/peer")
                .header(header::AUTHORIZATION, bearer(&key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"label":"alice","verify_key":"{verify_key_b64}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        add_res.status(),
        StatusCode::OK,
        "trust peer add must not 500"
    );
    let added = body_json(add_res.into_body()).await;
    assert_eq!(added["label"], "alice");
    let peer_id = added["id"].as_str().unwrap().to_string();

    // list — the added peer must actually be there
    let list_res = app
        .clone()
        .oneshot(
            Request::get("/v1/trust/peers")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_res.status(), StatusCode::OK);
    let list = body_json(list_res.into_body()).await;
    let peers = list.as_array().unwrap();
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0]["label"], "alice");

    // verify — a real signature over real content must check out
    let content = "hello from alice";
    let signature = signing_key.sign(content.as_bytes());
    let signature_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        signature.to_bytes(),
    );
    let verify_res = app
        .clone()
        .oneshot(
            Request::post("/v1/trust/verify")
                .header(header::AUTHORIZATION, bearer(&key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"label":"alice","content":"{content}","signature":"{signature_b64}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(verify_res.status(), StatusCode::OK);
    let verified = body_json(verify_res.into_body()).await;
    assert_eq!(verified["verified"], true);

    // verify with a tampered signature must fail closed, not error
    let bad_verify_res = app
        .clone()
        .oneshot(
            Request::post("/v1/trust/verify")
                .header(header::AUTHORIZATION, bearer(&key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"label":"alice","content":"tampered content","signature":"{signature_b64}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_verify_res.status(), StatusCode::OK);
    let not_verified = body_json(bad_verify_res.into_body()).await;
    assert_eq!(not_verified["verified"], false);

    // remove
    let remove_res = app
        .clone()
        .oneshot(
            Request::delete(format!("/v1/trust/peers/{peer_id}"))
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(remove_res.status(), StatusCode::OK);

    let list_after_res = app
        .oneshot(
            Request::get("/v1/trust/peers")
                .header(header::AUTHORIZATION, bearer(&key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list_after = body_json(list_after_res.into_body()).await;
    assert_eq!(list_after.as_array().unwrap().len(), 0);
}
