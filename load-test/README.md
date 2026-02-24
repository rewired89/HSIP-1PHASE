# HSIP API Load Testing

This directory contains load testing scripts to verify HSIP API performance under stress.

## Tools

### Option 1: wrk (Recommended)

**Installation:**

```bash
# Ubuntu/Debian
sudo apt-get install wrk

# macOS
brew install wrk

# From source
git clone https://github.com/wg/wrk.git
cd wrk && make
```

**Run tests:**

```bash
# Test health endpoint (warmup)
wrk -t4 -c100 -d30s http://localhost:3000/health

# Test identity creation (authenticated)
wrk -t4 -c100 -d60s \
  -H "Authorization: Bearer YOUR_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -s scripts/create_identity.lua \
  http://localhost:3000/v1/identity
```

### Option 2: hey

**Installation:**

```bash
go install github.com/rakyll/hey@latest
```

**Run tests:**

```bash
# Test health endpoint
hey -n 10000 -c 100 http://localhost:3000/health

# Test identity creation
hey -n 1000 -c 50 \
  -H "Authorization: Bearer YOUR_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -m POST \
  http://localhost:3000/v1/identity
```

### Option 3: Apache Bench (ab)

```bash
# Test health endpoint
ab -n 10000 -c 100 http://localhost:3000/health
```

## Test Scenarios

### 1. Health Check Load Test

**Purpose:** Verify server can handle high request volume

```bash
./scripts/test_health.sh
```

**Expected Results:**
- **Latency (p95):** < 50ms
- **Throughput:** > 1,000 req/s
- **Error Rate:** 0%

### 2. Identity Creation Load Test

**Purpose:** Test database write performance

```bash
./scripts/test_identity_creation.sh YOUR_ADMIN_KEY
```

**Expected Results:**
- **Latency (p95):** < 200ms
- **Throughput:** > 100 req/s
- **Error Rate:** < 1%

### 3. Credential Issuance Load Test

**Purpose:** Test cryptographic signing performance

```bash
./scripts/test_credential_issuance.sh YOUR_ADMIN_KEY
```

**Expected Results:**
- **Latency (p95):** < 300ms
- **Throughput:** > 50 req/s
- **Error Rate:** < 1%

### 4. Mixed Workload Test

**Purpose:** Simulate realistic production traffic

```bash
./scripts/test_mixed_workload.sh YOUR_ADMIN_KEY
```

**Mix:**
- 60% reads (consent lookups)
- 30% writes (credential issuance)
- 10% admin operations (key rotation)

## Performance Baselines

Tested on: **4-core CPU, 8GB RAM, PostgreSQL on same host**

| Endpoint | Throughput | p50 Latency | p95 Latency | p99 Latency |
|----------|------------|-------------|-------------|-------------|
| `/health` | 5,000 req/s | 5ms | 15ms | 30ms |
| `POST /v1/identity` | 500 req/s | 50ms | 150ms | 300ms |
| `POST /v1/credentials/issue` | 200 req/s | 100ms | 250ms | 500ms |
| `GET /v1/consent/:id` | 2,000 req/s | 10ms | 50ms | 100ms |

## Scaling Recommendations

| Expected Traffic | Configuration |
|------------------|---------------|
| < 100 req/s | 1 instance, SQLite or PostgreSQL |
| 100-1,000 req/s | 2 instances + PostgreSQL |
| 1,000-10,000 req/s | 3-5 instances + PostgreSQL + read replicas |
| > 10,000 req/s | 5+ instances + PostgreSQL cluster + Redis caching |

## Monitoring During Tests

**Terminal 1: Run load test**
```bash
wrk -t4 -c100 -d60s http://localhost:3000/health
```

**Terminal 2: Monitor metrics**
```bash
watch -n 1 'curl -s http://localhost:3000/metrics | grep hsip_requests_total'
```

**Terminal 3: Monitor database**
```bash
watch -n 1 'psql -U hsip_user -d hsip_db -c "SELECT count(*) FROM pg_stat_activity WHERE state = '\''active'\'';"'
```

## Troubleshooting

### High Error Rate

**Symptoms:** > 5% 5xx responses

**Possible Causes:**
1. Database connection pool exhausted
   - **Fix:** Increase `max_connections` in config.toml
2. Rate limiting triggered
   - **Fix:** Increase `rate_limit_per_minute` in config.toml
3. CPU/memory exhaustion
   - **Fix:** Scale horizontally (add more instances)

### High Latency

**Symptoms:** p95 latency > 1s

**Possible Causes:**
1. Database query performance
   - **Fix:** Analyze slow queries with `pg_stat_statements`
2. Too many concurrent connections
   - **Fix:** Tune PostgreSQL `max_connections` and `work_mem`
3. Insufficient resources
   - **Fix:** Increase CPU/RAM or scale horizontally

### Connection Timeouts

**Symptoms:** Connection refused errors

**Possible Causes:**
1. Server overwhelmed
   - **Fix:** Reduce concurrency level in test
2. File descriptor limit
   - **Fix:** Increase with `ulimit -n 65536`

## Best Practices

1. **Always warmup before testing:** Run 30s warmup to establish connection pools
2. **Test incrementally:** Start with low concurrency (10) and increase gradually
3. **Monitor resource usage:** Watch CPU, memory, disk I/O during tests
4. **Test on production-like hardware:** Cloud instances with similar specs
5. **Include authentication:** Real-world requests include auth headers
6. **Vary request payloads:** Don't send identical data every time
7. **Run tests for adequate duration:** Minimum 60s, ideally 5+ minutes
8. **Compare results over time:** Track performance regressions between versions

## Continuous Load Testing

Add to CI/CD pipeline:

```yaml
# .github/workflows/load-test.yml
name: Load Test
on: [push]
jobs:
  load-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Start server
        run: |
          cargo build --release
          ./target/release/hsip-api &
          sleep 5
      - name: Run load test
        run: |
          wrk -t2 -c50 -d30s http://localhost:3000/health
      - name: Check latency threshold
        run: |
          # Fail if p95 > 100ms
          ./scripts/check_latency_threshold.sh 100
```
