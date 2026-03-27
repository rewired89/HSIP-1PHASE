# HSIP Integration SDK

**Stable extension points for third-party HSIP integrations.**

This SDK provides traits and types that allow external systems to integrate with HSIP without modifying core protocol behavior. All interfaces are generic and protocol-observable only - no platform-specific logic, demographics, or unverifiable claims.

## Stability Guarantee

This SDK follows semantic versioning. Breaking changes will increment the major version. Integrations should pin to specific HSIP tags (e.g., `hsip-integration-sdk = "0.1.0"`).

## Core Traits

### 1. `PolicyHook`

Extend consent policy evaluation with custom logic based on protocol-observable state.

```rust
use hsip_integration_sdk::{PolicyHook, ConsentRequestContext, PolicyDecision, PolicyReason};

struct StrictPolicy;

impl PolicyHook for StrictPolicy {
    fn evaluate(&self, ctx: &ConsentRequestContext) -> Option<(PolicyDecision, PolicyReason)> {
        // Deny all unknown peers in strict mode
        if ctx.unknown_peer {
            return Some((
                PolicyDecision::AutoDeny,
                PolicyReason::CustomPolicyRule {
                    rule_id: "strict_mode".into(),
                    reason: "Unknown peer denied in strict mode".into(),
                },
            ));
        }
        None // Fall through to default HSIP policy
    }
}
```

### 2. `AuditSink`

Export audit events to external storage or monitoring systems while preserving HSIP's tamper-evident log integrity.

```rust
use hsip_integration_sdk::{AuditSink, AuditEvent, AuditExport};

struct DatabaseAuditSink {
    db: Database,
}

impl AuditSink for DatabaseAuditSink {
    fn log_event(&self, event: &AuditEvent) -> Result<(), String> {
        self.db.insert("audit_log", event)
            .map_err(|e| format!("DB error: {}", e))
    }

    fn verify_chain(&self, events: &[AuditEvent]) -> Result<bool, String> {
        // Verify hash chain integrity
        for i in 1..events.len() {
            let prev = &events[i - 1];
            let curr = &events[i];
            let expected_hash = hash_event(prev);
            if curr.prev_hash != expected_hash {
                return Ok(false); // Chain broken
            }
        }
        Ok(true)
    }

    fn export(&self) -> Result<AuditExport, String> {
        // Export with tamper-detection metadata
        // ...
    }
}
```

### 3. `CapabilityProvider`

Provide opaque capability tokens for peers/sessions (e.g., file transfer, video call permissions).

```rust
use hsip_integration_sdk::{CapabilityProvider, Capability};

struct FileTransferCapabilities;

impl CapabilityProvider for FileTransferCapabilities {
    fn capabilities_for_peer(&self, peer_id: &str) -> Vec<Capability> {
        if is_trusted(peer_id) {
            vec![Capability {
                capability_id: "file_transfer".into(),
                token: sign_token(peer_id, "file_transfer"),
                expires_ms: now() + 3600_000, // 1 hour
            }]
        } else {
            vec![]
        }
    }

    fn verify_capability(
        &self,
        peer_id: &str,
        capability_id: &str,
        token: &[u8],
    ) -> Result<bool, String> {
        // Verify token signature and expiration
        // ...
    }
}
```

## Usage

See `examples/integration-minimal/` for a complete working example showing all three traits.

## What NOT to Do

- ❌ Do not encode platform-specific identity (Roblox UserID, Discord snowflake, etc.)
- ❌ Do not make unverifiable claims (age, demographics, roles beyond protocol-observable)
- ❌ Do not modify wire format or cryptographic primitives
- ❌ Do not break HSIP Phase 1 compatibility

## Litigation and Court Readiness

**CRITICAL:** This SDK preserves HSIP's tamper-evident audit trail and evidence export capabilities. Any integration MUST maintain:

- Hash-chained append-only logs
- Genesis hash, head hash, export counter integrity
- Cryptographic receipts (Observer Effect)
- Consent decision logging with timestamps and peer binding

Do not weaken or remove these features. DFF eligibility depends on them.

## Versioning

Pin to specific HSIP versions in your `Cargo.toml`:

```toml
[dependencies]
hsip-integration-sdk = "0.1.0"  # Pin to exact version
```

When HSIP Phase 1 releases new tags (e.g., `v0.2.0-phase1`), review the changelog before upgrading.

## License

Same license as HSIP Phase 1 - see repository LICENSE file.
