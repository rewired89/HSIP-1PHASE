# Sample evidence

All of this is synthetic: made by a real platform run for the fictional "Acme Lending (sample)". It pins the proof format in `crates/hsip-proof/tests/samples.rs`.

| File | Expected result of `hsip-proof verify <file>` |
|---|---|
| `sample-proof.json` | VALID. One record, anchored in a signed batch, no Bitcoin timestamp (time evidence: `batch_only`). |
| `sample-evidence-pack.json` | VALID. Three contiguous records under one signed manifest. |
| `sample-evidence-pack-removed-record.json` | INVALID. The middle record was removed but the signed manifest still lists it, so the manifest and hash-chain checks fail. |

Try it on the Proof Check page, or with `hsip-proof verify`. A reviewer exercise: give someone the removed-record pack without saying what is wrong, and ask what they conclude. See `docs/PROVE_CLAIMS.md` for what a passing result does and does not mean.
