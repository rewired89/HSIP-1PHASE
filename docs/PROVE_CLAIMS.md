# What an HSIP proof covers

A proof is evidence for a reviewer. It is not a verdict. Use this table, word for word, when describing what HSIP can show.

| Question a reviewer asks | Covered today? | What you can say | What you cannot say |
|---|---|---|---|
| Are these the same bytes that were recorded? | **Yes** | "These bytes match this signed record." | That the record describes what really happened. |
| Who signed it? | **Partly** | "Signed by the key written in the file." Compare that key with the one the organization publishes, through a second channel. | "Signed by the organization", unless the key was compared. |
| Is the record in a signed batch? | **Yes** | "Included in this signed batch." | That the batch is the whole history. |
| Is the history unbroken and not rolled back? | **Partly** | Records link by hash inside a stream, and a record removed from an exported pack is detected. If you kept the platform's signed checkpoints, a rewritten history, a cut-off tail, two different heads for one position, and a head that moved backwards are detected. | That nothing was rewritten at positions you hold no checkpoint for. A pack alone cannot show rollback or a split history: the checkpoints must be kept by the customer, outside the platform. See `PROVE_PROOF_FORMAT.md`. |
| When did it exist? | **Partly** | The signer's own timestamp, the batch signature, and, if requested, a Bitcoin timestamp. The verifier reports which state the file is in: not anchored, signed batch only, pending, proof attached, missing proof. | That the time is proven. The Bitcoin proof is checked with an OpenTimestamps client, not by the verifier. |
| Was everything recorded? | **No** | "These actions were recorded." | That nothing else happened. A proof cannot show an action that never entered the recording path. Describe the capture boundary instead (see `CAPTURE_BOUNDARY_TEMPLATE.md`). |
| Was the decision right, authorized or lawful? | **No** | "Evidence for review." Signed permissions compare recorded actions with what was allowed, after the fact. | That the decision was correct or that anyone is compliant. HSIP does not stop an agent before it acts. |

A valid result with one green mark hides these differences. The Proof Check page and the command-line tool list each check separately for that reason.
