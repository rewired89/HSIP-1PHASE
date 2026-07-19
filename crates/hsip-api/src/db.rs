use crate::config::DatabaseConfig;
use sqlx::AnyPool;
use std::sync::Once;

pub type Db = AnyPool;

static DRIVERS: Once = Once::new();

pub async fn init(database_url: &str) -> anyhow::Result<Db> {
    DRIVERS.call_once(|| {
        sqlx::any::install_default_drivers();
    });

    // In-memory databases must use exactly 1 connection, otherwise each connection
    // gets a separate database instance and tables/data won't be shared.
    let max_conns = if database_url.contains(":memory:") {
        1
    } else {
        10
    };

    let pool = sqlx::pool::PoolOptions::<sqlx::Any>::new()
        .max_connections(max_conns)
        .connect(database_url)
        .await?;

    if database_url.starts_with("sqlite") {
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA foreign_keys=ON").execute(&pool).await?;
    }

    run_migrations(&pool).await?;
    Ok(pool)
}

pub async fn init_with_config(config: &DatabaseConfig) -> anyhow::Result<Db> {
    DRIVERS.call_once(|| {
        sqlx::any::install_default_drivers();
    });

    // In-memory databases must use exactly 1 connection, otherwise each connection
    // gets a separate database instance and tables/data won't be shared.
    let max_conns = if config.url.contains(":memory:") {
        1
    } else {
        config.max_connections
    };

    tracing::debug!("Connecting to database with {} max connections", max_conns);

    let pool = sqlx::pool::PoolOptions::<sqlx::Any>::new()
        .max_connections(max_conns)
        .connect(&config.url)
        .await?;

    if config.url.starts_with("sqlite") {
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA foreign_keys=ON").execute(&pool).await?;
    }

    if config.run_migrations {
        tracing::info!("Running database migrations...");
        run_migrations(&pool).await?;
    }

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
            timestamp        INTEGER NOT NULL,
            prev_hash        TEXT,
            entry_hash       TEXT
        )",
    )
    .execute(pool)
    .await?;

    // Non-fatal: columns may already exist on upgraded databases. Rows written
    // before this migration have NULL prev_hash/entry_hash — they predate the
    // hash chain and are excluded from it (see audit_log::verify_chain).
    let _ = sqlx::query("ALTER TABLE audit_entries ADD COLUMN prev_hash TEXT")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE audit_entries ADD COLUMN entry_hash TEXT")
        .execute(pool)
        .await;

    // Enforces the hash chain's append-only, non-forking property: two
    // concurrent writers cannot both extend the same tenant's chain from the
    // same prev_hash. NULLs (pre-migration rows) are exempt by SQL semantics.
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_audit_chain
         ON audit_entries (tenant_id, prev_hash)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS contacts (
            id         TEXT PRIMARY KEY,
            tenant_id  TEXT NOT NULL,
            nickname   TEXT NOT NULL,
            verify_key TEXT NOT NULL,
            added_at   INTEGER NOT NULL,
            UNIQUE(tenant_id, verify_key)
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

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS uploads (
            id           TEXT PRIMARY KEY,
            tenant_id    TEXT NOT NULL,
            filename     TEXT NOT NULL,
            content_type TEXT NOT NULL,
            data         BLOB NOT NULL,
            size         INTEGER NOT NULL,
            created_at   INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    // Node-level Ed25519 identity used to sign anchored Merkle roots. This
    // is deliberately separate from any tenant's identity: an anchor batch
    // spans decisions from every tenant, so "who vouches for this batch"
    // must not be arbitrarily attributed to one tenant's key. Singleton row
    // (id always 1), created on first anchor cycle.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS anchor_identity (
            id              INTEGER PRIMARY KEY,
            signing_key_b64 TEXT NOT NULL,
            verify_key_b64  TEXT NOT NULL,
            created_at      INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    // Batches of decisions anchored together under one RFC 6962 Merkle root.
    // `ots_status` starts 'pending' (submitted to OpenTimestamps calendars,
    // not yet confirmed in a Bitcoin block) and moves to 'confirmed' once
    // upgraded — see routes/decisions.rs and anchor.rs.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS decision_anchors (
            id               TEXT PRIMARY KEY,
            merkle_root      TEXT NOT NULL,
            leaf_count       INTEGER NOT NULL,
            anchor_signature TEXT NOT NULL,
            anchor_verify_key TEXT NOT NULL,
            ots_proof        BLOB,
            ots_status       TEXT NOT NULL DEFAULT 'pending',
            created_at       INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    // AI-agent decision attestations (M1: decision attestation feature).
    // Two-tier by design: accountability metadata is clear text; the actual
    // decision content the caller made is never stored here, only
    // `payload_hash` (SHA-256 of a payload only the caller holds).
    // `event_hash` is SHA256(JCS(envelope)) — see hsip-core::canonical.
    // UNIQUE(tenant_id, prev_hash) serializes each tenant's hash chain: two
    // concurrent inserts racing to extend the same prev_hash cannot both
    // succeed, so the chain can't fork.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS decisions (
            id              TEXT PRIMARY KEY,
            tenant_id       TEXT NOT NULL,
            agent_key_id    TEXT NOT NULL,
            accountable_key TEXT NOT NULL,
            model_version   TEXT NOT NULL,
            strategy_id     TEXT NOT NULL,
            decision_type   TEXT NOT NULL,
            payload_hash    TEXT NOT NULL,
            prev_hash       TEXT NOT NULL,
            event_hash      TEXT NOT NULL UNIQUE,
            signature       TEXT NOT NULL,
            sign_algo       TEXT NOT NULL,
            timestamp_iso   TEXT NOT NULL,
            timestamp_int   TEXT NOT NULL,
            hsip_gov_ext    TEXT NOT NULL,
            anchor_id       TEXT,
            merkle_index    INTEGER,
            created_at      INTEGER NOT NULL,
            UNIQUE(tenant_id, prev_hash)
        )",
    )
    .execute(pool)
    .await?;

    // Indexes on tenant_id for all high-traffic tables (L4)
    let indexes = [
        "CREATE INDEX IF NOT EXISTS idx_contacts_tenant     ON contacts (tenant_id)",
        "CREATE INDEX IF NOT EXISTS idx_api_keys_tenant    ON api_keys (tenant_id)",
        "CREATE INDEX IF NOT EXISTS idx_consents_tenant    ON consents (tenant_id)",
        "CREATE INDEX IF NOT EXISTS idx_messages_tenant    ON messages (tenant_id)",
        "CREATE INDEX IF NOT EXISTS idx_credentials_tenant ON credentials (tenant_id)",
        "CREATE INDEX IF NOT EXISTS idx_audit_tenant       ON audit_entries (tenant_id)",
        "CREATE INDEX IF NOT EXISTS idx_audit_timestamp    ON audit_entries (timestamp)",
        "CREATE INDEX IF NOT EXISTS idx_uploads_tenant     ON uploads (tenant_id)",
        "CREATE INDEX IF NOT EXISTS idx_decisions_tenant   ON decisions (tenant_id)",
        "CREATE INDEX IF NOT EXISTS idx_decisions_anchor   ON decisions (anchor_id)",
        "CREATE INDEX IF NOT EXISTS idx_decisions_created  ON decisions (tenant_id, created_at)",
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
