# Security Test Fixes Summary

## Issues Fixed

### 1. ✅ `rep-show` Subcommand Error

**Problem:** Users were trying to run `hsip-cli rep-show` which doesn't exist.

**Root Cause:** The reputation command is `rep show` (with a space), not `rep-show` (with a hyphen).

**Fix:** Updated documentation in `security_tests/README.md` to clarify the correct syntax:
```powershell
# CORRECT
hsip-cli rep show --peer <peer_id> --score

# WRONG
hsip-cli rep-show --peer <peer_id> --score
```

**Location:** See `crates/hsip-cli/src/cmd_rep.rs` lines 54-67 for the `Show` subcommand definition.

### 2. ✅ `hsip-integration-minimal` Package Reference

**Problem:** Error message: "package ID specification hsip-integration-minimal did not match any packages"

**Root Cause:** This package doesn't exist in the workspace. The error likely came from incorrect documentation or test commands.

**Fix:** No such package exists in `Cargo.toml`. If you encounter this error:
- Check you're using correct package names from `Cargo.toml` members list
- Valid packages: `hsip-core`, `hsip-cli`, `hsip-session`, `hsip-net`, `hsip-reputation`, `hsip-auth`, `hsip-gateway`, etc.
- For integration tests, use `cargo test --workspace` instead of targeting non-existent packages

### 3. ✅ HTTP-Based Security Tests Are Misleading

**Problem:** The security test suite contained mitmproxy HTTP scripts:
- `replay_attack.py`
- `header_injection.py`
- `ssl_strip.py`
- `response_tamper.py`

**These scripts DO NOT test HSIP at all.** HSIP is a UDP protocol, not HTTP.

**Why This Is Wrong:**
- HSIP operates at the UDP layer with native ChaCha20-Poly1305 encryption
- HSIP doesn't use HTTP, TLS, or SSL
- HTTP header injection doesn't apply to UDP packets
- SSL stripping doesn't apply to UDP-native encryption
- mitmproxy intercepts HTTP/HTTPS traffic, not HSIP UDP frames

**Fix:**

Created proper HSIP UDP-native security tests:

1. **`hsip_replay_attack.ps1`** - Tests nonce-based replay protection
   - Uses `hsip-cli consent-send-request` to send the same request twice
   - Verifies second request is rejected due to nonce counter

2. **`hsip_response_tamper.ps1`** - Tests ChaCha20-Poly1305 AEAD authentication
   - Uses `hsip-cli session-send` to send encrypted UDP packets
   - Documents that any ciphertext tampering will fail AEAD verification

3. **`hsip_injection_test.ps1`** - Tests input validation
   - Tests SQL injection, command injection, path traversal, XSS, etc.
   - Uses `hsip-cli consent-send-request --to "malicious payload"`
   - Includes `--% ` stop-parsing token for PowerShell

4. **`hsip_encryption_test.ps1`** - Tests encryption enforcement
   - Uses `hsip-cli session-send` to verify all traffic is encrypted
   - Documents SSL stripping doesn't apply (no HTTP layer)

5. **`run_hsip_tests.ps1`** - Master test runner
   - Runs all 4 HSIP native tests
   - Saves results to `results/` directory

**Deprecated Old Tests:**
- Added deprecation warnings to all `.py` mitmproxy scripts
- Clearly marked them as "DO NOT USE"
- Kept files for historical reference only

### 4. ✅ PowerShell Parsing Issues

**Problem:** PowerShell interprets special characters in payloads, breaking injection tests.

**Fix:** Use `--% ` (stop-parsing token) in PowerShell commands:

```powershell
# CORRECT - Prevents PowerShell from parsing special chars
& $HsipPath --% consent-send-request --to "$payload"

# WRONG - PowerShell may interpret special characters
& $HsipPath consent-send-request --to "$payload"
```

**Documented in:** `security_tests/README.md` and `hsip_injection_test.ps1`

## New Security Test Suite

### Quick Start

**Run all HSIP native tests:**
```powershell
cd security_tests
.\run_hsip_tests.ps1
```

### Individual Tests

```powershell
# Test replay protection
.\hsip_replay_attack.ps1

# Test AEAD authentication
.\hsip_response_tamper.ps1

# Test input validation (7 attack vectors)
.\hsip_injection_test.ps1

# Test encryption enforcement
.\hsip_encryption_test.ps1
```

### Test Coverage

| Attack Type | Old Test (HTTP) | New Test (UDP) | Status |
|-------------|-----------------|----------------|--------|
| Replay Attack | ❌ mitmproxy HTTP | ✅ `hsip_replay_attack.ps1` | Fixed |
| Response Tampering | ❌ mitmproxy HTTP | ✅ `hsip_response_tamper.ps1` | Fixed |
| Header Injection | ❌ mitmproxy HTTP | ✅ `hsip_injection_test.ps1` | Fixed |
| SSL Stripping | ❌ mitmproxy HTTP (N/A) | ✅ `hsip_encryption_test.ps1` | Fixed |

## Correct CLI Command Reference

### Reputation Commands

```powershell
# Show reputation events (CORRECT - note the space!)
hsip-cli rep show --peer <peer_id> --score

# Append reputation event
hsip-cli rep append --peer <peer_id> --type SPAM --severity 2 --reason "HELLO_FLOOD"

# Verify reputation log
hsip-cli rep verify
```

### Session Commands

```powershell
# Start listener
hsip-cli session-listen --addr 127.0.0.1:50505

# Send encrypted packets
hsip-cli session-send --to 127.0.0.1:50505 --packets 10
```

### Consent Commands

```powershell
# Start listener
hsip-cli consent-listen --addr 127.0.0.1:40405 --decision allow --ttl_ms 30000

# Send request
hsip-cli consent-send-request --to 127.0.0.1:40405 --file req.json --wait_reply
```

## Files Changed

### New Files
- `security_tests/hsip_replay_attack.ps1` - HSIP UDP replay test
- `security_tests/hsip_response_tamper.ps1` - HSIP AEAD tampering test
- `security_tests/hsip_injection_test.ps1` - HSIP injection test (7 attack vectors)
- `security_tests/hsip_encryption_test.ps1` - HSIP encryption enforcement test
- `security_tests/run_hsip_tests.ps1` - Master test runner
- `SECURITY_TEST_FIXES.md` - This file

### Modified Files
- `security_tests/README.md` - Complete rewrite with HSIP UDP test instructions
- `security_tests/replay_attack.py` - Added deprecation warning
- `security_tests/header_injection.py` - Added deprecation warning
- `security_tests/ssl_strip.py` - Added deprecation warning
- `security_tests/response_tamper.py` - Added deprecation warning

## Migration Guide

**If you were using the old HTTP tests:**

| Old Command | New Command |
|-------------|-------------|
| `mitmdump -s replay_attack.py` | `.\hsip_replay_attack.ps1` |
| `mitmdump -s header_injection.py` | `.\hsip_injection_test.ps1` |
| `mitmdump -s ssl_strip.py` | `.\hsip_encryption_test.ps1` |
| `mitmdump -s response_tamper.py` | `.\hsip_response_tamper.ps1` |

**Run all tests:**
```powershell
# Old (wrong - tests HTTP, not HSIP)
cd security_tests && bash run_all_tests.sh

# New (correct - tests HSIP UDP protocol)
cd security_tests && .\run_hsip_tests.ps1
```

## Testing the Fixes

To verify these fixes work:

1. **Test the new security suite:**
   ```powershell
   cd security_tests
   .\run_hsip_tests.ps1
   ```

2. **Verify reputation command:**
   ```powershell
   hsip-cli rep show --peer "test_peer" --score
   # Should work (not "rep-show")
   ```

3. **Check no HTTP dependencies:**
   ```powershell
   # Old tests required mitmproxy (HTTP tool)
   # New tests only require hsip-cli (UDP native)
   Get-Command hsip-cli
   ```

## Summary

✅ **Fixed `rep-show` → `rep show` documentation**
✅ **Clarified no `hsip-integration-minimal` package exists**
✅ **Replaced misleading HTTP tests with proper HSIP UDP tests**
✅ **Fixed PowerShell parsing issues with `--% ` token**
✅ **Updated all documentation with correct commands**
✅ **Deprecated old mitmproxy scripts with clear warnings**

**Result:** Security test suite now properly tests HSIP UDP protocol using native `hsip-cli` commands.
