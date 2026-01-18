@echo off
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
if %errorlevel% neq 0 (
    echo ERROR: Rust toolchain not found. Install from https://rustup.rs
    exit /b 1
)

REM Check for Inno Setup
set "INNO_PATH=C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
if not exist "%INNO_PATH%" (
    echo ERROR: Inno Setup not found at: %INNO_PATH%
    echo Install from: https://jrsoftware.org/isdl.php
    exit /b 1
)

echo Step 1: Building HSIP with all security and audit features...
echo.
cd ..

echo   Building hsip-cli...
cargo build --release -p hsip-cli --features full
if %errorlevel% neq 0 (
    echo ERROR: hsip-cli build failed
    exit /b 1
)

REM Tray icon build commented out (GTK3 dependencies have security warnings)
REM Uncomment if you need tray icon functionality
REM echo   Building hsip-tray...
REM cargo build --release -p hsip-cli --bin hsip-tray --features full,tray
REM if %errorlevel% neq 0 (
REM     echo ERROR: hsip-tray build failed
REM     exit /b 1
REM )

echo   Building hsip-gateway...
cargo build --release -p hsip-gateway
if %errorlevel% neq 0 (
    echo ERROR: hsip-gateway build failed
    exit /b 1
)

echo ✓ All executables built successfully
echo.

echo Step 2: Verifying executables exist...
if not exist "target\release\hsip-cli.exe" (
    echo ERROR: hsip-cli.exe not found
    exit /b 1
)
echo ✓ hsip-cli.exe found

REM Tray verification commented out (not building tray due to GTK3 warnings)
REM if not exist "target\release\hsip-tray.exe" (
REM     echo ERROR: hsip-tray.exe not found
REM     exit /b 1
REM )
REM echo ✓ hsip-tray.exe found

if not exist "target\release\hsip-gateway.exe" (
    echo ERROR: hsip-gateway.exe not found
    exit /b 1
)
echo ✓ hsip-gateway.exe found
echo.

echo Step 3: Creating installer output directory...
cd installer
if not exist "output" mkdir output
echo.

echo Step 4: Running Inno Setup compiler...
echo.
"%INNO_PATH%" hsip-installer.iss
if %errorlevel% neq 0 (
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
echo   ✓ PostgreSQL audit logs (write-once protected)
echo   ✓ NTP time synchronization (±2 seconds)
echo   ✓ Geolocation metadata (MaxMind GeoLite2)
echo   ✓ Enhanced device fingerprinting
echo   ✓ Ed25519 signatures
echo   ✓ ChaCha20-Poly1305 encryption
echo   ✓ Replay attack protection (nonce-based)
echo   ✓ DoS/Injection attack defenses
echo   ✓ Security fixes (RUSTSEC-2025-0132 patched)
echo   ✓ Enhanced error handling (no panic on invalid input)
echo.
echo Documentation included:
echo   ✓ AUDIT_LOG_GUIDE.md (court-ready evidence guide)
echo   ✓ TESTING_GUIDE.md (comprehensive testing)
echo   ✓ INSTALL_WITH_AUDIT_LOGS.md
echo   ✓ Getting Started Guide
echo   ✓ Security Information
echo.
echo Shortcuts created:
echo   - HSIP Command Line
echo   - Export Audit Logs
echo   - Verify Audit Integrity
echo   - Documentation
echo   - Audit Log Guide
echo.
echo ================================================================================
echo.
echo Next steps:
echo   1. Test installer on clean Windows machine
echo   2. Install PostgreSQL
echo   3. Run: hsip-cli audit-export
echo   4. Distribute installer to users
echo.
pause
