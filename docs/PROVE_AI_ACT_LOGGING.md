# Recording high-risk AI decisions (EU AI Act Article 12 style logging)

A starting template, not legal advice. HSIP proves a record existed unaltered at a time; it does not make a system compliant. Check final dates and scope with counsel (the Digital Omnibus delay was provisionally agreed but verify adoption).

One stream per AI system, one record per decision:

```python
prove.record(decision_json, stream="credit-scoring-v3", actor="model-v3.2",
             action="credit.decision", subject=application_id,
             metadata={"model_version": "3.2", "operator": "risk-team", "human_override": False})
```

Record at least: the decision hash, model version, time (platform-set), the operator or reviewer, and any human override as a separate record. Keep the original records yourself; HSIP holds only hashes. For a regulator, export an evidence pack for the period and give them the offline verifier. Use customer-held keys if you cannot accept platform custody.
