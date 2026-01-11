# HSIP Daemon - API Server for Banking Integration

The HSIP daemon is a local HTTP API server that provides message authentication services for banking applications. It runs on `127.0.0.1:8787` by default and exposes endpoints for status monitoring, session management, and consent control.

## Quick Start

### 1. Build the Project

```bash
cargo build --release
```

### 2. Initialize Your Identity (First Time Only)

```bash
./target/release/hsip-cli init
```

This creates a cryptographic identity in `~/.hsip/id.json` using Ed25519 signatures.

### 3. Start the Daemon

```bash
./target/release/hsip-cli daemon
```

Or specify a custom address:

```bash
./target/release/hsip-cli daemon --status_addr 127.0.0.1:9000
```

The daemon will:
- Ensure your device identity exists
- Start an HTTP API server on the specified address
- Serve real-time metrics and control endpoints

## API Endpoints

### GET /status

Returns the current protection status and metrics.

**Example:**
```bash
curl http://127.0.0.1:8787/status
```

**Response:**
```json
{
  "protected": true,
  "active_sessions": 1,
  "egress_peer": "NYTFBVDZFNSMDASRNINFBTWZJ4",
  "cipher": "ChaCha20-Poly1305",
  "since": "2024-01-11T10:30:00Z",
  "bytes_in": 123456,
  "bytes_out": 234567,
  "path": ["Local", "HSIP", "Exit-GW-1"],
  "blocked_connections": 0,
  "blocked_ips": 0,
  "blocked_trackers": 42
}
```

### GET /sessions

Lists all active encrypted sessions.

**Example:**
```bash
curl http://127.0.0.1:8787/sessions
```

**Response:**
```json
[
  {
    "peer": "NYTFBVDZFNSMDASRNINFBTWZJ4",
    "age_secs": 42,
    "bytes_in": 11111,
    "bytes_out": 22222,
    "cipher": "ChaCha20-Poly1305"
  }
]
```

### POST /consent/grant

Grant a consent token to a peer.

**Example:**
```bash
curl -X POST http://127.0.0.1:8787/consent/grant \
  -H "Content-Type: application/json" \
  -d '{
    "grantee_pubkey_hex": "abc123...",
    "purpose": "bank_transfer",
    "expires_ms": 3600000
  }'
```

**Response:**
```json
{
  "token": "cap::abc123.../bank_transfer::3600000"
}
```

### POST /consent/revoke

Revoke consent for a specific peer.

**Example:**
```bash
curl -X POST http://127.0.0.1:8787/consent/revoke \
  -H "Content-Type: application/json" \
  -d '{
    "peer_id": "NYTFBVDZFNSMDASRNINFBTWZJ4"
  }'
```

**Response:**
```json
{
  "ok": true,
  "revoked_for": "NYTFBVDZFNSMDASRNINFBTWZJ4"
}
```

### GET /reputation/:peer_id

Get reputation score for a specific peer.

**Example:**
```bash
curl http://127.0.0.1:8787/reputation/NYTFBVDZFNSMDASRNINFBTWZJ4
```

**Response:**
```json
{
  "peer_id": "NYTFBVDZFNSMDASRNINFBTWZJ4",
  "score": 0,
  "last_seen": "2024-01-11T10:30:00Z"
}
```

## Windows Testing

Use the provided PowerShell test script:

```powershell
.\test-daemon.ps1
```

This script will:
1. Build the release binary
2. Initialize your identity (if needed)
3. Start the daemon
4. Test all API endpoints
5. Display results
6. Clean up

## Integration with Banking Apps

### Python Example

```python
import requests

# Check HSIP protection status
response = requests.get('http://127.0.0.1:8787/status')
status = response.json()

if status['protected']:
    print(f"HSIP Active - {status['active_sessions']} sessions")
    print(f"Blocked trackers: {status['blocked_trackers']}")
```

### Node.js Example

```javascript
const axios = require('axios');

async function checkStatus() {
  const response = await axios.get('http://127.0.0.1:8787/status');
  console.log('Protected:', response.data.protected);
  console.log('Active sessions:', response.data.active_sessions);
}

checkStatus();
```

### Banking SDK Integration

The daemon is designed to work with the HSIP banking SDKs:

- **Python SDK**: `sdk/python/hsip_banking/`
- **iOS SDK**: `sdk/ios/` (coming soon)
- **Android SDK**: `sdk/android/` (coming soon)

These SDKs communicate with the daemon to:
1. Sign outgoing bank messages with Ed25519 signatures
2. Verify incoming message authenticity
3. Track message delivery with non-repudiation
4. Generate court-admissible evidence logs

## Architecture

```
┌─────────────────┐
│  Banking App    │
│  (Python/iOS/   │
│   Android)      │
└────────┬────────┘
         │ HTTP
         ▼
┌─────────────────┐
│  HSIP Daemon    │
│  127.0.0.1:8787 │
│                 │
│  • /status      │
│  • /sessions    │
│  • /consent/*   │
│  • /reputation/*│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  ~/.hsip/       │
│  • id.json      │
│  • sessions/    │
│  • metrics/     │
└─────────────────┘
```

## Security Notes

1. **Local Only**: The daemon binds to `127.0.0.1` by default - it's not exposed to the network
2. **Ed25519 Signatures**: All messages are cryptographically signed
3. **No Remote Access**: The API is designed for local banking app integration only
4. **Identity Protection**: Private keys never leave `~/.hsip/id.json`

## Troubleshooting

### Port Already in Use

If port 8787 is taken, specify a different port:

```bash
./target/release/hsip-cli daemon --status_addr 127.0.0.1:9000
```

### Identity Not Found

Initialize your identity first:

```bash
./target/release/hsip-cli init
```

### Daemon Won't Start

Check if another instance is running:

```bash
# Windows
netstat -ano | findstr :8787

# Linux/macOS
lsof -i :8787
```

## Development

The daemon code is located in:
- `crates/hsip-cli/src/daemon/mod.rs` - Core daemon logic
- `crates/hsip-cli/src/main.rs` - CLI command handling

To modify endpoints, edit the Axum router in `daemon/mod.rs`:

```rust
let app = Router::new()
    .route("/status", get(get_status))
    .route("/sessions", get(get_sessions))
    // Add your custom endpoints here
    .with_state(state);
```

## Next Steps

1. **Test the daemon**: Run `./test-daemon.ps1` on Windows
2. **Integrate with Python SDK**: See `sdk/python/README.md`
3. **Build mobile SDKs**: iOS and Android integration guides coming soon
4. **Deploy to production**: Docker compose files in `docker/`

## Support

For issues or questions about the daemon:
- Check `TROUBLESHOOTING.md`
- Review the integration examples in `examples/`
- Contact support at support@hsip.io (when available)
