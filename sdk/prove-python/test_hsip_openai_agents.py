from types import SimpleNamespace as NS
from hsip_openai_agents import HsipTracingProcessor
from hsip_prove import hash_payload


class FakeProve:
    def __init__(self): self.calls = []
    def record(self, record, **kw): self.calls.append((record, kw)); return {}


def span(kind, **d):
    return NS(span_id="sp1", trace_id="tr1", started_at="t0", ended_at="t1",
              span_data=NS(type=kind, export=lambda: {"type": kind, **d}))


def test_records_tools_and_handoffs_only_by_default():
    fp = FakeProve(); p = HsipTracingProcessor(fp, stream="s")
    p.on_span_end(span("function", name="refund", input="48211", output="ok"))
    p.on_span_end(span("generation", name="m"))
    p.on_span_end(span("handoff", name="to_billing"))
    p.force_flush(); p.shutdown()
    assert [c[1]["action"] for c in fp.calls] == ["function.refund", "handoff.to_billing"]
    rec, kw = fp.calls[0]
    assert kw["payload_hash"] == hash_payload(rec) and kw["subject"] == "sp1"


def test_generations_optional_and_errors_swallowed():
    class Boom:
        def record(self, *a, **k): raise RuntimeError("down")
    p = HsipTracingProcessor(Boom(), record_generations=True)
    p.on_span_end(span("generation", name="m"))
    p.force_flush(); p.shutdown()
