# HSIP Integration SDK Overview

**Date:** 2026-01-17
**Version:** 0.1.0
**Status:** Stable, ready for third-party integrations

## Purpose

The HSIP Integration SDK provides **stable extension points** for third-party integrations without requiring modifications to HSIP Phase 1 core protocol. This allows client-specific logic (Roblox, Discord, etc.) to live in **separate private repositories** while keeping HSIP Phase 1 behavior unchanged.

## Design Principles

1. **Protocol-Observable Only**: All interfaces operate on cryptographically verifiable state, not unverifiable claims
2. **No Platform Assumptions**: Zero client-specific logic in core HSIP
3. **Litigation Preservation**: Tamper-evident audit trail features remain intact
4. **Backward Compatible**: Existing HSIP Phase 1 behavior unchanged
5. **Opt-In Extensions**: Default implementations preserve current behavior

## What Was Added

### 1. New Crate: `hsip-integration-sdk`

Location: `crates/hsip-integration-sdk/`

Provides three stable traits for external integrations:

#### `PolicyHook` - Custom Consent Policy

Extend consent decision logic based on protocol-observable state:
- Unknown peer status
- Failed attempt counts
- Rate limiting triggers
- Cryptographic identity (Peer ID)
- Timestamp validity

**Does NOT support:**
- ❌ Demographics (age, gender, location)
- ❌ Platform-specific identity (Roblox UserID, Discord snowflake)
- ❌ Unverifiable claims or attributes

#### `AuditSink` - Custom Audit Logging

Export audit events to external storage while preserving:
- Hash-chained append-only logs
- Genesis hash, head hash tracking
- Export counter (detects selective exports)
- Cryptographic signatures

**Required for:**
- ✅ Court admissibility
- ✅ DFF eligibility
- ✅ Litigation support

#### `CapabilityProvider` - Generic Capabilities

Issue opaque capability tokens for protocol-level permissions:
- File transfer capabilities
- Video call permissions
- Custom protocol extensions

**Must NOT encode:**
- ❌ Platform-specific identity
- ❌ Unverifiable claims

### 2. Example Integration: `examples/integration-minimal/`

Complete working example showing:
- `StrictPolicyHook`: Auto-deny unknown peers
- `FileAuditSink`: JSON file logging with hash chain verification
- `SimpleCapabilityProvider`: File transfer permissions

**Copy this template to your private repo** and customize for your use case.

## Integration Workflow

### For Client Integrations (Roblox, Discord, etc.)

1. **Copy the template:**
   ```bash
   cp -r examples/integration-minimal /path/to/private/repo/hsip-adapter
   ```

2. **Pin to stable HSIP version:**
   ```toml
   [dependencies]
   hsip-integration-sdk = { git = "https://github.com/nyxsystems/HSIP-1PHASE-1", tag = "v0.2.0-phase1" }
   ```

3. **Implement the traits:**
   - Customize `PolicyHook` for your consent rules
   - Implement `AuditSink` for your database/logging system
   - Add `CapabilityProvider` for your permission model

4. **Keep platform logic separate:**
   - Platform-specific identity stays in your private repo
   - No modifications to HSIP core needed
   - Update HSIP by changing the tag version

### For HSIP Core Development

**NEVER add client-specific code to HSIP Phase 1.**

If a client needs new functionality:
1. Add generic extension points to `hsip-integration-sdk` (protocol-observable only)
2. Let clients implement in their private repos
3. Keep HSIP Phase 1 stable and litigation-safe

## Versioning and Stability

### Semantic Versioning

- **Major version** (0.x.0 → 1.0.0): Breaking changes to traits
- **Minor version** (0.1.0 → 0.2.0): New traits or optional fields
- **Patch version** (0.1.0 → 0.1.1): Bug fixes, documentation

### Pinning Strategy

Integrations **MUST** pin to specific versions:

```toml
# Good: Pin to exact version or tag
hsip-integration-sdk = "0.1.0"
hsip-integration-sdk = { git = "...", tag = "v0.2.0-phase1" }

# Bad: Track latest (breaks on updates)
hsip-integration-sdk = { git = "...", branch = "main" }
```

## Testing

All tests pass:

```bash
# SDK tests
cargo test -p hsip-integration-sdk
# Result: 3/3 passed

# Example integration tests
cd examples/integration-minimal && cargo test
# Result: 3/3 passed

# HSIP Phase 1 tests (no regression)
cargo test --lib -p hsip-core
# Result: 27/27 passed
```

## Litigation and Court Readiness

**CRITICAL: The SDK preserves all audit trail features required for DFF eligibility.**

Implementations MUST maintain:
- ✅ Hash-chained append-only logs
- ✅ Genesis hash, head hash, export counter
- ✅ Cryptographic receipts (Observer Effect)
- ✅ Consent decision logging with peer binding

**Do NOT:**
- ❌ Remove hash chain verification
- ❌ Skip signature validation
- ❌ Omit tamper-detection metadata from exports

These features were explicitly verified present and functional:
- `crates/hsip-reputation/src/store.rs`: Hash-chained event log
- `THREAT_MODEL.md`: Court admissibility claims
- `TEST_PLAN.md`: Evidence export tests

## Files Changed/Added

### New Files

```
crates/hsip-integration-sdk/
├── Cargo.toml
├── README.md
└── src/
    └── lib.rs                  # Traits: PolicyHook, AuditSink, CapabilityProvider

examples/integration-minimal/
├── Cargo.toml
├── README.md
└── src/
    └── main.rs                 # Example implementations

INTEGRATION_SDK.md              # This file
```

### Modified Files

```
Cargo.toml                      # Added hsip-integration-sdk to workspace
```

### No Breaking Changes

- ✅ All existing HSIP Phase 1 tests pass
- ✅ No wire format changes
- ✅ No cryptographic primitive changes
- ✅ Default behavior unchanged (no-op implementations)

## Next Steps

### For HSIP Maintainers

1. Tag this release: `v0.2.0-phase1-sdk`
2. Document in changelog
3. Notify integration teams

### For Integration Developers

1. Copy `examples/integration-minimal/` to your private repo
2. Implement the three traits for your platform
3. Test thoroughly with HSIP Phase 1
4. Deploy your adapter separately from HSIP core

## Support

- **SDK Documentation:** `crates/hsip-integration-sdk/README.md`
- **Example Code:** `examples/integration-minimal/`
- **Issues:** https://github.com/nyxsystems/HSIP-1PHASE-1/issues

## License

Same as HSIP Phase 1 - see LICENSE file.
