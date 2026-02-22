use std::sync::Once;
use sqlx::AnyPool;

pub type Db = AnyPool;

static DRIVERS: Once = Once::new();

pub async fn init(database_url: &str) -> anyhow::Result<Db> {
    DRIVERS.call_once(|| {
        sqlx::any::install_default_drivers();
    });

    // In-memory databases must use exactly 1 connection, otherwise each connection
    // gets a separate database instance and tables/data won't be shared.
    let max_conns = if database_url.contains(":memory:") { 1 } else { 10 };

    let pool = sqlx::pool::PoolOptions::<sqlx::Any>::new()
        .max_connections(max_conns)
        .connect(database_url)
        .await?;

    if database_url.starts_with("sqlite") {
        sqlx::query("PRAGMA journal_mode=WAL").execute(&pool).await?;
        sqlx::query("PRAGMA foreign_keys=ON").execute(&pool).await?;
    }

    run_migrations(&pool).await?;
    Ok(pool)
}

async fn run_migrations(pool: &AnyPool) -> anyhow::Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tenants (
            id         TEXT PRIMARY KEY,
            name       TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS api_keys (
            id         TEXT PRIMARY KEY,
            tenant_id  TEXT NOT NULL,
            key_hash   TEXT NOT NULL UNIQUE,
            name       TEXT NOT NULL DEFAULT 'default',
            agent_type TEXT NOT NULL DEFAULT 'human',
            created_at INTEGER NOT NULL,
            expires_at INTEGER,
            active     INTEGER NOT NULL DEFAULT 1
        )",
    )
    .execute(pool)
    .await?;

    // Non-fatal: column may already exist on upgraded databases
    let _ = sqlx::query("ALTER TABLE api_keys ADD COLUMN expires_at INTEGER")
        .execute(pool)
        .await;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS identities (
            tenant_id       TEXT PRIMARY KEY,
            signing_key_b64 TEXT NOT NULL,
            verify_key_b64  TEXT NOT NULL,
            created_at      INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS consents (
            id               TEXT PRIMARY KEY,
            tenant_id        TEXT NOT NULL,
            peer_verify_key  TEXT NOT NULL,
            status           TEXT NOT NULL,
            granted_at       INTEGER,
            expires_ms       INTEGER,
            revoked_at       INTEGER,
            created_at       INTEGER NOT NULL,
            UNIQUE(tenant_id, peer_verify_key)
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS messages (
            id               TEXT PRIMARY KEY,
            tenant_id        TEXT NOT NULL,
            peer_verify_key  TEXT NOT NULL,
            direction        TEXT NOT NULL,
            content          TEXT NOT NULL,
            signature        TEXT NOT NULL,
            timestamp        INTEGER NOT NULL,
            verified         INTEGER NOT NULL DEFAULT 0
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS audit_entries (
            id               TEXT PRIMARY KEY,
            tenant_id        TEXT NOT NULL,
            action           TEXT NOT NULL,
            peer_verify_key  TEXT,
            details          TEXT,
            timestamp        INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS credentials (
            id                TEXT PRIMARY KEY,
            tenant_id         TEXT NOT NULL,
            claim             TEXT NOT NULL,
            user_token        TEXT NOT NULL,
            issuer_verify_key TEXT NOT NULL,
            issued_at         INTEGER NOT NULL,
            expires_at        INTEGER NOT NULL,
            signature         TEXT NOT NULL,
            revoked           INTEGER NOT NULL DEFAULT 0
        )",
    )
    .execute(pool)
    .await?;

    // Indexes on tenant_id for all high-traffic tables (L4)
    let indexes = [
        "CREATE INDEX IF NOT EXISTS idx_api_keys_tenant    ON api_keys (tenant_id)",
        "CREATE INDEX IF NOT EXISTS idx_consents_tenant    ON consents (tenant_id)",
        "CREATE INDEX IF NOT EXISTS idx_messages_tenant    ON messages (tenant_id)",
        "CREATE INDEX IF NOT EXISTS idx_credentials_tenant ON credentials (tenant_id)",
        "CREATE INDEX IF NOT EXISTS idx_audit_tenant       ON audit_entries (tenant_id)",
        "CREATE INDEX IF NOT EXISTS idx_audit_timestamp    ON audit_entries (timestamp)",
    ];
    for idx in &indexes {
        sqlx::query(idx).execute(pool).await?;
    }

    Ok(())
}

pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
