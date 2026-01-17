# HSIP Native Security Test Suite
# Runs all HSIP UDP protocol security tests

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "HSIP Phase 1 Security Testing Suite" -ForegroundColor Cyan
Write-Host "UDP Protocol Native Tests" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan

$Timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$ResultsDir = Join-Path $PSScriptRoot "results"

if (-not (Test-Path $ResultsDir)) {
    New-Item -ItemType Directory -Path $ResultsDir | Out-Null
}

Write-Host "[*] Checking prerequisites..." -ForegroundColor Yellow

# Check for hsip-cli
$HsipPath = if (Test-Path "C:\Program Files\HSIP\hsip-cli.exe") {
    "C:\Program Files\HSIP\hsip-cli.exe"
} else {
    "hsip-cli"
}

try {
    $Version = & $HsipPath --version 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) {
        throw "hsip-cli not found"
    }
    Write-Host "[✓] Prerequisites OK" -ForegroundColor Green
} catch {
    Write-Host "[✗] HSIP CLI not found. Please install HSIP first." -ForegroundColor Red
    exit 1
}

Write-Host "`n[*] Checking HSIP services..." -ForegroundColor Yellow

# Check if daemon is running
try {
    $DaemonTest = Invoke-WebRequest -Uri "http://127.0.0.1:8787/status" -UseBasicParsing -TimeoutSec 2 -ErrorAction SilentlyContinue
    if ($DaemonTest.StatusCode -eq 200) {
        Write-Host "[✓] HSIP daemon running on port 8787" -ForegroundColor Green
    } else {
        Write-Host "[!] HSIP daemon not responding on port 8787" -ForegroundColor Yellow
        Write-Host "[!] Start daemon with: hsip-cli daemon" -ForegroundColor Yellow
    }
} catch {
    Write-Host "[!] HSIP daemon not responding on port 8787" -ForegroundColor Yellow
    Write-Host "[!] Tests will continue but may require manual daemon start" -ForegroundColor Yellow
}

# Test 1: Replay Attack
Write-Host "`n[1/4] Testing Replay Attack Protection..." -ForegroundColor Yellow
$Test1Log = Join-Path $ResultsDir "01_replay_attack_$Timestamp.log"
try {
    & (Join-Path $PSScriptRoot "hsip_replay_attack.ps1") *> $Test1Log
    if ($LASTEXITCODE -eq 0) {
        Write-Host "[✓] Results: $Test1Log" -ForegroundColor Green
    } else {
        Write-Host "[✗] Test failed - check log: $Test1Log" -ForegroundColor Red
    }
} catch {
    Write-Host "[✗] Test error: $_" -ForegroundColor Red
}

# Test 2: Response Tampering
Write-Host "`n[2/4] Testing Response Tampering (AEAD)..." -ForegroundColor Yellow
$Test2Log = Join-Path $ResultsDir "02_response_tamper_$Timestamp.log"
try {
    & (Join-Path $PSScriptRoot "hsip_response_tamper.ps1") *> $Test2Log
    if ($LASTEXITCODE -eq 0) {
        Write-Host "[✓] Results: $Test2Log" -ForegroundColor Green
    } else {
        Write-Host "[✗] Test failed - check log: $Test2Log" -ForegroundColor Red
    }
} catch {
    Write-Host "[✗] Test error: $_" -ForegroundColor Red
}

# Test 3: Injection Attacks
Write-Host "`n[3/4] Testing Input Injection Attacks..." -ForegroundColor Yellow
$Test3Log = Join-Path $ResultsDir "03_injection_test_$Timestamp.log"
try {
    & (Join-Path $PSScriptRoot "hsip_injection_test.ps1") *> $Test3Log
    if ($LASTEXITCODE -eq 0) {
        Write-Host "[✓] Results: $Test3Log" -ForegroundColor Green
    } else {
        Write-Host "[✗] Test failed - check log: $Test3Log" -ForegroundColor Red
    }
} catch {
    Write-Host "[✗] Test error: $_" -ForegroundColor Red
}

# Test 4: Encryption Enforcement
Write-Host "`n[4/4] Testing Encryption Enforcement..." -ForegroundColor Yellow
$Test4Log = Join-Path $ResultsDir "04_encryption_test_$Timestamp.log"
try {
    & (Join-Path $PSScriptRoot "hsip_encryption_test.ps1") *> $Test4Log
    if ($LASTEXITCODE -eq 0) {
        Write-Host "[✓] Results: $Test4Log" -ForegroundColor Green
    } else {
        Write-Host "[✗] Test failed - check log: $Test4Log" -ForegroundColor Red
    }
} catch {
    Write-Host "[✗] Test error: $_" -ForegroundColor Red
}

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "Test Suite Complete" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan

Write-Host "Results saved to: $ResultsDir" -ForegroundColor Green
Write-Host "`nReview the logs for detailed results:" -ForegroundColor Yellow
Get-ChildItem -Path $ResultsDir -Filter "*_$Timestamp.log" | ForEach-Object {
    $Size = [math]::Round($_.Length / 1KB, 2)
    Write-Host "  $($_.FullName) ($Size KB)" -ForegroundColor Gray
}

Write-Host "`nNext Steps:" -ForegroundColor Cyan
Write-Host "`n  1. Review all log files in $ResultsDir" -ForegroundColor Gray
Write-Host "  2. Check for any unexpected errors or crashes" -ForegroundColor Gray
Write-Host "  3. Verify HSIP properly rejected all attack attempts" -ForegroundColor Gray
Write-Host "  4. For manual verification, capture UDP traffic with Wireshark" -ForegroundColor Gray
Write-Host "  5. Run: hsip-cli rep show --peer <peer_id> --score" -ForegroundColor Gray
Write-Host "     to check reputation scores`n" -ForegroundColor Gray
