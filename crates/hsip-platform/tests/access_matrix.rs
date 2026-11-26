//! Who may call what. Every session-authenticated route is exercised as every
//! kind of caller, so a missing role check (an auditor creating a public share
//! link, say) fails a test instead of shipping. A second test reads
//! `routes/mod.rs` and fails when a route is added without a matrix entry.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{json, Value};

const FAKE: &str = "00000000-0000-0000-0000-000000000000";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Min {
    Auditor = 1,
    Developer = 2,
    Admin = 3,
    Owner = 4,
}

struct Route {
    method: &'static str,
    /// Same `:placeholders` as `routes/mod.rs`.
    path: &'static str,
    /// Lowest role allowed to call it.
    min: Min,
    /// Request body, given the caller's label so created records stay unique.
    body: fn(&str) -> Value,
}

fn nobody(_: &str) -> Value {
    Value::Null
}

fn dummy_mandate(_: &str) -> Value {
    json!({
        "format": "hsip-mandate/1", "mandate_id": "m", "grantor_key": "AAAA", "agent": "a",
        "valid_from_ms": 0, "valid_until_ms": 1, "rules": {}, "signature": "AAAA"
    })
}

fn routes() -> Vec<Route> {
    use Min::*;
    let r = |method, path, min, body: fn(&str) -> Value| Route {
        method,
        path,
        min,
        body,
    };
    vec![
        r("GET", "/v1/orgs/:org", Auditor, nobody),
        r(
            "PATCH",
            "/v1/orgs/:org",
            Admin,
            |_| json!({"name": "Matrix Org"}),
        ),
        r(
            "POST",
            "/v1/orgs/:org/key-custody",
            Owner,
            |_| json!({"verify_key": "x", "proof": "y"}),
        ),
        r("GET", "/v1/orgs/:org/overview", Auditor, nobody),
        r("GET", "/v1/orgs/:org/usage", Auditor, nobody),
        r(
            "POST",
            "/v1/orgs/:org/plan-request",
            Admin,
            |_| json!({"plan": "team"}),
        ),
        r(
            "POST",
            "/v1/orgs/:org/service-request",
            Admin,
            |_| json!({"service": "not-a-service"}),
        ),
        r("GET", "/v1/orgs/:org/streams", Auditor, nobody),
        r(
            "POST",
            "/v1/orgs/:org/streams",
            Developer,
            |c| json!({"name": format!("Matrix {c}")}),
        ),
        r(
            "PATCH",
            "/v1/orgs/:org/streams/:sid",
            Admin,
            |_| json!({"name": "Renamed"}),
        ),
        r(
            "POST",
            "/v1/orgs/:org/streams/:sid/proofs",
            Developer,
            |_| json!({"action": "a.b", "payload_hash": "0".repeat(64)}),
        ),
        r(
            "GET",
            "/v1/orgs/:org/streams/:sid/integrity",
            Auditor,
            nobody,
        ),
        r(
            "GET",
            "/v1/orgs/:org/streams/:sid/checkpoints",
            Auditor,
            nobody,
        ),
        r("GET", "/v1/orgs/:org/proofs", Auditor, nobody),
        r("GET", "/v1/orgs/:org/proofs/:pid", Auditor, nobody),
        r(
            "POST",
            "/v1/orgs/:org/proofs/:pid/shares",
            Developer,
            |_| json!({"expires_days": 1}),
        ),
        r("GET", "/v1/orgs/:org/shares", Auditor, nobody),
        r("DELETE", "/v1/orgs/:org/shares/:id", Developer, nobody),
        r("GET", "/v1/orgs/:org/api-keys", Admin, nobody),
        r(
            "POST",
            "/v1/orgs/:org/api-keys",
            Admin,
            |_| json!({"name": "k", "scope": "ingest"}),
        ),
        r("DELETE", "/v1/orgs/:org/api-keys/:id", Admin, nobody),
        r("GET", "/v1/orgs/:org/members", Auditor, nobody),
        r(
            "PATCH",
            "/v1/orgs/:org/members/:uid",
            Admin,
            |_| json!({"role": "auditor"}),
        ),
        r("DELETE", "/v1/orgs/:org/members/:uid", Admin, nobody),
        r(
            "POST",
            "/v1/orgs/:org/invites",
            Admin,
            |c| json!({"email": format!("{c}@invitee.test"), "role": "auditor"}),
        ),
        r("DELETE", "/v1/orgs/:org/invites/:id", Admin, nobody),
        r("GET", "/v1/orgs/:org/evidence", Auditor, nobody),
        r(
            "POST",
            "/v1/orgs/:org/evidence",
            Auditor,
            |_| json!({"stream_id": FAKE}),
        ),
        r("GET", "/v1/orgs/:org/evidence/:id/pack", Auditor, nobody),
        r("GET", "/v1/orgs/:org/evidence/:id/report", Auditor, nobody),
        r("GET", "/v1/orgs/:org/mandates", Auditor, nobody),
        r("POST", "/v1/orgs/:org/mandates", Developer, dummy_mandate),
        r("POST", "/v1/orgs/:org/mandates/:id/revoke", Admin, nobody),
        r(
            "GET",
            "/v1/orgs/:org/mandates/:id/report?stream_id=x",
            Auditor,
            nobody,
        ),
        r(
            "POST",
            "/v1/orgs/:org/billing/checkout",
            Owner,
            |_| json!({"plan": "team"}),
        ),
        r("POST", "/v1/orgs/:org/billing/portal", Owner, nobody),
    ]
}

/// Routes that are not role-gated by organization membership: public, API-key,
/// session-only, webhook and operator routes. Covered by `credentials_are_not_interchangeable`
/// or intentionally open.
const EXEMPT: &[(&str, &str)] = &[
    ("GET", "/health"),
    ("GET", "/v1/meta"),
    ("POST", "/v1/verify"),
    ("POST", "/v1/verify/match"),
    ("GET", "/v1/public/shares/:token"),
    ("GET", "/v1/public/orgs/:slug"),
    ("POST", "/v1/leads"),
    ("POST", "/v1/auth/signup"),
    ("POST", "/v1/auth/login"),
    ("POST", "/v1/auth/logout"),
    ("GET", "/v1/auth/me"),
    ("GET", "/v1/auth/invite/:token"),
    ("POST", "/v1/auth/accept-invite"),
    ("POST", "/v1/orgs"),
    ("POST", "/v1/ingest"),
    ("POST", "/v1/ingest/prepare"),
    ("POST", "/v1/ingest/commit"),
    ("POST", "/v1/ingest/batch"),
    ("GET", "/v1/proofs/:id"),
    ("GET", "/v1/checkpoints"),
    ("POST", "/v1/stripe/webhook"),
    ("GET", "/v1/operator/overview"),
    ("GET", "/v1/operator/leads"),
    ("POST", "/v1/operator/orgs/:id/plan"),
    ("POST", "/v1/operator/orgs/:id/status"),
];

/// True when `text` calls `name(` as a method or function, not as part of a longer word.
fn calls(text: &str, name: &str) -> bool {
    let needle = format!("{name}(");
    text.match_indices(&needle).any(|(i, _)| {
        text[..i]
            .chars()
            .next_back()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
    })
}

/// Every `(METHOD, path)` registered in `routes/mod.rs`.
fn registered_routes() -> Vec<(String, String)> {
    let src = include_str!("../src/routes/mod.rs");
    let mut out = Vec::new();
    for piece in src.split(".route(").skip(1) {
        let path = piece.split('"').nth(1).expect("route path").to_string();
        let end = piece.find(".with_state").unwrap_or(piece.len());
        let handlers = &piece[..end];
        for (method, name) in [
            ("GET", "get"),
            ("POST", "post"),
            ("PATCH", "patch"),
            ("DELETE", "delete"),
        ] {
            if calls(handlers, name) {
                out.push((method.to_string(), path.clone()));
            }
        }
    }
    out
}

fn bare(path: &str) -> &str {
    path.split('?').next().unwrap()
}

fn fill(path: &str, org: &str) -> String {
    let mut p = path.replace(":org", org);
    for ph in [":sid", ":pid", ":uid", ":id"] {
        p = p.replace(ph, FAKE);
    }
    p
}

async fn join(e: &Env, owner: &Tenant, who: &str, role: &str) -> Tenant {
    let (st, inv) = call(
        &e.app,
        "POST",
        &format!("/v1/orgs/{}/invites", owner.org),
        Some(&owner.token),
        Some(json!({"email": format!("{who}@matrix.test"), "role": role})),
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
    let (st, acc) = call(
        &e.app,
        "POST",
        "/v1/auth/accept-invite",
        None,
        Some(json!({"token": token, "name": who, "password": "correct-horse-battery"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{acc}");
    Tenant {
        token: acc["token"].as_str().unwrap().into(),
        org: owner.org.clone(),
    }
}

#[tokio::test]
async fn every_route_enforces_the_role_it_documents() {
    let e = env().await;
    let owner = signup(&e, "owner@matrix.test", "Matrix Org").await;
    let outsider = signup(&e, "out@matrix.test", "Outside Org").await;
    sqlx::query("UPDATE organizations SET plan = 'enterprise' WHERE id = $1")
        .bind(&owner.org)
        .execute(&e.state.db)
        .await
        .unwrap();
    let admin = join(&e, &owner, "admin", "admin").await;
    let developer = join(&e, &owner, "developer", "developer").await;
    let auditor = join(&e, &owner, "auditor", "auditor").await;

    // (label, token, role level; 0 = not a member)
    let callers: Vec<(&str, Option<&str>, u8)> = vec![
        ("none", None, 0),
        ("outsider", Some(outsider.token.as_str()), 0),
        ("auditor", Some(auditor.token.as_str()), Min::Auditor as u8),
        (
            "developer",
            Some(developer.token.as_str()),
            Min::Developer as u8,
        ),
        ("admin", Some(admin.token.as_str()), Min::Admin as u8),
        ("owner", Some(owner.token.as_str()), Min::Owner as u8),
    ];

    for route in routes() {
        let path = fill(route.path, &owner.org);
        for (label, token, level) in &callers {
            let body = (route.body)(label);
            let body = if body.is_null() { None } else { Some(body) };
            let (st, v) = call(&e.app, route.method, &path, *token, body).await;
            let msg = v["error"]["message"].as_str().unwrap_or("");
            let org_hidden = st == StatusCode::NOT_FOUND && msg == "organization not found";
            let ctx = format!("{} {} as {label}: {st} {v}", route.method, route.path);
            match *label {
                "none" => assert_eq!(st, StatusCode::UNAUTHORIZED, "{ctx}"),
                "outsider" => assert!(org_hidden, "{ctx}"),
                _ if *level < route.min as u8 => assert_eq!(st, StatusCode::FORBIDDEN, "{ctx}"),
                _ => assert!(
                    st != StatusCode::UNAUTHORIZED && st != StatusCode::FORBIDDEN && !org_hidden,
                    "{ctx}"
                ),
            }
        }
    }
}

#[test]
fn every_registered_route_is_in_the_matrix_or_explicitly_exempt() {
    let matrix: Vec<(String, String)> = routes()
        .iter()
        .map(|r| (r.method.to_string(), bare(r.path).to_string()))
        .collect();
    let exempt: Vec<(String, String)> = EXEMPT
        .iter()
        .map(|(m, p)| (m.to_string(), p.to_string()))
        .collect();
    let registered = registered_routes();
    assert!(registered.len() > 40, "route parser found too few routes");
    for r in &registered {
        assert!(
            matrix.contains(r) || exempt.contains(r),
            "{} {} is registered but is neither in the access matrix nor exempt: add it to \
             tests/access_matrix.rs with the role that may call it",
            r.0,
            r.1
        );
    }
    for r in matrix.iter().chain(exempt.iter()) {
        assert!(
            registered.contains(r),
            "{} {} is listed in tests/access_matrix.rs but not registered",
            r.0,
            r.1
        );
    }
}

#[tokio::test]
async fn credentials_are_not_interchangeable() {
    let e = env().await;
    let t = signup(&e, "o@creds.test", "Creds Co").await;
    let sid = stream(&e, &t, "Creds stream").await;
    let (key, _) = api_key(&e, &t, "ingest", Some(&sid)).await;
    let rec = json!({"actor": "a", "action": "a.b", "payload_hash": "0".repeat(64)});

    // A session token is not an API key.
    let (st, _) = call(
        &e.app,
        "POST",
        "/v1/ingest",
        Some(&t.token),
        Some(rec.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, _) = call(
        &e.app,
        "GET",
        &format!("/v1/proofs/{FAKE}"),
        Some(&t.token),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    // An API key is not a session.
    let (st, _) = call(
        &e.app,
        "GET",
        &format!("/v1/orgs/{}", t.org),
        Some(&key),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, _) = call(&e.app, "GET", "/v1/checkpoints", Some(&t.token), None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, _) = call(&e.app, "GET", "/v1/auth/me", Some(&key), None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    // Session-only routes need a session.
    let (st, _) = call(&e.app, "GET", "/v1/auth/me", None, None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, _) = call(
        &e.app,
        "POST",
        "/v1/orgs",
        None,
        Some(json!({"name": "No Login"})),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    // Operator routes need the operator key: not nothing, not a session, not an API key.
    for token in [None, Some(t.token.as_str()), Some(key.as_str())] {
        for path in ["/v1/operator/overview", "/v1/operator/leads"] {
            let (st, _) = call(&e.app, "GET", path, token, None).await;
            assert_eq!(st, StatusCode::UNAUTHORIZED, "{path}");
        }
    }
    let (st, _) = call(
        &e.app,
        "GET",
        "/v1/operator/overview",
        Some("operator-key-for-tests-0123456789"),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
}
