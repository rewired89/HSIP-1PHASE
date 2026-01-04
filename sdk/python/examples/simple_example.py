#!/usr/bin/env python3
"""
Simple example of using HSIP Bank SDK to sign messages
"""

from hsip_bank import BankMessageSigner, MessageType, generate_keypair, save_keypair

# For this example, we'll generate a temporary keypair
# In production, you'd use keys generated with hsip-keygen CLI
print("=" * 60)
print("HSIP Bank SDK - Simple Example")
print("=" * 60)
print()

# Step 1: Generate keys (one-time setup)
print("Step 1: Generating bank keypair...")
private_key, public_key = generate_keypair()
private_key_path, public_key_path = save_keypair(
    private_key,
    public_key,
    "example-bank.com",
    output_dir="./temp_keys"
)
print()

# Step 2: Initialize signer
print("Step 2: Initializing message signer...")
signer = BankMessageSigner(
    private_key_path=private_key_path,
    domain="example-bank.com"
)
print(f"✅ Signer ready for domain: example-bank.com")
print(f"   Public key: {signer.get_public_key_hex()[:32]}...")
print()

# Step 3: Sign different types of messages
print("Step 3: Signing messages...")
print()

# Transaction alert
print("📱 Transaction Alert:")
transaction_msg = signer.sign_message(
    message_text="Transaction: $50.00 at Starbucks",
    message_type=MessageType.TRANSACTION_ALERT,
    recipient="customer@email.com"
)
print(f"   Message: {transaction_msg.message_text}")
print(f"   Signature: {transaction_msg.signature[:32]}...")
print()

# Security alert
print("🔒 Security Alert:")
security_msg = signer.sign_message(
    message_text="New login from Chrome on Windows",
    message_type=MessageType.SECURITY_ALERT,
    recipient="customer@email.com"
)
print(f"   Message: {security_msg.message_text}")
print(f"   Signature: {security_msg.signature[:32]}...")
print()

# Account update
print("📄 Account Update:")
account_msg = signer.sign_message(
    message_text="Your monthly statement is ready to view",
    message_type=MessageType.ACCOUNT_UPDATE,
    recipient="customer@email.com"
)
print(f"   Message: {account_msg.message_text}")
print(f"   Signature: {account_msg.signature[:32]}...")
print()

# Step 4: Show what you'd send via push notification
print("Step 4: Push notification payload format:")
print()
payload = transaction_msg.to_notification_payload()
import json
print(json.dumps(payload, indent=2))
print()

print("=" * 60)
print("✅ Example complete!")
print()
print("Next steps:")
print("  1. Store private key securely (HSM in production)")
print("  2. Distribute public key to mobile apps")
print("  3. Integrate with your push notification system")
print("  4. Add HSIP mobile SDKs to iOS/Android apps")
print("=" * 60)
