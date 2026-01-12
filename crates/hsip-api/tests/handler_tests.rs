//! Integration tests for API handlers

use hsip_api::{
    handlers::ApiState,
    models::*,
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    Router,
};
use tower::ServiceExt;
use std::sync::Arc;
use serde_json;

#[tokio::test]
async fn test_health_check() {
    let state = Arc::new(ApiState {
        version: "2.0.0-test".to_string(),
    });

    let app = Router::new()
        .route("/health", axum::routing::get(hsip_api::handlers::health_check))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_send_message() {
    let state = Arc::new(ApiState {
        version: "2.0.0-test".to_string(),
    });

    let app = Router::new()
        .route("/send", axum::routing::post(hsip_api::handlers::send_message))
        .with_state(state);

    let request_body = serde_json::json!({
        "recipient": "user@example.com",
        "message_type": "transaction_alert",
        "content": {"amount": "$50.00", "merchant": "Coffee Shop"},
        "expires_in_seconds": 3600
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/send")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: SendMessageResponse = serde_json::from_slice(&body).unwrap();

    assert!(response_json.message_id.starts_with("msg_"));
    assert_eq!(response_json.verification_status, "verified");
}

#[tokio::test]
async fn test_grant_consent() {
    let state = Arc::new(ApiState {
        version: "2.0.0-test".to_string(),
    });

    let app = Router::new()
        .route("/consent/grant", axum::routing::post(hsip_api::handlers::grant_consent))
        .with_state(state);

    let request_body = serde_json::json!({
        "user_id": "user123",
        "sender_domain": "chase.com",
        "allowed_message_types": ["transaction_alert", "security_alert"],
        "duration_days": 30,
        "max_messages_per_day": 10
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/consent/grant")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: GrantConsentResponse = serde_json::from_slice(&body).unwrap();

    assert!(response_json.policy_id.starts_with("policy_user123_"));
}

#[tokio::test]
async fn test_revoke_consent() {
    let state = Arc::new(ApiState {
        version: "2.0.0-test".to_string(),
    });

    let app = Router::new()
        .route("/consent/revoke", axum::routing::post(hsip_api::handlers::revoke_consent))
        .with_state(state);

    let request_body = serde_json::json!({
        "user_id": "user123",
        "sender_domain": "chase.com"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/consent/revoke")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_export_audit() {
    let state = Arc::new(ApiState {
        version: "2.0.0-test".to_string(),
    });

    let app = Router::new()
        .route("/audit/export", axum::routing::post(hsip_api::handlers::export_audit))
        .with_state(state);

    let request_body = serde_json::json!({
        "start_date": "2024-01-01T00:00:00Z",
        "end_date": "2024-12-31T23:59:59Z",
        "format": "json"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/audit/export")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(response_json.get("export_id").is_some());
    assert_eq!(response_json.get("format").unwrap(), "json");
}

#[tokio::test]
async fn test_concurrent_requests() {
    // Test that the API can handle concurrent requests
    let state = Arc::new(ApiState {
        version: "2.0.0-test".to_string(),
    });

    let tasks: Vec<_> = (0..10)
        .map(|i| {
            let state = state.clone();
            tokio::spawn(async move {
                let app = Router::new()
                    .route("/send", axum::routing::post(hsip_api::handlers::send_message))
                    .with_state(state);

                let request_body = serde_json::json!({
                    "recipient": format!("user{}@example.com", i),
                    "message_type": "test",
                    "content": {"test": true}
                });

                let response = app
                    .oneshot(
                        Request::builder()
                            .uri("/send")
                            .method("POST")
                            .header("content-type", "application/json")
                            .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                            .unwrap(),
                    )
                    .await
                    .unwrap();

                response.status()
            })
        })
        .collect();

    // Wait for all requests to complete
    let results = futures::future::join_all(tasks).await;

    // All should succeed
    for result in results {
        assert_eq!(result.unwrap(), StatusCode::OK);
    }
}
