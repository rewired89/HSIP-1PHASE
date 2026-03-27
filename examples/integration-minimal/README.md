# Minimal HSIP Integration Example

This is a **template integration** showing how to use the HSIP Integration SDK. Copy this entire directory to your private repository and customize for your specific use case.

## What This Example Shows

1. **StrictPolicyHook**: Auto-deny unknown peers and enforce attempt limits
2. **FileAuditSink**: Log audit events to JSON files with hash chain verification
3. **SimpleCapabilityProvider**: Issue file transfer capabilities to trusted peers

## What This Example Does NOT Do

- ❌ Platform-specific logic (Roblox, Discord, etc.)
- ❌ Demographics or age verification
- ❌ Unverifiable claims or attributes
- ❌ Wire format or crypto changes

## Running the Example

```bash
cd examples/integration-minimal
cargo run
```

Expected output:

```
HSIP Integration SDK - Minimal Example

1. Policy Hook Demo:
   Decision: AutoDeny
   Reason: CustomPolicyRule { rule_id: "strict_mode", reason: "..." }

2. Audit Sink Demo:
   Logged audit event: event-001
   Export counter: 1
   Genesis hash: ...
   Head hash: ...

3. Capability Provider Demo:
   Capabilities for alice: 1 granted
      - file_transfer

Integration example complete!
```

## How to Use This Template

1. **Copy to your private repo:**
   ```bash
   cp -r examples/integration-minimal /path/to/your/repo/hsip-adapter
   ```

2. **Update Cargo.toml dependency:**
   ```toml
   [dependencies]
   hsip-integration-sdk = { git = "https://github.com/nyxsystems/HSIP-1PHASE-1", tag = "v0.2.0-phase1" }
   ```

3. **Customize the implementations:**
   - Modify `StrictPolicyHook` to match your policy requirements
   - Implement `FileAuditSink` to write to your database or logging system
   - Customize `SimpleCapabilityProvider` for your capability model

4. **Keep it generic:**
   - Base all decisions on protocol-observable state only
   - No platform-specific identity encoding
   - No unverifiable claims (age, demographics, etc.)

## Testing

Run the example tests:

```bash
cargo test
```

All tests should pass, verifying:
- Strict policy denies unknown peers
- Audit sink maintains hash chain integrity
- Capability provider grants and verifies tokens correctly

## Integration Guidelines

### Policy Hooks

Your `PolicyHook` implementation should:
- Return `None` to fall through to HSIP defaults
- Return `Some((decision, reason))` to override
- Base decisions only on `ConsentRequestContext` fields
- Execute quickly (sub-millisecond) to avoid DoS amplification

### Audit Sinks

Your `AuditSink` implementation MUST:
- Preserve hash chain integrity (`prev_hash` linkage)
- Include all tamper-detection metadata in exports
- Support chain verification via `verify_chain()`
- Be idempotent (duplicate `event_id` should not error)

### Capability Providers

Your `CapabilityProvider` implementation should:
- Issue opaque tokens (e.g., signed JWTs)
- Include expiration timestamps
- Verify token signatures in `verify_capability()`
- NOT encode platform-specific identity in tokens

## Litigation and Evidence Preservation

**CRITICAL:** Do not remove or weaken:
- Hash-chained audit logs
- Genesis/head hash tracking
- Export counter (detects selective exports)
- Signature verification

These features are required for court admissibility and DFF eligibility.

## Next Steps

After customizing this template for your needs:

1. Keep your adapter in a **private repository** (platform-specific logic should stay separate from HSIP core)
2. Pin to specific HSIP versions for stability
3. Test thoroughly with HSIP Phase 1
4. Document your integration for your team

## License

Same license as HSIP Phase 1 - see repository LICENSE file.
