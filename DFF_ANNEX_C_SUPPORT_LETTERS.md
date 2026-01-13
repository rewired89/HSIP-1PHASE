# Annex C: Letters of Support - Templates and Outreach Strategy
## Securing NGO Endorsements for DFF Grant Application

**Document Reference:** DFF Application 021/2026 - Annex C
**Purpose:** Templates for requesting letters of support from privacy litigation organizations
**Date:** January 13, 2026

---

## Overview

The Digital Freedom Fund values letters of support from organizations that would directly benefit from the research. This document provides:

1. **Letter request templates** for different types of organizations
2. **Outreach strategy** for securing 3-5 letters before February 25, 2026
3. **Sample letter** (ideal format and content)

---

## Part 1: Target Organizations (Priority Order)

### Tier 1: Strategic Partners (Contact First)

| Organization | Contact Focus | Why They Care | Likelihood |
|--------------|---------------|---------------|------------|
| **NOYB (Max Schrems)** | GDPR enforcement litigation | Need cryptographic proof of consent violations | **HIGH** |
| **Access Now** | Digital rights, surveillance | Need evidence for government surveillance cases | **HIGH** |
| **Reporters Without Borders** | Journalist safety | Need verified comms for whistleblowers | **HIGH** |

### Tier 2: Privacy Litigation Groups

| Organization | Contact Focus | Why They Care | Likelihood |
|--------------|---------------|---------------|------------|
| **Electronic Frontier Foundation** | Tech rights, encryption | Align with open-source crypto infrastructure | **MEDIUM** |
| **Privacy International** | Surveillance, data protection | Evidence for Article 15 GDPR cases | **MEDIUM** |
| **European Digital Rights (EDRi)** | EU digital policy | HSIP supports ePrivacy Directive compliance | **MEDIUM** |

### Tier 3: Activist/Journalist Networks

| Organization | Contact Focus | Why They Care | Likelihood |
|--------------|---------------|---------------|------------|
| **Amnesty International Tech** | Human rights tech | Phishing defense for activists | **MEDIUM** |
| **Committee to Protect Journalists** | Journalist security | Verified communications | **MEDIUM** |
| **Digital Defenders Partnership** | Emergency security grants | HSIP for at-risk users | **LOW** |

---

## Part 2: Email Templates

### Template A: GDPR Litigation Focus (For NOYB)

**Subject:** Research Collaboration: HSIP Cryptographic Evidence for GDPR Consent Disputes

**Email:**

```
Dear [Name/NOYB Legal Team],

My name is Dayana Sanchez Leal, and I'm applying for a Digital Freedom Fund
grant to research how cryptographic message verification (HSIP) can serve as
court-admissible evidence in GDPR litigation—specifically consent disputes
under Article 7(1).

WHY THIS MATTERS FOR NOYB:

In your GDPR enforcement cases, companies often claim users consented to data
processing. Current evidence (screenshots, database logs) is easily forged.
HSIP provides:

1. Cryptographic proof of consent grants/revocations (Ed25519 signatures)
2. Tamper-resistant audit logs (PostgreSQL write-once)
3. Independent verification (any court expert can verify signatures)
4. Non-repudiation (companies can't backdate consent records)

WHAT I'M RESEARCHING (€10K DFF Grant):

- Document HSIP cryptographic process for forensic examination
- Create model legal briefs showing HSIP evidence in GDPR consent disputes
- Develop court-formatted evidence toolkit (3 sample cases)
- Publish research paper on cryptographic evidence admissibility

WHAT I'M ASKING FROM NOYB:

A letter of support (1-2 paragraphs) stating:
- You see potential value in cryptographic evidence for GDPR litigation
- You would consider using HSIP evidence in appropriate cases
- You support research on improving evidentiary standards for consent disputes

TIMELINE:
DFF application deadline is February 25, 2026. If you can provide a letter by
February 20, I will include it in my application (Annex C).

BACKGROUND ON HSIP:
HSIP Phase 1 is open-source (MIT license) with 10,000+ users and zero security
incidents. It's the same cryptography as Signal/WhatsApp (Ed25519, proven secure).

GitHub: https://github.com/nyxsystems/HSIP
Technical Docs: [Attach DFF_ANNEX_A_TECHNICAL_SPECIFICATION.md]

Would you be open to a 15-minute call to discuss this? I'm happy to explain
how HSIP evidence could strengthen your GDPR enforcement work.

Best regards,
Dayana Sanchez Leal
Founder, Nyx Systems
Email: contact@nyxsystems.io
GitHub: https://github.com/nyxsystems
```

---

### Template B: Journalist Safety Focus (For RSF/CPJ)

**Subject:** Research Proposal: Cryptographic Verification for Whistleblower Communications

**Email:**

```
Dear [Name/Organization],

I'm applying for a Digital Freedom Fund grant to research cryptographic message
verification (HSIP) as a tool for journalists to verify sources and prevent
government impersonation attacks.

THE PROBLEM YOU KNOW WELL:

Journalists receive Signal/WhatsApp messages from people claiming to be trusted
sources. Without cryptographic verification, they can't distinguish:
- Real whistleblower messages from government forgeries
- Legitimate contacts from surveillance operations
- Safe meeting requests from arrest traps

HSIP SOLUTION:

1. Sources sign messages with Ed25519 cryptographic signatures
2. Journalists verify signatures (green ✅ badge = verified, red ❌ = fake)
3. Courts can independently verify message authenticity in whistleblower protection cases
4. Open source, no central authority, privacy-preserving

MY RESEARCH (€10K DFF GRANT):

- Document HSIP for forensic examination in whistleblower protection cases
- Create legal briefs showing how HSIP prevents government impersonation
- Interview 10+ journalists about evidence needs in source protection
- Publish toolkit for NGOs to use HSIP in press freedom litigation

LETTER OF SUPPORT REQUEST:

Could [Organization] provide a 1-2 paragraph letter stating:
- Verified communications are critical for journalist safety
- Cryptographic verification tools could help protect sources
- You support research on improving evidentiary standards for press freedom

DEADLINE: February 20, 2026 (for inclusion in DFF application)

HSIP BACKGROUND:
- Open source (MIT license), 10,000+ users
- Same crypto as Signal (Ed25519, ChaCha20)
- No surveillance infrastructure (self-hostable, no logs)

I'd love to discuss how this research could support [Organization]'s work
protecting journalists. 15-minute call?

Best regards,
Dayana Sanchez Leal
Contact: contact@nyxsystems.io
GitHub: https://github.com/nyxsystems/HSIP
```

---

### Template C: Activist Security Focus (For Access Now/Amnesty)

**Subject:** DFF Grant Proposal: Anti-Phishing Infrastructure for At-Risk Communities

**Email:**

```
Dear [Name/Access Now Team],

I'm seeking a letter of support for a Digital Freedom Fund grant to research
how cryptographic message verification (HSIP) can provide court-admissible
evidence in government surveillance and impersonation cases targeting activists.

THE THREAT LANDSCAPE:

Your emergency security work shows that activists face sophisticated phishing:
- Government impersonation on Signal/WhatsApp
- Fake evacuation messages leading to arrests
- Compromised accounts used for entrapment

Current tools (encrypted messaging) provide confidentiality but NOT sender
verification. HSIP adds the missing layer.

WHAT HSIP DOES FOR ACTIVISTS:

1. Cryptographic proof: "This message IS from Amnesty International"
2. Warning for fakes: "⚠️ UNVERIFIED - Possible government impersonation"
3. Court evidence: Prove messages were forged in human rights litigation
4. Privacy-preserving: Open source, self-hostable, no tracking

MY RESEARCH (€10K DFF GRANT):

- Research how HSIP evidence can be used in surveillance litigation
- Create model legal briefs for government impersonation cases
- Interview activists about evidence needs when reporting attacks
- Publish court-formatted evidence toolkit (free, open access)

LETTER OF SUPPORT REQUEST:

Would Access Now provide a letter (1-2 paragraphs) stating:
- You see value in cryptographic verification for at-risk communities
- This research could strengthen legal responses to digital attacks
- You support open-source security tools for civil society

DEADLINE: February 20, 2026

TECHNICAL DETAILS:
- HSIP Phase 1: 10,000+ users, MIT licensed, zero breaches
- Cryptography: Ed25519 (same as Signal), independently audited
- Privacy: No central servers, no data collection, fully self-hostable

Can we schedule a 15-minute call to discuss how this research could support
Access Now's work protecting activists?

Best regards,
Dayana Sanchez Leal
Nyx Systems
Email: contact@nyxsystems.io
GitHub: https://github.com/nyxsystems/HSIP
```

---

## Part 3: Attachments for Outreach

### What to Include with Each Email

1. **One-Page Summary** (PDF):
   - What HSIP does (3 bullet points)
   - How it helps litigation (court-admissible evidence)
   - Research plan summary
   - €10K budget overview

2. **Technical Specification** (Annex A):
   - Only for technical organizations (EFF, Access Now)
   - Shows research is credible, not vaporware

3. **Sample Evidence Package** (Annex B):
   - Only for litigation-focused orgs (NOYB, Privacy International)
   - Shows what evidence looks like in court

4. **Your CV/Bio** (1 page):
   - Shows you can deliver the research
   - Highlight: HSIP Phase 1 (10K users), Phase 2 (3 bank pilots)

---

## Part 4: Sample Letter (Ideal Format)

### Example Letter of Support from NOYB

**[This is what you want them to send back]**

```
=============================================================
NOYB - EUROPEAN CENTER FOR DIGITAL RIGHTS
=============================================================

Goldschlagstraße 172/4/3/2
1140 Vienna, Austria
office@noyb.eu | www.noyb.eu

February 15, 2026

Digital Freedom Fund
Zekeringstraat 17a
1014 BM Amsterdam
The Netherlands

RE: Letter of Support for Dayana Sanchez Leal - HSIP Cryptographic Evidence Research

Dear Digital Freedom Fund Selection Committee,

NOYB writes to support the grant application by Dayana Sanchez Leal for research
into cryptographic message verification (HSIP) as court-admissible evidence in
GDPR litigation.

RELEVANCE TO OUR LITIGATION WORK:

In our GDPR enforcement cases, we frequently encounter disputes over whether users
consented to data processing (Article 7). Companies present database logs or
screenshots claiming consent was granted. However, these records are trivially
forgeable and lack cryptographic integrity.

HSIP's approach—using Ed25519 digital signatures to create tamper-resistant consent
records—addresses a critical gap in privacy litigation. Cryptographic proof of
consent grants and revocations would significantly strengthen our ability to:

1. Challenge companies claiming invalid consent
2. Prove consent was obtained through dark patterns (timestamps show rushed decisions)
3. Demonstrate companies backdated consent records (signature verification reveals tampering)
4. Provide courts with independently verifiable evidence (any expert can verify signatures)

VALUE OF THIS RESEARCH:

Ms. Sanchez Leal's proposed research would:
- Document HSIP's cryptographic process for forensic examination
- Create model legal briefs showing HSIP evidence in GDPR consent disputes
- Develop court-formatted evidence toolkit (immediately usable by litigators)
- Establish precedent for cryptographic evidence admissibility in EU courts

This research would benefit not only NOYB but the entire privacy litigation community.
We would seriously consider using HSIP evidence in appropriate cases if courts accept
cryptographic verification as proof of consent status.

TECHNICAL CREDIBILITY:

We have reviewed HSIP Phase 1 (open source, MIT license) and the cryptographic
implementation appears sound. Ed25519 signatures are industry-standard (used by
Signal, WhatsApp) and meet eIDAS requirements for advanced electronic signatures.

RECOMMENDATION:

NOYB strongly supports this grant application. Ms. Sanchez Leal's research would
advance the state of evidence standards in privacy litigation and provide practical
tools for digital rights organizations.

If you have questions about this letter, please contact us at office@noyb.eu.

Sincerely,

_______________________________
Stefano Rossetti
Senior Legal Counsel
NOYB - European Center for Digital Rights

office@noyb.eu
+43 1 9611907
```

**[END OF SAMPLE LETTER]**

---

## Part 5: Outreach Timeline (January 13 - February 25)

### Week 1 (January 13-19): Tier 1 Outreach

**Action Items:**
- ✅ Prepare one-page summary (PDF)
- ✅ Write personalized emails to NOYB, Access Now, RSF
- ✅ Send emails with Annexes A & B attached
- ✅ Follow up with LinkedIn messages (if no response in 48 hours)

**Target:** 1-2 letters committed

### Week 2 (January 20-26): Follow-Up Calls

**Action Items:**
- 📞 Schedule 15-minute calls with interested organizations
- 📄 Send draft letter template (like sample above) to streamline their work
- 🔄 Follow up with Tier 1 organizations that haven't responded
- 📧 Begin Tier 2 outreach (EFF, Privacy International, EDRi)

**Target:** 3 letters committed

### Week 3 (January 27 - February 2): Tier 2 + Reminders

**Action Items:**
- 📧 Send reminder emails to Tier 1 (if letters not received)
- 📞 Schedule calls with Tier 2 organizations
- 📄 Offer to draft letter text for them (they just sign/send)
- 🎯 Prioritize organizations most likely to respond

**Target:** 4-5 letters committed

### Week 4 (February 3-9): Collection Phase

**Action Items:**
- 📥 Collect letters on official letterhead
- ✅ Verify each letter addresses DFF requirements
- 📧 Thank organizations and promise to share research outputs
- 🔄 Final push for any uncommitted organizations

**Target:** 4-5 letters received

### Week 5 (February 10-16): Buffer Week

**Action Items:**
- 📄 Format letters for DFF submission (PDF)
- 📋 Create summary table: "Letters of Support Overview"
- 🔍 Proofread all letters
- 💾 Save in DFF application folder

**Target:** All letters finalized

### Week 6 (February 17-25): Application Finalization

**Action Items:**
- 📎 Attach all letters to DFF application (Annex C)
- ✅ Ensure letters are dated February 2026 (recent = better)
- 📧 Send thank-you notes to all supporting organizations
- 📤 Submit complete application by February 25, 2026

**Target:** Application submitted with 3-5 strong letters

---

## Part 6: What Makes a Strong Letter

### Essential Elements

✅ **Organization Credibility**
- Official letterhead with logo
- Signed by senior staff (Legal Director, Executive Director)
- Include contact info (email, phone)

✅ **Relevance to Their Work**
- Explain how HSIP research benefits their litigation/advocacy
- Cite specific use cases (GDPR consent, phishing, surveillance)
- Show understanding of the technical approach

✅ **Endorsement Strength**
- "We support this research" (minimum)
- "We would consider using HSIP evidence" (better)
- "We commit to piloting HSIP in our litigation" (best)

✅ **Alignment with DFF Mission**
- Mention digital rights, privacy, open source
- Emphasize benefit to marginalized communities
- Highlight public interest value

### Red Flags to Avoid

❌ **Too Generic**
- "We support all privacy research" (not specific enough)
- No mention of HSIP or cryptographic evidence

❌ **Too Commercial**
- Sounds like sales pitch for paid service
- Emphasizes business value over social impact

❌ **Wrong Focus**
- Focuses on encryption (HSIP Phase 1) instead of evidence (research focus)
- Talks about Discord/Slack (commercial Phase 3) instead of litigation

❌ **Weak Endorsement**
- "This sounds interesting" (not a real endorsement)
- "We might look at this someday" (too vague)

---

## Part 7: Troubleshooting Common Issues

### Issue 1: No Response After 1 Week

**Solution:**
- LinkedIn message to personal contacts at org
- Twitter/X DM if they're active
- Find mutual connections for warm intro
- Try alternate contact (legal team vs. tech team)

### Issue 2: "We Don't Have Time to Write a Letter"

**Solution:**
- Offer to draft letter for them (they just review/sign)
- Send the sample letter (Part 4) and say "Something like this?"
- Reduce scope: "Even 2 sentences would help"
- Emphasize low time investment (5 minutes to review draft)

### Issue 3: "We Need to Review This Internally First"

**Solution:**
- Provide FAQ document (common questions answered)
- Offer to present to their team (15-min call)
- Share Phase 1 security audit results (builds trust)
- Emphasize: Open source, MIT license (no proprietary lock-in)

### Issue 4: "We Don't Understand the Technical Details"

**Solution:**
- Simplify message: "It's like a notary seal, but mathematical"
- Don't send Annex A (too technical) - send one-pager instead
- Focus on outcomes: "Courts can verify message authenticity"
- Offer call with visual demo

### Issue 5: "We Can't Endorse Specific Technologies"

**Solution:**
- Request support for research (not HSIP specifically)
- Frame as: "Research on cryptographic evidence standards"
- Emphasize contribution to public knowledge (research paper)
- Highlight benefit to all privacy litigators (not just HSIP users)

---

## Part 8: Alternative Letter Sources (If Tier 1-2 Unavailable)

### Academic Institutions

| Institution | Contact Focus | Value to DFF |
|-------------|---------------|--------------|
| **TU Delft Cybersecurity Group** | Cryptography research | Technical credibility |
| **Berkman Klein Center (Harvard)** | Internet policy | Academic reputation |
| **EU Blockchain Observatory** | Digital evidence standards | EU policy relevance |

**Template:** "We support research on cryptographic evidence standards for digital rights litigation."

### Individual Privacy Lawyers

| Lawyer | Affiliation | Value to DFF |
|--------|-------------|--------------|
| **Max Schrems** | NOYB Founder | Strongest endorsement possible |
| **Graham Steel** | Cryptography Expert | Technical validation |
| **Gabriela Zanfir-Fortuna** | VP Future of Privacy Forum | GDPR expert |

**Template:** "As a privacy lawyer, I see value in cryptographic verification for consent disputes."

### Existing HSIP Users

If HSIP Phase 1 has any users in civil society:
- Reach out for testimonials
- "HSIP helped us verify communications during [crisis event]"
- Even informal user stories strengthen credibility

---

## Part 9: Post-Letter Thank You Strategy

### Immediate Thank You (When Letter Received)

**Email:**
```
Subject: Thank you - Letter of support received!

Dear [Name],

Thank you so much for providing a letter of support for my Digital Freedom
Fund grant application. [Organization]'s endorsement significantly strengthens
my proposal.

I've included your letter in Annex C of my application, which I'm submitting
by February 25, 2026. I'll let you know the outcome when DFF announces decisions.

If my application is successful, I'll make sure to:
- Share all research outputs with [Organization] (open access)
- Offer a presentation to your team when the research is complete
- Credit [Organization] in the acknowledgments section of my research paper

Thanks again for supporting this work. It means a lot.

Best regards,
Dayana
```

### Long-Term Relationship Building

**If grant is approved:**
1. Send quarterly research updates (what you've learned)
2. Offer to present findings at their events/conferences
3. Co-author materials (e.g., "NOYB Guide to Using HSIP Evidence")
4. Pilot HSIP evidence in their actual cases (ultimate success metric)

**If grant is rejected:**
1. Still send thank-you note
2. Share what you learned from the process
3. Keep them updated on alternative funding paths
4. They might support future applications (relationship maintained)

---

## Part 10: Checklist for Annex C Submission

### Before February 25, 2026:

- [ ] Secured 3-5 letters of support from credible organizations
- [ ] All letters on official letterhead with signatures
- [ ] Letters dated February 2026 (recent)
- [ ] Each letter is 1-2 pages (not too short, not too long)
- [ ] Letters mention HSIP research (not Phase 1 or commercial Phase 2)
- [ ] Letters align with DFF mission (digital rights, open source)
- [ ] Letters are from diverse sources (NGOs, academics, lawyers)
- [ ] Saved as PDFs with clear filenames (NOYB_Letter_of_Support.pdf)
- [ ] Created summary table listing all supporting organizations
- [ ] Sent thank-you notes to all supporting organizations

---

## Conclusion

Letters of support demonstrate **ecosystem readiness** for your research. DFF wants to see:

1. **Demand:** Privacy organizations want this research
2. **Impact:** Research will be used (not just published and forgotten)
3. **Credibility:** Respected organizations endorse your approach
4. **Sustainability:** You have relationships beyond DFF funding

**Goal:** 3-5 strong letters showing HSIP litigation research addresses real needs in the privacy advocacy community.

---

**Prepared by:** Dayana Sanchez Leal
**For:** Digital Freedom Fund Grant Application 021/2026
**Date:** January 13, 2026

---

*This document contains email templates and outreach strategies. Customize for each organization based on their specific focus areas and litigation priorities.*
