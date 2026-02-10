# HSIP Python SDK

pip install hsip-sdk

## Quick Start

from hsip import HSIPClient

client = HSIPClient(api_key="hsip_...", base_url="http://localhost:3000")

# Create identity
identity = client.get_or_create_identity()
print(f"My verify key: {identity['verify_key']}")

# Grant consent to a peer
client.grant_consent(peer_verify_key="peer_key_here", ttl_ms=3_600_000)

# Sign a message (cryptographic proof of authorship)
signed = client.sign_message("Contract accepted by Alice on 2026-01-20")
print(f"Signature: {signed['signature']}")

# Verify a message from a peer
result = client.verify_message(
    content="Contract accepted by Alice on 2026-01-20",
    signature=signed['signature'],
    peer_verify_key=identity['verify_key'],
)
print(f"Verified: {result['verified']}")

# Get audit log (court-admissible)
log = client.get_audit_log(limit=100)
