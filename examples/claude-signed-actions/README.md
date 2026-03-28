# Example: Claude + HSIP — Signed Actions Agent

An agent powered by Claude (via the Anthropic API) that signs every important action with HSIP before taking it — creating a court-admissible audit trail of AI behavior.

## Why

Claude takes actions in your name. HSIP ensures every one of those actions is:
- Signed with your Ed25519 key
- Timestamped precisely
- Logged in a tamper-proof audit trail you own completely

If anyone disputes what the agent did — or claims it did something it didn't — you have cryptographic proof.

## Install

```bash
pip install anthropic requests
export ANTHROPIC_API_KEY=sk-ant-...
# Start HSIP first: hsip
```

## Run

```bash
python claude_hsip_agent.py
```

## What it demonstrates

1. **Tool use** — Claude calls HSIP tools (sign_message, grant_consent, get_audit_trail)
2. **Signed commitments** — every agreement or confirmation is cryptographically signed before Claude states it
3. **Minimal consent scope** — consent grants use the narrowest scope and shortest expiry
4. **Audit trail retrieval** — Claude confirms actions were logged after completing them

## Extending

Add your own tools to `HSIP_TOOLS` and corresponding logic to `execute_tool()`. Common additions:
- `send_email` — send an email after signing the content
- `create_calendar_event` — book an appointment after signing the confirmation
- `approve_document` — sign a document approval before filing it
