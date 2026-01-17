# Size Limit Test - Tests payload size handling

Write-Host "=== Size Limit Test ===" -ForegroundColor Cyan
Write-Host "Testing session-send with various payload sizes`n" -ForegroundColor Yellow

$hsipPath = ".\target\release\hsip-cli.exe"
$testPort = 9002
$testAddr = "127.0.0.1:$testPort"

if (-not (Test-Path $hsipPath)) {
    Write-Host "ERROR: hsip-cli.exe not found at $hsipPath" -ForegroundColor Red
    Write-Host "Build first: cargo build --release -p hsip-cli --features full" -ForegroundColor Yellow
    exit 1
}

Write-Host "Starting session-listen on $testAddr..." -ForegroundColor Green
$listener = Start-Job -ScriptBlock {
    param($path, $addr)
    & $path session-listen --addr $addr 2>&1
} -ArgumentList $hsipPath, $testAddr

Start-Sleep -Seconds 2

$testCases = @(
    @{ MinSize = 128; MaxSize = 512; Name = "512 B"; ShouldAccept = $true },
    @{ MinSize = 1024; MaxSize = 1024; Name = "1 KB"; ShouldAccept = $true },
    @{ MinSize = 102400; MaxSize = 102400; Name = "100 KB"; ShouldAccept = $true },
    @{ MinSize = 524288; MaxSize = 524288; Name = "512 KB"; ShouldAccept = $true },
    @{ MinSize = 1048576; MaxSize = 1048576; Name = "1 MB"; ShouldAccept = $true },
    @{ MinSize = 2097152; MaxSize = 2097152; Name = "2 MB"; ShouldAccept = $false }
)

$passed = 0
$failed = 0

foreach ($test in $testCases) {
    Write-Host "`nTest: $($test.Name)" -ForegroundColor Cyan

    $result = & $hsipPath session-send --to $testAddr --packets 1 --min-size $test.MinSize --max-size $test.MaxSize 2>&1 | Out-String

    $wasRejected = ($result -match "error|too large|size|exceeded|rejected|failed")

    if ($test.ShouldAccept) {
        if (-not $wasRejected) {
            Write-Host "  ✅ ACCEPTED (Expected)" -ForegroundColor Green
            $passed++
        } else {
            Write-Host "  ❌ REJECTED (Should Accept!)" -ForegroundColor Red
            Write-Host "  Error: $result" -ForegroundColor Yellow
            $failed++
        }
    } else {
        if ($wasRejected) {
            Write-Host "  ✅ REJECTED (Expected)" -ForegroundColor Green
            $passed++
        } else {
            Write-Host "  ❌ ACCEPTED (Security Issue!)" -ForegroundColor Red
            $failed++
        }
    }

    Start-Sleep -Milliseconds 500
}

Stop-Job -Job $listener -ErrorAction SilentlyContinue
Remove-Job -Job $listener -Force

Write-Host "`n=== Results ===" -ForegroundColor Cyan
Write-Host "Passed: $passed/$($testCases.Count)" -ForegroundColor Green
Write-Host "Failed: $failed/$($testCases.Count)" -ForegroundColor Red

if ($failed -eq 0) {
    Write-Host "`n✅ PASS: Size limits work as expected!" -ForegroundColor Green
} elseif ($passed -ge 3) {
    Write-Host "`n⚠️  PARTIAL: Some size limits enforced ($passed/$($testCases.Count))" -ForegroundColor Yellow
} else {
    Write-Host "`n❌ FAIL: $failed tests failed!" -ForegroundColor Red
}
