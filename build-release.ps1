# HSIP Release Build Script (Windows)
# Run this on your Windows machine to regenerate the binaries.
# Output goes to bin/ — include that folder when zipping for customers.

$ErrorActionPreference = "Stop"

Write-Host "Building HSIP release binaries..." -ForegroundColor Cyan

# Create output folder
New-Item -ItemType Directory -Force -Path "bin" | Out-Null

# Build Windows binary
Write-Host "Building Windows binary..."
cargo build --release -p hsip-api
Copy-Item "target\release\hsip-api.exe" "bin\hsip-api-windows.exe" -Force

Write-Host "Done. Binaries are in the bin/ folder:" -ForegroundColor Green
Get-ChildItem bin | Format-Table Name, Length
