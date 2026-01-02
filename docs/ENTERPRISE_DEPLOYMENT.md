# HSIP Phase 2: Enterprise Deployment Guide

**For:** Banks, Fintech, Healthcare, Enterprises
**Version:** 2.0.0
**License:** Commercial / Enterprise License Required

---

## Overview

This guide walks through deploying HSIP Phase 2 in a production enterprise environment.

## Prerequisites

### Infrastructure

- **Operating System:** Linux (Ubuntu 22.04 LTS recommended) or Windows Server 2019+
- **Memory:** 8GB minimum, 16GB recommended
- **CPU:** 4 cores minimum
- **Storage:** 100GB SSD for audit logs
- **Network:** Static IP address, firewall configured

### Security Requirements

- **HSM (Hardware Security Module):** For root key storage (Thales, YubiHSM, or AWS CloudHSM)
- **TLS Certificates:** Valid SSL/TLS certificates for API endpoints
- **Firewall:** Inbound rules for API ports (default: 8443)
- **Monitoring:** Log aggregation (Splunk, ELK, Datadog)

### Compliance

- SOC 2 Type II audit preparation
- GDPR data processing agreement (for EU customers)
- HIPAA BAA (for healthcare)

---

## Phase 1: Root Identity Creation (One-Time Setup)

### Step 1: Secure Key Ceremony

**This is the most critical step. The root key must NEVER leave the HSM.**

#### Option A: Using Hardware HSM

```bash
# 1. Connect to HSM
hsip-enterprise-cli hsm connect --device /dev/hsm0

# 2. Generate root identity
hsip-enterprise-cli identity create-root \
  --domain bank.example.com \
  --organization "Example Bank" \
  --industry "Banking" \
  --country US \
  --hsm-slot 0

# 3. Export public key only (safe to distribute)
hsip-enterprise-cli identity export-public \
  --domain bank.example.com \
  --output bank-root-public.json
```

#### Option B: Air-Gapped Machine (Cold Storage)

```bash
# On an air-gapped machine with no network connectivity:

# 1. Generate root identity
cargo run --bin hsip-enterprise -- identity create-root \
  --domain bank.example.com \
  --organization "Example Bank" \
  --output /secure/keys/bank-root.key

# 2. Export public key to USB drive
cargo run --bin hsip-enterprise -- identity export-public \
  --input /secure/keys/bank-root.key \
  --output /media/usb/bank-root-public.json

# 3. Store private key in vault
# NEVER connect this machine to the internet
# NEVER copy the private key off this machine
```

#### What Gets Created

- **Private Root Key:** Stays in HSM or air-gapped vault
- **Public Root Key:** Distributed to customer apps as trust anchor
- **Root Identity Certificate:** Metadata about the organization

### Step 2: Distribute Trust Anchor to Customers

Customers must pin your bank's public key in their mobile apps.

**iOS Example (Swift):**
```swift
let bankTrustAnchor = TrustAnchor(
    domain: "bank.example.com",
    publicKey: "...", // Base64-encoded public key
    verified: true
)
HSIPClient.shared.addTrustAnchor(bankTrustAnchor)
```

**Android Example (Kotlin):**
```kotlin
val bankTrustAnchor = TrustAnchor(
    domain = "bank.example.com",
    publicKey = "...", // Base64-encoded public key
    verified = true
)
HSIPClient.getInstance().addTrustAnchor(bankTrustAnchor)
```

---

## Phase 2: Deploy Enterprise Server

### Step 1: Install HSIP Enterprise

```bash
# Clone repository (private repo for licensed customers)
git clone https://github.com/nyx-systems/hsip-enterprise.git
cd hsip-enterprise

# Build in release mode
cargo build --release

# Install to /opt/hsip
sudo mkdir -p /opt/hsip
sudo cp target/release/hsip-enterprise /opt/hsip/
sudo cp target/release/hsip-api-server /opt/hsip/
```

### Step 2: Configuration

Create `/etc/hsip/config.toml`:

```toml
[server]
bind_address = "0.0.0.0:8443"
tls_cert = "/etc/hsip/certs/server.crt"
tls_key = "/etc/hsip/certs/server.key"

[identity]
root_domain = "bank.example.com"
root_public_key_path = "/etc/hsip/keys/bank-root-public.json"
# For HSM:
hsm_enabled = true
hsm_slot = 0

[audit]
log_path = "/var/log/hsip/audit.log"
rotate_daily = true
export_path = "/var/hsip/audit-exports"

[policy]
default_consent_duration_days = 30
max_messages_per_day = 100

[database]
# For production, use PostgreSQL
url = "postgresql://hsip:password@localhost/hsip_production"
```

### Step 3: Start Service

```bash
# Create systemd service
sudo nano /etc/systemd/system/hsip-enterprise.service
```

```ini
[Unit]
Description=HSIP Enterprise API Server
After=network.target

[Service]
Type=simple
User=hsip
Group=hsip
ExecStart=/opt/hsip/hsip-api-server --config /etc/hsip/config.toml
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

```bash
# Start service
sudo systemctl daemon-reload
sudo systemctl enable hsip-enterprise
sudo systemctl start hsip-enterprise

# Check status
sudo systemctl status hsip-enterprise
```

---

## Phase 3: Create Sub-Identities for Departments

### Fraud Alerts Department

```bash
hsip-enterprise-cli identity create-sub \
  --parent bank.example.com \
  --name fraud-alerts \
  --capabilities SEND_TRANSACTION_ALERTS \
  --duration-days 90 \
  --output /etc/hsip/identities/fraud-alerts.key
```

### Login Notices Department

```bash
hsip-enterprise-cli identity create-sub \
  --parent bank.example.com \
  --name login-notices \
  --capabilities SEND_LOGIN_NOTICES \
  --duration-days 90 \
  --output /etc/hsip/identities/login-notices.key
```

### Key Rotation (Every 90 Days)

```bash
# Automated via cron
0 0 1 * * /opt/hsip/scripts/rotate-sub-identities.sh
```

---

## Phase 4: Integrate with Bank Backend

### Transaction Alert Integration

**Python SDK Example:**

```python
from hsip_enterprise import EnterpriseClient, MessageType

# Initialize client
client = EnterpriseClient(
    identity_path="/etc/hsip/identities/fraud-alerts.key",
    api_endpoint="https://hsip.bank.example.com/api/v2"
)

# Send transaction alert
def send_transaction_alert(customer_id, amount, merchant, txn_id):
    message = MessageType.TransactionAlert(
        amount=amount,
        merchant=merchant,
        transaction_id=txn_id,
        timestamp=datetime.utcnow(),
        card_last_four="1234"
    )

    result = client.send_message(
        recipient=customer_id,
        message=message
    )

    if result.verified:
        print(f"✅ Message sent and verified: {result.message_id}")
        print(f"Audit entry: {result.audit_entry_id}")
    else:
        print(f"❌ Message failed verification")

# Example usage
send_transaction_alert(
    customer_id="user@example.com",
    amount=125.50,
    merchant="Amazon.com",
    txn_id="txn_20260102_001"
)
```

---

## Phase 5: Customer Consent Management

### Customer Onboarding Flow

1. **Customer downloads bank's mobile app**
2. **App includes HSIP SDK with bank's trust anchor pinned**
3. **On first launch, app prompts for consent**:

```
Example Bank would like to send you verified messages:
☑ Transaction Alerts
☑ Login Notices
☐ Marketing (optional)

These messages will show a ✅ VERIFIED badge.
Duration: 30 days (auto-renewal)
Max messages/day: 10

[Grant Consent] [Customize]
```

4. **User grants consent → Creates policy → Generates consent proof**
5. **Policy synced to HSIP servers**

### Revocation Flow

```python
# Customer revokes consent via app
client.revoke_consent(
    user_id="user@example.com",
    sender_domain="bank.example.com"
)
# Immediate effect: Future messages blocked
```

---

## Phase 6: Audit & Compliance

### Daily Audit Export

```bash
# Automated export for compliance team
hsip-enterprise-cli audit export \
  --start-date 2026-01-01 \
  --end-date 2026-01-02 \
  --format csv \
  --output /var/hsip/audit-exports/2026-01-02.csv
```

### SOC 2 Audit Preparation

Provide auditors with:
1. **Audit log exports** (tamper-evident, hash-chained)
2. **Key rotation logs** (who, when, why)
3. **Access control logs** (who can create sub-identities)
4. **Revocation lists** (which keys were revoked)

### Regulator Queries

```bash
# Export all messages to a specific customer
hsip-enterprise-cli audit query \
  --recipient user@example.com \
  --start-date 2025-01-01 \
  --end-date 2026-01-02 \
  --format json \
  --output regulator-request-001.json
```

---

## Phase 7: Monitoring & Operations

### Health Checks

```bash
# Endpoint: https://hsip.bank.example.com/health
curl https://hsip.bank.example.com/health

# Response:
{
  "status": "healthy",
  "version": "2.0.0",
  "uptime_seconds": 864000
}
```

### Metrics to Monitor

| Metric | Alert Threshold |
|--------|----------------|
| Message verification rate | < 95% |
| API latency (P99) | > 200ms |
| Audit log write failures | > 0 |
| Key rotation failures | > 0 |
| Failed login attempts to API | > 10/hour |

### Log Aggregation

Forward logs to SIEM:

```bash
# Filebeat configuration for ELK
filebeat.inputs:
  - type: log
    paths:
      - /var/log/hsip/*.log
    fields:
      service: hsip-enterprise
      environment: production

output.elasticsearch:
  hosts: ["elk.bank.example.com:9200"]
```

---

## Phase 8: Disaster Recovery

### Backup Strategy

**Critical Data:**
- Root public key (stored redundantly)
- Sub-identity keys (backed up encrypted)
- Audit logs (replicated to S3/Azure Blob)
- Consent policies (database backups)

**Recovery Plan:**
1. Restore service on new infrastructure
2. Load root public key (from HSM or cold storage)
3. Regenerate sub-identities (old ones still valid until expiry)
4. Restore audit log from backups
5. Re-sync consent policies

### Root Key Compromise

**If root key is compromised (CRITICAL INCIDENT):**

1. **Immediately:** Publish revocation notice
2. **Within 1 hour:** Notify all customers via traditional channels
3. **Within 24 hours:** Generate new root key, distribute to customers
4. **Within 7 days:** Force app updates with new trust anchor

---

## Licensing & Support

### Commercial License

Contact: sales@nyxsystems.com

**Tiers:**
- **Starter:** $5,000/month (up to 100K messages/month)
- **Professional:** $15,000/month (up to 1M messages/month)
- **Enterprise:** Custom pricing (unlimited, on-premise option)

### Support Channels

- **Email:** support@nyxsystems.com (24-hour SLA)
- **Slack:** Private channel for Enterprise customers
- **Emergency:** +1-XXX-XXX-XXXX (24/7 for Critical issues)

---

## Appendix A: Security Checklist

- [ ] Root key stored in HSM or air-gapped vault
- [ ] TLS certificates valid and auto-renewing
- [ ] Firewall rules configured (only port 8443 open)
- [ ] Sub-identities rotate every 90 days
- [ ] Audit logs backed up daily
- [ ] Monitoring alerts configured
- [ ] SOC 2 controls mapped
- [ ] Disaster recovery plan tested
- [ ] Customer trust anchors distributed
- [ ] API rate limiting enabled

---

**Document Version:** 2.0.0
**Last Updated:** 2026-01-02
**Author:** Nyx Systems LLC
