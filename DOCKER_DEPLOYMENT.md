# HSIP Phase 2 - Docker Deployment Guide

## Quick Start

### Development Mode

```bash
# Build and start all services
docker-compose -f docker-compose.dev.yml up --build

# Access the daemon
curl http://localhost:8787/status
```

### Production Mode

```bash
# Build production images
docker-compose build

# Start all services
docker-compose up -d

# Check status
docker-compose ps
docker-compose logs -f hsip-daemon
```

---

## Architecture

```
┌──────────────────────────────────────────────────┐
│           Load Balancer (nginx/caddy)            │
└────────────────┬─────────────────────────────────┘
                 │
    ┌────────────┼────────────┐
    │            │            │
┌───▼───┐    ┌──▼────┐   ┌──▼────┐
│ HSIP  │    │ HSIP  │   │ HSIP  │  ← Horizontal scaling
│Daemon │    │Daemon │   │Daemon │
└───┬───┘    └───┬───┘   └───┬───┘
    │            │            │
    └────────────┼────────────┘
                 │
        ┌────────┴────────┐
        │                 │
    ┌───▼────┐      ┌────▼────┐
    │Postgres│      │  Redis  │
    └────────┘      └─────────┘
```

---

## Services

### 1. HSIP Daemon
- **Port**: 8787
- **Purpose**: Main API server
- **Health**: `http://localhost:8787/status`
- **Logs**: `docker-compose logs -f hsip-daemon`

### 2. PostgreSQL
- **Port**: 5432
- **Database**: `hsip`
- **User**: `hsip`
- **Password**: Set via `POSTGRES_PASSWORD` env var

### 3. Redis
- **Port**: 6379
- **Purpose**: Caching and session storage

### 4. Prometheus (Production only)
- **Port**: 9090
- **Purpose**: Metrics collection
- **Dashboard**: `http://localhost:9090`

### 5. Grafana (Production only)
- **Port**: 3000
- **Purpose**: Metrics visualization
- **Dashboard**: `http://localhost:3000`
- **Default login**: admin/admin

---

## Environment Variables

Create a `.env` file:

```bash
# Database
POSTGRES_USER=hsip
POSTGRES_PASSWORD=your_secure_password_here
POSTGRES_DB=hsip

# Redis
REDIS_PASSWORD=your_redis_password_here

# HSIP
RUST_LOG=info,hsip_api=debug
HSIP_DB_URL=postgres://hsip:your_password@postgres:5432/hsip
HSIP_REDIS_URL=redis://:your_redis_password@redis:6379

# Security
JWT_SECRET=your_jwt_secret_here
ENCRYPTION_KEY=your_encryption_key_here

# Optional: External services
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USER=noreply@hsip.io
SMTP_PASSWORD=smtp_password
```

---

## Volume Management

### Data Persistence

Volumes are created automatically:

- `hsip-data`: HSIP identity keys and config
- `postgres-data`: Database files
- `redis-data`: Redis snapshots
- `prometheus-data`: Metrics history
- `grafana-data`: Dashboards and settings

### Backup Volumes

```bash
# Backup HSIP data
docker run --rm -v hsip-data:/data -v $(pwd):/backup \
  alpine tar czf /backup/hsip-backup-$(date +%Y%m%d).tar.gz /data

# Backup PostgreSQL
docker-compose exec postgres pg_dump -U hsip hsip > backup-$(date +%Y%m%d).sql

# Restore PostgreSQL
docker-compose exec -T postgres psql -U hsip hsip < backup.sql
```

---

## Networking

### Network Configuration

Default network: `hsip-network` (172.20.0.0/16)

Services communicate via internal DNS:
- `hsip-daemon:8787`
- `postgres:5432`
- `redis:6379`

### External Access

Expose only necessary ports:
- **8787**: HSIP API (reverse proxy recommended)
- **3000**: Grafana (VPN/firewall recommended)
- **9090**: Prometheus (internal only)

---

## Security Hardening

### 1. Use Secrets

```bash
# Create Docker secrets
echo "your_db_password" | docker secret create postgres_password -
echo "your_redis_password" | docker secret create redis_password -
```

Update `docker-compose.yml`:

```yaml
services:
  postgres:
    environment:
      - POSTGRES_PASSWORD_FILE=/run/secrets/postgres_password
    secrets:
      - postgres_password

secrets:
  postgres_password:
    external: true
```

### 2. Enable TLS

Add nginx reverse proxy:

```yaml
  nginx:
    image: nginx:alpine
    ports:
      - "443:443"
      - "80:80"
    volumes:
      - ./nginx/nginx.conf:/etc/nginx/nginx.conf:ro
      - ./nginx/ssl:/etc/nginx/ssl:ro
    depends_on:
      - hsip-daemon
```

### 3. Network Isolation

```yaml
networks:
  frontend:
    driver: bridge
  backend:
    driver: bridge
    internal: true  # No external access

services:
  hsip-daemon:
    networks:
      - frontend
      - backend
  postgres:
    networks:
      - backend  # Only internal access
```

---

## Monitoring

### Metrics Endpoints

- **HSIP Daemon**: `http://localhost:8787/metrics`
- **Prometheus**: `http://localhost:9090`
- **Grafana**: `http://localhost:3000`

### Key Metrics

- Message verification rate
- Signature validation errors
- API response times
- Database connection pool
- Redis cache hit rate

### Alerts

Configure in `monitoring/prometheus-rules.yml`:

```yaml
groups:
  - name: hsip_alerts
    rules:
      - alert: HighErrorRate
        expr: rate(hsip_errors_total[5m]) > 0.05
        annotations:
          summary: "High error rate detected"

      - alert: DatabaseDown
        expr: up{job="postgres"} == 0
        annotations:
          summary: "PostgreSQL is down"
```

---

## Scaling

### Horizontal Scaling

```bash
# Scale HSIP daemon to 3 instances
docker-compose up -d --scale hsip-daemon=3
```

Add load balancer:

```yaml
  nginx:
    image: nginx:alpine
    ports:
      - "8787:80"
    volumes:
      - ./nginx/upstream.conf:/etc/nginx/conf.d/default.conf
```

`nginx/upstream.conf`:

```nginx
upstream hsip_backend {
    least_conn;
    server hsip-daemon:8787;
}

server {
    listen 80;
    location / {
        proxy_pass http://hsip_backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### Database Replication

```yaml
  postgres-primary:
    image: postgres:16-alpine
    environment:
      - POSTGRES_REPLICATION_MODE=master

  postgres-replica:
    image: postgres:16-alpine
    environment:
      - POSTGRES_REPLICATION_MODE=slave
      - POSTGRES_MASTER_HOST=postgres-primary
```

---

## Troubleshooting

### Check Service Health

```bash
# All services
docker-compose ps

# Individual service logs
docker-compose logs -f hsip-daemon
docker-compose logs -f postgres

# Health check
curl http://localhost:8787/status
```

### Common Issues

**1. Port already in use**

```bash
# Find process using port 8787
lsof -i :8787
# Or
netstat -tulpn | grep 8787

# Change port in docker-compose.yml
ports:
  - "8788:8787"  # Map to different external port
```

**2. Database connection failed**

```bash
# Check PostgreSQL is running
docker-compose exec postgres pg_isready -U hsip

# Check connection string
docker-compose exec hsip-daemon env | grep HSIP_DB_URL

# Reset database
docker-compose down -v
docker-compose up -d
```

**3. Out of memory**

```bash
# Add resource limits
services:
  hsip-daemon:
    deploy:
      resources:
        limits:
          memory: 512M
        reservations:
          memory: 256M
```

### Debug Mode

```bash
# Run with debug logging
RUST_LOG=debug docker-compose up

# Exec into container
docker-compose exec hsip-daemon /bin/bash

# View recent logs
docker-compose logs --tail=100 hsip-daemon
```

---

## Production Checklist

Before deploying to production:

- [ ] Change all default passwords
- [ ] Enable TLS/SSL
- [ ] Configure firewall rules
- [ ] Set up monitoring alerts
- [ ] Configure backup schedules
- [ ] Test disaster recovery
- [ ] Enable rate limiting
- [ ] Configure log rotation
- [ ] Set resource limits
- [ ] Enable health checks
- [ ] Configure auto-restart policies
- [ ] Set up log aggregation
- [ ] Enable audit logging
- [ ] Test horizontal scaling
- [ ] Configure CDN (if needed)

---

## Performance Tuning

### PostgreSQL

```sql
-- In init.sql or via psql
ALTER SYSTEM SET shared_buffers = '256MB';
ALTER SYSTEM SET effective_cache_size = '1GB';
ALTER SYSTEM SET maintenance_work_mem = '64MB';
ALTER SYSTEM SET checkpoint_completion_target = 0.9;
ALTER SYSTEM SET wal_buffers = '16MB';
ALTER SYSTEM SET default_statistics_target = 100;
ALTER SYSTEM SET random_page_cost = 1.1;
ALTER SYSTEM SET effective_io_concurrency = 200;
```

### Redis

```yaml
  redis:
    command: >
      redis-server
      --maxmemory 256mb
      --maxmemory-policy allkeys-lru
      --save 60 1000
```

### HSIP Daemon

```yaml
  hsip-daemon:
    environment:
      - RUST_LOG=info  # Reduce log verbosity
      - TOKIO_WORKER_THREADS=4
```

---

## CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/docker.yml
name: Build Docker Image

on:
  push:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Build image
        run: docker build -t hsip-daemon:latest .

      - name: Push to registry
        run: |
          echo "${{ secrets.DOCKER_PASSWORD }}" | docker login -u "${{ secrets.DOCKER_USERNAME }}" --password-stdin
          docker push hsip-daemon:latest
```

### GitLab CI

```yaml
# .gitlab-ci.yml
stages:
  - build
  - deploy

build:
  stage: build
  script:
    - docker build -t registry.gitlab.com/hsip/daemon:$CI_COMMIT_SHA .
    - docker push registry.gitlab.com/hsip/daemon:$CI_COMMIT_SHA
```

---

## Support

For issues or questions:
- GitHub Issues: https://github.com/nyxsystems/HSIP-2PHASE/issues
- Documentation: https://docs.hsip.io
- Support Email: support@hsip.io
