//! Runs the product's core flow against a real PostgreSQL server.
//! Ignored by default so `cargo test` needs no database server:
//!
//!   HSP_TEST_POSTGRES_URL=postgres://postgres:pw@127.0.0.1:5432/postgres \
//!     cargo test -p hsip-platform --test postgres_compat -- --ignored
//!
//! Each run creates and uses its own throwaway database.

mod common;

use axum::{body::Body, http::Request, http::StatusCode};
use common::*;
use hsip_platform::{anchor, config::Config};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
#[ignore = "needs HSP_TEST_POSTGRES_URL"]
async fn core_flow_on_postgres() {
    let admin_url = std::env::var("HSP_TEST_POSTGRES_URL").expect("HSP_TEST_POSTGRES_URL");
    let dbname = format!("hsp_test_{}", uuid::Uuid::new_v4().simple());
    sqlx::any::install_default_drivers();
    let admin = sqlx::any::AnyPoolOptions::new()
        .max_connections(1)
        .connect(&admin_url)
        .await
        .unwrap();
    sqlx::query(&format!("CREATE DATABASE {dbname}"))
        .execute(&admin)
        .await
        .unwrap();
    let base = admin_url.rsplit_once('/').unwrap().0;

    let mut cfg = Config::for_tests();
    cfg.database_url = format!("{base}/{dbname}");
    let e = env_with(cfg).await;

    let t = signup(&e, "owner@pg.test", "Postgres Co").await;
    let sid = stream(&e, &t, "Orders").await;
    let (key, _) = api_key(&e, &t, "ingest", Some(&sid)).await;
    for n in 1..=6 {
        let (st, v) = ingest(&e, &key, &sid, n).await;
        assert_eq!(st, StatusCode::OK, "{v}");
    }
    assert!(anchor::run_cycle(&e.state, &[]).await.unwrap() >= 6);

    let q = |s: &str| format!("/v1/orgs/{}/proofs{s}", t.org);
    let (_, l) = call(
        &e.app,
        "GET",
        &q("?action=loan.decision&limit=4&offset=1"),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(l["total"], 6);
    assert_eq!(l["items"].as_array().unwrap().len(), 4);
    let (_, l) = call(
        &e.app,
        "GET",
        &q("?q=APP-3&anchored=true&from_ms=0"),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(l["total"], 1);
    let pid = l["items"][0]["id"].as_str().unwrap().to_string();

    let (_, d) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/proofs/{pid}", t.org),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(d["verification"]["valid"], true);
    assert_eq!(d["verification"]["anchored"], true);

    let (_, i) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/streams/{sid}/integrity", t.org),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(i["intact"], true);
    assert_eq!(i["checked"], 6);

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
        ov["daily"]["counts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_i64().unwrap())
            .sum::<i64>(),
        6
    );

    let (st, ev) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/evidence", t.org),
        Some(&t.token),
        Some(json!({"stream_id": sid})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{ev}");
    assert_eq!(ev["verification"]["valid"], true);

    let (_, ops) = call(
        &e.app,
        "GET",
        "/v1/operator/overview",
        Some("operator-key-for-tests-0123456789"),
        None,
    )
    .await;
    assert!(ops["orgs"][0]["proofs_total"].as_i64().unwrap() >= 6);

    // Concurrent writers on one stream must not fork the chain on Postgres either.
    let mut hs = Vec::new();
    for n in 100..120 {
        let (app, key, sid) = (e.app.clone(), key.clone(), sid.clone());
        hs.push(tokio::spawn(async move {
            let req = Request::builder().method("POST").uri("/v1/ingest")
                .header("authorization", format!("Bearer {key}")).header("content-type", "application/json")
                .body(Body::from(json!({"stream": sid, "actor": "a", "action": "x.y", "payload_hash": hash_of(&n.to_string())}).to_string())).unwrap();
            app.oneshot(req).await.unwrap().status()
        }));
    }
    let mut ok = 0;
    for h in hs {
        let s = h.await.unwrap();
        assert!(s == StatusCode::OK || s == StatusCode::CONFLICT, "{s}");
        if s == StatusCode::OK {
            ok += 1;
        }
    }
    let (_, i) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}/streams/{sid}/integrity", t.org),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(i["intact"], true, "{i}");
    assert_eq!(i["checked"], 6 + ok);

    sqlx::query(&format!("DROP DATABASE {dbname} WITH (FORCE)"))
        .execute(&admin)
        .await
        .ok();
}
