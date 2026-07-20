//! Copies an existing HSIP SQLite deployment's data into a PostgreSQL
//! database, so an operator can move from the desktop/small-deployment
//! default (SQLite) to Postgres without hand-rolling `INSERT`s.
//!
//! Usage:
//!   hsip-migrate --from sqlite:hsip.db --to postgresql://user:pass@host/db [--yes] [--force]
//!
//! What it does, in order:
//!   1. Connects to both databases.
//!   2. Creates the schema on the target via `hsip_api::db::run_migrations`
//!      (the exact same function the server itself calls at startup) — so
//!      the target schema can never drift from what the running server
//!      actually expects.
//!   3. Refuses to proceed if the target already has tenant rows, unless
//!      `--force` (protects against double-migrating into a live database).
//!   4. Copies every table's rows inside a single target-side transaction —
//!      if any table fails partway, nothing committed for this run.
//!   5. Verifies row counts match between source and target for every
//!      table, post-copy.
//!
//! Never writes to the source database.

use anyhow::{bail, Context, Result};
use sqlx::any::AnyPoolOptions;
use sqlx::{Any, AnyPool, Row, Transaction};
use std::io::{self, Write};

#[derive(Clone, Copy)]
enum Col {
    Text,
    OptText,
    Int,
    OptInt,
    Blob,
    OptBlob,
}

struct Table {
    name: &'static str,
    columns: &'static [(&'static str, Col)],
}

// Every table in db.rs::run_migrations, in a readable (not FK-required —
// this schema has no FOREIGN KEY constraints) order. Keep in sync with
// db.rs; a table added there without an entry here will simply not be
// migrated (and will show up as a row-count mismatch on the source side
// during a future migration if it's ever populated, since it would compare
// 0 rows copied vs N in source — but it won't, because it wouldn't be
// checked at all. See CLAUDE.md's "before adding new tables" guidance).
const TABLES: &[Table] = &[
    Table {
        name: "tenants",
        columns: &[
            ("id", Col::Text),
            ("name", Col::Text),
            ("created_at", Col::Int),
        ],
    },
    Table {
        name: "api_keys",
        columns: &[
            ("id", Col::Text),
            ("tenant_id", Col::Text),
            ("key_hash", Col::Text),
            ("name", Col::Text),
            ("agent_type", Col::Text),
            ("created_at", Col::Int),
            ("expires_at", Col::OptInt),
            ("active", Col::Int),
            ("role", Col::OptText),
            ("is_root_admin", Col::Int),
        ],
    },
    Table {
        name: "identities",
        columns: &[
            ("tenant_id", Col::Text),
            ("signing_key_b64", Col::Text),
            ("verify_key_b64", Col::Text),
            ("created_at", Col::Int),
        ],
    },
    Table {
        name: "consents",
        columns: &[
            ("id", Col::Text),
            ("tenant_id", Col::Text),
            ("peer_verify_key", Col::Text),
            ("status", Col::Text),
            ("granted_at", Col::OptInt),
            ("expires_ms", Col::OptInt),
            ("revoked_at", Col::OptInt),
            ("created_at", Col::Int),
            ("granted_by_key_type", Col::OptText),
        ],
    },
    Table {
        name: "messages",
        columns: &[
            ("id", Col::Text),
            ("tenant_id", Col::Text),
            ("peer_verify_key", Col::Text),
            ("direction", Col::Text),
            ("content", Col::Text),
            ("signature", Col::Text),
            ("timestamp", Col::Int),
            ("verified", Col::Int),
        ],
    },
    Table {
        name: "audit_entries",
        columns: &[
            ("id", Col::Text),
            ("tenant_id", Col::Text),
            ("action", Col::Text),
            ("peer_verify_key", Col::OptText),
            ("details", Col::OptText),
            ("timestamp", Col::Int),
            ("prev_hash", Col::OptText),
            ("entry_hash", Col::OptText),
            ("anchor_id", Col::OptText),
            ("merkle_index", Col::OptInt),
        ],
    },
    Table {
        name: "contacts",
        columns: &[
            ("id", Col::Text),
            ("tenant_id", Col::Text),
            ("nickname", Col::Text),
            ("verify_key", Col::Text),
            ("added_at", Col::Int),
        ],
    },
    Table {
        name: "trusted_peers",
        columns: &[
            ("id", Col::Text),
            ("tenant_id", Col::Text),
            ("label", Col::Text),
            ("verify_key", Col::Text),
            ("added_at", Col::Int),
        ],
    },
    Table {
        name: "credentials",
        columns: &[
            ("id", Col::Text),
            ("tenant_id", Col::Text),
            ("claim", Col::Text),
            ("user_token", Col::Text),
            ("issuer_verify_key", Col::Text),
            ("issued_at", Col::Int),
            ("expires_at", Col::Int),
            ("signature", Col::Text),
            ("revoked", Col::Int),
        ],
    },
    Table {
        name: "uploads",
        columns: &[
            ("id", Col::Text),
            ("tenant_id", Col::Text),
            ("filename", Col::Text),
            ("content_type", Col::Text),
            ("data", Col::Blob),
            ("size", Col::Int),
            ("created_at", Col::Int),
        ],
    },
    Table {
        name: "rate_limit_state",
        columns: &[
            ("kind", Col::Text),
            ("state_key", Col::Text),
            ("count", Col::Int),
            ("anomaly_count", Col::Int),
            ("window_start_ms", Col::Int),
            ("updated_at", Col::Int),
        ],
    },
    Table {
        name: "anchor_identity",
        columns: &[
            ("id", Col::Int),
            ("signing_key_b64", Col::Text),
            ("verify_key_b64", Col::Text),
            ("created_at", Col::Int),
        ],
    },
    Table {
        name: "decision_anchors",
        columns: &[
            ("id", Col::Text),
            ("merkle_root", Col::Text),
            ("leaf_count", Col::Int),
            ("anchor_signature", Col::Text),
            ("anchor_verify_key", Col::Text),
            ("ots_proof", Col::OptBlob),
            ("ots_status", Col::Text),
            ("created_at", Col::Int),
        ],
    },
    Table {
        name: "audit_anchors",
        columns: &[
            ("id", Col::Text),
            ("merkle_root", Col::Text),
            ("leaf_count", Col::Int),
            ("anchor_signature", Col::Text),
            ("anchor_verify_key", Col::Text),
            ("ots_proof", Col::OptBlob),
            ("ots_status", Col::Text),
            ("created_at", Col::Int),
        ],
    },
    Table {
        name: "decisions",
        columns: &[
            ("id", Col::Text),
            ("tenant_id", Col::Text),
            ("agent_key_id", Col::Text),
            ("accountable_key", Col::Text),
            ("model_version", Col::Text),
            ("strategy_id", Col::Text),
            ("decision_type", Col::Text),
            ("payload_hash", Col::Text),
            ("prev_hash", Col::Text),
            ("event_hash", Col::Text),
            ("signature", Col::Text),
            ("sign_algo", Col::Text),
            ("timestamp_iso", Col::Text),
            ("timestamp_int", Col::Text),
            ("hsip_gov_ext", Col::Text),
            ("anchor_id", Col::OptText),
            ("merkle_index", Col::OptInt),
            ("created_at", Col::Int),
        ],
    },
];

struct Opts {
    from: String,
    to: String,
    yes: bool,
    force: bool,
}

fn parse_args() -> Result<Opts> {
    let mut from = None;
    let mut to = None;
    let mut yes = false;
    let mut force = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--from" => from = Some(args.next().context("--from requires a value")?),
            "--to" => to = Some(args.next().context("--to requires a value")?),
            "--yes" | "-y" => yes = true,
            "--force" => force = true,
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => bail!("unrecognized argument: {other} (see --help)"),
        }
    }

    let from = from.context("--from <sqlite-url> is required (see --help)")?;
    let to = to.context("--to <postgres-url> is required (see --help)")?;
    Ok(Opts {
        from,
        to,
        yes,
        force,
    })
}

fn print_usage() {
    println!(
        "hsip-migrate — copy an HSIP SQLite database into PostgreSQL\n\n\
         USAGE:\n    \
         hsip-migrate --from <sqlite-url> --to <postgres-url> [--yes] [--force]\n\n\
         OPTIONS:\n    \
         --from <url>   Source, e.g. sqlite:hsip.db (required)\n    \
         --to <url>     Target, e.g. postgresql://user:pass@host/db (required)\n    \
         --yes, -y      Skip the interactive confirmation prompt\n    \
         --force        Proceed even if the target database already has data\n    \
         --help, -h     Show this message\n\n\
         Never writes to the source database. Creates the target schema\n\
         (idempotent) via the same migration function the server itself\n\
         uses, copies every table's rows inside one transaction, then\n\
         verifies row counts match on both sides."
    );
}

/// Redacts a `user:password@` userinfo segment from a connection URL before
/// it's ever printed — this tool's own stdout is not a safe place for a
/// database password to end up (terminal scrollback, CI logs, etc).
fn redact(url: &str) -> String {
    let Some(scheme_end) = url.find("://") else {
        return url.to_string();
    };
    let after_scheme = &url[scheme_end + 3..];
    let Some(at) = after_scheme.find('@') else {
        return url.to_string();
    };
    let userinfo = &after_scheme[..at];
    let Some(colon) = userinfo.find(':') else {
        return url.to_string();
    };
    let user = &userinfo[..colon];
    format!(
        "{}://{}:***@{}",
        &url[..scheme_end],
        user,
        &after_scheme[at + 1..]
    )
}

enum Val {
    Text(String),
    OptText(Option<String>),
    Int(i64),
    OptInt(Option<i64>),
    Blob(Vec<u8>),
    OptBlob(Option<Vec<u8>>),
}

fn extract(row: &sqlx::any::AnyRow, idx: usize, kind: Col) -> Result<Val> {
    Ok(match kind {
        Col::Text => Val::Text(row.try_get(idx)?),
        Col::OptText => Val::OptText(row.try_get(idx)?),
        Col::Int => Val::Int(row.try_get(idx)?),
        Col::OptInt => Val::OptInt(row.try_get(idx)?),
        Col::Blob => Val::Blob(row.try_get(idx)?),
        Col::OptBlob => Val::OptBlob(row.try_get(idx)?),
    })
}

async fn copy_table(
    source: &AnyPool,
    tx: &mut Transaction<'_, Any>,
    table: &Table,
) -> Result<usize> {
    let col_names: Vec<&str> = table.columns.iter().map(|c| c.0).collect();
    let select_sql = format!("SELECT {} FROM {}", col_names.join(", "), table.name);
    let rows = sqlx::query(&select_sql)
        .fetch_all(source)
        .await
        .with_context(|| format!("reading source table `{}`", table.name))?;

    if rows.is_empty() {
        return Ok(0);
    }

    let placeholders: Vec<String> = (1..=table.columns.len()).map(|i| format!("${i}")).collect();
    let insert_sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        table.name,
        col_names.join(", "),
        placeholders.join(", ")
    );

    let n = rows.len();
    for row in &rows {
        let mut q = sqlx::query(&insert_sql);
        for (idx, (_, kind)) in table.columns.iter().enumerate() {
            let val = extract(row, idx, *kind).with_context(|| {
                format!(
                    "reading column `{}` in table `{}`",
                    table.columns[idx].0, table.name
                )
            })?;
            q = match val {
                Val::Text(v) => q.bind(v),
                Val::OptText(v) => q.bind(v),
                Val::Int(v) => q.bind(v),
                Val::OptInt(v) => q.bind(v),
                Val::Blob(v) => q.bind(v),
                Val::OptBlob(v) => q.bind(v),
            };
        }
        q.execute(&mut **tx)
            .await
            .with_context(|| format!("inserting a row into `{}`", table.name))?;
    }
    Ok(n)
}

async fn row_count(pool: &AnyPool, table: &str) -> Result<i64> {
    let row = sqlx::query(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .with_context(|| format!("counting rows in `{table}`"))?;
    row.try_get::<i64, _>(0).context("reading COUNT(*) result")
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = parse_args()?;

    println!("HSIP SQLite → PostgreSQL migration tool");
    println!("  source: {}", redact(&opts.from));
    println!("  target: {}", redact(&opts.to));
    println!();

    if !opts.from.starts_with("sqlite") {
        bail!(
            "--from must be a sqlite:// URL (got: {})",
            redact(&opts.from)
        );
    }
    if !(opts.to.starts_with("postgres://") || opts.to.starts_with("postgresql://")) {
        bail!(
            "--to must be a postgres(ql):// URL (got: {})",
            redact(&opts.to)
        );
    }

    if !opts.yes {
        print!("This copies all data from the source SQLite database into the target PostgreSQL database. Continue? [y/N] ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .context("reading confirmation")?;
        if input.trim().to_lowercase() != "y" {
            println!("Aborted.");
            return Ok(());
        }
    }

    sqlx::any::install_default_drivers();

    let source = AnyPoolOptions::new()
        .max_connections(1)
        .connect(&opts.from)
        .await
        .context("connecting to source SQLite database")?;
    let target = AnyPoolOptions::new()
        .max_connections(5)
        .connect(&opts.to)
        .await
        .context("connecting to target PostgreSQL database")?;

    println!("Creating schema on target (idempotent — safe if tables already exist)...");
    hsip_api::db::run_migrations(&target)
        .await
        .context("creating schema on target")?;

    let existing_tenants = row_count(&target, "tenants").await?;
    if existing_tenants > 0 && !opts.force {
        bail!(
            "target database already has {existing_tenants} tenant row(s). Refusing to migrate \
             into a non-empty database without --force (would duplicate rows or fail on unique \
             constraints)."
        );
    }

    println!("Copying tables...");
    let mut tx = target
        .begin()
        .await
        .context("starting target transaction")?;
    for table in TABLES {
        let n = copy_table(&source, &mut tx, table)
            .await
            .with_context(|| format!("copying table `{}`", table.name))?;
        println!("  {:<20} {n} row(s)", table.name);
    }
    tx.commit().await.context("committing target transaction")?;

    println!();
    println!("Verifying row counts...");
    let mut mismatches = 0usize;
    for table in TABLES {
        let src = row_count(&source, table.name).await?;
        let tgt = row_count(&target, table.name).await?;
        if src != tgt {
            eprintln!("  MISMATCH {}: source={src} target={tgt}", table.name);
            mismatches += 1;
        }
    }

    if mismatches > 0 {
        bail!("{mismatches} table(s) had row count mismatches after migration — see above. The target transaction committed what it copied; investigate before pointing HSIP at it.");
    }

    println!("All {} tables verified — row counts match.", TABLES.len());
    println!();
    println!("Next steps:");
    println!(
        "  1. Point config.toml's [database] url at the PostgreSQL connection string used above."
    );
    println!("  2. Restart HSIP.");
    println!("  3. Confirm `GET /v1/audit/verify` reports a valid hash chain.");
    Ok(())
}
