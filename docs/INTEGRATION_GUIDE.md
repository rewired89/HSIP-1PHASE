# HSIP Phase 2: Integration Guide for Developers

**Target Audience:** Software Engineers integrating HSIP into banking/fintech/healthcare systems
**Version:** 2.0.0

---

## Quick Start (5 Minutes)

### 1. Add Dependencies

**Cargo.toml:**
```toml
[dependencies]
hsip-enterprise = "2.0"
hsip-verify = "2.0"
hsip-policy = "2.0"
hsip-audit = "2.0"
tokio = { version = "1", features = ["full"] }
```

### 2. Initialize Enterprise Identity

```rust
use hsip_enterprise::{EnterpriseIdentity, IdentityMetadata};

let metadata = IdentityMetadata {
    organization_name: "Your Bank".to_string(),
    contact_email: "security@yourbank.com".to_string(),
    industry: "Banking".to_string(),
    country_code: "US".to_string(),
};

// In production, load from HSM
let root_identity = EnterpriseIdentity::create(
    "yourbank.com".to_string(),
    metadata,
);
```

### 3. Send a Verified Message

```rust
use hsip_verify::{MessageType, MessageMetadata, Money, SignedMessage};
use hsip_enterprise::CertChain;
use chrono::Utc;

// Create sub-identity for your department
let fraud_alerts = root_identity.create_sub_identity(
    "fraud-alerts",
    IdentityCapabilities::SEND_TRANSACTION_ALERTS,
    chrono::Duration::days(90),
)?;

// Create message
let message = MessageMetadata {
    message_id: "msg_001".to_string(),
    sender_domain: "yourbank.com".to_string(),
    recipient: "customer@example.com".to_string(),
    message_type: MessageType::TransactionAlert {
        amount: Money::usd(100, 50),
        merchant: "Starbucks".to_string(),
        transaction_id: "txn_123".to_string(),
        timestamp: Utc::now(),
        card_last_four: "4242".to_string(),
    },
    sent_at: Utc::now(),
    expires_at: None,
    nonce: [0u8; 16], // Generate random in production
};

// Build cert chain
let mut chain = CertChain::new(root_identity.domain.clone(), root_identity.public_key);
chain.add_cert(fraud_alerts.delegation_cert.clone());

// Sign message
let mut signed_msg = SignedMessage {
    metadata: message,
    cert_chain: chain,
    signature: [0u8; 64],
};

let data = signed_msg.serialize_for_signing();
let signature = fraud_alerts.sign(&data)?;
signed_msg.signature = signature;

// Send via HSIP transport (or your own transport layer)
send_to_customer(signed_msg);
```

---

## Common Integration Patterns

### Pattern 1: Transaction Alert System

**Use Case:** Send real-time transaction alerts to customers

```rust
use hsip_verify::{MessageType, Money};

pub struct TransactionAlertService {
    fraud_dept: SubIdentity,
}

impl TransactionAlertService {
    pub async fn send_alert(
        &self,
        customer_id: &str,
        amount: f64,
        merchant: &str,
        txn_id: &str,
    ) -> Result<String> {
        let message = MessageMetadata {
            message_id: generate_message_id(),
            sender_domain: "yourbank.com".to_string(),
            recipient: customer_id.to_string(),
            message_type: MessageType::TransactionAlert {
                amount: Money::from_dollars(amount),
                merchant: merchant.to_string(),
                transaction_id: txn_id.to_string(),
                timestamp: Utc::now(),
                card_last_four: get_card_last_four(customer_id),
            },
            sent_at: Utc::now(),
            expires_at: Some(Utc::now() + Duration::hours(24)),
            nonce: generate_nonce(),
        };

        let signed = self.sign_message(message)?;
        self.send_via_hsip(signed).await
    }
}
```

### Pattern 2: Consent Management in Customer App

**Use Case:** Customer app managing consent policies

```rust
use hsip_policy::{ConsentPolicy, PolicyScope, TimeWindow};

pub struct ConsentManager {
    policy_engine: ConsentPolicyEngine,
}

impl ConsentManager {
    pub fn grant_consent_to_bank(
        &mut self,
        user_id: &str,
        bank_domain: &str,
        allowed_types: Vec<String>,
        duration_days: u32,
    ) -> Result<String> {
        let policy = ConsentPolicy {
            policy_id: generate_policy_id(),
            user_id: user_id.to_string(),
            sender_domain: bank_domain.to_string(),
            scope: PolicyScope::AllowedTypes(allowed_types),
            time_window: TimeWindow::Duration { days: duration_days },
            max_messages_per_day: Some(10),
            created_at: Utc::now(),
            expires_at: None,
            revoked_at: None,
        };

        self.policy_engine.add_policy(policy.clone());
        Ok(policy.policy_id)
    }

    pub fn check_message_allowed(
        &self,
        user_id: &str,
        sender_domain: &str,
        message_type: &str,
    ) -> bool {
        use hsip_policy::PolicyCheckResult;

        let context = MessageContext {
            message_type: message_type.to_string(),
            amount: None,
            origin_country: None,
            origin_ip: None,
            sender_full_name: None,
        };

        matches!(
            self.policy_engine.check_consent(user_id, sender_domain, message_type, &context),
            PolicyCheckResult::Allowed
        )
    }
}
```

### Pattern 3: Message Verification in Mobile App

**Use Case:** iOS/Android app verifying incoming messages

```rust
use hsip_verify::{VerificationEngine, MessageClassifier};
use hsip_enterprise::TrustStore;

pub struct MessageVerifier {
    verification_engine: VerificationEngine,
}

impl MessageVerifier {
    pub fn new() -> Self {
        let mut trust_store = TrustStore::new();

        // Add user's bank as trusted
        trust_store.add_anchor(TrustAnchor {
            domain: "mybank.com".to_string(),
            public_key: BANK_PUBLIC_KEY, // Embedded in app
            added_at: Utc::now(),
            metadata: TrustAnchorMetadata {
                organization_name: "My Bank".to_string(),
                industry: "Banking".to_string(),
                country_code: "US".to_string(),
                verified: true,
                notes: "Official bank".to_string(),
            },
        });

        let verification_engine = VerificationEngine::new(trust_store);

        Self { verification_engine }
    }

    pub fn verify_and_classify(
        &self,
        signed_msg: &SignedMessage,
    ) -> (VerificationResult, ClassificationResult) {
        let verification = self.verification_engine.verify(signed_msg);
        let classification = MessageClassifier::classify(&signed_msg.metadata, verification);

        (verification, classification)
    }

    pub fn should_show_message(&self, signed_msg: &SignedMessage) -> bool {
        let (_, classification) = self.verify_and_classify(signed_msg);
        MessageClassifier::should_display(&classification)
    }
}

// Usage in UI layer
fn display_message(verifier: &MessageVerifier, signed_msg: SignedMessage) {
    let (verification, classification) = verifier.verify_and_classify(&signed_msg);

    if MessageClassifier::should_display(&classification) {
        // Show message with badge
        println!("{} {}",
            classification.display_info.indicator,
            classification.display_info.status_text
        );

        if let MessageType::TransactionAlert { amount, merchant, .. } = &signed_msg.metadata.message_type {
            println!("Transaction: {} at {}", amount.format(), merchant);
        }
    } else {
        // Silent drop or show warning
        println!("🔴 Message rejected: unverified sender");
    }
}
```

### Pattern 4: Audit Logging for Compliance

**Use Case:** Log all message activity for regulatory compliance

```rust
use hsip_audit::{AuditLog, AuditEntry, AuditEventType, ExportFormat, AuditExporter};

pub struct ComplianceLogger {
    audit_log: AuditLog,
    signing_key: SigningKey,
}

impl ComplianceLogger {
    pub fn log_message_sent(
        &mut self,
        message_hash: [u8; 32],
        sender: &str,
        recipient: &str,
    ) -> Result<u64> {
        let prev_hash = self.audit_log.head_hash().unwrap_or([0u8; 32]);
        let seq = self.audit_log.entry_count() as u64 + 1;

        let entry = AuditEntry::new(
            seq,
            prev_hash,
            AuditEventType::MessageSent,
            message_hash,
            sender.to_string(),
            recipient.to_string(),
        );

        self.audit_log.append(entry, &self.signing_key)?;
        Ok(seq)
    }

    pub fn export_for_regulator(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<String> {
        // Filter entries by date range
        let filtered_log = self.filter_by_date(&self.audit_log, start_date, end_date);

        // Export to CSV
        let csv = AuditExporter::export(&filtered_log, ExportFormat::Csv)?;

        Ok(csv)
    }

    pub fn verify_log_integrity(&self) -> Result<()> {
        self.audit_log.verify_integrity()?;
        println!("✅ Audit log integrity verified");
        Ok(())
    }
}
```

---

## API Reference (REST Endpoints)

### POST /api/v2/messages/send

**Send a verified message**

**Request:**
```json
{
  "recipient": "customer@example.com",
  "message_type": "transaction_alert",
  "content": {
    "amount": 125.50,
    "merchant": "Amazon.com",
    "transaction_id": "txn_001"
  }
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "message_id": "msg_001",
    "verification_status": "verified",
    "audit_entry_id": 12345
  }
}
```

### POST /api/v2/messages/verify

**Verify a message signature**

**Request:**
```json
{
  "message_data": "base64_encoded_signed_message"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "verified": true,
    "sender_domain": "bank.example.com",
    "message_type": "transaction_alert",
    "verification_details": "Valid cert chain and signature"
  }
}
```

### POST /api/v2/consent/grant

**Grant consent policy**

**Request:**
```json
{
  "user_id": "user@example.com",
  "sender_domain": "bank.example.com",
  "allowed_message_types": ["transaction_alert", "login_notice"],
  "duration_days": 30,
  "max_messages_per_day": 10
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "policy_id": "policy_001",
    "created_at": "2026-01-02T12:00:00Z"
  }
}
```

### POST /api/v2/audit/export

**Export audit log**

**Request:**
```json
{
  "start_date": "2026-01-01T00:00:00Z",
  "end_date": "2026-01-02T00:00:00Z",
  "format": "csv"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "export_url": "https://example.com/audit/export_123.csv",
    "entry_count": 1500,
    "format": "csv"
  }
}
```

---

## Error Handling

### Common Errors

| Error | Code | Cause | Solution |
|-------|------|-------|----------|
| `CertChainInvalid` | 400 | Broken delegation chain | Regenerate sub-identity |
| `KeyRevoked` | 403 | Key in revocation list | Rotate key |
| `ConsentDenied` | 403 | User hasn't granted consent | Prompt user for consent |
| `PolicyExpired` | 403 | Consent policy expired | Request renewal |
| `InvalidSignature` | 401 | Signature verification failed | Check signing key |
| `MessageExpired` | 400 | Message TTL exceeded | Resend with new timestamp |

### Example Error Response

```json
{
  "success": false,
  "error": "ConsentDenied: User has not granted consent for message type 'marketing'",
  "timestamp": "2026-01-02T12:00:00Z"
}
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_verification() {
        let bank = create_test_bank_identity();
        let fraud_dept = bank.create_sub_identity(...).unwrap();

        let message = create_test_message();
        let signed = sign_message(&fraud_dept, message).unwrap();

        let verifier = create_test_verifier();
        let result = verifier.verify(&signed);

        assert_eq!(result, VerificationResult::Verified);
    }
}
```

### Integration Tests

Run the provided example:

```bash
cd examples/bank-integration
cargo run

# Expected output:
# ✅ Bank identity created: bank.example.com
# ✅ Sub-identity created: bank.example.com/fraud-alerts
# ✅ Consent granted for 30 days
# ✅ Message signed by: bank.example.com/fraud-alerts
# ✅ VERIFIED MESSAGE
# ✅ User protected from phishing!
```

---

## Performance Considerations

### Benchmarks (Reference Hardware: 4-core CPU, 16GB RAM)

| Operation | Latency (P50) | Latency (P99) |
|-----------|---------------|---------------|
| Sign message | 0.5ms | 2ms |
| Verify signature | 0.8ms | 3ms |
| Verify cert chain | 1.2ms | 5ms |
| Full verification | 2ms | 8ms |
| Audit log append | 1ms | 4ms |
| Policy check | 0.2ms | 1ms |

### Optimization Tips

1. **Cache verification results** (message hash → verification result) for 60 seconds
2. **Batch audit logging** (buffer 100 entries, write in batch)
3. **Preload trust store** (don't read from disk on every verification)
4. **Connection pooling** (reuse HTTP clients for API calls)

---

## Security Best Practices

### DO ✅

- Store root key in HSM or air-gapped vault
- Rotate sub-identity keys every 90 days
- Validate message timestamps (reject if > 5 minutes old)
- Rate limit API endpoints (100 requests/min per IP)
- Log all audit events (tamper-evident)
- Use TLS 1.3 for all network communication
- Pin trust anchors in mobile apps (no dynamic loading)

### DON'T ❌

- Store private keys in plaintext files
- Skip certificate chain validation
- Accept messages without valid signatures
- Expose private keys in logs/errors
- Allow wildcard trust anchors
- Disable audit logging for "testing"
- Use self-signed TLS certificates in production

---

## Troubleshooting

### Issue: "CertChainInvalid: Parent public key mismatch"

**Cause:** Sub-identity cert doesn't match root public key

**Solution:**
```rust
// Verify the chain manually
let chain = CertChain::new(root.domain, root.public_key);
chain.add_cert(sub.delegation_cert);
chain.validate()?; // Will show exact error
```

### Issue: "ConsentDenied" for valid customer

**Cause:** Policy expired or not yet created

**Solution:**
```rust
// Check policy status
let policy = policy_engine.get_policy(user_id, sender_domain);
if let Some(p) = policy {
    println!("Policy valid: {}", p.is_valid());
    println!("Expires: {:?}", p.expires_at);
}
```

### Issue: High API latency

**Cause:** Not caching trust store lookups

**Solution:**
```rust
// Cache trust store in memory
lazy_static! {
    static ref TRUST_STORE: Arc<Mutex<TrustStore>> = {
        let mut store = TrustStore::new();
        // Load from config once
        load_trust_anchors(&mut store);
        Arc::new(Mutex::new(store))
    };
}
```

---

## Support

### Community

- **GitHub:** https://github.com/nyxsystems/hsip-enterprise (licensed customers only)
- **Documentation:** https://docs.hsip.dev

### Commercial Support

- **Email:** support@nyxsystems.com
- **SLA:** 24-hour response for Enterprise customers
- **Emergency:** +1-XXX-XXX-XXXX (24/7 on-call)

---

**Document Version:** 2.0.0
**Last Updated:** 2026-01-02
**Author:** Nyx Systems LLC
