use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::{Context, Result, bail};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub security: SecurityConfig,
    pub cors: CorsConfig,
    pub metrics: MetricsConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
    #[serde(default = "default_true")]
    pub require_https: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_true")]
    pub run_migrations: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    pub master_key_path: String,
    pub admin_key_path: String,
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CorsConfig {
    #[serde(default)]
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetricsConfig {
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub format: LogFormat,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    #[default]
    Pretty,
    Json,
}

fn default_true() -> bool { true }
fn default_max_connections() -> u32 { 10 }
fn default_rate_limit() -> u32 { 60 }
fn default_log_level() -> String { "info".to_string() }

impl Config {
    /// Load configuration from file, with environment variable overrides
    pub fn load(path: &str) -> Result<Self> {
        let config_str = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;

        let mut config: Config = toml::from_str(&config_str)
            .with_context(|| format!("Failed to parse config file: {}", path))?;

        // Environment variable overrides
        if let Ok(db_url) = std::env::var("DATABASE_URL") {
            config.database.url = db_url;
        }
        if let Ok(master_key_path) = std::env::var("MASTER_KEY_PATH") {
            config.security.master_key_path = master_key_path;
        }
        if let Ok(admin_key_path) = std::env::var("ADMIN_KEY_PATH") {
            config.security.admin_key_path = admin_key_path;
        }
        if let Ok(port) = std::env::var("PORT") {
            config.server.port = port.parse()
                .context("PORT must be a valid u16")?;
        }

        Ok(config)
    }

    /// Validate configuration and check file existence
    pub fn validate(&self) -> Result<()> {
        // Validate database URL format
        if !self.database.url.starts_with("sqlite:")
            && !self.database.url.starts_with("postgres://")
            && !self.database.url.starts_with("postgresql://") {
            bail!("database.url must start with 'sqlite:', 'postgres://', or 'postgresql://'");
        }

        // Check master key file exists
        if !Path::new(&self.security.master_key_path).exists() {
            bail!("Master key file not found: {}", self.security.master_key_path);
        }

        // Check admin key file exists
        if !Path::new(&self.security.admin_key_path).exists() {
            bail!("Admin key file not found: {}", self.security.admin_key_path);
        }

        // Check TLS certificate files if TLS is configured
        if let Some(ref tls) = self.server.tls {
            if !Path::new(&tls.cert_path).exists() {
                bail!("TLS certificate file not found: {}", tls.cert_path);
            }
            if !Path::new(&tls.key_path).exists() {
                bail!("TLS private key file not found: {}", tls.key_path);
            }
        }

        // Validate port range
        if self.server.port == 0 {
            bail!("server.port must be non-zero");
        }

        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.to_lowercase().as_str()) {
            bail!("logging.level must be one of: trace, debug, info, warn, error");
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                tls: None,
            },
            database: DatabaseConfig {
                url: "sqlite::memory:".to_string(),
                max_connections: 10,
                run_migrations: true,
            },
            security: SecurityConfig {
                master_key_path: "hsip_master_key.bin".to_string(),
                admin_key_path: "hsip_admin_key.txt".to_string(),
                rate_limit_per_minute: 60,
            },
            cors: CorsConfig {
                allowed_origins: vec![],
            },
            metrics: MetricsConfig {
                token: None,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: LogFormat::Pretty,
            },
        }
    }
}
