# HSIP Phase 2 - Load Testing Suite

Performance and scalability testing for the HSIP Enterprise API using k6.

## Overview

This load testing suite validates that the HSIP API can handle production traffic levels:
- **Basic Load Test**: Normal operation (10-100 concurrent users)
- **Stress Test**: Beyond normal limits (100-1000 users)
- **Spike Test**: Sudden traffic spikes (20→1000→20 users)

## Prerequisites

### Install k6

**macOS:**
```bash
brew install k6
```

**Linux (Debian/Ubuntu):**
```bash
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6
```

**Docker:**
```bash
docker pull grafana/k6:latest
```

**From Binary:**
```bash
wget https://github.com/grafana/k6/releases/download/v0.48.0/k6-v0.48.0-linux-amd64.tar.gz
tar -xzf k6-v0.48.0-linux-amd64.tar.gz
sudo mv k6-v0.48.0-linux-amd64/k6 /usr/local/bin/
```

### Start HSIP API

```bash
# Start full stack (API + PostgreSQL + Redis)
docker-compose up -d

# Or start just the API
cargo run --bin hsip-cli -- daemon --status_addr 0.0.0.0:8787
```

## Running Tests

### Basic Load Test (Recommended First)

Tests normal operating conditions with gradual ramp-up:

```bash
k6 run load-tests/basic-load-test.js
```

**Expected results:**
- 95% of requests < 500ms
- Error rate < 1%
- Throughput: ~50-100 requests/second

**Metrics:**
```
✓ health check status is 200
✓ send message status is 200
✓ grant consent status is 200
✓ 95% latency < 500ms
```

### Stress Test (Find Breaking Points)

Pushes system beyond normal limits to identify failure modes:

```bash
k6 run load-tests/stress-test.js
```

**Expected results:**
- 99% of requests < 2s (during stress)
- Error rate < 10% (acceptable under stress)
- Identifies bottlenecks: database connections, memory, CPU

**What to watch:**
- Database connection pool exhaustion
- Memory growth
- CPU saturation
- Queue buildup

### Spike Test (Traffic Bursts)

Simulates sudden traffic spikes (e.g., fraud alert broadcast):

```bash
k6 run load-tests/spike-test.js
```

**Expected results:**
- System recovers from spike without crashing
- 95% of requests < 1s during spike
- No cascade failures

### Custom Configuration

**Override base URL:**
```bash
k6 run -e BASE_URL=https://api.hsip.io load-tests/basic-load-test.js
```

**Output to file:**
```bash
k6 run --out json=results.json load-tests/basic-load-test.js
```

**Run with Docker:**
```bash
docker run --rm -i --network host grafana/k6:latest run - <load-tests/basic-load-test.js
```

## Test Scenarios

### Basic Load Test

| Stage | Duration | Target Users | Purpose |
|-------|----------|--------------|---------|
| Warm-up | 30s | 10 | Establish baseline |
| Normal | 1m | 10 | Steady state |
| Ramp-up | 30s | 50 | Gradual increase |
| Normal | 2m | 50 | Sustained load |
| Peak | 30s | 100 | Peak traffic |
| Peak | 2m | 100 | Sustained peak |
| Cool-down | 30s | 0 | Ramp down |

**Total duration:** 7 minutes

### Stress Test

| Stage | Duration | Target Users | Purpose |
|-------|----------|--------------|---------|
| Baseline | 1m | 100 | Starting point |
| Moderate | 2m | 100 | Sustained |
| Ramp-up | 1m | 200 | Increase |
| Moderate | 2m | 200 | Sustained |
| Stress | 1m | 500 | High load |
| Stress | 3m | 500 | Sustained stress |
| Breaking | 1m | 1000 | Find limits |
| Breaking | 2m | 1000 | Hold at limits |
| Recovery | 2m | 0 | Cool down |

**Total duration:** 15 minutes

### Spike Test

| Stage | Duration | Target Users | Purpose |
|-------|----------|--------------|---------|
| Baseline | 30s | 20 | Normal |
| SPIKE 1 | 10s | 500 | Sudden jump |
| Hold | 1m | 500 | Sustain spike |
| Recovery | 10s | 20 | Back to normal |
| Baseline | 30s | 20 | Stabilize |
| SPIKE 2 | 10s | 1000 | Bigger spike |
| Hold | 1m | 1000 | Sustain |
| Recovery | 10s | 0 | Shutdown |

**Total duration:** 4 minutes

## Performance Thresholds

### Basic Load Test Thresholds

| Metric | Threshold | Acceptable |
|--------|-----------|------------|
| p(95) latency | < 500ms | ✅ |
| p(99) latency | < 1000ms | ✅ |
| Error rate | < 1% | ✅ |
| Success rate | > 99% | ✅ |

### Stress Test Thresholds

| Metric | Threshold | Acceptable |
|--------|-----------|------------|
| p(99) latency | < 2000ms | ✅ (under stress) |
| Error rate | < 10% | ✅ (under stress) |
| No crashes | - | ✅ Required |

### Spike Test Thresholds

| Metric | Threshold | Acceptable |
|--------|-----------|------------|
| p(95) latency | < 1000ms | ✅ (during spike) |
| Error rate | < 15% | ✅ (during spike) |
| Recovery time | < 30s | ✅ After spike |

## Interpreting Results

### Successful Test Output

```
     ✓ health check status is 200
     ✓ send message status is 200
     ✓ grant consent status is 200

     checks.........................: 99.50% ✓ 2985    ✗ 15
     data_received..................: 1.2 MB 20 kB/s
     data_sent......................: 890 kB 15 kB/s
     errors.........................: 0.50%  ✓ 15      ✗ 2985
     http_req_blocked...............: avg=1.2ms    min=0s     med=1ms    max=45ms    p(95)=3ms
     http_req_connecting............: avg=0.8ms    min=0s     med=0.7ms  max=30ms    p(95)=2ms
     http_req_duration..............: avg=245ms    min=10ms   med=200ms  max=1.5s    p(95)=480ms ✅
     http_req_failed................: 0.50%  ✓ 15      ✗ 2985
     http_req_receiving.............: avg=2.1ms    min=0s     med=1.8ms  max=50ms    p(95)=5ms
     http_req_sending...............: avg=0.5ms    min=0s     med=0.4ms  max=10ms    p(95)=1.2ms
     http_req_tls_handshaking.......: avg=0s       min=0s     med=0s     max=0s      p(95)=0s
     http_req_waiting...............: avg=242ms    min=9ms    med=197ms  max=1.48s   p(95)=475ms
     http_reqs......................: 3000   50/s
     iteration_duration.............: avg=5.2s     min=4.1s   med=5s     max=8s      p(95)=6.5s
     iterations.....................: 1000   16.67/s
     vus............................: 100    min=0     max=100
     vus_max........................: 100    min=100   max=100
```

**✅ PASS - All thresholds met**

### Failed Test Output

```
     ✗ health check status is 200
       ↳  85% — ✓ 850 / ✗ 150

     checks.........................: 85.00% ✓ 850     ✗ 150
     http_req_duration..............: avg=1.2s     min=10ms   med=800ms  max=5s      p(95)=3s ❌
     http_req_failed................: 15.00% ✓ 150     ✗ 850

ERRO[0180] thresholds on metrics 'http_req_duration' were crossed
```

**❌ FAIL - Performance degradation detected**

## Monitoring During Tests

### Real-Time Metrics (Terminal)

Watch live stats during test execution:
```bash
k6 run --summary-trend-stats="avg,min,med,max,p(95),p(99)" load-tests/basic-load-test.js
```

### Database Connections

Monitor PostgreSQL connections:
```sql
-- Active connections
SELECT count(*) FROM pg_stat_activity WHERE state = 'active';

-- Connection pool usage
SELECT max_conn, used, res_for_super
FROM (SELECT count(*) used FROM pg_stat_activity) t1,
     (SELECT setting::int res_for_super FROM pg_settings WHERE name='superuser_reserved_connections') t2,
     (SELECT setting::int max_conn FROM pg_settings WHERE name='max_connections') t3;
```

### System Resources

```bash
# CPU usage
top -p $(pgrep hsip-cli)

# Memory usage
ps aux | grep hsip-cli

# Network connections
netstat -an | grep :8787 | wc -l

# Docker stats
docker stats hsip-daemon
```

### Application Logs

```bash
# Follow logs during test
docker logs -f hsip-daemon

# Filter for errors
docker logs hsip-daemon 2>&1 | grep ERROR

# Count requests
docker logs hsip-daemon 2>&1 | grep "Sending message" | wc -l
```

## Grafana Dashboards (Optional)

### Export to InfluxDB

```bash
# Start InfluxDB
docker run -d -p 8086:8086 influxdb:2.0

# Run test with InfluxDB output
k6 run --out influxdb=http://localhost:8086 load-tests/basic-load-test.js
```

### Export to Prometheus

```bash
# Expose Prometheus endpoint
k6 run --out experimental-prometheus-rw load-tests/basic-load-test.js
```

### View in Grafana

1. Import k6 dashboard: https://grafana.com/grafana/dashboards/2587
2. Connect to InfluxDB/Prometheus
3. View real-time metrics

## Common Issues & Solutions

### Issue: Connection Refused

**Error:** `http://localhost:8787: dial tcp: connection refused`

**Solution:**
```bash
# Check if API is running
curl http://localhost:8787/health

# Start API
docker-compose up -d
```

### Issue: High Error Rate

**Error:** `http_req_failed: 15.00%`

**Causes:**
1. Database connection pool exhausted
2. Memory limits exceeded
3. CPU saturation

**Solutions:**
```bash
# Increase database connections
# In crates/hsip-api/src/database.rs:
max_connections: 50  # Was 20

# Increase Docker memory
docker-compose up -d --scale hsip-daemon=2

# Optimize queries (add indexes)
CREATE INDEX idx_audit_timestamp ON audit_log(timestamp);
```

### Issue: Slow Response Times

**Error:** `p(95) latency > 500ms`

**Solutions:**
1. Enable query caching
2. Add database indexes
3. Increase connection pool
4. Scale horizontally (multiple instances)

### Issue: Memory Leak

**Symptom:** Memory usage grows continuously

**Diagnosis:**
```bash
# Monitor memory over time
watch -n 1 'ps aux | grep hsip-cli'

# Check for leaks
valgrind --leak-check=full ./target/release/hsip-cli
```

**Solution:**
- Review database connection cleanup
- Check for unclosed file handles
- Profile with `cargo flamegraph`

## Performance Tuning

### Database Optimization

```sql
-- Increase connection limit
ALTER SYSTEM SET max_connections = '200';

-- Enable connection pooling
ALTER SYSTEM SET shared_buffers = '256MB';
ALTER SYSTEM SET effective_cache_size = '1GB';

-- Restart PostgreSQL
sudo systemctl restart postgresql
```

### API Configuration

```toml
# config.toml
[server]
workers = 8  # Number of worker threads
max_connections = 1000

[database]
max_connections = 50
connection_timeout = 30
idle_timeout = 600
```

### Operating System

```bash
# Increase file descriptor limits
ulimit -n 65536

# Enable TCP fast open
sysctl net.ipv4.tcp_fastopen=3

# Increase backlog queue
sysctl net.core.somaxconn=1024
```

## Benchmarking Goals

### MVP Targets (Phase 1)

| Metric | Target | Status |
|--------|--------|--------|
| 100 concurrent users | p(95) < 500ms | 🎯 Target |
| 1,000 req/sec | Sustained | 🎯 Target |
| 99.9% uptime | Monthly | 🎯 Target |

### Production Targets (Phase 2)

| Metric | Target | Status |
|--------|--------|--------|
| 1,000 concurrent users | p(95) < 500ms | 🎯 Target |
| 10,000 req/sec | Sustained | 🎯 Target |
| 99.99% uptime | Monthly | 🎯 Target |

### Enterprise Targets (Phase 3)

| Metric | Target | Status |
|--------|--------|--------|
| 10,000 concurrent users | p(95) < 500ms | 🎯 Target |
| 100,000 req/sec | Sustained | 🎯 Target |
| 99.999% uptime | Monthly | 🎯 Target |

## Next Steps

After running load tests:

1. **Analyze Results**: Identify bottlenecks (database, CPU, memory)
2. **Optimize**: Apply performance improvements
3. **Re-test**: Verify improvements with load tests
4. **Document**: Record baseline performance metrics
5. **Monitor**: Set up production monitoring (Prometheus/Grafana)
6. **Scale**: Plan horizontal scaling strategy

## References

- k6 Documentation: https://k6.io/docs/
- Performance Testing Best Practices: https://k6.io/docs/testing-guides/
- Grafana Dashboards: https://grafana.com/grafana/dashboards/
- PostgreSQL Performance Tuning: https://wiki.postgresql.org/wiki/Performance_Optimization
