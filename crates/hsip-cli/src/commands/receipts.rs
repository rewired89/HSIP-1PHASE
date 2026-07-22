//! Submit a local decision/audit proof bundle to a remote "collector" HSIP
//! instance — the client-side half of `routes::receipts` on the collector.
//! This is what actually lets a business run HSIP purely locally on every
//! employee's/agent's own machine and still get one centralized audit
//! trail: only the already-signed, self-contained proof bundle ever
//! leaves this machine, never the local database itself.

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use serde::Deserialize;
use std::time::Duration;

use super::util::load_admin_key;

const DEFAULT_API_URL: &str = "http://127.0.0.1:7474";

#[derive(Subcommand)]
pub enum ReceiptsCmd {
    /// Fetch a local decision's (or audit entry's) proof bundle and submit
    /// it to a remote collector for centralized, verified record-keeping
    Submit {
        /// decision_id (or audit entry id, with --type audit)
        id: String,

        /// "decision" (default) or "audit"
        #[arg(long, default_value = "decision")]
        r#type: String,

        /// Human-readable label identifying this instance to the collector
        /// (e.g. "alice-laptop") — informational only, not verified as an
        /// identity claim
        #[arg(long)]
        label: String,

        /// Base URL of the remote collector, e.g. https://audit.example.com
        #[arg(long)]
        collector_url: String,

        /// Bearer key for the collector
        #[arg(long, env = "HSIP_COLLECTOR_KEY")]
        collector_key: String,

        /// This machine's own local HSIP instance (defaults to the usual
        /// local resolution: --api-url / HSIP_API_URL / 127.0.0.1:7474)
        #[arg(long, env = "HSIP_API_URL")]
        api_url: Option<String>,

        /// This machine's own local HSIP key (defaults to the usual local
        /// resolution: --key / HSIP_API_KEY / the platform admin key file)
        #[arg(long, env = "HSIP_API_KEY")]
        key: Option<String>,
    },
}

#[derive(Deserialize, Debug)]
struct SubmitReceiptResponse {
    id: String,
    valid: bool,
    source_tenant_id: String,
    source_record_id: String,
}

fn http_client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("failed to build HTTP client")
}

pub fn run(cmd: ReceiptsCmd) -> Result<()> {
    match cmd {
        ReceiptsCmd::Submit {
            id,
            r#type,
            label,
            collector_url,
            collector_key,
            api_url,
            key,
        } => submit(
            id,
            r#type,
            label,
            collector_url,
            collector_key,
            api_url,
            key,
        ),
    }
}

fn submit(
    id: String,
    receipt_type: String,
    label: String,
    collector_url: String,
    collector_key: String,
    api_url: Option<String>,
    key: Option<String>,
) -> Result<()> {
    if receipt_type != "decision" && receipt_type != "audit" {
        bail!("--type must be \"decision\" or \"audit\", got \"{receipt_type}\"");
    }

    let local_base = api_url
        .unwrap_or_else(|| DEFAULT_API_URL.to_string())
        .trim_end_matches('/')
        .to_string();
    let local_key = match key {
        Some(k) => k,
        None => load_admin_key()?,
    };
    let http = http_client()?;

    let proof_path = if receipt_type == "decision" {
        format!("/v1/decisions/{id}/proof")
    } else {
        format!("/v1/audit/{id}/proof")
    };

    println!("Fetching proof bundle from local instance ({local_base}{proof_path})...");
    let proof_url = format!("{local_base}{proof_path}");
    let proof_res = http
        .get(&proof_url)
        .bearer_auth(&local_key)
        .send()
        .with_context(|| format!("GET {proof_url}"))?;
    if !proof_res.status().is_success() {
        let status = proof_res.status();
        let body: serde_json::Value = proof_res.json().unwrap_or_default();
        bail!("local instance returned {status} fetching proof: {body}");
    }
    let bundle: serde_json::Value = proof_res.json().context("parse local proof bundle")?;

    let collector_base = collector_url.trim_end_matches('/').to_string();
    let submit_body = serde_json::json!({
        "submitter_label": label,
        "receipt_type": receipt_type,
        "bundle": bundle,
    });

    let submit_url = format!("{collector_base}/v1/receipts/submit");
    println!("Submitting to collector ({submit_url})...");
    let submit_res = http
        .post(&submit_url)
        .bearer_auth(&collector_key)
        .json(&submit_body)
        .send()
        .with_context(|| format!("POST {submit_url}"))?;
    if !submit_res.status().is_success() {
        let status = submit_res.status();
        let body: serde_json::Value = submit_res.json().unwrap_or_default();
        bail!("collector rejected submission ({status}): {body}");
    }
    let resp: SubmitReceiptResponse = submit_res.json().context("parse collector response")?;

    println!();
    println!("✓ Receipt submitted and independently verified by the collector.");
    println!("  Receipt ID:        {}", resp.id);
    println!("  Source tenant:     {}", resp.source_tenant_id);
    println!("  Source record ID:  {}", resp.source_record_id);
    println!("  Valid:             {}", resp.valid);
    println!();
    Ok(())
}
