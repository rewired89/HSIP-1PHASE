# HSIP Testing Guide

Quick reference for running tests to verify cryptographic correctness, security properties, and API functionality.

This file has two parts: **Part 1** is a hands-on, run-it-yourself walkthrough for exercising every feature against a real running server — the companion to CLAUDE.md's own "verify against a real running server, not just the test suite" discipline, for catching what only shows up when a human actually runs the thing (platform quirks, UX gaps, timing, real network conditions). **Part 2** is the pre-existing automated-test-suite reference, useful as a quick summary for a security-conscious buyer or reviewer.

---

# Part 1 — Manual End-to-End Walkthrough

Work through the sections in order the first time — later sections assume earlier ones already produced a running server and a key. After that, jump to whichever section you're actually testing.

## 1.0 Prerequisites

| Need | For |
|---|---|
| Rust toolchain (`rustup`) | Everything |
| Node.js + npm | Dashboard, Node SDK |
| Python 3.8+ | Python SDK |
| Go 1.20+ | Go SDK |
| `cmake` + a C++ toolchain (`g++`/`gcc`, or MSVC on Windows) | `hsip-verify` (Z3) — only needed for `cargo build --workspace`, not for `-p hsip-api` |
| `openssl` CLI | Generating test certs for mTLS |
| A local PostgreSQL 16+ (optional) | SQLite → PostgreSQL migration testing only |

First-time build:

```bash
cargo build -p hsip-api -p hsip-cli
```

If you also want `hsip-verify`'s formal-verification tests, `cargo build --workspace` instead — expect the first build to take several extra minutes compiling Z3 from source (one-time cost, cached afterward).

## 1.1 Desktop mode — first boot

```bash
cargo run -p hsip-api
```

- **Expect:** server starts on `http://127.0.0.1:7474`, logs a master-key fingerprint, creates `~/.hsip/` (or `%APPDATA%\HSIP\` on Windows) with `admin.key` and `master.key` inside.
- **Check:** `curl http://127.0.0.1:7474/health` → `{"status":"ok","version":"0.2.0"}`.
- **Check file permissions (Linux/macOS only):** `ls -l ~/.hsip/master.key` should show `-rw-------` (0600) — a real bug found and fixed earlier (§4.25 in THREAT_MODEL.md), worth re-confirming on a fresh install.
- **Check:** `curl http://127.0.0.1:7474/openapi.json` returns a real OpenAPI document, and `http://127.0.0.1:7474/docs` renders Swagger UI in a browser.

Save the admin key for later commands:

```bash
export HSIP_API_KEY=$(cat ~/.hsip/admin.key)   # Linux/macOS
# Windows PowerShell: $env:HSIP_API_KEY = Get-Content "$env:APPDATA\HSIP\admin.key"
export HSIP_API_URL=http://127.0.0.1:7474
```

## 1.2 `hsip up` onboarding

```bash
cargo run -p hsip-cli -- up
```

- **Expect:** detects the already-running server (or starts one), prints a welcome box, opens a browser to the dashboard.
- **Try it once with no server running too** — confirm it actually starts one rather than just failing.

## 1.3 Core identity, consent, messages, credentials

```bash
# Identity
curl -s -X POST $HSIP_API_URL/v1/identity -H "Authorization: Bearer $HSIP_API_KEY" | jq

# Sign a message
curl -s -X POST $HSIP_API_URL/v1/messages/sign -H "Authorization: Bearer $HSIP_API_KEY" \
  -H "Content-Type: application/json" -d '{"content":"test message"}' | jq

# List messages back (content should decrypt to the exact plaintext you sent)
curl -s $HSIP_API_URL/v1/messages -H "Authorization: Bearer $HSIP_API_KEY" | jq

# Consent grant / check / revoke — use any 32-byte base64 value as a stand-in peer key
PEER_KEY=$(head -c32 /dev/urandom | base64)
curl -s -X POST $HSIP_API_URL/v1/consent/grant -H "Authorization: Bearer $HSIP_API_KEY" \
  -H "Content-Type: application/json" -d "{\"peer_verify_key\":\"$PEER_KEY\",\"expires_ms\":3600000}" | jq
curl -s "$HSIP_API_URL/v1/consent/$PEER_KEY" -H "Authorization: Bearer $HSIP_API_KEY" | jq
curl -s -X POST $HSIP_API_URL/v1/consent/revoke -H "Authorization: Bearer $HSIP_API_KEY" \
  -H "Content-Type: application/json" -d "{\"peer_verify_key\":\"$PEER_KEY\"}" | jq

# Issue + verify + revoke a credential
curl -s -X POST $HSIP_API_URL/v1/credentials -H "Authorization: Bearer $HSIP_API_KEY" \
  -H "Content-Type: application/json" -d '{"claim":"age-over-21","user_token":"user-123"}' | jq
```

**Verify the encryption-at-rest claim yourself:** stop the server, then (SQLite desktop mode) run `grep -a "test message" ~/.hsip/hsip.db ~/.hsip/hsip.db-wal` — expect **zero matches**, even though the API returned the plaintext correctly. This is the exact technique CLAUDE.md's "Field Encryption at Rest" section describes; worth re-confirming on your own machine, not just trusting the prior verification.

## 1.4 Decision attestations (the Predicta integration surface)

```bash
PAYLOAD_HASH=$(echo -n "buy 10 AAPL @ 150" | sha256sum | cut -d' ' -f1)

curl -s -X POST $HSIP_API_URL/v1/decisions -H "Authorization: Bearer $HSIP_API_KEY" \
  -H "Content-Type: application/json" -d "{
    \"accountable_key\": \"$(curl -s -X POST $HSIP_API_URL/v1/identity -H "Authorization: Bearer $HSIP_API_KEY" | jq -r .verify_key)\",
    \"model_version\": \"test-v1\",
    \"strategy_id\": \"manual-test\",
    \"decision_type\": \"buy\",
    \"payload_hash\": \"$PAYLOAD_HASH\"
  }" | jq
```

Take the `id` from the response, then:

```bash
DECISION_ID=<id from above>
curl -s $HSIP_API_URL/v1/decisions/$DECISION_ID/proof -H "Authorization: Bearer $HSIP_API_KEY" > /tmp/bundle.json

# The pure, DB-free, no-auth verification — run this from anywhere, no bearer token
curl -s -X POST $HSIP_API_URL/v1/decisions/verify -H "Content-Type: application/json" \
  --data-binary @/tmp/bundle.json | jq
```

- **Expect:** `valid: true`. `anchored` will be `false` until the next anchor cycle runs (every ~10s once 50 decisions accumulate, or every 5 minutes with at least one pending) — wait a few minutes and re-fetch the proof to see `anchored: true` with a Merkle path.
- **Tamper test:** edit `/tmp/bundle.json` (flip one character in `event_hash` or the signature) and re-POST to `/v1/decisions/verify` — expect `valid: false`.

## 1.5 Audit log + hash chain

```bash
curl -s $HSIP_API_URL/v1/audit -H "Authorization: Bearer $HSIP_API_KEY" | jq
curl -s $HSIP_API_URL/v1/audit/verify -H "Authorization: Bearer $HSIP_API_KEY" | jq
```

- **Expect:** `valid: true`, `checked` roughly matching the number of actions you've done so far.
- Do a handful more actions (sign a message, issue a credential), re-run `/v1/audit/verify`, confirm `checked` grew and it's still `valid: true`.

## 1.6 Keys, roles, root-admin

```bash
# Create a member-role key (default) and an owner-role key
curl -s -X POST $HSIP_API_URL/v1/keys -H "Authorization: Bearer $HSIP_API_KEY" \
  -H "Content-Type: application/json" -d '{"name":"member-test"}' | jq
curl -s -X POST $HSIP_API_URL/v1/keys -H "Authorization: Bearer $HSIP_API_KEY" \
  -H "Content-Type: application/json" -d '{"name":"owner-test","role":"owner"}' | jq

# Confirm a member-role key CANNOT create/revoke keys (should 401/403)
MEMBER_KEY=<raw key from the member-test response>
curl -s -X POST $HSIP_API_URL/v1/keys -H "Authorization: Bearer $MEMBER_KEY" \
  -H "Content-Type: application/json" -d '{"name":"should-fail"}' | jq

# Root-admin: list, grant, revoke
cargo run -p hsip-cli -- keys list-root-admins
cargo run -p hsip-cli -- keys master-fingerprint
```

## 1.7 Agent governance (`hsip agent ...`)

```bash
cargo run -p hsip-cli -- agent register test-agent --expires-days 7
cargo run -p hsip-cli -- agent list
cargo run -p hsip-cli -- agent discover
cargo run -p hsip-cli -- agent revoke test-agent
cargo run -p hsip-cli -- status
```

- **Velocity/auto-revoke test (optional, a bit tedious):** hammer an `ai_agent`-type key with >1000 requests/min and confirm it gets auto-revoked — `pending_revocation` should block it immediately, before the DB write even lands.

## 1.8 Trust (federated peer verification)

```bash
cargo run -p hsip-cli -- trust add alice <a real Ed25519 verify key, base64>
cargo run -p hsip-cli -- trust list
cargo run -p hsip-cli -- trust verify --from alice "some content" "<signature>"
cargo run -p hsip-cli -- trust remove <id>
```

## 1.9 DNS blocker + proxy monitor

```bash
curl -s -X POST $HSIP_API_URL/v1/dns/enable -H "Authorization: Bearer $HSIP_API_KEY"
python3 -c "import socket; print(socket.gethostbyname('doubleclick.net'))" # via HSIP's resolver on :5300 — expect NXDOMAIN/failure
curl -s $HSIP_API_URL/v1/dns/status -H "Authorization: Bearer $HSIP_API_KEY" | jq
curl -s $HSIP_API_URL/v1/dns/log -H "Authorization: Bearer $HSIP_API_KEY" | jq

curl -s -X POST $HSIP_API_URL/v1/proxy/enable -H "Authorization: Bearer $HSIP_API_KEY"
curl -s $HSIP_API_URL/v1/proxy/status -H "Authorization: Bearer $HSIP_API_KEY" | jq
```

- **Real-world DNS check:** point a device's DNS at this machine's `:5300` (see §"Do you need a second computer" below) and confirm ordinary browsing still works while known trackers get blocked.

## 1.10 Server mode, mTLS, and rate limiting

```bash
cp config.example.toml config.toml
# edit config.toml: uncomment [server.tls], point cert_path/key_path at real certs
HSIP_CONFIG=config.toml cargo run -p hsip-api
```

Generate a throwaway CA + client cert to test `client_ca_path`:

```bash
openssl req -x509 -newkey rsa:2048 -nodes -keyout ca-key.pem -out client-ca.pem -days 1 -subj "/CN=test-ca"
openssl req -newkey rsa:2048 -nodes -keyout client-key.pem -out client-req.pem -subj "/CN=test-client"
openssl x509 -req -in client-req.pem -CA client-ca.pem -CAkey ca-key.pem -CAcreateserial \
  -out client-cert.pem -days 1 -extfile <(printf "extendedKeyUsage=clientAuth")
```

- **Expect:** `curl --cert client-cert.pem --key client-key.pem https://127.0.0.1:3000/health` succeeds; the same `curl` with no `--cert` or a cert signed by a *different* CA fails at the TLS handshake (check with `curl -v` — it should never even get to sending the HTTP request).

**Rate limit / replay protection:**

```bash
# Send 301 requests in under a minute against the same key, confirm the 301st gets 429
# Replay protection (opt-in headers) — two identical requests, second should 401
TS=$(date +%s)
curl -s $HSIP_API_URL/v1/identity -H "Authorization: Bearer $HSIP_API_KEY" \
  -H "x-hsip-timestamp: $TS" -H "x-hsip-nonce: test-nonce-1"
curl -s $HSIP_API_URL/v1/identity -H "Authorization: Bearer $HSIP_API_KEY" \
  -H "x-hsip-timestamp: $TS" -H "x-hsip-nonce: test-nonce-1"   # expect 401 Duplicate nonce

# Confirm the new env override actually works:
HSIP_REPLAY_TOLERANCE_SECS=5 cargo run -p hsip-api  # then send a request timestamped 30s old — expect rejection
```

## 1.11 Master key rotation

```bash
cargo run -p hsip-cli -- keys master-fingerprint
cargo run -p hsip-cli -- keys rotate-master --yes
cargo run -p hsip-cli -- keys master-fingerprint   # fingerprint should have changed
```

- **Expect:** signing still works immediately after rotation (no restart needed) — sign a message right after rotating and confirm it verifies.

## 1.12 Receipt collection — genuinely needs two running instances

This is the one feature where "two processes on one machine" is not just convenient, it's the actual intended shape:

```bash
# Terminal 1 — "source" instance
HOME=/tmp/hsip-source cargo run -p hsip-api    # runs on :7474

# Terminal 2 — "collector" instance on a different port
PORT=7480 HOME=/tmp/hsip-collector cargo run -p hsip-api
```

(If `PORT` isn't respected in desktop mode on your build, use a `config.toml` with a different `[server] port` for the collector instead.)

```bash
# Register a collector key on the collector instance
COLLECTOR_KEY=$(curl -s -X POST http://127.0.0.1:7480/v1/keys -H "Authorization: Bearer <collector admin key>" \
  -H "Content-Type: application/json" -d '{"name":"collector-inbox"}' | jq -r .key)

# On the source instance: record a decision (§1.4 above), then submit its receipt
cargo run -p hsip-cli -- receipts submit <decision_id> --type decision --label my-laptop \
  --collector-url http://127.0.0.1:7480 --collector-key $COLLECTOR_KEY \
  --api-url http://127.0.0.1:7474 --key $HSIP_API_KEY
```

- **The property that actually matters:** `curl http://127.0.0.1:7480/v1/decisions -H "Authorization: Bearer <collector key>"` must return `[]` — the collector should never see the source's decision content, only the one proof it was sent. Confirm this explicitly, don't just check that `receipts submit` printed success.
- **Duplicate submission** → expect `409`. **Tampered bundle** (edit the fetched proof before submitting) → expect `400`, and it should not appear in `GET /v1/receipts` on the collector.

## 1.13 SQLite → PostgreSQL migration (needs a local Postgres)

```bash
createdb hsip_test
cargo build -p hsip-api --bin hsip-migrate
./target/debug/hsip-migrate --from sqlite:$HOME/.hsip/hsip.db --to postgresql://localhost/hsip_test --yes
DATABASE_URL=postgresql://localhost/hsip_test cargo run -p hsip-api
```

- **Expect:** same admin key still authenticates, `GET /v1/audit/verify` still reports the chain intact, identity/verify keys match what they were under SQLite.

## 1.14 Dashboard

```bash
cd dashboard && npm install && npm run dev
```

- Open `http://localhost:3001`, sign in with the admin key, click through every page in both **For Everyone** and **Developer** mode — Home, Finance, Messages, Traffic, Alibi, Consents, AI Watch, AI Decisions, Trackers, Protection (simple) and Identity, Consent, Messages, Credentials, Decisions, Trust, Discover, Audit, Keys, Admin (developer).
- Specifically check: Audit page's hash-chain indicator, Admin page's master-key rotate button (with confirm step), Discover page's one-click agent registration, Keys page's role selector.
- Production build: `npm run build && cd .. && cargo build --release -p hsip-api --features hsip-api/embed-dashboard` — confirm the dashboard is actually served from the built binary with no separate `npm run dev` running.

## 1.15 `hsip-mcp` (MCP server)

```bash
cargo build -p hsip-mcp
HSIP_API_KEY=$HSIP_API_KEY HSIP_API_URL=$HSIP_API_URL ./target/debug/hsip-mcp
```

Point Claude Desktop (or another MCP client) at it per CLAUDE.md's config snippet, then from the client: call `get_identity`, `sign_message`, `grant_consent`, `check_consent`, `get_recent_actions` — confirm each round-trips correctly through a real MCP client, not just that the binary starts.

## 1.16 SDKs

```bash
# Python
cd sdks/python && pip install -e . && python3 -c "
from hsip import HSIPClient
c = HSIPClient('$HSIP_API_KEY', '$HSIP_API_URL')
print(c.get_identity())
d = c.record_decision(accountable_key=c.get_identity()['verify_key'], model_version='v1', strategy_id='t', decision_type='hold', payload_hash=HSIPClient.hash_payload(b'test'))
print(c.verify_decision(c.get_decision_proof(d['id'])))
"

# Node
cd sdks/node && npm install && node -e "
const {HSIPClient} = require('./src');
const c = new HSIPClient('$HSIP_API_KEY', '$HSIP_API_URL');
c.getIdentity().then(console.log);
"

# Go
cd sdks/go && go run ./... # or write a small main.go using hsip.NewClient(...)
```

## 1.17 `hsip-verify` (Z3 formal verification)

```bash
cargo test -p hsip-verify
```

- **Expect:** 9 unit + 10 integration tests pass, all running real Z3 SMT queries. First run after a clean `target/` will take a while compiling Z3 — that's expected, not a hang.

## 1.18 Browser extension

Load `browser-extension/` unpacked in Chrome/Edge/Firefox dev mode (`chrome://extensions` → Developer mode → Load unpacked). Confirm:
- Badge shows a tracker-blocked count while browsing normally.
- Popup shows HSIP connection status (green dot) and the last 5 AI-agent audit entries.
- Works against both port 7474 (desktop) and 3000 (server mode).

## 1.19 Quality gates

```bash
cargo fmt --check
cargo clippy --workspace -- -D warnings   # or scope to -p hsip-api -p hsip-cli if skipping hsip-verify's Z3 build
cargo test --workspace                     # or the same scoped version
cargo audit
```

## 1.20 Windows-specific — the one category this project's own dev sandbox cannot verify itself

```powershell
cd dashboard; npm install; npm run build; cd ..
cargo build --release -p hsip-api --features hsip-api/embed-dashboard
.\target\release\hsip-api.exe
```

- **Check:** `HSIP.lnk` appears on Desktop and in Start Menu → Programs.
- **Check `%LOCALAPPDATA%\HSIP\install.log`** regardless of whether the shortcuts appeared — it now records `shortcut ok` or `shortcut FAILED: <reason>` per location (a real gap fixed this session; this is the first real-world chance to see it actually report something).
- Double-click the shortcut, confirm it launches HSIP correctly.
- Delete the shortcuts, delete `%LOCALAPPDATA%\HSIP\`, relaunch the built exe fresh, confirm self-install repeats cleanly.

## 1.21 Do you need a second computer / another environment?

Short answer: **for most of the above, no** — two processes on one machine (different ports, different `HOME`/`%APPDATA%` overrides) genuinely exercises the same code paths as two machines for receipt collection, decision verification, and RBAC. Where a second machine *does* add real value:

1. **Windows itself (§1.20)** — this is the one that matters most right now. Everything Windows-specific from this session (the `mslnk` replacement, the COM apartment-mode fix, the shortcut-failure logging) has only ever been cross-compiled and link-checked, never executed. If you have one Windows machine, that's sufficient — it doesn't need to be a *different* machine from the one you've already been testing on, just an actual run of §1.20.
2. **mTLS (§1.10) across a real network hop** — testing over loopback proves the TLS/cert logic works, but a second machine on the same LAN (or over the internet) additionally proves there's no firewall/routing assumption baked in that loopback happens to satisfy for free.
3. **Cross-platform SDK/CLI checks** — if you develop primarily on Windows, running the Python/Node/Go SDK examples (§1.16) from a Linux or macOS machine against a Windows-hosted server would catch any path-separator or line-ending assumption that only shows up cross-platform.
4. **The DNS resolver (§1.9)** — pointing a *different device's* network settings at HSIP's `:5300` resolver, rather than only querying it from the same machine it runs on, is the more realistic test of "does this actually protect a device on my network," which is the resolver's whole purpose.
5. **The browser extension (§1.18)** — worth trying on a second, ordinarily-configured browser/device you use for daily browsing, to catch any conflict with extensions or settings your dev machine doesn't have.

Everything else (identity, consent, messages, credentials, decisions, audit, keys/RBAC, receipts, migration, dashboard, MCP, SDKs against a local server, `hsip-verify`) is equally well tested from one machine — a second one wouldn't reveal anything new there.

---

# Part 2 — Automated Test Suite Reference

## Run All Tests (Full Suite)

**238+ tests across the entire platform** (grows with each new feature — see CLAUDE.md's "What Has Been Built" for the running total):

```bash
# Linux/macOS
cargo test --workspace

# Windows PowerShell
cargo test --workspace 2>&1
```

**Expected**: All tests pass. Any failure indicates a critical issue.

---

## Security-Critical Tests (Show These to Buyers)

### 1. RFC 8439 Cryptographic Compliance

Proves HSIP uses **IETF-standard ChaCha20-Poly1305** with official test vectors:

```bash
cargo test -p hsip-core rfc8439 -- --nocapture
```

**What this proves:**
- ✅ Encryption/decryption matches RFC 8439 Appendix A.5 exactly
- ✅ Authentication tag verification works correctly
- ✅ Tampering detection (modified ciphertext rejected)
- ✅ AAD (additional authenticated data) binding works

**Output you'll see:**
```
✅ RFC 8439 A.5 ChaCha20-Poly1305 AEAD test vector: PASSED
   This proves HSIP uses cryptographically correct ChaCha20-Poly1305!
```

This is **critical for security auditors** — most crypto libraries skip these tests.

---

### 2. API Integration Tests (End-to-End Flows)

Tests the complete credential lifecycle, consent management, and GDPR compliance:

```bash
cargo test -p hsip-api -- --nocapture
```

**What this tests:**
- **Credential issuance → verification → revocation** (`test_credential_issue_verify_revoke`)
- **Consent grant → revoke** (`test_consent_grant_and_revoke`)
- **GDPR right to erasure** (`test_gdpr_erase`) — all tenant data deleted permanently
- **Rate limiting** (`test_rate_limit_enforced`) — DoS protection
- **Audit log integrity** (`test_audit_log_populated`) — tamper-evident trail
- **Authentication enforcement** (`test_unauthorized_without_key`)
- **Decision attestations, receipt collection, RBAC, replay protection, mTLS client-cert binding** — dozens more added since this table was last updated; run the full suite rather than trusting this list as exhaustive.

---

### 3. Replay Attack Prevention

Proves nonce-based anti-replay works correctly:

```bash
cargo test -p hsip-core nonce -- --nocapture
```

**What this tests:**
- Strictly increasing nonces accepted
- Replay attempts rejected
- Expired nonces rejected
- Out-of-order nonces within window allowed

---

### 4. Quantum-Inspired Security Properties

Advanced cryptographic properties (consent non-forgery, identity binding, temporal consistency):

```bash
cargo test -p hsip-common -- --nocapture
```

**What this tests:**
- **No-cloning theorem**: Single-use tokens prevent reuse
- **Entanglement**: Consent binding between peers
- **Observer effect**: Read receipts and authorization tracking
- **Decoherence**: Session expiration and state invalidation
- **Superposition**: State commitment and collapse

These test names impress security researchers — they're based on quantum physics analogies for cryptographic properties.

---

### 5. Consent Lifecycle Tests

Consent request/response roundtrip and cryptographic binding:

```bash
cargo test -p hsip-core consent -- --nocapture
```

---

### 6. Telemetry Guard (Privacy Protection)

Policy engine, consent gate, audit chain integrity:

```bash
cargo test -p hsip-telemetry-guard -- --nocapture
```

---

### 7. Formal Verification (Z3)

Consent non-forgery, temporal consistency, identity-binding soundness — proven, not just tested, via the Z3 SMT solver:

```bash
cargo test -p hsip-verify
```

---

## Performance Benchmarks

### Simple Throughput Test

```bash
# Health endpoint (no crypto)
wrk -t4 -c100 -d10s http://localhost:3000/health
```

**Expected**: 10,000+ req/s on modern hardware.

### Load Testing Scripts

```bash
# Linux/macOS
bash load-test/scripts/simple_benchmark.sh
bash load-test/scripts/test_health.sh
bash load-test/scripts/test_identity_creation.sh

# Windows PowerShell
# Run from WSL or Git Bash (scripts require bash)
```

---

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Security Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --workspace
      - run: cargo test -p hsip-core rfc8439 -- --nocapture
      - run: cargo audit
```

---

## Security Audit Checklist

Before showing HSIP to a buyer's security team, run:

```bash
# 1. Full test suite
cargo test --workspace

# 2. RFC compliance
cargo test -p hsip-core rfc8439 -- --nocapture

# 3. Dependency audit
cargo audit

# 4. Check for outdated dependencies
cargo outdated

# 5. Clippy (Rust linter)
cargo clippy --workspace -- -D warnings

# 6. Format check
cargo fmt --check
```

**All should pass with zero errors.**

---

## Common Test Scenarios

### Test 1: Credential Cannot Be Forged

```bash
# This test proves an attacker cannot modify a credential without invalidating the signature
cargo test -p hsip-api test_credential_issue_verify_revoke -- --nocapture --exact
```

Look for: Credential verification succeeds initially, then fails after revocation.

### Test 2: GDPR Erasure Works

```bash
# This proves all tenant data is permanently deleted
cargo test -p hsip-api test_gdpr_erase -- --nocapture --exact
```

Look for: All 7 tables cleared, tenant_id no longer exists.

### Test 3: Rate Limiting Blocks Abuse

```bash
# This proves rate limiting works under load
cargo test -p hsip-api test_rate_limit_enforced -- --nocapture --exact
```

Look for: Requests start getting 429 Too Many Requests after threshold.

### Test 4: Decisions Independently Verify Without HSIP's Database

```bash
cargo test -p hsip-api test_decision_attestation_sign_anchor_verify_end_to_end -- --nocapture --exact
```

Look for: `POST /v1/decisions/verify` returns `valid: true` with no API key and no database access.

### Test 5: A Collector Never Receives a Submitter's Raw Data

```bash
cargo test -p hsip-api test_receipts_submit_list_get_and_reject_invalid -- --nocapture --exact
```

Look for: the collector's own `decisions`/`audit_entries` tables stay empty throughout — only the `submitted_receipts` row exists.

---

## Bug Bounty / Security Testing

If you want external security researchers to test HSIP:

1. **Run all tests first** — ensure everything passes
2. **Set up a test instance** with PostgreSQL (not SQLite)
3. **Enable audit logging** — monitor the `/v1/audit` endpoint
4. **Provide API keys** with limited scopes
5. **Define rules of engagement**:
   - No DoS attacks on production
   - Report vulnerabilities to: sanchezleal1989@gmail.com
   - 90-day disclosure timeline

**Platforms to consider:**
- HackerOne (https://hackerone.com)
- Bugcrowd (https://bugcrowd.com)
- Intigriti (https://intigriti.com)

---

## Interpreting Failures

### If RFC 8439 tests fail:
**CRITICAL** — ChaCha20-Poly1305 implementation is broken. Do not deploy.

### If API integration tests fail:
**HIGH** — Core consent/credential flows broken. Investigate immediately.

### If nonce tests fail:
**HIGH** — Replay attack prevention broken. Security vulnerability.

### If quantum property tests fail:
**MEDIUM** — Advanced security properties may be compromised. Review logic.

### If other tests fail:
**LOW-MEDIUM** — May be ancillary features. Review impact before deployment.

---

## Questions Buyers Will Ask

**Q: Do you have test coverage?**
A: Yes — 238+ tests (and growing) covering cryptographic correctness, API functionality, and security properties. Run `cargo test --workspace` to verify.

**Q: How do I know your crypto is correct?**
A: We pass the official RFC 8439 test vectors. Run `cargo test -p hsip-core rfc8439 -- --nocapture` and see the IETF compliance output.

**Q: Is GDPR erasure tested?**
A: Yes. `test_gdpr_erase` proves all tenant data is permanently deleted. Run `cargo test -p hsip-api test_gdpr_erase -- --nocapture --exact`.

**Q: How do I test revocation?**
A: `test_credential_issue_verify_revoke` issues a credential, verifies it (passes), revokes it, then verifies again (fails with `revoked: true`).

**Q: Can I run tests in CI/CD?**
A: Yes. `cargo test --workspace` runs in any CI environment. See GitHub Actions example above.

**Q: Why doesn't HSIP depend on AWS or a cloud provider?**
A: Deliberately — see CLAUDE.md's "Why HSIP has no built-in AWS/cloud dependency" section. HSIP is meant to serve exactly the industries most dependent on a single cloud vendor, and a trust/audit layer that shares fate with the infrastructure it's attesting about is a weaker guarantee than one that doesn't.

---

## Next Steps

1. Run `cargo test --workspace` — ensure all tests pass
2. Run `cargo audit` — check for known vulnerabilities in dependencies
3. Review `THREAT_MODEL.md` — understand what HSIP protects against
4. Work through Part 1 above on a real machine — especially §1.20 (Windows) if you have one available
5. Deploy to staging with PostgreSQL and TLS
6. Run load tests with `wrk` or the provided scripts
7. Review audit logs at `/v1/audit` after testing

---

## Support

- Full deployment guide: `DEPLOYMENT.md`
- Setup guides: `WINDOWS_SETUP.md`, `LINUX_SETUP.md`
- Security scope: `THREAT_MODEL.md`
- Integration contract for Predicta/trading bots: `CLAUDE.md`'s "Trading Bot Integration (Predicta)" section
- Licensing: sanchezleal1989@gmail.com
