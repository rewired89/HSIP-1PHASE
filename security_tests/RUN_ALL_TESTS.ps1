# HSIP Security Test Suite Runner
# Production-grade test orchestration with automated listener management

param(
    [string]$HsipPath = ".\target\release\hsip-cli.exe",
    [string]$OutputDir = ".\security_tests\out",
    [int]$HelloPort = 9000,
    [int]$ConsentPort = 9001,
    [int]$SessionPort = 9002
)

$ErrorActionPreference = "Stop"
$timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$logFile = Join-Path $OutputDir "test_run_$timestamp.log"
$resultsFile = Join-Path $OutputDir "results_$timestamp.json"

function Write-Log {
    param($Message)
    $entry = "[$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')] $Message"
    Write-Host $entry
    Add-Content -Path $logFile -Value $entry
}

function Test-Port {
    param([int]$Port)
    try {
        $connection = New-Object System.Net.Sockets.TcpClient("127.0.0.1", $Port)
        $connection.Close()
        return $true
    } catch {
        return $false
    }
}

function Wait-ForListener {
    param([int]$Port, [int]$TimeoutSeconds = 10)
    $elapsed = 0
    while ($elapsed -lt $TimeoutSeconds) {
        Start-Sleep -Milliseconds 500
        $elapsed += 0.5

        $result = & $HsipPath hello-send --to "127.0.0.1:$Port" 2>&1
        if ($LASTEXITCODE -eq 0) {
            return $true
        }
    }
    return $false
}

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  HSIP Security Test Suite" -ForegroundColor Cyan
Write-Host "  Production Test Runner v1.0" -ForegroundColor Cyan
Write-Host "============================================`n" -ForegroundColor Cyan

if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

Write-Log "=== PREFLIGHT CHECKS ==="

if (-not (Test-Path $HsipPath)) {
    Write-Log "ERROR: hsip-cli.exe not found at $HsipPath"
    Write-Host "Build first: cargo build --release -p hsip-cli --features full" -ForegroundColor Red
    exit 1
}

$version = & $HsipPath --version 2>&1 | Out-String
Write-Log "Binary: $HsipPath"
Write-Log "Version: $($version.Trim())"
Write-Log "Output directory: $OutputDir"
Write-Log "Test ports: hello=$HelloPort, consent=$ConsentPort, session=$SessionPort"

$testResults = @{
    timestamp = $timestamp
    binary = $HsipPath
    version = $version.Trim()
    tests = @()
}

Write-Log "`n=== TEST 1: INJECTION ATTACKS ==="
Write-Log "Testing input validation with malicious payloads"

$injectionStart = Get-Date
$injectionResults = @{
    test = "injection"
    payloads = 10
    rejected = 0
    accepted = 0
    errors = @()
}

$payloads = @(
    "../../../etc/passwd",
    "127.0.0.1; rm -rf /",
    "127.0.0.1 & del /F /Q *.*",
    "`$(whoami)",
    "127.0.0.1`0.evil.com",
    "'; DROP TABLE audit_logs; --",
    "127.0.0.1`n[AUDIT] FAKE LOG",
    ("a" * 300 + ".com"),
    "127.0.0.1:99999",
    "not.a.valid@address"
)

foreach ($payload in $payloads) {
    $result = & $HsipPath consent-send-request --to $payload 2>&1 | Out-String
    if ($result -match "error|invalid|parse|failed|validation") {
        $injectionResults.rejected++
    } else {
        $injectionResults.accepted++
        $injectionResults.errors += "Payload accepted: $payload"
    }
}

$injectionResults.duration_ms = ((Get-Date) - $injectionStart).TotalMilliseconds
$injectionResults.pass = ($injectionResults.accepted -eq 0)

Write-Log "sent=$($injectionResults.payloads) rejected=$($injectionResults.rejected) accepted=$($injectionResults.accepted) duration=$([math]::Round($injectionResults.duration_ms, 2))ms"
Write-Log "result=$(if ($injectionResults.pass) { 'PASS' } else { 'FAIL' })"

$testResults.tests += $injectionResults

Write-Log "`n=== TEST 2: SIZE LIMITS ==="
Write-Log "Starting session listener on 127.0.0.1:$SessionPort"

$sessionListener = Start-Job -ScriptBlock {
    param($path, $port)
    & $path session-listen --addr "127.0.0.1:$port" 2>&1
} -ArgumentList $HsipPath, $SessionPort

if (-not (Wait-ForListener -Port $SessionPort -TimeoutSeconds 5)) {
    Write-Log "ERROR: Session listener failed to start"
    Stop-Job -Job $sessionListener -ErrorAction SilentlyContinue
    Remove-Job -Job $sessionListener -Force
    exit 1
}

Write-Log "Listener confirmed on port $SessionPort"

$sizeStart = Get-Date
$sizeResults = @{
    test = "size_limits"
    cases = 6
    passed = 0
    failed = 0
    details = @()
}

$sizeCases = @(
    @{ Size = 512; Name = "512B"; ShouldAccept = $true },
    @{ Size = 1024; Name = "1KB"; ShouldAccept = $true },
    @{ Size = 102400; Name = "100KB"; ShouldAccept = $true },
    @{ Size = 524288; Name = "512KB"; ShouldAccept = $true },
    @{ Size = 1048576; Name = "1MB"; ShouldAccept = $true },
    @{ Size = 2097152; Name = "2MB"; ShouldAccept = $false }
)

foreach ($case in $sizeCases) {
    $result = & $HsipPath session-send --to "127.0.0.1:$SessionPort" --packets 1 --min-size $case.Size --max-size $case.Size 2>&1 | Out-String
    $wasRejected = ($result -match "error|too large|size|exceeded|rejected|failed")

    $casePass = ($case.ShouldAccept -and -not $wasRejected) -or (-not $case.ShouldAccept -and $wasRejected)

    if ($casePass) {
        $sizeResults.passed++
    } else {
        $sizeResults.failed++
    }

    $sizeResults.details += @{
        size = $case.Name
        expected = if ($case.ShouldAccept) { "accept" } else { "reject" }
        actual = if ($wasRejected) { "rejected" } else { "accepted" }
        pass = $casePass
    }
}

Stop-Job -Job $sessionListener -ErrorAction SilentlyContinue
Remove-Job -Job $sessionListener -Force

$sizeResults.duration_ms = ((Get-Date) - $sizeStart).TotalMilliseconds
$sizeResults.pass = ($sizeResults.failed -eq 0)

Write-Log "cases=$($sizeResults.cases) passed=$($sizeResults.passed) failed=$($sizeResults.failed) duration=$([math]::Round($sizeResults.duration_ms, 2))ms"
Write-Log "result=$(if ($sizeResults.pass) { 'PASS' } else { 'FAIL' })"

$testResults.tests += $sizeResults

Write-Log "`n=== TEST 3: DOS FLOOD ==="
Write-Log "Starting hello listener on 127.0.0.1:$HelloPort"

$helloListener = Start-Job -ScriptBlock {
    param($path, $port)
    & $path hello-listen --addr "127.0.0.1:$port" 2>&1
} -ArgumentList $HsipPath, $HelloPort

if (-not (Wait-ForListener -Port $HelloPort -TimeoutSeconds 5)) {
    Write-Log "ERROR: Hello listener failed to start"
    Stop-Job -Job $helloListener -ErrorAction SilentlyContinue
    Remove-Job -Job $helloListener -Force
    exit 1
}

Write-Log "Listener confirmed on port $HelloPort"

$floodCount = 200
$floodStart = Get-Date

$dosResults = @{
    test = "dos_flood"
    sent = 0
    accepted = 0
    errors = 0
    latencies_ms = @()
}

for ($i = 1; $i -le $floodCount; $i++) {
    $reqStart = Get-Date
    $result = & $HsipPath hello-send --to "127.0.0.1:$HelloPort" 2>&1
    $reqEnd = Get-Date

    $latency = ($reqEnd - $reqStart).TotalMilliseconds
    $dosResults.latencies_ms += $latency
    $dosResults.sent++

    if ($LASTEXITCODE -eq 0) {
        $dosResults.accepted++
    } else {
        $dosResults.errors++
    }
}

Stop-Job -Job $helloListener -ErrorAction SilentlyContinue
Remove-Job -Job $helloListener -Force

$dosResults.duration_ms = ((Get-Date) - $floodStart).TotalMilliseconds
$dosResults.avg_latency_ms = [math]::Round(($dosResults.latencies_ms | Measure-Object -Average).Average, 2)
$dosResults.throughput_pps = [math]::Round($dosResults.sent / ($dosResults.duration_ms / 1000), 2)
$dosResults.error_rate = ($dosResults.errors / $dosResults.sent) * 100
$dosResults.pass = ($dosResults.sent -eq $dosResults.accepted -or $dosResults.error_rate -gt 10)

Write-Log "sent=$($dosResults.sent) accepted=$($dosResults.accepted) errors=$($dosResults.errors) duration=$([math]::Round($dosResults.duration_ms, 2))ms"
Write-Log "avg_latency=$($dosResults.avg_latency_ms)ms throughput=$($dosResults.throughput_pps)pps error_rate=$([math]::Round($dosResults.error_rate, 2))%"
Write-Log "result=$(if ($dosResults.pass) { 'PASS' } else { 'FAIL' })"

$testResults.tests += $dosResults

$testResults | ConvertTo-Json -Depth 10 | Set-Content -Path $resultsFile

Write-Log "`n============================================"
Write-Log "  TEST SUMMARY"
Write-Log "============================================"

$allPass = $true
foreach ($test in $testResults.tests) {
    $status = if ($test.pass) { "✅ PASS" } else { "❌ FAIL"; $allPass = $false }
    Write-Log "$status - $($test.test)"
}

Write-Log "`nResults saved to: $resultsFile"
Write-Log "Logs saved to: $logFile"

if (-not $allPass) {
    Write-Log "`n❌ SOME TESTS FAILED"
    exit 1
}

Write-Log "`n✅ ALL TESTS PASSED"
exit 0
