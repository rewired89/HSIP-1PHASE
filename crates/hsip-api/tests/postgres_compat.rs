//! Regression test for two real, previously-undiscovered bugs that made
//! HSIP's schema and query layer non-functional against real PostgreSQL,
//! found while building `hsip-migrate` (see CLAUDE.md's "SQLite →
//! PostgreSQL migration tooling"):
//!
//!   1. `db.rs` declared millisecond-epoch timestamp columns as `INTEGER`.
//!      SQLite's only integer keyword is dynamically 8-byte, but Postgres's
//!      `INTEGER` is a real 4-byte `int4` (max ~2.1e9) — every insert of a
//!      real epoch-ms timestamp (~1.7e12) overflowed it. Fixed by using
//!      `BIGINT` for wide columns.
//!   2. Every parameterized query in this codebase used `?` placeholders.
//!      `sqlx::Any` does NOT rewrite placeholder syntax per backend — `?` is
//!      valid on SQLite but a syntax error on Postgres. Fixed by rewriting
//!      every query to PostgreSQL-style `$1, $2, ...` numbered placeholders,
//!      which SQLite also accepts identically.
//!
//! Ignored by default: this crate's normal test suite runs entirely against
//! in-memory SQLite and has no live-Postgres dependency, matching every
//! other test in this repo. Run explicitly against a real Postgres instance:
//!
//!   createdb hsip_pg_compat_test
//!   HSIP_TEST_POSTGRES_URL=postgresql://user:pass@localhost/hsip_pg_compat_test \
//!     cargo test -p hsip-api --test postgres_compat -- --ignored

use sqlx::any::AnyPoolOptions;
use sqlx::Row;

#[tokio::test]
#[ignore = "requires a live PostgreSQL instance via HSIP_TEST_POSTGRES_URL"]
async fn schema_and_placeholders_work_against_real_postgres() {
    let url = std::env::var("HSIP_TEST_POSTGRES_URL")
        .expect("set HSIP_TEST_POSTGRES_URL to a real, empty PostgreSQL database to run this test");

    sqlx::any::install_default_drivers();
    let pool = AnyPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect to HSIP_TEST_POSTGRES_URL");

    hsip_api::db::run_migrations(&pool)
        .await
        .expect("run_migrations must succeed against real Postgres");

    // A real millisecond-epoch timestamp, same magnitude `now_ms()` produces
    // — must NOT overflow (this is exactly what "INTEGER" used to do).
    let now = hsip_api::db::now_ms();
    assert!(
        now > i32::MAX as i64,
        "test is only meaningful if now_ms() exceeds int4 range"
    );

    sqlx::query("INSERT INTO tenants (id, name, created_at) VALUES ($1, $2, $3)")
        .bind("pg-compat-tenant")
        .bind("Postgres Compat Test")
        .bind(now)
        .execute(&pool)
        .await
        .expect("insert with a real epoch-ms timestamp must succeed");

    let row = sqlx::query("SELECT created_at FROM tenants WHERE id = $1")
        .bind("pg-compat-tenant")
        .fetch_one(&pool)
        .await
        .expect("select must succeed");
    let created_at: i64 = row.try_get(0).unwrap();
    assert_eq!(created_at, now);

    // BYTEA round-trip (the old schema used the SQLite/MySQL-only "BLOB"
    // keyword, which doesn't exist in Postgres).
    let blob = vec![0xDEu8, 0xAD, 0xBE, 0xEF, 0x00, 0xFF];
    sqlx::query(
        "INSERT INTO uploads (id, tenant_id, filename, content_type, data, size, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind("pg-compat-upload")
    .bind("pg-compat-tenant")
    .bind("test.bin")
    .bind("application/octet-stream")
    .bind(blob.clone())
    .bind(blob.len() as i64)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert of a BYTEA blob must succeed");

    let row = sqlx::query("SELECT data FROM uploads WHERE id = $1")
        .bind("pg-compat-upload")
        .fetch_one(&pool)
        .await
        .expect("select of a BYTEA blob must succeed");
    let read_back: Vec<u8> = row.try_get(0).unwrap();
    assert_eq!(read_back, blob);

    // Cleanup so the test is repeatable against a database the caller reuses.
    sqlx::query("DELETE FROM uploads WHERE id = $1")
        .bind("pg-compat-upload")
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind("pg-compat-tenant")
        .execute(&pool)
        .await
        .ok();
}
