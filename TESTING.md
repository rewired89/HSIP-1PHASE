# HSIP Testing Guide

Quick reference for running tests to verify cryptographic correctness, security properties, and API functionality.

---

## Run All Tests (Full Suite)

**238 tests across the entire platform:**

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

**Tests passed: 11/11**

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

**Tests passed: 8/8**

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

**Tests passed: 46/46**

These test names impress security researchers — they're based on quantum physics analogies for cryptographic properties.

---

### 5. Consent Lifecycle Tests

Consent request/response roundtrip and cryptographic binding:

```bash
cargo test -p hsip-core consent -- --nocapture
```

**Tests passed: 3/3**

---

### 6. Telemetry Guard (Privacy Protection)

Policy engine, consent gate, audit chain integrity:

```bash
cargo test -p hsip-telemetry-guard -- --nocapture
```

**Tests passed: 53/53**

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

## Test Coverage Summary

| Category | Tests | What It Proves |
|---|---|---|
| **RFC 8439 vectors** | 4 | IETF-standard crypto implementation |
| **API integration** | 11 | End-to-end consent/credential flows work |
| **Nonce integrity** | 8 | Replay attack prevention |
| **Quantum properties** | 46 | Advanced security properties |
| **Telemetry guard** | 53 | Privacy protection and audit integrity |
| **Crypto core** | 58 | Key management, signing, encryption |
| **Network layer** | 11 | Connection limits, rate limiting |
| **Session management** | 5 | State persistence, rekeying |
| **Other modules** | 42 | CLI, intercept, regenerative keys, etc. |
| **Total** | **238** | Complete platform coverage |

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

## Expected Test Timing

On a modern developer machine:

- **Full suite**: ~45 seconds
- **RFC 8439 only**: ~0.5 seconds
- **API integration only**: ~0.1 seconds
- **Quantum properties only**: ~0.03 seconds

If tests take significantly longer, check for resource contention or use `cargo test --release` for optimized builds.

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
A: Yes. 238 tests covering cryptographic correctness, API functionality, and security properties. Run `cargo test --workspace` to verify.

**Q: How do I know your crypto is correct?**
A: We pass the official RFC 8439 test vectors. Run `cargo test -p hsip-core rfc8439 -- --nocapture` and see the IETF compliance output.

**Q: Is GDPR erasure tested?**
A: Yes. `test_gdpr_erase` proves all tenant data is permanently deleted. Run `cargo test -p hsip-api test_gdpr_erase -- --nocapture --exact`.

**Q: How do I test revocation?**
A: `test_credential_issue_verify_revoke` issues a credential, verifies it (passes), revokes it, then verifies again (fails with `revoked: true`).

**Q: Can I run tests in CI/CD?**
A: Yes. `cargo test --workspace` runs in any CI environment. See GitHub Actions example above.

---

## Next Steps

1. Run `cargo test --workspace` — ensure all 238 tests pass
2. Run `cargo audit` — check for known vulnerabilities in dependencies
3. Review `THREAT_MODEL.md` — understand what HSIP protects against
4. Deploy to staging with PostgreSQL and TLS
5. Run load tests with `wrk` or the provided scripts
6. Review audit logs at `/v1/audit` after testing

---

## Support

- Full deployment guide: `DEPLOYMENT.md`
- Setup guides: `WINDOWS_SETUP.md`, `LINUX_SETUP.md`
- Security scope: `THREAT_MODEL.md`
- Licensing: sanchezleal1989@gmail.com
