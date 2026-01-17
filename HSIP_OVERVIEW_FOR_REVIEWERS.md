# HSIP Phase 1: Technical Overview for Review

**Document Version:** 1.0
**Date:** 2026-01-16
**Contact:** nyxsystemsllc@gmail.com

---

## What HSIP Is

HSIP (Human-Secure Internet Protocol) is a UDP-based transport protocol that enforces cryptographic consent before data exchange. Unlike TLS or Signal, which authenticate endpoints, HSIP requires **explicit signed authorization** from the recipient before a sender can transmit application data.

HSIP Phase 1 provides:
- Ed25519 identity (no registration required)
- Signed consent requests and responses
- ChaCha20-Poly1305 encrypted sessions
- Perfect forward secrecy via X25519 ephemeral keys
- Hash-chained tamper-evident audit logs

Phase 1 is a transport-layer consent gate. It does not provide anonymity, discovery, or application-layer features.

---

## Problem Statement

Current internet protocols prioritize connectivity over consent. A peer with your IP address can attempt connection without authorization. This enables:

**Abuse at scale:**
- Spam and harassment (no consent required)
- Unsolicited data transmission
- Resource exhaustion attacks (SYN floods, amplification)

**Evidence gaps:**
- TLS logs show "connection from X" but not whether X was authorized
- Consent is enforced by application policy, not cryptography
- Tampered logs are hard to detect in court

**Existing tools fall short:**
- Firewalls block by IP/port, not identity
- TLS authenticates servers, not peer-to-peer consent
- Signal/Matrix require central servers and phone numbers

HSIP enforces consent at the wire level. If Bob hasn't signed a consent response for Alice, Alice's packets are rejected before reaching Bob's application. Consent violations are logged with cryptographic proof.

---

## What HSIP Does Today (Phase 1 Capabilities)

### 1. Identity Without Registration
Each peer generates an Ed25519 keypair locally. Public key = identity. No email, phone, or central authority required.

### 2. Consent Enforcement
**Flow:**
1. Alice sends Bob a `ConsentRequest` containing: Alice's peer ID, purpose, TTL, timestamp, and Ed25519 signature.
2. Bob evaluates the request (manual approval, policy engine, or pre-authorized list).
3. Bob sends `ConsentResponse` signed with Bob's key, cryptographically bound to Alice's request via BLAKE3 hash.
4. If consent granted, Alice may initiate encrypted session. If denied or absent, Alice's traffic is blocked.

**Cryptographic binding:**
- Response includes BLAKE3(original request)
- Prevents attacker from reusing response for different request
- Signature verification happens before expensive crypto operations

### 3. Encrypted Sessions
After consent:
- Alice and Bob exchange ephemeral X25519 public keys (E1/E2 frames)
- Shared secret derived via X25519 Diffie-Hellman
- Session key derived via HKDF-SHA256
- All data encrypted with ChaCha20-Poly1305 AEAD
- Sessions rekey after 1 hour or 100,000 packets

**Replay protection:**
- Monotonic nonce counters per session
- Each packet authenticated (AEAD tag)
- Nonce reuse or out-of-order packets rejected

### 4. Tamper-Evident Audit Logs
Every consent decision logged:
- Decision type (Allow, Deny, Block)
- Source peer ID
- Timestamp
- Reason code
- Hash of previous entry (BLAKE3 chain)

**Tamper detection:**
- Modifying entry N breaks hash chain at entry N+1
- Genesis hash (first entry) never changes
- Export counter increments on every export (detects selective disclosure)
- Chain integrity verifiable via `AuditTrail::verify_chain()`

### 5. DoS Resistance
**Rate limiting (per source IP):**
- E1 handshakes: 20 per 5 seconds
- Bad signatures: 5 per minute
- Control frames: 120 per minute
- Consent requests: 30 per minute

**Size limits (enforced before crypto):**
- HELLO: 1KB
- Consent request/response: 2KB
- Control frames: 4KB

**Early rejection:**
- Invalid HSIP prefix dropped silently
- Timestamp checks (5-minute skew, 10-minute max age)
- Format validation before hex decode

**IP blocklist:**
- Known tracker IPs rejected immediately
- Configurable via `~/.hsip/tracker_blocklist.txt`

---

## What HSIP Does NOT Claim

### Network-Level Threats (Out of Scope)
- **IP spoofing:** HSIP runs over UDP. An attacker with raw socket access can send packets with forged source IPs. Rate limits apply per observed IP, not authenticated identity.
- **Large-scale DDoS:** A botnet with millions of nodes can overwhelm rate limits through sheer volume. HSIP is not a substitute for network-level DDoS mitigation.
- **Traffic analysis:** HSIP does not hide metadata. An observer can see packet sizes, timing, source/destination IPs.

### Cryptographic Limitations
- **Quantum computers:** HSIP uses Ed25519 and X25519 (not quantum-safe). Post-quantum crypto reserved for future phase.
- **Side channels:** No protection against timing attacks, power analysis, or other side channels.

### Endpoint Security
- **Malware:** If a peer's machine is compromised, the attacker can read keys, forge consent, or tamper with logs before export.
- **Key theft:** If Alice's signing key is stolen, attacker can impersonate Alice. Phase 1 has no revocation mechanism.
- **Coercion:** HSIP enforces cryptographic consent, not human consent. If Alice forces Bob to click "allow" at gunpoint, HSIP cannot detect this.

### Application-Layer Threats
- **Content filtering:** HSIP encrypts transport but does not inspect payload. Malware or exploits in application data are not detected.
- **Phishing:** An attacker can lie about their identity in the request purpose field. Bob must validate out-of-band.

### Privacy and Anonymity
- **Not a Tor replacement:** HSIP does not hide who is talking to whom. Peer IDs, IP addresses, and traffic patterns are visible to network observers.
- **No discovery:** HSIP does not include a DHT, directory, or peer-finding mechanism. Peers must exchange IP addresses and public keys out-of-band.

---

## Court and Litigation Value

HSIP generates evidence suitable for:
- Restraining orders (proof of contact attempt)
- Harassment cases (audit log of repeated unauthorized requests)
- Forensic investigation (tamper-evident chain shows what happened when)

**What the logs prove:**
- **Identity:** Peer ID X (derived from public key) sent request
- **Timestamp:** Request occurred at time T (within system clock skew)
- **Consent status:** Bob allowed/denied/blocked the request
- **Integrity:** Chain hash verifies log not modified after the fact

**What the logs DO NOT prove:**
- **Human identity:** Peer ID maps to cryptographic key, not person
- **Location:** IP address in log, but IPs can be spoofed or proxied
- **Content:** Audit log records consent decision, not message content
- **Absence:** Logs only show events that occurred. Cannot prove peer X "never" contacted peer Y (attacker could delete logs before export).

**Admissibility considerations:**
- HSIP logs are structured data (JSON export with metadata)
- Genesis hash, head hash, and export counter support expert testimony on integrity
- Jurisdiction-specific rules apply (consult legal counsel)
- HSIP is not a certified forensic tool (no chain-of-custody standard)

**Recommended practice for litigation:**
1. Export logs immediately when incident occurs
2. Store export in multiple locations (offline backup, notarized copy)
3. Document export counter and hashes
4. Retain raw log file until case resolved
5. Expert witness should verify chain integrity independently

---

## Threat Actor Scope

### In Scope (Phase 1 Targets These)

**Amateur harassers:**
- Individuals sending unsolicited messages
- Low-volume spam
- Single-source attacks

**Capability:** HSIP's consent layer blocks these entirely.

**Intermediate attackers:**
- Moderate-scale DoS (hundreds of requests/sec)
- Replay attacks
- Attempts to forge consent

**Capability:** Rate limits + cryptographic binding prevent most attacks. CPU usage stays manageable.

**Evidence collectors:**
- Domestic violence survivors
- Stalking victims
- Organizations needing compliance audit trails

**Capability:** Tamper-evident logs provide court-ready evidence.

### Out of Scope (Phase 1 Does Not Target)

**Nation-state adversaries:**
- Attacks with access to backbone infrastructure
- Zero-day exploits
- Quantum computers
- Traffic analysis at ISP level

**Capability:** HSIP cannot resist this level of threat. Use Tor + HSIP if nation-state surveillance is a concern.

**Large botnets:**
- Millions of nodes coordinated for DDoS
- Distributed across IPv6 /64 blocks

**Capability:** Rate limits help but cannot stop this volume. Network-level filtering required.

**Insider threats:**
- Someone with legitimate key access
- Social engineering of consent grants

**Capability:** Out of protocol scope. Operational security and key management required.

---

## Current Maturity and Completeness

### ✅ Complete (Production-Ready)

**Core cryptography:**
- Ed25519 signatures
- X25519 key exchange
- ChaCha20-Poly1305 encryption
- BLAKE3 hashing
- All cryptographic code is safe Rust (0% unsafe blocks)

**Wire protocol:**
- HELLO handshake with capability negotiation
- Consent request/response flow
- Ephemeral session establishment (E1/E2)
- Sealed data frames

**Consent enforcement:**
- Signed requests and responses
- Cryptographic binding via BLAKE3
- Consent cache with TTL
- Pre-validation before signature verification

**Session security:**
- Perfect forward secrecy
- Automatic rekey (1 hour / 100k packets)
- Replay prevention (monotonic nonces)
- Nonce exhaustion detection

**Audit logging:**
- Hash-chained append-only log
- Genesis/head hash tracking
- Export counter
- Chain verification (in code, not exposed in CLI)

**DoS mitigations:**
- Per-IP rate limiting (4 separate limits)
- Message size limits (4 different sizes)
- Early rejection mechanisms
- IP blocklist

**Testing:**
- 22 unit tests (all passing)
- 7 immediate security tests (all passing)
- Integration tests for consent flow
- Session encryption roundtrip tests

### ⚠️ Partially Complete

**CLI tooling:**
- ✅ keygen, init, hello, consent-listen, consent-send-request, session-listen, session-send
- ❌ Missing: audit-verify, audit-export, consent-revoke (code exists, not wired to CLI)

**Documentation:**
- ✅ Technical specs (wire format, handshake, security analysis)
- ✅ Threat model (what HSIP does/doesn't protect)
- ✅ Test plan (executable procedures)
- ⚠️ User-facing docs need UX clarity (see "Missing" section)

**Formal verification:**
- ✅ Z3 SMT solver hooks for consent non-forgery, temporal consistency, identity binding
- ❌ Not run in CI, no proof artifacts

### 🔴 Missing (Blocks "Production-Ready" Label)

**High-priority gaps:**
1. **Audit log CLI exposure** (3-4 hours):
   - `hsip-cli audit-verify` (verify chain, output hashes)
   - `hsip-cli audit-export` (export with metadata)
   - Unblocks tamper-detection validation

2. **Consent revocation CLI** (2-3 hours):
   - `hsip-cli consent-revoke <peer_id>` (clear from cache)
   - Unblocks mid-session revocation testing

3. **Security test tooling** (6-8 hours):
   - Bad signature generator (test CPU protection)
   - Packet replay tool (test nonce replay detection)
   - 8 tests currently blocked

4. **Third-party security audit** (weeks, requires funding):
   - No external review conducted yet
   - Cryptography implementation not independently verified

**Medium-priority gaps:**
- IPv6 prefix-based rate limiting (prevents /64 exhaustion attacks)
- Audit log persistence to disk (currently in-memory only, 50k entry cap)
- Key revocation mechanism (if key stolen, no way to invalidate)

**Low-priority / Future phase:**
- Post-quantum crypto (capability flag reserved, not implemented)
- Discovery/DHT (explicitly out of Phase 1 scope)
- Application-layer features (chat, file transfer, etc.)

---

## Next Steps (Realistic Roadmap)

### Immediate (Weeks)
1. Wire audit CLI commands to expose existing verification code
2. Add consent-revoke CLI command
3. Run all blocked security tests, document results
4. Load testing (100 concurrent sessions, measure CPU/memory)

### Short-term (Months)
5. Third-party security audit (requires funding: $15k-$30k)
6. Fuzzing of wire protocol parsers (AFL++, libFuzzer)
7. IPv6 prefix aggregation
8. Audit log rotation and disk persistence

### Medium-term (6-12 months)
9. Phase 2 planning: discovery, group chat, federation
10. Post-quantum crypto integration (ML-KEM for key exchange, ML-DSA for signatures)
11. Formal TLA+ spec of protocol state machine
12. Windows/Linux/macOS packages (currently Windows-only)

---

## User Experience Questions Answered

### "When someone tries to contact me, what happens?"

**Silently rejected (no log entry):**
- Packets without valid HSIP prefix
- Oversized messages (>4KB control frames)
- Bad timestamps (>10 minutes old or >5 minutes in future)

**Blocked with log entry (Deny/Block):**
- Consent request from peer with no prior authorization
- Consent request exceeding rate limit (>30/minute from same IP)
- Bad signatures (>5/minute from same IP)

**Auto-accepted (only if pre-configured):**
- Peer in allow-list (manual configuration)
- Consent policy engine grants based on reputation score
- Consent cached from previous grant (within TTL)

**Logged and queued for manual review:**
- First-time consent request from unknown peer (default behavior)
- Consent request with valid signature but no matching policy

**CLI/logging today (Phase 1):**
- `hsip-cli consent-listen` prints each request to terminal
- Logs written to `~/.hsip/audit.json` (not human-friendly)
- No GUI, no pop-up, no browser integration

**Future UI possibilities (Phase 2+):**
- System tray notification: "Peer X requests contact (purpose: Y)"
- Allow/Deny buttons with reason code dropdown
- History view showing past consent decisions
- Integration with desktop notification APIs

### "Does HSIP route through HTTPS?"

**No. HSIP is not HTTP or HTTPS.**

HSIP runs directly over UDP (port configurable, default varies). There is no HTTP layer, no web server, no TLS handshake.

**HSIP vs. HTTPS comparison:**

| Feature | HTTPS (HTTP over TLS) | HSIP Phase 1 |
|---------|----------------------|--------------|
| Transport | TCP | UDP |
| Encryption | TLS 1.2/1.3 | ChaCha20-Poly1305 |
| Authentication | X.509 certificates (servers only) | Ed25519 signatures (mutual) |
| Consent | None (anyone can request) | Required before data |
| Use case | Web browsing, REST APIs | Peer-to-peer consent-gated communication |

**Can HSIP tunnel through HTTPS?**

Not in Phase 1. Possible future extension:
- HTTP/3 (QUIC) uses UDP underneath (could coexist on same port)
- WebSocket tunnel (wrap HSIP packets in WebSocket frames over HTTPS)
- Fallback bridge (HTTP API that proxies to HSIP daemon)

These are **not implemented** and **not planned for Phase 1**.

**If you need HTTPS compatibility:**
- Phase 1: Use HSIP directly between peers (requires UDP reachability)
- Phase 2 (future): May include HTTPS bridge for web app integration
- Alternative: Run HSIP daemon locally, expose HTTP API, proxy via HTTPS reverse proxy (DIY, not officially supported)

**Key difference:**
HSIP enforces consent at the transport layer. HTTPS enforces it (if at all) in application logic. An HTTPS server can be contacted by anyone; an HSIP peer cannot be contacted without signed consent.

---

## Deployment Considerations

**Safe to deploy for:**
- Personal communication between mutually consenting parties
- Environments requiring cryptographic proof of consent (DV cases, stalking)
- Testing and educational use

**Do NOT deploy as sole protection for:**
- High-value targets (nation-state attackers)
- Safety-critical systems (HSIP can be DoS'd)
- Anonymity-required scenarios (use Tor instead)
- Production systems without third-party audit

**Operational requirements:**
- UDP reachability (port forwarding if behind NAT)
- Clock sync (NTP recommended, ±5min skew tolerated)
- Out-of-band key exchange (public keys distributed via Signal, email, QR code, etc.)
- Log backup (audit logs are in-memory, rotate to disk regularly)

---

## Funding and Organizational Fit

### Why HSIP Needs Funding

**Current status:**
- Alpha-quality codebase (functional but not hardened)
- No external security review
- Windows-only, no installer automation
- No user-facing applications (CLI only)

**Funding would enable:**
1. **Security audit** ($15k-$30k): Independent review of cryptographic implementation
2. **Full-time development** (3-6 months): Complete Phase 1 (CLI gaps, test tooling, docs)
3. **Cross-platform support** ($10k): Linux/macOS packages, CI for all platforms
4. **User applications** ($20k-$40k): GUI consent manager, browser extension, mobile apps

### Alignment with Fund Goals

**For DFF (Digital Freedom Fund):**
- Direct benefit to domestic violence survivors (restraining order evidence)
- Court-usable audit logs (expert witness can verify tamper-evidence)
- No central authority (peer-to-peer, no surveillance)
- Open protocol (not vendor-locked)

**For NGO/civil society:**
- Journalists protecting sources (consent prevents unwanted contact)
- Activists coordinating without central server
- Human rights defenders needing proof-of-harassment logs

**For technical reviewers:**
- Clean Rust implementation (memory-safe)
- Standard cryptography (Ed25519, ChaCha20, BLAKE3)
- Formal verification hooks (Z3 SMT solver)
- Reproducible builds (Cargo.lock pinned)

### Risk Assessment for Funders

**Technical risks:**
- No third-party audit yet (mitigate: fund audit)
- IPv6 DoS vector unmitigated (mitigate: add prefix aggregation)
- Key revocation not implemented (mitigate: add revocation lists in Phase 2)

**Adoption risks:**
- Requires UDP reachability (NAT traversal needed for widespread use)
- No discovery mechanism (users must exchange keys out-of-band)
- CLI-only (limits non-technical user adoption)

**Scope creep risks:**
- Phase 1 is deliberately minimal (no discovery, no federation, no anonymity)
- Future phases may expand scope (cost/timeline grow)
- HSIP is not a "replace the internet" project (stay focused on consent enforcement)

**Mitigation strategy:**
- Deliver Phase 1 fully before starting Phase 2
- Define success metrics (tests passing, audit complete, cross-platform builds)
- Monthly progress reports (code commits, test coverage, documentation)

---

## Summary for Decision-Makers

**HSIP Phase 1 is a cryptographic consent protocol.**

It works: 7 passing security tests, 22 unit tests, functional crypto.

It's not done: Missing CLI commands, no third-party audit, Windows-only.

It solves a real problem: Consent enforcement + litigation evidence.

It has clear limits: Not for anonymity, not DoS-proof, not quantum-safe.

Next steps are concrete: Wire CLI, run tests, fund audit, expand platforms.

Timeline to "production-ready": 3-6 months with full-time developer.

Cost to complete Phase 1: $30k-$50k (audit + dev + cross-platform).

Appropriate for funders seeking: Practical tools for abuse prevention, open protocols, evidence generation for legal cases.

Not appropriate for: Nation-state resistance, anonymity guarantees, immediate mass deployment.

---

**End of Overview**

For technical details, see:
- `THREAT_MODEL.md` (detailed threat analysis)
- `spec/` directory (wire format, handshake, security)
- `TEST_PLAN.md` (validation procedures)

For security questions: nyxsystemsllc@gmail.com
