#!/usr/bin/env python3
"""
Basic test of HSIP Bank SDK structure (without running)
"""

import sys
import os

# Add SDK to path
sys.path.insert(0, os.path.dirname(__file__))

print("=" * 60)
print("HSIP Bank SDK - Structure Test")
print("=" * 60)
print()

# Test imports work
try:
    from hsip_bank import types
    print("✅ types module loaded")
    print(f"   - MessageType enum: {list(types.MessageType)[:3]}...")
    print()
except Exception as e:
    print(f"❌ Failed to load types: {e}")
    print()

# Test that all expected classes/functions exist
from hsip_bank import types

print("✅ Module structure:")
print(f"   - MessageType: {types.MessageType}")
print(f"   - SignedMessage: {types.SignedMessage}")
print()

# Create a sample signed message (without actual cryptography)
sample_msg = types.SignedMessage(
    message_id="msg_test123",
    sender_domain="chase.com",
    recipient="customer@email.com",
    message_type="transaction_alert",
    message_text="Transaction: $50.00 at Starbucks",
    sent_at="2025-01-04T12:00:00Z",
    signature="a1b2c3d4" * 16,  # Fake signature for demo
    public_key="e5f6g7h8" * 8,  # Fake public key for demo
)

print("✅ Sample SignedMessage created:")
print(f"   Message ID: {sample_msg.message_id}")
print(f"   Domain: {sample_msg.sender_domain}")
print(f"   Text: {sample_msg.message_text}")
print()

# Test conversion to dict
msg_dict = sample_msg.to_dict()
print("✅ to_dict() works:")
for key in ["message_id", "message_text", "signature"][:3]:
    print(f"   {key}: {msg_dict[key][:30]}...")
print()

# Test notification payload
payload = sample_msg.to_notification_payload()
print("✅ to_notification_payload() works:")
print(f"   title: {payload['title']}")
print(f"   body: {payload['body']}")
print(f"   data keys: {list(payload['data'].keys())[:5]}...")
print()

print("=" * 60)
print("✅ SDK structure is correct!")
print()
print("Note: Full cryptography test requires installing dependencies:")
print("  pip install cryptography")
print("=" * 60)
