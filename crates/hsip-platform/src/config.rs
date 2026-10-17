//! Environment-driven configuration. Everything has a safe local default so
//! `cargo run -p hsip-platform` works with zero setup.

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: String,
    pub database_url: String,
    pub data_dir: PathBuf,
    /// Hex master key from `HSP_MASTER_KEY`; `None` means use/create the key file.
    pub master_key_hex: Option<String>,
    pub operator_key: Option<String>,
    pub public_url: String,
    pub signup_open: bool,
    pub ots_enabled: bool,
    pub anchor_interval_secs: u64,
    pub anchor_batch_trigger: i64,
    pub console_dir: Option<PathBuf>,
    /// Cheap Argon2 parameters, for tests only.
    pub fast_password_hash: bool,
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn flag(name: &str, default: bool) -> bool {
    match env(name).as_deref() {
        Some("1" | "true" | "yes" | "on") => true,
        Some("0" | "false" | "no" | "off") => false,
        _ => default,
    }
}

impl Config {
    pub fn from_env() -> Self {
        let data_dir = PathBuf::from(env("HSP_DATA_DIR").unwrap_or_else(|| "./data".into()));
        let bind = match (env("HSP_BIND"), env("PORT")) {
            (Some(b), _) => b,
            (None, Some(p)) => format!("0.0.0.0:{p}"),
            (None, None) => "127.0.0.1:8080".into(),
        };
        let database_url = env("DATABASE_URL")
            .unwrap_or_else(|| format!("sqlite://{}/platform.db?mode=rwc", data_dir.display()));
        let public_url = env("HSP_PUBLIC_URL")
            .unwrap_or_else(|| format!("http://{}", bind.replace("0.0.0.0", "127.0.0.1")))
            .trim_end_matches('/')
            .to_string();
        Self {
            bind,
            database_url,
            data_dir,
            master_key_hex: env("HSP_MASTER_KEY"),
            operator_key: env("HSP_OPERATOR_KEY"),
            public_url,
            signup_open: flag("HSP_SIGNUP", true),
            ots_enabled: flag("HSP_OTS", false),
            anchor_interval_secs: env("HSP_ANCHOR_INTERVAL_SECS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            anchor_batch_trigger: env("HSP_ANCHOR_BATCH")
                .and_then(|v| v.parse().ok())
                .unwrap_or(500),
            console_dir: env("HSP_CONSOLE_DIR").map(PathBuf::from),
            fast_password_hash: flag("HSP_FAST_PASSWORD_HASH", false),
        }
    }

    /// In-memory defaults for the integration tests.
    pub fn for_tests() -> Self {
        Self {
            bind: "127.0.0.1:0".into(),
            database_url: "sqlite::memory:".into(),
            data_dir: PathBuf::from("./data-test"),
            master_key_hex: Some("11".repeat(32)),
            operator_key: Some("operator-key-for-tests-0123456789".into()),
            public_url: "http://localhost:8080".into(),
            signup_open: true,
            ots_enabled: false,
            anchor_interval_secs: 3600,
            anchor_batch_trigger: 500,
            console_dir: None,
            fast_password_hash: true,
        }
    }
}
