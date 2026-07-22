//! HSIP MCP Server
//!
//! Speaks the Model Context Protocol over stdio so any MCP-capable AI client
//! (Claude Desktop, Cursor, Continue, etc.) can give its agents:
//!   • a cryptographic identity (Ed25519)
//!   • consent checks before sensitive actions
//!   • a tamper-proof signed audit trail
//!
//! # Setup (Claude Desktop)
//!
//! Add to ~/Library/Application Support/Claude/claude_desktop_config.json:
//!
//! ```json
//! {
//!   "mcpServers": {
//!     "hsip": {
//!       "command": "/path/to/hsip-mcp",
//!       "env": {
//!         "HSIP_API_KEY": "hsip_...",
//!         "HSIP_API_URL": "http://127.0.0.1:7474"
//!       }
//!     }
//!   }
//! }
//! ```
//!
//! The API key should be an ai_agent key created with:
//!   hsip agent register claude
//!
//! Key / URL resolution (highest priority first):
//!   HSIP_API_KEY env  >  ~/.hsip/admin.key
//!   HSIP_API_URL env  >  http://127.0.0.1:7474

use std::io::{BufRead, Write};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const SERVER_NAME: &str = "hsip";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const PROTOCOL_VERSION: &str = "2024-11-05";
const DEFAULT_API_URL: &str = "http://127.0.0.1:7474";

// ── JSON-RPC types ────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct RpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize)]
struct RpcResponse {
    jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Serialize)]
struct RpcError {
    code: i32,
    message: String,
}

impl RpcResponse {
    fn ok(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    fn err(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(RpcError {
                code,
                message: message.into(),
            }),
        }
    }
}

// ── HSIP API client ───────────────────────────────────────────────────────────

struct ApiClient {
    base: String,
    key: String,
    http: reqwest::blocking::Client,
}

impl ApiClient {
    fn new() -> Result<Self> {
        let base = std::env::var("HSIP_API_URL")
            .unwrap_or_else(|_| DEFAULT_API_URL.to_string())
            .trim_end_matches('/')
            .to_string();

        let key = std::env::var("HSIP_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())
            .map(Ok)
            .unwrap_or_else(load_admin_key)?;

        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self { base, key, http })
    }

    fn post<B: serde::Serialize>(&self, path: &str, body: &B) -> Result<Value> {
        let url = format!("{}{}", self.base, path);
        let res = self
            .http
            .post(&url)
            .bearer_auth(&self.key)
            .json(body)
            .send()
            .with_context(|| format!("POST {url} — is HSIP running at {}?", self.base))?;

        let status = res.status();
        let json: Value = res.json().unwrap_or(Value::Null);
        if !status.is_success() {
            bail!("HSIP API {status}: {json}");
        }
        Ok(json)
    }

    fn get(&self, path: &str) -> Result<Value> {
        let url = format!("{}{}", self.base, path);
        let res = self
            .http
            .get(&url)
            .bearer_auth(&self.key)
            .send()
            .with_context(|| format!("GET {url} — is HSIP running at {}?", self.base))?;

        let status = res.status();
        let json: Value = res.json().unwrap_or(Value::Null);
        if !status.is_success() {
            bail!("HSIP API {status}: {json}");
        }
        Ok(json)
    }
}

fn load_admin_key() -> Result<String> {
    let home = dirs::home_dir().context("cannot resolve home directory")?;
    let path = home.join(".hsip").join("admin.key");
    if !path.exists() {
        bail!(
            "No HSIP API key found. Set HSIP_API_KEY env var, or start HSIP once to generate ~/.hsip/admin.key.\n\
             For agents, create a dedicated key with: hsip agent register <name>"
        );
    }
    let key = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?
        .trim()
        .to_string();
    if key.is_empty() {
        bail!("admin.key is empty — start HSIP once to bootstrap the key");
    }
    Ok(key)
}

// ── Tool definitions ──────────────────────────────────────────────────────────

fn tool_list() -> Value {
    serde_json::json!([
        {
            "name": "sign_message",
            "description": "Sign a message with your Ed25519 identity key. Returns a cryptographic signature and timestamp proving exactly what was said and when. Use this to create tamper-proof records of decisions, authorizations, or statements.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The message text to sign"
                    },
                    "peer_verify_key": {
                        "type": "string",
                        "description": "Optional: recipient's HSIP public key for directed messages"
                    }
                },
                "required": ["content"]
            }
        },
        {
            "name": "verify_message",
            "description": "Verify that a message signature is valid — proving it came from the claimed sender unchanged.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "content": { "type": "string", "description": "The original message text" },
                    "signature": { "type": "string", "description": "Base64-encoded signature" },
                    "peer_verify_key": { "type": "string", "description": "Sender's base64 public key" }
                },
                "required": ["content", "signature", "peer_verify_key"]
            }
        },
        {
            "name": "get_identity",
            "description": "Get the public Ed25519 identity key for this agent. Share this with others so they can verify messages from you.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "grant_consent",
            "description": "Record that the user grants consent to a peer for a specific scope. Always call this before performing actions on behalf of another party.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "peer_verify_key": { "type": "string", "description": "The peer's HSIP public key" },
                    "scope": { "type": "string", "description": "What is being consented to (e.g. 'data_access', 'email', 'calendar')" },
                    "expires_in_seconds": { "type": "integer", "description": "Optional: consent expires after this many seconds" }
                },
                "required": ["peer_verify_key", "scope"]
            }
        },
        {
            "name": "check_consent",
            "description": "Check whether active consent exists for a peer. Call this before accessing another party's data or acting on their behalf.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "peer_verify_key": { "type": "string", "description": "The peer's HSIP public key to check" }
                },
                "required": ["peer_verify_key"]
            }
        },
        {
            "name": "revoke_consent",
            "description": "Immediately revoke consent for a peer. All future consent checks for this peer will fail.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "peer_verify_key": { "type": "string", "description": "The peer's HSIP public key" }
                },
                "required": ["peer_verify_key"]
            }
        },
        {
            "name": "log_action",
            "description": "Write a signed, timestamped record of an action to the tamper-proof audit trail. Use this to record any significant action you take on behalf of the user.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "description": "Short action identifier, e.g. 'email.sent', 'file.deleted', 'payment.initiated'" },
                    "detail": { "type": "string", "description": "Human-readable description of what was done" }
                },
                "required": ["action", "detail"]
            }
        },
        {
            "name": "get_recent_actions",
            "description": "Retrieve recent entries from the audit trail. Use this to show the user what has been done on their behalf.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Number of entries to return (max 100, default 20)" }
                }
            }
        }
    ])
}

// ── Tool handlers ─────────────────────────────────────────────────────────────

fn call_tool(name: &str, args: &Value, api: &ApiClient) -> Result<String> {
    match name {
        "sign_message" => {
            let content = req_str(args, "content")?;
            let mut body = serde_json::json!({ "content": content });
            if let Some(peer) = args.get("peer_verify_key").and_then(Value::as_str) {
                body["peer_verify_key"] = Value::String(peer.to_string());
            }
            let resp = api.post("/v1/messages/sign", &body)?;
            Ok(format!(
                "Message signed successfully.\n\
                 ID:        {}\n\
                 Signature: {}\n\
                 Timestamp: {} ms\n\n\
                 This signature cryptographically proves you wrote exactly these words at this time.",
                resp["id"].as_str().unwrap_or("?"),
                resp["signature"].as_str().unwrap_or("?"),
                resp["timestamp"].as_i64().unwrap_or(0),
            ))
        }

        "verify_message" => {
            let body = serde_json::json!({
                "content":        req_str(args, "content")?,
                "signature":      req_str(args, "signature")?,
                "peer_verify_key": req_str(args, "peer_verify_key")?,
            });
            let resp = api.post("/v1/messages/verify", &body)?;
            let verified = resp["verified"].as_bool().unwrap_or(false);
            if verified {
                Ok(format!(
                    "✓ Signature VALID. This message was genuinely sent by the owner of key {} and has not been altered.",
                    body["peer_verify_key"].as_str().unwrap_or("?")
                ))
            } else {
                Ok(
                    "✗ Signature INVALID. The message may have been altered or the key is wrong."
                        .to_string(),
                )
            }
        }

        "get_identity" => {
            // Auto-create identity if it doesn't exist yet
            let resp = api.post("/v1/identity", &serde_json::json!({}))?;
            Ok(format!(
                "Public key: {}\n\
                 Identity created: {} ms\n\n\
                 Share this public key with others so they can verify messages from you.",
                resp["verify_key"].as_str().unwrap_or("?"),
                resp["created_at"].as_i64().unwrap_or(0),
            ))
        }

        "grant_consent" => {
            let peer = req_str(args, "peer_verify_key")?;
            let scope = req_str(args, "scope")?;
            let mut body = serde_json::json!({
                "peer_verify_key": peer,
                "scope": scope,
            });
            if let Some(secs) = args.get("expires_in_seconds").and_then(Value::as_i64) {
                body["expires_in_seconds"] = Value::Number(secs.into());
            }
            let resp = api.post("/v1/consent/grant", &body)?;
            let expiry = if resp["expires_ms"].is_null() {
                "never (permanent)".to_string()
            } else {
                format!("{} ms from epoch", resp["expires_ms"].as_i64().unwrap_or(0))
            };
            Ok(format!(
                "✓ Consent granted.\n\
                 Peer:   {peer}\n\
                 Scope:  {scope}\n\
                 Expires: {expiry}\n\n\
                 This consent has been recorded in the tamper-proof audit trail.",
            ))
        }

        "check_consent" => {
            let peer = req_str(args, "peer_verify_key")?;
            let path = format!("/v1/consent/{}", urlenc(peer));
            match api.get(&path) {
                Ok(resp) => {
                    let status = resp["status"].as_str().unwrap_or("unknown");
                    if status == "granted" {
                        Ok(format!(
                            "✓ Consent ACTIVE for peer {}.\nScope: {}",
                            peer,
                            resp["scope"].as_str().unwrap_or("any")
                        ))
                    } else {
                        Ok(format!("✗ Consent NOT active for peer {} (status: {}).\nYou should not act on this peer's behalf without explicit consent.", peer, status))
                    }
                }
                Err(_) => Ok(format!(
                    "✗ No consent record found for peer {}.\n\
                     Call grant_consent first if the user has authorized this.",
                    peer
                )),
            }
        }

        "revoke_consent" => {
            let peer = req_str(args, "peer_verify_key")?;
            api.post(
                "/v1/consent/revoke",
                &serde_json::json!({ "peer_verify_key": peer }),
            )?;
            Ok(format!(
                "✓ Consent revoked for peer {peer}.\n\
                 All future consent checks for this peer will fail immediately."
            ))
        }

        "log_action" => {
            let action = req_str(args, "action")?;
            let detail = req_str(args, "detail")?;
            // Log via signed message so it's in the tamper-proof trail
            let content = format!("[ACTION:{action}] {detail}");
            let resp = api.post(
                "/v1/messages/sign",
                &serde_json::json!({ "content": content }),
            )?;
            Ok(format!(
                "✓ Action logged to tamper-proof audit trail.\n\
                 Action:    {action}\n\
                 Detail:    {detail}\n\
                 Signature: {}\n\
                 Timestamp: {} ms\n\n\
                 This record cannot be altered or deleted.",
                resp["signature"].as_str().unwrap_or("?"),
                resp["timestamp"].as_i64().unwrap_or(0),
            ))
        }

        "get_recent_actions" => {
            let limit = args
                .get("limit")
                .and_then(Value::as_i64)
                .unwrap_or(20)
                .min(100);
            let path = format!("/v1/audit?limit={limit}");
            let entries = api.get(&path)?;
            let arr = entries.as_array().cloned().unwrap_or_default();
            if arr.is_empty() {
                return Ok("No audit entries yet.".to_string());
            }
            let mut out = format!("Recent {} action(s):\n\n", arr.len());
            for e in &arr {
                let ts = e["timestamp"].as_i64().unwrap_or(0);
                let action = e["action"].as_str().unwrap_or("?");
                let detail = e["details"].as_str().unwrap_or("");
                let detail_str = if detail.is_empty() {
                    String::new()
                } else {
                    format!(" — {detail}")
                };
                out.push_str(&format!("  [{ts}ms] {action}{detail_str}\n"));
            }
            Ok(out)
        }

        other => bail!("Unknown tool: {other}"),
    }
}

// ── MCP protocol loop ─────────────────────────────────────────────────────────

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    // Initialise the API client once; errors are surfaced per-tool-call if needed.
    let api = ApiClient::new();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if l.trim().is_empty() => continue,
            Ok(l) => l,
            Err(_) => break,
        };

        let req: RpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                send(
                    &mut out,
                    &RpcResponse::err(None, -32700, format!("Parse error: {e}")),
                );
                continue;
            }
        };

        // Notifications have no id and require no response.
        if req.id.is_none() {
            continue;
        }

        let id = req.id.clone();
        let response = handle(&req, &api);
        send(
            &mut out,
            &response.unwrap_or_else(|e| RpcResponse::err(id, -32603, format!("{e:#}"))),
        );
    }
}

fn handle(req: &RpcRequest, api: &Result<ApiClient>) -> Result<RpcResponse> {
    let id = req.id.clone();

    match req.method.as_str() {
        "initialize" => Ok(RpcResponse::ok(
            id,
            serde_json::json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION },
                "instructions": "HSIP gives you a cryptographic identity and tamper-proof audit trail. \
                                 Call sign_message to create verifiable records. \
                                 Call check_consent before accessing another party's data. \
                                 Call log_action to record significant decisions. \
                                 Your HSIP API key determines your identity — treat it like a private key."
            }),
        )),

        "ping" => Ok(RpcResponse::ok(id, serde_json::json!({}))),

        "tools/list" => Ok(RpcResponse::ok(
            id,
            serde_json::json!({
                "tools": tool_list()
            }),
        )),

        "tools/call" => {
            let params = req.params.as_ref().context("missing params")?;
            let name = params["name"].as_str().context("missing tool name")?;
            let args = params.get("arguments").unwrap_or(&Value::Null);

            let api = match api {
                Ok(a) => a,
                Err(e) => {
                    return Ok(RpcResponse::ok(id, tool_error(format!(
                        "HSIP not configured: {e:#}\n\n\
                         Set HSIP_API_KEY env var or ensure HSIP is running and ~/.hsip/admin.key exists."
                    ))));
                }
            };

            match call_tool(name, args, api) {
                Ok(text) => Ok(RpcResponse::ok(id, tool_result(text))),
                Err(e) => Ok(RpcResponse::ok(id, tool_error(format!("{e:#}")))),
            }
        }

        other => Ok(RpcResponse::err(
            id,
            -32601,
            format!("Method not found: {other}"),
        )),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn send(out: &mut impl Write, resp: &RpcResponse) {
    if let Ok(json) = serde_json::to_string(resp) {
        let _ = writeln!(out, "{json}");
        let _ = out.flush();
    }
}

fn req_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .with_context(|| format!("missing required argument: {key}"))
}

fn tool_result(text: String) -> Value {
    serde_json::json!({
        "content": [{ "type": "text", "text": text }]
    })
}

fn tool_error(text: String) -> Value {
    serde_json::json!({
        "content": [{ "type": "text", "text": text }],
        "isError": true
    })
}

fn urlenc(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                vec![c]
            }
            c => format!("%{:02X}", c as u32).chars().collect(),
        })
        .collect()
}
