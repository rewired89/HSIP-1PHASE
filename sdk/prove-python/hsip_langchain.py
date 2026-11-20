"""HSIP callback handler for LangChain / LangGraph. No hard dependency on LangChain.

    from hsip_prove import Prove
    from hsip_langchain import HsipCallbackHandler

    handler = HsipCallbackHandler(Prove("https://hsip.example.com", api_key),
                                  stream="agent-actions", actor="support-agent-v2")
    agent.invoke({"input": "refund order 48211"}, config={"callbacks": [handler]})

Tool calls (and, with record_llm=True, model calls) are fingerprinted locally: the SHA-256
of {tool, input, output, run_id} is sent with the run id, never the content itself. Recording
runs on a background thread, so it cannot slow or break the agent. Call handler.flush()
before exit. Keep your own copy of the content to re-derive a hash later.
"""
import logging
import queue
import threading
from typing import Any, Dict, Optional

from hsip_prove import Prove, hash_payload

try:  # use the real base class when LangChain is installed
    from langchain_core.callbacks import BaseCallbackHandler as _Base
except Exception:  # pragma: no cover
    _Base = object

log = logging.getLogger("hsip_langchain")


class HsipCallbackHandler(_Base):
    def __init__(self, prove: Prove, *, stream: Optional[str] = None, actor: str = "langchain",
                 record_llm: bool = False, record_errors: bool = True, max_queue: int = 1000):
        try:
            super().__init__()
        except Exception:  # pragma: no cover
            pass
        self._prove, self._stream, self._actor = prove, stream, actor
        self._record_llm, self._record_errors = record_llm, record_errors
        self._open: Dict[str, Dict[str, Any]] = {}
        self._q: "queue.Queue" = queue.Queue(maxsize=max_queue)
        self._t = threading.Thread(target=self._run, name="hsip-langchain", daemon=True)
        self._t.start()

    # tools
    def on_tool_start(self, serialized, input_str, *, run_id, **kw):
        name = (serialized or {}).get("name") or kw.get("name") or "tool"
        self._open[str(run_id)] = {"kind": "tool", "tool": name, "input": input_str}

    def on_tool_end(self, output, *, run_id, **kw):
        self._finish(run_id, {"output": str(getattr(output, "content", output))}, "tool")

    def on_tool_error(self, error, *, run_id, **kw):
        if self._record_errors:
            self._finish(run_id, {"error": f"{type(error).__name__}: {error}"}, "tool.error")
        else:
            self._open.pop(str(run_id), None)

    # models
    def on_llm_start(self, serialized, prompts, *, run_id, **kw):
        if self._record_llm:
            self._open[str(run_id)] = {"kind": "llm", "model": (serialized or {}).get("name", "llm"),
                                       "input": list(prompts)}

    def on_chat_model_start(self, serialized, messages, *, run_id, **kw):
        if self._record_llm:
            self._open[str(run_id)] = {
                "kind": "llm", "model": (serialized or {}).get("name", "chat"),
                "input": [[str(getattr(m, "content", m)) for m in batch] for batch in messages]}

    def on_llm_end(self, response, *, run_id, **kw):
        gens = [[getattr(g, "text", str(g)) for g in batch] for batch in getattr(response, "generations", [])]
        self._finish(run_id, {"output": gens}, "llm")

    def on_llm_error(self, error, *, run_id, **kw):
        if self._record_llm and self._record_errors:
            self._finish(run_id, {"error": f"{type(error).__name__}: {error}"}, "llm.error")
        else:
            self._open.pop(str(run_id), None)

    # plumbing
    def _finish(self, run_id, result: Dict[str, Any], suffix: str):
        start = self._open.pop(str(run_id), None)
        if start is None:
            return
        content = {**start, **result, "run_id": str(run_id)}
        name = start.get("tool") or start.get("model") or "run"
        try:
            self._q.put_nowait((content, f"{start['kind']}.{name}" if suffix in ("tool", "llm")
                                else f"{suffix}.{name}"))
        except queue.Full:
            log.warning("hsip: queue full, run not recorded")

    def flush(self, timeout: float = 30.0):
        self._q.join()

    def close(self):
        self._q.put(None)
        self._t.join(timeout=10)

    def _run(self):
        while True:
            item = self._q.get()
            try:
                if item is None:
                    return
                content, action = item
                self._prove.record(content, actor=self._actor, action=action[:200], stream=self._stream,
                                   subject=content["run_id"], payload_hash=hash_payload(content),
                                   metadata={"run_id": content["run_id"]})
            except Exception:
                log.exception("hsip: record failed")
            finally:
                self._q.task_done()
