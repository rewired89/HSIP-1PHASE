# DAYANA SANCHEZ
**AI Engineer · Forward-Deployed Engineer · Secure AI Integration Specialist**

Orlando, FL · sanchezleal1989@gmail.com · github.com/rewired89

---

## Summary

Full-stack AI engineer specializing in **secure LLM integration and AI agent governance**. I take zero-spec problems to production systems solo — no handoff, no team scaffolding, just a vague problem and a shipped binary at the end. Background in cryptographic security (Ed25519, ChaCha20-Poly1305, BLAKE3 audit chains, post-quantum ML-KEM/ML-DSA) makes me rare in the AI space: I build AI systems at 10x velocity *and* I know what breaks them. Currently focused on fintech and AI safety infrastructure — where the demand for engineers who understand both LLM orchestration and adversarial threat modeling is highest and least supplied.

---

## Professional Experience

### HSIP — Cryptographic AI Identity & Compliance Infrastructure
**Founder · Solo Engineer** | 2024 – Present | github.com/rewired89/HSIP-1PHASE

Designed and shipped a production-grade local identity server solving one of the hardest problems in regulated AI deployment: *who authorized the AI agent, when, and can you prove it in court?*

- **Covered 8 financial compliance mandates** (MiFID II Art. 25, FINRA Rule 4511, SOX §404, PSD2, GDPR Art. 7, DORA, SWIFT CSCF, ISO 20022) in a single self-hosted binary — eliminating the need for separate audit vendors for each regulation.
- **Built 16 Rust crates and 238 automated tests** delivering Ed25519 non-repudiable signing, X25519 perfect forward secrecy, ChaCha20-Poly1305 encrypted key storage, and a BLAKE3 hash-chained audit log that is mathematically unalterable after write.
- **Shipped cross-platform distribution across 4 targets** (Windows x64, macOS ARM/Intel, Linux) with one-command Homebrew install, zero external dependencies, and a Chrome/Firefox browser extension blocking 61 tracker domains at the network layer.
- **Built Python, Node.js, and Go SDKs** and 30+ REST API endpoints enabling external AI agents to operate under cryptographically scoped, time-bounded user authorization — with per-agent Ed25519 keypairs, velocity anomaly detection, and auto-revocation at >1,000 req/min.
- **Shipped an MCP server** (JSON-RPC over stdio) for direct Claude Desktop integration, allowing AI clients to sign messages, check consent, and write audit entries under user-governed authorization.

---

### Nyx — Autonomous AI Trading and Personal Assistant Platform
**Independent AI Engineer** | 2023 – Present

Built and deployed a production multi-agent system combining live market execution, voice I/O, and AI orchestration — operating continuously without human-in-the-loop intervention.

- **Deployed an always-on trading assistant** (FastAPI on Railway) routing between Claude Sonnet/Haiku models via CrewAI, processing voice commands via Whisper STT and responding via OpenAI TTS with sub-second latency.
- **Engineered a 17-point signal-scoring engine** blending technical and fundamental indicators with half-Kelly position sizing; connected to Alpaca API for automated bracket-order execution with no manual confirmation required.
- **Designed CODEMAP protocol across 190+ functions** — a read-before-write discipline enforced on every function change that eliminated silent regressions during AI-assisted development sessions and cut debugging time across multi-session development cycles.

---

### Axiom-Nexus & Acheron-Nexus — Research Intelligence RAG Pipelines
**Independent AI Engineer** | 2023 – Present

Built production RAG infrastructure for cross-domain scientific literature analysis, replacing manual literature review with an autonomous evidence pipeline.

- **Ingested and classified research papers from PubMed, arXiv, and bioRxiv** into ChromaDB vector stores spanning multiple knowledge domains, with automatic re-indexing when local relevance scores fell below threshold — keeping knowledge bases current without manual curation.
- **Engineered a 5-mode reasoning engine with automatic intent detection**: routes between evidence synthesis, hypothesis generation, protocol design, decision verdicts, and plain-language tutoring based on query structure — no explicit mode selection required from the user.
- **Implemented adaptive live-retrieval fallback**: when local vector scores are insufficient, the system queries external literature APIs in real time and re-indexes high-relevance results, making the knowledge base self-improving over use.

---

### Public AI Learning Tools — CyberGuide, TradeGuide, BioGuide + 4 More
**Independent AI Engineer** | 2023 – Present | Deployed on GitHub Pages

Shipped 7 zero-dependency browser applications covering cybersecurity, circuit design, trading, bioinformatics, mathematics, physics, and world history — demonstrating the ability to deliver working, accessible software with no build tooling and no infrastructure cost.

- **CyberGuide**: Browser-based cybersecurity learning platform covering Network+ and Security+ certification paths through interconnected concept chains — zero npm, zero build step, runs offline.
- **Parallax**: Interactive historical timeline spanning 13.8 billion years with parallel civilization tracking — pure HTML/CSS/JS, deployed as a static GitHub Pages site with no server and no account required.
- All 7 tools deploy from a single `git push` with zero configuration — demonstrating full-cycle delivery discipline even on non-commercial projects.

---

### LibGuide — AI-Assisted Developer Learning Tool
**Independent AI Engineer** | 2023 – Present | Electron Desktop App

- **Built a pedagogically constrained teaching mode** that withholds complete solutions, forcing learners to build problem-solving intuition rather than copy outputs — two modes: project build-paths and library discovery.
- **Automated signed release pipeline** (GitHub Actions) producing signed Windows and macOS installers on every version tag — zero manual packaging steps after initial CI setup.

---

## Technical Skills

| Domain | Stack |
|---|---|
| **LLM Integration** | Claude API (Anthropic), CrewAI multi-agent routing, prompt engineering, RAG, context window management |
| **Secure AI Systems** | Ed25519 signing, X25519 key exchange, ChaCha20-Poly1305 AEAD, BLAKE3 hash chains, HKDF-SHA-256, ML-KEM-768, ML-DSA-65 (post-quantum) |
| **Backend** | Rust (Axum, Tokio), Python (FastAPI), REST API design, SQLite, PostgreSQL |
| **Voice & Real-time** | Whisper / faster-whisper STT, OpenAI TTS, push-to-talk UX, VAD pipelines |
| **Vector & Data** | ChromaDB, TF-IDF retrieval, embedding pipelines, PubMed/arXiv/bioRxiv ingestion |
| **Frontend & Delivery** | React, Electron, vanilla JS, GitHub Actions CI/CD, Homebrew tap, cross-platform installers |
| **Quant & ML** | LightGBM, walk-forward validation, backtesting, Alpaca API, signal scoring, half-Kelly sizing |
| **Security** | Penetration testing, OSINT/SOCMINT, threat modeling, audit log design, capability-gated systems |

---

## Education

**Cybersecurity Associate** — Miami Dade College
Degree in progress · Expected graduation: [ADD DATE]

Self-directed study: penetration testing, OSINT/SOCMINT methodology, applied bioinformatics, algorithmic trading, machine learning (Python, LightGBM, scikit-learn)

---

## Additional

- **Languages:** English (fluent), Spanish (native)
- **Availability:** Open to remote full-time or contract roles
- **Portfolio:** github.com/rewired89
- **Niche focus:** Secure LLM integration and AI agent governance for fintech and regulated industries

