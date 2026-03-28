"""
HSIP LangChain Tools
====================
Drop-in LangChain tools that give any LangChain agent access to HSIP:

  HSIPSignTool      — sign a message with the user's Ed25519 key
  HSIPConsentTool   — grant/revoke consent for a peer or scope
  HSIPAuditTool     — read the tamper-proof audit trail
  HSIPBlockerTool   — check if a domain is blocked by HSIP DNS

Usage:
  from langchain_hsip_tools import HSIPSignTool, HSIPConsentTool, HSIPAuditTool

  tools = [HSIPSignTool(), HSIPConsentTool(), HSIPAuditTool()]
  # Then pass tools to any LangChain agent

Requirements:
  pip install langchain pydantic requests
  HSIP running at http://127.0.0.1:7777
"""

import os
import requests
from typing import Optional, Type
from pydantic import BaseModel, Field
from langchain.tools import BaseTool

HSIP_BASE = os.getenv("HSIP_URL", "http://127.0.0.1:7777")
HSIP_KEY  = os.getenv("HSIP_KEY", "")


def _headers():
    key = HSIP_KEY or os.getenv("HSIP_KEY", "")
    if not key:
        try:
            key = open(os.path.expanduser("~/.hsip/admin.key")).read().strip()
        except FileNotFoundError:
            raise RuntimeError(
                "HSIP API key not found. Set HSIP_KEY env var or run HSIP first."
            )
    return {"Authorization": f"Bearer {key}", "Content-Type": "application/json"}


# ---------------------------------------------------------------------------
# Tool: Sign a message
# ---------------------------------------------------------------------------

class SignMessageInput(BaseModel):
    content: str = Field(description="The exact message text to sign and timestamp.")


class HSIPSignTool(BaseTool):
    name: str = "hsip_sign_message"
    description: str = (
        "Sign a message with the user's cryptographic key, creating a tamper-proof "
        "timestamped proof of authorship. Use this before sending important messages, "
        "confirming agreements, or recording any statement that may need to be proven later. "
        "Returns the signature and timestamp."
    )
    args_schema: Type[BaseModel] = SignMessageInput

    def _run(self, content: str) -> str:
        resp = requests.post(
            f"{HSIP_BASE}/v1/messages/sign",
            headers=_headers(),
            json={"content": content},
        )
        if not resp.ok:
            return f"Error signing message: {resp.status_code} {resp.text}"
        data = resp.json()
        return (
            f"Message signed successfully.\n"
            f"Signature: {data.get('signature', 'N/A')}\n"
            f"Timestamp: {data.get('timestamp', 'N/A')}\n"
            f"Message ID: {data.get('id', 'N/A')}"
        )

    async def _arun(self, content: str) -> str:
        return self._run(content)


# ---------------------------------------------------------------------------
# Tool: Grant consent
# ---------------------------------------------------------------------------

class GrantConsentInput(BaseModel):
    peer_verify_key: str = Field(description="The public key of the peer to grant consent to.")
    scope: str = Field(description="What the peer is allowed to do, e.g. 'send_message', 'read_data'.")
    expires_in_seconds: Optional[int] = Field(
        default=3600,
        description="How long the consent is valid (default: 1 hour). Set lower for sensitive actions."
    )


class HSIPConsentTool(BaseTool):
    name: str = "hsip_grant_consent"
    description: str = (
        "Grant time-bounded consent to a peer (another agent, service, or person) to perform "
        "a specific action on the user's behalf. Consent expires automatically. "
        "Always request the minimum scope needed and the shortest expiry that works. "
        "Returns the consent ID which can be used to revoke it."
    )
    args_schema: Type[BaseModel] = GrantConsentInput

    def _run(self, peer_verify_key: str, scope: str, expires_in_seconds: int = 3600) -> str:
        resp = requests.post(
            f"{HSIP_BASE}/v1/consent/grant",
            headers=_headers(),
            json={
                "peer_verify_key": peer_verify_key,
                "scope": scope,
                "expires_in_seconds": expires_in_seconds,
            },
        )
        if not resp.ok:
            return f"Error granting consent: {resp.status_code} {resp.text}"
        data = resp.json()
        return (
            f"Consent granted.\n"
            f"Consent ID: {data.get('id', 'N/A')}\n"
            f"Scope: {scope}\n"
            f"Expires in: {expires_in_seconds} seconds\n"
            f"Peer: {peer_verify_key[:16]}..."
        )

    async def _arun(self, peer_verify_key: str, scope: str, expires_in_seconds: int = 3600) -> str:
        return self._run(peer_verify_key, scope, expires_in_seconds)


# ---------------------------------------------------------------------------
# Tool: Revoke consent
# ---------------------------------------------------------------------------

class RevokeConsentInput(BaseModel):
    consent_id: str = Field(description="The consent ID to revoke.")


class HSIPRevokeConsentTool(BaseTool):
    name: str = "hsip_revoke_consent"
    description: str = (
        "Immediately revoke a previously granted consent. "
        "Call this after an action is complete to minimize the window of access. "
        "Best practice: revoke consent as soon as the authorized action is done."
    )
    args_schema: Type[BaseModel] = RevokeConsentInput

    def _run(self, consent_id: str) -> str:
        resp = requests.post(
            f"{HSIP_BASE}/v1/consent/revoke",
            headers=_headers(),
            json={"consent_id": consent_id},
        )
        if not resp.ok:
            return f"Error revoking consent: {resp.status_code} {resp.text}"
        return f"Consent {consent_id} revoked successfully."

    async def _arun(self, consent_id: str) -> str:
        return self._run(consent_id)


# ---------------------------------------------------------------------------
# Tool: Read audit trail
# ---------------------------------------------------------------------------

class AuditInput(BaseModel):
    limit: Optional[int] = Field(default=10, description="Number of recent entries to fetch.")


class HSIPAuditTool(BaseTool):
    name: str = "hsip_get_audit_trail"
    description: str = (
        "Retrieve the user's tamper-proof HSIP audit trail. "
        "Returns a list of recent operations: messages signed, consents granted/revoked, "
        "credentials issued, AI agent actions. Use this to review what has been done "
        "or to provide the user with a summary of recent activity."
    )
    args_schema: Type[BaseModel] = AuditInput

    def _run(self, limit: int = 10) -> str:
        resp = requests.get(
            f"{HSIP_BASE}/v1/audit",
            headers=_headers(),
            params={"limit": limit},
        )
        if not resp.ok:
            return f"Error fetching audit trail: {resp.status_code} {resp.text}"
        data = resp.json()
        entries = data if isinstance(data, list) else data.get("entries", [])
        if not entries:
            return "Audit trail is empty."
        lines = [f"Last {len(entries)} audit entries:"]
        for e in entries:
            lines.append(
                f"  [{e.get('timestamp', '?')}] {e.get('operation', '?')} — {e.get('details', '')}"
            )
        return "\n".join(lines)

    async def _arun(self, limit: int = 10) -> str:
        return self._run(limit)


# ---------------------------------------------------------------------------
# Tool: Check if domain is blocked
# ---------------------------------------------------------------------------

class DomainCheckInput(BaseModel):
    domain: str = Field(description="The domain to look up in the HSIP tracker database.")


class HSIPBlockerTool(BaseTool):
    name: str = "hsip_check_domain"
    description: str = (
        "Look up a domain in HSIP's tracker database to find out if it is a known tracker "
        "and what category it belongs to (analytics, ad network, session recording, etc.). "
        "Useful for explaining to the user what a site is doing with their data."
    )
    args_schema: Type[BaseModel] = DomainCheckInput

    def _run(self, domain: str) -> str:
        resp = requests.get(
            f"{HSIP_BASE}/v1/dns/lookup",
            headers=_headers(),
            params={"domain": domain},
        )
        if not resp.ok:
            return f"Error checking domain: {resp.status_code} {resp.text}"
        data = resp.json()
        if data.get("blocked"):
            return (
                f"{domain} is a known tracker.\n"
                f"Category: {data.get('category', 'Unknown')}\n"
                f"Vendor: {data.get('vendor', 'Unknown')}\n"
                f"Description: {data.get('description', 'N/A')}"
            )
        return f"{domain} is not in the HSIP tracker database."

    async def _arun(self, domain: str) -> str:
        return self._run(domain)


# ---------------------------------------------------------------------------
# All tools — import this list into your agent
# ---------------------------------------------------------------------------

ALL_HSIP_TOOLS = [
    HSIPSignTool(),
    HSIPConsentTool(),
    HSIPRevokeConsentTool(),
    HSIPAuditTool(),
    HSIPBlockerTool(),
]


if __name__ == "__main__":
    # Quick smoke test — requires HSIP running
    print("Testing HSIPSignTool...")
    tool = HSIPSignTool()
    result = tool._run("Test message from LangChain HSIP tool.")
    print(result)

    print("\nTesting HSIPAuditTool...")
    audit = HSIPAuditTool()
    print(audit._run(limit=3))
