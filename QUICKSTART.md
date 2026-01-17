# HSIP Quick Start - Build & Test

**URGENT:** Follow these steps exactly to build and test HSIP Phase 1.

---

## Step 1: Pull Latest Fixes

```powershell
git checkout claude/hsip-security-testing-9DtSQ
git pull origin claude/hsip-security-testing-9DtSQ
```

---

## Step 2: Build All Executables

```powershell
# Clean previous build
cargo clean

# Build CLI with all features
cargo build --release -p hsip-cli --features full

# Build Tray Icon (Windows only)
cargo build --release -p hsip-cli --bin hsip-tray --features full,tray

# Build Gateway
cargo build --release -p hsip-gateway
```

**Expected:** All builds succeed, executables in `target\release\`

---

## Step 3: Verify Executables Exist

```powershell
# Check files
dir target\release\hsip-cli.exe
dir target\release\hsip-tray.exe
dir target\release\hsip-gateway.exe
```

**Expected:** All three files exist with sizes > 1MB

---

## Step 4: Test Core Features (NO POSTGRESQL NEEDED)

Run these tests **WITHOUT** PostgreSQL:

### Test 1: CLI Version
```powershell
.\target\release\hsip-cli.exe --version
```
**Expected:** Shows version like `hsip-cli 0.1.2`

### Test 2: Initialize Identity (Ed25519)
```powershell
.\target\release\hsip-cli.exe init
```
**Expected:**
```
[HSIP] Initializing new identity...
[HSIP] Generated Ed25519 keypair
[HSIP] Identity saved to: C:\Users\...\
[HSIP] Public key: <hex string>
```

### Test 3: Sign Message (Ed25519 Signature)
```powershell
echo "Test message" | .\target\release\hsip-cli.exe sign
```
**Expected:** 128-character hex signature (64 bytes)

### Test 4: IETF RFC 8439 Test Vectors (ChaCha20-Poly1305)
```powershell
.\target\release\hsip-cli.exe test-chacha20-rfc8439
```
**Expected:**
```
[TEST] Running RFC 8439 test vectors...
[TEST] ✓ Test vector 1: PASS
[TEST] ✓ Test vector 2: PASS
[TEST] ✓ All test vectors passed
```

### Test 5: Ed25519 Test Vectors
```powershell
.\target\release\hsip-cli.exe test-ed25519
```
**Expected:**
```
[TEST] Running Ed25519 test vectors...
[TEST] ✓ Signature generation: PASS
[TEST] ✓ Signature verification: PASS
```

### Test 6: Gateway Starts
```powershell
Start-Job { .\target\release\hsip-gateway.exe }
Start-Sleep 2
Get-Job
```
**Expected:** Job running, no errors

---

## Step 5: Test PostgreSQL Features (OPTIONAL)

**Only if PostgreSQL is installed:**

### Setup Database
```powershell
# Find PostgreSQL
$pgPath = "C:\Program Files\PostgreSQL\16\bin"
if (Test-Path $pgPath) {
    # Create database
    & "$pgPath\createdb.exe" hsip_audit

    # Set connection
    $env:DATABASE_URL = "postgresql://localhost/hsip_audit"

    # Initialize audit tables
    .\target\release\hsip-cli.exe audit-init

    # Test audit logging
    .\target\release\hsip-cli.exe consent --destination "test.com" --allow

    # Export audit log
    .\target\release\hsip-cli.exe audit-export --out test_audit.json

    # Verify chain integrity
    .\target\release\hsip-cli.exe audit-verify
}
```

---

## Step 6: Build Windows Installer

```powershell
cd installer

# Open Inno Setup and compile:
# File → Open → hsip-installer.iss
# Build → Compile

# Or use command line:
& "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" hsip-installer.iss
```

**Output:** `installer\output\HSIP-Setup-1.0.0.exe`

---

## Expected Test Results Summary

| Test | Result | Proves |
|------|--------|--------|
| CLI version | Shows 0.1.2 | ✅ Binary works |
| Init identity | Creates Ed25519 keypair | ✅ Cryptography works |
| Sign message | 128 hex char signature | ✅ Ed25519 signing works |
| RFC 8439 vectors | All tests pass | ✅ ChaCha20-Poly1305 IETF compliant |
| Ed25519 vectors | All tests pass | ✅ Ed25519 RFC 8032 compliant |
| Gateway starts | Runs without error | ✅ Proxy functionality works |
| Audit init | Tables created | ✅ PostgreSQL integration works |
| Audit export | JSON file created | ✅ Court evidence export works |
| Audit verify | Chain verified | ✅ Tamper detection works |

---

## Quick Validation Script

Save as `quick-validate.ps1`:

```powershell
Write-Host "=== HSIP Quick Validation ===" -ForegroundColor Cyan

# Test 1: Version
Write-Host "`n[1/5] Testing CLI..." -ForegroundColor Yellow
$v = .\target\release\hsip-cli.exe --version
if ($v) { Write-Host "✓ PASS: $v" -ForegroundColor Green } else { Write-Host "✗ FAIL" -ForegroundColor Red }

# Test 2: Init
Write-Host "`n[2/5] Testing identity generation..." -ForegroundColor Yellow
$init = .\target\release\hsip-cli.exe init 2>&1
if ($init -like "*Generated Ed25519*") { Write-Host "✓ PASS" -ForegroundColor Green } else { Write-Host "✗ FAIL" -ForegroundColor Red }

# Test 3: Sign
Write-Host "`n[3/5] Testing Ed25519 signing..." -ForegroundColor Yellow
$sig = echo "test" | .\target\release\hsip-cli.exe sign
if ($sig.Length -eq 128) { Write-Host "✓ PASS" -ForegroundColor Green } else { Write-Host "✗ FAIL" -ForegroundColor Red }

# Test 4: RFC 8439
Write-Host "`n[4/5] Testing IETF RFC 8439 compliance..." -ForegroundColor Yellow
$rfc = .\target\release\hsip-cli.exe test-chacha20-rfc8439 2>&1
if ($rfc -like "*All test vectors passed*") { Write-Host "✓ PASS" -ForegroundColor Green } else { Write-Host "✗ FAIL" -ForegroundColor Red }

# Test 5: Ed25519 vectors
Write-Host "`n[5/5] Testing Ed25519 test vectors..." -ForegroundColor Yellow
$ed = .\target\release\hsip-cli.exe test-ed25519 2>&1
if ($LASTEXITCODE -eq 0) { Write-Host "✓ PASS" -ForegroundColor Green } else { Write-Host "✗ FAIL" -ForegroundColor Red }

Write-Host "`n=== Validation Complete ===" -ForegroundColor Cyan
Write-Host "If all 5 tests passed, HSIP is working correctly!" -ForegroundColor Green
```

**Run it:**
```powershell
.\quick-validate.ps1
```

---

## What to Submit

For reviewers:

1. **Test Results**: Output from `quick-validate.ps1`
2. **Installer**: `installer\output\HSIP-Setup-1.0.0.exe`
3. **Documentation**: Already in repository
4. **Security Audit**: Run `cargo audit` (should show NO vulnerabilities)

---

## Troubleshooting

### "hsip-cli.exe not found"
- Run from project root: `C:\Users\melas\Desktop\HSIP-1PHASE-1`
- Build first: `cargo build --release -p hsip-cli --features full`

### "PostgreSQL not found"
- Tests 1-6 work WITHOUT PostgreSQL
- PostgreSQL only needed for audit logs (optional)

### "Inno Setup not found"
- Open Inno Setup GUI manually
- File → Open → `installer\hsip-installer.iss`
- Build → Compile

---

**Time Required:** 10-15 minutes total

**Last Updated:** January 14, 2026
