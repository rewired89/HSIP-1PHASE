"""
CLI tools for HSIP Bank SDK
"""

import argparse
from .keygen import generate_keypair, save_keypair


def keygen_cli():
    """Command-line tool for generating bank keypairs"""
    parser = argparse.ArgumentParser(
        description="Generate Ed25519 keypair for HSIP bank identity"
    )
    parser.add_argument(
        "--domain",
        required=True,
        help="Bank domain (e.g., chase.com)",
    )
    parser.add_argument(
        "--output",
        default="./keys",
        help="Output directory for keys (default: ./keys)",
    )

    args = parser.parse_args()

    print(f"Generating keypair for {args.domain}...")
    private_key, public_key = generate_keypair()
    save_keypair(private_key, public_key, args.domain, args.output)


if __name__ == "__main__":
    keygen_cli()
