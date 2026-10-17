//! Database access. One `sqlx::Any` pool serves SQLite (zero-config) and
//! PostgreSQL (production). Rules that keep both backends working:
//! `$1, $2, ...` placeholders only, `BIGINT` for ms-epoch values, `BYTEA`
//! for binary, no `SUM()` (Postgres returns NUMERIC), no untyped NULL
//! binds inside `$n IS NULL OR ...` (build the WHERE clause instead).

use anyhow::Result;
use sqlx::{any::AnyPoolOptions, AnyPool};

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn ms_to_iso(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|d| d.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
        .unwrap_or_default()
}

/// `YYYY-MM` bucket used for monthly usage.
pub fn month_key(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|d| d.format("%Y-%m").to_string())
        .unwrap_or_default()
}

pub async fn connect(url: &str) -> Result<AnyPool> {
    sqlx::any::install_default_drivers();
    // In-memory SQLite is one database per connection.
    let max = if url.contains(":memory:") { 1 } else { 10 };
    let pool = AnyPoolOptions::new()
        .max_connections(max)
        .connect(url)
        .await?;
    run_migrations(&pool).await?;
    Ok(pool)
}

const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS organizations (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        slug TEXT NOT NULL UNIQUE,
        plan TEXT NOT NULL,
        status TEXT NOT NULL,
        signing_key_enc TEXT NOT NULL,
        verify_key_b64 TEXT NOT NULL,
        created_at BIGINT NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS users (
        id TEXT PRIMARY KEY,
        email TEXT NOT NULL UNIQUE,
        name TEXT NOT NULL,
        password_hash TEXT NOT NULL,
        created_at BIGINT NOT NULL,
        last_login_at BIGINT
    )",
    "CREATE TABLE IF NOT EXISTS memberships (
        org_id TEXT NOT NULL,
        user_id TEXT NOT NULL,
        role TEXT NOT NULL,
        created_at BIGINT NOT NULL,
        PRIMARY KEY (org_id, user_id)
    )",
    "CREATE INDEX IF NOT EXISTS idx_memberships_user ON memberships(user_id)",
    "CREATE TABLE IF NOT EXISTS sessions (
        token_hash TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        created_at BIGINT NOT NULL,
        expires_at BIGINT NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS invites (
        id TEXT PRIMARY KEY,
        org_id TEXT NOT NULL,
        email TEXT NOT NULL,
        role TEXT NOT NULL,
        token_hash TEXT NOT NULL UNIQUE,
        invited_by TEXT NOT NULL,
        created_at BIGINT NOT NULL,
        expires_at BIGINT NOT NULL,
        accepted_at BIGINT
    )",
    "CREATE TABLE IF NOT EXISTS streams (
        id TEXT PRIMARY KEY,
        org_id TEXT NOT NULL,
        name TEXT NOT NULL,
        slug TEXT NOT NULL,
        kind TEXT NOT NULL,
        description TEXT NOT NULL,
        archived INTEGER NOT NULL,
        created_at BIGINT NOT NULL,
        UNIQUE (org_id, slug)
    )",
    "CREATE TABLE IF NOT EXISTS api_keys (
        id TEXT PRIMARY KEY,
        org_id TEXT NOT NULL,
        stream_id TEXT,
        name TEXT NOT NULL,
        scope TEXT NOT NULL,
        key_prefix TEXT NOT NULL,
        key_hash TEXT NOT NULL UNIQUE,
        created_by TEXT NOT NULL,
        created_at BIGINT NOT NULL,
        last_used_at BIGINT,
        revoked_at BIGINT
    )",
    "CREATE INDEX IF NOT EXISTS idx_api_keys_org ON api_keys(org_id)",
    "CREATE TABLE IF NOT EXISTS proofs (
        id TEXT PRIMARY KEY,
        org_id TEXT NOT NULL,
        stream_id TEXT NOT NULL,
        seq BIGINT NOT NULL,
        key_id TEXT,
        actor TEXT NOT NULL,
        action TEXT NOT NULL,
        subject TEXT NOT NULL,
        payload_hash TEXT NOT NULL,
        metadata_json TEXT NOT NULL,
        prev_hash TEXT NOT NULL,
        event_hash TEXT NOT NULL,
        signature TEXT NOT NULL,
        timestamp_iso TEXT NOT NULL,
        created_at BIGINT NOT NULL,
        anchor_id TEXT,
        merkle_index BIGINT,
        UNIQUE (stream_id, seq),
        UNIQUE (stream_id, prev_hash)
    )",
    "CREATE INDEX IF NOT EXISTS idx_proofs_org_time ON proofs(org_id, created_at)",
    "CREATE INDEX IF NOT EXISTS idx_proofs_anchor ON proofs(anchor_id)",
    "CREATE INDEX IF NOT EXISTS idx_proofs_unanchored ON proofs(anchor_id, created_at)",
    "CREATE TABLE IF NOT EXISTS anchors (
        id TEXT PRIMARY KEY,
        merkle_root TEXT NOT NULL,
        leaf_count BIGINT NOT NULL,
        anchor_signature TEXT NOT NULL,
        anchor_verify_key TEXT NOT NULL,
        ots_proof BYTEA,
        ots_status TEXT NOT NULL,
        created_at BIGINT NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS anchor_identity (
        id INTEGER PRIMARY KEY,
        signing_key_enc TEXT NOT NULL,
        verify_key_b64 TEXT NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS shares (
        id TEXT PRIMARY KEY,
        org_id TEXT NOT NULL,
        proof_id TEXT NOT NULL,
        token_hash TEXT NOT NULL UNIQUE,
        created_by TEXT NOT NULL,
        created_at BIGINT NOT NULL,
        expires_at BIGINT,
        revoked_at BIGINT,
        views BIGINT NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS evidence_exports (
        id TEXT PRIMARY KEY,
        org_id TEXT NOT NULL,
        stream_id TEXT NOT NULL,
        title TEXT NOT NULL,
        from_ms BIGINT NOT NULL,
        to_ms BIGINT NOT NULL,
        proof_count BIGINT NOT NULL,
        manifest_hash TEXT NOT NULL,
        pack_json TEXT NOT NULL,
        created_by TEXT NOT NULL,
        created_at BIGINT NOT NULL
    )",
    "CREATE TABLE IF NOT EXISTS usage_monthly (
        org_id TEXT NOT NULL,
        month TEXT NOT NULL,
        proofs BIGINT NOT NULL,
        PRIMARY KEY (org_id, month)
    )",
    "CREATE TABLE IF NOT EXISTS leads (
        id TEXT PRIMARY KEY,
        org_id TEXT,
        email TEXT NOT NULL,
        company TEXT NOT NULL,
        message TEXT NOT NULL,
        kind TEXT NOT NULL,
        created_at BIGINT NOT NULL
    )",
];

pub async fn run_migrations(pool: &AnyPool) -> Result<()> {
    for stmt in SCHEMA {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}
