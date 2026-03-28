# Example: OpenAI Agents SDK + HSIP Audit Trail

Wraps any OpenAI Agents tool with HSIP so every tool call is signed before execution and logged to your tamper-proof audit trail.

## The Pattern

```python
@hsip_audit_wrap    # ← HSIP signs before execution
@function_tool      # ← OpenAI Agents decorator
def send_email(to: str, subject: str, body: str) -> str:
    ...
```

That's it. Every call to `send_email` is now:
1. Signed with your Ed25519 key before it runs
2. Logged to the HSIP audit trail with a precise timestamp

## Install

```bash
pip install openai-agents requests
export OPENAI_API_KEY=sk-...
# Start HSIP: hsip
```

## Run

```bash
python openai_hsip_audit.py
```

## What the audit record looks like

```json
{
  "operation": "message_sign",
  "content": "AI agent action: send_email | args={\"to\": \"alice@example.com\"} | timestamp=2026-03-28T14:32:00Z",
  "signature": "ed25519:abc123...",
  "timestamp": "2026-03-28T14:32:00Z"
}
```

The signature is tied to your public key. Anyone with your public key can verify this record independently — no HSIP required to verify.
