"""HSIP span processor for OpenTelemetry (Python). No hard dependency on OpenTelemetry.

    from opentelemetry.sdk.trace import TracerProvider
    from hsip_prove import Prove
    from hsip_otel import HsipSpanProcessor

    provider = TracerProvider()
    provider.add_span_processor(HsipSpanProcessor(
        Prove("https://hsip.example.com", api_key), stream="ai-decisions",
        should_record=lambda s: s.attributes.get("hsip.record") is True))

Each selected span is fingerprinted locally (SHA-256 of its canonical JSON) and only the
hash plus small metadata (trace id, span id, span name) is sent. Recording happens on a
background thread, so it never slows or breaks the instrumented code.
"""
import logging
import queue
import threading
from typing import Any, Callable, Dict, Optional

from hsip_prove import Prove, hash_payload

log = logging.getLogger("hsip_otel")


def span_content(span: Any) -> Dict[str, Any]:
    """The record that gets hashed. Keep the original spans yourself to re-derive the hash."""
    ctx = span.get_span_context() if hasattr(span, "get_span_context") else span.context
    status = getattr(span, "status", None)
    return {
        "name": span.name,
        "trace_id": format(ctx.trace_id, "032x"),
        "span_id": format(ctx.span_id, "016x"),
        "start_ns": span.start_time,
        "end_ns": span.end_time,
        "status": str(getattr(getattr(status, "status_code", None), "name", "UNSET")),
        "attributes": {str(k): str(v) for k, v in dict(span.attributes or {}).items()},
    }


class HsipSpanProcessor:
    def __init__(self, prove: Prove, *, stream: Optional[str] = None, actor: str = "otel",
                 action_attribute: str = "hsip.action",
                 should_record: Optional[Callable[[Any], bool]] = None, max_queue: int = 1000):
        self._prove, self._stream, self._actor, self._action_attr = prove, stream, actor, action_attribute
        self._should = should_record or (lambda s: bool(s.attributes and s.attributes.get("hsip.record")))
        self._q: "queue.Queue" = queue.Queue(maxsize=max_queue)
        self._stop = object()
        self._t = threading.Thread(target=self._run, name="hsip-otel", daemon=True)
        self._t.start()

    # SpanProcessor interface
    def on_start(self, span, parent_context=None):
        pass

    def on_end(self, span):
        try:
            if not self._should(span):
                return
            self._q.put_nowait(span_content(span))
        except queue.Full:
            log.warning("hsip: queue full, span not recorded")
        except Exception:
            log.exception("hsip: could not prepare span")

    def shutdown(self):
        self._q.put(self._stop)
        self._t.join(timeout=10)

    def force_flush(self, timeout_millis: int = 30000) -> bool:
        self._q.join()
        return True

    def _run(self):
        while True:
            item = self._q.get()
            try:
                if item is self._stop:
                    return
                attrs = item["attributes"]
                self._prove.record(
                    item, actor=attrs.get("hsip.actor", self._actor),
                    action=attrs.get(self._action_attr, item["name"]), stream=self._stream,
                    subject=attrs.get("hsip.subject", ""),
                    metadata={"trace_id": item["trace_id"], "span_id": item["span_id"]},
                    payload_hash=hash_payload(item))
            except Exception:
                log.exception("hsip: record failed")
            finally:
                self._q.task_done()
