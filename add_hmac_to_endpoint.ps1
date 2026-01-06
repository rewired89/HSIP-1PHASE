# Helper to add HMAC to specific endpoint
param(
    [Parameter(Mandatory=$true)]
    [string]$EndpointFunction
)

$daemonFile = "crates\hsip-cli\src\daemon\mod.rs"
$content = Get-Content $daemonFile -Raw

# Check if mod hmac already added
if ($content -notmatch 'mod hmac;') {
    Write-Host "Adding hmac module declaration..." -ForegroundColor Yellow
    $content = "mod hmac;`nuse hmac::create_signed_json;`n`n" + $content
    Set-Content $daemonFile $content
    Write-Host "✓ Added hmac module" -ForegroundColor Green
}

Write-Host ""
Write-Host "Manual steps for endpoint: $EndpointFunction" -ForegroundColor Cyan
Write-Host "1. Find the function in daemon/mod.rs"
Write-Host "2. Wrap the return value with create_signed_json()"
Write-Host "3. Handle the Result<String, String>"
Write-Host ""
Write-Host "See HMAC_INTEGRATION_GUIDE.md for examples"
