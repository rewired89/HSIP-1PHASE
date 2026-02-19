# HSIP License Valuation Analysis

*Prepared for internal reference — Nyx Systems LLC*
*Last updated: February 2026 — Phase 2 milestones completed*

---

## Milestone Status (Updated)

| Milestone | Previous Status | Current Status |
|-----------|----------------|----------------|
| Windows support | ✅ Production-ready | ✅ Production-ready |
| Android support | ✅ Production-ready | ✅ Production-ready |
| **Linux port** | ❌ Not started | ✅ **Complete** |
| **macOS port** | ❌ Not started | ✅ **Complete** |
| PQC (post-quantum) implementation | ✅ Feature-gated | ✅ **Enabled by default** |
| PQC integrated into session layer | ❌ Not integrated | ✅ **Complete** |
| Cross-platform workspace build | ❌ Excluded | ✅ **Included** |

**These completions directly move the valuation upward. See revised pricing in Section 4.**

---

## Purpose

This document provides a realistic valuation framework for an HSIP commercial license negotiation with a large AI infrastructure company (e.g., OpenAI). The goal is to anchor expectations, understand what drives value, and understand what limits it.

---

## 1. Honest Assessment of HSIP's Current State

### Strengths
- Technically rigorous: 100% Rust, audited RustCrypto libraries, no unsafe blocks in crypto core
- Unique feature set: formal verification (Z3 SMT), hash-chained audit logs, cryptographic consent enforcement
- Comprehensive documentation: protocol spec, threat model, security policy, API reference
- Working test suite: 11/11 automated tests passing with real cryptographic operations
- Multi-tenant REST API with Python, Node.js, and React SDKs
- Clear regulatory value: GDPR Article 17, audit export, evidence generation
- Strong IP clarity: custom protocol, dual-license model, original implementation

### Limitations (be honest in negotiations)
- No production deployments documented yet
- Small team / single-founder project (early stage)
- Not yet battle-tested at scale
- Nation-state and large-scale DDoS threats explicitly out of scope
- No external security audit by a named third-party firm yet (planned next)

### Resolved limitations (previously listed)
- ~~Windows-only~~ → Linux and macOS ports now complete
- ~~PQC reserved for Phase 2~~ → ML-KEM-768 + ML-DSA-65 hybrid enabled by default

---

## 2. Valuation Comparables

### Protocol IP Licensing
- Security protocol IP (no public comps, estimated ranges):
  - Niche cryptographic protocol licenses: $100K – $2M perpetual
  - Enterprise security SDK perpetual licenses: $250K – $5M
  - Full IP acquisitions of security-focused Rust projects (pre-revenue): $500K – $10M

### Relevant Context
- WireGuard was eventually integrated royalty-free (open source), which is the wrong comparison — HSIP is proprietary
- Tailscale (built on WireGuard) raised at $1B+ valuation with a full product, not just IP
- Small security protocol companies with working implementations and clear differentiators have been acquired for $2M–$15M in enterprise security M&A
- At Alpha stage with no revenue and no enterprise deployments, HSIP sits at the low-to-mid end of these ranges

---

## 3. Valuation Framework

### What OpenAI would be paying for:
1. **Protocol IP** — the core consent-enforcement mechanism (most valuable)
2. **Rust implementation** — production-grade, memory-safe codebase
3. **Formal verification layer** — Z3 SMT proofs (rare, high defensibility)
4. **Audit log system** — tamper-evident, court-admissible (regulatory value)
5. **Integration SDKs** — Python, Node.js, React (reduced integration cost)
6. **Documentation** — spec, threat model, API reference (reduces onboarding)

### Value Drivers for OpenAI Specifically:
- Agent-to-agent communication security at scale (OpenClaw use case)
- EU AI Act compliance infrastructure
- Enterprise customer procurement requirements (auditability)
- Differentiation from competitors in privacy/consent narrative

---

## 4. Recommended Pricing Positions

**Note: All prices revised upward from initial draft to reflect Linux/macOS completion and PQC enablement.**

### Option A: Annual Enterprise License
**Ask: $350,000 – $600,000/year**

Covers: unlimited internal use across OpenAI infrastructure and products; excludes sublicensing or resale.

Walk-away floor: $175,000/year

Rationale: Cross-platform support (Linux/macOS) and PQC-by-default meaningfully expand the addressable deployment surface and regulatory defensibility. Comparable enterprise security contracts at OpenAI's scale justify the upper range.

### Option B: Perpetual Enterprise License
**Ask: $1,200,000 – $2,500,000**

Covers: unlimited perpetual internal use; excludes sublicensing or resale to third parties.

Walk-away floor: $700,000

Rationale: Linux and macOS ports eliminate the "Windows-only" discount. PQC-by-default addresses enterprise procurement requirements for quantum-readiness, which is an active checkbox in large-enterprise vendor assessments as of 2026.

### Option C: Full IP Acquisition
**Ask: $3,500,000 – $7,000,000**

Covers: full transfer of all HSIP IP, source code, documentation, and related rights. Includes employment/consulting offer negotiation for continued development.

Walk-away floor: $2,000,000

Rationale: Four-platform support (Windows/Android/Linux/macOS), PQC-by-default, formal verification, and court-admissible audit logs constitute a complete, defensible IP package. The floor doubles from the initial draft due to completed milestones. A third-party security audit would push the top of the range toward $10M.

### Option D: License + Consulting/Employment Hybrid
**Ask: $1,500,000 – $3,000,000 license + $250K–$400K/year consulting retainer**

Covers: perpetual license plus a 2-year engineering retainer for integration support, maintenance, and next-phase development.

Rationale: If you want ongoing involvement and income without giving up the IP outright, this structure gives OpenAI full use rights and your continued involvement, while you retain the IP for other licensing. Suggested if they say "we want you involved long-term."

---

## 5. Negotiation Notes

**Anchor high.** Start any conversation at the top of the range. OpenAI's deal teams will negotiate down. If you open at the floor, you have nowhere to go.

**Lead with the formal verification angle.** This is the feature that no competitor has. Z3 SMT proofs of consent non-forgery are machine-checkable, not marketing. That is a credible moat.

**Leverage regulatory timing.** EU AI Act compliance requirements are active now. The longer OpenAI waits to implement audit-grade infrastructure, the more expensive that gap becomes. Create urgency.

**Separate the license from implementation.** If they ask for implementation support, that is a separate professional services engagement ($150–$300/hour engineering rate, or a fixed-fee integration contract). Do not bundle it into the license price.

**Do not accept equity only.** If they offer equity compensation, insist on a meaningful cash component. OpenAI is not a startup — they have the cash to pay for IP.

**Get NDA before sharing source code.** Share the protocol spec, threat model, and test results first. Full source access is behind NDA + signed LOI.

---

## 6. What Would Change This Valuation (Upward)

- ~~Completion of Linux/macOS ports~~ → **Done. Already reflected in revised pricing.**
- ~~Phase 2 PQC (ML-KEM/ML-DSA) completion~~ → **Done. Already reflected in revised pricing.**
- Third-party security audit from a named firm (Trail of Bits, NCC Group, etc.) → +40–60%
- Any documented production deployment → +significant
- Additional paying enterprise customers → dramatically changes leverage
- iOS native port (beyond Share Extension) → +10–15%

---

## 7. Bottom Line

With Linux/macOS ports complete and PQC enabled by default, the most defensible ask for a company of OpenAI's size is:

- **Full acquisition: $3.5M–$7M** (recommended if you want a clean exit)
- **Perpetual license: $1.2M–$2.5M** (recommended if you want to retain IP for other customers)
- **License + retainer: $1.5M–$3M + $250–400K/year** (recommended if you want continued income and involvement)

Do not accept less than:
- $2,000,000 for a full IP acquisition
- $700,000 for a perpetual license
- $175,000/year for an annual license

The "Alpha status" discount argument is significantly weaker now. You have four-platform support, PQC-by-default (NIST FIPS 203/204), formal verification, and a complete REST API with SDKs. That is not an Alpha — that is an early production system.

If they push back: ask what their timeline for quantum-safe infrastructure is. The EU AI Act audit requirements alone make HSIP's tamper-evident log system worth the annual license fee.

**Do not sell exclusively without a significant premium.** If they want exclusivity, add 50–100% to the acquisition price.

---

*This document is confidential and for internal planning purposes only.*
