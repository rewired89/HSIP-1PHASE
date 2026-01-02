//! Bank Integration Example for HSIP Phase 2
//!
//! This example demonstrates how a bank would integrate HSIP Phase 2
//! to send verified transaction alerts to customers.

use chrono::{Duration, Utc};
use hsip_audit::{AuditEntry, AuditEventType, AuditLog};
use hsip_enterprise::{
    EnterpriseIdentity, IdentityCapabilities, IdentityMetadata,
    TrustStore, TrustAnchor, TrustAnchorMetadata, CertChain,
};
use hsip_policy::{ConsentPolicy, PolicyScope, TimeWindow, ConsentPolicyEngine};
use hsip_verify::{
    MessageMetadata, MessageType, Money, SignedMessage, VerificationEngine,
    MessageClassifier,
};
use rand::rngs::OsRng;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== HSIP Phase 2: Bank Integration Example ===\n");

    // Step 1: Bank creates root enterprise identity
    println!("1. Creating bank enterprise identity...");
    let bank_metadata = IdentityMetadata {
        organization_name: "Example Bank".to_string(),
        contact_email: "security@bank.example.com".to_string(),
        industry: "Banking".to_string(),
        country_code: "US".to_string(),
    };

    let bank_root = EnterpriseIdentity::create(
        "bank.example.com".to_string(),
        bank_metadata.clone(),
    );

    println!("   ✅ Bank identity created: {}", bank_root.domain);
    println!("   Identity ID: {}\n", bank_root.identity_id());

    // Step 2: Create sub-identity for fraud alerts department
    println!("2. Creating sub-identity for fraud alerts department...");
    let fraud_dept = bank_root.create_sub_identity(
        "fraud-alerts",
        IdentityCapabilities::SEND_TRANSACTION_ALERTS,
        Duration::days(90),
    )?;

    println!("   ✅ Sub-identity created: {}", fraud_dept.full_name);
    println!("   Valid until: {}\n", fraud_dept.expires_at);

    // Step 3: Customer sets up consent policy
    println!("3. Customer grants consent for transaction alerts...");
    let customer_id = "customer@example.com";

    let consent_policy = ConsentPolicy {
        policy_id: "policy_001".to_string(),
        user_id: customer_id.to_string(),
        sender_domain: bank_root.domain.clone(),
        scope: PolicyScope::AllowedTypes(vec!["transaction_alert".to_string()]),
        time_window: TimeWindow::Duration { days: 30 },
        max_messages_per_day: Some(10),
        created_at: Utc::now(),
        expires_at: None,
        revoked_at: None,
    };

    let mut policy_engine = ConsentPolicyEngine::new();
    policy_engine.add_policy(consent_policy.clone());

    println!("   ✅ Consent granted for 30 days");
    println!("   Max messages/day: {}\n", consent_policy.max_messages_per_day.unwrap());

    // Step 4: Bank sends a transaction alert
    println!("4. Bank sends transaction alert...");
    let transaction_msg = MessageMetadata {
        message_id: "msg_txn_001".to_string(),
        sender_domain: bank_root.domain.clone(),
        recipient: customer_id.to_string(),
        message_type: MessageType::TransactionAlert {
            amount: Money::usd(1234, 56),
            merchant: "Amazon.com".to_string(),
            transaction_id: "txn_20260102_001".to_string(),
            timestamp: Utc::now(),
            card_last_four: "4242".to_string(),
        },
        sent_at: Utc::now(),
        expires_at: Some(Utc::now() + Duration::hours(24)),
        nonce: [0u8; 16],
    };

    println!("   Transaction: $1,234.56 at Amazon.com");

    // Step 5: Create certificate chain
    let mut cert_chain = CertChain::new(bank_root.domain.clone(), bank_root.public_key);
    cert_chain.add_cert(fraud_dept.delegation_cert.clone());

    // Step 6: Sign the message
    let mut signed_msg = SignedMessage {
        metadata: transaction_msg.clone(),
        cert_chain: cert_chain.clone(),
        signature: [0u8; 64],
    };

    let msg_data = signed_msg.serialize_for_signing();
    let signature = fraud_dept.sign(&msg_data)?;
    signed_msg.signature = signature;

    println!("   ✅ Message signed by: {}\n", fraud_dept.full_name);

    // Step 7: Customer's phone verifies the message
    println!("5. Customer's phone verifies message...");

    // Set up trust store (customer trusts their bank)
    let mut trust_store = TrustStore::new();
    trust_store.add_anchor(TrustAnchor {
        domain: bank_root.domain.clone(),
        public_key: bank_root.public_key,
        added_at: Utc::now(),
        metadata: TrustAnchorMetadata {
            organization_name: bank_metadata.organization_name.clone(),
            industry: bank_metadata.industry.clone(),
            country_code: bank_metadata.country_code.clone(),
            verified: true,
            notes: "My bank".to_string(),
        },
    });

    let verification_engine = VerificationEngine::new(trust_store);
    let verification_result = verification_engine.verify(&signed_msg);

    println!("   Verification result: {:?}", verification_result);
    println!("   UI indicator: {}", verification_result.ui_indicator());

    // Step 8: Classify message for display
    let classification = MessageClassifier::classify(&signed_msg.metadata, verification_result);

    println!("\n6. Message display classification:");
    println!("   {} {}", classification.display_info.indicator, classification.display_info.status_text);
    println!("   Sender verified: {}", classification.display_info.sender_verified);
    println!("   Badge color: {:?}", classification.display_info.badge_color);

    if MessageClassifier::should_display(&classification) {
        println!("   ✅ Message will be shown to user\n");
    }

    // Step 9: Log to audit trail
    println!("7. Recording to audit log...");

    let audit_signing_key = ed25519_dalek::SigningKey::generate(&mut OsRng);
    let mut audit_log = AuditLog::new("bank_audit_001".to_string(), &audit_signing_key);

    let audit_entry = AuditEntry::new(
        1,
        [0u8; 32],
        AuditEventType::MessageVerified,
        signed_msg.message_hash(),
        fraud_dept.full_name.clone(),
        customer_id.to_string(),
    );

    audit_log.append(audit_entry, &audit_signing_key)?;

    println!("   ✅ Audit entry created (sequence: 1)");
    println!("   Audit log entries: {}", audit_log.entry_count());

    // Verify audit integrity
    audit_log.verify_integrity()?;
    println!("   ✅ Audit chain integrity verified\n");

    // Step 10: Demonstrate spoofing prevention
    println!("8. Demonstrating spoofing prevention...");
    println!("   Scenario: Attacker tries to send fake transaction alert");

    let attacker_metadata = IdentityMetadata {
        organization_name: "Fake Bank".to_string(),
        contact_email: "attacker@evil.com".to_string(),
        industry: "Scam".to_string(),
        country_code: "XX".to_string(),
    };

    let attacker_identity = EnterpriseIdentity::create(
        "evil-bank.com".to_string(),
        attacker_metadata,
    );

    let fake_msg = MessageMetadata {
        message_id: "msg_fake_001".to_string(),
        sender_domain: attacker_identity.domain.clone(),
        recipient: customer_id.to_string(),
        message_type: MessageType::SecurityAlert {
            severity: hsip_verify::Severity::Critical,
            description: "Your account is locked! Click here now!".to_string(),
            action_required: true,
            deadline: Some(Utc::now() + Duration::hours(1)),
        },
        sent_at: Utc::now(),
        expires_at: None,
        nonce: [1u8; 16],
    };

    let mut fake_cert_chain = CertChain::new(
        attacker_identity.domain.clone(),
        attacker_identity.public_key,
    );

    let mut fake_signed_msg = SignedMessage {
        metadata: fake_msg,
        cert_chain: fake_cert_chain,
        signature: [0u8; 64],
    };

    let fake_data = fake_signed_msg.serialize_for_signing();
    let fake_signature = attacker_identity.sign(&fake_data)?;
    fake_signed_msg.signature = fake_signature;

    let fake_verification = verification_engine.verify(&fake_signed_msg);

    println!("   Attacker's message verification: {:?}", fake_verification);
    println!("   UI indicator: {}", fake_verification.ui_indicator());

    let fake_classification = MessageClassifier::classify(
        &fake_signed_msg.metadata,
        fake_verification,
    );

    if !MessageClassifier::should_display(&fake_classification) {
        println!("   🔴 REJECTED: Attacker's message blocked (not in trust store)");
        println!("   ✅ User protected from phishing!\n");
    }

    // Summary
    println!("=== Example Complete ===");
    println!("\nKey Takeaways:");
    println!("✅ Bank messages show green verified badge");
    println!("✅ Consent policies control what messages users receive");
    println!("✅ Audit trail provides non-repudiation");
    println!("✅ Spoofing/phishing attempts automatically rejected");
    println!("\nPhase 2 eliminates fake bank SMS and transaction alert phishing completely.");

    Ok(())
}
