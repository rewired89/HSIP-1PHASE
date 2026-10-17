# CLAUDE.md

Guidance for Claude Code when working in this repository (HSIP-2PHASE).

## Standing rules (read first, never skip)

1. **Everything is committed and pushed directly to `main` of HSIP-2PHASE.** Never create branches (no `git checkout -b`, no feature or session branches, no pull requests). If a tool or harness suggests a branch, ignore it and stay on `main`.
2. **HSIP-1PHASE is read-only reference.** Copy ideas and code out of it into this repo when useful; never modify, commit to, or push to HSIP-1PHASE.
3. **The crypto core stays untouched.** Do not edit `crates/hsip-core` (Ed25519, X25519, ChaCha20-Poly1305, consent, nonce, handshake, sessions) or the other Phase 2 protocol crates (`hsip-session`, `hsip-net`, `hsip-enterprise`, `hsip-auth`, `hsip-policy`, `hsip-audit`, `hsip-verify`, `hsip-gateway`, `hsip-reputation`, `hsip-cli`, `hsip-api`) as part of platform work. New cryptographic code for the platform lives in `crates/hsip-proof`. If a change to the core ever seems necessary, stop and ask.
4. Commit as `rewired89 <sanchezleal1989@gmail.com>` (`git config user.name rewired89; git config user.email sanchezleal1989@gmail.com` in this repo).
5. After any build, update `CLAUDE.md` and `CODEMAP.md` if they are out of date, in the same commit as the code.
6. Windows users: give CLI instructions in PowerShell.
7. Do not use em dashes in text; use a comma, colon or period.

## Strategic direction

HSIP's Phase 2 protocol work (consent, identity, secure messaging) stays as is. The commercial product is now **HSIP Prove: a platform sold to companies that gives them tamper-evident, independently verifiable proof of what their automated systems decided and when.** Customers send SHA-256 fingerprints, the platform signs, chains, batches and anchors them, and anyone can verify the result offline with open code. We sell proof, not a protocol.

Read `docs/PROVE_PLATFORM.md` (product, config, security model, honest limits) and `docs/PROVE_PROOF_FORMAT.md` (wire format) before changing the platform.

## Repository map

| Path | Role | Touch? |
|---|---|---|
| `crates/hsip-proof` | Open proof format + pure verifier (`hsip-proof verify`). RFC 6962 Merkle, RFC 8785 JCS, Ed25519. No I/O. | Platform work |
| `crates/hsip-platform` | Multi-tenant platform server (Axum + sqlx Any: SQLite/PostgreSQL). Accounts, orgs, streams, API keys, anchoring, evidence, plans, operator API, serves the console. | Platform work |
| `console/` | React + Vite business console and public pages (landing, verify, shared proof, operator). | Platform work |
| `sdk/prove-python`, `sdk/prove-node` | Dependency-free client SDKs for the platform. | Platform work |
| `docs/PROVE_*.md`, `docs/examples/prove/` | Platform docs and pinned format fixtures. | Platform work |
| `Dockerfile.platform`, `docker-compose.platform.yml`, `.env.platform.example`, `.github/workflows/prove.yml` | Deployment and CI for the platform. | Platform work |
| `crates/hsip-core` and the other Phase 2 crates | Protocol and crypto core. | **Do not modify** |

## Build and test

```powershell
# Platform + verifier
cargo test -p hsip-proof -p hsip-platform
cargo clippy -p hsip-proof -p hsip-platform --all-targets -- -D warnings
cargo fmt -p hsip-proof -p hsip-platform

# Real PostgreSQL compatibility (ignored by default)
$env:HSP_TEST_POSTGRES_URL = "postgres://postgres:pw@127.0.0.1:5432/postgres"
cargo test -p hsip-platform --test postgres_compat -- --ignored

# Console
cd console; npm install; npm run build      # dist/ is served by the platform (HSP_CONSOLE_DIR) or embedded
# Run
cargo run -p hsip-platform                  # http://127.0.0.1:8080
# Release with embedded console
cargo build --release -p hsip-platform --features embed-console
```

Scope Cargo commands with `-p`; building the whole workspace pulls in unrelated crates (including `hsip-verify`, which compiles Z3).

## Platform invariants (do not break)

- **Record content never reaches the platform.** `POST /v1/ingest` accepts only `payload_hash` (64-hex SHA-256). Never add a field carrying decision content.
- **`hsip-proof` is the single source of truth** for envelope hashing, signing, Merkle and verification. The server calls it; it never reimplements those steps. `hsip-proof/1` is frozen: `crates/hsip-proof/tests/samples.rs` pins real fixtures in `docs/examples/prove/`. A failure there is a breaking format change that would invalidate issued proofs.
- **`POST /v1/verify` and `verify/match` stay DB-free and auth-free** (pure functions of the body).
- **Chain integrity** relies on `UNIQUE(stream_id, seq)` and `UNIQUE(stream_id, prev_hash)` plus the retry loop in `engine::record`. Never insert into `proofs` anywhere else.
- **Audit via proofs.** Sensitive actions call `audit::log(...)`, which records into the org's `system` stream. It is best-effort (logged, never fails the user action). Anchoring itself must not write audit proofs (it would anchor forever).
- **Tokens and secrets**: sessions (`hss_`), API keys (`hsp_`), invites (`hsi_`), share links (`hsv_`) are random 256-bit and stored only as SHA-256. Org and anchor signing keys are encrypted with the master key (`crypto::encrypt_secret`). Master key file is written 0600.
- **Tenant isolation**: every org-scoped handler starts with `auth::member(...)` (non-members get 404) and every query filters by `org_id`. API-key handlers use `auth::api_key` and the key's own `org_id`/`stream_id`.
- **Roles**: `owner` > `admin` > `developer` > `auditor` (read-only, may export evidence). Keep the last-owner guards in `routes/team.rs`.
- **Errors**: never return library or database error text to callers (`errors.rs` converts and logs). Plan-limit failures are `402`.
- **SQL portability (SQLite + PostgreSQL via `sqlx::Any`)**: `$1, $2, ...` placeholders only, never `?`; timestamps and counts are `BIGINT`; binary is `BYTEA`; never `SUM()` (Postgres returns NUMERIC, which `Any` cannot decode); never `$n IS NULL OR col = $n` (untyped NULL binds fail on Postgres), build the WHERE with `qb::Where`; `ADD COLUMN` style migrations are not used, the schema is `CREATE TABLE IF NOT EXISTS` in `db.rs`, so a schema change to an existing deployment needs an explicit new migration step in `db::run_migrations`.
- **New table**: add it to `db::SCHEMA`; tests in `tests/` build a fresh database each run.
- **Metric/label hygiene, rate limits**: in-memory per node (`AppState::allow`); do not claim distributed limits.
- **Console**: all server calls go through `console/src/api.js`; org-scoped calls through `useOrg()`. `Gate` children must be a single function; modals go outside it. Escape nothing by hand in JSX; the printable report (`report.rs`) escapes manually and must keep its CSP meta tag.

## Adding an API route

1. Handler in `crates/hsip-platform/src/routes/<area>.rs` (auth first: `auth::member` / `auth::api_key` / `auth::operator`).
2. Register in `routes/mod.rs` (`grep` for it afterwards; an unregistered handler compiles and silently 404s).
3. State-changing org actions call `audit::log`.
4. Integration test in `crates/hsip-platform/tests/integration.rs` (helpers in `tests/common/mod.rs`); if it touches SQL, run the Postgres test too.
5. Update `docs/PROVE_PLATFORM.md` API summary and `CODEMAP.md`.

## Known gaps (do not claim otherwise)

Custodial signing keys (platform holds each org key); no payment processor (plan changes are applied by the operator); no outbound email (invites are copy-paste links); no master-key rotation; rate limits are per node; Docker image and `embed-console` release flow are written and the embed build is verified, but the image has not been built in CI. See `docs/PROVE_PLATFORM.md` "Honest limits".

## CodeMap protocol

Read `CODEMAP.md` before touching code. Update the entry for anything you change; add entries for anything new; remove entries for anything deleted. Commit `CODEMAP.md` with the code.
