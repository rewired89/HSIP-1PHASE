from types import SimpleNamespace as NS
from hsip_langchain import HsipCallbackHandler
from hsip_prove import hash_payload


class FakeProve:
    def __init__(self): self.calls = []
    def record(self, record, **kw): self.calls.append((record, kw)); return {}


def test_tool_call_is_hashed_with_input_and_output():
    fp = FakeProve(); h = HsipCallbackHandler(fp, stream="s", actor="a")
    h.on_tool_start({"name": "refund"}, "order 48211", run_id="r1")
    h.on_tool_end("done", run_id="r1")
    h.flush(); h.close()
    rec, kw = fp.calls[0]
    assert rec == {"kind": "tool", "tool": "refund", "input": "order 48211", "output": "done", "run_id": "r1"}
    assert kw["payload_hash"] == hash_payload(rec) and kw["action"] == "tool.refund"
    assert kw["subject"] == "r1" and kw["stream"] == "s" and kw["metadata"] == {"run_id": "r1"}


def test_llm_only_when_enabled_and_errors_recorded():
    fp = FakeProve(); h = HsipCallbackHandler(fp)
    h.on_llm_start({"name": "m"}, ["hi"], run_id="x")
    h.on_llm_end(NS(generations=[[NS(text="yo")]]), run_id="x")
    h.on_tool_start({"name": "t"}, "i", run_id="y")
    h.on_tool_error(ValueError("bad"), run_id="y")
    h.flush(); h.close()
    assert len(fp.calls) == 1 and fp.calls[0][1]["action"] == "tool.error.t"
    assert fp.calls[0][0]["error"] == "ValueError: bad"
    fp2 = FakeProve(); h2 = HsipCallbackHandler(fp2, record_llm=True)
    h2.on_llm_start({"name": "m"}, ["hi"], run_id="x")
    h2.on_llm_end(NS(generations=[[NS(text="yo")]]), run_id="x")
    h2.flush(); h2.close()
    assert fp2.calls[0][0]["output"] == [["yo"]]
