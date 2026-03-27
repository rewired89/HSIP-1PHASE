# HSIP Threat Model

Last updated: 2026-02

This document states what HSIP protects against and what it does not. It describes the current REST API architecture as deployed in Phase 1.

---

## What HSIP Is

HSIP is a multi-tenant cryptographic consent management API. It allows organizations (tenants) to:

- Issue **Ed25519-signed credentials** proving that a user granted consent for a specific action or data access
- **Verify** those credentials cryptographically, confirming authenticity and checking revocation status
- **Revoke** credentials instantly when consent is withdrawn
- **Grant and revoke peer consent** with time-limited validity
- **Sign and verify messages** with non-repudiable Ed25519 signatures
- Generate a **tamper-evident audit trail** of all consent and identity operations

Primary use cases: AI agent authorization, API data access governance, GDPR consent enforcement, and cryptographic proof of authorization for regulated industries.

---

## System Architecture

HSIP runs as a Rust HTTP API (Axum framework) backed by either SQLite or PostgreSQL. Tenants authenticate with bearer API keys. Each tenant has an isolated Ed25519 keypair used to sign credentials and messages.

**Data isolation**: All tables are partitioned by `tenant_id`. One tenant cannot read or modify another tenant's data through the API.

**Transport security**: Production deployments use TLS (via axum-server with rustls). Credentials are only ever transmitted over encrypted channels in production configurations.

**Key protection**: Tenant signing keys are encrypted at rest using ChaCha20-Poly1305 with a server-side master key. The master key is loaded from disk at startup and never stored in the database.

---

## What HSIP Protects Against

### 1. Credential Forgery

HSIP credentials carry Ed25519 signatures over a canonical JSON payload. An attacker who intercepts or copies a credential cannot modify the `claim`, `user_token`, `issuer_verify_key`, `issued_at`, or `expires_at` fields without invalidating the signature. The verification endpoint (`POST /v1/credentials/verify`) will reject any credential with an invalid signature.

**Guarantee**: Without access to the tenant's private signing key, credential forgery is computationally infeasible.

### 2. Credential Replay After Revocation

Once a credential is revoked via `DELETE /v1/credentials/:id`, all subsequent verification requests return `revoked: true` and `valid: false`. The revocation check happens at the database level on every verify call — there is no client-side cache that could serve a stale "valid" result.

**Guarantee**: Revocation takes effect immediately on the next verification call.

### 3. Expired Credential Acceptance

Credentials carry an `expires_at` timestamp (milliseconds since epoch). The verify endpoint checks expiration on every call and returns `expired: true` if the current time exceeds `expires_at`. Expired credentials return `valid: false` regardless of signature validity.

**Guarantee**: Time-limited credentials are enforced server-side on every verification.

### 4. Identity Impersonation

Tenant identities are Ed25519 keypairs. The `verify_key` (public key) is the tenant's cryptographic identity. Messages signed by a tenant can be verified against their `verify_key` by any party. An attacker cannot produce a valid signature without the corresponding private signing key.

**Guarantee**: Tenant identity is cryptographically bound to key possession.

### 5. Unauthorized API Access

All `/v1/*` endpoints require a valid bearer API key. Keys are stored as BLAKE3 hashes in the database — the plaintext key is shown only once on creation and never stored. An attacker who reads the database cannot recover plaintext API keys.

The admin key (created at first startup) is required to provision new tenant keys. Per-tenant keys are scoped to that tenant's data only.

**Guarantee**: No cross-tenant data access is possible through the API without possession of a valid key for that tenant.

### 6. Audit Log Manipulation

Every security-relevant operation (identity creation, key rotation, credential issuance, credential revocation, consent grants and revocations, GDPR erasure) is recorded in the `audit_entries` table with action type, timestamp, and relevant peer key. Audit entries are append-only — there is no update or delete endpoint for audit records.

**Guarantee**: The audit log is a complete, append-only record of all consent and identity operations for compliance and legal evidence purposes.

### 7. Unauthorized Peer Access

The consent system (`/v1/consent`) tracks which peer public keys a tenant has explicitly granted access to, and for how long. Applications integrating HSIP can check consent status before serving data to a peer. Consents expire automatically based on the configured TTL (default: 1 hour, configurable per grant).

**Guarantee**: Consent grants are time-bounded and instantly revocable.

### 8. Non-Repudiation of Messages

Messages signed via `POST /v1/messages/sign` carry Ed25519 signatures over the message content and timestamp. The signing tenant cannot later deny having signed a message — the signature is verifiable by any party who knows the tenant's `verify_key`.

**Guarantee**: Signed messages provide cryptographic non-repudiation.

### 9. Signing Key Exposure at Rest

Tenant signing keys are encrypted using ChaCha20-Poly1305 with HKDF key derivation from the server master key before being stored in the database. An attacker who reads the database without the master key cannot recover tenant signing keys.

**Guarantee**: Database compromise alone does not expose signing keys.

---

## What HSIP Does Not Protect Against

### Endpoint Compromise

If the HSIP server itself is compromised (OS-level access, process injection), an attacker can read the master key from memory, decrypt all tenant signing keys, and forge arbitrary credentials. HSIP is not designed to resist a fully compromised host.

**Mitigation**: Use OS-level hardening, container isolation, and secrets management (e.g., HashiCorp Vault for the master key) to reduce risk.

### API Key Theft

If a tenant's API key is stolen (e.g., via credential leak, environment variable exposure, or insider access), an attacker can issue credentials, revoke existing ones, and access all data for that tenant until the key is rotated. HSIP does not detect unauthorized key use.

**Mitigation**: Use short-lived API keys (`expires_in_days`), rotate keys regularly, restrict key permissions by `agent_type`, and monitor the `/v1/audit` log for anomalous activity.

### Consent Coercion

HSIP enforces cryptographic consent, not voluntary human consent. If a user is coerced into granting consent, the credential is cryptographically valid even though the human decision was not free. HSIP cannot distinguish coerced consent from genuine consent.

**Out of scope**: Social engineering, duress, or deception leading to consent grants.

### Quantum Cryptography

HSIP uses Ed25519 (elliptic curve) and ChaCha20-Poly1305, which are not quantum-safe. A sufficiently powerful quantum computer could forge Ed25519 signatures or break key derivation. Post-quantum cryptography is reserved for a future phase (capability flag already exists in the protocol).

**Out of scope**: Quantum adversaries.

### Side-Channel Attacks

HSIP does not protect against timing attacks on signing operations, power analysis, or other hardware-level side channels. Constant-time comparisons are used for nonce validation but not uniformly throughout the codebase.

**Out of scope**: Physical or hardware-level side channels.

### Large-Scale DoS

The API includes in-memory rate limiting and request size enforcement, but a distributed botnet with sufficient volume can exhaust connection pools, CPU, or memory. HSIP is not designed to withstand nation-state-level denial of service.

**Mitigation**: Deploy behind a CDN or DDoS protection layer (Cloudflare, AWS Shield) for production.

### Network-Level Traffic Analysis

HSIP encrypts data in transit (TLS) but does not hide traffic patterns, request timing, or the fact that two parties are communicating. Observers on the network can see that a client is calling the HSIP API even if they cannot read the content.

**Out of scope**: Anonymity or metadata privacy.

### Malicious Content in Credentials

HSIP signs and verifies the `claim` and `user_token` fields as strings but does not interpret, sanitize, or validate their semantic meaning. An application can issue a credential with any claim value. HSIP does not prevent issuance of misleading or malicious claim strings.

**Mitigation**: Enforce claim validation at the application layer before calling `/v1/credentials/issue`.

### Third-Party Dependency Vulnerabilities

HSIP depends on external crates (ed25519-dalek, axum, sqlx, chacha20poly1305, blake3). Vulnerabilities in these dependencies could compromise HSIP security. Use `cargo audit` regularly and monitor the RustSec advisory database.

---

## Trust Boundaries

| Boundary | Trust Level | Notes |
|---|---|---|
| HTTPS API client → HSIP server | Authenticated | Valid bearer key required |
| HSIP server → database | Trusted | Database should not be publicly accessible |
| Master key storage → HSIP server | Critical | Compromise = all signing keys exposed |
| Tenant A ↔ Tenant B | Untrusted | Tenant isolation enforced by tenant_id scoping |
| Admin key holder | Full | Can provision all tenant keys; protect carefully |

---

## Residual Risks

1. **In-memory rate limiter**: The rate limiter resets on server restart. A burst attack timed around deployments or crashes may bypass rate limits temporarily.

2. **SQLite in development**: The default development config uses SQLite in-memory. Data is lost on restart. Do not use in-memory SQLite for production.

3. **TTL clock skew**: Expiration checks use server time. If the server clock is incorrect, credentials may expire early or late. Use NTP synchronization in production.

4. **Admin key bootstrapping**: On first run, the admin key is written to `hsip_admin_key.txt`. If this file is accessible to unauthorized parties, they gain full control. Secure file permissions immediately after first run.

5. **Audit log size**: Audit entries are stored indefinitely in the database. High-volume deployments should implement log rotation or archival to prevent unbounded storage growth.

---

## Threat Actors

### In Scope

- **Unauthorized API callers**: Clients without a valid bearer token. Blocked by authentication middleware.
- **Credential tamperers**: Attackers attempting to modify intercepted credentials. Blocked by Ed25519 signature verification.
- **Stale credential replay**: Using an expired or revoked credential. Blocked by server-side expiration and revocation checks.
- **Cross-tenant data access**: One tenant attempting to access another's data. Blocked by tenant_id scoping.
- **Audit log erasure**: Attempting to delete audit records. No delete endpoint exists for audit entries.

### Out of Scope

- **Nation-state adversaries** with access to the host OS or network infrastructure.
- **Compromised server** (root access, memory dump, or hardware-level attacks).
- **Quantum computers** capable of breaking elliptic curve cryptography.
- **Social engineering** of users into granting consent they do not intend.
- **Large-scale DDoS** exceeding the capacity of the deployment infrastructure.

---

## API Security Controls Summary

| Control | Implementation |
|---|---|
| Authentication | Bearer API keys (BLAKE3-hashed in DB) |
| Authorization | Tenant isolation via tenant_id |
| Transport security | TLS (rustls, configurable) |
| Credential integrity | Ed25519 signatures |
| Signing key protection | ChaCha20-Poly1305 encryption at rest |
| Revocation | Immediate, server-side, checked on every verify call |
| Expiration | Server-side TTL enforcement |
| Audit trail | Append-only, covers all security-relevant operations |
| Rate limiting | In-memory per-endpoint limits |
| GDPR erasure | `/v1/tenant/erase` permanently removes all tenant data |

---

## Out of Scope for Phase 1

- Post-quantum cryptography (reserved for Phase 2)
- Federated or cross-server credential verification
- Key revocation certificates or a PKI infrastructure
- Webhook notifications for revocation events
- Hardware security module (HSM) integration for master key storage
- Anonymous or privacy-preserving credential schemes

---

## Questions This Document Answers

**Q: Can a credential be forged by someone who intercepts it?**
A: No. Ed25519 signatures cannot be forged without the signing key. Intercepted credentials can be replayed (until expiration or revocation) but not modified.

**Q: What happens if a signing key is compromised?**
A: All credentials issued by that tenant become suspect. Rotate the key immediately via `POST /v1/identity/rotate`. Existing issued credentials retain the old `issuer_verify_key` and should be treated as potentially forged after a key compromise.

**Q: Can HSIP credentials be verified without calling the HSIP API?**
A: Signature verification is mathematically self-contained — any Ed25519 library can verify a credential's signature offline using the `issuer_verify_key`. However, revocation and expiration status require calling `POST /v1/credentials/verify` against the HSIP server.

**Q: Is HSIP GDPR-compliant?**
A: HSIP provides the technical primitives for GDPR compliance (cryptographic consent records, right-to-erasure via `/v1/tenant/erase`, audit trail). Whether a deployment is GDPR-compliant depends on how the application uses HSIP, not on HSIP alone.

**Q: Can the audit log be falsified?**
A: Not through the API — there is no update or delete endpoint for audit entries. A compromise of the database or server OS could allow direct database manipulation. Use database-level access controls and audit your infrastructure access separately.

---

## Summary

HSIP Phase 1 is a cryptographic consent management API. It protects against credential forgery, unauthorized data access, replay of revoked or expired credentials, and identity impersonation. It generates an append-only audit log for compliance.

HSIP does not protect against a compromised server host, stolen API keys, consent coercion, quantum adversaries, or large-scale DDoS.

Deploy HSIP behind TLS, restrict database access, secure the master key, and monitor the audit log for anomalous activity.
