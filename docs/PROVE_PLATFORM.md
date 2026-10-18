# HSIP platform

A multi-tenant service businesses use to **prove what their automated systems decided, and when**. Customers send a SHA-256 fingerprint of each decision (never the content). The platform signs it, chains it to the previous record, batches it into a signed Merkle tree, optionally commits the root to Bitcoin, and hands back a self-contained bundle anyone can verify offline.

| Part | Where | Notes |
|---|---|---|
| Proof format + verifier | `crates/hsip-proof` | Open, pure, no I/O. `hsip-proof verify file.json` |
| Platform server | `crates/hsip-platform` | Axum, SQLite (zero-config) or PostgreSQL |
| Console | `console/` | React + Vite, embedded into the binary for releases |
| SDKs | `sdk/prove-python`, `sdk/prove-node` | Dependency-free single files |
| Format spec | `docs/PROVE_PROOF_FORMAT.md` | Enough to reimplement the verifier |

The existing HSIP Phase 2 crates (`hsip-core` crypto, session, net, enterprise identity, etc.) are **not modified** by the platform; it depends only on `hsip-proof`.

## Run it

```bash
# Dev: API on :8080, console from console/dist
cd console && npm install && npm run build && cd ..
cargo run -p hsip-platform            # http://127.0.0.1:8080

# Console with hot reload (proxies /v1 to :8080)
cd console && npm run dev             # http://127.0.0.1:5173

# Release binary with the console embedded
cd console && npm ci && npm run build && cd ..
cargo build --release -p hsip-platform --features embed-console

# Docker + PostgreSQL
cp .env.platform.example .env         # set the secrets
docker compose -f docker-compose.platform.yml up -d --build
```

First run creates `./data/platform.db` and `./data/master.key` (mode 0600). **Back up the master key**: it encrypts every organization's signing key, and signing keys are unrecoverable without it.

## Configuration (environment)

| Variable | Default | Purpose |
|---|---|---|
| `HSP_BIND` / `PORT` | `127.0.0.1:8080` (`0.0.0.0:$PORT` if `PORT` set) | Listen address |
| `DATABASE_URL` | SQLite under `HSP_DATA_DIR` | `postgres://...` for production |
| `HSP_DATA_DIR` | `./data` | Master key file, SQLite file |
| `HSP_MASTER_KEY` | generated file | 64 hex chars; prefer this (from a secrets manager) in production |
| `HSP_OPERATOR_KEY` | unset | 24+ chars; enables `/v1/operator/*` and the `/operator` console. Unset = those routes 404 |
| `HSP_PUBLIC_URL` | derived from bind | Used in share and invite links |
| `HSP_SIGNUP` | `true` | `false` = invitation-only |
| `HSP_OTS` | `false` | Commit batch roots to Bitcoin through OpenTimestamps calendars |
| `HSP_ANCHOR_INTERVAL_SECS` / `HSP_ANCHOR_BATCH` | `60` / `500` | Anchor when the oldest unanchored record is this old, or this many are waiting |
| `HSP_CONSOLE_DIR` | `console/dist` | Console files when not embedded |

## Product model

- **Organization**: a customer. Owns one Ed25519 signing key (encrypted at rest), a plan, members, streams, keys.
- **Stream**: one independent hash chain, normally one system or workflow. A hidden `system` stream per organization records the platform's own audit trail (sign-ins, key and team changes, exports) as signed, chained proofs.
- **Roles**: `owner`, `admin` (manage streams, keys, people), `developer` (create streams, record, read), `auditor` (read-only, can export evidence). Non-members get 404, not 403.
- **API keys**: `hsp_...`, scope `ingest` or `read`, optionally bound to one stream. Stored as SHA-256 only; shown once.
- **Plans**: `free`, `team`, `business`, `enterprise` (`crates/hsip-platform/src/plans.rs`). Limits on proofs/month, streams, seats and evidence-pack size are enforced by the API (HTTP 402). **Prices there are placeholders; set them before selling.** There is no payment processor: customers request a plan in the console and the operator applies it (`/operator`). Wire Stripe or similar into `routes/operator.rs::set_plan` when ready.

## Prospects and enterprise services

- **Every free sign-up is a prospect.** Signing up records a `signup` lead (email, company) and the operator console (`/operator`, needs `HSP_OPERATOR_KEY`) lists every organization with its owner's email, plan, proofs this month and in total, last proof time and member count. Filter to "Free plan only" and download a CSV to plan offers. Usage is the signal: a free org near its limit, or one that exported evidence, is the natural upgrade conversation.
- **Enterprise page** (`/enterprise` in the console, signed-in users of any plan; also the landing page's Enterprise section) lists the service catalogue in `plans.rs::SERVICES`, each marked `available`, `on_request` or `roadmap` so nothing planned is sold as shipped. Admins and owners click **Request this** / **Register interest**; it becomes a `service_request` lead in the operator console. Nothing is provisioned automatically.
- Plan requests from the Plan & usage page arrive as `plan_request` leads. The operator applies the plan from the customer table.
- To preview the Enterprise experience on your own account, open `/operator`, enter the operator key, and switch your organization's plan to `enterprise`.

## API summary

Machine API (Bearer `hsp_...`):

```
POST /v1/ingest            {stream?, actor, action, subject?, payload_hash, metadata?}   ingest key
POST /v1/ingest/batch      {stream?, items:[...]}  up to 100, recorded in order            ingest key
GET  /v1/proofs/:id        full bundle + verification                                      read key
```

Public (no auth): `POST /v1/verify` (bundle or evidence pack), `POST /v1/verify/match` (compare original content with a bundle), `GET /v1/public/shares/:token`, `GET /v1/public/orgs/:slug` (published key), `POST /v1/leads`, `GET /v1/meta` (plans and services), `GET /health`.

Console API (Bearer `hss_...` session, under `/v1/orgs/:org/...`): overview, usage, streams (+ integrity check), proofs (list with filters, detail, share), shares, api-keys, members, invites, evidence (create, list, pack, printable report), plan-request, service-request. Accounts: `/v1/auth/{signup,login,logout,me,invite/:token,accept-invite}`, `POST /v1/orgs`.

Operator (Bearer `HSP_OPERATOR_KEY`): `/v1/operator/{overview,leads,orgs/:id/plan,orgs/:id/status}`.

Errors are `{"error":{"code","message"}}`. Internal error text is logged, never returned.

## Security model

- Tokens (sessions, API keys, invites, share links) are 256-bit random and stored only as SHA-256. Passwords use Argon2id.
- Signing keys are ChaCha20-Poly1305 encrypted with an HKDF-derived key from the master key. The master key file is written 0600.
- Login (per email and per IP), sign-up, verification and share-view endpoints are rate limited; API keys are limited to 1,200 calls/minute. **Limits are in-memory per node**, so run behind a gateway for multi-node abuse protection.
- Request bodies are capped (2 MB default, 32 MB for `/v1/verify`). Responses carry `nosniff`, frame-deny, no-referrer and a CSP. The printable evidence report is HTML-escaped and carries its own restrictive CSP.
- Record content never reaches the platform. `actor`, `action`, `subject` and `metadata` **are stored in clear** (they power search); do not put secrets there.

### Honest limits

1. **The platform holds each organization's signing key** (custodial). A compromised or malicious platform operator could sign new records. What bounds this: customers receive the signed bundle at ingest and can keep it, so any later server-side alteration fails against their copy; and with `HSP_OTS=true` batch roots are committed to Bitcoin, so history cannot be rewritten past the confirmation. Customer-held signing keys (BYOK or client-side signing) are the planned next step for customers who cannot accept custody.
2. **Without `HSP_OTS`, "time" is only as trustworthy as the operator**: the anchor key is the platform's own.
3. **Low-entropy records are guessable from their hash.** If a record is just `"approve"`, anyone can hash candidates. Include a random salt inside the hashed record (the SDKs and samples do) and keep the salt with the original.
4. A proof shows a record existed, unaltered, at a time. It does not show the decision was right, and only covers what the customer chooses to send.
5. No master-key rotation yet. No outbound email (invites are links the inviter shares). Rate limits are per node.

## Operations

- Health: `GET /health` (checks the database). Anchoring runs in-process. Several nodes can share one `DATABASE_URL` safely: batches claim rows atomically and the anchor identity lives in the database.
- Back up: the database **and** `HSP_MASTER_KEY`. Restoring one without the other loses the ability to sign.
- Independent audit of integrity: console "Check integrity" on a stream recomputes the chain from the database; an evidence pack plus `hsip-proof verify` does it off-platform.
- Throughput: records in one stream serialize on that stream's chain (contention returns 409 after retries). Use one stream per independent system and batch calls.

## Tests

```bash
cargo test -p hsip-proof -p hsip-platform                    # 30 tests, SQLite in memory
HSP_TEST_POSTGRES_URL=postgres://postgres:pw@127.0.0.1:5432/postgres \
  cargo test -p hsip-platform --test postgres_compat -- --ignored   # real PostgreSQL
```

## Roadmap (not built)

Customer-held signing keys; payment processor integration; outbound email and webhooks; SSO/SAML and SCIM; master-key rotation; per-key IP allow-lists; data export and account deletion tooling; a hosted verifier widget for embedding on customer sites.
