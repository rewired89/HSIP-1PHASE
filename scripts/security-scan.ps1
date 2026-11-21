# Local security scan of HSIP with Strix. Runs a THROWAWAY instance (temp data, random keys,
# fake accounts) and scans it plus the source. Never point this at production.
#
#   $env:LLM_API_KEY = "sk-..."          # key for your LLM provider
#   .\scripts\security-scan.ps1 -Model "anthropic/claude-opus-4-6"
#
# Needs: Docker running, Rust, Node, and pipx (pip install pipx). Check Strix docs for current model names.
param(
  [string]$Model = $env:STRIX_LLM,
  [int]$Port = 8080,
  [switch]$SourceOnly
)
$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
if (-not $Model) { throw "Pass -Model or set STRIX_LLM (for example anthropic/claude-opus-4-6)." }
if (-not $env:LLM_API_KEY) { throw "Set `$env:LLM_API_KEY first." }
docker ps *> $null; if ($LASTEXITCODE -ne 0) { throw "Docker is not running." }
if (-not (Get-Command strix -ErrorAction SilentlyContinue)) { pipx install strix-agent }
$env:STRIX_LLM = $Model

Push-Location $root
$data = Join-Path $env:TEMP ("hsip-scan-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory $data | Out-Null
$proc = $null
try {
  Push-Location console; npm install; npm run build; Pop-Location
  cargo build --release -p hsip-platform
  $rand = { -join ((1..64) | ForEach-Object { "{0:x}" -f (Get-Random -Max 16) }) }
  $env:HSP_DATA_DIR = $data
  $env:HSP_MASTER_KEY = & $rand
  $env:HSP_OPERATOR_KEY = & $rand
  $env:HSP_CONSOLE_DIR = Join-Path $root "console\dist"
  $env:HSP_BIND = "127.0.0.1:$Port"
  $env:HSP_PUBLIC_URL = "http://host.docker.internal:$Port"
  $proc = Start-Process -FilePath (Join-Path $root "target\release\hsip-platform.exe") -PassThru -WindowStyle Hidden
  $up = $false
  for ($i = 0; $i -lt 30 -and -not $up; $i++) {
    try { Invoke-RestMethod "http://127.0.0.1:$Port/health" | Out-Null; $up = $true } catch { Start-Sleep 1 }
  }
  if (-not $up) { throw "Platform did not start." }

  # Two fake tenants so Strix can test tenant isolation.
  foreach ($n in "a","b") {
    $body = @{ email = "scan-$n@example.test"; password = "Scan-Pass-$n-123456"; name = "Scan $n"; org_name = "Scan Org $n" } | ConvertTo-Json
    Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$Port/v1/auth/signup" -ContentType "application/json" -Body $body | Out-Null
  }
  $instr = "Target is HSIP, a proof platform (Rust/Axum API plus React console). Two throwaway accounts exist: scan-a@example.test / Scan-Pass-a-123456 and scan-b@example.test / Scan-Pass-b-123456 (separate organizations). Priorities: tenant isolation (can A read or change B's orgs, streams, proofs, shares, API keys, evidence, mandates), authentication and session handling, API key scopes, role escalation (auditor, developer, admin, owner), share-link and invite token handling, rate limiting, injection, SSRF, Stripe webhook signature bypass at /v1/stripe/webhook, and information leaks in error messages. Do not attack anything outside this instance. Record content never reaches the platform by design; flag any path that accepts content instead of a payload_hash."
  $targets = @("--target", $root)
  if (-not $SourceOnly) { $targets += @("--target", "http://host.docker.internal:$Port") }
  strix @targets --instruction $instr
  Write-Host "Reports are in strix_runs\. Review every finding by hand before fixing."
}
finally {
  if ($proc -and -not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }
  Pop-Location
  Remove-Item -Recurse -Force $data -ErrorAction SilentlyContinue
}
