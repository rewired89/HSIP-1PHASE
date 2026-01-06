# HMAC Integration Guide for HSIP-2PHASE

## What Was Added

A new HMAC protection module at `crates/hsip-cli/src/daemon/hmac.rs` that provides:
- `sign_response<T>(data: &T)` - Creates HMAC-SHA256 signature
- `create_signed_json<T>(data: T)` - Wraps data with signature

## How to Integrate

### 1. Add module to daemon/mod.rs

Add at the top of `crates/hsip-cli/src/daemon/mod.rs`:

```rust
mod hmac;
use hmac::{create_signed_json, SignedResponse};
```

### 2. Wrap API responses

**Before:**
```rust
async fn get_status() -> impl IntoResponse {
    Json(status)
}
```

**After:**
```rust
async fn get_status() -> impl IntoResponse {
    match create_signed_json(status) {
        Ok(json) => (StatusCode::OK, json).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
```

### 3. Update all endpoints

Apply the pattern to:
- `/status` - System status
- `/sessions` - Active sessions
- `/consent/grant` - Token issuance
- `/consent/revoke` - Session termination
- Any other API endpoints

## Response Format

All responses will now look like:
```json
{
  "data": { ... },
  "signature": "hmac-sha256-hex-signature",
  "sig_alg": "HMAC-SHA256"
}
```

## Testing

Run the security test suite:
```powershell
.\security_tests\test_hmac_complete.ps1
```

## Security Notes

- Change `RESPONSE_HMAC_KEY` in production
- Store keys in secure keystore/HSM
- Rotate keys periodically
- Clients MUST verify signatures

## OWASP Protection

This fixes OWASP A08 (Software and Data Integrity Failures):
- ✅ Response tampering detected
- ✅ MITM attacks visible to clients
- ✅ Banking-grade integrity protection
