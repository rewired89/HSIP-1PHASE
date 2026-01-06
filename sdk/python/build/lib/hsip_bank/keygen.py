"""
Ed25519 key generation for HSIP bank identities
"""

import os
import json
from pathlib import Path
from typing import Tuple
from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey,
    Ed25519PublicKey,
)
from cryptography.hazmat.primitives import serialization


def generate_keypair() -> Tuple[Ed25519PrivateKey, Ed25519PublicKey]:
    """
    Generate a new Ed25519 keypair for the bank

    Returns:
        (private_key, public_key)
    """
    private_key = Ed25519PrivateKey.generate()
    public_key = private_key.public_key()
    return private_key, public_key


def save_keypair(
    private_key: Ed25519PrivateKey,
    public_key: Ed25519PublicKey,
    domain: str,
    output_dir: str = ".",
) -> Tuple[str, str]:
    """
    Save keypair to files

    Args:
        private_key: The private key
        public_key: The public key
        domain: Bank domain (e.g., "chase.com")
        output_dir: Directory to save keys

    Returns:
        (private_key_path, public_key_path)
    """
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    # Save private key (PEM format, no encryption for simplicity)
    # In production, you'd encrypt this with a password
    private_pem = private_key.private_bytes(
        encoding=serialization.Encoding.PEM,
        format=serialization.PrivateFormat.PKCS8,
        encryption_algorithm=serialization.NoEncryption(),
    )

    private_key_path = output_path / f"{domain}.private.pem"
    private_key_path.write_bytes(private_pem)

    # Set restrictive permissions (owner read/write only)
    os.chmod(private_key_path, 0o600)

    # Save public key (PEM format)
    public_pem = public_key.public_bytes(
        encoding=serialization.Encoding.PEM,
        format=serialization.PublicFormat.SubjectPublicKeyInfo,
    )

    public_key_path = output_path / f"{domain}.public.pem"
    public_key_path.write_bytes(public_pem)

    # Also save as hex for easy distribution
    public_hex = public_key.public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw,
    ).hex()

    metadata = {
        "domain": domain,
        "public_key_hex": public_hex,
        "algorithm": "Ed25519",
    }

    metadata_path = output_path / f"{domain}.metadata.json"
    metadata_path.write_text(json.dumps(metadata, indent=2))

    print(f"✅ Keys generated for {domain}")
    print(f"   Private key: {private_key_path} (KEEP SECURE!)")
    print(f"   Public key: {public_key_path}")
    print(f"   Metadata: {metadata_path}")
    print(f"\n   Public key (hex): {public_hex}")
    print(f"\n⚠️  IMPORTANT: Store the private key in a secure location!")
    print(f"   Consider using a Hardware Security Module (HSM) in production.")

    return str(private_key_path), str(public_key_path)


def load_keypair(private_key_path: str) -> Tuple[Ed25519PrivateKey, Ed25519PublicKey]:
    """
    Load keypair from private key file

    Args:
        private_key_path: Path to private key PEM file

    Returns:
        (private_key, public_key)
    """
    private_pem = Path(private_key_path).read_bytes()

    private_key = serialization.load_pem_private_key(
        private_pem,
        password=None,  # Add password support in production
    )

    if not isinstance(private_key, Ed25519PrivateKey):
        raise ValueError("Key file is not an Ed25519 private key")

    public_key = private_key.public_key()
    return private_key, public_key
