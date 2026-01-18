# HSIP Windows Installer - Build & Validation Guide

**Company:** Nyx Systems LLC
**Contact:** nyxsystemsllc@gmail.com
**Version:** 1.0.0

---

## What This Installer Does

Creates a professional Windows installer for HSIP that:
- ✅ Installs all executables with full security features
- ✅ Starts HSIP silently in background (no console windows)
- ✅ Auto-starts on Windows login (daemon + gateway)
- ✅ Includes all documentation and licenses
- ✅ Clean uninstall (removes startup entries)

---

## Build Requirements

### 1. Rust Toolchain
- **Required:** Rust 1.87 or higher
- Install from: https://rustup.rs

```powershell
# Verify installation
rustc --version
cargo --version
```

### 2. Inno Setup Compiler
- **Required:** Inno Setup 6.0 or higher
- Install from: https://jrsoftware.org/isdl.php
- **Default paths checked:**
  - `C:\Program Files (x86)\Inno Setup 6\ISCC.exe`
  - `C:\Program Files\Inno Setup 6\ISCC.exe`
  - `C:\Program Files (x86)\Inno Setup 5\ISCC.exe`

---

## Build Instructions

### Step 1: Navigate to Installer Directory

```powershell
cd C:\path\to\HSIP-1PHASE\installer
```

### Step 2: Run Build Script

```powershell
.\build-installer.bat
```

**Build process:**
1. Checks for Rust and Inno Setup
2. Builds `hsip-cli.exe` with full features (postgres, ntp-sync, geolocation)
3. Builds `hsip-gateway.exe`
4. Compiles installer with Inno Setup
5. Creates `installer\output\HSIP-Setup-1.0.0.exe`

**Build time:** 5-10 minutes

### Step 3: Find Output

Installer location: `installer\output\HSIP-Setup-1.0.0.exe`

---

## What Gets Installed

### Executables (in C:\Program Files\HSIP\)

1. **hsip-cli.exe** (~10-15 MB)
   - Main CLI tool with all security features
   - Built with `--features full`:
     - PostgreSQL audit logging
     - NTP time synchronization
     - MaxMind geolocation support
   - All security patches included

2. **hsip-gateway.exe** (~5-8 MB)
   - Gateway service for network operations
   - Release build (optimized)

3. **launch-hidden.vbs**
   - VBScript launcher for silent startup
   - Prevents console windows from appearing

### Documentation

- `LICENSE` - Free for non-commercial use
- `COMMERCIAL_LICENSE.md` - Commercial licensing terms
- `README.md` - Project overview
- `SECURITY_AUDIT.md` - Security audit report
- `GETTING_STARTED.md` - Quick start guide
- `SECURITY.md` - Security information
- `TESTING_GUIDE.md` - Testing documentation
- `AUDIT_LOG_GUIDE.md` - Audit logging guide

### Start Menu Shortcuts

- HSIP Command Line
- Documentation
- License (Free Non-Commercial)
- Commercial License Info
- Security Audit Report
- Getting Started
- Uninstall HSIP

---

## Silent Background Startup

### How It Works

The installer creates registry entries for auto-start:
- `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\HSIP Daemon`
- `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\HSIP Gateway`

**Startup mechanism:**
```
wscript.exe launch-hidden.vbs "hsip-cli.exe" "daemon"
wscript.exe launch-hidden.vbs "hsip-gateway.exe" ""
```

**Benefits:**
- ✅ No console windows (completely hidden)
- ✅ Starts automatically on Windows login
- ✅ Runs in background silently
- ✅ Clean uninstall (registry entries removed)

### Verification After Install

1. **Check processes running:**
   ```powershell
   tasklist | findstr hsip
   ```
   Should show: `hsip-cli.exe` and `hsip-gateway.exe`

2. **No console windows should be visible**
   - Check taskbar - no black windows
   - Check Task Manager - processes run without windows

3. **After reboot:**
   - Processes auto-start
   - Still no console windows

---

## Tray Icon Status Colors

**NOTE:** Tray icon currently disabled due to GTK3 dependency warnings.
**If re-enabled, colors mean:**

- 🔴 **RED** = HSIP offline or error (daemon not running)
- 🟡 **YELLOW** = Active threats being blocked
- 🟢 **GREEN** = Protected and secure (no threats)

**Status is checked every 3 seconds via HTTP://127.0.0.1:8787/status**

---

## Validation Checklist

After building installer, test on clean Windows 10/11 machine:

### Install Validation

- [ ] Run `HSIP-Setup-1.0.0.exe`
- [ ] Installer shows license agreement
- [ ] Installation completes without errors
- [ ] Files exist in `C:\Program Files\HSIP\`
- [ ] Start Menu shortcuts created

### Silent Startup Validation

- [ ] After install: Check Task Manager
  - `hsip-cli.exe` running
  - `hsip-gateway.exe` running
- [ ] No console/terminal windows visible
- [ ] No black windows in taskbar
- [ ] Reboot Windows
- [ ] After reboot: Processes auto-start
- [ ] Still no console windows

### Uninstall Validation

- [ ] Run uninstaller from Start Menu
- [ ] Confirmation dialog appears
- [ ] Uninstall completes
- [ ] Files removed from `C:\Program Files\HSIP\`
- [ ] Registry entries removed (check with regedit)
- [ ] Reboot Windows
- [ ] HSIP processes do NOT auto-start
- [ ] Config preserved in `%APPDATA%\.hsip\` (intentional)

---

## Troubleshooting

### Build Fails: "Rust toolchain not found"

**Fix:** Install Rust from https://rustup.rs

```powershell
# Update Rust
rustup update
rustup default stable
```

### Build Fails: "Inno Setup compiler not found"

**Fix:** Install Inno Setup or add to PATH

**Manual check:**
```powershell
dir "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
```

If installed elsewhere, script will auto-detect common locations.

### Build Succeeds but Installer Missing

**Fix:** Check output directory exists

```powershell
cd installer
dir output\HSIP-Setup-1.0.0.exe
```

### Console Windows Appear After Install

**Diagnosis:** VBScript launcher not working

**Fix:** Check registry entries:
```powershell
reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v "HSIP Daemon"
```

Should show: `wscript.exe` command (NOT PowerShell or direct exe)

### Processes Don't Auto-Start After Reboot

**Fix:** Check "Start HSIP automatically" was selected during install

**Manual enable:**
1. Run installer again
2. Select "Start HSIP automatically when Windows starts"
3. Complete installation

---

## Security Features Included

All builds include these security features:

✅ **Cryptography:**
- Ed25519 digital signatures (non-repudiation)
- ChaCha20-Poly1305 AEAD encryption
- X25519 key exchange (perfect forward secrecy)

✅ **Attack Defenses:**
- Replay attack protection (nonce-based counter)
- DoS protection (rate limiting, connection guards)
- Injection attack defenses (input validation)
- Timing attack resistance (constant-time crypto)
- OWASP Top 10 hardening

✅ **Audit & Compliance:**
- PostgreSQL audit logs (write-once protection)
- NTP time synchronization (±2 seconds)
- Geolocation metadata (MaxMind GeoLite2)
- Court-ready evidence export

✅ **Recent Security Fixes:**
- Fixed panic vulnerability in ConsentSendRequest
- Updated maxminddb (RUSTSEC-2025-0132)
- Enhanced error handling (no crashes on invalid input)

---

## License Information

**For End Users:**
- ✅ FREE for personal, educational, non-commercial use
- ❌ Commercial use requires license from Nyx Systems LLC

**Documentation included in installer:**
- `LICENSE` - HSIP Commons License (non-commercial)
- `COMMERCIAL_LICENSE.md` - Commercial terms and pricing

**Contact for commercial licensing:**
- Email: nyxsystemsllc@gmail.com
- See COMMERCIAL_LICENSE.md for details

---

## Distribution

### Before Distributing

1. ✅ Test on clean Windows VM
2. ✅ Verify no console windows
3. ✅ Test reboot auto-start
4. ✅ Test uninstall
5. ✅ Generate SHA256 hash

### Generate Hash for Verification

```powershell
Get-FileHash .\output\HSIP-Setup-1.0.0.exe -Algorithm SHA256
```

Include hash in release notes so users can verify integrity.

### Upload Locations

- GitHub Releases: https://github.com/nyxsystems/HSIP-1PHASE/releases
- Website: https://hsip.io/download (if applicable)
- Include SHA256 hash in download page

---

## Support

**Build Issues:**
- GitHub: https://github.com/nyxsystems/HSIP-1PHASE/issues
- Email: nyxsystemsllc@gmail.com

**Licensing Questions:**
- Email: nyxsystemsllc@gmail.com
- See COMMERCIAL_LICENSE.md

**Security Issues:**
- Email: nyxsystemsllc@gmail.com (private disclosure)
- Response time: 48 hours

---

## Files Modified

This installer build system consists of:

1. **build-installer.bat** - Main build script
   - Checks for Rust and Inno Setup
   - Builds executables with full features
   - Compiles installer

2. **hsip-installer.iss** - Inno Setup script
   - Defines what gets installed
   - Sets up silent startup (no console windows)
   - Creates Start Menu shortcuts
   - Handles uninstall

3. **launch-hidden.vbs** - VBScript launcher
   - Launches processes with no visible windows
   - Used for silent background startup

4. **INSTALLER_README.md** - This file
   - Build instructions
   - Validation checklist
   - Troubleshooting guide

---

**Build verified:** January 2026
**Next review:** After each security update
