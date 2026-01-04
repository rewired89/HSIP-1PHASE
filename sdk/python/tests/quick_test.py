#!/usr/bin/env python3
"""
Quick test of HSIP Bank SDK without installation
Run this to verify the SDK works on your system
"""

import sys
import os

# Add SDK to path so we can test without installing
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

print("=" * 60)
print("HSIP Bank SDK - Quick Test (No Install Required)")
print("=" * 60)
print()

# Test 1: Import the types module
try:
    from hsip_bank import types
    print("✓ Step 1: Types module imported")
    print(f"  Available message types: {[t.value for t in types.MessageType]}")
    print()
except Exception as e:
    print(f"✗ Step 1 failed: {e}")
    sys.exit(1)

# Test 2: Create a sample message (without crypto)
try:
    sample = types.SignedMessage(
        message_id="test_123",
        sender_domain="testbank.com",
        recipient="customer@example.com",
        message_type=types.MessageType.TRANSACTION_ALERT.value,
        message_text="Transaction: $50.00 at Starbucks",
        sent_at="2025-01-04T12:00:00Z",
        signature="a" * 128,  # Fake signature for testing
        public_key="b" * 64,  # Fake public key for testing
    )
    print("✓ Step 2: SignedMessage created")
    print(f"  Message: {sample.message_text}")
    print(f"  Type: {sample.message_type}")
    print()
except Exception as e:
    print(f"✗ Step 2 failed: {e}")
    sys.exit(1)

# Test 3: Convert to dictionary
try:
    msg_dict = sample.to_dict()
    print("✓ Step 3: Converted to dictionary")
    print(f"  Keys: {list(msg_dict.keys())}")
    print()
except Exception as e:
    print(f"✗ Step 3 failed: {e}")
    sys.exit(1)

# Test 4: Create notification payload
try:
    payload = sample.to_notification_payload()
    print("✓ Step 4: Created notification payload")
    print(f"  Title: {payload['title']}")
    print(f"  Body: {payload['body']}")
    print()
except Exception as e:
    print(f"✗ Step 4 failed: {e}")
    sys.exit(1)

# Test 5: Check if cryptography can be imported (needed for full functionality)
print("Step 5: Checking cryptography library...")
try:
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
    print("✓ Cryptography library is available")
    print("  Full SDK functionality ready!")
    print()

    # Test key generation
    print("  Testing key generation...")
    from hsip_bank.keygen import generate_keypair
    private_key, public_key = generate_keypair()
    print("  ✓ Ed25519 keypair generated successfully")
    print()

except ImportError as e:
    print("⚠ Cryptography library not found or has issues")
    print(f"  Error: {e}")
    print()
    print("  To install: pip install cryptography")
    print("  SDK structure works, but crypto features unavailable")
    print()

print("=" * 60)
print("✓ Basic SDK structure is working!")
print()
print("Next steps:")
print("  1. Install cryptography: pip install cryptography")
print("  2. Run full example: python examples/simple_example.py")
print("=" * 60)
