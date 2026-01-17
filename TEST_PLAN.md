# HSIP Phase 1 Security Validation Test Plan

Last updated: 2026-01-16

This document contains executable tests to verify HSIP Phase 1 security claims.

Tests are divided into **IMMEDIATE** (runnable now with existing CLI) and **BLOCKED** (need additional tooling).

---

## Prerequisites

```bash
cd /home/user/HSIP-1PHASE-1
cargo build --release
export HSIP_CLI="./target/release/hsip-cli"
mkdir -p ~/hsip-test-evidence
```

---

## Test 1: Consent Enforcement

**Claim:** HSIP blocks communication without explicit consent.

### Test 1.1: Auto-Deny Without Consent (IMMEDIATE)

```bash
# Terminal 1: Bob listens, auto-denies all requests
$HSIP_CLI consent-listen \
  --addr 127.0.0.1:9001 \
  --decision deny \
  --ttl-ms 0 \
  > ~/hsip-test-evidence/bob-deny.log 2>&1 &
BOB_PID=$!

sleep 2

# Terminal 2: Alice attempts consent request
$HSIP_CLI consent-send-request \
  --to 127.0.0.1:9001 \
  2>&1 | tee ~/hsip-test-evidence/alice-denied.log

kill $BOB_PID
```

**Expected:**
- Alice receives consent denial or timeout
- No session established
- Bob's log contains `decision: deny` response

**Failure:**
- Session established despite denial
- No consent check occurred

**Evidence location:** `~/hsip-test-evidence/bob-deny.log`, `alice-denied.log`

---

### Test 1.2: Auto-Allow Enables Session (IMMEDIATE)

```bash
# Terminal 1: Bob auto-allows all requests
$HSIP_CLI consent-listen \
  --addr 127.0.0.1:9002 \
  --decision allow \
  --ttl-ms 30000 \
  > ~/hsip-test-evidence/bob-allow.log 2>&1 &
BOB_PID=$!

sleep 2

# Terminal 2: Session listener after consent
$HSIP_CLI session-listen \
  --addr 127.0.0.1:9003 \
  > ~/hsip-test-evidence/bob-session.log 2>&1 &
SESSION_PID=$!

sleep 2

# Terminal 3: Alice sends session traffic
$HSIP_CLI session-send \
  --to 127.0.0.1:9003 \
  --packets 5 \
  2>&1 | tee ~/hsip-test-evidence/alice-session.log

kill $SESSION_PID
kill $BOB_PID
```

**Expected:**
- 5 packets sent and received
- Bob's session log shows decrypted frames
- ChaCha20-Poly1305 AEAD encryption confirmed

**Failure:**
- Packets not delivered
- Plaintext transmission (no encryption)
- Session fails to establish

**Evidence location:** `~/hsip-test-evidence/bob-session.log`, `alice-session.log`

---

## Test 2: Rate Limiting

**Claim:** Per-IP rate limits prevent resource exhaustion.

### Test 2.1: Consent Request Rate Limit (IMMEDIATE)

```bash
# Bob listens
$HSIP_CLI consent-listen \
  --addr 127.0.0.1:9004 \
  --decision deny \
  > ~/hsip-test-evidence/rate-limit.log 2>&1 &
BOB_PID=$!

sleep 2

# Flood with 50 consent requests
for i in {1..50}; do
  $HSIP_CLI consent-send-request --to 127.0.0.1:9004 &
done
wait

sleep 1
kill $BOB_PID

# Check if rate limit was enforced
grep -c "rate" ~/hsip-test-evidence/rate-limit.log || echo "No rate limit messages found"
```

**Expected:**
- First ~30 requests processed
- Subsequent requests blocked with rate limit error
- Log contains "rate exceeded" or "consent request rate limit"

**Failure:**
- All 50 requests processed without rate limiting
- No error messages
- CPU usage spikes to 100%

**Evidence location:** `~/hsip-test-evidence/rate-limit.log`

**Measurement:**
```bash
# Count distinct "rate" messages
grep -i "rate" ~/hsip-test-evidence/rate-limit.log | wc -l
```

Should show >10 rate limit enforcements.

---

### Test 2.2: HELLO Size Limit (BLOCKED - needs raw packet tool)

**Status:** Cannot test without packet crafting tool.

**Requirements:**
- Tool to send oversized UDP packets with HSIP prefix
- Payload >1024 bytes (HELLO limit)

**Expected when implemented:**
- Oversized HELLO rejected before signature verification
- Error: "HELLO message too large: N > 1024"

---

## Test 3: Replay Protection

**Claim:** Old consent tokens cannot be reused.

### Test 3.1: Nonce Reuse Detection (BLOCKED - needs packet capture)

**Status:** Requires capturing and replaying consent responses.

**Requirements:**
- Packet sniffer (tcpdump/wireshark)
- Tool to replay captured packets

**Test procedure when ready:**
```bash
# Capture valid consent exchange
sudo tcpdump -i lo -w /tmp/consent.pcap port 9001 &
TCPDUMP_PID=$!

# Run legitimate consent flow
$HSIP_CLI consent-send-request --to 127.0.0.1:9001

sleep 2
kill $TCPDUMP_PID

# Replay captured response packet
tcpreplay -i lo /tmp/consent.pcap

# Expected: Second attempt rejected as replay
```

---

## Test 4: Session Rekey

**Claim:** Sessions automatically rekey after time or packet limits.

### Test 4.1: Packet Count Rekey (IMMEDIATE)

```bash
# Send >100,000 packets to trigger rekey
$HSIP_CLI session-listen --addr 127.0.0.1:9005 \
  > ~/hsip-test-evidence/rekey-test.log 2>&1 &
SESSION_PID=$!

sleep 2

$HSIP_CLI session-send \
  --to 127.0.0.1:9005 \
  --packets 100001 \
  2>&1 | tee ~/hsip-test-evidence/rekey-sender.log

kill $SESSION_PID

# Check for rekey event
grep -i "rekey" ~/hsip-test-evidence/rekey-test.log
```

**Expected:**
- Rekey triggered at packet ~100,000
- New ephemeral key exchange
- Session continues with new key

**Failure:**
- No rekey event
- Session continues indefinitely with same key
- Error or session drop at 100k packets

**Evidence location:** `~/hsip-test-evidence/rekey-test.log`

---

### Test 4.2: Time-Based Rekey (BLOCKED - requires 1 hour wait)

**Status:** Cannot run in reasonable test timeframe.

**Expected when implemented:**
- Session rekeys after 1 hour (MAX_SESSION_AGE)
- New key derived from fresh X25519 exchange

**Workaround for testing:**
- Modify MAX_SESSION_AGE to 60 seconds
- Rebuild and test

---

## Test 5: DoS Resistance

**Claim:** HSIP remains responsive under attack.

### Test 5.1: Garbage Packet Flood (IMMEDIATE)

```bash
# Start legitimate session
$HSIP_CLI session-listen --addr 127.0.0.1:9006 \
  > ~/hsip-test-evidence/dos-test.log 2>&1 &
SESSION_PID=$!

sleep 2

# Flood with garbage UDP packets
(
  for i in {1..1000}; do
    echo "GARBAGE$i" | nc -u -w0 127.0.0.1 9006
  done
) &
FLOOD_PID=$!

# Attempt legitimate traffic during flood
time $HSIP_CLI session-send \
  --to 127.0.0.1:9006 \
  --packets 10 \
  2>&1 | tee ~/hsip-test-evidence/dos-legitimate.log

kill $FLOOD_PID 2>/dev/null
kill $SESSION_PID
```

**Expected:**
- Legitimate session succeeds despite garbage flood
- Latency <1 second for 10 packets
- CPU usage <80%

**Failure:**
- Legitimate session times out
- CPU at 100%
- Process crash or OOM

**Evidence location:** `~/hsip-test-evidence/dos-test.log`, `dos-legitimate.log`

---

### Test 5.2: Bad Signature CPU Burn (BLOCKED - needs invalid sig generator)

**Status:** Requires tool to send packets with invalid Ed25519 signatures.

**Expected when implemented:**
- First 5 bad signatures verified and rejected
- Subsequent bad signatures blocked by rate limiter BEFORE verification
- CPU usage returns to normal after rate limit kicks in

---

## Test 6: Audit Log Integrity

**Claim:** Logs are tamper-evident.

### Test 6.1: Log File Existence (IMMEDIATE)

```bash
# Run consent listener to generate audit events
$HSIP_CLI consent-listen --addr 127.0.0.1:9007 --decision allow \
  > /dev/null 2>&1 &
BOB_PID=$!

sleep 2

# Generate some events
for i in {1..5}; do
  $HSIP_CLI consent-send-request --to 127.0.0.1:9007
  sleep 1
done

kill $BOB_PID

# Check for audit log or telemetry output
find ~/.hsip -name "*audit*" -o -name "*log*" 2>/dev/null
ls -la ~/.hsip/ 2>/dev/null || echo "No .hsip directory"
```

**Expected:**
- Audit log file exists in `~/.hsip/` or similar
- Contains entries for consent decisions

**Failure:**
- No audit log created
- Empty log file
- No .hsip directory

**Evidence location:** `~/.hsip/audit.json` (or wherever logs are stored)

---

### Test 6.2: Chain Verification (BLOCKED - needs audit-verify CLI)

**Status:** Audit verification not exposed in CLI.

**Requirements:**
- `hsip-cli audit-verify` command (not implemented)
- Programmatic access to `AuditTrail::verify_chain()`

**Test procedure when ready:**
```bash
$HSIP_CLI audit-verify ~/.hsip/audit.json

# Expected output:
# Chain valid: true
# Genesis hash: abc123...
# Head hash: def456...
# Entry count: 42
```

---

### Test 6.3: Tamper Detection (BLOCKED - needs audit-verify CLI)

**Status:** Same as 6.2.

**Test procedure when ready:**
```bash
# Modify an entry
jq '(.entries[3].decision = "Tampered")' ~/.hsip/audit.json > /tmp/tampered.json

# Verify tampered log
$HSIP_CLI audit-verify /tmp/tampered.json

# Expected output:
# Chain valid: false
# Error: Hash mismatch at entry 4
```

---

## Test 7: Identity and Signature Verification

**Claim:** Peer IDs are derived from public keys and cannot be forged.

### Test 7.1: Keygen and Identity (IMMEDIATE)

```bash
# Generate keypair
$HSIP_CLI keygen > ~/hsip-test-evidence/alice-key.json

# Extract peer ID and public key
PEER_ID=$(jq -r .peer_id ~/hsip-test-evidence/alice-key.json)
PUBKEY=$(jq -r .public_key ~/hsip-test-evidence/alice-key.json)

echo "Peer ID: $PEER_ID"
echo "Public Key: $PUBKEY"

# Verify peer ID is 32-byte hex (64 chars)
if [ ${#PEER_ID} -eq 64 ]; then
  echo "PASS: Peer ID is 64 hex chars (32 bytes)"
else
  echo "FAIL: Peer ID wrong length: ${#PEER_ID}"
fi
```

**Expected:**
- Peer ID is 64 hex characters
- Public key is 64 hex characters (Ed25519)
- Peer ID derived from public key (not random)

**Failure:**
- Peer ID != 64 chars
- Peer ID changes on repeated keygen with same key

**Evidence location:** `~/hsip-test-evidence/alice-key.json`

---

### Test 7.2: HELLO Signature Verification (IMMEDIATE)

```bash
# Initialize identity
$HSIP_CLI init

# Generate signed HELLO
$HSIP_CLI hello > ~/hsip-test-evidence/hello.json

# Check HELLO structure
jq . ~/hsip-test-evidence/hello.json

# Verify signature field exists
SIG=$(jq -r .signature ~/hsip-test-evidence/hello.json)
if [ -n "$SIG" ]; then
  echo "PASS: HELLO contains signature"
else
  echo "FAIL: No signature in HELLO"
fi
```

**Expected:**
- HELLO contains `signature` field (128 hex chars = 64 bytes)
- Contains `peer_id`, `protocol_version`, `capabilities`, `timestamp_ms`

**Failure:**
- Missing signature
- Signature wrong length
- Missing required fields

**Evidence location:** `~/hsip-test-evidence/hello.json`

---

## Test 8: Consent Revocation

**Claim:** Revoking consent terminates active sessions.

### Test 8.1: Mid-Session Revocation (BLOCKED - needs consent cache API)

**Status:** Requires CLI command to revoke consent during active session.

**Requirements:**
- `hsip-cli consent-revoke <peer_id>` command
- Active session management
- Real-time consent cache updates

**Test procedure when ready:**
```bash
# Establish session with consent
$HSIP_CLI session-listen --addr 127.0.0.1:9008 &
SESSION_PID=$!

# Send initial packets (should succeed)
$HSIP_CLI session-send --to 127.0.0.1:9008 --packets 5

# Revoke consent
$HSIP_CLI consent-revoke <peer_id>

# Attempt to send more packets (should fail)
$HSIP_CLI session-send --to 127.0.0.1:9008 --packets 5

# Expected: Second batch fails with ConsentRevoked error
```

---

## Summary: Test Status Matrix

| Test | Status | Blocks | Notes |
|------|--------|--------|-------|
| 1.1 Auto-Deny | **IMMEDIATE** | None | Run now |
| 1.2 Auto-Allow | **IMMEDIATE** | None | Run now |
| 2.1 Rate Limit | **IMMEDIATE** | None | Run now, check logs manually |
| 2.2 Size Limit | **BLOCKED** | Need packet tool | Low priority |
| 3.1 Replay | **BLOCKED** | Need pcap replay | Medium priority |
| 4.1 Packet Rekey | **IMMEDIATE** | None | Long-running (100k packets) |
| 4.2 Time Rekey | **BLOCKED** | 1 hour wait | Use modified constant |
| 5.1 DoS Flood | **IMMEDIATE** | None | Run now |
| 5.2 CPU Attack | **BLOCKED** | Need bad sig tool | Medium priority |
| 6.1 Log Exists | **IMMEDIATE** | None | Run now |
| 6.2 Chain Verify | **BLOCKED** | Need CLI command | High priority |
| 6.3 Tamper Detect | **BLOCKED** | Need CLI command | High priority |
| 7.1 Keygen | **IMMEDIATE** | None | Run now |
| 7.2 HELLO Sig | **IMMEDIATE** | None | Run now |
| 8.1 Revocation | **BLOCKED** | Need revoke API | High priority |

---

## Immediate Action Items

**Run these tests now:**

```bash
# Create evidence directory
mkdir -p ~/hsip-test-evidence

# Test 1.1
echo "=== Test 1.1: Auto-Deny ==="
./run_test_1_1.sh

# Test 1.2
echo "=== Test 1.2: Auto-Allow ==="
./run_test_1_2.sh

# Test 2.1
echo "=== Test 2.1: Rate Limit ==="
./run_test_2_1.sh

# Test 5.1
echo "=== Test 5.1: DoS Flood ==="
./run_test_5_1.sh

# Test 6.1
echo "=== Test 6.1: Log Exists ==="
./run_test_6_1.sh

# Test 7.1
echo "=== Test 7.1: Keygen ==="
./run_test_7_1.sh

# Test 7.2
echo "=== Test 7.2: HELLO Signature ==="
./run_test_7_2.sh
```

**Results:**
- **PASS**: Behavior matches expected
- **FAIL**: Behavior violates claim (document and fix)
- **INCONCLUSIVE**: Test ran but results unclear (need better instrumentation)

---

## Tooling Gaps (Priority Order)

1. **High Priority:**
   - `hsip-cli audit-verify <file>` - verify chain integrity
   - `hsip-cli audit-export <file>` - export with metadata
   - `hsip-cli consent-revoke <peer_id>` - mid-session revocation

2. **Medium Priority:**
   - Bad signature generator (for test 5.2, 8C)
   - Packet replay tool (for test 3.1, 6A)

3. **Low Priority:**
   - Oversized packet generator (for test 2.2)
   - Session hijack simulator (for test 6C)

---

## Evidence Collection

All test output goes to `~/hsip-test-evidence/` with timestamps:

```bash
# After running tests, package evidence
cd ~/hsip-test-evidence
tar czf hsip-test-results-$(date +%Y%m%d-%H%M%S).tar.gz *.log *.json
```

This archive can be shared with auditors or used as litigation evidence.

---

## Failure Response Protocol

If any IMMEDIATE test fails:

1. Document exact failure in `~/hsip-test-evidence/FAILURES.md`
2. Include full log output
3. File GitHub issue with "security-test-failure" label
4. Do not deploy until resolved

If BLOCKED test cannot be implemented:

1. Document missing tooling in `~/hsip-test-evidence/TOOLING_GAPS.md`
2. Prioritize based on security impact
3. Schedule for next sprint

---

## Next Steps

After running immediate tests:

1. Review evidence files
2. Document pass/fail in `TEST_RESULTS.md`
3. Implement high-priority tooling gaps
4. Re-run tests on next code changes
5. Expand test coverage as new features added

This is a living document. Update as protocol evolves.
