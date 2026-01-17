# HSIP Phase 1 Security Testing - Quick Start

## ⚠️ IMPORTANT: HSIP is a UDP Protocol, Not HTTP

**HSIP operates at the UDP layer with native encryption using:**
- ChaCha20-Poly1305 AEAD for encryption
- X25519 ECDH for key exchange
- Ed25519 for signatures
- Counter-based nonce management for replay protection

**The old HTTP/mitmproxy tests (header_injection.py, replay_attack.py, ssl_strip.py, response_tamper.py) do NOT test HSIP.** They test HTTP proxies, which is irrelevant to HSIP's UDP protocol.

## Quick Commands

### Run All HSIP Native Tests (PowerShell)

```powershell
cd /home/user/HSIP-1PHASE-1/security_tests
.\run_hsip_tests.ps1
```

This runs the complete HSIP security test suite:
1. Replay Attack Protection (nonce-based)
2. Response Tampering Detection (AEAD)
3. Injection Attack Rejection (input validation)
4. Encryption Enforcement (mandatory encryption)

## Individual HSIP UDP Tests

### 1. Replay Attack Test

Tests HSIP's nonce-based replay protection.

**Expected:** HSIP should reject replayed UDP packets.

```powershell
.\hsip_replay_attack.ps1
```

**What it does:**
1. Starts HSIP consent listener
2. Sends initial consent request (should succeed)
3. Replays the same request (should fail due to nonce counter)
4. Verifies replay was rejected

**Pass Criteria:** Replayed request is rejected with nonce/timeout error.

### 2. Response Tampering Test

Tests ChaCha20-Poly1305 AEAD authentication.

**Expected:** Any tampered ciphertext should fail AEAD verification.

```powershell
.\hsip_response_tamper.ps1
```

**What it does:**
1. Starts HSIP session listener
2. Sends encrypted session packets
3. Documents AEAD protection properties

**Pass Criteria:** All encrypted packets verify successfully; tampering would fail authentication.

**Note:** To manually tamper, capture UDP with Wireshark, modify bytes, replay packet → HSIP will reject.

### 3. Injection Attack Test

Tests input validation against malicious inputs.

**Expected:** All injection attempts should be rejected cleanly (no crashes, no execution).

```powershell
.\hsip_injection_test.ps1
```

**Attack vectors tested:**
- SQL injection: `'; DROP TABLE users; --`
- Command injection: `127.0.0.1:40407; rm -rf /`
- Path traversal: `../../etc/passwd:40407`
- XSS injection: `<script>alert('xss')</script>:40407`
- Format string: `127.0.0.1%n%n%n%n:40407`
- Buffer overflow: Very long input strings
- NULL byte injection: NULL characters in strings

**Pass Criteria:** All attacks rejected with "invalid socket address" or parse errors.

**PowerShell Note:** Use `--% ` to prevent PowerShell from parsing special characters:
```powershell
& $HsipPath --% consent-send-request --to "$payload"
```

### 4. Encryption Enforcement Test

Tests that HSIP enforces encryption at protocol level.

**Expected:** All UDP traffic is encrypted; no plaintext downgrade possible.

```powershell
.\hsip_encryption_test.ps1
```

**What it does:**
1. Starts HSIP session listener
2. Sends encrypted session packets
3. Documents encryption properties

**Pass Criteria:** All traffic is ChaCha20-Poly1305 encrypted.

**SSL Stripping Resistance:** HSIP operates at UDP layer (not HTTP/TLS), so SSL stripping attacks don't apply.

**Manual Verification:**
```bash
# Capture UDP traffic
tcpdump -i any -w hsip_traffic.pcap 'udp port 50507'

# Verify no plaintext in packet payloads
strings hsip_traffic.pcap | grep -i "password\|secret\|data"
# Should find nothing (all encrypted)
```

## Manual HSIP CLI Commands

### Reputation System Tests

```powershell
# Correct command syntax (note: space, not hyphen!)
hsip-cli rep show --peer <peer_id> --score

# Append a reputation event
hsip-cli rep append --peer <peer_id> --type SPAM --severity 2 --reason "HELLO_FLOOD" --text "Sent 1000 hellos in 1 second"

# Verify reputation log integrity
hsip-cli rep verify
```

**Common mistake:** Using `rep-show` instead of `rep show` (with space).

### Session Commands

```powershell
# Start session listener
hsip-cli session-listen --addr 127.0.0.1:50505

# Send encrypted session packets
hsip-cli session-send --to 127.0.0.1:50505 --packets 10 --min_size 64 --max_size 512

# With cover traffic (decoy packets)
hsip-cli session-listen --addr 127.0.0.1:50505 --cover --cover_rate_per_min 120
```

### Consent Commands

```powershell
# Start consent listener
hsip-cli consent-listen --addr 127.0.0.1:40405 --decision allow --ttl_ms 30000

# Create consent request
hsip-cli consent-request --file data.txt --purpose "test" --expires_ms 300000 --out req.json

# Send consent request and wait for reply
hsip-cli consent-send-request --to 127.0.0.1:40405 --file req.json --wait_reply --wait_timeout_ms 3000
```

### Ping Commands

```powershell
# Start ping listener
hsip-cli ping-listen --addr 127.0.0.1:51515

# Send encrypted pings
hsip-cli ping --to 127.0.0.1:51515 --count 10 --size 128 --timeout_ms 2000
```

## HTTP Status API Tests (Daemon Only)

**Note:** These tests apply ONLY to the HTTP status API on port 8787, NOT the HSIP UDP protocol.

```bash
# Test HTTP status endpoint
curl http://127.0.0.1:8787/status

# Rate limiting test
for i in {1..100}; do curl -s http://127.0.0.1:8787/status & done; wait

# Large payload test
dd if=/dev/zero bs=1M count=10 | curl -X POST http://127.0.0.1:8787/status --data-binary @-
```

**Expected:**
- Rate limiting should throttle excessive requests
- Large payloads should be rejected (413 or connection close)
- API should not crash

## Expected Results

| Attack | HSIP Protection Mechanism | Pass Criteria |
|--------|---------------------------|---------------|
| Replay Attack | Nonce counter prevents reuse | Duplicate packet rejected |
| Response Tampering | ChaCha20-Poly1305 AEAD | Auth tag mismatch, rejection |
| Injection Attacks | Input validation | Clean error, no execution |
| Session Hijacking | Ephemeral keys (PFS) | Cannot derive session key |
| MITM | Ed25519 signature verification | Cannot impersonate peer |
| SSL Stripping | UDP-native encryption | No HTTP layer to strip |
| Nonce Exhaustion | 64-bit counter | ~18 quintillion packets |
| Signature Forgery | Ed25519 (128-bit security) | Computationally infeasible |

## Analyzing Results

### 1. Check for Crashes

Any segfault or panic is critical:

```bash
dmesg | tail -50
journalctl -xe | grep hsip
```

### 2. Monitor Memory

During stress tests:

```bash
watch -n 1 'ps -o pid,rss,vsz,cmd | grep hsip'
```

### 3. Network Inspection

Verify encryption:

```bash
# Capture UDP traffic
tcpdump -i any -w hsip_traffic.pcap 'udp port 50505'

# Verify no plaintext leaked
strings hsip_traffic.pcap | grep -i "password\|secret\|token"
```

### 4. Reputation Logs

Check for proper event logging:

```bash
cat ~/.hsip/reputation.log
hsip-cli rep verify
```

## Deprecated Tests

The following Python files are **deprecated** and **do not test HSIP**:

- ❌ `header_injection.py` - mitmproxy HTTP script (HSIP is UDP)
- ❌ `replay_attack.py` - mitmproxy HTTP script (HSIP is UDP)
- ❌ `ssl_strip.py` - mitmproxy HTTP script (HSIP is UDP)
- ❌ `response_tamper.py` - mitmproxy HTTP script (HSIP is UDP)

**Why deprecated:** These test HTTP proxies, not HSIP UDP protocol. HSIP doesn't use HTTP/TLS, so HTTP attacks don't apply.

**Use instead:** The new PowerShell scripts (`hsip_*.ps1`) that directly test HSIP UDP protocol using `hsip-cli` commands.

## Checklist Before Production

- [ ] All HSIP native tests PASSED (run_hsip_tests.ps1)
- [ ] Replay attacks: Detected and blocked (nonce protection)
- [ ] Session hijacking: Impossible (PFS verified)
- [ ] Signature forgery: All attempts failed (Ed25519 verified)
- [ ] Nonce reuse: Detected and rejected
- [ ] Memory safety: No leaks or corruption
- [ ] Injection attacks: All rejected cleanly
- [ ] DoS resistance: Graceful degradation
- [ ] Error messages: No information disclosure
- [ ] UDP traffic inspection: No plaintext found
- [ ] Reputation system: Logging and verification working

## Getting Help

If tests fail or you find vulnerabilities:

1. **Save test results**: All output is in `security_tests/results/`
2. **Document the issue**: Attack vector, repro steps, severity
3. **Report privately**: security@nyxsystems.io
4. **Do NOT disclose publicly** until patched (responsible disclosure)

## References

- Main documentation: `../GETTING_STARTED.md`
- Protocol spec: `../spec/README.md`
- Code: `../crates/hsip-core/src/`
- CLI code: `../crates/hsip-cli/src/main.rs`
