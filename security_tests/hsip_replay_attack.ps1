# HSIP UDP Replay Attack Test
# Tests HSIP's nonce-based replay protection
# Expected: HSIP should reject replayed packets

Write-Host "[TEST] HSIP Replay Attack Test" -ForegroundColor Cyan
Write-Host "[INFO] Testing nonce-based replay protection" -ForegroundColor Gray

$HsipPath = if (Test-Path "C:\Program Files\HSIP\hsip-cli.exe") {
    "C:\Program Files\HSIP\hsip-cli.exe"
} else {
    "hsip-cli"
}

# Start listener in background
Write-Host "[SETUP] Starting HSIP listener..." -ForegroundColor Yellow
$ListenerJob = Start-Job -ScriptBlock {
    param($hsip)
    & $hsip consent-listen --addr "127.0.0.1:40406" --decision "allow" --ttl_ms 30000
} -ArgumentList $HsipPath

Start-Sleep -Seconds 2

# Create a consent request
Write-Host "[SETUP] Creating consent request..." -ForegroundColor Yellow
$TempReq = Join-Path $env:TEMP "hsip_replay_req.json"
$TempData = Join-Path $env:TEMP "hsip_replay_data.txt"
"test data for replay" | Out-File -FilePath $TempData -Encoding ASCII

& $HsipPath consent-request --file $TempData --purpose "test" --expires-ms 300000 --out $TempReq

if (-not (Test-Path $TempReq)) {
    Write-Host "[FAIL] Could not create consent request" -ForegroundColor Red
    Stop-Job $ListenerJob -ErrorAction SilentlyContinue
    Remove-Job $ListenerJob -ErrorAction SilentlyContinue
    exit 1
}

# Send initial request (should succeed)
Write-Host "`n[ATTACK] Sending initial consent request..." -ForegroundColor Magenta
$Result1 = & $HsipPath consent-send-request --to "127.0.0.1:40406" --file $TempReq --wait-reply --wait_timeout_ms 3000 2>&1 | Out-String

if ($Result1 -match "decision='allow'") {
    Write-Host "[PASS] Initial request succeeded (expected)" -ForegroundColor Green
} else {
    Write-Host "[WARN] Initial request may have failed: $Result1" -ForegroundColor Yellow
}

Start-Sleep -Milliseconds 500

# Replay the same request (should fail due to nonce reuse)
Write-Host "`n[ATTACK] Replaying the same consent request..." -ForegroundColor Magenta
Write-Host "[ATTACK] Expected: HSIP should reject due to nonce counter mismatch" -ForegroundColor Gray

$Result2 = & $HsipPath consent-send-request --to "127.0.0.1:40406" --file $TempReq --wait-reply --wait_timeout_ms 3000 2>&1 | Out-String

if ($Result2 -match "failed to open reply|timeout|error|no reply") {
    Write-Host "[PASS] Replay attack BLOCKED - nonce protection working!" -ForegroundColor Green
    $TestResult = "PASS"
} else {
    Write-Host "[FAIL] Replay attack may have succeeded - check nonce implementation!" -ForegroundColor Red
    Write-Host "[FAIL] Response: $Result2" -ForegroundColor Red
    $TestResult = "FAIL"
}

# Cleanup
Write-Host "`n[CLEANUP] Stopping listener..." -ForegroundColor Yellow
Stop-Job $ListenerJob -ErrorAction SilentlyContinue
Remove-Job $ListenerJob -ErrorAction SilentlyContinue
Remove-Item $TempReq -ErrorAction SilentlyContinue
Remove-Item $TempData -ErrorAction SilentlyContinue

Write-Host "`n[RESULT] Replay Attack Test: $TestResult" -ForegroundColor $(if ($TestResult -eq "PASS") { "Green" } else { "Red" })

if ($TestResult -eq "FAIL") {
    exit 1
}
