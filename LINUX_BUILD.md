# HSIP Phase 2 - Linux Build Guide

This guide covers building and running HSIP Phase 2 on Linux systems without GUI dependencies.

## Prerequisites

### Rust Installation
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### System Dependencies

**Debian/Ubuntu:**
```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    ca-certificates
```

**RHEL/CentOS/Fedora:**
```bash
sudo yum install -y \
    gcc \
    openssl-devel \
    pkg-config \
    ca-certificates
```

**Arch Linux:**
```bash
sudo pacman -S base-devel openssl pkg-config
```

## Building

### Standard Build (Daemon Only - No GUI)

Build the HSIP daemon without system tray support (recommended for servers):

```bash
cargo build --release --bin hsip-cli
```

The binary will be located at: `target/release/hsip-cli`

**Build time:** ~15 seconds on modern hardware
**No GTK dependencies required** ✅

### Build with System Tray (Desktop Only)

To build with optional system tray support (requires GTK):

```bash
# Install GTK development libraries first
sudo apt-get install -y libgtk-3-dev  # Debian/Ubuntu
sudo yum install -y gtk3-devel        # RHEL/CentOS
sudo pacman -S gtk3                   # Arch Linux

# Build with tray feature
cargo build --release --bin hsip-tray --features tray
```

## Running

### Start the Daemon

```bash
./target/release/hsip-cli daemon --status_addr 0.0.0.0:8787
```

**Daemon features:**
- REST API on port 8787
- Health check endpoint: `http://localhost:8787/health`
- Status endpoint: `http://localhost:8787/status`

### Verify Installation

```bash
# Check health
curl http://localhost:8787/health

# Check status
curl http://localhost:8787/status

# Send test message
curl -X POST http://localhost:8787/api/v1/send \
  -H "Content-Type: application/json" \
  -d '{
    "recipient": "test@example.com",
    "message_type": "transaction_alert",
    "content": {"amount": "$100.00", "merchant": "Test Store"}
  }'
```

## Docker Deployment

### Quick Start

```bash
# Build Docker image
docker build -t hsip-daemon:latest .

# Run container
docker run -d \
  --name hsip-daemon \
  -p 8787:8787 \
  -v hsip-data:/home/hsip/.hsip \
  hsip-daemon:latest

# Check logs
docker logs -f hsip-daemon

# Health check
docker exec hsip-daemon curl -f http://localhost:8787/health
```

### Production Stack (PostgreSQL + Redis + Monitoring)

```bash
# Start full production stack
docker-compose up -d

# View logs
docker-compose logs -f

# Stop stack
docker-compose down
```

**Included services:**
- HSIP Daemon (port 8787)
- PostgreSQL 16 (port 5432)
- Redis 7 (port 6379)
- Prometheus (port 9090)
- Grafana (port 3000)

See `DOCKER_DEPLOYMENT.md` for detailed production deployment guide.

## Systemd Service (Production)

Create `/etc/systemd/system/hsip-daemon.service`:

```ini
[Unit]
Description=HSIP Phase 2 Banking Anti-Phishing Daemon
After=network.target

[Service]
Type=simple
User=hsip
Group=hsip
WorkingDirectory=/opt/hsip
ExecStart=/usr/local/bin/hsip-cli daemon --status_addr 0.0.0.0:8787
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/hsip

[Install]
WantedBy=multi-user.target
```

**Install and enable:**

```bash
# Create user
sudo useradd -r -s /bin/false hsip

# Create directories
sudo mkdir -p /opt/hsip /var/lib/hsip
sudo chown hsip:hsip /var/lib/hsip

# Install binary
sudo cp target/release/hsip-cli /usr/local/bin/
sudo chmod +x /usr/local/bin/hsip-cli

# Enable service
sudo systemctl daemon-reload
sudo systemctl enable hsip-daemon
sudo systemctl start hsip-daemon

# Check status
sudo systemctl status hsip-daemon
```

## Troubleshooting

### Build Errors

**Error: `tray-icon` requires GTK**
- Solution: Build without tray feature: `cargo build --release --bin hsip-cli`

**Error: `openssl` not found**
- Solution: Install libssl-dev: `sudo apt-get install libssl-dev`

**Error: Linker errors**
- Solution: Install build-essential: `sudo apt-get install build-essential`

### Runtime Errors

**Error: Port 8787 already in use**
```bash
# Find process using port
sudo lsof -i :8787

# Kill process
sudo kill -9 <PID>
```

**Error: Permission denied**
```bash
# Run with elevated privileges or change port
./target/release/hsip-cli daemon --status_addr 0.0.0.0:8888
```

**Error: Cannot connect to PostgreSQL**
```bash
# Start PostgreSQL
sudo systemctl start postgresql

# Check connection
psql -U hsip -d hsip_dev -h localhost
```

## Performance Tuning

### Optimize Build

```bash
# Enable LTO and codegen-units=1 for maximum performance
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

### File Descriptor Limits

For high-load production deployments:

```bash
# Edit /etc/security/limits.conf
hsip soft nofile 65536
hsip hard nofile 65536

# Verify
ulimit -n
```

### Database Connection Pool

Edit configuration file `~/.hsip/config.toml`:

```toml
[database]
max_connections = 100
connection_timeout_seconds = 30
idle_timeout_seconds = 600
```

## Security Hardening

### Firewall Configuration

```bash
# Allow only necessary ports
sudo ufw allow 8787/tcp
sudo ufw enable
```

### TLS/SSL Configuration

```bash
# Generate self-signed certificate (development)
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# Production: Use Let's Encrypt
sudo apt-get install certbot
sudo certbot certonly --standalone -d yourdomain.com
```

### File Permissions

```bash
# Restrict access to config files
chmod 600 ~/.hsip/config.toml
chmod 700 ~/.hsip
```

## Monitoring

### Health Check Endpoint

```bash
curl http://localhost:8787/health
```

**Expected response:**
```json
{
  "status": "healthy",
  "version": "2.0.0",
  "uptime_seconds": 3600
}
```

### Prometheus Metrics

Metrics available at `http://localhost:8787/metrics`:
- `hsip_messages_sent_total`
- `hsip_messages_verified_total`
- `hsip_verification_failures_total`
- `hsip_api_request_duration_seconds`

### Log Monitoring

```bash
# Systemd journal
journalctl -u hsip-daemon -f

# Docker logs
docker logs -f hsip-daemon

# File logs (if configured)
tail -f /var/log/hsip/daemon.log
```

## Development Build

For development with hot-reload:

```bash
# Install cargo-watch
cargo install cargo-watch

# Run with auto-reload
cargo watch -x 'run --bin hsip-cli -- daemon'
```

## Upgrading

```bash
# Pull latest code
git pull origin main

# Rebuild
cargo build --release --bin hsip-cli

# Restart service
sudo systemctl restart hsip-daemon
```

## Support

- **Documentation**: See `README.md` and `DOCKER_DEPLOYMENT.md`
- **Issues**: Report bugs on GitHub
- **Business inquiries**: Contact Nyx Systems LLC

## Feature Flags Reference

| Feature | Description | Default | Dependencies |
|---------|-------------|---------|--------------|
| `default` | Standard daemon build | ✅ | None |
| `tray` | System tray support | ❌ | GTK3 |

**Build examples:**
```bash
# Default (no tray)
cargo build --release --bin hsip-cli

# With tray
cargo build --release --bin hsip-tray --features tray
```
