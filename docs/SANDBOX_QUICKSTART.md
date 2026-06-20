# HSIP Sandbox — Test in 60 Seconds

The HSIP sandbox lets you evaluate the full API without installing anything.
Each request to `/v1/sandbox/provision` gives you an isolated tenant and a
24-hour trial API key.

---

## Step 1 — Get a trial key

```bash
curl -X POST https://demo.hsip.io/v1/sandbox/provision
```

Response:
```json
{
  "api_key": "hsip_a3f9...",
  "expires_at": "2026-06-21T14:32:00Z",
  "expires_at_ms": 1750516320000,
  "base_url": "https://demo.hsip.io",
  "note": "Trial key expires in 24 hours. Production licensing: sanchezleal1989@gmail.com",
  "quickstart": {
    "step1_sign_message": "curl -X POST https://demo.hsip.io/v1/messages/sign ...",
    "step2_get_identity": "...",
    "step3_view_audit_trail": "...",
    "step4_grant_consent": "...",
    "step5_agent_capabilities": "..."
  }
}
```

Copy your `api_key` and set it:
```bash
export KEY="hsip_a3f9..."
export BASE="https://demo.hsip.io"
```

---

## Step 2 — Sign a message (Ed25519, tamper-proof)

```bash
curl -X POST $BASE/v1/messages/sign \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{"content": "I authorize this transaction."}'
```

Returns an Ed25519 signature + timestamp. Anyone holding your public key
can verify this message was signed by you at exactly that time.

---

## Step 3 — Get your cryptographic identity

```bash
curl -X POST $BASE/v1/identity \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{}'
```

Returns your Ed25519 keypair (public verify key shown, private key stays
encrypted on the server with ChaCha20-Poly1305 + HKDF-SHA-256).

---

## Step 4 — View your audit trail

```bash
curl "$BASE/v1/audit" \
  -H "Authorization: Bearer $KEY"
```

Every action you've taken is in this BLAKE3 hash-chained log. Tamper with
any entry and the chain breaks — detectable by any verifier.

---

## Step 5 — Grant time-bounded consent (PSD2 / Open Banking)

```bash
curl -X POST $BASE/v1/consent/grant \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "peer_verify_key": "<counterparty_ed25519_pubkey>",
    "scope": "payment_initiation",
    "expires_in_seconds": 3600
  }'
```

---

## Step 6 — Get the AI agent capabilities spec

```bash
curl "$BASE/v1/agent/capabilities" \
  -H "Authorization: Bearer $KEY"
```

Paste this output into any AI system prompt. The AI now knows every HSIP
capability and can sign messages, check consent, and log actions under your
authorization.

---

## Sandbox limits

| Limit | Value |
|---|---|
| Trial key lifetime | 24 hours |
| Provisions per IP per hour | 5 |
| Requests per key per minute | 300 |
| Max audit entries returned | 500 |

---

## Ready to deploy for real?

HSIP is a single binary. No cloud required.

```bash
# macOS / Linux
curl -sSf https://raw.githubusercontent.com/rewired89/HSIP-1PHASE/main/install.sh | sh

# Or build from source
git clone https://github.com/rewired89/HSIP-1PHASE
cd HSIP-1PHASE
cargo build --release -p hsip-api --features hsip-api/embed-dashboard
./target/release/hsip-api
```

**Production and commercial licensing:** [sanchezleal1989@gmail.com](mailto:sanchezleal1989@gmail.com)
