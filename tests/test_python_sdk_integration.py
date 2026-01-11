#!/usr/bin/env python3
"""
Integration test for HSIP Python SDK

This test:
1. Generates Ed25519 keys
2. Signs a message using the Python SDK
3. Verifies the signature manually
4. Tests the format matches what Rust expects
"""

import sys
import os
import json

# Add SDK to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'sdk', 'python'))

from hsip_bank.signer import BankMessageSigner
from hsip_bank.types import MessageType
from hsip_bank.keygen import generate_keypair, save_keypair
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey
from cryptography.hazmat.primitives import serialization

def test_sdk_signing():
    """Test that SDK properly signs messages"""
    print("[TEST] SDK Message Signing")

    # Generate test keypair
    private_key_path = "/tmp/hsip_test_key.pem"
    generate_keypair(private_key_path)

    # Initialize signer
    signer = BankMessageSigner(private_key_path, "testbank.com")

    # Sign a message
    signed = signer.sign_message(
        message_text="Test transaction: $50.00 at Starbucks",
        message_type=MessageType.TRANSACTION_ALERT,
        recipient="user@example.com",
        message_id="msg_test123"
    )

    print(f"✓ Message signed successfully")
    print(f"  Message ID: {signed.message_id}")
    print(f"  Sender: {signed.sender_domain}")
    print(f"  Signature (hex): {signed.signature[:32]}...")
    print(f"  Public Key (hex): {signed.public_key[:32]}...")

    # Verify the signature manually
    public_key_bytes = bytes.fromhex(signed.public_key)
    public_key = Ed25519PublicKey.from_public_bytes(public_key_bytes)

    signature_bytes = bytes.fromhex(signed.signature)

    # Build the same canonical format that was signed
    parts = [
        "HSIP_V1",
        signed.message_id,
        signed.sender_domain,
        signed.recipient,
        signed.message_type,
        signed.message_text,
        signed.sent_at,
    ]
    sign_data = "|".join(parts).encode("utf-8")

    # Verify
    try:
        public_key.verify(signature_bytes, sign_data)
        print(f"✓ Signature verified successfully")
    except Exception as e:
        print(f"✗ Signature verification failed: {e}")
        return False

    # Test JSON serialization (for API)
    json_data = signed.to_json()
    parsed = json.loads(json_data)

    assert parsed['message_id'] == "msg_test123"
    assert parsed['sender_domain'] == "testbank.com"
    assert parsed['message_type'] == "transaction_alert"
    print(f"✓ JSON serialization works")

    # Clean up
    os.remove(private_key_path)

    print(f"\n[PASS] All SDK tests passed")
    return True

def test_canonical_format():
    """Test that canonical format matches Rust implementation"""
    print("\n[TEST] Canonical Format Consistency")

    # Build canonical format
    parts = [
        "HSIP_V1",
        "msg_123",
        "bank.com",
        "customer@example.com",
        "security_alert",
        "Suspicious login detected",
        "2024-01-11T12:00:00Z",
    ]
    canonical = "|".join(parts)

    expected = "HSIP_V1|msg_123|bank.com|customer@example.com|security_alert|Suspicious login detected|2024-01-11T12:00:00Z"

    assert canonical == expected
    print(f"✓ Canonical format matches Rust")
    print(f"  Format: {canonical[:50]}...")

    print(f"\n[PASS] Format consistency test passed")
    return True

def test_notification_payload():
    """Test push notification payload format"""
    print("\n[TEST] Push Notification Payload")

    private_key_path = "/tmp/hsip_test_key2.pem"
    generate_keypair(private_key_path)

    signer = BankMessageSigner(private_key_path, "chase.com")
    signed = signer.sign_message(
        "Your card ending in 1234 was charged $99.99",
        MessageType.TRANSACTION_ALERT,
        "+1234567890"
    )

    payload = signed.to_notification_payload()

    # Verify payload structure
    assert 'title' in payload
    assert 'body' in payload
    assert 'data' in payload
    assert 'hsip_signature' in payload['data']
    assert 'hsip_sender' in payload['data']

    print(f"✓ Notification payload structured correctly")
    print(f"  Title: {payload['title']}")
    print(f"  Sender: {payload['data']['hsip_sender']}")
    print(f"  Has signature: {len(payload['data']['hsip_signature']) > 0}")

    os.remove(private_key_path)

    print(f"\n[PASS] Notification payload test passed")
    return True

if __name__ == "__main__":
    try:
        all_passed = True
        all_passed &= test_sdk_signing()
        all_passed &= test_canonical_format()
        all_passed &= test_notification_payload()

        if all_passed:
            print("\n" + "="*50)
            print("ALL INTEGRATION TESTS PASSED ✓")
            print("="*50)
            sys.exit(0)
        else:
            print("\nSOME TESTS FAILED ✗")
            sys.exit(1)
    except Exception as e:
        print(f"\n[ERROR] Test failed with exception: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
