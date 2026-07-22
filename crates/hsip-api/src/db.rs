use crate::config::DatabaseConfig;
use sqlx::AnyPool;
use std::sync::Once;

pub type Db = AnyPool;

static DRIVERS: Once = Once::new();

// Unused from the `hsip-api` binary target (which calls `init_with_config`
// directly), but exercised by the `hsip_api` library target's own test
// call sites (rate_limit_persistence.rs, anchor_job.rs, audit_log.rs,
// system_health.rs) and by tests/integration.rs — both separate
// compilations from this binary's own `mod db;`.
#[allow(dead_code)]
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

/// Creates every table/index/column this version of HSIP needs, and applies
/// non-fatal backfills/widenings for upgraded databases. Idempotent and
/// backend-agnostic (SQLite or PostgreSQL via `sqlx::Any`) — the single
/// source of truth for HSIP's schema; there are no separate migration files.
/// `pub` (not just `pub(crate)`) so `bin/hsip_migrate.rs` can call it
/// directly to create the target schema before copying data — see
/// "SQLite → PostgreSQL migration tooling" in CLAUDE.md.
pub async fn run_migrations(pool: &AnyPool) -> anyhow::Result<()> {
    // NOTE on INTEGER vs BIGINT: every "created_at"/"timestamp"/"*_at"/"*_ms"
    // column below stores a millisecond-epoch value from `now_ms()` (~1.7e12
    // as of 2026) or a similarly wide value. Plain "INTEGER" is SQLite's only
    // integer keyword and is dynamically typed up to 8 bytes there, so it was
    // never a problem on SQLite — but on PostgreSQL "INTEGER" is a real 4-byte
    // int4 (max ~2.1e9), and every insert of a real epoch-ms timestamp
    // overflows it. This was never caught because HSIP had never actually
    // been run against a live Postgres instance. Millisecond-epoch and
    // similarly-wide columns use BIGINT (int8 on Postgres, identical INTEGER
    // affinity storage on SQLite — a no-op rename there). Small bounded
    // values (0/1 flags, in-batch Merkle leaf indices, anchor batch leaf
    // counts) stay INTEGER.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tenants (
            id         TEXT PRIMARY KEY,
            name       TEXT NOT NULL,
            created_at BIGINT NOT NULL
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
            created_at BIGINT NOT NULL,
            expires_at BIGINT,
            active     INTEGER NOT NULL DEFAULT 1
        )",
    )
    .execute(pool)
    .await?;

    // Non-fatal: column may already exist on upgraded databases
    let _ = sqlx::query("ALTER TABLE api_keys ADD COLUMN expires_at BIGINT")
        .execute(pool)
        .await;

    // Non-fatal: columns may already exist on upgraded databases.
    // `role` ('owner' | 'member') gates tenant-scoped key management
    // (create/revoke other keys in the same tenant) — see routes/keys.rs.
    // `is_root_admin` gates node-level operations that span every tenant
    // (master key rotation) — see routes/admin.rs. Replaces the old
    // "key named 'admin' in the first tenant ever created" heuristic with an
    // explicit, grantable flag so more than one root admin can exist.
    let _ = sqlx::query("ALTER TABLE api_keys ADD COLUMN role TEXT")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE api_keys ADD COLUMN is_root_admin INTEGER NOT NULL DEFAULT 0")
        .execute(pool)
        .await;

    // Non-fatal: column may already exist on upgraded databases. NULL (the
    // default) means this key is unbound — its bearer token alone is
    // sufficient, unchanged from before this column existed. When set (hex
    // SHA-256 of a client certificate's DER bytes), `auth.rs` additionally
    // requires the request to arrive over an mTLS connection presenting
    // that exact certificate — closing "a stolen bearer token works from
    // anywhere" for whichever keys an operator opts in. See mtls.rs.
    let _ = sqlx::query("ALTER TABLE api_keys ADD COLUMN bound_client_cert_fingerprint TEXT")
        .execute(pool)
        .await;

    // Backfill for upgraded databases: the earliest-created key in each
    // tenant becomes that tenant's 'owner' if no role is set yet, every
    // other still-unset key becomes 'member'. Preserves today's behavior
    // exactly for single-key tenants and gives every existing multi-key
    // tenant a sensible first owner instead of leaving every key in it
    // unable to manage any other after upgrade. Rows created after this
    // migration get their role set explicitly at creation time instead
    // (bootstrap_admin, sandbox::provision, routes::keys::create).
    let _ = sqlx::query(
        "UPDATE api_keys
         SET role = 'owner'
         WHERE role IS NULL
           AND id = (
             SELECT id FROM api_keys ak2
             WHERE ak2.tenant_id = api_keys.tenant_id
             ORDER BY ak2.created_at ASC
             LIMIT 1
           )",
    )
    .execute(pool)
    .await;
    let _ = sqlx::query("UPDATE api_keys SET role = 'member' WHERE role IS NULL")
        .execute(pool)
        .await;

    // Backfill for upgraded databases: preserve today's exact bootstrap-admin
    // behavior — the key named 'admin' in the very first tenant ever created
    // becomes a root admin, matching what routes::admin::require_root_admin
    // checked before this column existed. No-op once already set.
    let _ = sqlx::query(
        "UPDATE api_keys
         SET is_root_admin = 1
         WHERE is_root_admin = 0
           AND name = 'admin'
           AND tenant_id = (SELECT id FROM tenants ORDER BY created_at ASC LIMIT 1)",
    )
    .execute(pool)
    .await;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS identities (
            tenant_id       TEXT PRIMARY KEY,
            signing_key_b64 TEXT NOT NULL,
            verify_key_b64  TEXT NOT NULL,
            created_at      BIGINT NOT NULL
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
            granted_at       BIGINT,
            expires_ms       BIGINT,
            revoked_at       BIGINT,
            created_at       BIGINT NOT NULL,
            granted_by_key_type TEXT,
            UNIQUE(tenant_id, peer_verify_key)
        )",
    )
    .execute(pool)
    .await?;

    // Non-fatal: column may already exist on upgraded databases. Distinguishes
    // whether a consent grant was authorized by a human key, a service key, or
    // an ai_agent key acting on its own behalf — NULL for pre-migration rows.
    let _ = sqlx::query("ALTER TABLE consents ADD COLUMN granted_by_key_type TEXT")
        .execute(pool)
        .await;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS messages (
            id               TEXT PRIMARY KEY,
            tenant_id        TEXT NOT NULL,
            peer_verify_key  TEXT NOT NULL,
            direction        TEXT NOT NULL,
            content          TEXT NOT NULL,
            signature        TEXT NOT NULL,
            timestamp        BIGINT NOT NULL,
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
            timestamp        BIGINT NOT NULL,
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

    // Non-fatal: columns may already exist on upgraded databases. Mirror of
    // `decisions.anchor_id`/`merkle_index` — which batch (if any) this
    // entry's `entry_hash` was folded into, and its leaf position in that
    // batch's Merkle tree. NULL until `anchor_job::run_audit_anchor_cycle`
    // picks it up; rows with NULL `entry_hash` (pre-chain-migration) are
    // never eligible for anchoring in the first place.
    let _ = sqlx::query("ALTER TABLE audit_entries ADD COLUMN anchor_id TEXT")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE audit_entries ADD COLUMN merkle_index INTEGER")
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
            added_at   BIGINT NOT NULL,
            UNIQUE(tenant_id, verify_key)
        )",
    )
    .execute(pool)
    .await?;

    // Federated trust store (see routes/trust.rs and CLAUDE.md's "Federated
    // Trust" section) — a tenant's locally-registered Ed25519 verify keys
    // for other HSIP nodes/peers, keyed by a human-readable label. This
    // table was documented and routed (POST /v1/trust/peer,
    // GET /v1/trust/peers, DELETE /v1/trust/peers/:id,
    // POST /v1/trust/verify) but never actually created here — every one
    // of those routes has 500'd with "no such table" since the feature
    // shipped, on every fresh database. UNIQUE(tenant_id, verify_key)
    // backs trust::add's ON CONFLICT upsert.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS trusted_peers (
            id         TEXT PRIMARY KEY,
            tenant_id  TEXT NOT NULL,
            label      TEXT NOT NULL,
            verify_key TEXT NOT NULL,
            added_at   BIGINT NOT NULL,
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
            issued_at         BIGINT NOT NULL,
            expires_at        BIGINT NOT NULL,
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
            data         BYTEA NOT NULL,
            size         BIGINT NOT NULL,
            created_at   BIGINT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    // Periodic snapshot of the in-memory rate-limit / AI-agent-velocity
    // DashMaps (see `state.rs`'s `RateLimiter`/`AgentTracker`/`SandboxRate`
    // and `rate_limit_persistence.rs`), so a restart doesn't silently reset
    // abuse-detection counters — a key mid-way toward the 1000 req/min
    // auto-revoke threshold (see auth.rs) would otherwise get a clean slate
    // on every restart. `kind` distinguishes which of the three trackers a
    // row belongs to; `(kind, key)` is one row per live key/IP, upserted on
    // every snapshot rather than accumulating a history.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS rate_limit_state (
            kind            TEXT NOT NULL,
            state_key       TEXT NOT NULL,
            count           INTEGER NOT NULL,
            anomaly_count   INTEGER NOT NULL DEFAULT 0,
            window_start_ms BIGINT NOT NULL,
            updated_at      BIGINT NOT NULL,
            PRIMARY KEY (kind, state_key)
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
            created_at      BIGINT NOT NULL
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
            ots_proof        BYTEA,
            ots_status       TEXT NOT NULL DEFAULT 'pending',
            created_at       BIGINT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    // Batches of audit_entries anchored together under one RFC 6962 Merkle
    // root — same shape as `decision_anchors`, same node-level
    // `anchor_identity` key signs both. Closes THREAT_MODEL.md §4.8's
    // documented gap: the BLAKE3 chain (`audit_log.rs`) is self-verifiable
    // but wasn't anchored outside this database, so an attacker with DB
    // write access could delete the whole chain undetected (just not alter
    // what remained). See `anchor_job::run_audit_anchor_cycle`.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS audit_anchors (
            id               TEXT PRIMARY KEY,
            merkle_root      TEXT NOT NULL,
            leaf_count       INTEGER NOT NULL,
            anchor_signature TEXT NOT NULL,
            anchor_verify_key TEXT NOT NULL,
            ots_proof        BYTEA,
            ots_status       TEXT NOT NULL DEFAULT 'pending',
            created_at       BIGINT NOT NULL
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
            created_at      BIGINT NOT NULL,
            UNIQUE(tenant_id, prev_hash)
        )",
    )
    .execute(pool)
    .await?;

    // Non-fatal: column may already exist on upgraded databases. Empty/NULL
    // (the default, and the only possibility before this column existed)
    // means accountable_key remains purely caller-asserted metadata,
    // unchanged. When set (base64 Ed25519 signature by accountable_key's
    // own private key over accountable_proof_preimage_hash — see
    // hsip-core::canonical), it proves whoever submitted the decision
    // actually holds accountable_key's private key, not just its public
    // identifier. See routes/decisions.rs::record.
    let _ = sqlx::query("ALTER TABLE decisions ADD COLUMN accountable_key_signature TEXT")
        .execute(pool)
        .await;

    // Non-fatal: column may already exist on upgraded databases. NULL means
    // this decision predates per-transaction key derivation and was signed
    // directly with the tenant's root identity key — proof() falls back to
    // identities.verify_key_b64 for those rows. When set (base64 Ed25519
    // public key), it's the per-decision key HKDF-derived from the tenant's
    // root seed — see hsip_core::tx_key and routes/decisions.rs::record.
    let _ = sqlx::query("ALTER TABLE decisions ADD COLUMN issuer_verify_key TEXT")
        .execute(pool)
        .await;

    // Non-fatal, repeated every startup: widens every millisecond-epoch (or
    // similarly wide) column from the old plain INTEGER to BIGINT, in case
    // this pool is a PostgreSQL database whose tables were created by an
    // older version of this function (before the INTEGER/int4-overflow bug
    // above was found) — those installs could CREATE TABLE successfully but
    // had every write of a real epoch-ms timestamp fail with "integer out of
    // range", so there is no data-loss risk in widening them in place.
    // `ALTER COLUMN ... TYPE` is Postgres-only syntax; SQLite has no such
    // statement and errors on every one of these (harmless — SQLite's
    // INTEGER/BIGINT column types already store identically, so there was
    // never anything to widen there in the first place).
    let bigint_widenings = [
        "ALTER TABLE tenants ALTER COLUMN created_at TYPE BIGINT",
        "ALTER TABLE api_keys ALTER COLUMN created_at TYPE BIGINT",
        "ALTER TABLE api_keys ALTER COLUMN expires_at TYPE BIGINT",
        "ALTER TABLE identities ALTER COLUMN created_at TYPE BIGINT",
        "ALTER TABLE consents ALTER COLUMN granted_at TYPE BIGINT",
        "ALTER TABLE consents ALTER COLUMN expires_ms TYPE BIGINT",
        "ALTER TABLE consents ALTER COLUMN revoked_at TYPE BIGINT",
        "ALTER TABLE consents ALTER COLUMN created_at TYPE BIGINT",
        "ALTER TABLE messages ALTER COLUMN timestamp TYPE BIGINT",
        "ALTER TABLE audit_entries ALTER COLUMN timestamp TYPE BIGINT",
        "ALTER TABLE contacts ALTER COLUMN added_at TYPE BIGINT",
        "ALTER TABLE credentials ALTER COLUMN issued_at TYPE BIGINT",
        "ALTER TABLE credentials ALTER COLUMN expires_at TYPE BIGINT",
        "ALTER TABLE uploads ALTER COLUMN size TYPE BIGINT",
        "ALTER TABLE uploads ALTER COLUMN created_at TYPE BIGINT",
        "ALTER TABLE rate_limit_state ALTER COLUMN window_start_ms TYPE BIGINT",
        "ALTER TABLE rate_limit_state ALTER COLUMN updated_at TYPE BIGINT",
        "ALTER TABLE anchor_identity ALTER COLUMN created_at TYPE BIGINT",
        "ALTER TABLE decision_anchors ALTER COLUMN created_at TYPE BIGINT",
        "ALTER TABLE audit_anchors ALTER COLUMN created_at TYPE BIGINT",
        "ALTER TABLE decisions ALTER COLUMN created_at TYPE BIGINT",
    ];
    for stmt in &bigint_widenings {
        let _ = sqlx::query(stmt).execute(pool).await;
    }

    // Indexes on tenant_id for all high-traffic tables (L4)
    let indexes = [
        "CREATE INDEX IF NOT EXISTS idx_contacts_tenant     ON contacts (tenant_id)",
        "CREATE INDEX IF NOT EXISTS idx_trusted_peers_tenant ON trusted_peers (tenant_id)",
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
        "CREATE INDEX IF NOT EXISTS idx_audit_anchor       ON audit_entries (anchor_id)",
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
