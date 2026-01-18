# HSIP Windows Installer Build Instructions

This document explains how to build the HSIP installer with all security features, updates, and licensing.

---

## Prerequisites

### 1. Rust Toolchain (Windows)

Install Rust from: https://rustup.rs

```powershell
# Verify installation
rustc --version
cargo --version
```

**Required:** Rust 1.87 or higher

### 2. Inno Setup Compiler

Download and install Inno Setup 6.0+ from: https://jrsoftware.org/isdl.php

**Default installation path:** `C:\Program Files (x86)\Inno Setup 6\ISCC.exe`

### 3. Build Tools (Optional but Recommended)

For native dependencies:
- Visual Studio Build Tools or
- MinGW-w64

---

## What's Included in the Installer

### Security Features (All Built-In)

✅ **Core Cryptography:**
- Ed25519 digital signatures (non-repudiation)
- ChaCha20-Poly1305 AEAD encryption (Signal-grade)
- X25519 key exchange (perfect forward secrecy)

✅ **Attack Defenses:**
- Replay attack protection (nonce-based counter)
- DoS protection (rate limiting, connection guards)
- Injection attack defenses (input validation)
- Timing attack resistance (constant-time crypto)
- OWASP Top 10 hardening

✅ **Audit & Compliance:**
- PostgreSQL audit logs (write-once protection)
- NTP time synchronization (±2 seconds accuracy)
- Geolocation metadata (MaxMind GeoLite2)
- Device fingerprinting
- Court-ready evidence export

✅ **Recent Security Fixes:**
- Fixed panic vulnerability in ConsentSendRequest (RUSTSEC DoS prevention)
- Updated maxminddb dependency (RUSTSEC-2025-0132 patched)
- Enhanced error handling for malformed inputs
- No crashes on invalid JSON or missing files

### License & Documentation

✅ **Licensing:**
- LICENSE - Free for non-commercial use
- COMMERCIAL_LICENSE.md - Commercial licensing terms
- Clear dual-license model enforced

✅ **Documentation:**
- README.md - Project overview
- SECURITY_AUDIT.md - Security audit report
- GETTING_STARTED.md - Quick start guide
- security_tests/README.md - Attack test report

---

## Build Process

### Step 1: Navigate to Installer Directory

```powershell
cd C:\path\to\HSIP-1PHASE\installer
```

### Step 2: Run Build Script

```powershell
.\build-installer.bat
```

This script will:
1. Build `hsip-cli.exe` with full features (postgres, ntp-sync, geolocation)
2. Build `hsip-tray.exe` (optional, commented out due to GTK3 warnings)
3. Build `hsip-gateway.exe`
4. Compile installer with Inno Setup

**Build time:** 5-10 minutes depending on system

### Step 3: Find the Installer

Output location: `installer\output\HSIP-Setup-1.0.0.exe`

---

## What Gets Built

### Executables

1. **hsip-cli.exe** (~10-15 MB)
   - Main CLI tool with all security features
   - Built with `--features full` flag:
     - `postgres` - PostgreSQL audit log support
     - `ntp-sync` - NTP time synchronization
     - `geolocation` - MaxMind GeoLite2 support

2. **hsip-gateway.exe** (~5-8 MB)
   - Gateway service for network operations
   - Release build (optimized)

3. **hsip-tray.exe** (Optional, ~8-12 MB)
   - System tray status indicator
   - Currently commented out due to GTK3 dependency warnings
   - Enable in `hsip-installer.iss` if needed

### Installer Features

The generated `HSIP-Setup-1.0.0.exe` installer:
- **Size:** ~20-30 MB compressed (LZMA2/max)
- **Requires:** Windows 10/11 64-bit
- **Admin rights:** Yes (for system-wide installation)
- **Silent install:** Supported (`/VERYSILENT /NORESTART`)

---

## Verification

### After Building

```powershell
# Check executables exist
dir ..\target\release\hsip-cli.exe
dir ..\target\release\hsip-gateway.exe

# Check installer was created
dir output\HSIP-Setup-1.0.0.exe
```

### Test the Installer

1. **Test on clean Windows VM:**
   - Windows 10/11 64-bit
   - No Rust toolchain installed
   - Fresh user profile

2. **Install and verify:**
   ```powershell
   # Run installer
   HSIP-Setup-1.0.0.exe

   # After installation, test CLI
   cd "C:\Program Files\HSIP"
   .\hsip-cli.exe --version
   .\hsip-cli.exe init
   .\hsip-cli.exe whoami
   ```

3. **Check shortcuts:**
   - Start Menu → HSIP folder
   - Documentation links work
   - License files present

---

## Features Verified in This Build

### Security Fixes Included

✅ **Panic Vulnerability Fixed** (main.rs:830-843)
- `ConsentSendRequest` no longer panics on invalid input
- Clean error handling for missing/malformed files
- Exit code 1 instead of process crash

✅ **Dependency Updates**
- maxminddb: 0.24.0 → 0.27.1 (RUSTSEC-2025-0132)
- Geolocation API updated
- No unmaintained critical dependencies

✅ **Attack Defenses Tested**
- Replay attacks: ✓ Blocked
- Injection attacks: ✓ Rejected
- DoS/flooding: ✓ Rate limited
- Tampered responses: ✓ AEAD auth fails
- Invalid JSON: ✓ Clean error (no panic)

### License Enforcement

✅ **All permissive licenses removed:**
- No MIT in Cargo.toml files
- No Apache-2.0 in source headers
- All crates reference LICENSE file

✅ **Dual-license model:**
- Source headers: "Free for non-commercial use. Commercial use requires a license."
- LICENSE file included in installer
- COMMERCIAL_LICENSE.md included in installer

---

## Troubleshooting

### Build Fails: "cargo not found"

**Fix:** Install Rust toolchain from https://rustup.rs

```powershell
rustup update
rustup default stable
```

### Build Fails: "Inno Setup not found"

**Fix:** Install Inno Setup or update path in `build-installer.bat`:

```batch
set "INNO_PATH=C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
```

### Build Fails: "linking failed"

**Fix:** Install Visual Studio Build Tools:
- Download: https://visualstudio.microsoft.com/downloads/
- Install "Desktop development with C++"

### Build Succeeds but Installer Missing

**Check:** `installer\output\` directory exists

```powershell
mkdir output
.\build-installer.bat
```

### GTK3 Dependency Warnings

**Note:** hsip-tray is commented out in the installer due to unmaintained GTK3 dependencies (RUSTSEC-2024-0412 through 0420).

**If you need tray icon:**
1. Uncomment tray icon lines in `hsip-installer.iss`
2. Build with `--features full,tray`
3. Accept the GTK3 warnings (low risk for optional feature)

---

## Distribution

### Before Distributing

1. ✅ Test installer on clean Windows VM
2. ✅ Verify all shortcuts work
3. ✅ Check LICENSE file is visible
4. ✅ Test basic commands (init, whoami, ping)
5. ✅ Verify PostgreSQL requirement is documented

### Upload Locations

- **GitHub Releases:** https://github.com/nyxsystems/HSIP-1PHASE/releases
- **Website:** https://hsip.io/download
- **Direct download:** Provide SHA256 hash for verification

### SHA256 Hash

```powershell
# Generate hash for users to verify
Get-FileHash .\output\HSIP-Setup-1.0.0.exe -Algorithm SHA256
```

Include this hash in release notes.

---

## Advanced: Custom Builds

### Build with Specific Features

```powershell
# CLI only (no postgres, no geolocation)
cargo build --release -p hsip-cli

# CLI with postgres only
cargo build --release -p hsip-cli --features postgres

# CLI with all features (same as installer)
cargo build --release -p hsip-cli --features full
```

### Manual Inno Setup Compilation

```powershell
"C:\Program Files (x86)\Inno Setup 6\ISCC.exe" hsip-installer.iss
```

### Silent Install (IT Departments)

```powershell
# Install without user interaction
HSIP-Setup-1.0.0.exe /VERYSILENT /NORESTART /LOG="install.log"

# Uninstall silently
"C:\Program Files\HSIP\unins000.exe" /VERYSILENT /NORESTART
```

---

## Support

**Build Issues:**
- GitHub Issues: https://github.com/nyxsystems/HSIP-1PHASE/issues
- Email: nyxsystemsllc@gmail.com

**Licensing Questions:**
- Commercial licensing: See COMMERCIAL_LICENSE.md
- Email: nyxsystemsllc@gmail.com

**Security Issues:**
- Private disclosure: nyxsystemsllc@gmail.com
- Response time: 48 hours

---

## Changelog

**v1.0.0 (Current)**
- Fixed panic vulnerability in ConsentSendRequest
- Updated maxminddb (RUSTSEC-2025-0132)
- Enforced dual-license model
- Removed all permissive license references
- Added COMMERCIAL_LICENSE.md
- Enhanced installer with security audit information

---

**Build verified:** January 2026
**Next review:** After each security update
