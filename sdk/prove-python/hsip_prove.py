"""HSIP Prove client. Standard library only.

    from hsip_prove import Prove
    prove = Prove("https://prove.example.com", api_key)          # an `ingest` key
    proof = prove.record(
        {"application": "48211", "decision": "approve"},          # hashed locally,
        stream="loan-decisions", actor="underwriting-model-v3",   # never sent
        action="loan.decision", subject="application-48211",
    )
    print(proof["proof_id"])

The record's content never leaves your process: only its SHA-256 is sent.
"""

import hashlib
import json
import urllib.error
import urllib.request
from typing import Any, Dict, Iterable, List, Optional

__all__ = ["Prove", "ProveError", "hash_payload"]


class ProveError(Exception):
    def __init__(self, status: int, code: str, message: str):
        super().__init__(f"{status} {code}: {message}")
        self.status, self.code, self.message = status, code, message


def hash_payload(record: Any) -> str:
    """SHA-256 hex of a record. Bytes/str are hashed as-is; anything else is
    serialized as canonical JSON (sorted keys, no whitespace) first, so the
    same dict always yields the same hash. Keep the original so you can prove
    it later: hashing it again must reproduce this value exactly."""
    if isinstance(record, bytes):
        data = record
    elif isinstance(record, str):
        data = record.encode()
    else:
        data = json.dumps(record, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
    return hashlib.sha256(data).hexdigest()


class Prove:
    def __init__(self, base_url: str, api_key: Optional[str] = None, timeout: float = 15.0):
        self.base_url = base_url.rstrip("/")
        self.api_key = api_key
        self.timeout = timeout

    def _request(self, method: str, path: str, body: Any = None, auth: bool = True) -> Any:
        headers = {"Content-Type": "application/json", "User-Agent": "hsip-prove-python/0.1"}
        if auth and self.api_key:
            headers["Authorization"] = f"Bearer {self.api_key}"
        data = json.dumps(body).encode() if body is not None else None
        req = urllib.request.Request(self.base_url + path, data=data, method=method, headers=headers)
        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                return json.loads(resp.read() or b"null")
        except urllib.error.HTTPError as e:
            try:
                err = json.loads(e.read()).get("error", {})
            except Exception:
                err = {}
            raise ProveError(e.code, err.get("code", "error"), err.get("message", e.reason)) from None

    def record(
        self,
        record: Any = None,
        *,
        actor: str,
        action: str,
        stream: Optional[str] = None,
        subject: str = "",
        metadata: Optional[Dict[str, str]] = None,
        payload_hash: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Record one proof. Pass `record` (hashed here) or a precomputed `payload_hash`."""
        return self._request("POST", "/v1/ingest", self._item(record, actor, action, stream, subject, metadata, payload_hash))

    def record_batch(self, items: Iterable[Dict[str, Any]], stream: Optional[str] = None) -> List[Dict[str, Any]]:
        """Record up to 100 items in order. Each item takes the same keyword
        fields as `record` (`record`/`payload_hash`, `actor`, `action`, ...)."""
        payload = [
            {k: v for k, v in self._item(i.get("record"), i["actor"], i["action"], None, i.get("subject", ""), i.get("metadata"), i.get("payload_hash")).items() if k != "stream"}
            for i in items
        ]
        body: Dict[str, Any] = {"items": payload}
        if stream:
            body["stream"] = stream
        return self._request("POST", "/v1/ingest/batch", body)["results"]

    @staticmethod
    def _item(record, actor, action, stream, subject, metadata, payload_hash) -> Dict[str, Any]:
        if payload_hash is None:
            if record is None:
                raise ValueError("pass `record` or `payload_hash`")
            payload_hash = hash_payload(record)
        item: Dict[str, Any] = {"actor": actor, "action": action, "subject": subject, "payload_hash": payload_hash, "metadata": metadata or {}}
        if stream:
            item["stream"] = stream
        return item

    def get_proof(self, proof_id: str) -> Dict[str, Any]:
        """Fetch a proof bundle with a `read` key. Returns {"bundle", "verification"}."""
        return self._request("GET", f"/v1/proofs/{proof_id}")

    def verify(self, bundle_or_pack: Dict[str, Any]) -> Dict[str, Any]:
        """Server-side verification. For independent assurance run the open
        `hsip-proof verify` tool offline instead; this endpoint is a pure
        function of the body but you still reach it through the vendor."""
        return self._request("POST", "/v1/verify", bundle_or_pack, auth=False)
