# HSIP Consent Prompt Guide

**What this document explains:** How HSIP asks for your permission before letting someone contact you, and what information you see when making that decision.

**Audience:** Anyone using HSIP, both technical and non-technical users.

---

## The Core Concept: You Control Who Contacts You

In HSIP, **nobody can send you messages unless you've explicitly authorized them**. This isn't a spam filter that tries to guess what's unwanted - it's a cryptographic requirement enforced at the protocol level.

When someone tries to contact you for the first time:
1. Their request arrives at your HSIP endpoint
2. HSIP checks if you've already authorized this person
3. If not, HSIP decides whether to **show you a prompt**, **silently reject**, or **automatically deny**
4. You only see legitimate requests from real people - abusive traffic is filtered out before it reaches you

---

## What Happens When Someone Requests Contact

### Step 1: HSIP Pre-Filters the Request

Before you ever see a notification, HSIP automatically handles several cases:

#### **Silent Rejection (No Notification)**
These requests are dropped without logging details:
- Malformed packets (corrupted data, wrong protocol version)
- Oversized messages (potential DoS attempt)
- Invalid timestamps (replay attack attempt)
- Missing required fields

**Why silent?** Logging detailed rejections could help attackers probe your system. These aren't legitimate contact attempts.

#### **Automatic Denial (Logged but Not Shown)**
These requests are rejected with logging for your audit trail:
- **Rate limit exceeded** - Same IP sending too many requests per minute
- **Too many failed attempts** - Someone repeatedly trying after being denied
- **Previously denied peer** - You already said no to this person (if configured)
- **Policy mismatch** - Request violates your configured rules (see below)

**Why automatic?** These are either abuse patterns or violations of rules you've already set. No need to bother you repeatedly.

#### **Queued for Your Review**
Legitimate first-time requests from unknown peers are queued for you to review.

#### **Automatically Accepted**
If you've previously granted consent and it hasn't expired, the connection proceeds without prompting you again.

---

### Step 2: What You See in the Prompt

When a legitimate unknown peer requests contact, you'll see:

```
╔═══════════════════════════════════════════════════════════╗
║              Consent Request from New Peer                ║
╠═══════════════════════════════════════════════════════════╣
║                                                           ║
║  Peer ID (verified):                                      ║
║    hsip:ed25519:abc123...def789                          ║
║                                                           ║
║  Display Name (claimed, not verified):                    ║
║    "Alice Smith"                                          ║
║                                                           ║
║  Purpose (claimed):                                       ║
║    "Discussing project collaboration"                     ║
║                                                           ║
║  Request Time:                                            ║
║    2026-01-16 14:23:07 UTC                               ║
║                                                           ║
╠═══════════════════════════════════════════════════════════╣
║  [Allow]  [Deny]  [Block and Report]                     ║
╚═══════════════════════════════════════════════════════════╝
```

### What Each Field Means

#### **Peer ID (VERIFIED ✓)**
This is the **only verified piece of information**. It's a cryptographic public key that proves:
- The request is signed by the holder of this key
- Future messages from this peer will use the same identity
- You can block this specific peer ID permanently

**What it does NOT prove:**
- Who the person is in real life
- Whether they've been honest about anything else
- That they won't create a new identity and try again

#### **Display Name (CLAIMED - NOT VERIFIED ✗)**
This is whatever the requester chose to call themselves. Could be:
- Their real name
- A nickname
- A fake name
- Anything they typed

**HSIP does not verify this.** It's shown so you can recognize people you know, but assume it could be false.

#### **Purpose (CLAIMED - NOT VERIFIED ✗)**
Whatever reason they gave for wanting to contact you. Again, **not verified**. Someone could lie about their intent.

#### **Request Time (VERIFIED ✓)**
Timestamp of when the request arrived. HSIP validates this is within acceptable clock skew (prevents replay attacks), so the timestamp is **approximately trustworthy** within a few minutes.

---

## Your Decision Options

### **Allow**
- Grants consent for this peer to contact you
- Consent is valid for a configurable duration (default: 30 days)
- The peer can send messages, start sessions, transfer data
- You can revoke consent later if needed

### **Deny**
- Rejects this specific request
- The peer is NOT permanently blocked (they can request again)
- Logged in your audit trail as "denied consent request"

### **Block and Report**
- Permanently denies this Peer ID
- Future requests from this peer are automatically rejected
- Logged as "blocked peer" in audit trail
- "Report" means logged locally for your records (HSIP doesn't send reports to anyone)

---

## Policy-Based Auto-Deny Rules

You can configure policies to automatically deny certain requests **before they reach your prompt**. This is useful for:

- **Strict mode**: Deny all unknown peers (you must manually allow-list everyone)
- **Attempt limits**: Auto-deny peers with too many failed attempts
- **Previous denials**: Auto-deny peers you've denied before (no retry)

These policies are **applied after signature verification** but **before user prompts**, so legitimate but unwanted traffic is filtered out automatically.

### Example Policies

#### **Default Policy (Balanced)**
```
- Unknown peers: Queue for review
- Failed attempts limit: 5 attempts
- Previously denied: Allow retry
```

#### **Strict Policy (High Security)**
```
- Unknown peers: Auto-deny
- Failed attempts limit: 3 attempts
- Previously denied: Auto-deny (no retry)
```

#### **Permissive Policy (Low Friction)**
```
- Unknown peers: Queue for review
- Failed attempts limit: 10 attempts
- Previously denied: Allow retry
```

### Configuring Policies

Policies are set in your HSIP configuration file (`~/.hsip/config.toml`):

```toml
[consent_policy]
deny_unknown_peers = false
deny_declared_under18 = false
deny_roles = []  # Options: "Service", "Educational"
max_failed_attempts = 5
deny_previously_denied = false
```

---

## What HSIP Actually Verifies

This section clarifies **what HSIP can cryptographically prove** vs **what HSIP cannot verify**.

### ✓ HSIP DOES Verify

1. **Cryptographic Identity (Peer ID)**
   - The request was signed by the holder of the Ed25519 private key
   - Future messages from this peer will use the same key
   - The peer cannot impersonate a different Peer ID

2. **Message Integrity**
   - The request was not modified in transit
   - Timestamps are within acceptable bounds (prevents replay attacks)
   - Signatures are valid

3. **Session Encryption**
   - After consent is granted, messages are encrypted end-to-end
   - ChaCha20-Poly1305 AEAD with perfect forward secrecy
   - Sessions automatically rekey after 100,000 packets or 1 hour

4. **Audit Trail Integrity**
   - Logs are hash-chained (tampering is detectable)
   - Export metadata prevents selective log export
   - Genesis hash prevents log replacement

### ✗ HSIP DOES NOT Verify

1. **Real-World Identity**
   - HSIP has no idea who the person is in real life
   - Display names are self-asserted claims
   - No registration, no ID checking, no background checks

2. **Age Range**
   - Age declarations are self-reported, not verified
   - Do NOT rely on this for regulatory compliance (COPPA, GDPR, etc.)
   - Parental control policies are **hints for safety, not guarantees**

3. **Role / Purpose**
   - Anyone can claim to be "Educational" or "Individual"
   - No verification of intent or affiliation

4. **Location / IP Address**
   - HSIP does not verify geographic location
   - IP addresses can be spoofed (though HSIP has DoS mitigations)
   - No country-of-origin checks

5. **Content Authenticity**
   - HSIP doesn't filter for phishing, malware, or harmful content
   - Message encryption prevents inspection
   - You're responsible for evaluating message content

6. **Absence of Events**
   - Logs prove what DID happen (requests, denials, consents)
   - Logs do NOT prove what DIDN'T happen (can't prove no contact occurred)
   - Audit logs can be deleted entirely (though tampering is detectable)

---

## Safety Model and Threat Protection

### What HSIP Protects Against

#### **Spam and Unsolicited Contact**
- **Protection:** Consent requirement means no messages without your authorization
- **Limitation:** Attackers can create unlimited new identities and request consent repeatedly (rate limiting mitigates this)

#### **Harassment Campaigns**
- **Protection:** Block specific Peer IDs permanently
- **Limitation:** Harasser can generate new identities (but you can use strict policy to deny all unknown peers)

#### **Traffic Analysis by Central Servers**
- **Protection:** Peer-to-peer design, no central routing
- **Limitation:** Your ISP and network observers can still see IP-level traffic

#### **DoS Attacks (Rate Limiting)**
- **Protection:** Per-IP rate limits, packet size limits, signature pre-validation
- **Limitation:** Large botnets or IP spoofing can still overwhelm network bandwidth

#### **Replay Attacks**
- **Protection:** Timestamp validation, nonce tracking
- **Limitation:** Assumes clocks are reasonably synchronized (within a few minutes)

#### **Man-in-the-Middle Attacks (Session Layer)**
- **Protection:** Ephemeral X25519 key exchange, AEAD encryption
- **Limitation:** Initial consent request uses public key infrastructure (no out-of-band verification)

#### **Log Tampering**
- **Protection:** Hash-chained audit logs, export metadata
- **Limitation:** Entire log can be deleted (tampering individual entries is detectable, but wholesale deletion is not)

### What HSIP Does NOT Protect Against

#### **Phishing and Social Engineering**
HSIP verifies cryptographic identity, not trustworthiness. Someone with consent can still:
- Send phishing links
- Lie about their identity or intent
- Manipulate you into revealing information

**Mitigation:** Same precautions as email/messaging (verify links, don't share sensitive info)

#### **Malware and Endpoint Compromise**
If your device is infected, HSIP can't help:
- Keyloggers can steal your private keys
- Screen capture can read decrypted messages
- Malware can approve consent requests on your behalf

**Mitigation:** Keep systems updated, use antivirus, follow endpoint security best practices

#### **Network-Level Attacks**
HSIP runs over UDP, which has inherent limitations:
- **IP spoofing** - Attackers can fake source addresses (though signatures prevent impersonation)
- **DDoS** - Large botnets can flood your network interface (HSIP can't prevent bandwidth exhaustion)
- **Traffic correlation** - Observers can see when you communicate (even if content is encrypted)

**Mitigation:** Firewall rules, DDoS protection services, VPN/Tor for anonymity (separate from HSIP)

#### **Quantum Computer Attacks**
HSIP Phase 1 uses Ed25519 and X25519, which are vulnerable to quantum computers:
- Signature forgery (if quantum computers become practical)
- Session decryption (breaking X25519 key exchange)

**Mitigation:** HSIP Phase 2 will add post-quantum cryptography (ML-KEM, ML-DSA)

#### **Insider Threats**
If someone you trust and gave consent to turns malicious:
- They have valid authorization to send you messages
- You must manually revoke consent to stop them
- Past sessions may already be decrypted (depends on key compromise)

**Mitigation:** Revoke consent promptly, review audit logs regularly

#### **Legal Coercion**
HSIP cannot prevent:
- Court orders to provide private keys
- Government surveillance (if they compromise your endpoint)
- Subpoenas for audit logs

**Mitigation:** HSIP logs are local-only (not stored on servers), but if your device is seized, logs are accessible

---

## Audit Logging and Evidence

Every consent request decision is logged in `~/.hsip/audit.json` with:

- **Peer ID** (verified)
- **Your decision** (Allow, Deny, Block)
- **Timestamp** (verified)
- **Declared attributes** (logged as claims, not facts)
- **Policy decision** (if auto-denied, the reason is logged)
- **Hash chain** (each log entry references the previous entry's hash)

### What Logs Are Good For

#### **Personal Accountability**
- Review who you've granted consent to
- Check for suspicious patterns (repeated requests from new peers)
- Audit trail of your decisions

#### **Evidence in Disputes**
- Prove you denied consent to someone
- Demonstrate someone requested contact despite prior denial
- Show repeated harassment attempts (for legal proceedings)

#### **Tamper Detection**
- HSIP logs are hash-chained (modifying old entries breaks the chain)
- Export metadata prevents selective log filtering

### What Logs Are NOT Good For

#### **Proving Identity in Court**
- Logs show a **Peer ID**, not a person's real-world identity
- You'd need additional evidence linking the Peer ID to a specific individual
- Legal admissibility varies by jurisdiction

#### **Proving Absence of Contact**
- Logs show consent requests that arrived
- Logs do NOT prove no other contact attempts occurred (could have been silently rejected)

#### **Regulatory Compliance (Age Verification)**
- Declared age range is **not verified**
- Do NOT use HSIP logs as proof of age for COPPA, GDPR, or similar regulations

---

## User Experience Walkthrough

### Scenario 1: First Contact from a Known Person

**Setup:** Your friend Alice wants to message you. You haven't talked via HSIP before.

1. Alice runs: `hsip consent-send-request --to your_peer_id --display-name "Alice" --purpose "Catching up"`
2. HSIP validates the request (signature, timestamp, size)
3. Rate limiting check passes (Alice hasn't spammed you)
4. Policy check: Unknown peer, no violations → **Queue for review**
5. Your HSIP client shows prompt:
   ```
   Consent Request from New Peer
   Peer ID: hsip:ed25519:alice...
   Display Name: "Alice"
   Purpose: "Catching up"
   [Allow] [Deny] [Block]
   ```
6. You recognize Alice, click **Allow**
7. Consent is granted for 30 days (configurable)
8. Alice can now send messages, and you can reply

**Logged:** Allow decision, Alice's Peer ID, timestamp, declared attributes

### Scenario 2: Spam Request

**Setup:** Spammer tries to send you an advertisement.

1. Spammer runs: `hsip consent-send-request --to your_peer_id --display-name "CLICK HERE FOR PRIZES"`
2. HSIP validates request (passes - it's well-formed)
3. **Rate limiting check:** Spammer sent 50 requests in 1 minute → **Auto-deny**
4. You never see the request (filtered before prompt)
5. Spammer's IP is temporarily blocked from further requests

**Logged:** Rate limit exceeded, Peer ID, timestamp (no detailed spam content logged)

### Scenario 3: Harasser with Multiple Attempts

**Setup:** Someone you denied keeps trying with the same Peer ID.

1. First attempt: You see prompt, click **Deny**
2. Second attempt (1 hour later): You see prompt, click **Deny**
3. Third attempt (next day): You see prompt, click **Block and Report**
4. Fourth attempt: HSIP auto-denies (peer is permanently blocked)
5. You never see prompts from this Peer ID again

**Logged:** All attempts, your decisions, timestamps (audit trail for harassment evidence)

**Note:** If harasser generates new Peer IDs, you'd need to enable strict policy (deny all unknown peers) or rely on rate limiting to slow them down.

### Scenario 4: Parental Control Active

**Setup:** You've enabled parental control policy (auto-deny declared Under 18).

1. Unknown peer requests contact, declares age range "Under 18"
2. HSIP validates request (passes signature check)
3. **Policy check:** Declared Under 18 + parental_control policy → **Auto-deny**
4. You never see the request
5. Request is logged with reason: "AgeRangePolicyMismatch"

**Logged:** Auto-deny decision, Peer ID, claimed age range (as evidence of policy enforcement)

**Important:** This does NOT mean the requester is actually under 18 (claim is unverified). The policy blocks *claimed* minors as a safety hint, not a guarantee.

---

## Plain-Language Summary

**What is the consent prompt?**
A notification asking if you want to allow someone to contact you.

**When do I see it?**
Only when a legitimate unknown peer requests contact for the first time. Spam, abuse, and policy violations are filtered before you see them.

**What information do I get?**
- **Verified:** Cryptographic Peer ID (proves this specific key signed the request)
- **Claimed (not verified):** Display name, purpose, age range, role

**What should I trust?**
- Trust the Peer ID (it's cryptographically verified)
- Do NOT trust display name, age, role, or purpose (self-reported claims)

**What does "Allow" mean?**
The peer can send you encrypted messages for the next 30 days (or until you revoke consent).

**What does "Deny" mean?**
Reject this request, but the peer can try again later.

**What does "Block and Report" mean?**
Permanently block this Peer ID. Future requests are auto-denied. "Report" means logged locally (not sent anywhere).

**Can I change my mind?**
Yes. Revoke consent anytime with: `hsip consent-revoke --peer <peer_id>`

**Is my decision logged?**
Yes. All consent decisions are logged in `~/.hsip/audit.json` with timestamps and hash-chaining for tamper detection.

**Can I use logs as legal evidence?**
Depends on jurisdiction. Logs prove a Peer ID requested contact (not the person's real identity). Consult a lawyer for admissibility questions.

**What if someone lies about their age or name?**
HSIP doesn't verify those fields. Use the same judgment you'd use for email or social media (trust but verify, be skeptical of claims).

**What if a harasser keeps creating new identities?**
Enable strict policy (deny all unknown peers) or manually allow-list trusted contacts only.

---

## Technical Implementation Notes

(For developers integrating HSIP consent prompts)

### Data Structures

**ConsentRequestMetadata** (defined in `crates/hsip-core/src/consent.rs`):
```rust
pub struct ConsentRequestMetadata {
    pub peer_id: String,                    // Verified (Ed25519 public key)
    pub declared_attrs: DeclaredAttributes, // Unverified claims
    pub purpose: String,                    // Unverified claim
    pub timestamp_ms: u64,                  // Verified (within clock skew)
    pub flags: ConsentRequestFlags,         // System-assigned flags
}
```

**DeclaredAttributes** (all fields unverified):
```rust
pub struct DeclaredAttributes {
    pub display_name: Option<String>,
    pub age_range: DeclaredAgeRange,  // Under18 | Adult18Plus | Unknown
    pub role: DeclaredRole,           // Individual | Service | Educational | Unknown
}
```

**ConsentRequestFlags** (assigned by HSIP, not requester):
```rust
pub struct ConsentRequestFlags {
    pub unknown_peer: bool,      // First-time requester
    pub denied_before: bool,     // You denied this peer previously
    pub failed_attempts: u32,    // Count of failed attempts
    pub rate_limited: bool,      // Currently rate-limited
    pub suspicious: bool,        // Malformed or anomalous
}
```

### Policy Evaluation

**ConsentPolicy** (defined in `crates/hsip-core/src/consent_policy.rs`):
```rust
pub struct ConsentPolicy {
    pub deny_unknown_peers: bool,
    pub deny_declared_under18: bool,
    pub deny_roles: Vec<DeclaredRole>,
    pub max_failed_attempts: u32,
    pub deny_previously_denied: bool,
}

impl ConsentPolicy {
    pub fn evaluate(&self, metadata: &ConsentRequestMetadata)
        -> (PolicyDecision, PolicyReason);
}
```

**PolicyDecision** enum:
```rust
pub enum PolicyDecision {
    AutoDeny,         // Policy violation, logged
    QueueForReview,   // Show user prompt
    AutoAccept,       // Prior consent valid (checked in cache layer)
    SilentReject,     // Malformed/suspicious, minimal logging
}
```

### Integration Points

1. **Pre-validation** (`crates/hsip-core/src/consent.rs`):
   - Check signature before expensive verification
   - Reject oversized/malformed requests early

2. **Rate limiting** (`crates/hsip-net/src/guard.rs`):
   - Per-IP consent request limits
   - Automatic denial for rate-limited IPs

3. **Policy evaluation** (`crates/hsip-core/src/consent_policy.rs`):
   - Apply user-configured rules
   - Return decision + reason for logging

4. **User prompt** (CLI/GUI layer, not yet implemented):
   - Display ConsentRequestMetadata
   - Clearly mark verified vs unverified fields
   - Collect user decision (Allow/Deny/Block)

5. **Audit logging** (`crates/hsip-telemetry-guard/src/audit.rs`):
   - Log decision, Peer ID, timestamp, declared attributes, policy reason
   - Hash-chain for tamper detection

### Security Considerations for UI Developers

1. **Visual distinction:** Clearly mark verified fields (Peer ID) vs unverified (display name, age, role)
2. **Phishing resistance:** Do NOT auto-fill trust based on display name (could be spoofed)
3. **Default to deny:** If user ignores prompt, default should be deny (not allow)
4. **Rate limit UI spam:** If user sees 10+ prompts in 1 minute, show bulk deny option
5. **Revocation UI:** Make consent revocation easily accessible (don't hide in settings)

---

## Frequently Asked Questions

### **Q: Can I trust the display name?**
**A:** No. It's whatever the requester typed. Could be real, fake, or misleading. Use it as a hint to recognize people you know, but verify separately (ask them via another channel).

### **Q: What if someone claims to be under 18 but isn't?**
**A:** HSIP can't detect this. Age range is self-reported. Parental control policies block *claims* of Under 18, not actual minors. Do not rely on this for legal compliance.

### **Q: Can I see past consent decisions?**
**A:** Yes. Audit logs in `~/.hsip/audit.json` show all requests and your decisions. You can also run: `hsip audit-list` (when CLI command is implemented).

### **Q: How long does consent last?**
**A:** Default is 30 days. Configurable in `~/.hsip/config.toml`. You can revoke anytime with: `hsip consent-revoke --peer <peer_id>`

### **Q: What if I accidentally allow someone?**
**A:** Revoke immediately: `hsip consent-revoke --peer <peer_id>`. Future requests will be denied. Past sessions may already be decrypted (depends on whether they captured traffic).

### **Q: Can I allow someone temporarily (e.g., 1 hour)?**
**A:** Not yet implemented. Consent duration is global, not per-peer. Future versions may add per-peer TTL.

### **Q: What happens if I ignore a consent request?**
**A:** Current CLI: Request times out after 30 seconds, defaults to deny. Future GUI: Request queues until you decide (configurable timeout).

### **Q: Can someone bypass my policies with a new identity?**
**A:** Yes. HSIP identities are free to create. To prevent this, enable strict policy (deny all unknown peers) and manually allow-list trusted contacts.

### **Q: Are consent prompts end-to-end encrypted?**
**A:** No. Consent requests are **signed** (integrity + authenticity) but not encrypted (public key infrastructure). After consent is granted, **session messages** are encrypted.

### **Q: Can my ISP see consent requests?**
**A:** Yes. Consent requests are transmitted in plaintext (only signatures are encrypted). Your ISP can see metadata (Peer IDs, timestamps, display names). They cannot decrypt session messages after consent is granted.

### **Q: Is HSIP compliant with GDPR / COPPA / CCPA?**
**A:** HSIP is a protocol, not a service. Compliance depends on how you use it:
- **GDPR:** HSIP minimizes data collection (peer-to-peer, local logs). No central data processor.
- **COPPA:** HSIP does NOT verify age. Do not rely on declared age range for compliance.
- **CCPA:** HSIP doesn't sell data (no central authority). Local logs are under your control.

**Recommendation:** Consult a lawyer if using HSIP in a regulated context (healthcare, finance, children's services).

---

## Conclusion

HSIP's consent prompt gives you **proactive control** over who can contact you. Unlike spam filters that react after the fact, HSIP enforces consent **before the first message**.

**Key principles:**
1. **Consent is required** - No messages without your authorization
2. **Cryptographic enforcement** - Not just a filter, mathematically enforced
3. **You control the rules** - Policies auto-deny unwanted traffic before you see it
4. **Transparency** - Verified info is marked, unverified claims are labeled
5. **Audit trail** - All decisions logged with tamper detection

**What to remember:**
- Trust the Peer ID (verified)
- Don't trust display names or declared attributes (claims only)
- Use policies to auto-deny patterns you don't want
- Revoke consent if someone becomes unwanted
- Audit logs are evidence of requests/decisions, not real-world identity

**HSIP makes consent a protocol requirement, not an optional feature.**

---

**Document version:** 1.0
**Last updated:** 2026-01-16
**Related documentation:**
- `THREAT_MODEL.md` - What HSIP protects against (and doesn't)
- `HSIP_OVERVIEW_FOR_REVIEWERS.md` - Technical overview for auditors
- `SECURITY.md` - Defense-in-depth breakdown
- `spec/consent-spec.md` - Consent protocol specification
