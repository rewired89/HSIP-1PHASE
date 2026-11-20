from types import SimpleNamespace as NS
from hsip_otel import HsipSpanProcessor, span_content
from hsip_prove import hash_payload


class FakeProve:
    def __init__(self): self.calls = []
    def record(self, record, **kw): self.calls.append((record, kw)); return {"proof_id": "p1"}


def span(attrs):
    return NS(name="loan.decision", attributes=attrs, start_time=1, end_time=2,
              status=NS(status_code=NS(name="OK")),
              get_span_context=lambda: NS(trace_id=255, span_id=15))


def test_records_only_selected_spans_and_sends_hash_and_ids():
    fp = FakeProve()
    p = HsipSpanProcessor(fp, stream="s")
    p.on_end(span({"hsip.record": True, "hsip.subject": "app-1", "secret": "x"}))
    p.on_end(span({"other": 1}))
    p.force_flush(); p.shutdown()
    assert len(fp.calls) == 1
    rec, kw = fp.calls[0]
    assert kw["payload_hash"] == hash_payload(span_content(span({"hsip.record": True, "hsip.subject": "app-1", "secret": "x"})))
    assert kw["subject"] == "app-1" and kw["stream"] == "s" and kw["action"] == "loan.decision"
    assert kw["metadata"] == {"trace_id": "0" * 30 + "ff", "span_id": "0" * 15 + "f"}


def test_failures_do_not_raise():
    class Boom:
        def record(self, *a, **k): raise RuntimeError("down")
    p = HsipSpanProcessor(Boom())
    p.on_end(span({"hsip.record": True}))
    p.force_flush(); p.shutdown()
