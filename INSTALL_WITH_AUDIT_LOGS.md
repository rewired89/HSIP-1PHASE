# Installing HSIP with Court-Ready Audit Logs

## Quick Start (Windows)

### Option 1: Download Pre-Built Installer (Recommended)

**Coming Soon**: Windows installer with all features enabled will be available on our releases page.

For now, build from source using Option 2 below.

---

### Option 2: Build From Source

#### Prerequisites

1. **Install Rust**
   ```powershell
   # Download from https://rustup.rs or:
   winget install Rustlang.Rustup
   ```

2. **Install PostgreSQL**
   ```powershell
   # Download from https://www.postgresql.org/download/windows/
   # Or use chocolatey:
   choco install postgresql
   ```

3. **Install Git**
   ```powershell
   winget install Git.Git
   ```

#### Build HSIP with Full Features

```powershell
# Clone repository
git clone https://github.com/HSIP/hsip.git
cd hsip

# Build with all security and audit features
cargo build --release -p hsip-cli --features full
cargo build --release -p hsip-cli --bin hsip-tray --features full,tray
cargo build --release -p hsip-gateway

# Binaries will be at:
# target\release\hsip-cli.exe     (main CLI and daemon)
# target\release\hsip-tray.exe    (system tray icon)
# target\release\hsip-gateway.exe (proxy gateway)
```

The `--features full` flag enables:
- ✅ PostgreSQL audit logs (write-once)
- ✅ NTP time synchronization (±2 seconds)
- ✅ Geolocation metadata (MaxMind GeoLite2)
- ✅ Enhanced device fingerprinting
- ✅ All cryptographic features

---

## Setting Up Audit Logs

### 1. Create PostgreSQL Database

After installing PostgreSQL:

```powershell
# Open PowerShell as Administrator
# Create database
& "C:\Program Files\PostgreSQL\16\bin\createdb.exe" hsip_audit

# Or create manually:
& "C:\Program Files\PostgreSQL\16\bin\psql.exe" -U postgres
# Then run: CREATE DATABASE hsip_audit;
```

### 2. Set Database Connection

**Option A: Environment Variable (Recommended)**

```powershell
# Add to PowerShell profile (permanent):
$env:DATABASE_URL = "postgresql://localhost/hsip_audit"

# Or use Windows System Properties:
# 1. Win+Pause → Advanced system settings
# 2. Environment Variables
# 3. Add new system variable:
#    Name: DATABASE_URL
#    Value: postgresql://localhost/hsip_audit
```

**Option B: Pass on Command Line**

```powershell
.\hsip-cli.exe audit-export --db "postgresql://localhost/hsip_audit"
```

---

## Using Audit Log Commands

### Export for Court Evidence

```powershell
# Export all audit logs
.\hsip-cli.exe audit-export --out evidence.json

# Export last 1000 entries
.\hsip-cli.exe audit-export --out recent.json --limit 1000
```

### Verify Integrity

```powershell
# Verify chain hasn't been tampered with
.\hsip-cli.exe audit-verify
```

**Output:**
```
[AUDIT] Verifying chain integrity...
[AUDIT] ✅ Chain integrity verified
[AUDIT] 1523 entries checked - no tampering detected
```

### Search History

```powershell
# Search by destination
.\hsip-cli.exe audit-query --destination "facebook.com"

# Search by decision type
.\hsip-cli.exe audit-query --decision "Block" --limit 100

# Combined search
.\hsip-cli.exe audit-query --destination "analytics" --decision "Block"
```

### Show Statistics

```powershell
.\hsip-cli.exe audit-stats
```

---

## What Users Get

### NO Contact with HSIP Required

Users have **complete control** of their audit logs:

✅ Stored **locally** on your computer (PostgreSQL database)
✅ Accessible via **hsip-cli commands** (no website, no account)
✅ **Export anytime** to JSON for court evidence
✅ **Verify integrity** with cryptographic proof
✅ **Full privacy** - HSIP developers have zero access

### Court-Ready Evidence

Every audit log entry includes:

- **Ed25519 signature** - Cryptographic proof of authenticity
- **BLAKE3 chain hash** - Tamper detection
- **NTP-synced timestamp** - Accurate to ±2 seconds
- **Decision reason** - Why consent was granted/denied
- **Destination info** - Domain, IP, geolocation

### Legal Use Cases

✅ **GDPR Consent Disputes**
   - Prove user granted or revoked consent
   - Show exact timestamp of consent decision
   - Demonstrate compliance with Article 7

✅ **Message Authenticity**
   - Ed25519 signatures provide non-repudiation
   - Prove message sender identity
   - Verify message content integrity

✅ **Phishing/Unauthorized Access**
   - Document blocked connection attempts
   - Show source IP and geolocation
   - Provide device fingerprints

✅ **Privacy Compliance**
   - CCPA opt-out enforcement
   - Data minimization evidence
   - Cryptographic consent enforcement

---

## Example Workflow

### Preparing Evidence for Court

```powershell
# 1. Create evidence folder
mkdir court_evidence_20260113
cd court_evidence_20260113

# 2. Export complete audit log
..\hsip-cli.exe audit-export --out full_audit_log.json --limit 0

# 3. Verify integrity and save proof
..\hsip-cli.exe audit-verify > chain_verification.txt

# 4. Copy documentation
copy ..\AUDIT_LOG_GUIDE.md .
copy ..\README.md .

# 5. Create evidence package
Compress-Archive -Path * -DestinationPath ..\hsip_evidence_20260113.zip
```

Now you have a **court-ready evidence package** with:
- Complete audit trail (full_audit_log.json)
- Integrity verification proof (chain_verification.txt)
- Technical documentation (AUDIT_LOG_GUIDE.md, README.md)

---

## Troubleshooting

### "PostgreSQL audit logs not enabled"

**Problem**: Built without postgres feature

**Solution**:
```powershell
cargo clean
cargo build --release --features full
```

### "Failed to connect to database"

**Problem**: PostgreSQL not running or database doesn't exist

**Solution**:
```powershell
# Check PostgreSQL service
Get-Service postgresql-x64-16

# Start if stopped
Start-Service postgresql-x64-16

# Create database
& "C:\Program Files\PostgreSQL\16\bin\createdb.exe" hsip_audit
```

### "DATABASE_URL not set"

**Problem**: No connection string configured

**Solution**:
```powershell
# Set environment variable
$env:DATABASE_URL = "postgresql://localhost/hsip_audit"

# Or pass on command line
.\hsip-cli.exe audit-export --db "postgresql://localhost/hsip_audit"
```

---

## Building Windows Installer

### For Developers

To build a Windows installer with all features:

```powershell
# Option 1: Use the build script (recommended)
cd installer
.\build-installer.bat

# Option 2: Manual build
# 1. Build all executables with full features
cargo build --release -p hsip-cli --features full
cargo build --release -p hsip-cli --bin hsip-tray --features full,tray
cargo build --release -p hsip-gateway

# 2. Run Inno Setup
cd installer
& "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" hsip-installer.iss

# Installer will be in: installer\output\HSIP-Setup-1.0.0.exe
```

**Note:** The build must be done on Windows as hsip-tray requires Windows-specific GUI libraries.

### Installer Features

The Windows installer includes:
- ✅ hsip-cli.exe (CLI with all security and audit features)
- ✅ hsip-tray.exe (system tray status icon)
- ✅ hsip-gateway.exe (SOCKS5/HTTP proxy gateway)
- ✅ PowerShell auto-start scripts
- ✅ PostgreSQL audit log support
- ✅ Desktop shortcuts
- ✅ Start menu entries for audit commands
- ✅ Complete documentation (AUDIT_LOG_GUIDE.md, README.md)
- ✅ Security test scripts

---

## Additional Resources

### Documentation

- **AUDIT_LOG_GUIDE.md** - Complete guide to court-ready evidence
- **TESTING_GUIDE.md** - Comprehensive testing instructions
- **README.md** - Project overview and getting started

### Support

- **Email**: support@hsip.io
- **Legal Questions**: legal@hsip.io
- **GitHub Issues**: https://github.com/HSIP/hsip/issues

### Expert Testimony

For legal proceedings requiring expert technical testimony, contact legal@hsip.io.

---

## Summary

### What You Get

✅ **Full control** - Audit logs stored locally on your computer
✅ **No third-party** - Access logs via CLI, no website/account needed
✅ **Court-ready** - Cryptographic proof of authenticity
✅ **Enterprise-grade** - All security features implemented and tested

### How to Access Audit Logs

```powershell
# Export for court
hsip-cli audit-export --out evidence.json

# Verify integrity
hsip-cli audit-verify

# Search history
hsip-cli audit-query --destination "example.com"

# Show statistics
hsip-cli audit-stats
```

### Build Command

```powershell
cargo build --release --features full
```

That's it! Users have complete autonomy over their audit logs with cryptographic proof of authenticity.

---

**Last Updated**: January 14, 2026
**HSIP Version**: Phase 1 (v0.1.2)
**Features**: All security and audit features implemented
