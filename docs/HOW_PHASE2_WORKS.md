# How HSIP Phase 2 Works - Visual Guide

**For:** Understanding what Phase 2 does and how to see it in action
**Audience:** Non-technical stakeholders, new developers
**Time to read:** 10 minutes

---

## 🎯 What Problem Does Phase 2 Solve?

### The Problem Today (Without HSIP Phase 2)

```
📱 Customer receives SMS:
┌─────────────────────────────────────┐
│ From: Bank Alert                    │
│                                     │
│ URGENT: Transaction of $1,234.56    │
│ at Amazon.com declined.             │
│                                     │
│ Click here to verify:               │
│ http://evil-phishing-site.com       │
└─────────────────────────────────────┘

❓ Questions customer asks:
- Is this really from my bank?
- How do I know it's not a scam?
- Should I click this link?

🔴 RESULT: Customer can't tell real from fake!
```

### The Solution: Phase 2 (Verified Messages)

```
📱 Customer receives message:
┌─────────────────────────────────────┐
│ ✅ VERIFIED MESSAGE                 │
│ From: Example Bank                  │
│                                     │
│ Transaction: $1,234.56              │
│ Merchant: Amazon.com                │
│ Card: ****4242                      │
│                                     │
│ This message is cryptographically   │
│ verified. Only your bank can send   │
│ messages with this green badge.     │
└─────────────────────────────────────┘

✅ Customer knows: Green badge = Real
🔴 No badge = Fake (automatically rejected)
```

---

## 🏗️ How Phase 2 Works (Step by Step)

### Step 1: Bank Sets Up Enterprise Identity (One-Time)

**What happens:**
```rust
// Bank creates root identity
let bank = EnterpriseIdentity::create(
    "bank.example.com",
    // Organization details
);

// Bank creates sub-identity for fraud department
let fraud_dept = bank.create_sub_identity(
    "fraud-alerts",  // Department name
    SEND_TRANSACTION_ALERTS,  // What they can do
    90_days,  // Key valid for 90 days
);
```

**What you see:**
```
✅ Root identity created: bank.example.com
   Identity ID: a1b2c3d4e5f6...
   Public Key: 32 bytes (shared with customers)
   Private Key: STORED IN HSM (never leaves secure vault)

✅ Sub-identity created: bank.example.com/fraud-alerts
   Can send: Transaction Alerts only
   Valid until: 2026-04-02
   Delegation cert: Proves this dept works for the bank
```

**Where this lives:**
- Root key: HSM (Hardware Security Module) or air-gapped vault
- Public key: Embedded in customer mobile apps
- Sub-identity keys: Secure server storage (/etc/hsip/identities/)

---

### Step 2: Customer Grants Consent (In Mobile App)

**What happens:**
```rust
// Customer's phone creates consent policy
let policy = ConsentPolicy {
    user_id: "customer@example.com",
    sender_domain: "bank.example.com",
    allowed_types: ["transaction_alert"],  // Only these
    duration: 30 days,  // Auto-renew every 30 days
    max_per_day: 10,
};
```

**What customer sees:**
```
┌─────────────────────────────────────────┐
│ Example Bank would like to send you:   │
│                                         │
│ ☑ Transaction Alerts                   │
│ ☑ Login Notices                        │
│ ☐ Marketing (you can uncheck this)     │
│                                         │
│ Duration: 30 days (auto-renewal)        │
│ Max messages/day: 10                    │
│                                         │
│ [ Grant Consent ]  [ Customize ]       │
└─────────────────────────────────────────┘
```

**Where this lives:**
- Consent policy: Stored on customer's phone AND bank's server
- Consent proof: Cryptographically signed, stored in audit log
- Can be revoked instantly in app settings

---

### Step 3: Bank Sends Transaction Alert

**What happens:**
```rust
// Bank's backend detects transaction
let transaction = detect_transaction(); // $1,234.56 at Amazon

// Fraud dept signs the message
let message = MessageMetadata {
    message_type: TransactionAlert {
        amount: Money::usd(1234, 56),
        merchant: "Amazon.com",
        transaction_id: "txn_20260102_001",
        card_last_four: "4242",
    },
    sender_domain: "bank.example.com",
    recipient: "customer@example.com",
};

// Sign with fraud dept's key
let signed_msg = fraud_dept.sign_message(message);

// Send via HSIP (or SMS with HSIP wrapper)
send_to_customer(signed_msg);
```

**What you see in logs:**
```
[2026-01-02 14:30:00] INFO Transaction detected
  Amount: $1,234.56
  Merchant: Amazon.com
  Customer: customer@example.com

[2026-01-02 14:30:01] INFO Message created
  Message ID: msg_txn_001
  Type: transaction_alert
  Expires: 2026-01-03 14:30:01

[2026-01-02 14:30:02] INFO Message signed
  Signer: bank.example.com/fraud-alerts
  Signature: a1b2c3d4... (64 bytes)
  Cert chain: 1 delegation

[2026-01-02 14:30:03] INFO Message sent
  Transport: HSIP
  Status: delivered

[2026-01-02 14:30:03] INFO Audit logged
  Entry #12345 added to audit chain
  Hash: e5f6a7b8...
```

---

### Step 4: Customer's Phone Verifies Message

**What happens:**
```rust
// Customer's phone receives message
let received_msg = receive_message();

// Verification engine checks:
let verifier = VerificationEngine::new(trust_store);
let result = verifier.verify(&received_msg);

// Steps performed:
// 1. ✅ Signature valid? (Ed25519 verify)
// 2. ✅ Cert chain valid? (fraud dept → bank root)
// 3. ✅ Bank in trust store? (public key matches)
// 4. ✅ Consent granted? (customer approved this type)
// 5. ✅ Not revoked? (check revocation list)
// 6. ✅ Not expired? (timestamp within 24 hours)

match result {
    VerificationResult::Verified => {
        // Show green badge ✅
    }
}
```

**What customer sees:**
```
📱 Phone screen:

┌─────────────────────────────────────┐
│ ✅ VERIFIED MESSAGE                 │  ← Green badge
│ Example Bank                        │  ← Sender name
│ bank.example.com/fraud-alerts       │  ← Full identity
│                                     │
│ Transaction Alert                   │
│ $1,234.56 at Amazon.com             │
│ Card: ****4242                      │
│                                     │
│ Jan 2, 2026 2:30 PM                 │
│                                     │
│ [ View Details ]                    │
└─────────────────────────────────────┘

Tap "View Details" to see:
- Full transaction info
- Verification details (cert chain, signature)
- Consent policy applied
- When this policy expires
```

---

### Step 5: What Happens with Fake Messages?

**Attacker tries to send fake alert:**
```rust
// Attacker creates identity (they can do this)
let attacker = EnterpriseIdentity::create(
    "evil-bank.com",  // Different domain
    // ...
);

// Attacker sends message claiming to be bank
let fake_msg = create_fake_transaction_alert();
let signed_fake = attacker.sign_message(fake_msg);

send_to_customer(signed_fake);
```

**What happens on verification:**
```rust
let result = verifier.verify(&signed_fake);

// Checks:
// 1. ✅ Signature valid? YES (attacker signed it correctly)
// 2. ❌ Cert chain valid? FAIL (evil-bank.com ≠ bank.example.com)
// 3. ❌ In trust store? NO (customer only trusts bank.example.com)

result == VerificationResult::Unverified
```

**What customer sees:**
```
Option A (Silent drop - recommended):
  → Message never appears
  → Customer never sees it
  → Logged in audit as "rejected"

Option B (Show warning):
┌─────────────────────────────────────┐
│ 🔴 UNVERIFIED MESSAGE               │
│ Unknown Sender                      │
│                                     │
│ This message could not be verified. │
│ It may not be legitimate.           │
│                                     │
│ ⚠️ DO NOT click any links           │
│                                     │
│ [ Delete ]  [ Report Spam ]         │
└─────────────────────────────────────┘
```

**Result:** Phishing eliminated!

---

## 📊 Where You Can SEE Phase 2 Working

### 1. Run the Example (Interactive Demo)

```bash
cd examples/bank-integration
cargo run
```

**You'll see:**
```
=== HSIP Phase 2: Bank Integration Example ===

1. Creating bank enterprise identity...
   ✅ Bank identity created: bank.example.com
   Identity ID: a1b2c3d4e5f6a7b8...

2. Creating sub-identity for fraud alerts department...
   ✅ Sub-identity created: bank.example.com/fraud-alerts
   Valid until: 2026-04-02

3. Customer grants consent for transaction alerts...
   ✅ Consent granted for 30 days
   Max messages/day: 10

4. Bank sends transaction alert...
   Transaction: $1,234.56 at Amazon.com
   ✅ Message signed by: bank.example.com/fraud-alerts

5. Customer's phone verifies message...
   Verification result: Verified
   UI indicator: ✅

6. Message display classification:
   ✅ VERIFIED MESSAGE
   Sender verified: true
   Badge color: Green
   ✅ Message will be shown to user

7. Recording to audit log...
   ✅ Audit entry created (sequence: 1)
   Audit log entries: 1
   ✅ Audit chain integrity verified

8. Demonstrating spoofing prevention...
   Scenario: Attacker tries to send fake transaction alert
   Attacker's message verification: Unverified
   UI indicator: 🔴
   🔴 REJECTED: Attacker's message blocked (not in trust store)
   ✅ User protected from phishing!

=== Example Complete ===

Key Takeaways:
✅ Bank messages show green verified badge
✅ Consent policies control what messages users receive
✅ Audit trail provides non-repudiation
✅ Spoofing/phishing attempts automatically rejected
```

---

### 2. Check the Audit Log

**After running the example:**
```bash
# Export audit log
cd examples/bank-integration
cargo run --bin export-audit

# You'll get a CSV file like:
```

| Sequence | Timestamp | Event Type | Sender | Recipient | Verification |
|----------|-----------|------------|--------|-----------|--------------|
| 1 | 2026-01-02T14:30:03Z | MessageVerified | bank.example.com/fraud-alerts | customer@example.com | Verified |
| 2 | 2026-01-02T14:30:04Z | MessageRejected | evil-bank.com | customer@example.com | Rejected |

**Integrity check:**
```bash
# Verify audit chain hasn't been tampered with
cargo run --bin verify-audit

✅ Audit chain integrity verified
   Total entries: 2
   Head hash: e5f6a7b8c9d0...
   All signatures valid
```

---

### 3. Inspect Enterprise Identities

**View bank's identity:**
```bash
cargo run --bin inspect-identity bank.example.com

Enterprise Identity:
  Domain: bank.example.com
  Organization: Example Bank
  Industry: Banking
  Country: US

  Public Key (hex):
  a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6...

  Capabilities:
  ✅ SEND_TRANSACTION_ALERTS
  ✅ SEND_LOGIN_NOTICES
  ✅ SEND_SECURITY_ALERTS
  ✅ CREATE_SUB_IDENTITIES
  ✅ REVOKE_KEYS
  ✅ AUDIT_EXPORT

  Created: 2026-01-02T12:00:00Z
```

**View sub-identity:**
```bash
cargo run --bin inspect-identity bank.example.com/fraud-alerts

Sub-Identity:
  Full Name: bank.example.com/fraud-alerts
  Parent: bank.example.com

  Public Key (hex):
  f1e2d3c4b5a6978869504132...

  Capabilities:
  ✅ SEND_TRANSACTION_ALERTS (only this one)

  Validity:
  Created: 2026-01-02T12:00:00Z
  Expires: 2026-04-02T12:00:00Z
  Status: ✅ Valid

  Delegation Certificate:
  Signed by: bank.example.com
  Signature valid: ✅
  Chain depth: 1
```

---

### 4. Test Message Verification

**Verify a message manually:**
```bash
# Create a test message
cargo run --bin create-test-message \
  --sender bank.example.com/fraud-alerts \
  --recipient customer@example.com \
  --type transaction_alert \
  --amount 1234.56

Message created: msg_test_001.json

# Verify it
cargo run --bin verify-message msg_test_001.json

Verifying message: msg_test_001.json

Step 1: Check signature
  ✅ Signature valid

Step 2: Verify certificate chain
  ✅ Chain valid (1 delegation)

Step 3: Check trust store
  ✅ Root domain trusted: bank.example.com

Step 4: Check revocation
  ✅ Key not revoked

Step 5: Check expiration
  ✅ Not expired (valid for 23 hours)

RESULT: ✅ VERIFIED
UI Display: Show green badge
```

---

### 5. Monitor Consent Policies

**Check consent status:**
```bash
cargo run --bin check-consent \
  --user customer@example.com \
  --sender bank.example.com \
  --type transaction_alert

Consent Policy:
  Policy ID: policy_001
  User: customer@example.com
  Sender: bank.example.com

  Allowed Types:
  ✅ transaction_alert
  ✅ login_notice
  ❌ marketing (denied)

  Time Window: 30 days
  Expires: 2026-02-01T12:00:00Z
  Status: ✅ Active

  Max Messages/Day: 10
  Today's count: 1 / 10

CHECK RESULT: ✅ ALLOWED
Message type "transaction_alert" is permitted
```

---

## 🎨 Visual Flow Diagram

```
┌─────────────┐                           ┌──────────────┐
│    BANK     │                           │   CUSTOMER   │
│  BACKEND    │                           │    PHONE     │
└──────┬──────┘                           └──────┬───────┘
       │                                         │
       │ 1. Transaction detected                 │
       │    ($1,234.56 at Amazon)                │
       │                                         │
       ▼                                         │
┌──────────────────┐                             │
│ Create Message   │                             │
│ - Type: txn alert│                             │
│ - Amount: $1234  │                             │
│ - Merchant: AMZN │                             │
└──────┬───────────┘                             │
       │                                         │
       ▼                                         │
┌──────────────────┐                             │
│ Sign with        │                             │
│ fraud-alerts key │                             │
│ + cert chain     │                             │
└──────┬───────────┘                             │
       │                                         │
       │ 2. Send via HSIP                        │
       │ ──────────────────────────────────────► │
       │                                         │
       │                                         ▼
       │                              ┌────────────────────┐
       │                              │ Verify Message     │
       │                              │ 1. ✅ Signature    │
       │                              │ 2. ✅ Cert chain   │
       │                              │ 3. ✅ Trust store  │
       │                              │ 4. ✅ Consent      │
       │                              │ 5. ✅ Not revoked  │
       │                              └────────┬───────────┘
       │                                       │
       │                                       ▼
       │                              ┌────────────────────┐
       │                              │ Display to User    │
       │                              │                    │
       │                              │ ✅ VERIFIED MSG    │
       │                              │ Example Bank       │
       │                              │                    │
       │                              │ Transaction:       │
       │                              │ $1,234.56 Amazon   │
       │                              └────────────────────┘
       │                                       │
       │ 3. Log to audit                       │
       ▼                                       ▼
┌──────────────────┐              ┌────────────────────┐
│ Audit Log        │              │ Audit Log          │
│ Entry #12345     │              │ (local copy)       │
│ - Msg hash       │              │ - Consent proof    │
│ - Sender         │              │ - Verification OK  │
│ - Recipient      │              │ - Timestamp        │
│ - Verified ✅    │              └────────────────────┘
└──────────────────┘
```

---

## 🔍 How to Know Phase 2 is Working

### Checklist for Success

**✅ Identity Setup:**
- [ ] Root enterprise identity created
- [ ] Public key exported (safe to share)
- [ ] Sub-identities created for departments
- [ ] Delegation certificates valid

**✅ Message Flow:**
- [ ] Messages signed with sub-identity keys
- [ ] Certificate chain included with messages
- [ ] Signature verification passes
- [ ] Trust store has bank's public key

**✅ Verification:**
- [ ] Verified messages show ✅ green badge
- [ ] Unverified messages show 🔴 warning
- [ ] Fake messages automatically rejected

**✅ Consent:**
- [ ] Customer can grant consent
- [ ] Consent policies enforced
- [ ] Customer can revoke consent instantly

**✅ Audit:**
- [ ] All messages logged
- [ ] Audit chain integrity verified
- [ ] Can export to CSV/JSON

---

## 🚀 Quick Test Commands

**Test the full flow:**
```bash
# 1. Run the example
cd examples/bank-integration && cargo run

# 2. Create identities
cargo run --bin hsip-enterprise identity create --domain yourbank.com

# 3. Create sub-identity
cargo run --bin hsip-enterprise identity create-sub \
  --parent yourbank.com \
  --name fraud-alerts \
  --capabilities SEND_TRANSACTION_ALERTS

# 4. Create test message
cargo run --bin hsip-enterprise message create \
  --sender yourbank.com/fraud-alerts \
  --type transaction_alert \
  --amount 100.00

# 5. Verify message
cargo run --bin hsip-enterprise message verify msg_001.json

# 6. Check audit log
cargo run --bin hsip-enterprise audit export --format csv
```

---

## 💡 Key Takeaways

1. **Phase 2 = Verified Messages**
   - Green ✅ badge means cryptographically verified
   - No badge = fake (automatically rejected)

2. **How You See It:**
   - Run the example: `cargo run` in `examples/bank-integration`
   - Check audit logs: CSV export with all events
   - Inspect identities: View cert chains and capabilities
   - Monitor verification: Real-time logs showing ✅/🔴

3. **What Makes It Work:**
   - Enterprise identities (root + sub-identities)
   - Certificate chains (prove delegation)
   - Trust stores (customer pins bank's public key)
   - Consent policies (user controls what they receive)
   - Audit logs (tamper-evident, exportable)

4. **The Result:**
   - Phishing eliminated (fake messages can't show ✅)
   - Compliance achieved (audit trail for regulators)
   - Customer trust (green badge = provably real)

---

**Next:** Try running the example and watch the verification flow in action!
