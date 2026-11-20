"""HSIP tracing processor for the OpenAI Agents SDK (Python). No hard dependency on it.

    from agents import add_trace_processor
    from hsip_prove import Prove
    from hsip_openai_agents import HsipTracingProcessor

    add_trace_processor(HsipTracingProcessor(Prove(url, key), stream="agent-actions",
                                             actor="support-agent"))

Tool calls ("function" spans) and handoffs are fingerprinted locally: only the SHA-256 plus
the span id is sent. Pass record_generations=True to include model calls. Recording runs on a
background thread and never raises into the agent. For LangGraph, use hsip_langchain.
"""
import logging
import queue
import threading
from typing import Any, Dict, Optional

from hsip_prove import Prove, hash_payload

log = logging.getLogger("hsip_openai_agents")


def _data(span: Any) -> Dict[str, Any]:
    sd = span.span_data
    try:
        d = sd.export()
    except Exception:
        d = dict(vars(sd))
    return {"span_id": str(span.span_id), "trace_id": str(getattr(span, "trace_id", "")),
            "started_at": str(getattr(span, "started_at", "")), "ended_at": str(getattr(span, "ended_at", "")),
            "data": {str(k): str(v) for k, v in d.items()}}


class HsipTracingProcessor:
    def __init__(self, prove: Prove, *, stream: Optional[str] = None, actor: str = "openai-agents",
                 record_generations: bool = False, max_queue: int = 1000):
        self._prove, self._stream, self._actor = prove, stream, actor
        self._kinds = {"function", "handoff"} | ({"generation"} if record_generations else set())
        self._q: "queue.Queue" = queue.Queue(maxsize=max_queue)
        self._t = threading.Thread(target=self._run, name="hsip-agents", daemon=True)
        self._t.start()

    def on_trace_start(self, trace): pass
    def on_trace_end(self, trace): pass
    def on_span_start(self, span): pass

    def on_span_end(self, span):
        try:
            kind = getattr(span.span_data, "type", "")
            if kind not in self._kinds:
                return
            c = _data(span)
            name = c["data"].get("name") or kind
            self._q.put_nowait((c, f"{kind}.{name}"[:200]))
        except queue.Full:
            log.warning("hsip: queue full, span not recorded")
        except Exception:
            log.exception("hsip: could not prepare span")

    def force_flush(self):
        self._q.join()

    def shutdown(self):
        self._q.put(None)
        self._t.join(timeout=10)

    def _run(self):
        while True:
            item = self._q.get()
            try:
                if item is None:
                    return
                c, action = item
                self._prove.record(c, actor=self._actor, action=action, stream=self._stream,
                                   subject=c["span_id"], payload_hash=hash_payload(c),
                                   metadata={"span_id": c["span_id"], "trace_id": c["trace_id"]})
            except Exception:
                log.exception("hsip: record failed")
            finally:
                self._q.task_done()
