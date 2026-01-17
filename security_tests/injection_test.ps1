# Injection Attack Test - Tests input validation

Write-Host "=== Injection Attack Test ===" -ForegroundColor Cyan
Write-Host "Testing input validation with malicious payloads`n" -ForegroundColor Yellow

$hsipPath = ".\target\release\hsip-cli.exe"

if (-not (Test-Path $hsipPath)) {
    Write-Host "ERROR: hsip-cli.exe not found at $hsipPath" -ForegroundColor Red
    Write-Host "Build first: cargo build --release -p hsip-cli --features full" -ForegroundColor Yellow
    exit 1
}

$payloads = @(
    @{
        Name = "Path Traversal"
        Value = "../../../etc/passwd"
    },
    @{
        Name = "Command Injection (Unix)"
        Value = "127.0.0.1; rm -rf /"
    },
    @{
        Name = "Command Injection (Windows)"
        Value = "127.0.0.1 & del /F /Q *.*"
    },
    @{
        Name = "Command Substitution"
        Value = "`$(whoami)"
    },
    @{
        Name = "Null Byte Injection"
        Value = "127.0.0.1`0.evil.com"
    },
    @{
        Name = "SQL Injection"
        Value = "'; DROP TABLE audit_logs; --"
    },
    @{
        Name = "Log Injection"
        Value = "127.0.0.1`n[AUDIT] FAKE LOG"
    },
    @{
        Name = "Oversized Domain"
        Value = "a" * 300 + ".com"
    },
    @{
        Name = "Invalid Port"
        Value = "127.0.0.1:99999"
    },
    @{
        Name = "Malformed Address"
        Value = "not.a.valid@address"
    }
)

$passed = 0
$failed = 0

foreach ($payload in $payloads) {
    Write-Host "`nTest: $($payload.Name)" -ForegroundColor Cyan
    Write-Host "  Payload: $($payload.Value)" -ForegroundColor Gray

    $result = & $hsipPath consent-send-request --to $payload.Value 2>&1 | Out-String

    if ($result -match "error|invalid|parse|failed|validation") {
        Write-Host "  ✅ REJECTED" -ForegroundColor Green
        $passed++
    } else {
        Write-Host "  ❌ ACCEPTED (Security Issue!)" -ForegroundColor Red
        Write-Host "  Response: $result" -ForegroundColor Yellow
        $failed++
    }
}

Write-Host "`n=== Results ===" -ForegroundColor Cyan
Write-Host "Rejected: $passed/$($payloads.Count)" -ForegroundColor Green
Write-Host "Accepted: $failed/$($payloads.Count)" -ForegroundColor Red

if ($failed -eq 0) {
    Write-Host "`n✅ PASS: All injection attempts were blocked!" -ForegroundColor Green
} else {
    Write-Host "`n❌ FAIL: $failed payloads bypassed validation!" -ForegroundColor Red
}
