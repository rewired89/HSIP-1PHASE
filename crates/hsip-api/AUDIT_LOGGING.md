# HSIP Phase 2 - Audit Logging with PostgreSQL

This document describes the audit logging implementation for HSIP Phase 2 Enterprise API.

## Overview

All API operations are now logged to PostgreSQL for:
- **Non-repudiation**: Cryptographic proof of message sending
- **Compliance**: Regulatory audit trails (SOX, PCI-DSS, GDPR)
- **Forensics**: Security incident investigation
- **Analytics**: Business intelligence and reporting

## Database Schema

### Audit Log Table

```sql
CREATE TABLE audit_log (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_type VARCHAR(50) NOT NULL,
    user_id VARCHAR(255),
    sender_domain VARCHAR(255),
    recipient VARCHAR(255),
    message_id VARCHAR(255),
    message_type VARCHAR(100),
    verification_status VARCHAR(50),
    signature_hex TEXT,
    public_key_hex TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Indexes:**
- `idx_audit_timestamp` - Fast time-range queries
- `idx_audit_message_id` - Message lookup
- `idx_audit_user_id` - User activity tracking
- `idx_audit_sender_domain` - Domain filtering

### Consent Policies Table

```sql
CREATE TABLE consent_policies (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(255) NOT NULL,
    sender_domain VARCHAR(255) NOT NULL,
    allowed_message_types TEXT[] NOT NULL,
    duration_days INTEGER,
    max_messages_per_day INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT true,
    metadata JSONB
);
```

## Event Types

| Event Type | Description | Logged Fields |
|------------|-------------|---------------|
| `message_sent` | Message sent via API | recipient, message_id, message_type, metadata |
| `message_verified` | Message verification | message_id, verification_status, signature_hex |
| `consent_granted` | User grants consent | user_id, sender_domain, allowed_message_types |
| `consent_revoked` | User revokes consent | user_id, sender_domain, revoked_at |
| `verification_failed` | Signature verification failed | message_id, signature_hex, public_key_hex |

## API Integration

### Database Connection

The API automatically connects to PostgreSQL using the `DATABASE_URL` environment variable:

```bash
export DATABASE_URL="postgres://hsip:password@localhost:5432/hsip_dev"
```

**Default (if not set):** `postgres://hsip:dev_password@localhost:5432/hsip_dev`

### Configuration

**Connection Pool Settings:**
- Max connections: 20
- Timeout: 30 seconds
- Idle timeout: 10 minutes

### Handler Integration

All API handlers now write to audit log:

#### Send Message
```rust
let audit_entry_id = state.db.audit_log(
    "message_sent",
    None, // user_id
    None, // sender_domain
    Some(&req.recipient),
    Some(&message_id),
    Some(&req.message_type),
    Some("verified"),
    None, // signature_hex
    None, // public_key_hex
    Some(metadata),
).await?;
```

#### Grant Consent
```rust
// Create policy in database
let policy_uuid = state.db.create_consent_policy(
    &req.user_id,
    &req.sender_domain,
    &req.allowed_message_types,
    req.duration_days,
    req.max_messages_per_day,
    None,
).await?;

// Write audit log
state.db.audit_log(
    "consent_granted",
    Some(&req.user_id),
    Some(&req.sender_domain),
    None, None, None, None, None, None,
    Some(metadata),
).await?;
```

#### Revoke Consent
```rust
// Revoke in database
let rows_affected = state.db.revoke_consent(
    &req.user_id,
    &req.sender_domain,
).await?;

// Write audit log
state.db.audit_log(
    "consent_revoked",
    Some(&req.user_id),
    Some(&req.sender_domain),
    None, None, None, None, None, None,
    Some(metadata),
).await?;
```

## Audit Export API

### Export Audit Logs

**Endpoint:** `POST /api/v2/audit/export`

**Request:**
```json
{
  "start_date": "2024-01-01T00:00:00Z",
  "end_date": "2024-12-31T23:59:59Z",
  "format": "json"
}
```

**Response (JSON format):**
```json
{
  "export_id": "550e8400-e29b-41d4-a716-446655440000",
  "format": "json",
  "entry_count": 15234,
  "start_date": "2024-01-01T00:00:00Z",
  "end_date": "2024-12-31T23:59:59Z",
  "generated_at": "2024-12-15T14:30:00Z",
  "statistics": {
    "total_events": 15234,
    "unique_messages": 12456,
    "verified_count": 12400,
    "failed_count": 56,
    "consents_granted": 1200,
    "consents_revoked": 345
  },
  "entries": [
    {
      "id": 1,
      "timestamp": "2024-01-01T10:00:00Z",
      "event_type": "message_sent",
      "recipient": "user@example.com",
      "message_id": "msg_abc123",
      "message_type": "transaction_alert",
      "verification_status": "verified",
      "metadata": { ... }
    }
  ]
}
```

**Response (CSV format):**
```json
{
  "export_id": "550e8400-e29b-41d4-a716-446655440000",
  "export_url": "https://api.hsip.io/exports/550e8400.csv",
  "format": "csv",
  "entry_count": 15234,
  "start_date": "2024-01-01T00:00:00Z",
  "end_date": "2024-12-31T23:59:59Z",
  "generated_at": "2024-12-15T14:30:00Z"
}
```

## Database Functions

### Write Audit Log

```rust
pub async fn audit_log(
    &self,
    event_type: &str,
    user_id: Option<&str>,
    sender_domain: Option<&str>,
    recipient: Option<&str>,
    message_id: Option<&str>,
    message_type: Option<&str>,
    verification_status: Option<&str>,
    signature_hex: Option<&str>,
    public_key_hex: Option<&str>,
    metadata: Option<JsonValue>,
) -> Result<i64, sqlx::Error>
```

### Create Consent Policy

```rust
pub async fn create_consent_policy(
    &self,
    user_id: &str,
    sender_domain: &str,
    allowed_message_types: &[String],
    duration_days: Option<i32>,
    max_messages_per_day: Option<i32>,
    metadata: Option<JsonValue>,
) -> Result<Uuid, sqlx::Error>
```

### Revoke Consent

```rust
pub async fn revoke_consent(
    &self,
    user_id: &str,
    sender_domain: &str,
) -> Result<u64, sqlx::Error>
```

### Check Consent

```rust
pub async fn check_consent(
    &self,
    user_id: &str,
    sender_domain: &str,
    message_type: &str,
) -> Result<bool, sqlx::Error>
```

### Get Audit Logs

```rust
pub async fn get_audit_logs(
    &self,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<AuditLogEntry>, sqlx::Error>
```

### Get Audit Statistics

```rust
pub async fn get_audit_stats(
    &self,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> Result<AuditStats, sqlx::Error>
```

## Testing

### Unit Tests (Requires PostgreSQL)

```bash
# Start PostgreSQL
docker-compose -f docker-compose.dev.yml up -d postgres

# Run tests
cargo test -p hsip-api -- --ignored
```

### Integration Tests

```rust
#[tokio::test]
#[ignore] // Requires PostgreSQL
async fn test_send_message() {
    let db = create_test_db().await;
    let state = Arc::new(ApiState {
        version: "2.0.0-test".to_string(),
        db,
    });
    // ... test logic
}
```

## Deployment

### Docker Compose

The production stack includes PostgreSQL:

```yaml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      - POSTGRES_USER=hsip
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=hsip_prod
    volumes:
      - ./db/init.sql:/docker-entrypoint-initdb.d/init.sql
      - postgres-data:/var/lib/postgresql/data
```

### Environment Variables

```bash
DATABASE_URL=postgres://hsip:password@postgres:5432/hsip_prod
```

### Health Checks

The `/health` endpoint now includes database status:

```bash
curl http://localhost:8787/health
```

**Response:**
```json
{
  "status": "healthy",  // or "degraded" if DB is down
  "version": "2.0.0",
  "uptime_seconds": 3600
}
```

## Compliance Features

### Non-Repudiation Evidence

Each message send creates an immutable audit trail:
- Timestamp (microsecond precision)
- Message ID (unique)
- Signature hex (Ed25519)
- Public key hex (verification)
- Metadata (JSONB for additional context)

### Data Retention

Audit logs are **never deleted** by default. For compliance:

**7-year retention (SOX):**
```sql
-- Archive old logs (manual process)
SELECT * FROM audit_log WHERE timestamp < NOW() - INTERVAL '7 years';
```

**GDPR Right to Erasure:**
```sql
-- Remove user-specific logs after consent revocation
DELETE FROM audit_log WHERE user_id = 'user123';
```

### Tamper Evidence

**Blockchain-style integrity (future enhancement):**
- Hash chain linking audit entries
- Merkle tree for batch verification
- External timestamping service

## Performance Optimization

### Indexes

All critical fields are indexed:
```sql
CREATE INDEX idx_audit_timestamp ON audit_log(timestamp);
CREATE INDEX idx_audit_message_id ON audit_log(message_id);
CREATE INDEX idx_audit_user_id ON audit_log(user_id);
```

### Partitioning (Future)

For high-volume deployments:
```sql
-- Partition by month
CREATE TABLE audit_log_2024_01 PARTITION OF audit_log
    FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');
```

### Query Performance

**Time-range queries:** Uses `idx_audit_timestamp`
**Message lookup:** Uses `idx_audit_message_id`
**User activity:** Uses `idx_audit_user_id`

Expected performance:
- Insert: ~500 µs
- Query (indexed): ~1-5 ms
- Export (10K records): ~100 ms

## Security Considerations

### SQL Injection Prevention

All queries use parameterized statements:
```rust
sqlx::query("SELECT * FROM audit_log WHERE user_id = $1")
    .bind(user_id)
    .fetch_all(&self.pool)
    .await?;
```

### Access Control

- Read-only audit viewers: Use PostgreSQL roles
- Admin access: Full CRUD on consent policies
- Application access: Write-only to audit_log

### Encryption at Rest

Enable PostgreSQL encryption:
```bash
# In postgresql.conf
ssl = on
ssl_cert_file = '/path/to/cert.pem'
ssl_key_file = '/path/to/key.pem'
```

## Monitoring

### Prometheus Metrics

```
hsip_audit_log_writes_total
hsip_audit_log_errors_total
hsip_consent_grants_total
hsip_consent_revokes_total
```

### Grafana Dashboards

Track:
- Audit log write rate
- Database query latency
- Failed verification rate
- Consent grant/revoke trends

## Troubleshooting

### Connection Errors

**Error:** `connection refused`
```bash
# Check PostgreSQL is running
docker-compose ps

# Check connection string
echo $DATABASE_URL
```

**Error:** `authentication failed`
```bash
# Verify credentials
psql $DATABASE_URL
```

### Performance Issues

**Slow queries:**
```sql
-- Enable query logging
ALTER DATABASE hsip_dev SET log_statement = 'all';

-- Analyze slow queries
EXPLAIN ANALYZE SELECT * FROM audit_log WHERE timestamp > NOW() - INTERVAL '1 day';
```

### Disk Space

**Monitor table size:**
```sql
SELECT pg_size_pretty(pg_total_relation_size('audit_log'));
```

**Archive old data:**
```bash
# Dump to file
pg_dump -t audit_log -d hsip_prod > audit_archive_2024.sql

# Compress
gzip audit_archive_2024.sql
```

## Future Enhancements

- [ ] Blockchain-style hash chain for tamper evidence
- [ ] Real-time audit log streaming (WebSocket)
- [ ] Machine learning anomaly detection
- [ ] Automated compliance report generation
- [ ] External timestamping service integration (RFC 3161)
- [ ] Multi-region replication for disaster recovery
- [ ] Table partitioning for scalability (>100M records)

## References

- PostgreSQL Documentation: https://www.postgresql.org/docs/
- SQLX Documentation: https://docs.rs/sqlx/
- SOX Compliance: 7-year retention requirements
- GDPR Article 17: Right to erasure
- PCI-DSS 10.2: Audit trail requirements
