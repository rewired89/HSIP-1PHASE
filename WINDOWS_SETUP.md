# HSIP API - Windows Setup Guide

Quick start guide for running HSIP API on Windows with PowerShell.

## Prerequisites

- Rust installed (`rustup` via https://rustup.rs)
- OpenSSL for Windows (optional, for TLS)

---

## Quick Start (No TLS - Development Only)

### 1. Generate Master Key

```powershell
# Generate 32-byte master encryption key
openssl rand -hex 32 > hsip_master_key.bin

# If OpenSSL not installed, use PowerShell:
$bytes = New-Object byte[] 32
[System.Security.Cryptography.RNGCryptoServiceProvider]::Create().GetBytes($bytes)
($bytes | ForEach-Object { $_.ToString("x2") }) -join '' | Out-File -Encoding ASCII -NoNewline hsip_master_key.bin
```

### 2. Use Development Config

The `config.toml` file is already configured for local development:
- TLS disabled (HTTP only)
- SQLite in-memory database
- Localhost binding only

```powershell
# View config
cat config.toml
```

### 3. Build and Run

```powershell
# Build release binary
cargo build --release -p hsip-api --features hsip-api/embed-dashboard

# Run server
.\target\release\hsip-api.exe
```

**Expected output:**

```
⚠️  TLS is DISABLED - this is insecure for production!
   Configure [server.tls] in config.toml to enable HTTPS
🚀 HSIP API listening on http://127.0.0.1:3000
   Docs:    http://127.0.0.1:3000/docs
   Metrics: http://127.0.0.1:3000/metrics
   Health:  http://127.0.0.1:3000/health
```

### 4. Test the API

```powershell
# Health check
Invoke-WebRequest http://localhost:3000/health

# Get admin key (shown on first run)
$adminKey = Get-Content "$env:APPDATA\HSIP\admin.key"

# Create identity
Invoke-RestMethod -Method POST `
  -Uri http://localhost:3000/v1/identity `
  -Headers @{ "Authorization" = "Bearer $adminKey" }
```

---

## Enable TLS/HTTPS (Production)

### Option 1: OpenSSL Self-Signed Certificate

```powershell
# Single-line command (no backslashes in PowerShell)
openssl req -x509 -newkey rsa:4096 -nodes -keyout key.pem -out cert.pem -days 365 -subj "/CN=localhost"
```

### Option 2: PowerShell Self-Signed Certificate

```powershell
# Generate self-signed certificate (PowerShell native)
$cert = New-SelfSignedCertificate -DnsName "localhost" -CertStoreLocation "Cert:\CurrentUser\My" -NotAfter (Get-Date).AddYears(1)

# Export certificate and private key
$certPath = "cert.pem"
$keyPath = "key.pem"

# Export cert
Export-Certificate -Cert $cert -FilePath "cert.cer"
certutil -encode cert.cer $certPath
Remove-Item cert.cer

# Export private key (requires manual password - leave empty for no password)
$pwd = ConvertTo-SecureString -String "" -Force -AsPlainText
Export-PfxCertificate -Cert $cert -FilePath "cert.pfx" -Password $pwd
openssl pkcs12 -in cert.pfx -nodes -out $keyPath -nocerts -passin pass:
Remove-Item cert.pfx
```

**Simpler: Just use OpenSSL** (if you have Git for Windows, OpenSSL is included)

### Enable TLS in config.toml

```toml
# Uncomment this section in config.toml:
[server.tls]
cert_path = "cert.pem"
key_path = "key.pem"
require_https = true
```

Then restart the server:

```powershell
.\target\release\hsip-api.exe
```

---

## Using PostgreSQL (Production)

### 1. Install PostgreSQL for Windows

Download from: https://www.postgresql.org/download/windows/

### 2. Create Database

```powershell
# Open psql shell
psql -U postgres
```

```sql
CREATE DATABASE hsip_db;
CREATE USER hsip_user WITH ENCRYPTED PASSWORD 'your_secure_password';
GRANT ALL PRIVILEGES ON DATABASE hsip_db TO hsip_user;
\q
```

### 3. Update config.toml

```toml
[database]
url = "postgresql://hsip_user:your_secure_password@localhost/hsip_db"
max_connections = 10
run_migrations = true
```

### 4. Restart Server

```powershell
.\target\release\hsip-api.exe
```

---

## Running as Windows Service

### Option 1: NSSM (Non-Sucking Service Manager)

```powershell
# Download NSSM from https://nssm.cc/download
# Install service
nssm install HSIP-API "C:\path\to\hsip-api.exe"
nssm set HSIP-API AppDirectory "C:\path\to\HSIP-1PHASE"
nssm start HSIP-API
```

### Option 2: Task Scheduler

1. Open Task Scheduler
2. Create Basic Task → "HSIP API"
3. Trigger: "When the computer starts"
4. Action: "Start a program"
5. Program: `C:\path\to\target\release\hsip-api.exe`
6. Start in: `C:\path\to\HSIP-1PHASE`

---

## Load Testing on Windows

### Simple Benchmark (PowerShell Native)

```powershell
# Test health endpoint
$adminKey = Get-Content "$env:APPDATA\HSIP\admin.key"
$start = Get-Date

1..100 | ForEach-Object {
    Invoke-WebRequest -Uri http://localhost:3000/health -UseBasicParsing | Out-Null
    if ($_ % 10 -eq 0) { Write-Host -NoNewline "." }
}

$duration = ((Get-Date) - $start).TotalMilliseconds
$avgLatency = $duration / 100
$throughput = [math]::Round(100000 / $duration, 2)

Write-Host "`n"
Write-Host "Total time: ${duration}ms"
Write-Host "Avg latency: ${avgLatency}ms"
Write-Host "Throughput: ${throughput} req/s"
```

### Using wrk (via WSL)

If you have WSL (Windows Subsystem for Linux):

```powershell
# In PowerShell
wsl

# In WSL
wrk -t4 -c100 -d30s http://localhost:3000/health
```

---

## Troubleshooting

### "OpenSSL not found"

Install OpenSSL for Windows:
- **Via Git for Windows:** OpenSSL is included (`C:\Program Files\Git\usr\bin\openssl.exe`)
- **Via Chocolatey:** `choco install openssl`
- **Standalone:** https://slproweb.com/products/Win32OpenSSL.html

Or use PowerShell certificate generation (see above).

### "Port 3000 already in use"

```powershell
# Find process using port 3000
netstat -ano | findstr :3000

# Kill process (replace PID with actual process ID)
taskkill /PID <PID> /F
```

Or change port in `config.toml`:

```toml
[server]
port = 8080
```

### "Access denied" when writing files

Run PowerShell as Administrator:
1. Right-click PowerShell
2. "Run as administrator"

### Database file locked (SQLite)

If using `sqlite:hsip.db`, ensure no other processes have the file open:

```powershell
# Check if file is locked
Get-Process | Where-Object {$_.Modules.FileName -like "*hsip.db*"}
```

---

## Production Deployment on Windows

For production on Windows Server:

1. ✅ Use PostgreSQL (not SQLite)
2. ✅ Enable TLS with real certificates
3. ✅ Run as Windows Service (NSSM)
4. ✅ Configure Windows Firewall
5. ✅ Set up automated backups
6. ✅ Use IIS or nginx as reverse proxy (optional)

See `DEPLOYMENT.md` for full production deployment guide.

---

## Environment Variables (Alternative to config.toml)

You can override config with environment variables:

```powershell
$env:DATABASE_URL = "postgresql://hsip_user:password@localhost/hsip_db"
$env:PORT = "8080"
$env:RUST_LOG = "debug"

.\target\release\hsip-api.exe
```

---

## Support

- Full deployment guide: `DEPLOYMENT.md`
- API documentation: http://localhost:3000/docs
- Threat model and security scope: `THREAT_MODEL.md`
- Licensing: sanchezleal1989@gmail.com

---

**Quick command reference:**

```powershell
# Generate master key
openssl rand -hex 32 > hsip_master_key.bin

# Build
cargo build --release -p hsip-api --features hsip-api/embed-dashboard

# Run
.\target\release\hsip-api.exe

# Test
Invoke-WebRequest http://localhost:3000/health
```
