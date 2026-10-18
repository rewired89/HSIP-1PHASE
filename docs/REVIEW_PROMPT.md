# Prompt for an outside reviewer (DeepSeek or any LLM)

How to use: run `pwsh ./scripts/make-review-bundle.ps1`, then give the reviewer one bundle at a time (paste or upload `review-bundles/*.txt`) with the prompt below. Order: 1 (format and verifier, small, highest value), 2a (platform core), 2b (routes), then 3 and 4 if you want more. Each is roughly 8k to 25k tokens. Models lose accuracy on very long inputs, so do not paste everything at once.

Before sending: this repository is proprietary. Anything you paste goes to that provider. The bundles contain source only (no keys, no `.env`, no data directory), but decide whether you are comfortable sharing the source.

---

## Prompt (copy from here)

You are a senior application-security engineer and applied cryptographer. Review the attached code adversarially. I want real defects, not praise.

**What it is.** HSIP is a multi-tenant service that gives companies tamper-evident proof of automated decisions. Customers send only a SHA-256 hash of each record. The server wraps it in an envelope, canonicalizes it (RFC 8785 JCS), hashes it (SHA-256), signs it with the organization's Ed25519 key, and chains it to the previous record (`prev_hash`, `seq`). Records from all organizations are batched into an RFC 6962 Merkle tree whose root is signed by a node anchor key and optionally committed to Bitcoin via OpenTimestamps. Anyone can verify a proof bundle or evidence pack offline with the open verifier (`hsip-proof`). The server is Rust (Axum, sqlx on SQLite or PostgreSQL), with a React console.

**Already known and accepted (do not report these as findings):** the platform holds each organization's signing key (custodial); no master-key rotation; no payment processor; no outbound email; rate limits are per node and in memory; `actor`/`action`/`subject`/`metadata` are stored in clear; low-entropy records are guessable from their hash unless salted; a proof does not show a decision was correct.

**Review these, in this order:**
1. **Format and verifier** (`hsip-proof`): can a valid-looking bundle or pack be forged, replayed across organizations or streams, or made to verify when it should not? Check canonicalization assumptions, signature domain separation (the same org key signs both an envelope hash and a manifest hash, with no domain tag; the anchor key signs a raw root), Merkle edge cases (single leaf, second-preimage resistance, index and proof-length handling), evidence-pack completeness (can a record be omitted at the start or end of a range without detection?), and panics or resource blowups on hostile JSON.
2. **Authentication and tenant isolation**: session and API-key handling, role checks, invite flow, share links, anything that lets tenant A read or write tenant B's data, privilege escalation (auditor to admin, member to owner), and the operator endpoints.
3. **Server logic**: race conditions in `engine::record` (chain forks, double counting of usage), plan-limit bypasses, anchoring edge cases (partial batches, multi-node races), SQL injection or portability bugs, error messages that leak internals, denial of service (unbounded queries, huge bodies, expensive endpoints reachable without auth).
4. **Secrets handling**: key encryption at rest, token generation and storage, password hashing, logging of sensitive values.
5. **Console and SDKs**: XSS, open redirects, token storage, unsafe HTML, hash-computation mismatches between SDKs.
6. **What would make an enterprise security team or auditor reject this product**, even if it is not a bug.

**Output format.** A table sorted by severity (Critical, High, Medium, Low, Info) with columns: ID, severity, file and function (and line if you can), what is wrong, a concrete exploit or failure scenario in 1 to 3 steps, and a minimal fix. Mark each finding as **Confirmed** (you traced it in the code) or **Suspected** (needs testing). If you find nothing in an area, say what you checked and why you believe it is sound. Do not invent line numbers or functions that are not in the files. Then list the five questions you would ask the authors before trusting this in production.

I will give you the code in several parts. After each part, reply with your findings for that part and wait for the next.

---

## After the review

Bring the findings back here. For each one: reproduce it, fix it with a test, and discard anything the reviewer got wrong (LLM reviewers produce false positives, especially on line numbers and on code they cannot see). Treat a second opinion as a lead generator, not as an audit.
