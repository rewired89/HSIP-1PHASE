use anyhow::{Context, Result};
use clap::Args;
use serde::Deserialize;
use std::time::Duration;

use super::util::{admin_key_path, load_admin_key};

const DEFAULT_API_URL: &str = "http://127.0.0.1:7474";
const HEALTH_RETRIES: u32 = 20; // 20 × 500 ms = 10 s

#[derive(Args)]
pub struct UpArgs {
    #[arg(long, env = "HSIP_API_URL")]
    pub api_url: Option<String>,
    /// Skip opening the dashboard in your browser
    #[arg(long)]
    pub no_browser: bool,
}

#[derive(Deserialize)]
struct IdentityResponse {
    verify_key: String,
}

pub fn run(args: UpArgs) -> Result<()> {
    let base = args.api_url.unwrap_or_else(|| DEFAULT_API_URL.to_string());
    let base = base.trim_end_matches('/').to_string();

    let http = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .context("failed to build HTTP client")?;

    println!();
    print!("  Checking HSIP server at {}... ", base);

    if probe_health(&http, &base) {
        println!("already running ✓");
    } else {
        println!("not running");

        match find_hsip_api_bin() {
            Some(bin) => {
                print!("  Starting {} ... ", bin.display());
                match std::process::Command::new(&bin)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                {
                    Ok(_) => println!("started"),
                    Err(e) => {
                        println!("failed: {e}");
                        print_start_hint(&base);
                        return Ok(());
                    }
                }
            }
            None => {
                println!("  hsip-api binary not found in PATH or ~/.cargo/bin.");
                print_start_hint(&base);
                return Ok(());
            }
        }

        print!("  Waiting for server to come up");
        let mut came_up = false;
        for i in 0..HEALTH_RETRIES {
            std::thread::sleep(Duration::from_millis(500));
            if probe_health(&http, &base) {
                came_up = true;
                println!(" ✓");
                break;
            }
            print!(".");
            if i == HEALTH_RETRIES - 1 {
                println!(" timed out");
                println!("  Server did not respond in 10 s. Check logs.");
                return Ok(());
            }
        }
        if !came_up {
            return Ok(());
        }
    }

    // Give the server a moment to flush the admin key to disk after first boot.
    std::thread::sleep(Duration::from_millis(400));

    let key = load_admin_key()?;

    let vk_short = match get_identity(&http, &base, &key) {
        Ok(id) => {
            let vk = id.verify_key;
            if vk.len() > 28 {
                format!("{}…", &vk[..28])
            } else {
                vk
            }
        }
        Err(_) => "(identity not found — run: hsip agent register <name>)".to_string(),
    };

    let key_path = admin_key_path().display().to_string();

    println!();
    println!("  ┌───────────────────────────────────────────────────────────────┐");
    println!("  │  HSIP is ready ✓                                              │");
    println!("  ├───────────────────────────────────────────────────────────────┤");
    println!("  │  Identity:   {:<52}│", format!("{vk_short} (Ed25519)"));
    println!("  │  Dashboard:  {:<52}│", base);
    println!("  │  Admin key:  {:<52}│", key_path);
    println!("  ├───────────────────────────────────────────────────────────────┤");
    println!("  │  Register your first AI agent:                                │");
    println!("  │    hsip agent register claude                                 │");
    println!("  │    hsip agent register cursor                                 │");
    println!("  │  Discover agents already running:                             │");
    println!("  │    hsip agent discover                                        │");
    println!("  │  Full status:                                                 │");
    println!("  │    hsip status                                                │");
    println!("  └───────────────────────────────────────────────────────────────┘");
    println!();
    println!("  Share your verify key with peers: `hsip status` shows it.");
    println!("  They run `hsip trust add <label> <key>` to trust your messages.");
    println!();

    if !args.no_browser {
        open_in_browser(&base);
    }

    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn probe_health(http: &reqwest::blocking::Client, base: &str) -> bool {
    http.get(format!("{base}/health"))
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn find_hsip_api_bin() -> Option<std::path::PathBuf> {
    // 1. Sibling of the current binary (installed via cargo install)
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.with_file_name("hsip-api");
        if sibling.exists() {
            return Some(sibling);
        }
        #[cfg(target_os = "windows")]
        {
            let sibling_exe = exe.with_file_name("hsip-api.exe");
            if sibling_exe.exists() {
                return Some(sibling_exe);
            }
        }
    }
    // 2. ~/.cargo/bin/hsip-api
    if let Some(home) = dirs::home_dir() {
        let p = home.join(".cargo").join("bin").join("hsip-api");
        if p.exists() {
            return Some(p);
        }
    }
    // 3. PATH lookup
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let c = dir.join("hsip-api");
            if c.exists() {
                Some(c)
            } else {
                None
            }
        })
    })
}

fn get_identity(
    http: &reqwest::blocking::Client,
    base: &str,
    key: &str,
) -> Result<IdentityResponse> {
    let res = http
        .post(format!("{base}/v1/identity"))
        .bearer_auth(key)
        .json(&serde_json::json!({}))
        .send()?;
    if !res.status().is_success() {
        anyhow::bail!("identity request failed: {}", res.status());
    }
    res.json::<IdentityResponse>()
        .context("parse identity response")
}

fn open_in_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", url])
        .spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

fn print_start_hint(base: &str) {
    println!();
    println!("  To start HSIP manually:");
    println!("    cargo build --release -p hsip-api");
    println!("    ./target/release/hsip-api &");
    println!("    hsip up --api-url {base}");
}
