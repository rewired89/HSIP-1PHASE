# HSIP Injection Attack Test
# Tests HSIP's input validation and rejection of malformed data
# Expected: Invalid inputs should be rejected cleanly

Write-Host "[TEST] HSIP Injection Attack Test" -ForegroundColor Cyan
Write-Host "[INFO] Testing input validation with malformed/malicious inputs" -ForegroundColor Gray

$HsipPath = if (Test-Path "C:\Program Files\HSIP\hsip-cli.exe") {
    "C:\Program Files\HSIP\hsip-cli.exe"
} else {
    "hsip-cli"
}

$PassCount = 0
$FailCount = 0

# Start listener
Write-Host "[SETUP] Starting HSIP consent listener..." -ForegroundColor Yellow
$ListenerJob = Start-Job -ScriptBlock {
    param($hsip)
    & $hsip consent-listen --addr "127.0.0.1:40407" --decision "allow" --ttl_ms 30000
} -ArgumentList $HsipPath

Start-Sleep -Seconds 2

# Test 1: Invalid socket address (should reject)
Write-Host "`n[ATTACK 1] Testing invalid socket address injection..." -ForegroundColor Magenta
$Payload1 = "'; DROP TABLE users; --"
$Result1 = & $HsipPath --% consent-send-request --to $Payload1 | Out-String

if ($Result1 -match "invalid socket address|parse error|error") {
    Write-Host "[PASS] SQL injection in address rejected (expected)" -ForegroundColor Green
    $PassCount++
} else {
    Write-Host "[FAIL] Injection may have been processed: $Result1" -ForegroundColor Red
    $FailCount++
}

# Test 2: Command injection attempt
Write-Host "`n[ATTACK 2] Testing command injection..." -ForegroundColor Magenta
$Payload2 = "127.0.0.1:40407; rm -rf /"
$Result2 = & $HsipPath --% consent-send-request --to $Payload2 | Out-String

if ($Result2 -match "invalid socket address|parse error|error") {
    Write-Host "[PASS] Command injection rejected (expected)" -ForegroundColor Green
    $PassCount++
} else {
    Write-Host "[FAIL] Command injection may have been processed: $Result2" -ForegroundColor Red
    $FailCount++
}

# Test 3: Path traversal attempt
Write-Host "`n[ATTACK 3] Testing path traversal..." -ForegroundColor Magenta
$Payload3 = "../../etc/passwd:40407"
$Result3 = & $HsipPath --% consent-send-request --to $Payload3 | Out-String

if ($Result3 -match "invalid socket address|parse error|error") {
    Write-Host "[PASS] Path traversal rejected (expected)" -ForegroundColor Green
    $PassCount++
} else {
    Write-Host "[FAIL] Path traversal may have been processed: $Result3" -ForegroundColor Red
    $FailCount++
}

# Test 4: XSS injection attempt
Write-Host "`n[ATTACK 4] Testing XSS injection..." -ForegroundColor Magenta
$Payload4 = "<script>alert('xss')</script>:40407"
$Result4 = & $HsipPath --% consent-send-request --to $Payload4 | Out-String

if ($Result4 -match "invalid socket address|parse error|error") {
    Write-Host "[PASS] XSS injection rejected (expected)" -ForegroundColor Green
    $PassCount++
} else {
    Write-Host "[FAIL] XSS injection may have been processed: $Result4" -ForegroundColor Red
    $FailCount++
}

# Test 5: Format string injection
Write-Host "`n[ATTACK 5] Testing format string injection..." -ForegroundColor Magenta
$Payload5 = "127.0.0.1%n%n%n%n:40407"
$Result5 = & $HsipPath --% consent-send-request --to $Payload5 | Out-String

if ($Result5 -match "invalid socket address|parse error|error") {
    Write-Host "[PASS] Format string injection rejected (expected)" -ForegroundColor Green
    $PassCount++
} else {
    Write-Host "[FAIL] Format string injection may have been processed: $Result5" -ForegroundColor Red
    $FailCount++
}

# Test 6: Buffer overflow attempt (very long input)
Write-Host "`n[ATTACK 6] Testing buffer overflow with long input..." -ForegroundColor Magenta
$Payload6 = "A" * 10000 + ":40407"
$Result6 = & $HsipPath consent-send-request --to $Payload6 | Out-String

if ($Result6 -match "invalid socket address|parse error|error|too long") {
    Write-Host "[PASS] Buffer overflow attempt rejected (expected)" -ForegroundColor Green
    $PassCount++
} else {
    Write-Host "[FAIL] Long input may have been processed: $Result6" -ForegroundColor Red
    $FailCount++
}

# Test 7: NULL byte injection
Write-Host "`n[ATTACK 7] Testing NULL byte injection..." -ForegroundColor Magenta
$Payload7 = "127.0.0.1`0malicious:40407"
$Result7 = & $HsipPath --% consent-send-request --to $Payload7 | Out-String

if ($Result7 -match "invalid socket address|parse error|error") {
    Write-Host "[PASS] NULL byte injection rejected (expected)" -ForegroundColor Green
    $PassCount++
} else {
    Write-Host "[FAIL] NULL byte injection may have been processed: $Result7" -ForegroundColor Red
    $FailCount++
}

# Cleanup
Write-Host "`n[CLEANUP] Stopping listener..." -ForegroundColor Yellow
Stop-Job $ListenerJob -ErrorAction SilentlyContinue
Remove-Job $ListenerJob -ErrorAction SilentlyContinue

# Summary
Write-Host "`n[SUMMARY] Injection Attack Test Results:" -ForegroundColor Cyan
Write-Host "  Passed: $PassCount/7" -ForegroundColor $(if ($PassCount -eq 7) { "Green" } else { "Yellow" })
Write-Host "  Failed: $FailCount/7" -ForegroundColor $(if ($FailCount -eq 0) { "Green" } else { "Red" })

$TestResult = if ($FailCount -eq 0) { "PASS" } else { "FAIL" }
Write-Host "`n[RESULT] Injection Attack Test: $TestResult" -ForegroundColor $(if ($TestResult -eq "PASS") { "Green" } else { "Red" })

if ($FailCount -gt 0) {
    exit 1
}
