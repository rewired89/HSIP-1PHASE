# HSIP Phase 1 Threat Model

Last updated: 2026-01

This document states what HSIP Phase 1 protects against and what it does not.

---

## What HSIP Phase 1 Does

HSIP enforces consent before communication. If Alice wants to contact Bob, Bob must explicitly authorize that contact before Alice can send data. The protocol generates court-usable, tamper-evident logs of these decisions.

**Core protections:**

1. **Consent enforcement**: No peer can send application data without a cryptographically signed consent response from the recipient.

2. **Identity binding**: Peer IDs are derived from Ed25519 public keys. Signatures prevent impersonation at the protocol level.

3. **Replay prevention**: Consent requests include nonces and timestamps. Responses are cryptographically bound to their requests via BLAKE3 hashes.

4. **Session confidentiality**: Data is encrypted with ChaCha20-Poly1305 after ephemeral X25519 key exchange. Perfect forward secrecy: compromising long-term keys doesn't decrypt past sessions.

5. **Tamper-evident audit logs**: All consent decisions are recorded in a hash-chained append-only log. Chain integrity is verifiable. Exports include genesis hash, head hash, and export counter to detect selective or modified exports.

6. **Mid-session revocation**: Consent can be revoked during active connections. Sessions check consent status before encrypting or decrypting each packet.

7. **DoS resistance** (Phase 1 scope):
   - Rate limits per source IP:
     - E1 handshakes: 20 per 5 seconds
     - Bad signatures: 5 per minute
     - Control frames: 120 per minute
     - Consent requests: 30 per minute
   - Message size limits:
     - HELLO: 1024 bytes
     - Consent request: 2048 bytes
     - Consent response: 2048 bytes
     - Control frames: 4096 bytes
   - Early validation before expensive signature verification
   - IP blocklist for known tracker infrastructure
   - Timestamp checks reject stale or far-future requests

8. **Automatic key rotation**: Sessions rekey after 1 hour or 100,000 packets, whichever comes first.

---

## What HSIP Phase 1 Does Not Protect Against

HSIP is not a complete security system. It solves consent enforcement and evidence generation, not all possible threats.

### Network-level attacks

- **IP spoofing**: HSIP runs over UDP. Source IP validation depends on the network. An attacker with raw socket access can send packets with forged source IPs. Rate limits apply per observed source IP, not authenticated identity.

- **Amplification or reflection**: HSIP responses are not smaller than requests. An attacker cannot use HSIP as an amplifier, but they can still exhaust bandwidth by flooding both sides of a connection.

- **Network infrastructure compromise**: If an attacker controls routers or ISPs between peers, they can drop packets, delay handshakes, or observe encrypted traffic metadata. HSIP does not protect against traffic analysis or correlation attacks.

### Cryptographic limitations

- **Quantum computers**: HSIP uses Ed25519 and X25519, which are not quantum-safe. A sufficiently powerful quantum computer could forge signatures or decrypt past sessions.

- **Implementation bugs**: HSIP uses audited Rust cryptography libraries (ed25519-dalek, chacha20poly1305, blake3), but bugs in those libraries or in HSIP itself could compromise security.

- **Side channels**: HSIP does not protect against timing attacks, power analysis, or other side-channel attacks on endpoints.

### Endpoint security

- **Malware on endpoints**: If Alice's or Bob's machine is compromised, the attacker can read keys, forge consent, or tamper with logs. HSIP cannot protect data on a compromised endpoint.

- **Consent coercion**: HSIP enforces cryptographic consent, not human consent. If Alice forces Bob to click "allow" at gunpoint, the protocol has no way to detect this.

- **Key theft**: If an attacker steals Alice's signing key, they can impersonate Alice for future communications. HSIP does not include key revocation or certificate infrastructure in Phase 1.

### Application-layer threats

- **Content filtering**: HSIP encrypts data in transit but does not inspect or filter malicious content. Bob can consent to receive data from Alice, but HSIP does not protect Bob if that data contains malware or exploits.

- **Phishing or social engineering**: HSIP does not authenticate the human behind a peer ID. Alice can lie about who she is when requesting consent.

- **Spam or abuse after consent**: If Bob grants Alice consent, Alice can send Bob data up to the TTL expiration. HSIP does not rate-limit application data within a consented session.

### Availability

- **Targeted DoS**: Rate limits reduce impact, but a determined attacker with sufficient resources (botnets, distributed infrastructure) can still exhaust CPU, memory, or bandwidth.

- **State exhaustion**: HSIP maintains per-IP counters and per-peer session state. An attacker can force memory allocation by using many source IPs or peer IDs. The system will eventually run out of memory if attacked at scale.

- **Asymmetric resource costs**: Signature verification costs more CPU than sending garbage. Rate limits help, but an attacker can still force the defender to burn cycles on bad packets.

### Out of scope for Phase 1

These are not threats HSIP Phase 1 claims to solve:

- **Anonymity or metadata privacy**: HSIP does not hide who is talking to whom. Peer IDs, IP addresses, and traffic patterns are visible to network observers.

- **Discovery or peer finding**: HSIP does not include a directory, DHT, or discovery mechanism. Peers must already know each other's IP addresses and public keys.

- **Multi-party or group communication**: HSIP is peer-to-peer. Group chat, broadcast, or multicast is not supported.

- **Long-term key management**: HSIP does not include key rotation, revocation lists, or certificate authorities. If a key is compromised, there's no standard way to revoke it.

- **Legal enforceability outside the protocol**: HSIP generates evidence, but whether that evidence is admissible in court depends on jurisdiction, local rules, and expert testimony. HSIP does not guarantee legal outcomes.

---

## Residual Risks

Even with all protections active, HSIP has residual risks:

1. **Clock skew**: HSIP allows 5 minutes of clock skew for timestamp checks. An attacker can replay requests within that window if they intercept them.

2. **Memory limits**: Audit logs are capped at 50,000 entries in memory. Older entries are evicted. If an attacker generates enough events, early evidence may be lost unless exported to durable storage.

3. **Export integrity**: Exports include tamper-detection metadata, but HSIP does not enforce external storage or backup. If logs are deleted before export, there's no recovery.

4. **Computational cost**: Signature verification is cheap (sub-millisecond on modern CPUs), but not free. An attacker with enough volume can still cause CPU spikes even within rate limits.

5. **IPv6 address space**: Rate limits are per-IP. An attacker with a /64 IPv6 block can distribute attacks across trillions of source addresses. HSIP does not aggregate by prefix in Phase 1.

---

## Threat Actors HSIP Is Designed For

### In scope (Phase 1 targets these):

- **Amateur harassers**: Individuals sending unsolicited messages or low-volume spam. HSIP's consent layer blocks these entirely.

- **Intermediate attackers**: Small-scale DoS attempts, replay attacks, or attempts to forge consent. Rate limits and cryptographic binding prevent these.

- **Evidence collection for litigation**: Domestic violence survivors, stalking victims, or others who need court-admissible proof of who contacted them and when.

### Out of scope (Phase 1 does not claim to stop these):

- **Nation-state adversaries**: Attackers with access to backbone infrastructure, zero-day exploits, or quantum computers.

- **Large-scale DDoS**: Botnets with millions of nodes can overwhelm any single endpoint regardless of rate limits.

- **Insider threats**: Someone with legitimate access to keys or infrastructure.

---

## Testing and Validation

HSIP Phase 1 includes:

- Unit tests for cryptographic primitives
- Integration tests for handshake and consent flows
- Formal verification hooks (optional Z3-based proofs of consent non-forgery, temporal consistency, identity binding)
- Security test suite for OWASP Top 10 classes of attacks

HSIP does not include:

- Third-party security audit (not yet funded)
- Penetration testing against live deployments
- Fuzzing of wire protocol parsers (planned for Phase 2)

---

## Comparison to Other Protocols

- **vs. TLS**: HSIP enforces consent before data exchange. TLS authenticates servers but does not require recipient consent for each connection.

- **vs. Signal Protocol**: Signal provides strong endpoint security and forward secrecy for messaging. HSIP focuses on consent enforcement and evidence generation, not confidentiality of long-term conversations.

- **vs. WireGuard**: WireGuard is a VPN protocol optimized for performance. HSIP is a consent layer, not a tunneling protocol.

- **vs. Tor**: Tor provides anonymity. HSIP explicitly does not hide identities or metadata.

---

## Deployment Considerations

**Safe to use HSIP for:**

- Personal communication between mutually consenting parties who want cryptographic proof of consent
- Environments where legal evidence of consent is required (restraining orders, stalking cases)
- Testing and educational purposes

**Do not use HSIP as the sole protection for:**

- Systems requiring anonymity
- High-value targets likely to attract nation-state attackers
- Applications where consent coercion is a risk (use additional out-of-band verification)
- Safety-critical systems where availability is essential (HSIP can be DoS'd)

---

## Updates and Scope Expansion

This threat model applies to **HSIP Phase 1** as of January 2026.

Future phases may add:

- Post-quantum cryptography (reserved capability flag already exists)
- Prefix-based IPv6 rate limiting
- Distributed audit log verification
- Federation or discovery mechanisms

This document will be updated as the protocol evolves. The version number and last-updated date at the top indicate the current scope.

---

## Questions This Document Answers

**Q: Can HSIP stop a determined attacker from disrupting my communication?**
A: No. HSIP reduces the cost of attacks and makes them visible in logs, but a sufficiently resourced attacker can still cause disruption.

**Q: Is HSIP safe to use for activists under government surveillance?**
A: No. HSIP does not provide anonymity and is not designed to resist nation-state adversaries.

**Q: Will HSIP logs be admissible in court?**
A: That depends on your jurisdiction and legal representation. HSIP generates tamper-evident evidence, but admissibility is a legal question, not a technical one.

**Q: Can I use HSIP to prove someone did NOT contact me?**
A: No. HSIP only logs events that happen. Proving absence requires continuous monitoring and is outside the protocol's scope.

**Q: What happens if my signing key is stolen?**
A: An attacker can impersonate you for future communications. HSIP Phase 1 does not include key revocation. Generate a new keypair and notify your contacts out of band.

---

## Summary

HSIP Phase 1 enforces consent and generates evidence. It protects against unauthorized contact, impersonation, and replay attacks. It does not protect against nation-states, anonymity threats, or large-scale DDoS.

If your threat model requires anonymity, resistance to advanced persistent threats, or guarantees of availability under attack, HSIP Phase 1 is not sufficient by itself.

Use HSIP for what it's built to do: making consent cryptographically enforceable and generating court-ready logs of who contacted whom and when.
