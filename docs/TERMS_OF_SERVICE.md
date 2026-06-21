# HSIP Terms of Service

**Effective date:** 2026-06-21  
**Contact:** sanchezleal1989@gmail.com

> **Note to the author (Dayana Sanchez):** This is a draft for your review — not published legal advice. Have a lawyer review it before making it publicly binding, especially before signing enterprise contracts. The key decisions you need to make are marked with **[REVIEW]**.

---

## 1. What HSIP Is

HSIP ("High Security Internet Protocol") is software that provides cryptographic identity management, consent records, message signing, verifiable credentials, and an audit trail. It is distributed as open-source software and as a hosted demo at `hsip-1phase-production.up.railway.app`.

HSIP is a **tool**, not a compliance certification. Using HSIP does not make you compliant with any law or regulation. Whether HSIP satisfies a specific regulatory requirement in your jurisdiction is a legal and compliance question that HSIP does not answer.

---

## 2. The Demo Instance

The hosted demo at `hsip-1phase-production.up.railway.app` is provided **for evaluation purposes only**:

- Trial keys expire after **24 hours**.
- Data may be **erased at any time** without notice (demo database is not backed up).
- The demo instance is **not suitable for production use**, sensitive data, or commercial deployments.
- There are **no uptime guarantees** on the demo.

For production use, deploy your own HSIP instance. See `DEPLOYMENT.md`.

---

## 3. License

HSIP source code is available on GitHub at `https://github.com/rewired89/HSIP-1PHASE`.

**[REVIEW — choose one:]**

**Option A — Commercial license:**  
Use of the HSIP software requires a valid commercial license. Contact sanchezleal1989@gmail.com to discuss licensing terms. The source code is publicly visible for inspection but is not licensed for production use without a written agreement.

**Option B — Open source with commercial support:**  
The software is licensed under [INSERT LICENSE — e.g. MIT / Apache 2.0 / AGPL]. You may use it freely under those terms. Commercial support, SLAs, and custom integrations are available under a separate agreement.

---

## 4. No Warranty

HSIP is provided **"as is"** without warranty of any kind, express or implied. The author makes no warranties regarding:

- Fitness for a particular purpose
- Absence of security vulnerabilities
- Continuous availability
- Accuracy of documentation

No third-party security audit has been completed as of the effective date above. A third-party audit is planned before v1.0 commercial release. The `THREAT_MODEL.md` documents known limitations openly.

---

## 5. Limitation of Liability

To the maximum extent permitted by applicable law, the author shall not be liable for any indirect, incidental, special, consequential, or punitive damages arising from your use of HSIP, including but not limited to:

- Data loss
- Security breaches resulting from misconfiguration
- Business losses
- Regulatory penalties

**[REVIEW — cap the liability:]** In no event shall total liability exceed the amount you paid for HSIP in the twelve months preceding the claim, or $100 USD, whichever is greater.

---

## 6. What Data We Collect

**Self-hosted instances:** We collect nothing. All data stays on your server. You are the data controller for everything stored in your HSIP database.

**The hosted demo:** When you use the demo, your browser connects to the Railway-hosted server. We do not intentionally collect personal information beyond what you provide (e.g., messages you sign, consent records you create). Demo data may be retained for up to 30 days before being purged. We do not sell or share demo data.

**[REVIEW]** If you intend to target EU users, you may need a GDPR-compliant privacy policy separate from this ToS, including a lawful basis for processing, data retention periods, and a data subject rights procedure.

---

## 7. Acceptable Use

You may not use HSIP (including the hosted demo) to:

- Violate any applicable law or regulation
- Impersonate another person or organization
- Conduct automated attacks against other services
- Store illegal content
- Circumvent security controls of systems you do not own

---

## 8. Changes

These terms may be updated at any time. Continued use of the demo after an update constitutes acceptance. Commercial licensees will be notified of material changes in writing.

---

## 9. Governing Law

**[REVIEW — insert your jurisdiction, e.g.:]**  
These terms are governed by the laws of [State/Country]. Disputes shall be resolved in [City, State/Country].

---

## 10. Contact

Questions about these terms: sanchezleal1989@gmail.com  
Security vulnerabilities: same address with subject `[HSIP SECURITY]`
