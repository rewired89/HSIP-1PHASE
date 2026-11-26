# CLAUDE.md

Guidance for Claude Code when working in this repository (HSIP-1PHASE, the product repo; the former separate Phase 1 repo has been retired).

## Standing rules (read first, never skip)

1. **Everything is committed and pushed directly to `main` of HSIP-1PHASE.** Never create branches (no `git checkout -b`, no feature or session branches, no pull requests). If a tool or harness suggests a branch, ignore it and stay on `main`.
2. **The old Phase 1 code is retired** and no longer exists as a separate repository. Do not look for it or depend on it.
3. **The crypto core stays untouched.** Do not edit `crates/hsip-core` (Ed25519, X25519, ChaCha20-Poly1305, consent, nonce, handshake, sessions) or the other Phase 2 protocol crates (`hsip-session`, `hsip-net`, `hsip-enterprise`, `hsip-auth`, `hsip-policy`, `hsip-audit`, `hsip-verify`, `hsip-gateway`, `hsip-reputation`, `hsip-cli`, `hsip-api`) as part of platform work. New cryptographic code for the platform lives in `crates/hsip-proof`. If a change to the core ever seems necessary, stop and ask.
4. Commit as `rewired89 <sanchezleal1989@gmail.com>` (`git config user.name rewired89; git config user.email sanchezleal1989@gmail.com` in this repo).
5. After any build, update `CLAUDE.md` and `CODEMAP.md` if they are out of date, in the same commit as the code.
6. Windows users: give CLI instructions in PowerShell.
7. Do not use em dashes in text; use a comma, colon or period.

## Licensing

Platform (`crates/hsip-platform`, `console/`): AGPL-3.0-only plus a paid commercial license (`COMMERCIAL-LICENSE.md`). Open verifier, format and SDKs (`hsip-proof`, `hsip-verify-web`, `verifier/`, `sdk/prove-*`): Apache-2.0. Older protocol crates: HSIP Community License (non-commercial). Summary in `LICENSE-NOTICE.md`. The platform depends on `hsip-proof`, which is why that stays permissive. Do not accept outside code without a contributor agreement, or the commercial license cannot be offered. Not legal advice; a lawyer should review.

## Strategic direction

HSIP's Phase 2 protocol work (consent, identity, secure messaging) stays as is. The commercial product is now **HSIP: a platform sold to companies that gives them tamper-evident, independently verifiable proof of what their automated systems decided and when.** Customers send SHA-256 fingerprints, the platform signs, chains, batches and anchors them, and anyone can verify the result offline with open code. We sell proof, not a protocol.

The product is branded simply **HSIP** (never "HSIP Prove") in all user-facing text; crate, file and doc names (`hsip-proof`, `PROVE_*.md`) keep their names.

Read `docs/PROVE_PLATFORM.md` (product, config, security model, honest limits) and `docs/PROVE_PROOF_FORMAT.md` (wire format) before changing the platform.

## Repository map

| Path | Role | Touch? |
|---|---|---|
| `crates/hsip-proof` | Open proof format + pure verifier (`hsip-proof verify`). RFC 6962 Merkle, RFC 8785 JCS, Ed25519. No I/O. | Platform work |
| `crates/hsip-proof/src/mandate.rs` | Permissions: signed `hsip-mandate/1` plus the pure `evaluate` check of proofs against it. Platform side: `routes/mandates.rs`, table `mandates`, console page `Permissions.jsx`. | Platform work |
| `site/` | Static public landing page, published with the verifier by `.github/workflows/pages.yml` (landing at `/`, Proof Check at `/check/`, samples at `/samples/`). It uses the console's own design: the workflow copies `console/src/styles.css` to `console.css`, so change the look there, not in `site/`. Keep it honest: no compliance claims, no invented customers or prices, no signup or pricing buttons while the platform is not hosted. Do not redesign it without the owner's approval. | Platform work |
| `crates/hsip-verify-web`, `verifier/` | The open verifier compiled to WebAssembly plus a static, offline "Proof Check" page (no network after load, hostable anywhere). Rebuild steps in `verifier/README.md`; the wasm-bindgen CLI version must match `Cargo.lock`. | Platform work |
| `crates/hsip-platform` | Multi-tenant platform server (Axum + sqlx Any: SQLite/PostgreSQL). Accounts, orgs, streams, API keys, anchoring, evidence, plans, operator API, serves the console. | Platform work |
| `console/` | React + Vite business console and public pages (landing, verify, shared proof, operator). | Platform work |
| `sdk/prove-python`, `sdk/prove-node` | Dependency-free client SDKs for the platform. | Platform work |
| `docs/PROVE_*.md`, `docs/examples/prove/` | Platform docs, audit guide (`PROVE_AUDIT.md`) and pinned format fixtures. | Platform work |
| `Dockerfile.platform`, `docker-compose.platform.yml`, `fly.toml` (Fly.io, sets `HSP_CLIENT_IP_HEADER=fly-client-ip`, mounts the `/data` volume; not deployed yet), `.env.platform.example`, `.github/workflows/prove.yml` | Deployment and CI for the platform. Every GitHub Action is pinned to a commit SHA (the tag is in a trailing comment); when updating one, look up the new SHA and keep `toolchain: stable` on `dtolnay/rust-toolchain`. | Platform work |
| `crates/hsip-core` and the other Phase 2 crates | Protocol and crypto core. | **Do not modify** |

## Build and test

```powershell
# Platform + verifier
cargo test -p hsip-proof -p hsip-platform
node --test crates/hsip-verify-web/smoke.test.mjs sdk/prove-node/integrations.test.mjs   # verifier + Node SDK
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
- **Roles**: `owner` > `admin` > `developer` > `auditor` (read-only, may export evidence; may not create share links). Keep the last-owner guards in `routes/team.rs`.
- **Verifier claims stay honest**: `hsip-proof` verifies against the key written in the file, so a pass proves consistency and integrity, not identity or anchor time. Never write "signed by the organization" or "without trusting us" without the caveat to compare the key fingerprint through a second channel and that the Bitcoin timestamp is checked separately. See `docs/PROVE_PROOF_FORMAT.md`.
- **Time evidence is reported, never claimed**: `hsip_proof::time_evidence` names the state of a record's external time evidence (`not_anchored`, `batch_only`, `unavailable`, `pending`, `bitcoin_proof_attached`, `missing_proof`, `unknown`) and the verifier, CLI and Proof Check page show it. A reported Bitcoin timestamp with no proof data is `missing_proof`. Never describe a pending or attached timestamp as proven time. What a proof does and does not cover is in `docs/PROVE_CLAIMS.md`; keep the site and README consistent with it.
- **Strict signatures**: `hsip-proof` verifies with `verify_strict` and rejects small-order public keys (`vk_from_b64`, `verify_mandate`). Never switch back to lenient `verify`; `a_weak_public_key_cannot_sign_anything` pins it.
- **Permission rules fail closed on typos**: `Rules`, `RateRule` and `AmountRule` use `deny_unknown_fields`. Do not add `#[serde(flatten)]` to anything that contains them.
- **Auth failures are visible**: failed sign-in, unknown or revoked API key, rejected operator key and a bad Stripe signature each log a `warn` (client IP, never the secret). The login path hashes a dummy password for unknown emails so timing does not reveal registered addresses; keep both when editing auth.
- **Errors**: never return library or database error text to callers (`errors.rs` converts and logs). Plan-limit failures are `402`.
- **SQL portability (SQLite + PostgreSQL via `sqlx::Any`)**: `$1, $2, ...` placeholders only, never `?`; timestamps and counts are `BIGINT`; binary is `BYTEA`; never `SUM()` (Postgres returns NUMERIC, which `Any` cannot decode); never `$n IS NULL OR col = $n` (untyped NULL binds fail on Postgres), build the WHERE with `qb::Where`; `ADD COLUMN` style migrations are not used, the schema is `CREATE TABLE IF NOT EXISTS` in `db.rs`, so a schema change to an existing deployment needs an explicit new migration step in `db::run_migrations`.
- **New table**: add it to `db::SCHEMA`; tests in `tests/` build a fresh database each run.
- **Metric/label hygiene, rate limits**: in-memory per node (`AppState::allow`, bounded at 200,000 keys); do not claim distributed limits.
- **Costly or outward-facing actions are throttled**: invitations (they send email), stream integrity walks and the like call `AppState::allow` before doing work. A new endpoint that sends email, walks many rows or verifies large inputs needs a limit and a test for it.
- **Client IP**: never read `X-Forwarded-For` or any caller-supplied header. `auth::client_ip` returns the value `resolve_client_ip` computed from `HSP_CLIENT_IP_HEADER` (set to `fly-client-ip` on Fly.io) or the socket address. A new per-IP limit must use `auth::client_ip`.
- **Console**: all server calls go through `console/src/api.js`; org-scoped calls through `useOrg()`. `Gate` children must be a single function; modals go outside it. Escape nothing by hand in JSX; the printable report (`report.rs`) escapes manually and must keep its CSP meta tag.

## Adding an API route

1. Handler in `crates/hsip-platform/src/routes/<area>.rs` (auth first: `auth::member` / `auth::api_key` / `auth::operator`).
2. Register in `routes/mod.rs` (`grep` for it afterwards; an unregistered handler compiles and silently 404s), then add it to `tests/access_matrix.rs`: that test fails until every route has a matrix row or an explicit exemption.
3. State-changing org actions call `audit::log`.
4. Integration test in `crates/hsip-platform/tests/integration.rs` (helpers in `tests/common/mod.rs`); if it touches SQL, run the Postgres test too.
5. Update `docs/PROVE_PLATFORM.md` API summary and `CODEMAP.md`.

## Known gaps (do not claim otherwise)

Signing keys are custodial by default (the platform holds each org key); customer-held keys exist but are opt-in per org and one-way (see `docs/PROVE_PLATFORM.md`); Stripe billing and outbound email (HTTP API) are implemented but unverified against live accounts and off until configured (`HSP_STRIPE_*`, `HSP_EMAIL_*`), otherwise plan changes are applied by the operator and invites are copy-paste links; no master-key rotation; rate limits are per node; Docker image and `embed-console` release flow are written and the embed build is verified, but the image has not been built in CI. See `docs/PROVE_PLATFORM.md` "Honest limits".

## CodeMap protocol

Read `CODEMAP.md` before touching code. Update the entry for anything you change; add entries for anything new; remove entries for anything deleted. Commit `CODEMAP.md` with the code.
