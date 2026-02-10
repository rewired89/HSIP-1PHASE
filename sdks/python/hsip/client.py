"""HSIP Python SDK - Cryptographic consent and message verification."""

import json
import urllib.request
import urllib.error
from typing import Optional, List, Dict, Any


class HSIPError(Exception):
    pass


class HSIPClient:
    """
    HSIP REST API client.

    Usage:
        client = HSIPClient(api_key="hsip_...", base_url="http://localhost:3000")
        identity = client.get_or_create_identity()
        client.grant_consent(peer_verify_key="...", ttl_ms=3_600_000)
        result = client.sign_message("Hello, world!")
        verified = client.verify_message(content="Hello", signature="...", peer_verify_key="...")
    """

    def __init__(self, api_key: str, base_url: str = "http://localhost:3000"):
        self.api_key  = api_key
        self.base_url = base_url.rstrip("/")

    def _request(self, method: str, path: str, body: Optional[Dict] = None) -> Any:
        url     = f"{self.base_url}{path}"
        data    = json.dumps(body).encode() if body else None
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type":  "application/json",
        }
        req = urllib.request.Request(url, data=data, headers=headers, method=method)
        try:
            with urllib.request.urlopen(req) as resp:
                return json.loads(resp.read())
        except urllib.error.HTTPError as e:
            error_body = json.loads(e.read())
            raise HSIPError(f"HTTP {e.code}: {error_body.get('error', str(e))}") from e

    # ── Identity ──────────────────────────────────────────────────────────────

    def get_or_create_identity(self) -> Dict:
        """Create or retrieve this tenant's HSIP identity (Ed25519 keypair)."""
        return self._request("POST", "/v1/identity")

    def get_identity(self) -> Dict:
        """Get current public identity."""
        return self._request("GET", "/v1/identity")

    # ── Consent ───────────────────────────────────────────────────────────────

    def grant_consent(self, peer_verify_key: str, ttl_ms: int = 3_600_000) -> Dict:
        """Grant consent to a peer. Returns consent record."""
        return self._request("POST", "/v1/consent/grant", {
            "peer_verify_key": peer_verify_key,
            "ttl_ms":          ttl_ms,
        })

    def revoke_consent(self, peer_verify_key: str) -> Dict:
        """Instantly revoke consent from a peer."""
        return self._request("POST", "/v1/consent/revoke", {
            "peer_verify_key": peer_verify_key,
        })

    def list_consents(self) -> List[Dict]:
        """List all consent records."""
        return self._request("GET", "/v1/consent")

    def get_consent(self, peer_verify_key: str) -> Dict:
        """Check consent status for a specific peer."""
        return self._request("GET", f"/v1/consent/{peer_verify_key}")

    # ── Messages ──────────────────────────────────────────────────────────────

    def sign_message(self, content: str, peer_verify_key: Optional[str] = None) -> Dict:
        """
        Sign a message with this tenant's HSIP identity.
        Returns: { id, content, signature, timestamp }
        The signature is cryptographic proof of authorship.
        """
        body: Dict[str, Any] = {"content": content}
        if peer_verify_key:
            body["peer_verify_key"] = peer_verify_key
        return self._request("POST", "/v1/messages/sign", body)

    def verify_message(self, content: str, signature: str, peer_verify_key: str) -> Dict:
        """
        Verify a message signature from a peer.
        Returns: { verified: bool, peer_verify_key, timestamp }
        """
        return self._request("POST", "/v1/messages/verify", {
            "content":         content,
            "signature":       signature,
            "peer_verify_key": peer_verify_key,
        })

    def list_messages(self) -> List[Dict]:
        """List recent messages (inbound and outbound)."""
        return self._request("GET", "/v1/messages")

    # ── Audit ─────────────────────────────────────────────────────────────────

    def get_audit_log(self, limit: int = 50, action: Optional[str] = None) -> List[Dict]:
        """
        Retrieve tamper-evident audit log.
        Suitable for compliance reporting and court submission.
        """
        params = f"?limit={limit}"
        if action:
            params += f"&action={action}"
        return self._request("GET", f"/v1/audit{params}")

    # ── API Keys ──────────────────────────────────────────────────────────────

    def create_key(self, name: str = "default") -> Dict:
        """Create a new API key. Returns key (shown once only)."""
        return self._request("POST", "/v1/keys", {"name": name})

    def list_keys(self) -> List[Dict]:
        """List all API keys (hashes only)."""
        return self._request("GET", "/v1/keys")

    def revoke_key(self, key_id: str) -> Dict:
        """Revoke an API key by ID."""
        return self._request("DELETE", f"/v1/keys/{key_id}")
