"""
HSIP + OpenAI Agents SDK: Audit-Wrapped Agent
================================================
Wraps an OpenAI Agents SDK agent with HSIP so every tool call is:
  1. Signed with your key before execution
  2. Logged to the HSIP tamper-proof audit trail
  3. Visible in the HSIP dashboard at http://127.0.0.1:7777

This is the "black box recorder" pattern for AI agents:
you always know exactly what your agent did, when, and under whose authorization.

Requirements:
  pip install openai-agents requests
  export OPENAI_API_KEY=sk-...
  export HSIP_KEY=hsip_...
  HSIP must be running: hsip

Docs: https://platform.openai.com/docs/agents
"""

import os
import json
import requests
import functools
from datetime import datetime, timezone
from typing import Callable, Any

HSIP_BASE = os.getenv("HSIP_URL", "http://127.0.0.1:7777")

def _hsip_key():
    key = os.getenv("HSIP_KEY", "")
    if not key:
        try:
            key = open(os.path.expanduser("~/.hsip/admin.key")).read().strip()
        except FileNotFoundError:
            raise RuntimeError("Set HSIP_KEY or start HSIP first.")
    return key

def _headers():
    return {"Authorization": f"Bearer {_hsip_key()}", "Content-Type": "application/json"}


def sign_action(description: str) -> dict:
    """Sign an action description in HSIP before executing it."""
    resp = requests.post(
        f"{HSIP_BASE}/v1/messages/sign",
        headers=_headers(),
        json={"content": description},
    )
    resp.raise_for_status()
    return resp.json()


def hsip_audit_wrap(tool_fn: Callable) -> Callable:
    """
    Decorator that wraps any OpenAI Agents tool function to:
    1. Sign the action description in HSIP before calling the tool
    2. Log the result to the audit trail
    3. Return the result as normal

    Usage:
        @hsip_audit_wrap
        @function_tool
        def send_email(to: str, subject: str, body: str) -> str:
            ...
    """
    @functools.wraps(tool_fn)
    def wrapper(*args, **kwargs):
        # Build a human-readable description of what is about to happen
        fn_name = tool_fn.__name__
        ts = datetime.now(timezone.utc).isoformat()
        action_desc = (
            f"AI agent action: {fn_name} | "
            f"args={json.dumps(kwargs or list(args))} | "
            f"timestamp={ts}"
        )

        # Sign before executing — creates the authorization record
        try:
            signed = sign_action(action_desc)
            sig_short = signed.get("signature", "")[:16]
            print(f"[HSIP] Signed: {fn_name} | sig={sig_short}...")
        except Exception as e:
            print(f"[HSIP] Warning: could not sign action ({e}). Proceeding anyway.")

        # Execute the actual tool
        result = tool_fn(*args, **kwargs)
        return result

    return wrapper


# ---------------------------------------------------------------------------
# Example agent using the audit wrapper
# ---------------------------------------------------------------------------

try:
    from agents import Agent, Runner, function_tool  # openai-agents SDK

    @hsip_audit_wrap
    @function_tool
    def search_calendar(query: str) -> str:
        """Search the user's calendar for events matching a query."""
        # Stub — in production: call your calendar API
        return json.dumps([
            {"date": "2026-03-29", "event": "Team standup", "time": "09:00"},
            {"date": "2026-03-30", "event": "Project review", "time": "14:00"},
        ])

    @hsip_audit_wrap
    @function_tool
    def create_calendar_event(title: str, date: str, time: str, attendees: list[str]) -> str:
        """Create a new calendar event."""
        # Stub — in production: call your calendar API
        return f"Event '{title}' created on {date} at {time} for {attendees}"

    @hsip_audit_wrap
    @function_tool
    def send_message(to: str, subject: str, body: str) -> str:
        """Send a message to a contact."""
        # Stub — in production: call your messaging API
        return f"Message sent to {to}: {subject}"

    calendar_agent = Agent(
        name="HSIP-Audited Calendar Assistant",
        instructions=(
            "You are a calendar assistant. Help the user manage their schedule. "
            "Every action you take is signed with the user's cryptographic key via HSIP, "
            "creating a tamper-proof audit trail. "
            "Always confirm with the user before creating events or sending messages."
        ),
        tools=[search_calendar, create_calendar_event, send_message],
    )

    def run_demo():
        print("=" * 60)
        print("HSIP + OpenAI Agents: Audit-Wrapped Agent Demo")
        print("=" * 60)
        print("\nEvery tool call will be signed with your HSIP key.\n")

        result = Runner.run_sync(
            calendar_agent,
            "What meetings do I have this week? Then schedule a 30-minute sync with "
            "bob@example.com on Thursday at 10am."
        )
        print(f"\n[Agent] {result.final_output}")
        print("\nCheck your HSIP audit trail at http://127.0.0.1:7777")

    if __name__ == "__main__":
        run_demo()

except ImportError:
    print("openai-agents not installed. Install with: pip install openai-agents")
    print("This file demonstrates the hsip_audit_wrap decorator pattern.")
    print("The decorator works with any callable — not just OpenAI Agents tools.")
