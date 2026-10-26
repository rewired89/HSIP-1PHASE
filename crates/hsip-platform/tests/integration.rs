mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::*;
use hsip_platform::{anchor, app, config::Config, state::AppState};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn signup_validation_and_duplicate() {
    let e = env().await;
    let (st, _) = call(
        &e.app,
        "POST",
        "/v1/auth/signup",
        None,
        Some(json!({"email":"a@b.co","password":"short","name":"A","org_name":"Acme"})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let t = signup(&e, "owner@acme.test", "Acme").await;
    let (st, v) = call(&e.app, "GET", "/v1/auth/me", Some(&t.token), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["orgs"][0]["role"], "owner");
    let (st, _) = call(&e.app, "POST", "/v1/auth/signup", None, Some(json!({"email":"OWNER@acme.test","password":"correct-horse-battery","name":"A","org_name":"Acme2"}))).await;
    assert_eq!(st, StatusCode::CONFLICT);
    let (st, _) = call(&e.app, "GET", "/v1/auth/me", None, None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn end_to_end_record_anchor_verify_share_evidence() {
    let e = env().await;
    let t = signup(&e, "owner@acme.test", "Acme Bank").await;
    let sid = stream(&e, &t, "Loan decisions").await;
    let (key, _) = api_key(&e, &t, "ingest", Some(&sid)).await;

    let mut ids = Vec::new();
    for n in 1..=5 {
        let (st, v) = ingest(&e, &key, &sid, n).await;
        assert_eq!(st, StatusCode::OK, "{v}");
        assert_eq!(v["seq"], n.to_string());
        ids.push(v["proof_id"].as_str().unwrap().to_string());
    }

    // Before anchoring: valid, not anchored.
    let (_, v) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/proofs/{}", t.org, ids[0]),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(v["verification"]["valid"], true);
    assert_eq!(v["verification"]["anchored"], false);

    // Anchor, then every proof carries a verifiable Merkle path.
    let n = anchor::run_cycle(&e.state, &[]).await.unwrap();
    assert!(n >= 5);
    let (_, v) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/proofs/{}", t.org, ids[2]),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(v["verification"]["valid"], true);
    assert_eq!(v["verification"]["anchored"], true);
    let bundle = v["bundle"].clone();

    // Public verification: no auth, no DB.
    let (st, pv) = call(&e.app, "POST", "/v1/verify", None, Some(bundle.clone())).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(pv["valid"], true);

    // Tampering with any envelope field is caught.
    let mut forged = bundle.clone();
    forged["envelope"]["action"] = json!("loan.approve");
    let (_, pv) = call(&e.app, "POST", "/v1/verify", None, Some(forged)).await;
    assert_eq!(pv["valid"], false);
    // A re-hashed forgery fails the signature instead.
    let mut forged2 = bundle.clone();
    forged2["envelope"]["actor"] = json!("someone-else");
    let env_typed: hsip_proof::Envelope =
        serde_json::from_value(forged2["envelope"].clone()).unwrap();
    forged2["event_hash"] = json!(hex::encode(hsip_proof::event_hash(&env_typed).unwrap()));
    let (_, pv) = call(&e.app, "POST", "/v1/verify", None, Some(forged2)).await;
    assert_eq!(pv["valid"], false);

    // Original content matching.
    let (_, m) = call(
        &e.app,
        "POST",
        "/v1/verify/match",
        None,
        Some(json!({"bundle": bundle, "content": "record 3"})),
    )
    .await;
    assert_eq!(m["matches"], true);
    let (_, m) = call(
        &e.app,
        "POST",
        "/v1/verify/match",
        None,
        Some(json!({"bundle": bundle, "content": "record 4"})),
    )
    .await;
    assert_eq!(m["matches"], false);

    // Stream integrity.
    let (_, i) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/streams/{}/integrity", t.org, sid),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(i["intact"], true);
    assert_eq!(i["checked"], 5);

    // Share link.
    let (st, sh) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/proofs/{}/shares", t.org, ids[1]),
        Some(&t.token),
        Some(json!({"expires_days": 30})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{sh}");
    let tok = sh["token"].as_str().unwrap().to_string();
    let (st, view) = call(
        &e.app,
        "GET",
        &format!("/v1/public/shares/{tok}"),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(view["verification"]["valid"], true);
    assert_eq!(view["org_name"], "Acme Bank");
    let (st, _) = call(
        &e.app,
        "DELETE",
        &format!("/v1/orgs/{}/shares/{}", t.org, sh["id"].as_str().unwrap()),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = call(
        &e.app,
        "GET",
        &format!("/v1/public/shares/{tok}"),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Evidence pack: created, downloadable, and independently verifiable.
    let (st, ev) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/evidence", t.org),
        Some(&t.token),
        Some(json!({"stream_id": sid, "title": "Q4 loans"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{ev}");
    assert_eq!(ev["proof_count"], 5);
    assert_eq!(ev["verification"]["valid"], true);
    let eid = ev["id"].as_str().unwrap();
    let (st, pack_bytes) = call_raw(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/evidence/{eid}/pack", t.org),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let pack: Value = serde_json::from_slice(&pack_bytes).unwrap();
    let (_, pv) = call(&e.app, "POST", "/v1/verify", None, Some(pack.clone())).await;
    assert_eq!(pv["kind"], "evidence_pack");
    assert_eq!(pv["valid"], true);
    // Removing a record from the pack is detected.
    let mut cut = pack.clone();
    cut["proofs"].as_array_mut().unwrap().remove(2);
    let (_, pv) = call(&e.app, "POST", "/v1/verify", None, Some(cut)).await;
    assert_eq!(pv["valid"], false);
    // Printable report renders and escapes.
    let (st, html) = call_raw(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/evidence/{eid}/report", t.org),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let html = String::from_utf8(html).unwrap();
    assert!(
        html.contains("VERIFIED")
            && html.contains("Q4 loans")
            && html.contains("Content-Security-Policy")
    );

    // The platform's own audit trail is itself a proof stream.
    let (_, streams) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/streams", t.org),
        Some(&t.token),
        None,
    )
    .await;
    let sys = streams["streams"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["kind"] == "system")
        .unwrap();
    assert!(sys["proof_count"].as_i64().unwrap() >= 4, "{sys}");
}

#[tokio::test]
async fn database_tampering_is_detected_by_integrity_check() {
    let e = env().await;
    let t = signup(&e, "o@x.test", "TamperCo").await;
    let sid = stream(&e, &t, "Orders").await;
    let (key, _) = api_key(&e, &t, "ingest", Some(&sid)).await;
    for n in 1..=4 {
        ingest(&e, &key, &sid, n).await;
    }
    sqlx::query("UPDATE proofs SET action = 'order.cancelled' WHERE stream_id = $1 AND seq = 2")
        .bind(&sid)
        .execute(&e.state.db)
        .await
        .unwrap();
    let (_, i) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/streams/{}/integrity", t.org, sid),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(i["intact"], false);
    assert_eq!(i["first_break_seq"], 2);

    // Deleting a record is a sequence gap.
    let e2 = env().await;
    let t2 = signup(&e2, "o@x.test", "GapCo").await;
    let s2 = stream(&e2, &t2, "Orders").await;
    let (k2, _) = api_key(&e2, &t2, "ingest", Some(&s2)).await;
    for n in 1..=4 {
        ingest(&e2, &k2, &s2, n).await;
    }
    sqlx::query("DELETE FROM proofs WHERE stream_id = $1 AND seq = 3")
        .bind(&s2)
        .execute(&e2.state.db)
        .await
        .unwrap();
    let (_, i) = call(
        &e2.app,
        "GET",
        &format!("/v1/orgs/{}/streams/{}/integrity", t2.org, s2),
        Some(&t2.token),
        None,
    )
    .await;
    assert_eq!(i["intact"], false);
    assert_eq!(i["first_break_seq"], 4);
}

#[tokio::test]
async fn roles_invites_and_tenant_isolation() {
    let e = env().await;
    let a = signup(&e, "owner@a.test", "Org A").await;
    let b = signup(&e, "owner@b.test", "Org B").await;
    let sid_a = stream(&e, &a, "A stream").await;

    // Org B cannot see Org A at all.
    for path in [
        format!("/v1/orgs/{}", a.org),
        format!("/v1/orgs/{}/streams", a.org),
        format!("/v1/orgs/{}/api-keys", a.org),
    ] {
        let (st, _) = call(&e.app, "GET", &path, Some(&b.token), None).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{path}");
    }
    // A key from Org B cannot write into Org A's stream.
    let sid_b = stream(&e, &b, "B stream").await;
    let (kb, _) = api_key(&e, &b, "ingest", None).await;
    let (st, _) = ingest(&e, &kb, &sid_a, 1).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = ingest(&e, &kb, &sid_b, 1).await;
    assert_eq!(st, StatusCode::OK);

    // Invite an auditor.
    let (st, inv) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/invites", a.org),
        Some(&a.token),
        Some(json!({"email":"aud@a.test","role":"auditor"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{inv}");
    let token = inv["invite_url"]
        .as_str()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .to_string();
    let (st, pre) = call(
        &e.app,
        "GET",
        &format!("/v1/auth/invite/{token}"),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(pre["role"], "auditor");
    let (st, acc) = call(
        &e.app,
        "POST",
        "/v1/auth/accept-invite",
        None,
        Some(json!({"token": token, "name": "Aud", "password": "correct-horse-battery"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{acc}");
    let aud = Tenant {
        token: acc["token"].as_str().unwrap().into(),
        org: a.org.clone(),
    };
    // Invite is single-use.
    let (st, _) = call(
        &e.app,
        "POST",
        "/v1/auth/accept-invite",
        None,
        Some(json!({"token": token, "name": "X", "password": "correct-horse-battery"})),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Auditor: read yes, write no, evidence yes.
    let (st, _) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/proofs", a.org),
        Some(&aud.token),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/streams", a.org),
        Some(&aud.token),
        Some(json!({"name":"nope"})),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/api-keys", a.org),
        Some(&aud.token),
        Some(json!({"name":"k","scope":"ingest"})),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/streams/{}/proofs", a.org, sid_a),
        Some(&aud.token),
        Some(json!({"action":"x.y","payload_hash":hash_of("z")})),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // Last-owner guard.
    let (_, me) = call(&e.app, "GET", "/v1/auth/me", Some(&a.token), None).await;
    let uid = me["user"]["id"].as_str().unwrap();
    let (st, _) = call(
        &e.app,
        "PATCH",
        &format!("/v1/orgs/{}/members/{uid}", a.org),
        Some(&a.token),
        Some(json!({"role":"admin"})),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    let (st, _) = call(
        &e.app,
        "DELETE",
        &format!("/v1/orgs/{}/members/{uid}", a.org),
        Some(&a.token),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
}

#[tokio::test]
async fn api_key_scopes_revocation_and_stream_binding() {
    let e = env().await;
    let t = signup(&e, "o@k.test", "KeyCo").await;
    let s1 = stream(&e, &t, "One").await;
    let s2 = stream(&e, &t, "Two").await;
    let (bound, bound_id) = api_key(&e, &t, "ingest", Some(&s1)).await;
    let (reader, _) = api_key(&e, &t, "read", None).await;

    let (st, v) = ingest(&e, &bound, &s1, 1).await;
    assert_eq!(st, StatusCode::OK);
    let pid = v["proof_id"].as_str().unwrap().to_string();
    let (st, _) = ingest(&e, &bound, &s2, 2).await;
    assert_eq!(
        st,
        StatusCode::FORBIDDEN,
        "bound key must not write other streams"
    );
    let (st, _) = ingest(&e, &reader, &s1, 3).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "read key must not ingest");
    let (st, g) = call(
        &e.app,
        "GET",
        &format!("/v1/proofs/{pid}"),
        Some(&reader),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(g["verification"]["valid"], true);
    let (st, _) = call(
        &e.app,
        "GET",
        &format!("/v1/proofs/{pid}"),
        Some(&bound),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    let (st, _) = call(
        &e.app,
        "DELETE",
        &format!("/v1/orgs/{}/api-keys/{bound_id}", t.org),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = ingest(&e, &bound, &s1, 4).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, _) = ingest(&e, "hsp_garbage", &s1, 5).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    // Input validation: raw content is refused, only a hash is accepted.
    let (k, _) = api_key(&e, &t, "ingest", Some(&s1)).await;
    let (st, _) = call(
        &e.app,
        "POST",
        "/v1/ingest",
        Some(&k),
        Some(json!({"actor":"a","action":"x.y","payload_hash":"not-a-hash"})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn plan_limits_and_operator_controls() {
    let e = env().await;
    let t = signup(&e, "o@p.test", "PlanCo").await;
    let sid = stream(&e, &t, "Main").await;
    let (key, _) = api_key(&e, &t, "ingest", Some(&sid)).await;
    let month = hsip_platform::db::month_key(hsip_platform::db::now_ms());

    sqlx::query("INSERT INTO usage_monthly (org_id, month, proofs) VALUES ($1,$2,999)")
        .bind(&t.org)
        .bind(&month)
        .execute(&e.state.db)
        .await
        .unwrap();
    let (st, _) = ingest(&e, &key, &sid, 1).await;
    assert_eq!(st, StatusCode::OK);
    let (st, v) = ingest(&e, &key, &sid, 2).await;
    assert_eq!(st, StatusCode::PAYMENT_REQUIRED, "{v}");

    // Stream limit on the free plan is 2.
    let _ = stream(&e, &t, "Second").await;
    let (st, _) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/streams", t.org),
        Some(&t.token),
        Some(json!({"name":"Third"})),
    )
    .await;
    assert_eq!(st, StatusCode::PAYMENT_REQUIRED);

    // Operator endpoints need the operator key.
    let op = "operator-key-for-tests-0123456789";
    let (st, _) = call(&e.app, "GET", "/v1/operator/overview", Some(&t.token), None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, ov) = call(&e.app, "GET", "/v1/operator/overview", Some(op), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(ov["orgs"][0]["name"], "PlanCo");

    // Upgrade unlocks ingestion and streams.
    let (st, _) = call(
        &e.app,
        "POST",
        &format!("/v1/operator/orgs/{}/plan", t.org),
        Some(op),
        Some(json!({"plan":"team"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = ingest(&e, &key, &sid, 3).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/streams", t.org),
        Some(&t.token),
        Some(json!({"name":"Third"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // Suspension blocks ingestion but not reads.
    call(
        &e.app,
        "POST",
        &format!("/v1/operator/orgs/{}/status", t.org),
        Some(op),
        Some(json!({"status":"suspended"})),
    )
    .await;
    let (st, _) = ingest(&e, &key, &sid, 4).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/proofs", t.org),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
}

#[tokio::test]
async fn overview_listing_and_filters() {
    let e = env().await;
    let t = signup(&e, "o@o.test", "OverviewCo").await;
    let sid = stream(&e, &t, "Trades").await;
    let (key, _) = api_key(&e, &t, "ingest", Some(&sid)).await;
    for n in 1..=6 {
        let action = if n % 2 == 0 {
            "trade.buy"
        } else {
            "trade.sell"
        };
        call(&e.app, "POST", "/v1/ingest", Some(&key), Some(json!({"actor": "bot-1", "action": action, "payload_hash": hash_of(&n.to_string())}))).await;
    }
    let (_, ov) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/overview", t.org),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(ov["total_proofs"], 6);
    assert_eq!(ov["usage"]["proofs_used"], 6);
    assert_eq!(
        ov["hourly"]["counts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_i64().unwrap())
            .sum::<i64>(),
        6
    );
    assert_eq!(ov["top_actions"].as_array().unwrap().len(), 2);

    let q = |extra: &str| format!("/v1/orgs/{}/proofs?{extra}", t.org);
    let (_, l) = call(&e.app, "GET", &q("action=trade.buy"), Some(&t.token), None).await;
    assert_eq!(l["total"], 3);
    let (_, l) = call(
        &e.app,
        "GET",
        &q("q=BOT-1&limit=2&offset=2"),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(l["total"], 6);
    assert_eq!(l["items"].as_array().unwrap().len(), 2);
    let (_, l) = call(&e.app, "GET", &q("anchored=true"), Some(&t.token), None).await;
    assert_eq!(l["total"], 0);
    // LIKE wildcards in search text are literal.
    let (_, l) = call(&e.app, "GET", &q("q=%25"), Some(&t.token), None).await;
    assert_eq!(l["total"], 0);
    // The default listing hides the system stream.
    let (_, all) = call(&e.app, "GET", &q(""), Some(&t.token), None).await;
    assert_eq!(all["total"], 6);
}

#[tokio::test]
async fn login_is_throttled_and_errors_do_not_leak() {
    let e = env().await;
    signup(&e, "o@l.test", "LoginCo").await;
    let mut last = StatusCode::OK;
    for _ in 0..12 {
        let (st, v) = call(
            &e.app,
            "POST",
            "/v1/auth/login",
            None,
            Some(json!({"email":"o@l.test","password":"wrong-password-here"})),
        )
        .await;
        if st == StatusCode::UNAUTHORIZED {
            assert_eq!(v["error"]["message"], "incorrect email or password");
        }
        last = st;
    }
    assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn concurrent_ingest_never_forks_the_chain() {
    let dir = std::env::temp_dir().join(format!("hsp-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut cfg = Config::for_tests();
    cfg.database_url = format!("sqlite://{}/t.db?mode=rwc", dir.display());
    let state = AppState::new(cfg).await.unwrap();
    let e = Env {
        app: app(state.clone()),
        state,
    };
    let t = signup(&e, "o@c.test", "ConcCo").await;
    let sid = stream(&e, &t, "Hot").await;
    let (key, _) = api_key(&e, &t, "ingest", Some(&sid)).await;

    let mut handles = Vec::new();
    for n in 0..16 {
        let (app, key, sid) = (e.app.clone(), key.clone(), sid.clone());
        handles.push(tokio::spawn(async move {
            let req = Request::builder().method("POST").uri("/v1/ingest")
                .header("authorization", format!("Bearer {key}")).header("content-type", "application/json")
                .body(Body::from(json!({"stream": sid, "actor":"a", "action":"x.y", "payload_hash": hash_of(&n.to_string())}).to_string())).unwrap();
            app.oneshot(req).await.unwrap().status()
        }));
    }
    let mut ok = 0;
    for h in handles {
        let st = h.await.unwrap();
        assert!(
            st == StatusCode::OK || st == StatusCode::CONFLICT,
            "unexpected {st}"
        );
        if st == StatusCode::OK {
            ok += 1;
        }
    }
    assert!(ok >= 1);
    let (_, i) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/streams/{}/integrity", t.org, sid),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(i["intact"], true, "{i}");
    assert_eq!(i["checked"], ok);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn free_signups_become_prospects_and_service_requests_reach_the_operator() {
    let e = env().await;
    let op = "operator-key-for-tests-0123456789";
    let t = signup(&e, "founder@prospect.test", "Prospect Ltd").await;

    // Catalogue is public and honest about status.
    let (_, meta) = call(&e.app, "GET", "/v1/meta", None, None).await;
    let svc = meta["services"].as_array().unwrap();
    assert!(svc
        .iter()
        .any(|s| s["id"] == "dedicated" && s["status"] == "available"));
    assert!(svc
        .iter()
        .any(|s| s["id"] == "sso" && s["status"] == "roadmap"));
    assert_eq!(meta["name"], "HSIP");

    // Unknown services are rejected; real ones are recorded for the operator.
    let path = format!("/v1/orgs/{}/service-request", t.org);
    let (st, _) = call(
        &e.app,
        "POST",
        &path,
        Some(&t.token),
        Some(json!({"service": "nope"})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, _) = call(
        &e.app,
        "POST",
        &path,
        Some(&t.token),
        Some(json!({"service": "dedicated", "message": "EU data residency"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    let (_, leads) = call(&e.app, "GET", "/v1/operator/leads", Some(op), None).await;
    let kinds: Vec<&str> = leads["leads"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["kind"].as_str().unwrap())
        .collect();
    assert!(
        kinds.contains(&"signup") && kinds.contains(&"service_request"),
        "{kinds:?}"
    );
    assert!(leads["leads"]
        .as_array()
        .unwrap()
        .iter()
        .any(|l| l["email"] == "founder@prospect.test"
            && l["message"].as_str().unwrap().contains("EU data residency")));

    // Operator sees who owns the free account and when it was last active.
    let (_, ov) = call(&e.app, "GET", "/v1/operator/overview", Some(op), None).await;
    assert_eq!(ov["orgs"][0]["owner_email"], "founder@prospect.test");
    assert_eq!(ov["orgs"][0]["plan"], "free");
    assert!(ov["orgs"][0]["last_proof_at"].is_null());

    // Auditors cannot request services on the organization's behalf.
    let (_, inv) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/invites", t.org),
        Some(&t.token),
        Some(json!({"email":"aud@prospect.test","role":"auditor"})),
    )
    .await;
    let tok = inv["invite_url"]
        .as_str()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .to_string();
    let (_, acc) = call(
        &e.app,
        "POST",
        "/v1/auth/accept-invite",
        None,
        Some(json!({"token": tok, "name": "Aud", "password": "correct-horse-battery"})),
    )
    .await;
    let (st, _) = call(
        &e.app,
        "POST",
        &path,
        acc["token"].as_str(),
        Some(json!({"service": "sso"})),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn customer_held_keys_full_flow() {
    let e = env().await;
    let t = signup(&e, "owner@bank.test", "Bank").await;
    let sid = stream(&e, &t, "Loans").await;
    let (key, _) = api_key(&e, &t, "ingest", Some(&sid)).await;
    let ckey = hsip_proof::generate_signing_key();
    let cvk = hsip_proof::verify_key_b64(&ckey);
    let url = format!("/v1/orgs/{}/key-custody", t.org);

    // Platform-signed record before the switch.
    let (st, v) = ingest(&e, &key, &sid, 1).await;
    assert_eq!(st, StatusCode::OK, "{v}");
    // Prepare is refused while the platform holds the key.
    let prep =
        json!({"actor":"m","action":"a","subject":"s","payload_hash":hash_of("x"),"metadata":{}});
    let (st, _) = call(
        &e.app,
        "POST",
        "/v1/ingest/prepare",
        Some(&key),
        Some(prep.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // Registration: wrong proof (made for another org) is rejected.
    let bad = hsip_proof::sign_registration("someone-else", &ckey);
    let (st, _) = call(
        &e.app,
        "POST",
        &url,
        Some(&t.token),
        Some(json!({"verify_key": cvk, "proof": bad})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let proof = hsip_proof::sign_registration(&t.org, &ckey);
    // Only the owner may register.
    let (st, _) = call(
        &e.app,
        "POST",
        &url,
        None,
        Some(json!({"verify_key": cvk, "proof": proof})),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, v) = call(
        &e.app,
        "POST",
        &url,
        Some(&t.token),
        Some(json!({"verify_key": cvk, "proof": proof})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    let (_, o) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}", t.org),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(o["custody"]["mode"], "customer");

    // Single-step ingest is now refused.
    let (st, _) = ingest(&e, &key, &sid, 2).await;
    assert_eq!(st, StatusCode::CONFLICT);

    // Prepare, check, sign, commit.
    let meta = std::collections::BTreeMap::new();
    let mut commit_body = Value::Null;
    let mut first_id = String::new();
    for n in 0..3 {
        let payload = hash_of(&format!("c{n}"));
        let (st, p) = call(&e.app, "POST", "/v1/ingest/prepare", Some(&key), Some(
            json!({"actor":"m","action":"a","subject":"s","payload_hash":payload,"metadata":{}}))).await;
        assert_eq!(st, StatusCode::OK, "{p}");
        let env: hsip_proof::Envelope = serde_json::from_value(p["envelope"].clone()).unwrap();
        hsip_proof::check_prepared(
            &env,
            p["event_hash"].as_str().unwrap(),
            "m",
            "a",
            "s",
            &payload,
            &meta,
        )
        .unwrap();
        let (_, sig) = hsip_proof::sign_envelope(&env, &ckey).unwrap();
        let body = json!({"envelope": p["envelope"], "signature": sig});
        // Signature by a different key is rejected.
        let other = hsip_proof::generate_signing_key();
        let (_, osig) = hsip_proof::sign_envelope(&env, &other).unwrap();
        let (st, _) = call(
            &e.app,
            "POST",
            "/v1/ingest/commit",
            Some(&key),
            Some(json!({"envelope": p["envelope"], "signature": osig})),
        )
        .await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
        // Tampered envelope is rejected.
        let mut tampered = p["envelope"].clone();
        tampered["actor"] = json!("evil");
        let (st, _) = call(
            &e.app,
            "POST",
            "/v1/ingest/commit",
            Some(&key),
            Some(json!({"envelope": tampered, "signature": sig})),
        )
        .await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
        let (st, v) = call(
            &e.app,
            "POST",
            "/v1/ingest/commit",
            Some(&key),
            Some(body.clone()),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{v}");
        assert_eq!(v["bundle"]["org_verify_key"], cvk);
        // Replay of the same commit is refused (chain moved on).
        let (st, _) = call(
            &e.app,
            "POST",
            "/v1/ingest/commit",
            Some(&key),
            Some(body.clone()),
        )
        .await;
        assert_eq!(st, StatusCode::CONFLICT);
        if n == 0 {
            first_id = v["proof_id"].as_str().unwrap().into();
        }
        commit_body = v["bundle"].clone();
    }
    // Public verification of a customer-signed bundle.
    let (_, pv) = call(&e.app, "POST", "/v1/verify", None, Some(commit_body)).await;
    assert_eq!(pv["valid"], true, "{pv}");

    // A stale prepare is refused once the chain has moved.
    let (_, p1) = call(
        &e.app,
        "POST",
        "/v1/ingest/prepare",
        Some(&key),
        Some(prep.clone()),
    )
    .await;
    let (_, p2) = call(
        &e.app,
        "POST",
        "/v1/ingest/prepare",
        Some(&key),
        Some(prep.clone()),
    )
    .await;
    let env1: hsip_proof::Envelope = serde_json::from_value(p1["envelope"].clone()).unwrap();
    let env2: hsip_proof::Envelope = serde_json::from_value(p2["envelope"].clone()).unwrap();
    let (_, s1) = hsip_proof::sign_envelope(&env1, &ckey).unwrap();
    let (_, s2) = hsip_proof::sign_envelope(&env2, &ckey).unwrap();
    let (st, _) = call(
        &e.app,
        "POST",
        "/v1/ingest/commit",
        Some(&key),
        Some(json!({"envelope": p1["envelope"], "signature": s1})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = call(
        &e.app,
        "POST",
        "/v1/ingest/commit",
        Some(&key),
        Some(json!({"envelope": p2["envelope"], "signature": s2})),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);

    // Anchor, then the stored proof (mixed signers) still verifies.
    anchor::run_cycle(&e.state, &[]).await.unwrap();
    let (_, v) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/proofs/{first_id}", t.org),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(v["verification"]["valid"], true, "{v}");
    assert_eq!(v["verification"]["anchored"], true, "{v}");
    let (_, v) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/streams/{sid}/integrity", t.org),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(v["intact"], true, "{v}");

    // Evidence pack across mixed signers verifies; swapping a signer key fails.
    let (st, ev) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/evidence", t.org),
        Some(&t.token),
        Some(json!({"stream_id": sid})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{ev}");
    let eid = ev["id"].as_str().unwrap();
    let (_, pack) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/evidence/{eid}/pack", t.org),
        Some(&t.token),
        None,
    )
    .await;
    assert!(
        pack["manifest"]["issuer_keys"].as_array().unwrap().len() == 2,
        "{pack}"
    );
    let (_, pv) = call(&e.app, "POST", "/v1/verify", None, Some(pack.clone())).await;
    assert_eq!(pv["valid"], true, "{pv}");
    let mut forged = pack.clone();
    let evil = hsip_proof::verify_key_b64(&hsip_proof::generate_signing_key());
    forged["manifest"]["issuer_keys"] = json!([evil]);
    let (_, pv) = call(&e.app, "POST", "/v1/verify", None, Some(forged)).await;
    assert_eq!(pv["valid"], false);

    // Console test-proof path is blocked too: system stream still verifies.
    let (_, sys) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/streams", t.org),
        Some(&t.token),
        None,
    )
    .await;
    for s in sys.as_array().cloned().unwrap_or_default() {
        if s["kind"] == "system" {
            let id = s["id"].as_str().unwrap();
            let (_, v) = call(
                &e.app,
                "GET",
                &format!("/v1/orgs/{}/streams/{id}/integrity", t.org),
                Some(&t.token),
                None,
            )
            .await;
            assert_eq!(v["intact"], true, "{v}");
        }
    }
}

#[tokio::test]
async fn permissions_flag_records_outside_what_was_allowed() {
    use hsip_proof::{mandate::*, signing_key_from_b64, verify_key_b64};
    let e = env().await;
    let t = signup(&e, "owner@perm.test", "Perm Co").await;
    let sid = stream(&e, &t, "Agent actions").await;
    let (key, _) = api_key(&e, &t, "ingest", Some(&sid)).await;
    let grantor =
        signing_key_from_b64(&hsip_proof::seed_b64(&hsip_proof::generate_signing_key())).unwrap();
    let now = hsip_platform::db::now_ms();
    let body = MandateBody {
        format: MANDATE_FORMAT.into(),
        mandate_id: "m-1".into(),
        grantor_key: verify_key_b64(&grantor),
        agent: "pay-bot".into(),
        valid_from_ms: now - 3_600_000,
        valid_until_ms: now + 3_600_000,
        rules: Rules {
            allowed_actions: vec!["pay".into()],
            max_per_period: Some(RateRule {
                count: 2,
                period_secs: 3600,
            }),
            max_amount: Some(AmountRule {
                field: "amount".into(),
                limit: 100,
            }),
            allowed_values: Default::default(),
        },
    };
    let m = sign_mandate(body, &grantor).unwrap();
    let url = format!("/v1/orgs/{}/mandates", t.org);

    // A tampered mandate is refused, the genuine one is stored once.
    let mut forged = serde_json::to_value(&m).unwrap();
    forged["rules"]["max_amount"]["limit"] = json!(1_000_000);
    let (st, _) = call(&e.app, "POST", &url, Some(&t.token), Some(forged)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, v) = call(
        &e.app,
        "POST",
        &url,
        Some(&t.token),
        Some(serde_json::to_value(&m).unwrap()),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    let (st, _) = call(
        &e.app,
        "POST",
        &url,
        Some(&t.token),
        Some(serde_json::to_value(&m).unwrap()),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);

    let send =
        |action: &'static str, amount: &'static str, n: usize| {
            let (app, key, sid) = (e.app.clone(), key.clone(), sid.clone());
            async move {
                call(&app, "POST", "/v1/ingest", Some(&key), Some(json!({
                "stream": sid, "actor": "pay-bot", "action": action, "subject": format!("r{n}"),
                "payload_hash": hash_of(&format!("rec {n}")), "metadata": {"amount": amount}
            }))).await
            }
        };
    assert_eq!(send("pay", "50", 1).await.0, StatusCode::OK);
    assert_eq!(send("pay", "250", 2).await.0, StatusCode::OK); // over the amount
    assert_eq!(send("transfer", "10", 3).await.0, StatusCode::OK); // wrong action, also 3rd in the hour
                                                                   // Another actor is ignored.
    let (st, _) = call(
        &e.app,
        "POST",
        "/v1/ingest",
        Some(&key),
        Some(json!({
            "stream": sid, "actor": "someone-else", "action": "pay", "subject": "x",
            "payload_hash": hash_of("other"), "metadata": {"amount": "99999"}
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    let rurl = format!("/v1/orgs/{}/mandates/m-1/report?stream_id={sid}", t.org);
    let (st, v) = call(&e.app, "GET", &rurl, Some(&t.token), None).await;
    assert_eq!(st, StatusCode::OK, "{v}");
    let rules: Vec<String> = v["report"]["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["rule"].as_str().unwrap().to_string())
        .collect();
    assert!(
        rules.contains(&"amount_over_limit".to_string()),
        "{rules:?}"
    );
    assert!(
        rules.contains(&"action_not_allowed".to_string()),
        "{rules:?}"
    );
    assert!(rules.contains(&"rate_exceeded".to_string()), "{rules:?}");
    assert_eq!(v["report"]["checked"], 3);
    assert_eq!(v["report"]["within_permission"], false);

    // The same answer offline, from the mandate and an evidence pack.
    let (_, ev) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/evidence", t.org),
        Some(&t.token),
        Some(json!({"stream_id": sid})),
    )
    .await;
    let eid = ev["id"].as_str().unwrap();
    let (_, pack) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/evidence/{eid}/pack", t.org),
        Some(&t.token),
        None,
    )
    .await;
    let proofs = proofs_from_value(pack).unwrap();
    let offline = evaluate(&m, &proofs);
    assert_eq!(
        offline.violations.len(),
        v["report"]["violations"].as_array().unwrap().len()
    );

    // Auditors may read but not create or revoke.
    let (st, _) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/mandates/m-1/revoke", t.org),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, _) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/mandates/m-1/revoke", t.org),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (_, l) = call(&e.app, "GET", &url, Some(&t.token), None).await;
    assert!(l["mandates"][0]["revoked_at"].is_i64());
}
