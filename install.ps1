# HSIP installer for Windows
# Usage (run in PowerShell as any user — no admin required):
#   irm https://raw.githubusercontent.com/rewired89/HSIP-1PHASE/main/install.ps1 | iex
#
# What this does:
#   1. Detects your architecture
#   2. Downloads the correct HSIP binary from GitHub Releases
#   3. Installs it to %LOCALAPPDATA%\HSIP\hsip.exe
#   4. Adds that folder to your user PATH (no admin required)
#   5. Verifies the binary runs
#
# To uninstall:
#   Remove-Item -Recurse "$env:LOCALAPPDATA\HSIP"
#   Then remove the HSIP entry from your user PATH in System Properties.

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Repo        = "rewired89/HSIP-1PHASE"
$InstallDir  = Join-Path $env:LOCALAPPDATA "HSIP"
$BinaryName  = "hsip.exe"
$BinaryPath  = Join-Path $InstallDir $BinaryName

function Write-Step  { param($msg) Write-Host "  $msg" -ForegroundColor Cyan }
function Write-Ok    { param($msg) Write-Host "  [OK] $msg" -ForegroundColor Green }
function Write-Warn  { param($msg) Write-Host "  [!]  $msg" -ForegroundColor Yellow }
function Write-Fail  { param($msg) Write-Host "  [X]  $msg" -ForegroundColor Red; exit 1 }

Write-Host ""
Write-Host "HSIP Installer" -ForegroundColor White -BackgroundColor DarkBlue
Write-Host "==============" -ForegroundColor DarkBlue
Write-Host ""

# Detect arch
$Arch = (Get-CimInstance Win32_OperatingSystem).OSArchitecture
if ($Arch -notlike "*64*") {
    Write-Fail "Only 64-bit Windows is supported."
}
Write-Step "Detected: Windows x64"

# Get latest release version
Write-Step "Fetching latest version from GitHub..."
try {
    $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -UseBasicParsing
    $Version = $Release.tag_name
} catch {
    Write-Fail "Could not fetch release info. Check your internet connection or visit: https://github.com/$Repo/releases"
}
Write-Step "Latest version: $Version"

# Build download URL
$AssetName   = "hsip-windows-x64.exe"
$DownloadUrl = "https://github.com/$Repo/releases/download/$Version/$AssetName"

# Create install directory
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}
Write-Step "Install directory: $InstallDir"

# Download
Write-Step "Downloading $AssetName..."
$TempPath = Join-Path $env:TEMP "hsip_download.exe"
try {
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempPath -UseBasicParsing
} catch {
    Write-Fail "Download failed. Verify that $Version has a $AssetName asset at:`n    $DownloadUrl"
}

# Move to install dir
Copy-Item $TempPath $BinaryPath -Force
Remove-Item $TempPath -ErrorAction SilentlyContinue
Write-Ok "Binary installed to $BinaryPath"

# Add to user PATH if not already there
$UserPath = [System.Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$InstallDir*") {
    $NewPath = "$InstallDir;$UserPath"
    [System.Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
    # Also update current session
    $env:PATH = "$InstallDir;$env:PATH"
    Write-Ok "Added $InstallDir to your user PATH"
} else {
    Write-Step "$InstallDir is already in PATH"
}

# Verify
Write-Step "Verifying installation..."
try {
    $null = & $BinaryPath --version 2>&1
    Write-Ok "HSIP is working"
} catch {
    # Some binaries exit non-zero on --version, check existence instead
    if (Test-Path $BinaryPath) {
        Write-Ok "HSIP binary is in place"
    } else {
        Write-Fail "Installation verification failed. Try running: $BinaryPath"
    }
}

Write-Host ""
Write-Host "  Done!" -ForegroundColor Green
Write-Host ""
Write-Host "  Run HSIP:" -ForegroundColor White
Write-Host "    hsip           " -NoNewline; Write-Host "starts the server and opens your browser" -ForegroundColor Gray
Write-Host "    hsip --help    " -NoNewline; Write-Host "server startup options" -ForegroundColor Gray
Write-Host ""
Write-Host "  This is the hsip-api server binary. For the separate hsip-cli tool" -ForegroundColor Gray
Write-Host "  (agent management, trust, key rotation), build it from source:" -ForegroundColor Gray
Write-Host "    cargo build --release -p hsip-cli" -ForegroundColor Gray
Write-Host ""
Write-Host "  Your API key will be saved to: $env:LOCALAPPDATA\HSIP\admin.key" -ForegroundColor Gray
Write-Host "  Docs and API reference: http://127.0.0.1:7474/docs  (once running)" -ForegroundColor Gray
Write-Host ""
Write-Host "  NOTE: Open a new terminal window for PATH changes to take effect." -ForegroundColor Yellow
Write-Host ""
