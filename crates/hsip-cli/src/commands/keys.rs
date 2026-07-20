//! `hsip keys` subcommands — inspect and rotate the node's master key.
//!
//! Key resolution order (highest priority first):
//!   1. --key flag
//!   2. HSIP_API_KEY env var
//!   3. Platform-aware admin key file via commands::util::load_admin_key()
//!
//! URL resolution order:
//!   1. --api-url flag
//!   2. HSIP_API_URL env var
//!   3. http://127.0.0.1:7474 (desktop default)

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use serde::Deserialize;
use std::io::Write;
use std::time::Duration;

use super::util::load_admin_key;

const DEFAULT_API_URL: &str = "http://127.0.0.1:7474";

// ── Clap types ────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum KeysCmd {
    /// Show the running master key's fingerprint — safe to compare against
    /// a backup file, never reveals the key itself
    MasterFingerprint {
        #[arg(long, env = "HSIP_API_URL")]
        api_url: Option<String>,
        #[arg(long, env = "HSIP_API_KEY")]
        key: Option<String>,
    },

    /// Rotate the master key: re-encrypts every identity under a fresh key
    /// and swaps it live, no restart. Admin key only.
    RotateMaster {
        #[arg(long, env = "HSIP_API_URL")]
        api_url: Option<String>,
        #[arg(long, env = "HSIP_API_KEY")]
        key: Option<String>,
        /// Skip the interactive confirmation prompt (for scripts)
        #[arg(long)]
        yes: bool,
    },
}

// ── API response types ────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct FingerprintResponse {
    fingerprint: String,
    master_key_path: Option<String>,
}

#[derive(Deserialize, Debug)]
struct RotateResponse {
    identities_reencrypted: u64,
    anchor_identity_reencrypted: bool,
    old_key_fingerprint: String,
    new_key_fingerprint: String,
    master_key_path: String,
    note: String,
}

// ── HTTP client helper (same pattern as agent.rs / trust.rs) ───────────────────

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
            .timeout(Duration::from_secs(30))
            .build()
            .context("failed to build HTTP client")?;
        Ok(Self { base, key, http })
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

    fn post<R: serde::de::DeserializeOwned>(&self, path: &str) -> Result<R> {
        let url = format!("{}{}", self.base, path);
        let res = self
            .http
            .post(&url)
            .bearer_auth(&self.key)
            .json(&serde_json::json!({}))
            .send()
            .with_context(|| format!("POST {url} failed — is HSIP running?"))?;
        if !res.status().is_success() {
            let status = res.status();
            let body: serde_json::Value = res.json().unwrap_or_default();
            bail!("API error {status}: {body}");
        }
        res.json().context("failed to parse response")
    }
}

// ── Command handlers ──────────────────────────────────────────────────────────

pub fn run(cmd: KeysCmd) -> Result<()> {
    match cmd {
        KeysCmd::MasterFingerprint { api_url, key } => master_fingerprint(api_url, key),
        KeysCmd::RotateMaster { api_url, key, yes } => rotate_master(api_url, key, yes),
    }
}

fn master_fingerprint(api_url: Option<String>, key: Option<String>) -> Result<()> {
    let client = ApiClient::new(api_url, key)?;
    let resp: FingerprintResponse = client.get("/v1/admin/master-key/fingerprint")?;

    println!();
    println!("Master key fingerprint: {}", resp.fingerprint);
    match &resp.master_key_path {
        Some(path) => println!("Source: file at {path}"),
        None => println!(
            "Source: HSIP_MASTER_KEY env var (no file — `hsip keys rotate-master` will refuse)"
        ),
    }
    println!();
    println!("To confirm a backup file matches, on the machine holding the backup:");
    println!("  echo -n \"$(cat <backup-file>)\" | xxd -r -p | sha256sum | cut -c1-16");
    println!("(the fingerprint above is the first 8 bytes of SHA-256 of the raw 32-byte key, hex-encoded)");

    Ok(())
}

fn rotate_master(api_url: Option<String>, key: Option<String>, yes: bool) -> Result<()> {
    let client = ApiClient::new(api_url, key)?;

    if !yes {
        println!();
        println!("This rotates the master key that encrypts every tenant's private signing key.");
        println!("  • Every identity is re-encrypted under a new key, live — no restart needed.");
        println!(
            "  • The OLD key will no longer decrypt anything on this server after this completes."
        );
        println!("  • Any other process still holding the old key (e.g. a backup script reading");
        println!("    HSIP_MASTER_KEY, or a copy of the old key file) is now stale.");
        println!();
        print!("Type \"yes\" to continue: ");
        std::io::stdout().flush().ok();
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .context("failed to read confirmation")?;
        if input.trim() != "yes" {
            println!("Aborted. No changes were made.");
            return Ok(());
        }
    }

    let resp: RotateResponse = client.post("/v1/admin/master-key/rotate")?;

    println!();
    println!("✓ Master key rotated.");
    println!(
        "  Identities re-encrypted:      {}",
        resp.identities_reencrypted
    );
    println!(
        "  Anchor identity re-encrypted: {}",
        resp.anchor_identity_reencrypted
    );
    println!("  Old key fingerprint: {}", resp.old_key_fingerprint);
    println!("  New key fingerprint: {}", resp.new_key_fingerprint);
    println!("  Key file:            {}", resp.master_key_path);
    println!();
    println!("{}", resp.note);

    Ok(())
}
