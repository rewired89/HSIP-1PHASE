# CODEMAP

Entries cover the HSIP platform code. The Phase 2 protocol crates are untouched and are not mapped here. Format: **purpose**, `calls`, `called_by`, `mutates`.

## crates/hsip-proof (open format + verifier, no I/O)

- `src/lib.rs`
  - `Envelope` the signed statement (format `hsip-proof/1`). `canonical_bytes`/`event_hash` JCS + SHA-256. `sign_envelope(env, key)` returns `(event_hash_hex, signature_b64)`. called_by: `platform::engine::record`, tests.
  - `ProofBundle`, `AnchorBlock`, `ProofStepDto` wire types for a proof and its batch evidence.
  - `verify_bundle(&ProofBundle) -> Verification` checks content hash, signature, format, Merkle inclusion, anchor signature. called_by: `routes::{proofs,public}`, `evidence`, CLI.
  - `payload_matches(bundle, content)` SHA-256 compare of original content with `payload_hash`.
  - `PackManifest`, `EvidencePack`, `sign_manifest`, `manifest_hash`, `verify_pack` signed export: manifest hash/signature, manifest vs proofs, per-proof validity, single issuer, contiguous chain.
- `src/merkle.rs` RFC 6962 `MerkleTree` (`from_leaves`, `root`, `inclusion_proof`), `verify_inclusion`, `leaf_hash`/`node_hash`. called_by: `platform::anchor`, `engine::bundle_for`, `lib::verify_bundle`.
- `src/bin/hsip_proof.rs` `hsip-proof verify <file>` offline verifier for a bundle or pack; exit 0 valid, 1 invalid, 2 usage.
- `tests/samples.rs` pins `docs/examples/prove/*.json`; a failure is a breaking format change.

## crates/hsip-platform/src

- `main.rs` boots config, `AppState`, spawns `anchor::spawn`, serves `app(state)`.
- `lib.rs` `app(state)` = `routes::api` + console fallback (`ui::serve`) + security header layers.
- `config.rs` `Config::from_env` (all `HSP_*`, `DATABASE_URL`, `PORT`), `Config::for_tests`.
- `state.rs` `AppState` (pool, config, master key, anchor key, org key cache, throttle). `org_signing_key(org)` decrypts and caches; `allow(bucket, limit, window)` fixed-window limiter; `sweep_throttle_if_due`. Creates the master key file (0600) and the `anchor_identity` row. mutates: `anchor_identity`.
- `db.rs` `connect` (sqlx Any, runs `run_migrations`), `SCHEMA` (all tables), `now_ms`, `ms_to_iso`, `month_key`. Portability rules in header comment.
- `crypto.rs` `encrypt_secret`/`decrypt_secret` (ChaCha20-Poly1305, HKDF), `sha256_hex`, `random_token(prefix)`, `hash_password`/`verify_password` (Argon2id), `ct_eq`.
- `errors.rs` `ApiError` (400/401/402/403/404/409/429/500), converts sqlx/anyhow errors without leaking text.
- `auth.rs` `Role`, `bearer`, `session_user`, `member(state, headers, org)` (404 for non-members), `Member::{require_admin,require_write,require_owner,actor}`, `api_key` (+rate limit, touches `last_used_at`), `operator`, `client_ip`.
- `engine.rs` core. `record(state, NewProof)` validates, enforces org status + plan limit, signs with the org key, chains with retry on unique violation, bumps `usage_monthly`. mutates: `proofs`, `usage_monthly`. called_by: `routes::proofs`, `audit::log`. `bundle_for(db, row, org, cache)` rebuilds a bundle with Merkle path (`AnchorCache` per request). `prepare(state, NewProof)` and `commit(...)` implement customer-held keys (the customer signs `event_hash`; `commit` verifies it against `organizations.customer_verify_key`, checks the ±300s window and that seq/prev_hash equal the current tip, then inserts with `signer_key`). `record` refuses user streams for such orgs (409). `fetch_proof`, `org_info`, `month_usage`, `ProofRow`, `ProofSummary`, `PROOF_COLS`.
- `audit.rs` `log(state, org, actor, action, subject, detail)` best-effort proof into the org's system stream. called_by: most mutating routes.
- `anchor.rs` OpenTimestamps client (ported from the retired Phase 1 repo: `submit_digest_to`, `check_for_upgrade`, `extract_pending_calendar_uri`, `contains_bitcoin_attestation`), `run_cycle` (batch unanchored proofs into a Merkle tree, sign root with anchor key, optional OTS, atomic claim), `retry_unreachable`, `run_upgrade_cycle`, `spawn` (5 s due-check loop, 15 min upgrade loop). mutates: `anchors`, `proofs.anchor_id/merkle_index`.
- `qb.rs` `Where` builder (`eq_s`, `cmp_i`, `contains_any`, `raw`) and `bind_args` for dynamic filters on both backends.
- `plans.rs` `PLANS`, `get(id)`, `exists(id)`; limits and display prices (placeholders). `SERVICES` enterprise catalogue (`available`/`on_request`/`roadmap`), `service_exists`.
- `report.rs` `render(...)` printable HTML evidence report (escaped, CSP meta).
- `ui.rs` `serve(uri, dir)` console static files (embedded with `embed-console`, else from disk), SPA fallback, no path traversal.

### routes/ (all registered in `routes/mod.rs`)

- `account.rs` signup (also records a `signup` lead), login, logout, me, `create_org` (key + owner + system stream), invite preview/accept, new org. mutates: `users`, `sessions`, `organizations`, `memberships`, `streams`, `invites`.
- `orgs.rs` get org (+ usage, role), rename, `usage`, `overview` (stats, 24 h/30 d buckets, top actions/actors), `request_plan` (lead), `request_service` (`service_request` lead, admin only), `fingerprint(vk)`, `set_custody` (owner registers a customer public key after `verify_registration`; audit `org.custody_enabled`/`rotated`; `get` returns `custody`).
- `streams.rs` list (with stats), create (plan stream limit), update/archive (admin).
- `proofs.rs` `ingest`, `ingest_prepare`, `ingest_commit`, `ingest_batch`, `api_get` (API key); `console_record`, `list` (filters, paging), `get_one`, `create_share`, `list_shares`, `revoke_share`, `integrity` (recompute a stream's chain).
- `keys.rs` list/create/revoke API keys (admin). Raw key returned once.
- `team.rs` members list, invite (seat limit), revoke invite, set role, remove; last-owner guards.
- `evidence.rs` create (self-verifies before release, plan size cap), list, `pack` (JSON), `report_html`. mutates: `evidence_exports`.
- `public.rs` health, meta, `verify`, `match_content`, `view_share`, `org_key`, `lead`. No auth on verify.
- `operator.rs` overview (owner email, last proof, usage per org), `set_plan`, `set_status` (suspend), leads (`signup`, `contact`, `plan_request`, `service_request`). Gated by `HSP_OPERATOR_KEY`.

### tests/

- `common/mod.rs` helpers (`env`, `env_with`, `call`, `signup`, `stream`, `api_key`, `ingest`, `hash_of`).
- `integration.rs` 9 tests: signup rules, full record/anchor/verify/share/evidence flow, DB tampering detection, roles + isolation, key scopes, plan limits + operator, filters + overview, login throttle, concurrent chain safety.
- `postgres_compat.rs` same core flow on real PostgreSQL (ignored unless `HSP_TEST_POSTGRES_URL`).

## console/src (React + Vite)

- `main.jsx`, `App.jsx` router (public: landing, login, signup, invite, verify, shared proof, operator; protected: shell routes). `Shell.jsx` sidebar/topbar, theme toggle, error boundary.
- `api.js` fetch wrapper, token in localStorage, `request/api/qs`. `auth.jsx` `AuthProvider`, `useAuth`, `useOrg` (org-bound helpers, role flags). `ui.jsx` toasts, `useLoad`, `Gate`, `ErrorBoundary`, Card/Stat/Badge/Modal/Tabs/CopyField/CodeBlock/BarChart/Meter/Checks/Verdict. `format.js`, `icons.jsx`, `styles.css` (tokens, light/dark).
- `pages/`: `Landing` (+ `PublicFrame`, pricing from `/v1/meta`, lead form), `AuthPages`, `Overview`, `Streams`, `StreamDetail` (integrity, test proof), `Records` (`RecordsTable`), `ProofDetail`, `ProofView` (shared view + original-content check), `SharedProof`, `Verify`, `Evidence`, `ApiKeys`, `Quickstart`, `Team`, `Billing`, `Enterprise` (+ `ServiceGrid`; public route `/enterprise`, works signed out, linked from landing nav and login; `/operator` is the "Admin login"), `Activity`, `Settings`, `Operator` (prospects table, free-only filter, CSV export, leads).

## Other

- `crates/hsip-proof/src/mandate.rs` `Mandate`/`MandateBody`/`Rules`, `sign_mandate`, `verify_mandate`, `evaluate` (rules: allowed_actions, time window, max_per_period, max_amount on a metadata field, allowed_values; reports `Violation`s), `proofs_from_value`. CLI `hsip-proof mandate-sign`/`mandate-check`. `routes/mandates.rs` `create`, `list`, `revoke`, `report` (table `mandates`); console `pages/Permissions.jsx` (list, add a signed permission, check a stream, revoke; route `/permissions`); wasm `check_permission` powers the Proof Check page's permission box.
- `crates/hsip-verify-web/src/lib.rs` `verify(text)` (wasm-bindgen export) and `verify_text`: dispatch a pasted bundle or pack to `hsip_proof::verify_bundle`/`verify_pack`; pure, returns JSON. Built with `hsip-proof` default features off (no `keygen`/`rand`). `verifier/index.html` is the static page (CSP blocks all outside requests; samples embedded); `verifier/pkg/` is the generated wasm.
- `hsip-proof` also exposes customer-key helpers: `generate_signing_key`, `verify_key_b64`, `registration_digest`/`sign_registration`/`verify_registration`, `verify_hash_signature`, `check_prepared`; CLI `keygen`, `register-proof`. `PackManifest.issuer_keys` lists extra signer keys.
- `sdk/prove-python/hsip_prove.py`, `sdk/prove-node/index.mjs` ingest/read/verify clients; identical `hash_payload` output across languages for ASCII keys.
- `docs/PROVE_PLATFORM.md`, `docs/PROVE_PROOF_FORMAT.md`, `docs/examples/prove/` (real fixtures).
- `Dockerfile.platform`, `docker-compose.platform.yml`, `.env.platform.example`, `.github/workflows/prove.yml`.
