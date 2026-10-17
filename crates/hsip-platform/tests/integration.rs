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
