use anyhow::{bail, Context, Result};
use clap::Subcommand;
use serde::Deserialize;
use std::time::Duration;

use super::util::load_admin_key;

const DEFAULT_API_URL: &str = "http://127.0.0.1:7474";

// ── Clap types ────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum TrustCmd {
    /// Add a trusted peer by label and their Ed25519 verify key
    Add {
        /// Human-readable name, e.g. "alice" or "claude-desktop"
        label: String,
        /// Base64-encoded Ed25519 verify key (from `hsip status` or GET /v1/identity)
        verify_key: String,
        #[arg(long, env = "HSIP_API_URL")]
        api_url: Option<String>,
        #[arg(long, env = "HSIP_API_KEY")]
        key: Option<String>,
    },

    /// List all trusted peers
    List {
        #[arg(long, env = "HSIP_API_URL")]
        api_url: Option<String>,
        #[arg(long, env = "HSIP_API_KEY")]
        key: Option<String>,
    },

    /// Remove a trusted peer by ID (from `hsip trust list`)
    Remove {
        id: String,
        #[arg(long, env = "HSIP_API_URL")]
        api_url: Option<String>,
        #[arg(long, env = "HSIP_API_KEY")]
        key: Option<String>,
    },

    /// Verify a signature from a trusted peer identified by label
    Verify {
        /// Label of the trusted peer who signed the message
        #[arg(long)]
        from: String,
        /// The original message content
        content: String,
        /// Base64-encoded Ed25519 signature
        signature: String,
        #[arg(long, env = "HSIP_API_URL")]
        api_url: Option<String>,
        #[arg(long, env = "HSIP_API_KEY")]
        key: Option<String>,
    },
}

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct TrustedPeer {
    id: String,
    label: String,
    verify_key: String,
    added_at: i64,
}

#[derive(Deserialize, Debug)]
struct TrustVerifyResponse {
    verified: bool,
    label: String,
    verify_key: String,
}

// ── HTTP client ───────────────────────────────────────────────────────────────

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
            .with_context(|| format!("POST {url}"))?;
        if !res.status().is_success() {
            let s = res.status();
            let b: serde_json::Value = res.json().unwrap_or_default();
            bail!("API error {s}: {b}");
        }
        res.json().context("parse response")
    }

    fn get<R: serde::de::DeserializeOwned>(&self, path: &str) -> Result<R> {
        let url = format!("{}{}", self.base, path);
        let res = self
            .http
            .get(&url)
            .bearer_auth(&self.key)
            .send()
            .with_context(|| format!("GET {url}"))?;
        if !res.status().is_success() {
            let s = res.status();
            let b: serde_json::Value = res.json().unwrap_or_default();
            bail!("API error {s}: {b}");
        }
        res.json().context("parse response")
    }

    fn delete(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base, path);
        let res = self
            .http
            .delete(&url)
            .bearer_auth(&self.key)
            .send()
            .with_context(|| format!("DELETE {url}"))?;
        if !res.status().is_success() {
            let s = res.status();
            let b: serde_json::Value = res.json().unwrap_or_default();
            bail!("API error {s}: {b}");
        }
        res.json().context("parse response")
    }
}

// ── Command dispatcher ────────────────────────────────────────────────────────

pub fn run(cmd: TrustCmd) -> Result<()> {
    match cmd {
        TrustCmd::Add {
            label,
            verify_key,
            api_url,
            key,
        } => add(label, verify_key, api_url, key),
        TrustCmd::List { api_url, key } => list(api_url, key),
        TrustCmd::Remove { id, api_url, key } => remove(id, api_url, key),
        TrustCmd::Verify {
            from,
            content,
            signature,
            api_url,
            key,
        } => verify(from, content, signature, api_url, key),
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

fn add(
    label: String,
    verify_key: String,
    api_url: Option<String>,
    key: Option<String>,
) -> Result<()> {
    let client = ApiClient::new(api_url, key)?;
    let peer: TrustedPeer = client.post(
        "/v1/trust/peer",
        &serde_json::json!({ "label": label, "verify_key": verify_key }),
    )?;

    println!();
    println!("✓ Trusted peer added.");
    println!("  ID:         {}", peer.id);
    println!("  Label:      {}", peer.label);
    println!(
        "  Verify key: {}…",
        &peer.verify_key[..peer.verify_key.len().min(32)]
    );
    println!();
    println!("Verify their messages with:");
    println!(
        "  hsip trust verify --from \"{}\" <content> <signature>",
        peer.label
    );
    Ok(())
}

fn list(api_url: Option<String>, key: Option<String>) -> Result<()> {
    let client = ApiClient::new(api_url, key)?;
    let peers: Vec<TrustedPeer> = client.get("/v1/trust/peers")?;

    if peers.is_empty() {
        println!("No trusted peers yet.");
        println!("Add one with:  hsip trust add <label> <verify-key>");
        return Ok(());
    }

    println!();
    println!(
        "{:<36}  {:<20}  {:<26}  Added",
        "ID", "Label", "Verify key (truncated)"
    );
    println!("{}", "─".repeat(94));
    for p in &peers {
        let vk_short = format!("{}…", &p.verify_key[..p.verify_key.len().min(24)]);
        let ago = format_ago(p.added_at);
        println!(
            "{:<36}  {:<20}  {:<26}  {}",
            p.id,
            truncate(&p.label, 20),
            vk_short,
            ago
        );
    }
    println!();
    println!(
        "{} trusted peer(s).  Remove: hsip trust remove <id>",
        peers.len()
    );
    Ok(())
}

fn remove(id: String, api_url: Option<String>, key: Option<String>) -> Result<()> {
    let client = ApiClient::new(api_url, key)?;
    client.delete(&format!("/v1/trust/peers/{id}"))?;
    println!("✓ Trusted peer {id} removed.");
    Ok(())
}

fn verify(
    from: String,
    content: String,
    signature: String,
    api_url: Option<String>,
    key: Option<String>,
) -> Result<()> {
    let client = ApiClient::new(api_url, key)?;
    let resp: TrustVerifyResponse = client.post(
        "/v1/trust/verify",
        &serde_json::json!({ "label": from, "content": content, "signature": signature }),
    )?;

    println!();
    if resp.verified {
        println!("✓ Signature VALID");
        println!(
            "  From:  {} ({}…)",
            resp.label,
            &resp.verify_key[..resp.verify_key.len().min(24)]
        );
        println!("  Message: \"{}\"", truncate(&content, 60));
    } else {
        println!("✗ Signature INVALID");
        println!("  The message may have been tampered with or signed by a different key.");
        println!(
            "  Claimed peer: {} ({}…)",
            resp.label,
            &resp.verify_key[..resp.verify_key.len().min(24)]
        );
    }
    println!();
    Ok(())
}

// ── Utilities ─────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

fn format_ago(ms: i64) -> String {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    let dt = UNIX_EPOCH + Duration::from_millis(ms as u64);
    let elapsed = SystemTime::now().duration_since(dt).unwrap_or_default();
    let s = elapsed.as_secs();
    if s < 60 {
        "just now".to_string()
    } else if s < 3600 {
        format!("{}m ago", s / 60)
    } else if s < 86400 {
        format!("{}h ago", s / 3600)
    } else {
        format!("{}d ago", s / 86400)
    }
}
