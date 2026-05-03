//! `hsip agent` subcommands — register, list, revoke AI agents via the HSIP API.
//!
//! Key resolution order (highest priority first):
//!   1. --key flag
//!   2. HSIP_API_KEY env var
//!   3. ~/.hsip/admin.key file
//!
//! URL resolution order:
//!   1. --api-url flag
//!   2. HSIP_API_URL env var
//!   3. http://127.0.0.1:7474 (desktop default)

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use serde::Deserialize;
use std::time::Duration;

const DEFAULT_API_URL: &str = "http://127.0.0.1:7474";

// ── Clap types ────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum AgentCmd {
    /// Register a new AI agent and print its API key (shown once)
    Register {
        /// Name for this agent (e.g. "claude", "cursor", "my-script")
        name: String,
        /// Expire after this many days (omit for no expiry)
        #[arg(long)]
        expires_days: Option<i64>,
        #[arg(long, env = "HSIP_API_URL")]
        api_url: Option<String>,
        #[arg(long, env = "HSIP_API_KEY")]
        key: Option<String>,
    },

    /// List registered AI agents with live velocity stats
    List {
        #[arg(long, env = "HSIP_API_URL")]
        api_url: Option<String>,
        #[arg(long, env = "HSIP_API_KEY")]
        key: Option<String>,
    },

    /// Revoke an AI agent's access immediately (by name or key ID)
    Revoke {
        /// Agent name or key ID to revoke
        target: String,
        #[arg(long, env = "HSIP_API_URL")]
        api_url: Option<String>,
        #[arg(long, env = "HSIP_API_KEY")]
        key: Option<String>,
    },

    /// Scan localhost for running AI agents / MCP servers and suggest registering them
    Discover {
        #[arg(long, env = "HSIP_API_URL")]
        api_url: Option<String>,
        #[arg(long, env = "HSIP_API_KEY")]
        key: Option<String>,
    },
}

// ── API response types ────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct CreateKeyResponse {
    id: String,
    key: String,
    name: String,
    agent_type: String,
    created_at: i64,
    expires_at: Option<i64>,
}

#[derive(Deserialize, Debug)]
struct AgentStats {
    key_id: String,
    name: String,
    active: bool,
    request_count: u64,
    anomaly_count: u64,
}

#[derive(Deserialize, Debug)]
struct DiscoveredAgent {
    port: u16,
    url: String,
    hint: String,
    description: String,
    already_registered: bool,
    suggested_name: String,
}

#[derive(Deserialize, Debug)]
struct IdentityResponse {
    verify_key: String,
    created_at: i64,
}

#[derive(Deserialize, Debug)]
struct AuditEntry {
    action: String,
    details: Option<String>,
    timestamp: i64,
}

// ── HTTP client helper ────────────────────────────────────────────────────────

struct ApiClient {
    base: String,
    key: String,
    http: reqwest::blocking::Client,
}

impl ApiClient {
    fn new(api_url: Option<String>, key_flag: Option<String>) -> Result<Self> {
        let base = api_url
            .unwrap_or_else(|| DEFAULT_API_URL.to_string())
            .trim_end_matches('/')
            .to_string();

        let key = match key_flag {
            Some(k) => k,
            None => load_admin_key()?,
        };

        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self { base, key, http })
    }

    fn post<B: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<R> {
        let url = format!("{}{}", self.base, path);
        let res = self
            .http
            .post(&url)
            .bearer_auth(&self.key)
            .json(body)
            .send()
            .with_context(|| format!("POST {url} failed — is HSIP running?"))?;

        if !res.status().is_success() {
            let status = res.status();
            let body: serde_json::Value = res.json().unwrap_or_default();
            bail!("API error {status}: {body}");
        }
        res.json().context("failed to parse response")
    }

    fn get<R: serde::de::DeserializeOwned>(&self, path: &str) -> Result<R> {
        let url = format!("{}{}", self.base, path);
        let res = self
            .http
            .get(&url)
            .bearer_auth(&self.key)
            .send()
            .with_context(|| format!("GET {url} failed — is HSIP running?"))?;

        if !res.status().is_success() {
            let status = res.status();
            let body: serde_json::Value = res.json().unwrap_or_default();
            bail!("API error {status}: {body}");
        }
        res.json().context("failed to parse response")
    }

    fn delete(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base, path);
        let res = self
            .http
            .delete(&url)
            .bearer_auth(&self.key)
            .send()
            .with_context(|| format!("DELETE {url} failed — is HSIP running?"))?;

        if !res.status().is_success() {
            let status = res.status();
            let body: serde_json::Value = res.json().unwrap_or_default();
            bail!("API error {status}: {body}");
        }
        res.json().context("failed to parse response")
    }
}

// ── Command handlers ──────────────────────────────────────────────────────────

pub fn run(cmd: AgentCmd) -> Result<()> {
    match cmd {
        AgentCmd::Register { name, expires_days, api_url, key } => {
            register(name, expires_days, api_url, key)
        }
        AgentCmd::List { api_url, key } => list(api_url, key),
        AgentCmd::Revoke { target, api_url, key } => revoke(target, api_url, key),
        AgentCmd::Discover { api_url, key } => discover(api_url, key),
    }
}

fn register(name: String, expires_days: Option<i64>, api_url: Option<String>, key: Option<String>) -> Result<()> {
    let client = ApiClient::new(api_url, key)?;

    let body = serde_json::json!({
        "name": name,
        "agent_type": "ai_agent",
        "expires_in_days": expires_days,
    });

    let resp: CreateKeyResponse = client.post("/v1/keys", &body)?;

    let expiry_str = resp.expires_at
        .map(|ms| format_timestamp(ms))
        .unwrap_or_else(|| "never".to_string());

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              AI Agent Registered                            ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Name:       {:<48}  ║", resp.name);
    println!("║  Type:       {:<48}  ║", resp.agent_type);
    println!("║  ID:         {:<48}  ║", &resp.id[..resp.id.len().min(48)]);
    println!("║  Expires:    {:<48}  ║", expiry_str);
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  API Key (save this — shown only once):                     ║");
    println!("║                                                              ║");
    println!("║  {:<60}  ║", resp.key);
    println!("║                                                              ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Add to your agent's environment:                           ║");
    println!("║    export HSIP_AGENT_KEY=\"{}\"", &resp.key[..resp.key.len().min(42)]);
    println!("║                                                              ║");
    println!("║  To revoke instantly:                                       ║");
    println!("║    hsip agent revoke \"{}\"", name);
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("Created at: {}", format_timestamp(resp.created_at));

    Ok(())
}

fn list(api_url: Option<String>, key: Option<String>) -> Result<()> {
    let client = ApiClient::new(api_url, key)?;
    let agents: Vec<AgentStats> = client.get("/v1/agents")?;

    if agents.is_empty() {
        println!("No AI agents registered.");
        println!("Register one with:  hsip agent register <name>");
        return Ok(());
    }

    println!();
    println!("{:<36}  {:<20}  {:<6}  {:<8}  {:<8}",
        "Key ID", "Name", "Active", "Req/min", "Anomaly");
    println!("{}", "─".repeat(84));

    for a in &agents {
        let status = if a.active { "✓" } else { "✗ revoked" };
        let anomaly = if a.anomaly_count > 0 {
            format!("⚠ {}", a.anomaly_count)
        } else {
            "0".to_string()
        };
        println!(
            "{:<36}  {:<20}  {:<6}  {:<8}  {:<8}",
            truncate(&a.key_id, 36),
            truncate(&a.name, 20),
            status,
            a.request_count,
            anomaly,
        );
    }

    println!();
    println!("{} agent(s) total.", agents.len());
    println!("To revoke:  hsip agent revoke <name>");

    Ok(())
}

fn revoke(target: String, api_url: Option<String>, key: Option<String>) -> Result<()> {
    let client = ApiClient::new(api_url, key)?;

    // Resolve the target to a key ID — target may be a name or already an ID
    let agents: Vec<AgentStats> = client.get("/v1/agents")?;

    let matched: Vec<&AgentStats> = agents
        .iter()
        .filter(|a| a.name == target || a.key_id == target)
        .collect();

    if matched.is_empty() {
        bail!("No agent found with name or ID \"{target}\". Run `hsip agent list` to see registered agents.");
    }
    if matched.len() > 1 {
        bail!(
            "Multiple agents match \"{target}\". Use the key ID instead:\n{}",
            matched.iter().map(|a| format!("  {} ({})", a.key_id, a.name)).collect::<Vec<_>>().join("\n")
        );
    }

    let agent = matched[0];
    if !agent.active {
        println!("Agent \"{}\" ({}) is already revoked.", agent.name, agent.key_id);
        return Ok(());
    }

    let path = format!("/v1/keys/{}", agent.key_id);
    client.delete(&path)?;

    println!();
    println!("✓ Agent \"{}\" revoked.", agent.name);
    println!("  Key ID:  {}", agent.key_id);
    println!("  All in-flight requests from this agent are now blocked.");
    println!("  The action has been recorded in the audit log.");

    Ok(())
}

// ── `hsip agent discover` ─────────────────────────────────────────────────────

fn discover(api_url: Option<String>, key: Option<String>) -> Result<()> {
    let client = ApiClient::new(api_url, key)?;
    let candidates: Vec<DiscoveredAgent> = client.get("/v1/agents/discover")?;

    if candidates.is_empty() {
        println!();
        println!("No AI agents or MCP servers detected on localhost.");
        println!();
        println!("Start an agent (e.g. Ollama, Claude Desktop, LM Studio) and run this again.");
        return Ok(());
    }

    println!();
    println!("Discovered agents on localhost:");
    println!("{}", "─".repeat(72));
    println!("{:<6}  {:<18}  {:<30}  {}", "Port", "Name hint", "Description", "Status");
    println!("{}", "─".repeat(72));

    for c in &candidates {
        let status = if c.already_registered {
            "✓ registered".to_string()
        } else {
            "not registered".to_string()
        };
        println!(
            "{:<6}  {:<18}  {:<30}  {}",
            c.port,
            truncate(&c.hint, 18),
            truncate(&c.description, 30),
            status,
        );
    }

    println!("{}", "─".repeat(72));
    println!();

    let unregistered: Vec<&DiscoveredAgent> =
        candidates.iter().filter(|c| !c.already_registered).collect();

    if unregistered.is_empty() {
        println!("All discovered agents are already registered with HSIP.");
    } else {
        println!(
            "{} agent(s) found, {} not yet registered.",
            candidates.len(),
            unregistered.len()
        );
        println!();
        println!("To register them, run:");
        for c in &unregistered {
            println!("  hsip agent register \"{}\"   # {}", c.suggested_name, c.url);
        }
    }

    println!();
    Ok(())
}

// ── `hsip status` ─────────────────────────────────────────────────────────────

pub fn status(api_url: Option<String>, key: Option<String>) -> Result<()> {
    let client = ApiClient::new(api_url, key)?;

    // Identity
    let identity: Result<IdentityResponse> = client.get("/v1/identity");
    // Agents
    let agents: Result<Vec<AgentStats>> = client.get("/v1/agents");
    // Recent audit
    let audit: Result<Vec<AuditEntry>> = client.get("/v1/audit?limit=5");

    println!();
    println!("HSIP Status");
    println!("{}", "═".repeat(60));

    // Identity section
    match identity {
        Ok(id) => {
            println!("  Identity:   ✓ active");
            println!("  Public key: {}…", &id.verify_key[..id.verify_key.len().min(24)]);
            println!("  Created:    {}", format_timestamp(id.created_at));
        }
        Err(_) => {
            println!("  Identity:   ✗ not created yet");
            println!("              Run: hsip agent register <name> (identity auto-created)");
        }
    }

    println!();

    // Agents section
    match agents {
        Ok(ref list) => {
            let active: Vec<_> = list.iter().filter(|a| a.active).collect();
            let revoked = list.len() - active.len();
            println!("  AI Agents:  {} active, {} revoked", active.len(), revoked);
            if active.is_empty() {
                println!("              Register one with: hsip agent register <name>");
            } else {
                for a in &active {
                    let anomaly_flag = if a.anomaly_count > 0 { " ⚠ anomaly" } else { "" };
                    println!("    • {}  [{} req/min{}]", truncate(&a.name, 24), a.request_count, anomaly_flag);
                }
            }
        }
        Err(e) => println!("  AI Agents:  (unavailable: {e})"),
    }

    println!();

    // Audit section
    match audit {
        Ok(entries) if !entries.is_empty() => {
            println!("  Recent activity:");
            for e in entries {
                let detail = e.details.as_deref().unwrap_or("");
                let detail_str = if detail.is_empty() {
                    String::new()
                } else {
                    format!(" — {}", truncate(detail, 40))
                };
                println!("    {} {}  {}{}", format_timestamp(e.timestamp), " ", e.action, detail_str);
            }
        }
        Ok(_) => println!("  Recent activity: (none yet)"),
        Err(e) => println!("  Recent activity: (unavailable: {e})"),
    }

    println!();
    println!("{}", "═".repeat(60));
    println!("  Dashboard: http://127.0.0.1:7474");
    println!("  Docs:      http://127.0.0.1:7474/docs");

    Ok(())
}

// ── Utilities ─────────────────────────────────────────────────────────────────

fn load_admin_key() -> Result<String> {
    let home = dirs::home_dir().context("cannot resolve home directory")?;
    let key_path = home.join(".hsip").join("admin.key");

    if !key_path.exists() {
        bail!(
            "No API key found.\n\
             Provide one with --key or HSIP_API_KEY env var,\n\
             or check that HSIP has been started at least once (key saved to {}).",
            key_path.display()
        );
    }

    let key = std::fs::read_to_string(&key_path)
        .with_context(|| format!("failed to read {}", key_path.display()))?;

    let key = key.trim().to_string();
    if key.is_empty() {
        bail!(
            "Admin key file is empty: {}\n\
             Start HSIP once to generate the key, or provide --key manually.",
            key_path.display()
        );
    }

    Ok(key)
}

fn format_timestamp(ms: i64) -> String {
    use std::time::SystemTime;
    let secs = (ms / 1000) as u64;
    let dt = SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
    let elapsed = SystemTime::now().duration_since(dt).unwrap_or_default();
    if elapsed.as_secs() < 60 {
        "just now".to_string()
    } else if elapsed.as_secs() < 3600 {
        format!("{}m ago", elapsed.as_secs() / 60)
    } else if elapsed.as_secs() < 86400 {
        format!("{}h ago", elapsed.as_secs() / 3600)
    } else {
        format!("{}d ago", elapsed.as_secs() / 86400)
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}
