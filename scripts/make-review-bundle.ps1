# Builds plain-text bundles of HSIP's most security-relevant files so they can
# be pasted or uploaded to an outside reviewer (DeepSeek, another LLM, a human).
# Run from the repo root:   pwsh ./scripts/make-review-bundle.ps1
# Output: ./review-bundles/*.txt  (git-ignored)
# Secrets are never included: only the files listed below are read.

$ErrorActionPreference = 'Stop'
$out = Join-Path (Get-Location) 'review-bundles'
New-Item -ItemType Directory -Force -Path $out | Out-Null

$bundles = [ordered]@{
  '1-proof-format-and-verifier' = @(
    'docs/PROVE_PROOF_FORMAT.md',
    'crates/hsip-proof/src/lib.rs',
    'crates/hsip-proof/src/merkle.rs',
    'crates/hsip-proof/src/bin/hsip_proof.rs',
    'crates/hsip-proof/Cargo.toml',
    'docs/examples/prove/sample-proof.json'
  )
  '2a-platform-core' = @(
    'docs/PROVE_PLATFORM.md',
    'crates/hsip-platform/Cargo.toml',
    'crates/hsip-platform/src/lib.rs',
    'crates/hsip-platform/src/config.rs',
    'crates/hsip-platform/src/crypto.rs',
    'crates/hsip-platform/src/state.rs',
    'crates/hsip-platform/src/auth.rs',
    'crates/hsip-platform/src/errors.rs',
    'crates/hsip-platform/src/db.rs',
    'crates/hsip-platform/src/qb.rs',
    'crates/hsip-platform/src/engine.rs',
    'crates/hsip-platform/src/audit.rs',
    'crates/hsip-platform/src/anchor.rs',
    'crates/hsip-platform/src/report.rs',
    'crates/hsip-platform/src/ui.rs'
  )
  '2b-platform-routes' = @(
    'crates/hsip-platform/src/routes/mod.rs',
    'crates/hsip-platform/src/routes/account.rs',
    'crates/hsip-platform/src/routes/orgs.rs',
    'crates/hsip-platform/src/routes/streams.rs',
    'crates/hsip-platform/src/routes/proofs.rs',
    'crates/hsip-platform/src/routes/keys.rs',
    'crates/hsip-platform/src/routes/team.rs',
    'crates/hsip-platform/src/routes/evidence.rs',
    'crates/hsip-platform/src/routes/public.rs',
    'crates/hsip-platform/src/routes/operator.rs'
  )
  '3-console-and-sdks' = @(
    'console/src/api.js',
    'console/src/auth.jsx',
    'console/src/Shell.jsx',
    'console/src/pages/ProofView.jsx',
    'console/src/pages/Operator.jsx',
    'console/src/pages/Verify.jsx',
    'sdk/prove-python/hsip_prove.py',
    'sdk/prove-node/index.mjs'
  )
  '4-tests' = @(
    'crates/hsip-proof/tests/samples.rs',
    'crates/hsip-platform/tests/common/mod.rs',
    'crates/hsip-platform/tests/integration.rs'
  )
}

foreach ($name in $bundles.Keys) {
  $sb = New-Object System.Text.StringBuilder
  foreach ($f in $bundles[$name]) {
    if (-not (Test-Path $f)) { Write-Warning "missing: $f"; continue }
    [void]$sb.AppendLine("")
    [void]$sb.AppendLine("=================== FILE: $f ===================")
    [void]$sb.AppendLine((Get-Content -Raw -Encoding UTF8 $f))
  }
  $path = Join-Path $out "$name.txt"
  Set-Content -Path $path -Value $sb.ToString() -Encoding UTF8
  $kb = [math]::Round((Get-Item $path).Length / 1KB)
  # Rough size guide: about 4 characters per token.
  Write-Host ("{0,-34} {1,6} KB  (~{2,6} tokens)" -f "$name.txt", $kb, ($kb * 256))
}
Write-Host "`nBundles are in: $out"
