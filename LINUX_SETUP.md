# HSIP API - Linux Setup Guide

Quick start guide for running HSIP API on Linux (Ubuntu/Debian, RHEL/CentOS, or any systemd-based distro).

## Prerequisites

- Rust installed via `rustup`:
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source "$HOME/.cargo/env"
  ```
- OpenSSL (for TLS certificate generation):
  ```bash
  # Ubuntu/Debian
  sudo apt update && sudo apt install -y openssl pkg-config libssl-dev

  # RHEL/CentOS/Fedora
  sudo dnf install -y openssl openssl-devel pkgconf
  ```
- `build-essential` or equivalent (for compiling Rust dependencies):
  ```bash
  # Ubuntu/Debian
  sudo apt install -y build-essential

  # RHEL/CentOS/Fedora
  sudo dnf groupinstall -y "Development Tools"
  ```

---

## Quick Start (No TLS - Development Only)

### 1. Generate Master Key

```bash
# Generate 32-byte master encryption key
openssl rand -hex 32 > hsip_master_key.bin
chmod 600 hsip_master_key.bin
```

### 2. Use Development Config

The `config.toml` file is already configured for local development:
- TLS disabled (HTTP only)
- SQLite in-memory database
- Localhost binding only

```bash
cat config.toml
```

### 3. Build and Run

```bash
# Build release binary
cargo build --release --bin hsip-api

# Run server
./target/release/hsip-api
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

```bash
# Health check
curl http://localhost:3000/health

# Get admin key (printed on first run, saved to file)
ADMIN_KEY=$(cat hsip_admin_key.txt)

# Create identity
curl -s -X POST http://localhost:3000/v1/identity \
  -H "Authorization: Bearer $ADMIN_KEY" | jq .

# Issue a credential
curl -s -X POST http://localhost:3000/v1/credentials/issue \
  -H "Authorization: Bearer $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{"claim":"access_calendar","user_token":"user_alice","ttl_seconds":86400}' | jq .
```

---

## Enable TLS/HTTPS (Production)

### Generate Self-Signed Certificate (Development/Testing)

```bash
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout key.pem \
  -out cert.pem \
  -days 365 \
  -subj "/CN=localhost"
```

### Use Let's Encrypt (Production — Recommended)

```bash
# Install certbot
sudo apt install -y certbot       # Ubuntu/Debian
# or
sudo dnf install -y certbot       # RHEL/Fedora

# Obtain certificate (requires a domain name and port 80 open)
sudo certbot certonly --standalone -d yourdomain.com

# Certificates are saved to:
# /etc/letsencrypt/live/yourdomain.com/fullchain.pem
# /etc/letsencrypt/live/yourdomain.com/privkey.pem
```

### Enable TLS in config.toml

```toml
# Uncomment and update this section in config.toml:
[server.tls]
cert_path = "/etc/letsencrypt/live/yourdomain.com/fullchain.pem"
key_path  = "/etc/letsencrypt/live/yourdomain.com/privkey.pem"
require_https = true
```

Then restart the server:

```bash
./target/release/hsip-api
```

---

## Using PostgreSQL (Production)

### 1. Install PostgreSQL

```bash
# Ubuntu/Debian
sudo apt install -y postgresql postgresql-client

# RHEL/CentOS/Fedora
sudo dnf install -y postgresql-server postgresql
sudo postgresql-setup --initdb
sudo systemctl enable --now postgresql
```

### 2. Create Database

```bash
sudo -u postgres psql
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

```bash
./target/release/hsip-api
```

---

## Running as a systemd Service (Production)

### 1. Copy Binary and Config

```bash
# Create app directory
sudo mkdir -p /opt/hsip-api
sudo cp target/release/hsip-api /opt/hsip-api/
sudo cp config.toml /opt/hsip-api/
sudo cp hsip_master_key.bin /opt/hsip-api/
chmod 600 /opt/hsip-api/hsip_master_key.bin
```

### 2. Create a Dedicated User

```bash
sudo useradd --system --no-create-home --shell /bin/false hsip
sudo chown -R hsip:hsip /opt/hsip-api
```

### 3. Create systemd Unit File

```bash
sudo tee /etc/systemd/system/hsip-api.service > /dev/null << 'EOF'
[Unit]
Description=HSIP API - Cryptographic Consent Management
After=network.target postgresql.service
Wants=postgresql.service

[Service]
Type=simple
User=hsip
Group=hsip
WorkingDirectory=/opt/hsip-api
ExecStart=/opt/hsip-api/hsip-api
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=hsip-api

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ReadWritePaths=/opt/hsip-api

[Install]
WantedBy=multi-user.target
EOF
```

### 4. Enable and Start Service

```bash
sudo systemctl daemon-reload
sudo systemctl enable hsip-api
sudo systemctl start hsip-api

# Check status
sudo systemctl status hsip-api

# View logs
sudo journalctl -u hsip-api -f
```

---

## Firewall Configuration

### UFW (Ubuntu/Debian)

```bash
# Allow HTTPS (port 443) from anywhere
sudo ufw allow 443/tcp

# Allow HTTP (port 80) only if needed for Let's Encrypt renewal
sudo ufw allow 80/tcp

# If running on a custom port (e.g., 3000) — restrict to trusted IPs only
sudo ufw allow from <trusted-ip>/32 to any port 3000

sudo ufw enable
sudo ufw status
```

### firewalld (RHEL/CentOS/Fedora)

```bash
sudo firewall-cmd --permanent --add-service=https
sudo firewall-cmd --permanent --add-service=http
sudo firewall-cmd --reload
```

---

## Using nginx as a Reverse Proxy (Optional)

If you want nginx in front of HSIP (for load balancing, caching, or SSL termination):

### Install nginx

```bash
sudo apt install -y nginx       # Ubuntu/Debian
# or
sudo dnf install -y nginx       # RHEL/Fedora
```

### Configure nginx

```bash
sudo tee /etc/nginx/sites-available/hsip-api << 'EOF'
server {
    listen 443 ssl http2;
    server_name yourdomain.com;

    ssl_certificate     /etc/letsencrypt/live/yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/yourdomain.com/privkey.pem;

    location / {
        proxy_pass         http://127.0.0.1:3000;
        proxy_set_header   Host $host;
        proxy_set_header   X-Real-IP $remote_addr;
        proxy_set_header   X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header   X-Forwarded-Proto $scheme;
    }
}

server {
    listen 80;
    server_name yourdomain.com;
    return 301 https://$host$request_uri;
}
EOF

sudo ln -s /etc/nginx/sites-available/hsip-api /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

When using nginx for SSL termination, disable TLS in `config.toml` (let nginx handle it):

```toml
# In config.toml — no [server.tls] section needed when nginx handles TLS
[server]
host = "127.0.0.1"
port = 3000
```

---

## Load Testing on Linux

### Install wrk

```bash
# Ubuntu/Debian
sudo apt install -y wrk

# Build from source (if not in package manager)
git clone https://github.com/wg/wrk.git && cd wrk && make && sudo cp wrk /usr/local/bin/
```

### Run Benchmarks

```bash
# Health endpoint throughput
wrk -t4 -c100 -d30s http://localhost:3000/health

# Run the included benchmark scripts
chmod +x load-test/scripts/*.sh
bash load-test/scripts/simple_benchmark.sh
```

---

## Troubleshooting

### "permission denied" on port 443 or 80

Ports below 1024 require root or special capabilities:

```bash
# Option 1: Grant capability to the binary
sudo setcap cap_net_bind_service=ep /opt/hsip-api/hsip-api

# Option 2: Use a reverse proxy (nginx) on privileged ports, HSIP on port 3000

# Option 3: Run with systemd AmbientCapabilities
# Add to [Service] in the unit file:
# AmbientCapabilities=CAP_NET_BIND_SERVICE
```

### "Address already in use"

```bash
# Find what's using port 3000
sudo ss -tlnp | grep :3000
# or
sudo lsof -i :3000

# Kill the process
sudo kill <PID>
```

Or change the port in `config.toml`:

```toml
[server]
port = 8080
```

### "OpenSSL not found" during build

```bash
# Ubuntu/Debian
sudo apt install -y libssl-dev pkg-config

# RHEL/CentOS/Fedora
sudo dnf install -y openssl-devel pkgconf
```

### Database connection refused (PostgreSQL)

```bash
# Check PostgreSQL is running
sudo systemctl status postgresql

# Check pg_hba.conf allows local connections
sudo -u postgres psql -c "\l"

# Verify connection string
psql "postgresql://hsip_user:password@localhost/hsip_db" -c "SELECT 1;"
```

### View logs

```bash
# If running via systemd
sudo journalctl -u hsip-api -f

# If running directly in terminal
RUST_LOG=debug ./target/release/hsip-api
```

---

## Production Checklist

- [ ] PostgreSQL (not SQLite in-memory)
- [ ] TLS enabled with valid certificates
- [ ] `hsip_master_key.bin` permissions set to `600`
- [ ] `hsip_admin_key.txt` permissions set to `600`, stored securely
- [ ] Running as non-root user via systemd
- [ ] Firewall configured (only ports 80/443 public)
- [ ] Automated database backups configured
- [ ] Log rotation configured (`logrotate` or systemd journal limits)
- [ ] `cargo audit` run to check for known vulnerabilities

See `DEPLOYMENT.md` for the full production deployment guide.

---

## Environment Variables (Alternative to config.toml)

You can override config values with environment variables:

```bash
export DATABASE_URL="postgresql://hsip_user:password@localhost/hsip_db"
export PORT=8080
export RUST_LOG=info

./target/release/hsip-api
```

---

## Quick Command Reference

```bash
# Generate master key
openssl rand -hex 32 > hsip_master_key.bin && chmod 600 hsip_master_key.bin

# Build
cargo build --release --bin hsip-api

# Run
./target/release/hsip-api

# Health check
curl http://localhost:3000/health

# Get admin key
ADMIN_KEY=$(cat hsip_admin_key.txt)

# Create identity
curl -s -X POST http://localhost:3000/v1/identity \
  -H "Authorization: Bearer $ADMIN_KEY" | jq .

# View audit log
curl -s http://localhost:3000/v1/audit \
  -H "Authorization: Bearer $ADMIN_KEY" | jq .
```

---

## Support

- Full deployment guide: `DEPLOYMENT.md`
- API documentation: http://localhost:3000/docs
- Threat model and security scope: `THREAT_MODEL.md`
- Licensing: sanchezleal1989@gmail.com
