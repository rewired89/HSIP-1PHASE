"""
HSIP Example: AI Agent Consent Gate
====================================
Demonstrates the full HSIP consent lifecycle for an AI agent:
  1. Agent discovers what actions HSIP supports
  2. Agent requests user consent before acting
  3. Agent signs the action and logs it to the audit trail
  4. User can review the full audit trail at any time

Requirements:
  pip install hsip-sdk requests
  HSIP must be running: hsip
"""

import os
import json
import requests
from datetime import datetime

HSIP_BASE = os.getenv("HSIP_URL", "http://127.0.0.1:7777")
HSIP_KEY  = os.getenv("HSIP_KEY", open(os.path.expanduser("~/.hsip/admin.key")).read().strip())

headers = {
    "Authorization": f"Bearer {HSIP_KEY}",
    "Content-Type": "application/json",
}


# ---------------------------------------------------------------------------
# Step 1: Agent discovers HSIP capabilities
# ---------------------------------------------------------------------------

def discover_capabilities():
    """
    An AI agent calls this endpoint to understand what HSIP can do.
    The response can be injected directly into an AI system prompt.
    """
    resp = requests.get(f"{HSIP_BASE}/v1/agent/capabilities", headers=headers)
    resp.raise_for_status()
    return resp.json()


# ---------------------------------------------------------------------------
# Step 2: Agent requests a scoped, time-bounded consent grant
# ---------------------------------------------------------------------------

def request_consent(peer_verify_key: str, scope: str, expires_in_seconds: int = 600):
    """
    Before taking any action, the agent requests consent.
    - scope: what the agent is allowed to do ("send_message", "read_calendar", etc.)
    - expires_in_seconds: consent auto-revokes after this many seconds
    """
    resp = requests.post(
        f"{HSIP_BASE}/v1/consent/grant",
        headers=headers,
        json={
            "peer_verify_key": peer_verify_key,
            "scope": scope,
            "expires_in_seconds": expires_in_seconds,
        },
    )
    resp.raise_for_status()
    return resp.json()


# ---------------------------------------------------------------------------
# Step 3: Agent signs the action it is about to take
# ---------------------------------------------------------------------------

def sign_action(action_description: str):
    """
    The agent signs the exact action it is about to take.
    This creates a tamper-proof, timestamped record that cannot be repudiated.

    The signature proves:
    - These exact words were produced under your key
    - At this exact timestamp
    - Have not been altered since
    """
    resp = requests.post(
        f"{HSIP_BASE}/v1/messages/sign",
        headers=headers,
        json={"content": action_description},
    )
    resp.raise_for_status()
    return resp.json()


# ---------------------------------------------------------------------------
# Step 4: Revoke consent when action is complete (optional but recommended)
# ---------------------------------------------------------------------------

def revoke_consent(consent_id: str):
    resp = requests.post(
        f"{HSIP_BASE}/v1/consent/revoke",
        headers=headers,
        json={"consent_id": consent_id},
    )
    resp.raise_for_status()
    return resp.json()


# ---------------------------------------------------------------------------
# Step 5: Review audit trail
# ---------------------------------------------------------------------------

def get_audit_trail(limit: int = 20):
    resp = requests.get(
        f"{HSIP_BASE}/v1/audit",
        headers=headers,
        params={"limit": limit},
    )
    resp.raise_for_status()
    return resp.json()


# ---------------------------------------------------------------------------
# Demo: Email assistant that gates on consent before sending
# ---------------------------------------------------------------------------

class ConsentGatedEmailAgent:
    """
    An AI email assistant that:
    1. Drafts an email
    2. Shows the draft to the user
    3. Requests explicit consent before sending
    4. Signs the send action with the user's HSIP key
    5. Revokes consent after the action is complete
    """

    def __init__(self, agent_verify_key: str):
        self.agent_verify_key = agent_verify_key
        self.active_consent_id = None

    def draft_email(self, to: str, subject: str, body: str) -> dict:
        return {"to": to, "subject": subject, "body": body}

    def request_send_permission(self, draft: dict) -> str:
        """Ask the user (via HSIP) for consent to send this specific email."""
        print(f"\n[AGENT] Requesting consent to send email:")
        print(f"  To:      {draft['to']}")
        print(f"  Subject: {draft['subject']}")
        print(f"  Body:    {draft['body'][:100]}...")
        print(f"  Consent expires in 10 minutes.\n")

        result = request_consent(
            peer_verify_key=self.agent_verify_key,
            scope=f"send_email:to={draft['to']}",
            expires_in_seconds=600,
        )
        self.active_consent_id = result.get("id")
        print(f"[HSIP] Consent granted. ID: {self.active_consent_id}")
        return self.active_consent_id

    def send_email(self, draft: dict):
        """Simulate sending, then sign the action in HSIP."""
        # In a real agent: actually send the email here via SMTP/API
        print(f"\n[AGENT] Sending email to {draft['to']}...")

        # Sign the action — this is the tamper-proof record
        action = (
            f"Sent email: to={draft['to']}, "
            f"subject='{draft['subject']}', "
            f"timestamp={datetime.utcnow().isoformat()}Z"
        )
        signed = sign_action(action)
        print(f"[HSIP] Action signed. Signature: {signed['signature'][:32]}...")
        print(f"[HSIP] Timestamp: {signed.get('timestamp', 'N/A')}")

        # Revoke consent — action is done, no further access needed
        if self.active_consent_id:
            revoke_consent(self.active_consent_id)
            print(f"[HSIP] Consent {self.active_consent_id} revoked.")
            self.active_consent_id = None

        return signed


def main():
    print("=" * 60)
    print("HSIP — AI Agent Consent Gate Demo")
    print("=" * 60)

    # Discover capabilities (agents do this to know what HSIP supports)
    print("\n[1] Discovering HSIP capabilities...")
    caps = discover_capabilities()
    print(f"    Capabilities: {list(caps.keys()) if isinstance(caps, dict) else caps}")

    # Simulate an AI agent with its own identity key
    # In production: agent generates its own keypair and registers with HSIP
    agent_key = "agent_demo_verify_key_placeholder"

    agent = ConsentGatedEmailAgent(agent_verify_key=agent_key)

    # Agent drafts an email
    print("\n[2] Agent drafts an email...")
    draft = agent.draft_email(
        to="alice@example.com",
        subject="Project update",
        body="Hi Alice, just a quick update on the project status. Everything is on track.",
    )

    # Agent requests consent before sending
    print("\n[3] Agent requests consent...")
    agent.request_send_permission(draft)

    # Simulate user granting consent (in a real UI, user reviews and clicks "Allow")
    input("\n    [USER] Press Enter to approve sending this email... ")

    # Agent sends and signs the action
    print("\n[4] Agent sends email and signs action in HSIP...")
    signed = agent.send_email(draft)

    # Review audit trail
    print("\n[5] Audit trail (last 5 entries):")
    trail = get_audit_trail(limit=5)
    entries = trail if isinstance(trail, list) else trail.get("entries", [])
    for entry in entries[:5]:
        print(f"    [{entry.get('timestamp', '?')}] {entry.get('operation', '?')}")

    print("\n" + "=" * 60)
    print("Demo complete. Your HSIP dashboard at http://127.0.0.1:7777")
    print("shows the full audit trail with all signed entries.")
    print("=" * 60)


if __name__ == "__main__":
    main()
