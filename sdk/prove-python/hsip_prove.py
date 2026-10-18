"""HSIP client. Standard library only.

    from hsip_prove import Prove
    prove = Prove("https://prove.example.com", api_key)          # an `ingest` key
    proof = prove.record(
        {"application": "48211", "decision": "approve"},          # hashed locally,
        stream="loan-decisions", actor="underwriting-model-v3",   # never sent
        action="loan.decision", subject="application-48211",
    )
    print(proof["proof_id"])

The record's content never leaves your process: only its SHA-256 is sent.

Customer-held keys: if your organization registered its own signing key
(Settings > Key custody), pass its private key and records are signed here,
so the platform never holds it. Needs `pip install cryptography`.

    prove = Prove(url, api_key, signing_key_b64=open("hsip-key.json").read())
"""

import base64
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


def _jcs(value: Any) -> str:
    """RFC 8785 canonical JSON for the string-only envelope."""
    if isinstance(value, dict):
        keys = sorted(value, key=lambda k: k.encode("utf-16-be"))
        return "{" + ",".join(json.dumps(k, ensure_ascii=False) + ":" + _jcs(value[k]) for k in keys) + "}"
    return json.dumps(value, ensure_ascii=False)


def event_hash(envelope: Dict[str, Any]) -> str:
    return hashlib.sha256(_jcs(envelope).encode()).hexdigest()


def _load_signing_key(material: str):
    try:
        from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
    except ImportError as e:  # pragma: no cover
        raise ImportError("customer-held keys need `pip install cryptography`") from e
    material = material.strip()
    if material.startswith("{"):  # the JSON written by `hsip-proof keygen`
        material = json.loads(material)["private_key_b64"]
    return Ed25519PrivateKey.from_private_bytes(base64.b64decode(material))


class Prove:
    def __init__(self, base_url: str, api_key: Optional[str] = None, timeout: float = 15.0,
                 signing_key_b64: Optional[str] = None):
        self.base_url = base_url.rstrip("/")
        self.api_key = api_key
        self.timeout = timeout
        self._signer = _load_signing_key(signing_key_b64) if signing_key_b64 else None

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
        item = self._item(record, actor, action, stream, subject, metadata, payload_hash)
        if self._signer is None:
            return self._request("POST", "/v1/ingest", item)
        # Customer-held key: ask for the envelope, check it, sign it here, commit.
        prep = self._request("POST", "/v1/ingest/prepare", item)
        env = prep["envelope"]
        if event_hash(env) != prep["event_hash"].lower():
            raise ProveError(0, "tampered", "the platform's event_hash does not match its envelope")
        for k in ("actor", "action", "subject", "payload_hash"):
            if env[k] != item[k]:
                raise ProveError(0, "tampered", f"the platform changed `{k}`")
        if env["metadata"] != item["metadata"]:
            raise ProveError(0, "tampered", "the platform changed `metadata`")
        sig = base64.b64encode(self._signer.sign(bytes.fromhex(prep["event_hash"]))).decode()
        body: Dict[str, Any] = {"envelope": env, "signature": sig}
        if stream:
            body["stream"] = stream
        for attempt in range(3):  # another writer may have moved the chain
            try:
                return self._request("POST", "/v1/ingest/commit", body)
            except ProveError as e:
                if e.status != 409 or attempt == 2:
                    raise
                return self.record(record, actor=actor, action=action, stream=stream, subject=subject,
                                   metadata=metadata, payload_hash=payload_hash)

    def record_batch(self, items: Iterable[Dict[str, Any]], stream: Optional[str] = None) -> List[Dict[str, Any]]:
        """Record up to 100 items in order. Each item takes the same keyword
        fields as `record` (`record`/`payload_hash`, `actor`, `action`, ...).
        With a customer-held key each item is signed and committed in turn."""
        if self._signer is not None:
            out = []
            for i in items:
                try:
                    r = self.record(i.get("record"), actor=i["actor"], action=i["action"], stream=stream,
                                    subject=i.get("subject", ""), metadata=i.get("metadata"),
                                    payload_hash=i.get("payload_hash"))
                    out.append({"ok": True, "proof_id": r["proof_id"], "seq": r["seq"], "event_hash": r["event_hash"]})
                except ProveError as e:
                    out.append({"ok": False, "error": e.message})
                    break
            return out
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
