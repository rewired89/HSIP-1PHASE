use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub type Db = Arc<Mutex<Connection>>;

pub fn init(path: &str) -> anyhow::Result<Db> {
    let conn = Connection::open(path)?;
    run_migrations(&conn)?;
    Ok(Arc::new(Mutex::new(conn)))
}

fn run_migrations(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch("
        PRAGMA journal_mode=WAL;

        CREATE TABLE IF NOT EXISTS tenants (
            id         TEXT PRIMARY KEY,
            name       TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS api_keys (
            id         TEXT PRIMARY KEY,
            tenant_id  TEXT NOT NULL,
            key_hash   TEXT NOT NULL UNIQUE,
            name       TEXT NOT NULL DEFAULT 'default',
            created_at INTEGER NOT NULL,
            active     INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE IF NOT EXISTS identities (
            tenant_id       TEXT PRIMARY KEY,
            signing_key_b64 TEXT NOT NULL,
            verify_key_b64  TEXT NOT NULL,
            created_at      INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS consents (
            id               TEXT PRIMARY KEY,
            tenant_id        TEXT NOT NULL,
            peer_verify_key  TEXT NOT NULL,
            status           TEXT NOT NULL,
            granted_at       INTEGER,
            expires_ms       INTEGER,
            revoked_at       INTEGER,
            created_at       INTEGER NOT NULL,
            UNIQUE(tenant_id, peer_verify_key)
        );

        CREATE TABLE IF NOT EXISTS messages (
            id               TEXT PRIMARY KEY,
            tenant_id        TEXT NOT NULL,
            peer_verify_key  TEXT NOT NULL,
            direction        TEXT NOT NULL,
            content          TEXT NOT NULL,
            signature        TEXT NOT NULL,
            timestamp        INTEGER NOT NULL,
            verified         INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS audit_entries (
            id               TEXT PRIMARY KEY,
            tenant_id        TEXT NOT NULL,
            action           TEXT NOT NULL,
            peer_verify_key  TEXT,
            details          TEXT,
            timestamp        INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS credentials (
            id               TEXT PRIMARY KEY,
            tenant_id        TEXT NOT NULL,
            claim            TEXT NOT NULL,
            user_token       TEXT NOT NULL,
            issuer_verify_key TEXT NOT NULL,
            issued_at        INTEGER NOT NULL,
            expires_at       INTEGER NOT NULL,
            signature        TEXT NOT NULL,
            revoked          INTEGER NOT NULL DEFAULT 0
        );
    ")?;
    Ok(())
}

pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
