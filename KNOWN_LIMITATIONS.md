# HSIP Phase 1 - Known Limitations

**Last Updated:** January 19, 2026
**Version:** 1.0.0

This document describes known limitations in HSIP Phase 1. These are **documented by design**, not bugs. They represent trade-offs between simplicity, security, and feature completeness for Phase 1.

---

## 1. Active Consent Revocation Delay

**Limitation:** When a user revokes consent for a peer, active sessions with that peer continue until natural expiry.

**Details:**
- Revoking consent immediately removes the peer from the consent cache ✅
- New connection attempts from that peer are immediately blocked ✅
- **However:** Active sessions that were established before revocation continue running
- Sessions expire after: 1 hour OR 100,000 packets (whichever comes first)

**Impact:**
- Maximum delay: 1 hour until revoked session terminates
- The peer can continue sending data during this window

**Workaround:**
- Manually stop the session listener process
- Firewall the peer's IP address at OS level
- Restart HSIP daemon to force all sessions to close

**Phase 2 Fix:**
- Add session ID tracking to ConsentCache
- Implement session manager with force-terminate capability
- Allow instant termination when consent is revoked

**Severity:** LOW - Most use cases involve consent expiry (TTL), not active revocation

---

## 2. Metadata Visibility (By Design)

**Limitation:** HSIP does not hide metadata from network observers.

**What's Visible:**
- Source and destination IP addresses
- Packet timing and sizes
- Peer IDs (linkable across sessions)
- Traffic patterns and communication frequency

**What's Hidden:**
- ✅ Message content (ChaCha20-Poly1305 encrypted)
- ✅ Consent requests/responses (Ed25519 signed)
- ✅ Session keys (ephemeral X25519, forward secrecy)

**Why This is By Design:**
- HSIP prioritizes **non-repudiation** over anonymity
- Ed25519 signatures prove who sent messages (transferable proof)
- Metadata is necessary for court evidence and GDPR compliance

**Use Cases Where This Matters:**
- ❌ Whistleblowing (use Tor + SecureDrop instead)
- ❌ Activist coordination under oppressive regimes (use Signal instead)
- ❌ Anonymous tips (use OnionShare instead)

**Use Cases Where This is Fine:**
- ✅ GDPR compliance (provable consent)
- ✅ Contract enforcement (non-repudiable signatures)
- ✅ Evidence-based dispute resolution
- ✅ Blocking unwanted contact

**Mitigation:**
- Run HSIP over Tor for metadata protection
- Combine with VPN for IP masking
- Use Signal for deniable messaging alongside HSIP for provable consent

**Severity:** N/A - This is an intentional design choice

---

## 3. Gateway Requires Manual Configuration

**Limitation:** The HTTP/HTTPS gateway (tracker blocking) requires manual browser proxy configuration.

**Current Behavior:**
- Gateway listens on 127.0.0.1:8080 ✅
- Blocks tracking domains (doubleclick.net, google-analytics.com, etc.) ✅
- Handles HTTP requests and HTTPS tunneling ✅
- **But:** User must manually set browser proxy to 127.0.0.1:8080

**What's Missing:**
- Automatic proxy detection (PAC file)
- System-wide transparent proxy
- OS-level traffic interception

**Why Not Automatic:**
- Requires OS-specific drivers (Windows WFP, Linux iptables, macOS Network Extensions)
- Complex compatibility issues across OS versions
- Phase 1 focuses on core protocol, not system integration

**Workaround:**
- Manually configure browser proxy:
  - Firefox: Settings → Network Settings → Manual Proxy → HTTP Proxy: 127.0.0.1:8080
  - Chrome: Settings → System → Proxy → HTTP: 127.0.0.1:8080
- Use browser extensions for quick proxy switching (FoxyProxy, Proxy SwitchyOmega)

**Phase 2 Fix:**
- Add PAC file generation for automatic proxy detection
- Implement SOCKS5 proxy (broader compatibility)
- Investigate OS-level integration (drivers, network extensions)

**Severity:** LOW - Manual configuration is one-time, proxy works perfectly once set

---

## 4. No Handshake Retransmission

**Limitation:** HSIP does not automatically retry failed handshakes (HELLO, E1, E2 packets).

**Details:**
- UDP is inherently unreliable (packets can be lost)
- If a HELLO or handshake packet is lost, the handshake fails
- Applications must handle retries manually

**Why This is Acceptable for Phase 1:**
- UDP is designed for unreliable transport
- Adding automatic retries requires async/tokio integration (complex)
- Most networks have low packet loss (<1%)

**Workaround:**
- Applications can retry HELLO sends after timeout
- Use TCP-based transport for reliability (future enhancement)

**Phase 2 Fix:**
- Add async handshake with configurable retry logic
- Implement exponential backoff (2s, 4s, 8s, 16s)
- Add timeout detection and automatic retry

**Severity:** LOW - Handshakes usually succeed on first attempt on stable networks

---

## 5. Dormant Security Modules

**Limitation:** Two security modules exist in the codebase but are not actively used.

**Modules:**
- `crates/hsip-net/src/rate_limiter.rs` (TokenBucket algorithm)
- `crates/hsip-net/src/connection_guard.rs` (Connection slot limits, bandwidth tracking)

**Why Not Used:**
- The `Guard` module (`crates/hsip-net/src/guard.rs`) provides equivalent protection
- Guard uses sliding window rate limiting (simpler, effective)
- Guard is actively integrated into all control-plane listeners

**Are These Bugs?**
- ❌ No - They are alternative implementations for future use
- ✅ Guard module provides active DoS protection **right now**

**What Guard Provides:**
- Per-IP rate limiting (20 handshakes per 5 seconds)
- Bad signature tracking (5 per minute before ban)
- Control frame limits (120 per minute)
- Consent request limits (30 per minute)
- Frame size validation
- IP blocklists and pinned peers

**Status:**
- Guard module is **production-ready and active**
- Dormant modules remain for potential Phase 2 enhancements
- Clearly marked with "NOT CURRENTLY INTEGRATED" comments

**Severity:** N/A - No functional impact (Guard provides protection)

---

## 6. No DNS-Level Tracker Blocking

**Limitation:** Gateway blocks trackers by domain (HTTP/HTTPS), but does not block DNS queries.

**What This Means:**
- Gateway blocks HTTP requests to google-analytics.com ✅
- **But:** DNS queries to google-analytics.com still resolve (visible to ISP)

**Why This Matters:**
- ISPs and DNS providers can see which domains you query
- Metadata leakage (who you're trying to connect to)

**Workaround:**
- Use DNS-over-HTTPS (DoH) or DNS-over-TLS (DoT) for encrypted DNS
- Use DNS providers with privacy focus (Quad9, Cloudflare 1.1.1.1)
- Configure OS-level DNS blocklists

**Phase 2 Fix:**
- Implement DNS-over-HSIP (encrypted DNS through gateway)
- Add local DNS resolver with blocklist support

**Severity:** LOW - Most users combine with DoH/DoT for DNS privacy

---

## Summary: What HSIP Phase 1 Does and Doesn't Do

### ✅ What HSIP IS (Production-Ready):
- Consent-based encrypted communication
- Court-ready evidence (non-repudiable signatures, audit logs)
- GDPR compliance (provable consent, tamper-evident records)
- DoS protection (Guard module active rate limiting)
- Tracker blocking (HTTP/HTTPS gateway)
- Blocking unwanted contact (cryptographic consent enforcement)

### ⚠️ What HSIP IS NOT (By Design):
- Anonymous communication (use Tor)
- Metadata protection (traffic analysis visible)
- Transparent system-wide proxy (requires manual config)
- Instant consent revocation (up to 1 hour delay for active sessions)

### 📋 Phase 2 Enhancements:
- Active session termination on consent revocation
- Transparent proxy (automatic browser configuration)
- DNS-over-HSIP (encrypted DNS queries)
- Handshake retransmission (automatic retry logic)
- SOCKS5 proxy support
- OS-level traffic interception (drivers)

---

## Reporting New Limitations

If you discover limitations not listed here, please report them at:
- GitHub Issues: https://github.com/nyxsystems/HSIP-1PHASE/issues
- Contact: nyxsystemsllc@gmail.com

**Please include:**
- Clear description of the limitation
- Steps to reproduce
- Expected vs actual behavior
- Use case impact (HIGH/MEDIUM/LOW severity)
