# HSIP — CAIQ v4.1 Self-Assessment (Condensed)

**Document:** Cloud Security Alliance Consensus Assessments Initiative Questionnaire (CAIQv4.1)  
**Product:** High Security Internet Protocol (HSIP) API v0.2.0  
**Date:** February 2026  
**Prepared by:** HSIP Engineering Team  

---

> **Answer key:** ✅ Yes (control implemented) | ⚠️ Partial (in progress) | ❌ No (not yet implemented) | N/A (not applicable)

---

## 1. Application & Interface Security (AIS)

| # | Question | Answer | Implementation Notes |
|---|----------|--------|----------------------|
| 1 | Are there documented application security policies governing the development of the HSIP Rust crates? | ⚠️ Partial | Secure coding practices are followed and documented in ARCHITECTURE.md. A formal written policy is in progress. |
| 2 | Does the project follow a secure SDLC including code reviews and security testing? | ✅ Yes | All changes are made via Git branches with commit history. Automated integration tests run before each release (`cargo test`). See TEST_RESULTS.md. |
| 3 | How is API security enforced for the Axum-based `/v1/` endpoints? | ✅ Yes | Every protected endpoint requires `Authorization: Bearer <key>`. Keys are SHA-256 hashed before storage — raw keys are never persisted. Ed25519 signatures authenticate all identity and message operations. Input validation enforced at handler level (max lengths, type checks). |
| 4 | Is there an automated testing suite to verify security requirements before new versions are released? | ✅ Yes | 11 integration tests covering auth, identity, consent, credentials, GDPR erasure, rate limiting, and audit logging. See TEST_RESULTS.md. |
| 5 | How does the system handle traffic shaping to prevent metadata leakage or timing attacks? | ⚠️ Partial | Request body size capped at 2 MB via `RequestBodyLimitLayer`. Per-key rate limiting (HTTP 429 after configurable threshold). Full timing-attack hardening (constant-time comparison) is on the roadmap. |

---

## 2. Cryptography, Encryption & Key Management (CEK)

| # | Question | Answer | Implementation Notes |
|---|----------|--------|----------------------|
| 1 | Are cryptographic primitives implemented using industry-standard libraries? | ✅ Yes | Ed25519 signing/verification via `ed25519-dalek v2` (audited, widely deployed). SHA-256 hashing via `sha2` (RustCrypto). Both are industry-standard, memory-safe Rust crates with no unsafe blocks. |
| 2 | Can tenants generate and manage their own unique keypairs (Sovereign Identity)? | ✅ Yes | `POST /v1/identity` generates a unique Ed25519 keypair per tenant. The signing key is stored encrypted in the tenant's own database — HSIP never transmits private keys. |
| 3 | How is data protected at-rest and in-transit? | ⚠️ Partial | **At-rest:** API keys stored as SHA-256 hashes only (not plaintext). Signing keys stored as Base64-encoded bytes in tenant-controlled SQLite. **In-transit:** TLS termination is the responsibility of the deploying organization via reverse proxy (nginx, Caddy) or Kubernetes ingress (TLS config included in Helm chart). Native TLS is on the roadmap. |
| 4 | Are there processes for rotation and instant revocation of API keys and credentials? | ✅ Yes | `DELETE /v1/keys/:id` immediately deactivates a key. `DELETE /v1/credentials/:id/revoke` immediately marks a credential as revoked. Expiry dates supported via `expires_in_days` on key creation. Revocation takes effect on the next request — no caching window. |
| 5 | Is key recovery managed to ensure availability after system failure? | ⚠️ Partial | Admin key is written to `hsip_admin_key.txt` on first startup. The `hsip-regenerative` crate provides recovery mechanisms. Full automated key recovery procedure documentation is in progress. |

---

## 3. Identity & Access Management (IAM)

| # | Question | Answer | Implementation Notes |
|---|----------|--------|----------------------|
| 1 | Does the system enforce Least Privilege and Zero Trust principles? | ✅ Yes | Every API request must present a valid, active, non-expired Bearer token. There is no implicit trust between tenants. Multi-tenant isolation enforced at every database query level — one tenant cannot read another's data. |
| 2 | Is explicit, time-bound consent (TTL) required before communication? | ✅ Yes | `POST /v1/consent/grant` requires an explicit TTL (`ttl_ms`). Consent records carry `granted_at`, `expires_ms`, and `revoked_at` timestamps. Consent can be revoked instantly via `POST /v1/consent/revoke`. |
| 3 | How are identities uniquely associated with API keys and cryptographic proofs? | ✅ Yes | Each tenant has exactly one Ed25519 keypair (identity). API keys are linked to the tenant's identity at the database level. Message signatures and credential signatures are cryptographically tied to the tenant's verify key, enabling non-repudiation. |
| 4 | Is there a process to provision and deprovision access instantly? | ✅ Yes | Keys created via `POST /v1/keys` are active immediately. Keys revoked via `DELETE /v1/keys/:id` are blocked on the very next request (no session caching). In-memory velocity tracker for AI agents is also purged on revocation. |
| 5 | Does the API support multi-tenant authentication with hashed keys? | ✅ Yes | Tenant isolation is enforced at every layer. SHA-256 key hashing prevents credential theft from the database — even if the database file is stolen, raw API keys cannot be recovered. |

---

## 4. Data Security & Privacy (DSP)

| # | Question | Answer | Implementation Notes |
|---|----------|--------|----------------------|
| 1 | Is there a defined data classification and handling policy for the local-first SQLite storage model? | ⚠️ Partial | Data classification is documented in ARCHITECTURE.md (Section 4). Formal written data handling policy is in progress. |
| 2 | Does the system support the Right to Erasure (GDPR Article 17)? | ✅ Yes | `POST /v1/tenant/erase` permanently and irreversibly deletes all data for a tenant from all 7 database tables (credentials, messages, consents, identities, audit_entries, api_keys, tenants). Returns a confirmation with timestamp and list of tables cleared. |
| 3 | Is PII not leaked during claim verification (Zero-Knowledge inspired)? | ✅ Yes | Credentials carry an opaque `user_token` chosen by the issuer — HSIP never sees the underlying identity document (no passport number, SSN, or date of birth is stored). Verification confirms the Ed25519 signature is valid without revealing the subject. |
| 4 | Is data flow documented? | ✅ Yes | See ARCHITECTURE.md, Section 4: Data Flow. |
| 5 | Does the implementation use accepted methods for secure data disposal? | ⚠️ Partial | GDPR erasure endpoint (`POST /v1/tenant/erase`) performs SQL DELETE on all records. Cryptographic overwriting of SQLite pages (secure erase) is on the roadmap. |

---

## 5. Logging & Monitoring (LOG)

| # | Question | Answer | Implementation Notes |
|---|----------|--------|----------------------|
| 1 | Does the `/v1/audit` endpoint provide a timestamped log of every interaction? | ✅ Yes | Every significant action (identity creation, consent grant/revoke, message sign/verify, credential issue/verify/revoke, key creation/revocation, GDPR erasure, AI agent anomalies) generates an immutable audit entry with UUID, tenant_id, action, optional peer key, details, and Unix millisecond timestamp. |
| 2 | Are audit logs protected from unauthorized access? | ✅ Yes | Audit logs are tenant-scoped — a tenant can only read their own entries. No cross-tenant access is possible. Note: for cryptographically tamper-evident logs (hash-chained), that feature is on the roadmap. |
| 3 | Does the system integrate with Prometheus for anomaly monitoring? | ✅ Yes | `GET /metrics` exposes: `hsip_requests_total`, `hsip_auth_failures_total`, `hsip_credentials_issued_total`, `hsip_credentials_verified_total`, `hsip_agent_anomalies_total`, `hsip_active_tenants`, `hsip_messages_signed_total`. Compatible with any Prometheus/Grafana stack. |
| 4 | Does the telemetry system anonymize logging data? | ⚠️ Partial | The `hsip-telemetry-guard` crate provides metadata anonymization for transport-layer telemetry. API-layer log anonymization is in progress. |
| 5 | Is there a synchronized time source for all cryptographic timestamping? | ⚠️ Partial | Timestamps use the host system clock (millisecond precision). NTP-enforced clock synchronization is the deploying organization's responsibility. Tamper-resistant timestamping via an external time authority is on the roadmap. |

---

## 6. Governance, Risk & Compliance (GRC)

| # | Question | Answer | Implementation Notes |
|---|----------|--------|----------------------|
| 1 | Is there a formal Enterprise Risk Management (ERM) program? | ❌ No | HSIP is an early-stage product. A formal ERM program is planned as part of the SOC 2 Type II preparation roadmap. |
| 2 | Are security policies reviewed at least annually? | ⚠️ Partial | Security review is conducted before each major release. Annual formal review cycle to be established. |
| 3 | Is there a documented inventory of applicable standards (GDPR, HIPAA, etc.)? | ⚠️ Partial | GDPR (Article 17 erasure) is implemented. HIPAA and ISO 27001 applicability is being assessed. Full compliance mapping is in progress. |
| 4 | Does the project maintain an inventory of all logic crates and security dependencies? | ✅ Yes | The workspace `Cargo.toml` lists all 11 crates. `cargo tree` generates a full dependency graph. `cargo audit` can be run against the RustSec Advisory Database. See ARCHITECTURE.md, Section 6. |
| 5 | Is there a defined exception process for deviations from security configurations? | ❌ No | Exception process to be formalized as part of the compliance roadmap. |

---

## 7. Change Control & Configuration Management (CCC)

| # | Question | Answer | Implementation Notes |
|---|----------|--------|----------------------|
| 1 | Is a change control process implemented for all updates to protocol crates? | ✅ Yes | All changes made via Git branches. Commits are reviewed and must pass the integration test suite before merging. Semantic versioning applied to all crates. |
| 2 | Does the system provide rollback capability? | ⚠️ Partial | Git provides rollback of code changes. SQLite database can be restored from backup. Automated database migration rollback is not yet implemented. |
| 3 | Are configuration baselines established? | ✅ Yes | All configurable parameters are environment variables with documented defaults: `DATABASE_URL`, `PORT`, `RATE_LIMIT_RPM`, `RUST_LOG`, `HSIP_DB_PATH`. See ARCHITECTURE.md, Section 5. |
| 4 | How are changes that impact tenant environments communicated? | ⚠️ Partial | Breaking API changes are documented in version release notes. Automated tenant notification is on the roadmap. |
| 5 | Are unauthorized changes to the protocol detected and logged? | ⚠️ Partial | Runtime unauthorized access attempts are logged to the audit trail and counted in Prometheus metrics. Code-level change detection (file integrity monitoring) is the deploying organization's responsibility. |

---

## 8. Infrastructure & Virtualization Security (I&S)

| # | Question | Answer | Implementation Notes |
|---|----------|--------|----------------------|
| 1 | Is the infrastructure hardened according to best practices? | ✅ Yes | Helm chart deploys with non-root user (UID 1000), read-only root filesystem, resource limits, liveness/readiness probes. Security context enforces `allowPrivilegeEscalation: false`. |
| 2 | How is network segmentation enforced? | N/A | HSIP is a software protocol layer. Network segmentation between IT/OT environments is the deploying organization's responsibility. The Helm chart supports NetworkPolicy for Kubernetes environments. |
| 3 | Does the protocol support encryption for east-west traffic between service nodes? | ✅ Yes | All messages between nodes are Ed25519-signed. TLS for transport encryption is provided via ingress (Kubernetes) or reverse proxy configuration. |
| 4 | Are production and development environments logically separated? | ⚠️ Partial | Separate `DATABASE_URL` configuration is the deploying organization's responsibility. Environment separation guidance is included in the deployment documentation. |
| 5 | How is resource capacity monitored? | ✅ Yes | Prometheus metrics provide request rate monitoring. Container resource limits (CPU/memory) are configurable in the Helm `values.yaml`. |

---

## 9. Business Continuity & Operational Resilience (BCR)

| # | Question | Answer | Implementation Notes |
|---|----------|--------|----------------------|
| 1 | Is there a documented disaster response plan? | ❌ No | Formal disaster recovery plan is in development. |
| 2 | Are database backups performed and tested? | ⚠️ Partial | SQLite database is a single portable file — backup is straightforward (`cp hsip_api.db backup/`). The Helm chart includes a PersistentVolumeClaim. Automated scheduled backup is the deploying organization's responsibility. PostgreSQL mode (`DATABASE_URL=postgresql://...`) enables native database replication and HA. |
| 3 | Does the architecture eliminate single points of failure? | ⚠️ Partial | In SQLite mode: single-node. In PostgreSQL mode: multi-node deployment with database replication is fully supported. Kubernetes deployment via Helm supports horizontal pod scaling. |
| 4 | How does the system handle connection resilience in low-quality networks? | ⚠️ Partial | The `hsip-session` crate implements handshake retry logic. API-level retry and circuit-breaker patterns are the client's responsibility. |
| 5 | Is there a Business Impact Analysis for the identity verification engine? | ❌ No | BIA to be completed as part of the compliance roadmap. |

---

## 10. Threat & Vulnerability Management (TVM)

| # | Question | Answer | Implementation Notes |
|---|----------|--------|----------------------|
| 1 | Is there a process to identify and patch vulnerabilities in third-party Rust crates? | ✅ Yes | `cargo audit` checks all dependencies against the RustSec Advisory Database. This is run before each release. Dependabot or equivalent automated scanning is recommended for CI/CD pipelines. |
| 2 | How frequently is penetration testing conducted? | ❌ No | Independent penetration testing has not yet been conducted. This is a priority for the next compliance milestone. |
| 3 | Is there protection against DDoS and state-exhaustion attacks? | ⚠️ Partial | Per-key rate limiting (HTTP 429) and 2 MB request body cap are implemented. IP-level DDoS protection (rate limiting by IP, connection limiting) should be provided by the reverse proxy or CDN layer (nginx, Cloudflare). |
| 4 | Does the system prioritize vulnerability remediation based on risk to the root-of-trust identity? | ⚠️ Partial | Critical cryptographic dependencies (ed25519-dalek, sha2) are pinned and monitored. Formal vulnerability severity classification is in progress. |
| 5 | Are threat models built for AI agent and autonomous workflow risks? | ✅ Yes | Dedicated AI agent velocity tracking: anomaly logged at 100 req/min, auto-revocation at 1000 req/min. All anomaly events recorded in audit log and counted in Prometheus. `GET /v1/agents` provides real-time visibility into agent behavior. |

---

## 11. Security Incident Management (SEF)

| # | Question | Answer | Implementation Notes |
|---|----------|--------|----------------------|
| 1 | Is there a documented security incident response plan? | ❌ No | Incident response plan is in development as part of the compliance roadmap. |
| 2 | Are metrics for security-related events monitored? | ✅ Yes | Prometheus exports: `hsip_auth_failures_total` (by reason: missing_header, invalid_key), `hsip_agent_anomalies_total` (by event: threshold_exceeded, auto_revoked). |
| 3 | Is there a secure repository for incident records? | ❌ No | To be established. The audit log provides runtime event records. |
| 4 | How does the system handle material security breach notifications? | ❌ No | Breach notification process is not yet formalized. Deploying organizations are responsible for regulatory breach notification under GDPR Art. 33 and applicable local law. |
| 5 | Does the reputation system automatically block misbehaving nodes? | ⚠️ Partial | The `hsip-reputation` crate provides reputation scoring for P2P nodes. AI agent keys are automatically revoked upon exceeding the hard velocity threshold. Full reputation-based auto-blocking for general nodes is in progress. |

---

## 12. Supply Chain & Endpoint Management (STA/UEM)

| # | Question | Answer | Implementation Notes |
|---|----------|--------|----------------------|
| 1 | Are policies in place to manage crate-jacking / supply chain risks? | ⚠️ Partial | All dependencies are pinned via `Cargo.lock`. Only well-known, widely-audited crates are used (tokio, axum, sqlx, ed25519-dalek). `cargo audit` scans for known advisories. Formal supply chain security policy is in progress. |
| 2 | Is there a Software Bill of Materials (SBOM)? | ⚠️ Partial | `cargo metadata --format-version 1` generates a full machine-readable dependency graph. CycloneDX SBOM generation via `cargo-cyclonedx` is supported. |
| 3 | How is the security of third-party client SDKs managed? | N/A | HSIP provides an OpenAPI 3.0 spec (`/openapi.json`). Client SDK generation from this spec (using openapi-generator) is the customer's responsibility. Generated SDKs should be reviewed by the customer's security team. |
| 4 | Are there measures to ensure only approved devices can initiate connections? | ⚠️ Partial | Access is controlled via API key authentication. IP allowlisting can be enforced at the network/proxy layer. Device certificate-based authentication is on the roadmap. |
| 5 | Does the system allow remote revocation of access if an endpoint device is compromised? | ✅ Yes | `DELETE /v1/keys/:id` immediately and permanently revokes any API key. Takes effect on the next request with no caching delay. For AI agents, auto-revocation also occurs automatically if velocity thresholds are exceeded. |

---

## Summary

| Domain | ✅ Yes | ⚠️ Partial | ❌ No | N/A |
|--------|--------|------------|-------|-----|
| AIS — Application Security | 3 | 2 | 0 | 0 |
| CEK — Cryptography | 3 | 2 | 0 | 0 |
| IAM — Identity & Access | 5 | 0 | 0 | 0 |
| DSP — Data Privacy | 3 | 2 | 0 | 0 |
| LOG — Logging | 3 | 2 | 0 | 0 |
| GRC — Governance | 1 | 2 | 2 | 0 |
| CCC — Change Control | 3 | 2 | 0 | 0 |
| I&S — Infrastructure | 3 | 1 | 0 | 1 |
| BCR — Business Continuity | 0 | 3 | 2 | 0 |
| TVM — Threat Management | 2 | 2 | 1 | 0 |
| SEF — Incident Management | 1 | 1 | 3 | 0 |
| STA — Supply Chain | 1 | 2 | 0 | 2 |
| **Total** | **28** | **23** | **8** | **3** |

**Controls fully implemented: 28/59 (47%)**  
**Controls partially implemented: 23/59 (39%)**  
**Total coverage (Yes + Partial): 51/59 (86%)**

---

*HSIP is actively pursuing SOC 2 Type II certification. The items marked ❌ are documented in our compliance roadmap and are being addressed in order of risk priority.*
