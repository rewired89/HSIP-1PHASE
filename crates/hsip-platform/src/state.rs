use std::sync::Arc;

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use sqlx::{AnyPool, Row};

use crate::{config::Config, crypto, db};

#[derive(Clone)]
pub struct AppState {
    pub db: AnyPool,
    pub cfg: Arc<Config>,
    pub master_key: Arc<Vec<u8>>,
    /// Node-level key that signs anchor batch roots. Distinct from any
    /// organization's key: a batch spans every customer.
    pub anchor_key: Arc<SigningKey>,
    org_keys: Arc<DashMap<String, SigningKey>>,
    throttle: Arc<DashMap<String, (u32, i64)>>,
    last_sweep: Arc<std::sync::atomic::AtomicI64>,
}

fn load_master_key(cfg: &Config) -> Result<Vec<u8>> {
    if let Some(h) = &cfg.master_key_hex {
        let k = hex::decode(h.trim()).map_err(|_| anyhow!("HSP_MASTER_KEY must be hex"))?;
        if k.len() != 32 {
            return Err(anyhow!("HSP_MASTER_KEY must be 32 bytes (64 hex chars)"));
        }
        return Ok(k);
    }
    let path = cfg.data_dir.join("master.key");
    if path.exists() {
        let k = hex::decode(std::fs::read_to_string(&path)?.trim())?;
        if k.len() != 32 {
            return Err(anyhow!("{} is not a 32-byte hex key", path.display()));
        }
        return Ok(k);
    }
    std::fs::create_dir_all(&cfg.data_dir)?;
    let mut k = [0u8; 32];
    rand::RngCore::fill_bytes(&mut OsRng, &mut k);
    write_secret_file(&path, &hex::encode(k))?;
    tracing::warn!(path = %path.display(), "generated a new master key; back it up, signing keys are unrecoverable without it");
    Ok(k.to_vec())
}

fn write_secret_file(path: &std::path::Path, content: &str) -> Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(content.as_bytes())?;
    }
    #[cfg(not(unix))]
    std::fs::write(path, content)?;
    Ok(())
}

impl AppState {
    pub async fn new(cfg: Config) -> Result<Self> {
        let master = load_master_key(&cfg)?;
        if let Some(op) = &cfg.operator_key {
            if op.len() < 24 {
                return Err(anyhow!("HSP_OPERATOR_KEY must be at least 24 characters"));
            }
        }
        let pool = db::connect(&cfg.database_url).await?;
        let anchor_key = load_or_create_anchor_key(&pool, &master).await?;
        Ok(Self {
            db: pool,
            cfg: Arc::new(cfg),
            master_key: Arc::new(master),
            anchor_key: Arc::new(anchor_key),
            org_keys: Arc::new(DashMap::new()),
            throttle: Arc::new(DashMap::new()),
            last_sweep: Arc::new(std::sync::atomic::AtomicI64::new(0)),
        })
    }

    /// The organization's Ed25519 signing key, decrypted on first use.
    pub async fn org_signing_key(&self, org_id: &str) -> Result<SigningKey> {
        if let Some(k) = self.org_keys.get(org_id) {
            return Ok(k.clone());
        }
        let row = sqlx::query("SELECT signing_key_enc FROM organizations WHERE id = $1")
            .bind(org_id)
            .fetch_one(&self.db)
            .await?;
        let enc: String = row.try_get(0)?;
        let seed: [u8; 32] = crypto::decrypt_secret(&self.master_key, &enc)?
            .try_into()
            .map_err(|_| anyhow!("stored signing key has wrong length"))?;
        let key = SigningKey::from_bytes(&seed);
        self.org_keys.insert(org_id.to_string(), key.clone());
        Ok(key)
    }

    /// Fixed-window counter. Returns false once `limit` hits within `window_ms`.
    pub fn allow(&self, bucket: &str, limit: u32, window_ms: i64) -> bool {
        let now = db::now_ms();
        let mut e = self.throttle.entry(bucket.to_string()).or_insert((0, now));
        if now - e.1 >= window_ms {
            *e = (0, now);
        }
        e.0 += 1;
        e.0 <= limit
    }

    /// Drops expired throttle windows, at most once a minute.
    pub fn sweep_throttle_if_due(&self) {
        let now = db::now_ms();
        let last = self.last_sweep.load(std::sync::atomic::Ordering::Relaxed);
        if now - last < 60_000 {
            return;
        }
        self.last_sweep
            .store(now, std::sync::atomic::Ordering::Relaxed);
        self.throttle.retain(|_, v| now - v.1 < 3_600_000);
    }
}

async fn load_or_create_anchor_key(pool: &AnyPool, master: &[u8]) -> Result<SigningKey> {
    if let Some(row) = sqlx::query("SELECT signing_key_enc FROM anchor_identity WHERE id = 1")
        .fetch_optional(pool)
        .await?
    {
        let enc: String = row.try_get(0)?;
        let seed: [u8; 32] = crypto::decrypt_secret(master, &enc)?
            .try_into()
            .map_err(|_| anyhow!("stored anchor key has wrong length"))?;
        return Ok(SigningKey::from_bytes(&seed));
    }
    let key = SigningKey::generate(&mut OsRng);
    let enc = crypto::encrypt_secret(master, &key.to_bytes())?;
    let vk = base64_std(&key.verifying_key().to_bytes());
    // Two nodes booting at once: the loser re-reads the winner's key.
    let res = sqlx::query(
        "INSERT INTO anchor_identity (id, signing_key_enc, verify_key_b64) VALUES (1, $1, $2)",
    )
    .bind(&enc)
    .bind(&vk)
    .execute(pool)
    .await;
    match res {
        Ok(_) => Ok(key),
        Err(_) => {
            let row = sqlx::query("SELECT signing_key_enc FROM anchor_identity WHERE id = 1")
                .fetch_one(pool)
                .await?;
            let enc: String = row.try_get(0)?;
            let seed: [u8; 32] = crypto::decrypt_secret(master, &enc)?
                .try_into()
                .map_err(|_| anyhow!("stored anchor key has wrong length"))?;
            Ok(SigningKey::from_bytes(&seed))
        }
    }
}

pub fn base64_std(b: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.encode(b)
}
