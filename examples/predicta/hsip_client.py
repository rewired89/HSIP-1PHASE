"""
HSIP connector for Predicta (drop-in replacement for fetchers/hsip_client.py).

Same function and arguments as the old connector, so none of Predicta's call
sites change: attest_transaction(decision_type, strategy_id, payload,
model_version). It records one real, already-executed transaction on an HSIP
platform. Only a SHA-256 fingerprint of the payload leaves Predicta, plus a
few small labels (never symbol, price, quantity or reasoning).

Config (silently disabled unless the first two are set, and it never raises,
so HSIP can never block a trade):
  HSIP_API_URL   e.g. http://127.0.0.1:8080 or your hosted HSIP platform
  HSIP_API_KEY   an `ingest` API key (hsp_...) created in the HSIP console
  HSIP_STREAM    stream id or slug, only needed if the key is not bound to one
  HSIP_DISCLOSE_NOTIONAL=true   optional: also record a rounded dollar amount
                 (qty x price) as a label, so a permission can cap trade size.
                 Off by default because it reveals trade size to the platform.

What gets recorded: actor "predicta", action = "buy" or "sell", subject = the
strategy, metadata = model version and strategy (and notional if enabled).
"""
from __future__ import annotations

import hashlib
import json
import logging
import os
from typing import Optional

import requests

log = logging.getLogger(__name__)

ACTOR = "predicta"
_TIMEOUT_SECS = 5


def _normalize_url(url: str) -> str:
    url = url.strip().rstrip("/")
    if url and not url.startswith(("http://", "https://")):
        url = f"https://{url}"
    return url


HSIP_API_KEY = os.environ.get("HSIP_API_KEY", "")
HSIP_API_URL = _normalize_url(os.environ.get("HSIP_API_URL", ""))
HSIP_STREAM = os.environ.get("HSIP_STREAM", "").strip()
HSIP_ENABLED = bool(HSIP_API_KEY and HSIP_API_URL)
DISCLOSE_NOTIONAL = os.environ.get("HSIP_DISCLOSE_NOTIONAL", "").lower() in ("1", "true", "yes")


def hash_payload(payload: dict) -> str:
    """Hex SHA-256 of a canonical (sorted-key) JSON encoding of payload."""
    canonical = json.dumps(payload, sort_keys=True, separators=(",", ":"), default=str)
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def _notional(payload: dict) -> Optional[int]:
    try:
        qty = float(payload.get("qty"))
        price = float(payload.get("entry_price") or payload.get("exit_price"))
        return int(round(qty * price))
    except (TypeError, ValueError):
        return None


def attest_transaction(
    decision_type: str,
    strategy_id: str,
    payload: dict,
    model_version: str = "predicta-v1",
) -> Optional[dict]:
    """Record one executed transaction. Returns HSIP's receipt, or None if disabled or failed."""
    if not HSIP_ENABLED:
        return None
    try:
        metadata = {"model_version": model_version, "strategy": strategy_id}
        if DISCLOSE_NOTIONAL:
            n = _notional(payload)
            if n is not None:
                metadata["notional_usd"] = str(n)
        body = {
            "actor": ACTOR,
            "action": decision_type,
            "subject": strategy_id,
            "payload_hash": hash_payload(payload),
            "metadata": metadata,
        }
        if HSIP_STREAM:
            body["stream"] = HSIP_STREAM
        r = requests.post(
            f"{HSIP_API_URL}/v1/ingest",
            headers={"Authorization": f"Bearer {HSIP_API_KEY}", "Content-Type": "application/json"},
            json=body,
            timeout=_TIMEOUT_SECS,
        )
        r.raise_for_status()
        receipt = r.json()
        log.info(f"[HSIP] Recorded {decision_type} ({strategy_id}) proof_id={receipt.get('proof_id')}")
        return receipt
    except Exception as exc:
        log.warning(f"[HSIP] Recording failed (the transaction itself is unaffected): {exc}")
        return None
