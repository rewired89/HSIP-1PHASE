# HSIP Daemon Test Script for Windows
# This script builds and tests the HSIP daemon

Write-Host "=== HSIP Daemon Test ===" -ForegroundColor Cyan

# 1. Build the project
Write-Host "`n[1/5] Building HSIP..." -ForegroundColor Yellow
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}
Write-Host "Build successful!" -ForegroundColor Green

# 2. Initialize HSIP identity if not exists
Write-Host "`n[2/5] Checking HSIP identity..." -ForegroundColor Yellow
$hsipPath = "$env:USERPROFILE\.hsip"
if (-Not (Test-Path "$hsipPath\id.json")) {
    Write-Host "Creating new HSIP identity..." -ForegroundColor Yellow
    .\target\release\hsip-cli.exe init
} else {
    Write-Host "HSIP identity exists" -ForegroundColor Green
}

# 3. Start daemon in background
Write-Host "`n[3/5] Starting daemon..." -ForegroundColor Yellow
$daemonJob = Start-Job -ScriptBlock {
    Set-Location $using:PWD
    .\target\release\hsip-cli.exe daemon --status_addr 127.0.0.1:8787
}

# Wait for daemon to start
Start-Sleep -Seconds 3

# 4. Test daemon endpoints
Write-Host "`n[4/5] Testing daemon endpoints..." -ForegroundColor Yellow

try {
    # Test /status endpoint
    Write-Host "`nGET /status:" -ForegroundColor Cyan
    $response = Invoke-RestMethod -Uri "http://127.0.0.1:8787/status" -Method Get
    $response | ConvertTo-Json -Depth 3
    Write-Host "✓ /status working" -ForegroundColor Green

    # Test /sessions endpoint
    Write-Host "`nGET /sessions:" -ForegroundColor Cyan
    $response = Invoke-RestMethod -Uri "http://127.0.0.1:8787/sessions" -Method Get
    $response | ConvertTo-Json -Depth 3
    Write-Host "✓ /sessions working" -ForegroundColor Green

    # Test /reputation endpoint
    Write-Host "`nGET /reputation/test:" -ForegroundColor Cyan
    $response = Invoke-RestMethod -Uri "http://127.0.0.1:8787/reputation/test" -Method Get
    $response | ConvertTo-Json -Depth 3
    Write-Host "✓ /reputation working" -ForegroundColor Green

} catch {
    Write-Host "Error testing endpoints: $_" -ForegroundColor Red
} finally {
    # 5. Cleanup
    Write-Host "`n[5/5] Stopping daemon..." -ForegroundColor Yellow
    Stop-Job -Job $daemonJob
    Remove-Job -Job $daemonJob
    Write-Host "Daemon stopped" -ForegroundColor Green
}

Write-Host "`n=== Test Complete ===" -ForegroundColor Cyan
Write-Host "`nTo run daemon manually:" -ForegroundColor Yellow
Write-Host "  .\target\release\hsip-cli.exe daemon" -ForegroundColor White
Write-Host "`nDaemon API endpoints:" -ForegroundColor Yellow
Write-Host "  GET  http://127.0.0.1:8787/status" -ForegroundColor White
Write-Host "  GET  http://127.0.0.1:8787/sessions" -ForegroundColor White
Write-Host "  POST http://127.0.0.1:8787/consent/grant" -ForegroundColor White
Write-Host "  POST http://127.0.0.1:8787/consent/revoke" -ForegroundColor White
Write-Host "  GET  http://127.0.0.1:8787/reputation/:peer_id" -ForegroundColor White
