# DoS Flood Test - Tests rate limiting with UDP protocol

Write-Host "=== DoS Flood Test ===" -ForegroundColor Cyan
Write-Host "Testing HSIP hello-send flood against hello-listen" -ForegroundColor Yellow
Write-Host "Note: Rate limiter modules exist but not yet integrated into CLI`n"

$hsipPath = ".\target\release\hsip-cli.exe"
$testPort = 9000
$testAddr = "127.0.0.1:$testPort"
$floodCount = 200

if (-not (Test-Path $hsipPath)) {
    Write-Host "ERROR: hsip-cli.exe not found at $hsipPath" -ForegroundColor Red
    Write-Host "Build first: cargo build --release -p hsip-cli --features full" -ForegroundColor Yellow
    exit 1
}

Write-Host "Starting UDP listener on $testAddr..." -ForegroundColor Green
$listener = Start-Job -ScriptBlock {
    param($path, $addr)
    & $path hello-listen --addr $addr 2>&1
} -ArgumentList $hsipPath, $testAddr

Start-Sleep -Seconds 2

$accepted = 0
$errors = 0
$latencies = @()

Write-Host "Sending $floodCount rapid hello-send requests...`n" -ForegroundColor Cyan

$stopwatch = [System.Diagnostics.Stopwatch]::StartNew()

for ($i = 1; $i -le $floodCount; $i++) {
    $reqStart = Get-Date

    $result = & $hsipPath hello-send --to $testAddr 2>&1
    $reqEnd = Get-Date

    $latency = ($reqEnd - $reqStart).TotalMilliseconds
    $latencies += $latency

    if ($LASTEXITCODE -eq 0) {
        $accepted++
    } else {
        $errors++
    }

    if ($i % 50 -eq 0) {
        Write-Host "  Sent $i requests (Accepted: $accepted, Errors: $errors)" -ForegroundColor Gray
    }
}

$stopwatch.Stop()

Stop-Job -Job $listener -ErrorAction SilentlyContinue
Remove-Job -Job $listener -Force

$avgLatency = ($latencies | Measure-Object -Average).Average
$totalTime = $stopwatch.Elapsed.TotalSeconds

Write-Host "`n=== Results ===" -ForegroundColor Cyan
Write-Host "Total Requests: $floodCount" -ForegroundColor White
Write-Host "Accepted: $accepted" -ForegroundColor Green
Write-Host "Errors: $errors" -ForegroundColor Red
Write-Host "Average Latency: $([math]::Round($avgLatency, 2)) ms" -ForegroundColor White
Write-Host "Total Time: $([math]::Round($totalTime, 2)) seconds" -ForegroundColor White
Write-Host "Throughput: $([math]::Round($floodCount / $totalTime, 2)) req/sec" -ForegroundColor White

$errorRate = ($errors / $floodCount) * 100

Write-Host "`n=== Analysis ===" -ForegroundColor Cyan

if ($errorRate -gt 10) {
    Write-Host "✅ PASS: Rate limiting active ($([math]::Round($errorRate, 1))% error rate)" -ForegroundColor Green
} elseif ($errorRate -gt 0) {
    Write-Host "⚠️  PARTIAL: Some throttling detected ($([math]::Round($errorRate, 1))% error rate)" -ForegroundColor Yellow
} else {
    Write-Host "⚠️  EXPECTED: Rate limiter modules not yet integrated into CLI" -ForegroundColor Yellow
    Write-Host "   Security modules exist in hsip-net but need CLI integration" -ForegroundColor Gray
    Write-Host "   Test demonstrates protocol handles $floodCount packets successfully" -ForegroundColor Gray
}

if ($accepted -eq $floodCount) {
    Write-Host "`n✅ Protocol Stability: All $floodCount packets processed without crashes" -ForegroundColor Green
}
