# HSIP Outreach Email Templates

## **Email Type 1: VC Fundraising (Target: $5M-10M Series A)**

### **Subject Line Options:**
1. "Banking anti-phishing infrastructure - Series A"
2. "Preventing $10B in bank phishing losses - seeking lead investor"
3. "HSIP: Message authentication for financial institutions"

### **Email Template - Cold Outreach to Partner:**

```
Subject: Banking anti-phishing infrastructure - Series A

Hi [Partner Name],

I'm [Your Name], founder of HSIP. We've built cryptographic authentication
infrastructure that prevents phishing attacks on bank customers.

THE PROBLEM:
Banks lose $10B/year to phishing. Customers can't tell if a message saying
"Your card was charged $500" is really from their bank or a scammer. Current
solutions (2FA, biometrics) authenticate users, not messages.

OUR SOLUTION:
Banks sign every message with Ed25519 cryptographic signatures. Customers see
✅ VERIFIED (authentic) or 🔴 UNVERIFIED (phishing) badges. Works with existing
SMS/push infrastructure - no new channels required.

PRODUCT STATUS:
- Complete SDKs: Python (backend signing), iOS/Android (mobile verification)
- Patent pending: 2 provisional patents filed
- Pilot discussions: [Bank 1], [Bank 2] (or write "reaching out to Chime,
  Varo, Ally")

MARKET:
- TAM: $500M+ (25,000 banks worldwide, $15K-50K/year each)
- Comparable: Auth0 ($6.5B exit), Duo ($2.3B exit), Plaid ($13.4B valuation)
- Competition: None (banks currently have no solution for message authentication)

THE ASK:
Raising $5M-10M Series A at $30M-40M pre-money valuation.
- Use of funds: Sales (hire 3 BDRs), engineering (2 engineers), compliance
  (SOC2, security audits)
- 18-month runway to 50+ bank customers
- Exit: Strategic acquisition by Twilio, Okta, Visa, or Mastercard ($200M-500M)

Would love 30 minutes to show you a demo and discuss further.

Best,
[Your Name]
Founder & CEO, HSIP
[email] | [phone]

P.S. - Deck attached. Happy to intro you to banking partners or security
advisors for diligence.
```

---

### **Follow-up Email (If No Response After 7 Days):**

```
Subject: Re: Banking anti-phishing infrastructure - Series A

Hi [Partner Name],

Following up on my email from last week about HSIP.

Since I last reached out:
- [Update: New bank pilot signed, patent filed, press mention, etc.]

Quick question: Is fintech/security infrastructure in your focus area?
If not, would you mind introducing me to the right partner at [VC Firm]?

Either way, appreciate your time.

Best,
[Your Name]
```

---

## **Email Type 2: M&A - Acquisition Interest**

### **Email Template - Twilio**

```
Subject: Message authentication infrastructure - acquisition interest

Hi [BD Director / M&A Team],

I'm [Your Name], founder of HSIP. We've built message authentication
infrastructure that complements Twilio Verify perfectly.

THE GAP IN TWILIO'S PRODUCT:
- Twilio Verify authenticates users (2FA, phone verification) ✅
- But nothing authenticates the MESSAGES Twilio sends ❌
- Result: Customers can't tell if an SMS is really from the company or a scammer

HSIP FILLS THIS GAP:
- Banks/companies sign messages using Ed25519 signatures
- Customers verify signatures on their phones (✅ verified / 🔴 unverified)
- Works with Twilio SendGrid (email) and Programmable SMS

PRODUCT FIT:
- Twilio Verify: "Is this user real?"
- HSIP: "Is this message real?"
- Bundle them: Complete trust infrastructure

TRACTION:
- Complete SDKs (Python, iOS, Android)
- Patent pending (2 provisional patents)
- Pilot discussions with [Banks]

WHY NOW:
- Banking regulators (FDIC, OCC) are pushing for anti-phishing solutions
- $10B annual losses create urgency
- First-mover advantage: If Twilio doesn't own this, competitors will

STRATEGIC VALUE TO TWILIO:
- Upsell to existing SendGrid customers (148B emails/month)
- Differentiate from AWS SNS, Google Cloud Messaging
- Expand into banking/fintech (your fastest-growing segment)

THE ASK:
Open to acquisition discussions. Would Twilio be interested in exploring a
strategic acquisition?

Happy to show you a demo or send over technical documentation.

Best,
[Your Name]
Founder, HSIP
[email] | [phone]
```

---

### **Email Template - Okta**

```
Subject: Message authentication for identity platforms - acquisition interest

Hi [Name],

Quick intro: I built HSIP, a message authentication system that extends Okta's
identity platform to verify messages, not just users.

THE OPPORTUNITY:
Okta authenticates users (login, MFA) but doesn't authenticate the messages
companies send to users. HSIP fills this gap using Ed25519 signatures.

USE CASE (Banking):
1. Bank sends: "Transaction: $500 at Amazon"
2. Bank signs message with HSIP
3. Customer's phone verifies signature
4. Shows ✅ VERIFIED (real bank) or 🔴 UNVERIFIED (phishing)

WHY OKTA SHOULD CARE:
- 17,000+ Okta customers (many banks, fintechs) need this
- Prevents $10B/year in phishing losses
- Natural bundle: Okta Auth (user identity) + HSIP (message identity)
- Competitive: Auth0 (now Okta) needs new revenue streams post-acquisition

PRODUCT STATUS:
- Complete SDKs: Python, iOS, Android
- Patent pending: Message authentication system
- Early pilots: [Banks or "in discussions with digital banks"]

STRATEGIC FIT:
Similar to Auth0 acquisition ($6.5B) - you're buying developer infrastructure
that solves a massive problem (phishing vs authentication).

Open to acquisition. 15-minute call to discuss?

Best,
[Your Name]
[email] | [phone]
```

---

### **Email Template - Stripe**

```
Subject: Anti-fraud infrastructure for financial communications

Hi [Stripe BD Team],

I'm [Your Name], founder of HSIP. We've built message authentication
infrastructure that complements Stripe Radar (fraud detection).

THE PROBLEM STRIPE FACES:
Stripe Radar detects fraudulent transactions. But what about fraudulent
MESSAGES pretending to be from Stripe or your customers?

Example phishing attack:
- Scammer sends: "Stripe payment failed - update your card at [fake-link]"
- Customer clicks → loses money
- Blames Stripe → support costs, brand damage

HSIP SOLUTION:
- Stripe signs every notification with cryptographic signatures
- Customers see ✅ VERIFIED (real Stripe) or 🔴 UNVERIFIED (phishing)
- Works with existing email/SMS infrastructure

PRODUCT STATUS:
- Complete SDKs (Python backend, iOS/Android mobile)
- Patent pending
- Designed for high-volume (millions of messages/day)

WHY STRIPE:
- You're expanding beyond payments → billing, fraud, banking
- HSIP is fraud prevention for messages (fits Stripe Radar)
- Your customers (banks, fintechs) desperately need this

COMPARABLE ACQUISITIONS:
- Bouncer (email validation): $20M
- Radar (fraud): Built in-house
- HSIP: Similar strategic value to Radar for message trust

Open to acquisition or partnership. 20-minute demo?

Best,
[Your Name]
[email] | [phone]
```

---

### **Email Template - Plaid**

```
Subject: Banking security infrastructure - acquisition interest

Hi [Plaid BD],

I'm [Your Name], founder of HSIP. We built message authentication for banks -
perfect fit for Plaid's banking API infrastructure.

THE OPPORTUNITY:
Plaid connects 11,000+ banks/fintechs. HSIP secures the messages those
institutions send to customers.

HOW IT WORKS:
- Bank signs transaction alerts: "Spent $50 at Starbucks"
- Customer sees ✅ VERIFIED (authentic) or 🔴 UNVERIFIED (phishing)
- Prevents $10B/year in banking phishing losses

PLAID INTEGRATION:
HSIP could be bundled with Plaid's API:
- Banks use Plaid for account linking
- Banks use HSIP for secure messaging
- One integration, complete banking infrastructure

STRATEGIC VALUE:
- Upsell to existing 11,000 bank/fintech customers
- Differentiate from competitors (Yodlee, MX)
- Expand beyond data → security/trust layer

PRODUCT STATUS:
- Complete SDKs (Python, iOS, Android)
- Patent pending
- Early banking pilots

Plaid's mission is "unlock financial freedom for everyone." HSIP unlocks trust
in financial communications.

Interested in exploring acquisition?

Best,
[Your Name]
[email] | [phone]
```

---

## **Email Type 3: Creating Competitive Process**

### **Template for "We Have Multiple Offers" Email:**

```
Subject: HSIP acquisition - final round

Hi [Name],

Thanks for the interest in HSIP. Quick update:

We're finalizing acquisition discussions with 2-3 strategic acquirers. Final
round bids are due [Date - 2 weeks from now].

Current indicative offers range from $80M-150M. We're optimizing for:
1. Strategic fit (product integration potential)
2. Valuation (cash + stock mix)
3. Team retention (we want to build this inside your company)

If [Twilio/Okta/Stripe/Plaid] is seriously interested, would love to schedule:
- Technical deep dive (2 hours)
- Business metrics review (1 hour)
- Founder/exec meeting (1 hour)

Timeline:
- This week: Technical diligence
- Next week: Final offers
- Week after: Term sheet signing

Let me know if you'd like to participate in this round.

Best,
[Your Name]
```

**Note:** Only send this if you actually have multiple parties interested. Never lie in M&A.

---

## **Email Type 4: Bank Pilot Outreach**

### **Template for Digital Banks (Chime, Varo, Ally):**

```
Subject: Free pilot: Anti-phishing for your bank

Hi [Innovation Team / Head of Product],

I'm [Your Name], founder of HSIP. We built a system that shows your customers
✅ VERIFIED badges on real messages from your bank (and 🔴 warnings on phishing).

THE PROBLEM:
Your customers get phishing texts every day:
"ALERT: Suspicious charge $500. Click here to verify."

They can't tell if it's really from [Bank Name] or a scammer.

OUR SOLUTION:
1. Your backend signs every message (transaction alerts, security alerts)
2. Your mobile app verifies signatures
3. Customers see: ✅ chase.com VERIFIED or 🔴 Unverified Message

Result: Phishing attempts are instantly recognizable.

THE OFFER:
Free 3-month pilot:
- We integrate HSIP into your backend (1-2 sprints)
- We integrate into your iOS/Android apps (1 sprint)
- Test with internal team, then 1,000 beta customers
- No cost, no commitment

After pilot: $15K/year licensing (or we discuss acquisition).

WHY [BANK NAME]:
- Digital-first banks like yours have the most to gain (tech-savvy customers
  appreciate this)
- Fast decision-making (no legacy bureaucracy)
- Marketing win: "First bank to eliminate phishing"

Interested? Can we schedule 30 minutes to demo?

Best,
[Your Name]
[email] | [phone]

P.S. - Happy to sign NDA and show you the complete technical architecture.
```

---

## **Sending Strategy**

### **Week 1: Bank Pilots (Establish Traction)**
Send to:
- Chime (partnerships@chime.com)
- Varo (partnerships@varomoney.com)
- Ally Bank (innovation@ally.com)
- Cross River Bank (info@crossriver.com)

### **Week 2: VCs (Build Valuation)**
Send to:
- Andreessen Horowitz (a16z - fintech team)
- Sequoia Capital
- Greylock Partners
- Kleiner Perkins
- Bessemer Venture Partners (fintech focused)

### **Week 3: Strategic Acquirers (Create Competition)**
Send same day (creates FOMO):
- Twilio (partnerships@twilio.com, bd@twilio.com)
- Okta (bd@okta.com)
- Stripe (partnerships@stripe.com)
- Plaid (bd@plaid.com)
- Visa (innovation@visa.com)

### **Week 4: Follow-ups**
- Chase responses
- Schedule calls
- Send pitch deck to interested parties

---

## **Important Notes**

**DO:**
- Personalize each email (reference their recent acquisitions, product launches)
- Keep emails under 250 words
- Include one clear call-to-action
- Follow up exactly once after 7 days

**DON'T:**
- Mass BCC (send individually)
- Lie about traction (be honest about pilot stage)
- Send without proofreading
- Attach large files (link to deck in Google Drive instead)

**Response Rates:**
- VCs: 10-20% (1-2 out of 10 will respond)
- Strategic acquirers: 30-50% (they're always looking)
- Banks: 5-10% (enterprise sales is slow)

**If you get ONE interested party, you've won.** That's all you need to start negotiations.

---

Should I now create the pitch deck?
