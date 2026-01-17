# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in HSIP, please report it responsibly:

**Email:** nyxsystemsllc@gmail.com

### What to Include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if available)

### Response Timeline

- **Acknowledgment:** Within 48 hours
- **Initial Assessment:** Within 7 days
- **Fix Timeline:** 90 days for responsible disclosure

### Disclosure Policy

We follow coordinated vulnerability disclosure:

1. Report sent to security contact
2. Issue is confirmed and assessed
3. Fix is developed and tested
4. Security advisory is published
5. CVE is assigned (if applicable)

### Security Features

HSIP implements defense-in-depth across multiple layers:

**Consent Enforcement:**
- Cryptographically signed consent requests and responses (Ed25519)
- Consent responses cryptographically bound to specific requests via BLAKE3 hashes
- Mid-session consent revocation enforced on every encrypt/decrypt operation
- Consent cache with configurable TTL (default 90 days)

**Cryptographic Protections:**
- Ed25519 signatures for identity and integrity
- X25519 ephemeral key exchange for perfect forward secrecy
- ChaCha20-Poly1305 AEAD for session encryption
- BLAKE3 hashing for content addressing and audit chain integrity
- Automatic session rekey after 1 hour or 100,000 packets

**DoS and Abuse Mitigation:**
- Per-IP rate limiting:
  - E1 handshakes: 20 per 5 seconds
  - Bad signatures: 5 per minute
  - Control frames: 120 per minute
  - Consent requests: 30 per minute
- Message size limits enforced before expensive operations:
  - HELLO: 1024 bytes
  - Consent request: 2048 bytes
  - Consent response: 2048 bytes
  - Control frames: 4096 bytes
- Early validation (format, timestamps, size) before signature verification
- Timestamp freshness checks (5 minute skew tolerance, 10 minute max age)
- IP blocklist for known tracker infrastructure
- Pinning of recently-allowed peers to reduce overhead

**Audit and Evidence:**
- Hash-chained append-only audit log (BLAKE3)
- Genesis hash and head hash tracking
- Export counter to detect selective exports
- Export verification hash for tamper detection
- Integration with Observer Effect for cryptographic receipts
- All entries individually verifiable, full chain integrity provable

**Memory Safety:**
- 100% safe Rust code in core cryptographic operations (0% unsafe blocks)
- Sensitive key material zeroized on drop
- Constant-time comparisons for nonce prefixes

**Replay and Downgrade Protection:**
- Monotonic nonce counters per session
- Protocol version enforcement in HELLO messages
- Capability negotiation prevents feature downgrade
- Nonce window validation prevents replay

**Implementation Validation:**
- Optional Z3 SMT-based formal verification at startup
- Proves consent non-forgery, temporal consistency, identity binding
- Security test suite targeting OWASP Top 10 attack classes

### Security Audits

See [SECURITY_AUDIT.md](./SECURITY_AUDIT.md) for detailed security audit reports.

### Dependencies

We actively monitor dependencies for vulnerabilities using:

- `cargo-audit` (RustSec Advisory Database)
- `cargo-deny` (license and policy enforcement)
- GitHub Dependabot (if enabled)

### Scope

**In Scope:**
- Protocol implementation (hsip-core, hsip-session)
- Cryptographic operations and key handling
- Network handling and guard mechanisms (hsip-net)
- Consent enforcement and caching
- Audit log integrity
- Rate limiting and DoS mitigations
- Authentication and identity binding
- CLI tools (hsip-cli)

**Out of Scope:**
- Third-party dependencies (report to upstream maintainers)
- Large-scale distributed DoS (requires network-level defenses)
- Physical access to endpoints
- Quantum computer attacks (post-quantum crypto reserved for future phase)
- Side-channel attacks (timing, power analysis)
- Anonymity or metadata privacy (not a protocol goal)
- Social engineering or consent coercion

For a complete description of what HSIP does and does not protect against, see [THREAT_MODEL.md](./THREAT_MODEL.md).

### Hall of Fame

We recognize security researchers who responsibly disclose vulnerabilities:

*No reports yet*

---

Thank you for helping keep HSIP secure!
