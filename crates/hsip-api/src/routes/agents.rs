use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::Row;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::{auth::TenantId, errors::ApiResult, state::AppState};

/// GET /v1/agent/capabilities
/// Machine-readable description of everything an AI agent can do via HSIP.
/// Inject this into an AI's system prompt so it knows the available tools.
pub async fn capabilities(_tenant: TenantId) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "HSIP",
        "description": "Personal privacy and identity layer. Lets you send tamper-proof signed messages, record consent decisions, and manage your digital identity — all stored locally.",
        "auth": {
            "type": "Bearer",
            "header": "Authorization",
            "format": "Bearer <your_ai_agent_key>"
        },
        "base_url": "http://127.0.0.1:7777",
        "actions": [
            {
                "id": "send_message",
                "summary": "Send a signed message",
                "description": "Signs the message with the user's private key and stores it with a cryptographic timestamp. Use this when the user asks you to send, record, or timestamp a message. The resulting signature is legally verifiable proof of exactly what was said and when.",
                "method": "POST",
                "path": "/v1/messages/sign",
                "body": {
                    "content": "(required) The message text to sign",
                    "peer_verify_key": "(optional) Recipient's HSIP public key for directed messages"
                },
                "example_body": { "content": "I agree to the terms discussed on this date." },
                "returns": { "id": "string", "content": "string", "signature": "base64", "timestamp": "unix_ms" }
            },
            {
                "id": "verify_message",
                "summary": "Verify a received message",
                "description": "Checks that a message's signature is valid — proving it came from the claimed sender and was not altered.",
                "method": "POST",
                "path": "/v1/messages/verify",
                "body": {
                    "content": "The message text",
                    "signature": "base64 signature string",
                    "peer_verify_key": "Sender's base64 public key"
                },
                "returns": { "verified": "bool", "timestamp": "unix_ms" }
            },
            {
                "id": "list_messages",
                "summary": "Get message history",
                "description": "Returns the last 100 messages (sent and received), most recent first.",
                "method": "GET",
                "path": "/v1/messages",
                "returns": "Array of message records with id, direction, content, signature, timestamp, verified"
            },
            {
                "id": "get_identity",
                "summary": "Get the user's public identity key",
                "description": "Returns the user's public Ed25519 key. Share this with others so they can verify messages you send.",
                "method": "GET",
                "path": "/v1/identity",
                "returns": { "verify_key_b64": "base64 public key" }
            },
            {
                "id": "log_consent",
                "summary": "Record a consent decision",
                "description": "Stores a timestamped record that the user grants or revokes consent for a given party.",
                "method": "POST",
                "path": "/v1/consent/grant",
                "body": {
                    "peer_verify_key": "The other party's HSIP public key",
                    "scope": "What is being consented to (e.g. 'data_sharing', 'contact')",
                    "note": "(optional) Human-readable description"
                },
                "returns": { "id": "string", "granted_at": "unix_ms" }
            }
        ],
        "voice_command_examples": [
            "Hey Siri, send HSIP message: I confirm we spoke today at 3pm",
            "Send HSIP message saying I agree to proceed",
            "Sign a message that says the package was delivered"
        ]
    }))
}

#[derive(Serialize)]
pub struct AgentStats {
    pub key_id: String,
    pub name: String,
    pub active: bool,
    pub request_count: u64,
    pub anomaly_count: u64,
    pub window_start_ms: i64,
}

pub async fn list(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<Vec<AgentStats>>> {
    let rows = sqlx::query(
        "SELECT id, name, active FROM api_keys
         WHERE tenant_id = ? AND agent_type = 'ai_agent'
         ORDER BY created_at DESC",
    )
    .bind(&tenant.0)
    .fetch_all(&state.db)
    .await?;

    let agent_keys: Vec<(String, String, bool)> = rows
        .iter()
        .map(|r| -> Result<_, sqlx::Error> {
            Ok((
                r.try_get::<String, _>(0)?,
                r.try_get::<String, _>(1)?,
                r.try_get::<i64, _>(2)? != 0,
            ))
        })
        .collect::<Result<_, _>>()?;

    let stats: Vec<AgentStats> = agent_keys
        .into_iter()
        .map(|(key_id, name, active)| {
            let (request_count, anomaly_count, window_start_ms) =
                if let Some(rec) = state.agent_tracker.get(&key_id) {
                    (
                        rec.request_count.load(Ordering::Relaxed),
                        rec.anomaly_count.load(Ordering::Relaxed),
                        rec.window_start_ms.load(Ordering::Relaxed),
                    )
                } else {
                    (0, 0, 0)
                };
            AgentStats {
                key_id,
                name,
                active,
                request_count,
                anomaly_count,
                window_start_ms,
            }
        })
        .collect();

    Ok(Json(stats))
}

// ── Agent discovery ────────────────────────────────────────────────────────────

/// Well-known localhost ports where AI agents / MCP servers commonly listen.
static PROBE_PORTS: &[(u16, &str, &str)] = &[
    (11434, "ollama", "Ollama local LLM server"),
    (5173, "vite", "Vite dev server (MCP plugin likely)"),
    (8787, "wrangler", "Cloudflare Workers (wrangler dev)"),
    (8080, "http-agent", "Generic HTTP agent / proxy"),
    (8000, "uvicorn", "FastAPI / Django / Python agent"),
    (5000, "flask", "Flask API / Python agent"),
    (4000, "agent", "Generic agent port"),
    (3001, "dashboard", "Secondary dev server"),
    (3000, "dev-api", "Generic dev API / Next.js"),
    (9000, "agent", "Generic agent port"),
    (1234, "lmstudio", "LM Studio local LLM API"),
    (8888, "jupyter", "Jupyter notebook server"),
];

const PROBE_TIMEOUT_MS: u64 = 150;

#[derive(Serialize)]
pub struct DiscoveredAgent {
    pub port: u16,
    pub url: String,
    pub hint: String,
    pub description: String,
    pub reachable: bool,
    pub already_registered: bool,
    pub suggested_name: String,
}

/// GET /v1/agents/discover
/// Probes well-known localhost ports for MCP / AI agent servers.
/// Returns all ports that accepted a TCP connection, annotated with hints and
/// whether an agent with that name is already registered.
pub async fn discover(
    State(state): State<AppState>,
    tenant: TenantId,
) -> ApiResult<Json<Vec<DiscoveredAgent>>> {
    // Fetch registered agent names once so we can flag duplicates.
    let rows = sqlx::query(
        "SELECT name FROM api_keys WHERE tenant_id = ? AND agent_type = 'ai_agent' AND active = 1",
    )
    .bind(&tenant.0)
    .fetch_all(&state.db)
    .await?;

    let registered_names: Vec<String> = rows
        .iter()
        .filter_map(|r| r.try_get::<String, _>(0).ok())
        .map(|s| s.to_lowercase())
        .collect();

    // Probe each port concurrently.
    let dur = Duration::from_millis(PROBE_TIMEOUT_MS);
    let mut handles = Vec::with_capacity(PROBE_PORTS.len());
    for &(port, hint, description) in PROBE_PORTS {
        handles.push(tokio::spawn(async move {
            let addr = format!("127.0.0.1:{port}");
            let reachable = timeout(dur, TcpStream::connect(&addr)).await.is_ok();
            (port, hint, description, reachable)
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        if let Ok((port, hint, description, reachable)) = handle.await {
            if !reachable {
                continue;
            }
            let already_registered = registered_names
                .iter()
                .any(|n| n == hint || n.starts_with(hint));
            results.push(DiscoveredAgent {
                port,
                url: format!("http://127.0.0.1:{port}"),
                hint: hint.to_string(),
                description: description.to_string(),
                reachable: true,
                already_registered,
                suggested_name: hint.to_string(),
            });
        }
    }

    // Sort by port for deterministic output.
    results.sort_by_key(|r| r.port);
    Ok(Json(results))
}
