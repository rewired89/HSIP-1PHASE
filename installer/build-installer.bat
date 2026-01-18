@echo off
setlocal enabledelayedexpansion

REM Build HSIP Windows Installer with Full Security Features
REM This script builds the executable with all features and creates the installer

echo ================================================================================
echo      HSIP Windows Installer Builder - Full Features Edition
echo ================================================================================
echo.

REM Check if we're in the installer directory
if not exist "hsip-installer.iss" (
    echo ERROR: Must run this script from the installer directory
    exit /b 1
)

REM Check for Rust toolchain
where cargo >nul 2>&1
if errorlevel 1 (
    echo ERROR: Rust toolchain not found. Install from https://rustup.rs
    exit /b 1
)

REM Check for Inno Setup (try multiple common locations)
set "ISCC_EXE="
if exist "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" set "ISCC_EXE=C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
if exist "C:\Program Files\Inno Setup 6\ISCC.exe" set "ISCC_EXE=C:\Program Files\Inno Setup 6\ISCC.exe"
if exist "C:\Program Files (x86)\Inno Setup 5\ISCC.exe" set "ISCC_EXE=C:\Program Files (x86)\Inno Setup 5\ISCC.exe"

if "!ISCC_EXE!"=="" (
    echo ERROR: Inno Setup compiler not found in standard locations
    echo.
    echo Looked in:
    echo   - C:\Program Files ^(x86^)\Inno Setup 6\
    echo   - C:\Program Files\Inno Setup 6\
    echo   - C:\Program Files ^(x86^)\Inno Setup 5\
    echo.
    echo Install from: https://jrsoftware.org/isdl.php
    exit /b 1
)

echo Found Inno Setup: !ISCC_EXE!
echo.

echo Step 1: Building HSIP with all security and audit features...
echo.
cd ..

echo   Building hsip-cli with full features...
cargo build --release -p hsip-cli --features full
if errorlevel 1 (
    echo ERROR: hsip-cli build failed
    exit /b 1
)

echo   Building hsip-gateway...
cargo build --release -p hsip-gateway
if errorlevel 1 (
    echo ERROR: hsip-gateway build failed
    exit /b 1
)

echo.
echo Step 2: Verifying executables exist...
if not exist "target\release\hsip-cli.exe" (
    echo ERROR: hsip-cli.exe not found
    exit /b 1
)
echo   [OK] hsip-cli.exe found

if not exist "target\release\hsip-gateway.exe" (
    echo ERROR: hsip-gateway.exe not found
    exit /b 1
)
echo   [OK] hsip-gateway.exe found
echo.

echo Step 3: Creating installer output directory...
cd installer
if not exist "output" mkdir output
echo.

echo Step 4: Running Inno Setup compiler...
echo.
"!ISCC_EXE!" hsip-installer.iss
if errorlevel 1 (
    echo ERROR: Inno Setup compilation failed
    exit /b 1
)
echo.

echo ================================================================================
echo                    BUILD COMPLETED SUCCESSFULLY!
echo ================================================================================
echo.
echo Installer created at:
echo   installer\output\HSIP-Setup-1.0.0.exe
echo.
echo Features included:
echo   - PostgreSQL audit logs (write-once protected)
echo   - NTP time synchronization (±2 seconds)
echo   - Geolocation metadata (MaxMind GeoLite2)
echo   - Ed25519 signatures / ChaCha20-Poly1305 encryption
echo   - Replay attack protection (nonce-based)
echo   - DoS/Injection attack defenses
echo   - Security fixes (RUSTSEC-2025-0132 patched)
echo   - Enhanced error handling (no panic on invalid input)
echo.
echo Documentation included:
echo   - LICENSE (Free for non-commercial use)
echo   - COMMERCIAL_LICENSE.md
echo   - SECURITY_AUDIT.md
echo   - Getting Started, Testing, and Security guides
echo.
echo ================================================================================
echo.
echo Next steps:
echo   1. Test installer on clean Windows machine
echo   2. Verify no console windows appear on startup
echo   3. Check tray icon colors (Red=offline, Green=online)
echo   4. Distribute installer to users
echo.
pause
