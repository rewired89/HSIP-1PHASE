# HSIP Testing Guide

This guide provides comprehensive testing commands to verify all HSIP Phase 1 features are working correctly. Use this to validate your HSIP installation and ensure all security features are operational.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Basic Installation Tests](#basic-installation-tests)
3. [Cryptography Tests](#cryptography-tests)
4. [Audit Log Tests](#audit-log-tests)
5. [Network Gateway Tests](#network-gateway-tests)
6. [Consent System Tests](#consent-system-tests)
7. [Security Feature Tests](#security-feature-tests)
8. [Integration Tests](#integration-tests)

---

## Prerequisites

### Required Software

```bash
# Check Rust installation
cargo --version

# Check PostgreSQL installation (for audit logs)
psql --version

# On Windows
Get-Service postgresql-x64-16
```

### Build HSIP with Full Features

**Linux/macOS:**
```bash
cargo build --release -p hsip-cli --features full
cargo build --release -p hsip-gateway
```

**Windows:**
```powershell
cargo build --release -p hsip-cli --features full
cargo build --release -p hsip-cli --bin hsip-tray --features full,tray
cargo build --release -p hsip-gateway
```

---

## Basic Installation Tests

### Test 1: CLI Help Command

```bash
./target/release/hsip-cli --help
```

**Expected Output:**
- Shows all available commands
- Lists: init, daemon, consent, audit-export, audit-verify, etc.

### Test 2: Version Information

```bash
./target/release/hsip-cli --version
```

**Expected Output:**
- Shows HSIP version (e.g., `hsip-cli 0.1.2`)

### Test 3: Initialize Identity

```bash
./target/release/hsip-cli init
```

**Expected Output:**
```
[HSIP] Initializing new identity...
[HSIP] Generated Ed25519 keypair
[HSIP] Identity saved to: ~/.hsip/identity.json
[HSIP] Public key: <hex string>
```

### Test 4: View Identity

```bash
./target/release/hsip-cli show-identity
```

**Expected Output:**
- Shows public key in hex format
- Displays identity file location

---

## Cryptography Tests

### Test 5: Ed25519 Signature Generation

```bash
echo "Test message" | ./target/release/hsip-cli sign
```

**Expected Output:**
- Displays hex-encoded Ed25519 signature
- Signature should be 128 hex characters (64 bytes)

### Test 6: Signature Verification

```bash
# Sign a message
echo "Test message" | ./target/release/hsip-cli sign > sig.txt

# Verify signature
echo "Test message" | ./target/release/hsip-cli verify --sig $(cat sig.txt)
```

**Expected Output:**
```
[HSIP] ✓ Signature valid
```

### Test 7: ChaCha20-Poly1305 Encryption

```bash
# Encrypt data
echo "Sensitive data" | ./target/release/hsip-cli encrypt --recipient <public_key> > encrypted.bin

# Decrypt data
cat encrypted.bin | ./target/release/hsip-cli decrypt
```

**Expected Output:**
- Original message: "Sensitive data"
- Ciphertext should be different each time (nonce randomization)

### Test 8: X25519 Key Exchange

```bash
# Generate ephemeral keypair
./target/release/hsip-cli keygen

# Perform key exchange
./target/release/hsip-cli dh --their-key <peer_public_key>
```

**Expected Output:**
- Shows shared secret (32 bytes hex)
- Secret should be consistent for same key pairs

---

## Audit Log Tests

### Prerequisites for Audit Tests

```bash
# Create PostgreSQL database
createdb hsip_audit

# Or on Windows:
& "C:\Program Files\PostgreSQL\16\bin\createdb.exe" hsip_audit

# Set database connection
export DATABASE_URL="postgresql://localhost/hsip_audit"
# Windows PowerShell:
$env:DATABASE_URL = "postgresql://localhost/hsip_audit"
```

### Test 9: Audit Log Initialization

```bash
./target/release/hsip-cli audit-init
```

**Expected Output:**
```
[AUDIT] Creating audit log tables...
[AUDIT] ✓ Tables created successfully
[AUDIT] ✓ Write-once triggers enabled
```

### Test 10: Record Audit Entry

```bash
./target/release/hsip-cli consent --destination example.com --allow
```

**Expected Output:**
```
[AUDIT] Recording consent decision...
[AUDIT] ✓ Audit entry created
```

### Test 11: Export Audit Logs

```bash
./target/release/hsip-cli audit-export --out test_audit.json
```

**Expected Output:**
- Creates `test_audit.json` file
- File contains JSON array of audit entries
- Each entry has: entry_id, timestamp, decision, destination, etc.

**Verify Export:**
```bash
cat test_audit.json | jq '.[0]'
```

### Test 12: Verify Audit Chain Integrity

```bash
./target/release/hsip-cli audit-verify
```

**Expected Output:**
```
[AUDIT] Verifying chain integrity...
[AUDIT] ✅ Chain integrity verified
[AUDIT] <N> entries checked - no tampering detected
```

### Test 13: Query Audit History

```bash
# Search by destination
./target/release/hsip-cli audit-query --destination "example.com"

# Search by decision type
./target/release/hsip-cli audit-query --decision "Block"

# Combined search with limit
./target/release/hsip-cli audit-query --destination "tracker" --decision "Block" --limit 50
```

**Expected Output:**
- Lists matching audit entries
- Shows timestamp, decision, destination, reason

### Test 14: Audit Statistics

```bash
./target/release/hsip-cli audit-stats
```

**Expected Output:**
```
[AUDIT] === Audit Log Statistics ===
[AUDIT] Total entries: <count>
[AUDIT] Chain integrity: ✅ Valid
[AUDIT] Database: PostgreSQL (write-once protected)
```

### Test 15: Attempt Tampering (Should Fail)

```bash
# Try to modify audit log directly
psql hsip_audit -c "UPDATE hsip_audit_log SET decision='Allow' WHERE id=1;"
```

**Expected Output:**
```
ERROR: Audit log entries are write-once (immutable)
```

---

## Network Gateway Tests

### Test 16: Start Gateway

```bash
./target/release/hsip-gateway &
```

**Expected Output:**
```
[gateway] Listening on 127.0.0.1:8080
[gateway] SOCKS5/HTTP proxy ready
```

### Test 17: Test HTTP Proxy

```bash
# Configure proxy
export http_proxy=http://127.0.0.1:8080
export https_proxy=http://127.0.0.1:8080

# Test request
curl -v http://example.com
```

**Expected Output:**
- Request goes through gateway
- Gateway logs show connection

### Test 18: Test SOCKS5 Proxy

```bash
curl --socks5 127.0.0.1:8080 http://example.com
```

**Expected Output:**
- Successful connection through SOCKS5
- Response from example.com

### Test 19: Gateway Consent Check

```bash
# Start daemon (manages consent)
./target/release/hsip-cli daemon &

# Make request through gateway
curl --proxy http://127.0.0.1:8080 http://tracker.example.com
```

**Expected Output:**
- Gateway checks with daemon for consent
- Blocks/allows based on user consent policy

---

## Consent System Tests

### Test 20: Grant Consent

```bash
./target/release/hsip-cli consent --destination "trusted.com" --allow
```

**Expected Output:**
```
[CONSENT] Granting consent for: trusted.com
[CONSENT] ✓ Consent recorded
[AUDIT] ✓ Audit entry created
```

### Test 21: Deny Consent

```bash
./target/release/hsip-cli consent --destination "tracker.com" --deny
```

**Expected Output:**
```
[CONSENT] Denying consent for: tracker.com
[CONSENT] ✓ Denial recorded
[AUDIT] ✓ Audit entry created
```

### Test 22: Query Consent Status

```bash
./target/release/hsip-cli consent-status --destination "trusted.com"
```

**Expected Output:**
```
[CONSENT] Status for trusted.com: Allowed
[CONSENT] Granted at: <timestamp>
```

### Test 23: Revoke Consent

```bash
./target/release/hsip-cli consent-revoke --destination "trusted.com"
```

**Expected Output:**
```
[CONSENT] Revoking consent for: trusted.com
[CONSENT] ✓ Consent revoked
[AUDIT] ✓ Revocation recorded
```

### Test 24: List All Consent Decisions

```bash
./target/release/hsip-cli consent-list
```

**Expected Output:**
- Lists all granted/denied consents
- Shows destinations, decisions, timestamps

---

## Security Feature Tests

### Test 25: NTP Time Synchronization

```bash
# Check if NTP sync is working (requires --features full)
./target/release/hsip-cli test-ntp
```

**Expected Output:**
```
[NTP] Synchronizing with pool.ntp.org...
[NTP] ✓ Time synchronized (offset: ±X ms)
[NTP] System time accuracy: within 2 seconds
```

### Test 26: Geolocation Metadata

```bash
# Test geolocation lookup (requires --features full)
./target/release/hsip-cli geolocate --ip 8.8.8.8
```

**Expected Output:**
```
[GEO] IP: 8.8.8.8
[GEO] Country: United States
[GEO] City: Mountain View
[GEO] ISP: Google LLC
```

### Test 27: Device Fingerprinting

```bash
./target/release/hsip-cli fingerprint
```

**Expected Output:**
```
[FINGERPRINT] OS: Linux/Windows/macOS
[FINGERPRINT] Architecture: x86_64
[FINGERPRINT] Hostname: <hostname>
[FINGERPRINT] User Agent: <agent>
```

### Test 28: HMAC Response Integrity

```bash
# Test HMAC signing of responses
./target/release/hsip-cli test-hmac --message "Test data"
```

**Expected Output:**
```
[HMAC] Message: Test data
[HMAC] HMAC-SHA256: <64 hex chars>
[HMAC] ✓ Integrity tag generated
```

### Test 29: Anti-Replay Protection

```bash
# Attempt to replay a consent request
./target/release/hsip-cli test-replay --nonce <old_nonce>
```

**Expected Output:**
```
[REPLAY] ❌ Nonce already used
[REPLAY] Replay attack detected and blocked
```

---

## Integration Tests

### Test 30: Full Consent Flow

```bash
# 1. Initialize identity
./target/release/hsip-cli init

# 2. Start daemon
./target/release/hsip-cli daemon &

# 3. Start gateway
./target/release/hsip-gateway &

# 4. Grant consent
./target/release/hsip-cli consent --destination "example.com" --allow

# 5. Make request through gateway
curl --proxy http://127.0.0.1:8080 http://example.com

# 6. Verify audit log
./target/release/hsip-cli audit-export --out full_test.json
./target/release/hsip-cli audit-verify
```

**Expected Output:**
- All steps complete successfully
- Audit log shows consent grant + request
- Chain verification passes

### Test 31: Multi-User Consent Exchange

```bash
# User A
./target/release/hsip-cli init --output user_a_identity.json

# User B
./target/release/hsip-cli init --output user_b_identity.json

# User A requests consent from User B
./target/release/hsip-cli consent-request \
    --their-key $(cat user_b_identity.json | jq -r '.public_key') \
    --purpose "Testing"

# User B grants consent to User A
./target/release/hsip-cli consent-grant \
    --their-key $(cat user_a_identity.json | jq -r '.public_key')
```

**Expected Output:**
- Consent request created with Ed25519 signature
- Consent grant recorded
- Both users have audit entries

### Test 32: Court Evidence Package Creation

```bash
# Create evidence package
mkdir court_evidence_$(date +%Y%m%d)
cd court_evidence_$(date +%Y%m%d)

# Export audit logs
../target/release/hsip-cli audit-export --out full_audit_log.json --limit 0

# Verify integrity
../target/release/hsip-cli audit-verify > chain_verification.txt

# Copy documentation
cp ../AUDIT_LOG_GUIDE.md .
cp ../README.md .

# Create package
tar -czf ../hsip_evidence_$(date +%Y%m%d).tar.gz .
```

**Expected Output:**
- Evidence package with:
  - Complete audit log (JSON)
  - Integrity verification proof
  - Documentation
- Ready for court submission

---

## Performance Tests

### Test 33: Encryption Throughput

```bash
# Generate 1MB test file
dd if=/dev/urandom of=test_1mb.bin bs=1M count=1

# Measure encryption speed
time ./target/release/hsip-cli encrypt --recipient <key> < test_1mb.bin > encrypted_1mb.bin
```

**Expected Output:**
- Should complete in < 1 second for 1MB
- ChaCha20 is very fast (~4GB/s on modern CPUs)

### Test 34: Audit Log Write Performance

```bash
# Write 1000 audit entries
for i in {1..1000}; do
    ./target/release/hsip-cli consent --destination "test$i.com" --allow
done

# Measure time
time ./target/release/hsip-cli audit-verify
```

**Expected Output:**
- 1000 entries should write in < 10 seconds
- Chain verification should complete in < 5 seconds

---

## Troubleshooting Tests

### Test 35: Database Connection

```bash
# Test PostgreSQL connection
psql -U postgres -d hsip_audit -c "SELECT COUNT(*) FROM hsip_audit_log;"
```

**Expected Output:**
- Shows count of audit entries
- No connection errors

### Test 36: Feature Verification

```bash
# Check if compiled with all features
./target/release/hsip-cli features
```

**Expected Output:**
```
[FEATURES] Compiled features:
  ✓ postgres      (Audit logs)
  ✓ ntp-sync      (Time sync)
  ✓ geolocation   (GeoIP)
  ✓ full          (All features)
```

### Test 37: Network Connectivity

```bash
# Test NTP connectivity
./target/release/hsip-cli test-ntp

# Test DNS resolution
./target/release/hsip-cli test-dns --domain example.com
```

**Expected Output:**
- NTP sync successful
- DNS resolution works

---

## Security Validation Tests

### Test 38: IETF RFC 8439 Test Vectors

```bash
# Run ChaCha20-Poly1305 test vectors
./target/release/hsip-cli test-chacha20-rfc8439
```

**Expected Output:**
```
[TEST] Running RFC 8439 test vectors...
[TEST] ✓ Test vector 1: PASS
[TEST] ✓ Test vector 2: PASS
[TEST] ✓ All test vectors passed
```

### Test 39: Ed25519 Test Vectors

```bash
# Run Ed25519 signature test vectors
./target/release/hsip-cli test-ed25519
```

**Expected Output:**
```
[TEST] Running Ed25519 test vectors...
[TEST] ✓ Signature generation: PASS
[TEST] ✓ Signature verification: PASS
```

### Test 40: Constant-Time Operations

```bash
# Verify constant-time crypto operations
./target/release/hsip-cli test-constant-time
```

**Expected Output:**
```
[TEST] Testing constant-time operations...
[TEST] ✓ Signature verification is constant-time
[TEST] ✓ Key comparison is constant-time
[TEST] ✓ No timing side-channels detected
```

---

## Quick Test Suite

Run all basic tests in sequence:

```bash
#!/bin/bash
# quick-test.sh

echo "Running HSIP Quick Test Suite..."

# Build
cargo build --release -p hsip-cli --features full
cargo build --release -p hsip-gateway

# Basic tests
./target/release/hsip-cli --version
./target/release/hsip-cli init

# Crypto tests
echo "Test" | ./target/release/hsip-cli sign

# Audit tests (requires PostgreSQL)
createdb hsip_test_$(date +%s)
export DATABASE_URL="postgresql://localhost/hsip_test_$(date +%s)"
./target/release/hsip-cli audit-init
./target/release/hsip-cli consent --destination "test.com" --allow
./target/release/hsip-cli audit-verify
./target/release/hsip-cli audit-export --out test.json

echo "✓ All quick tests passed!"
```

---

## Expected Test Results Summary

All tests should:
- ✅ Complete without errors
- ✅ Show expected output messages
- ✅ Cryptographic operations produce correct results
- ✅ Audit chain integrity verifies successfully
- ✅ Network operations complete within reasonable time
- ✅ Security features (NTP, geo, fingerprint) work if compiled with `--features full`

---

## Reporting Issues

If any test fails:

1. **Capture full output:**
   ```bash
   ./target/release/hsip-cli <command> 2>&1 | tee error.log
   ```

2. **Check build configuration:**
   ```bash
   cargo build --release --features full --verbose
   ```

3. **Verify dependencies:**
   ```bash
   cargo tree -p hsip-cli
   ```

4. **Submit issue with:**
   - Test number that failed
   - Full error output
   - OS and Rust version
   - Build command used

---

## Performance Benchmarks

Expected performance on modern hardware (2020+ CPU):

| Operation | Throughput | Latency |
|-----------|------------|---------|
| ChaCha20-Poly1305 encryption | ~4 GB/s | ~0.1 ms/MB |
| Ed25519 signing | ~10,000/s | ~0.1 ms |
| Ed25519 verification | ~5,000/s | ~0.2 ms |
| X25519 key exchange | ~20,000/s | ~0.05 ms |
| Audit log write | ~1,000/s | ~1 ms |
| Audit chain verify | ~10,000 entries/s | ~0.1 ms/entry |

---

## Next Steps

After all tests pass:

1. ✅ Build production installer (Windows)
2. ✅ Test installer on clean system
3. ✅ Verify auto-start functionality
4. ✅ Test tray icon (Windows only)
5. ✅ Create court evidence package
6. ✅ Deploy to users

---

**Last Updated:** January 14, 2026
**HSIP Version:** Phase 1 (v0.1.2)
**Test Coverage:** All core features
