"""
HSIP + Claude API: Signed Actions Agent
=========================================
An agent powered by Claude that signs every important action with HSIP
before taking it — creating a court-admissible audit trail of AI behavior.

Scenario: A personal assistant that manages appointments, sends messages,
and confirms agreements — all with cryptographic proof tied to your key.

Requirements:
  pip install anthropic requests
  export ANTHROPIC_API_KEY=sk-ant-...
  export HSIP_KEY=hsip_...   (or run HSIP first — key is at ~/.hsip/admin.key)
  HSIP must be running: hsip

Why this matters:
  Every action this agent takes is signed with your Ed25519 key and logged in
  HSIP's tamper-proof audit trail. If anyone disputes what the agent did — or
  didn't do — you have cryptographic proof.
"""

import os
import json
import requests
import anthropic
from datetime import datetime, timezone

HSIP_BASE = os.getenv("HSIP_URL", "http://127.0.0.1:7777")

def _hsip_key():
    key = os.getenv("HSIP_KEY", "")
    if not key:
        try:
            key = open(os.path.expanduser("~/.hsip/admin.key")).read().strip()
        except FileNotFoundError:
            raise RuntimeError("HSIP not running or key not found. Start HSIP first.")
    return key

def _headers():
    return {"Authorization": f"Bearer {_hsip_key()}", "Content-Type": "application/json"}


# ---------------------------------------------------------------------------
# HSIP tool definitions for Claude
# ---------------------------------------------------------------------------

HSIP_TOOLS = [
    {
        "name": "sign_message",
        "description": (
            "Sign a message with the user's cryptographic key. Creates tamper-proof, "
            "timestamped proof that this exact text was produced under the user's key at "
            "this exact time. Use before any important statement, agreement, or commitment."
        ),
        "input_schema": {
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The exact message to sign.",
                }
            },
            "required": ["content"],
        },
    },
    {
        "name": "grant_consent",
        "description": (
            "Grant time-bounded consent for a specific action scope. "
            "Always use the minimum scope and shortest expiry needed."
        ),
        "input_schema": {
            "type": "object",
            "properties": {
                "scope": {
                    "type": "string",
                    "description": "What is being authorized (e.g. 'send_message:to=alice@example.com').",
                },
                "peer_verify_key": {
                    "type": "string",
                    "description": "Public key of the peer being granted consent.",
                },
                "expires_in_seconds": {
                    "type": "integer",
                    "description": "How long the consent lasts. Default: 300 (5 minutes).",
                    "default": 300,
                },
            },
            "required": ["scope", "peer_verify_key"],
        },
    },
    {
        "name": "get_audit_trail",
        "description": "Retrieve the recent HSIP audit trail showing all signed actions.",
        "input_schema": {
            "type": "object",
            "properties": {
                "limit": {
                    "type": "integer",
                    "description": "Number of entries to retrieve.",
                    "default": 5,
                }
            },
        },
    },
]


# ---------------------------------------------------------------------------
# Tool execution
# ---------------------------------------------------------------------------

def execute_tool(name: str, inputs: dict) -> str:
    if name == "sign_message":
        resp = requests.post(
            f"{HSIP_BASE}/v1/messages/sign",
            headers=_headers(),
            json={"content": inputs["content"]},
        )
        if not resp.ok:
            return f"Error: {resp.status_code} {resp.text}"
        d = resp.json()
        return json.dumps({
            "signed": True,
            "signature": d.get("signature", "")[:32] + "...",
            "timestamp": d.get("timestamp"),
            "message_id": d.get("id"),
        })

    elif name == "grant_consent":
        resp = requests.post(
            f"{HSIP_BASE}/v1/consent/grant",
            headers=_headers(),
            json={
                "peer_verify_key": inputs["peer_verify_key"],
                "scope": inputs["scope"],
                "expires_in_seconds": inputs.get("expires_in_seconds", 300),
            },
        )
        if not resp.ok:
            return f"Error: {resp.status_code} {resp.text}"
        d = resp.json()
        return json.dumps({"consent_id": d.get("id"), "scope": inputs["scope"]})

    elif name == "get_audit_trail":
        resp = requests.get(
            f"{HSIP_BASE}/v1/audit",
            headers=_headers(),
            params={"limit": inputs.get("limit", 5)},
        )
        if not resp.ok:
            return f"Error: {resp.status_code} {resp.text}"
        data = resp.json()
        entries = data if isinstance(data, list) else data.get("entries", [])
        return json.dumps(entries[:inputs.get("limit", 5)], indent=2)

    return f"Unknown tool: {name}"


# ---------------------------------------------------------------------------
# Agent loop
# ---------------------------------------------------------------------------

def run_agent(user_request: str, max_turns: int = 10) -> str:
    client = anthropic.Anthropic()

    system_prompt = f"""You are a personal assistant with access to HSIP, a local identity server.

HSIP gives you the ability to:
1. Sign messages with the user's cryptographic key — creating tamper-proof proof of authorship
2. Grant time-bounded consent for specific actions
3. Read the audit trail of everything that has been done

IMPORTANT RULES:
- Before making any commitment, agreement, or important statement on behalf of the user,
  ALWAYS use sign_message to create a cryptographic record.
- Always use the minimum scope and shortest expiry when granting consent.
- After completing a task, retrieve the audit trail entry to confirm it was logged.
- Never take an action on behalf of the user without signing it first.

Current time: {datetime.now(timezone.utc).isoformat()}
HSIP server: {HSIP_BASE}
"""

    messages = [{"role": "user", "content": user_request}]

    for turn in range(max_turns):
        response = client.messages.create(
            model="claude-sonnet-4-6",
            max_tokens=4096,
            system=system_prompt,
            tools=HSIP_TOOLS,
            messages=messages,
        )

        # Collect text from this response
        text_parts = [b.text for b in response.content if hasattr(b, "text")]
        if text_parts:
            print(f"\n[Claude] {' '.join(text_parts)}")

        # Done
        if response.stop_reason == "end_turn":
            return " ".join(text_parts)

        # Execute tool calls
        if response.stop_reason == "tool_use":
            messages.append({"role": "assistant", "content": response.content})

            tool_results = []
            for block in response.content:
                if block.type == "tool_use":
                    print(f"\n[Tool] {block.name}({json.dumps(block.input, indent=2)})")
                    result = execute_tool(block.name, block.input)
                    print(f"[Result] {result}")
                    tool_results.append({
                        "type": "tool_result",
                        "tool_use_id": block.id,
                        "content": result,
                    })

            messages.append({"role": "user", "content": tool_results})

    return "Agent reached max turns without completing."


# ---------------------------------------------------------------------------
# Demo scenarios
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    print("=" * 60)
    print("HSIP + Claude: Signed Actions Agent Demo")
    print("=" * 60)

    scenarios = [
        "Sign a message confirming that I reviewed and approved the Q1 budget report today.",
        "Show me the last 3 entries in my audit trail.",
        "Sign a message saying I authorize the transfer of project ownership to the new team lead.",
    ]

    for i, scenario in enumerate(scenarios, 1):
        print(f"\n{'=' * 60}")
        print(f"Scenario {i}: {scenario}")
        print("=" * 60)
        result = run_agent(scenario, max_turns=5)
        print(f"\n[Final] {result}")

    print("\n\nAll actions are now in your HSIP audit trail.")
    print("View them at: http://127.0.0.1:7777")
