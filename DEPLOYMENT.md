# HSIP API Production Deployment Guide

This guide covers production deployment, high availability architecture, backup strategies, and disaster recovery procedures for the HSIP API.

---

## Table of Contents

1. [Desktop Mode — Zero Config](#desktop-mode--zero-config)
2. [Deploy to Railway — Hosted Demo](#deploy-to-railway--hosted-demo)
3. [Production Configuration](#production-configuration)
4. [TLS/HTTPS Setup](#tlshttps-setup)
5. [Database Configuration](#database-configuration)
6. [High Availability Architecture](#high-availability-architecture)
7. [Backup and Disaster Recovery](#backup-and-disaster-recovery)
8. [Monitoring and Alerting](#monitoring-and-alerting)
9. [Security Hardening](#security-hardening)
10. [Performance Tuning](#performance-tuning)
11. [Troubleshooting](#troubleshooting)

---

## Desktop Mode — Zero Config

The simplest way to run HSIP is desktop mode. No `config.toml` needed.

```bash
# Build
cargo build --release -p hsip-api

# Run — starts on port 7474, creates ~/.hsip/ automatically
./target/release/hsip-api
```

On first boot the server prints your admin key:

```
Admin API key (save this): hsip_3bef3de24adf00194efb...
Key written to: ~/.hsip/admin.key
```

Open `http://127.0.0.1:7474` in your browser to access the dashboard.

Desktop mode stores everything in `~/.hsip/` (Linux/macOS) or `%APPDATA%\HSIP\` (Windows).  
SQLite is used by default. For production, set `DATABASE_URL` to a PostgreSQL URL.

---

## Deploy to Railway — Hosted Demo

Railway is the fastest path to a public HSIP instance. The repo ships a `Dockerfile` and `railway.toml` that are ready to use.

### One-Time Setup

1. **Push the repo** to GitHub (already done if you're reading this).
2. **Create a Railway project** → New Project → Deploy from GitHub repo.
3. **Set environment variables** in Railway → Variables:

| Variable | Required | Example value | Notes |
|---|---|---|---|
| `PORT` | Yes | `7474` | Railway injects this automatically |
| `HSIP_ADMIN_KEY` | **Yes** | `hsip_3bef3de24adf00194efb4dd4f625e9da828738c5f162d709a6adedafb084d12d` | Fixed admin key — survives restarts. Generate: `openssl rand -hex 32 \| sed 's/^/hsip_/'` |
| `HSIP_PUBLIC_URL` | Yes | `https://hsip-1phase-production.up.railway.app` | Used for CORS and self-links |
| `HSIP_SANDBOX` | Optional | `true` | Enables `POST /v1/sandbox/provision` — auto-creates 24-hour trial keys for visitors |
| `CORS_ALLOW_ALL` | Optional | `true` | Opens CORS for all origins — fine for public demos, remove for production |
| `DATABASE_URL` | Optional | `postgresql://...` | Defaults to SQLite at `/tmp/hsip.db`. Use a Railway Postgres addon for persistence. |

> **Warning:** Railway containers use ephemeral storage. Without a `DATABASE_URL` pointing to a persistent database, all data (tenants, keys, audit log) resets on every redeploy. For a persistent demo, add a Railway PostgreSQL addon and set `DATABASE_URL`.

### Generate a Persistent Admin Key

```bash
openssl rand -hex 32 | sed 's/^/hsip_/'
# Output: hsip_3bef3de24adf00194efb4dd4f625e9da828738c5f162d709a6adedafb084d12d
```

Copy that value into `HSIP_ADMIN_KEY` in Railway Variables. Save it somewhere safe — this is your permanent admin credential for this deployment.

### Verify the Deployment

```bash
# Health check
curl https://your-app.up.railway.app/health
# → {"status":"ok","version":"0.2.0"}

# Sign in with your admin key
curl -X POST https://your-app.up.railway.app/v1/identity \
  -H "Authorization: Bearer hsip_<your-admin-key>"
```

---

## Quick Start (Self-Hosted Server Mode)

### 1. Generate Configuration Files

```bash
# Copy example config
cp crates/hsip-api/config.toml.example config.toml

# Generate master encryption key (32 bytes = 64 hex chars)
openssl rand -hex 32 > hsip_master_key.bin
chmod 600 hsip_master_key.bin

# Generate TLS certificates (self-signed for testing)
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout key.pem -out cert.pem -days 365 \
  -subj "/CN=yourdomain.com"
chmod 600 key.pem
```

### 2. Edit Configuration

Edit `config.toml` with your production settings:

```toml
[server]
host = "0.0.0.0"
port = 3000

[server.tls]
cert_path = "cert.pem"
key_path = "key.pem"
require_https = true

[database]
url = "postgresql://hsip_user:password@localhost/hsip_db"
max_connections = 10
run_migrations = true

[security]
master_key_path = "hsip_master_key.bin"
admin_key_path = "hsip_admin_key.txt"
rate_limit_per_minute = 60

[cors]
allowed_origins = ["https://yourdomain.com"]

[logging]
level = "info"
format = "json"
```

### 3. Build and Run

```bash
# Build release binary
cargo build --release --bin hsip-api

# Run server
./target/release/hsip-api
```

---

## Production Configuration

### Environment Variables

The following environment variables override config file settings:

| Variable | Description | Example |
|----------|-------------|---------|
| `HSIP_CONFIG` | Path to config file | `config.toml` |
| `DATABASE_URL` | Database connection string | `postgresql://...` |
| `HSIP_ADMIN_KEY` | Fixed admin key that survives restarts (recommended for cloud deploys). Must start with `hsip_` and be ≥ 20 chars. | `hsip_3bef3de24a...` |
| `HSIP_PUBLIC_URL` | Public base URL of this instance — used in CORS and self-links | `https://example.com` |
| `HSIP_SANDBOX` | Set to `true` to enable `POST /v1/sandbox/provision` (24-hour trial key provisioning) | `true` |
| `HSIP_MASTER_KEY` | Master encryption key as a hex string — alternative to key file | `a3f1...` (64 hex chars) |
| `CORS_ALLOW_ALL` | Set to `true` to allow requests from any origin — use only for public demos | `true` |
| `RATE_LIMIT_RPM` | Per-key rate limit in requests/minute (default: 300) | `300` |
| `PORT` | Server port (default: 7474 desktop / 3000 server mode) | `7474` |
| `RUST_LOG` | Log level override | `info,hsip_api=debug` |
| `METRICS_TOKEN` | Bearer token for `/metrics` | `secret-token` |

### Configuration Validation

The server performs comprehensive validation on startup:

- ✅ Database URL format (SQLite/PostgreSQL)
- ✅ Master key file exists and is readable
- ✅ Admin key file path is valid
- ✅ TLS certificate and private key exist (if TLS enabled)
- ✅ Port number is valid (1-65535)
- ✅ Log level is valid (trace/debug/info/warn/error)

**The server will exit immediately with a clear error message if validation fails.**

---

## TLS/HTTPS Setup

### Production Certificates (Let's Encrypt)

For production deployments, use Let's Encrypt certificates:

```bash
# Install certbot
sudo apt-get install certbot

# Generate certificate (standalone mode)
sudo certbot certonly --standalone -d yourdomain.com

# Certificates will be in:
# /etc/letsencrypt/live/yourdomain.com/fullchain.pem
# /etc/letsencrypt/live/yourdomain.com/privkey.pem

# Update config.toml
[server.tls]
cert_path = "/etc/letsencrypt/live/yourdomain.com/fullchain.pem"
key_path = "/etc/letsencrypt/live/yourdomain.com/privkey.pem"
require_https = true
```

### Certificate Auto-Renewal

```bash
# Test renewal
sudo certbot renew --dry-run

# Add cron job for auto-renewal
sudo crontab -e

# Renew daily at 3am, restart server if renewed
0 3 * * * certbot renew --quiet --post-hook "systemctl restart hsip-api"
```

### Reverse Proxy (Alternative)

If you prefer nginx/Caddy for TLS termination:

**Caddy (automatic HTTPS):**

```caddy
yourdomain.com {
    reverse_proxy localhost:3000
}
```

**nginx:**

```nginx
server {
    listen 443 ssl http2;
    server_name yourdomain.com;

    ssl_certificate /etc/letsencrypt/live/yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/yourdomain.com/privkey.pem;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

If using a reverse proxy, **disable TLS in HSIP config** (remove `[server.tls]` section).

---

## Database Configuration

### PostgreSQL Setup (Recommended)

PostgreSQL is **strongly recommended** for production due to:
- ✅ Better concurrency handling
- ✅ ACID compliance
- ✅ Point-in-time recovery
- ✅ Replication support

**Installation:**

```bash
# Install PostgreSQL
sudo apt-get install postgresql postgresql-contrib

# Create database and user
sudo -u postgres psql
```

```sql
CREATE DATABASE hsip_db;
CREATE USER hsip_user WITH ENCRYPTED PASSWORD 'your_secure_password';
GRANT ALL PRIVILEGES ON DATABASE hsip_db TO hsip_user;
\q
```

**Connection String:**

```toml
[database]
url = "postgresql://hsip_user:your_secure_password@localhost/hsip_db"
max_connections = 10
run_migrations = true
```

### Connection Pool Tuning

Adjust `max_connections` based on your workload:

- **Low traffic (< 100 req/s):** `max_connections = 5`
- **Medium traffic (100-1000 req/s):** `max_connections = 10`
- **High traffic (> 1000 req/s):** `max_connections = 20`

**Formula:** `max_connections = (CPU cores * 2) + effective_spindle_count`

For cloud databases (AWS RDS, etc.), monitor connection pool exhaustion and adjust accordingly.

### SQLite (Development Only)

⚠️ **Do NOT use SQLite for production** — it does not support:
- High concurrency
- Remote connections
- Replication
- Point-in-time recovery

SQLite is acceptable for:
- Local development
- Testing
- Single-user demos

---

## High Availability Architecture

### Single Instance (Minimum)

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  HSIP API   │
│  + TLS      │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ PostgreSQL  │
└─────────────┘
```

**Pros:** Simple, low cost
**Cons:** Single point of failure, no redundancy

### Multi-Instance with Load Balancer (Recommended)

```
                      ┌─────────────┐
                      │   Client    │
                      └──────┬──────┘
                             │
                             ▼
                      ┌─────────────┐
                      │Load Balancer│  (nginx/HAProxy/AWS ALB)
                      └──────┬──────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
              ▼              ▼              ▼
       ┌─────────────┐┌─────────────┐┌─────────────┐
       │  HSIP API   ││  HSIP API   ││  HSIP API   │
       │  Instance 1 ││  Instance 2 ││  Instance 3 │
       └──────┬──────┘└──────┬──────┘└──────┬──────┘
              │              │              │
              └──────────────┼──────────────┘
                             │
                             ▼
                      ┌─────────────┐
                      │ PostgreSQL  │
                      │  Primary    │
                      └──────┬──────┘
                             │
                      ┌──────┴──────┐
                      │             │
                      ▼             ▼
               ┌─────────────┐┌─────────────┐
               │ PostgreSQL  ││ PostgreSQL  │
               │  Replica 1  ││  Replica 2  │
               └─────────────┘└─────────────┘
```

**Pros:** High availability, horizontal scaling, zero-downtime deployments
**Cons:** More complex, higher cost

### Configuration for Multi-Instance

**Key Requirements:**

1. **Shared Database:** All HSIP instances connect to the same PostgreSQL cluster
2. **Shared Master Key:** All instances must use the **identical** `hsip_master_key.bin` file
3. **Stateless Design:** HSIP API is stateless (in-memory rate limiter is eventually consistent)

**Deployment Steps:**

```bash
# On each instance, copy the SAME master key
scp hsip_master_key.bin instance1:/opt/hsip/
scp hsip_master_key.bin instance2:/opt/hsip/
scp hsip_master_key.bin instance3:/opt/hsip/

# Update config.toml on each instance to point to shared database
[database]
url = "postgresql://hsip_user:password@db.internal/hsip_db"
```

### Health Checks

Configure your load balancer to check `/health`:

```bash
# Health check endpoint
GET https://hsip-api.yourdomain.com/health

# Expected response (200 OK):
{"status":"ok","version":"0.2.0"}
```

**HAProxy Example:**

```
backend hsip_api
    balance roundrobin
    option httpchk GET /health
    http-check expect status 200
    server hsip1 10.0.1.10:3000 check
    server hsip2 10.0.1.11:3000 check
    server hsip3 10.0.1.12:3000 check
```

---

## Backup and Disaster Recovery

### Database Backups

**PostgreSQL Automated Backups:**

```bash
#!/bin/bash
# /opt/hsip/backup.sh

BACKUP_DIR="/opt/hsip/backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
DATABASE="hsip_db"

# Create backup directory
mkdir -p $BACKUP_DIR

# Dump database
pg_dump -U hsip_user -Fc $DATABASE > "$BACKUP_DIR/hsip_db_$TIMESTAMP.dump"

# Delete backups older than 30 days
find $BACKUP_DIR -name "*.dump" -mtime +30 -delete

# Upload to S3 (optional)
aws s3 cp "$BACKUP_DIR/hsip_db_$TIMESTAMP.dump" s3://your-backup-bucket/hsip/
```

**Cron Job (Daily at 2 AM):**

```bash
0 2 * * * /opt/hsip/backup.sh >> /var/log/hsip-backup.log 2>&1
```

### Point-in-Time Recovery (PITR)

Enable WAL archiving in PostgreSQL:

```bash
# postgresql.conf
wal_level = replica
archive_mode = on
archive_command = 'test ! -f /mnt/wal_archive/%f && cp %p /mnt/wal_archive/%f'
```

### Critical Files to Backup

| File | Purpose | Backup Frequency | Storage |
|------|---------|------------------|---------|
| `hsip_master_key.bin` | Encryption key | **ONCE** (never changes) | **Encrypted off-site** |
| `hsip_admin_key.txt` | Admin API key | **ONCE** (never changes) | Secure vault |
| `config.toml` | Server config | On change | Version control |
| PostgreSQL database | All data | **Daily** | Encrypted S3/GCS |
| PostgreSQL WAL files | PITR | **Continuous** | Encrypted S3/GCS |

### Disaster Recovery Procedure

**Scenario: Complete server failure**

1. **Provision new server**
2. **Install HSIP API binary**
3. **Restore critical files:**

```bash
# Restore master key (from encrypted backup)
aws s3 cp s3://your-backup-bucket/hsip_master_key.bin /opt/hsip/
chmod 600 /opt/hsip/hsip_master_key.bin

# Restore config
cp /opt/hsip/config.toml.backup /opt/hsip/config.toml
```

4. **Restore database:**

```bash
# Create new database
createdb -U postgres hsip_db

# Restore from dump
pg_restore -U hsip_user -d hsip_db /path/to/hsip_db_20260223_020000.dump
```

5. **Start server and verify:**

```bash
./hsip-api

# Test health endpoint
curl https://yourdomain.com/health
```

**RTO (Recovery Time Objective):** < 1 hour
**RPO (Recovery Point Objective):** < 24 hours (with daily backups)

---

## Monitoring and Alerting

### Metrics Endpoint

HSIP exposes Prometheus-compatible metrics at `/metrics`:

```bash
# Protect with bearer token
export METRICS_TOKEN="your-secret-token"

# Query metrics
curl -H "Authorization: Bearer your-secret-token" \
  https://yourdomain.com/metrics
```

**Key Metrics** (see `crates/hsip-api/src/metrics.rs` for the authoritative list — this section lists the ones that matter most operationally, not every metric):

- `hsip_requests_total` — Total requests by endpoint
- `hsip_active_tenants` — Number of active tenants
- `hsip_system_health_issues{severity}` — **The one to alert on first.** A gauge (not a counter — drops back to zero once resolved) covering conditions HSIP cannot recover from automatically: an incomplete master-key rotation, zero remaining root-admin keys on the node, or OTS anchor batches that gave up retrying. See "System Health" in `CLAUDE.md`.
- `hsip_audit_write_failures_total{action}` — An audit-trail write failed after its underlying operation already succeeded (the operation itself isn't blocked on this, but a missing audit entry is otherwise invisible).
- `hsip_auth_failures_total{reason}` — Rejected requests by reason (includes rate-limit and replay-protection rejections).
- `hsip_credentials_issued_total`, `hsip_decisions_recorded_total`, `hsip_messages_signed_total` — throughput counters, unlabeled by design (see `CLAUDE.md`'s "Structured QA Pass" — these used to carry unbounded caller-controlled label values, a real cardinality/info-disclosure bug, fixed by dropping the labels).

There is currently no request-latency histogram exposed — the "Alerting Rules" example below reflects that; don't copy an alert for a metric that doesn't exist.

### Prometheus Configuration

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'hsip-api'
    scrape_interval: 15s
    bearer_token: 'your-secret-token'
    static_configs:
      - targets: ['hsip-api-1:3000', 'hsip-api-2:3000', 'hsip-api-3:3000']
```

### Grafana Dashboard

Import the HSIP Grafana dashboard (create custom dashboard with):

- **Request rate** (requests/sec)
- **Error rate** (5xx responses)
- **Latency** (p50, p95, p99)
- **Active tenants**
- **Rate limit violations**
- **Database connection pool usage**

### Alerting Rules

**Prometheus Alert Rules:**

```yaml
groups:
  - name: hsip_alerts
    rules:
      - alert: HSIPSystemHealthCritical
        expr: hsip_system_health_issues{severity="critical"} > 0
        for: 5m
        annotations:
          summary: "HSIP reports a critical system-health issue (incomplete key rotation, zero root admins, or similar) — see GET /v1/admin/system-health for detail"

      - alert: HighErrorRate
        expr: rate(hsip_requests_total{status=~"5.."}[5m]) > 0.05
        for: 5m
        annotations:
          summary: "HSIP API error rate > 5%"

      - alert: AuditWriteFailures
        expr: increase(hsip_audit_write_failures_total[15m]) > 0
        annotations:
          summary: "An HSIP operation succeeded but its audit-trail entry failed to write — investigate before it recurs"

      - alert: ServiceDown
        expr: up{job="hsip-api"} == 0
        for: 1m
        annotations:
          summary: "HSIP API instance is down"
```

### Structured Logging

HSIP uses JSON-formatted structured logs in production:

```toml
[logging]
level = "info"
format = "json"
```

**Integrate with log aggregation:**

- **CloudWatch Logs** (AWS)
- **Stackdriver** (GCP)
- **ELK Stack** (Elasticsearch/Logstash/Kibana)
- **Loki** (Grafana)

**Example log entry:**

```json
{
  "timestamp": "2026-02-23T20:30:45.123Z",
  "level": "INFO",
  "target": "hsip_api",
  "message": "Request completed",
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "method": "POST",
  "path": "/v1/credentials/issue",
  "status": 200,
  "duration_ms": 45
}
```

---

## Security Hardening

### Operating System

```bash
# Run as non-root user
sudo useradd -r -s /bin/false hsip
sudo chown -R hsip:hsip /opt/hsip

# Restrict file permissions
chmod 600 /opt/hsip/hsip_master_key.bin
chmod 600 /opt/hsip/hsip_admin_key.txt
chmod 640 /opt/hsip/config.toml
```

### Systemd Service

```ini
# /etc/systemd/system/hsip-api.service
[Unit]
Description=HSIP API Server
After=network.target postgresql.service

[Service]
Type=simple
User=hsip
Group=hsip
WorkingDirectory=/opt/hsip
ExecStart=/opt/hsip/hsip-api
Restart=on-failure
RestartSec=10s

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/hsip

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable hsip-api
sudo systemctl start hsip-api
sudo systemctl status hsip-api
```

### Firewall Configuration

```bash
# Allow HTTPS only
sudo ufw allow 443/tcp
sudo ufw enable

# If using reverse proxy, restrict API to localhost
sudo ufw allow from 127.0.0.1 to any port 3000
```

### Rate Limiting

HSIP includes built-in rate limiting (default: 300 requests/minute per API key, configurable via `RATE_LIMIT_RPM` env var). AI agent keys have an additional velocity layer: anomaly logged at >100 req/min, key auto-revoked at >1000 req/min.

Adjust via environment variable (takes effect without restarting):

```bash
export RATE_LIMIT_RPM=100   # Lower for stricter public deployments
```

For additional DDoS protection, use a reverse proxy or cloud WAF.

---

## Performance Tuning

### Database Optimization

**PostgreSQL tuning (`postgresql.conf`):**

```ini
# Connections
max_connections = 100

# Memory
shared_buffers = 256MB
effective_cache_size = 1GB
work_mem = 4MB
maintenance_work_mem = 64MB

# WAL
wal_buffers = 16MB
checkpoint_completion_target = 0.9
```

**Analyze query performance:**

```sql
-- Enable slow query logging
ALTER SYSTEM SET log_min_duration_statement = 1000;  -- Log queries > 1s
SELECT pg_reload_conf();

-- Check slow queries
SELECT query, mean_exec_time, calls
FROM pg_stat_statements
ORDER BY mean_exec_time DESC
LIMIT 10;
```

### Horizontal Scaling

Add more HSIP API instances behind a load balancer to handle increased load.

**Scaling strategy:**

- **< 1,000 req/s:** 2 instances
- **1,000-10,000 req/s:** 3-5 instances
- **> 10,000 req/s:** 5+ instances + database read replicas

### Caching

HSIP does not include built-in caching. For high-read workloads, consider:

- **Redis** for session/credential caching
- **PostgreSQL read replicas** for consent lookups
- **CDN** for static assets (Swagger UI)

---

## Troubleshooting

### Server Won't Start

**Issue:** Configuration validation error

```
❌ Configuration validation failed: Master key file not found: hsip_master_key.bin
```

**Solution:**

```bash
openssl rand -hex 32 > hsip_master_key.bin
chmod 600 hsip_master_key.bin
```

---

**Issue:** TLS certificate error

```
❌ Failed to open certificate: cert.pem
```

**Solution:**

```bash
# Generate self-signed cert
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout key.pem -out cert.pem -days 365 \
  -subj "/CN=localhost"
```

---

### Database Connection Errors

**Issue:** `connection refused`

```
Error: error connecting to database: Connection refused (os error 111)
```

**Solution:**

```bash
# Check PostgreSQL is running
sudo systemctl status postgresql

# Check connection string in config.toml
[database]
url = "postgresql://hsip_user:password@localhost/hsip_db"
```

---

**Issue:** `too many connections`

```
Error: FATAL: sorry, too many clients already
```

**Solution:**

```sql
-- Increase max_connections in PostgreSQL
ALTER SYSTEM SET max_connections = 200;
SELECT pg_reload_conf();
```

Or reduce `max_connections` in HSIP config:

```toml
[database]
max_connections = 5
```

---

### High Latency

**Issue:** Slow API responses

**Diagnosis:**

```bash
# Check database query performance
sudo -u postgres psql hsip_db
```

```sql
SELECT * FROM pg_stat_activity WHERE state = 'active';
```

**Solution:**

1. Add database indexes (already included in migrations)
2. Increase `max_connections` in config
3. Add read replicas for PostgreSQL
4. Scale horizontally (add more HSIP instances)

---

### Memory Leaks

**Issue:** Server memory usage grows over time

**Diagnosis:**

```bash
# Monitor memory usage
watch -n 5 'ps aux | grep hsip-api'
```

**Solution:**

1. Update to latest version
2. Report issue to GitHub with logs
3. Restart service (temporary):

```bash
sudo systemctl restart hsip-api
```

---

## Production Checklist

Before going live, verify:

- [ ] TLS/HTTPS enabled with valid certificate (or reverse proxy handles it)
- [ ] PostgreSQL database configured (not SQLite) — or Railway Postgres addon
- [ ] `HSIP_ADMIN_KEY` set to a fixed value and backed up securely
- [ ] Master key backed up to encrypted off-site storage
- [ ] Config file reviewed and secured (chmod 640)
- [ ] Firewall rules configured
- [ ] Systemd service installed and enabled (self-hosted) or Railway deploy active
- [ ] Automated backups configured (daily)
- [ ] Monitoring/alerting configured (Prometheus + Grafana)
- [ ] Load balancer health checks configured (multi-instance only)
- [ ] Disaster recovery procedure documented and tested
- [ ] `cargo audit` running in CI (see `.github/workflows/security-audit.yml`)
- [ ] Load testing performed with expected traffic patterns
- [ ] At least one active root-admin key confirmed via `GET /v1/admin/root-admins` or `hsip keys list-root-admins` — there is no recovery path from zero root admins except editing the database directly

---

## Support

For production deployment support:

- **Documentation:** See `README.md`, `THREAT_MODEL.md`, `docs/SANDBOX_QUICKSTART.md`
- **Issues:** https://github.com/rewired89/HSIP-1PHASE/issues
- **Security vulnerabilities:** sanchezleal1989@gmail.com with subject `[HSIP SECURITY]`

---

**End of Deployment Guide**
