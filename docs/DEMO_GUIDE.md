# HSIP Demo Guide

**High Security Internet Protocol** — Identity, consent, and message verification API for platforms that handle sensitive user data.

---

## What HSIP does

HSIP gives your platform a cryptographic backbone:
- Every user or service gets a signed identity (Ed25519 keypair)
- Data exchanges require explicit consent that can be revoked at any time
- Messages are signed and verifiable — proving who sent what and when
- Every action is written to a tamper-evident audit log
- API keys support expiry, per-key rate limiting, and instant revocation

---

## Option A — Run with Docker (recommended)

**Requires:** [Docker Desktop](https://www.docker.com/products/docker-desktop/)

```bash
# 1. Unzip the package and open a terminal in the folder
# 2. Start the server
docker compose up

# The server starts at http://localhost:3000
# Your admin API key is printed in the terminal and saved to hsip_admin_key.txt
```

---

## Option B — Run the binary directly

**No installation required.** Pick the binary for your OS from the `bin/` folder:

| OS | File |
|---|---|
| Windows | `bin/hsip-api-windows.exe` |
| Linux | `bin/hsip-api-linux` |

```bash
# Linux
chmod +x bin/hsip-api-linux
./bin/hsip-api-linux

# Windows (PowerShell)
.\bin\hsip-api-windows.exe
```

The server starts on port 3000. Your admin API key is printed in the terminal and saved to `hsip_admin_key.txt`.

---

## Explore the API — no code required

Once the server is running, open your browser:

```
http://localhost:3000/docs
```

You'll see the full interactive API documentation (Swagger UI). Every endpoint is listed with request/response examples and a live "Try it out" button.

**To authenticate:**
1. Copy the API key from the terminal or `hsip_admin_key.txt`
2. Click **Authorize** (top right of the docs page)
3. Paste the key — all requests are now authenticated

---

## Try these three endpoints

### 1. Create an identity
`POST /v1/identity`

Generates an Ed25519 keypair for your tenant. Returns a `verify_key` that any third party can use to verify your signatures.

### 2. Issue a credential
`POST /v1/credentials/issue`

```json
{
  "claim": "age_over_18",
  "user_token": "user_abc123",
  "ttl_seconds": 3600
}
```

Returns a signed credential. Share it with any party who needs to verify the claim — without exposing the underlying user data.

### 3. Check the audit log
`GET /v1/audit`

Returns every action taken on this tenant. Tamper-evident, timestamped, ready for compliance reporting.

---

## API key management

You can create additional keys for different services, with optional expiry:

```json
POST /v1/keys
{
  "name": "my-backend-service",
  "agent_type": "service",
  "expires_in_days": 90
}
```

Revoke any key instantly with `DELETE /v1/keys/{id}`.

---

## Download the OpenAPI spec

```
http://localhost:3000/openapi.json
```

Use this to generate a client library in any language. See `docs/SDK_INTEGRATION.md` for step-by-step instructions.

Ready-made SDKs for **Python**, **Go**, and **Node.js** are in the `sdks/` folder.

---

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `PORT` | `3000` | Port to listen on |
| `DATABASE_URL` | SQLite file | Use `postgresql://...` for PostgreSQL |
| `RATE_LIMIT_RPM` | `300` | Max requests per minute per API key |
| `RUST_LOG` | `hsip_api=info` | Log level |

---

## Contact

For licensing, integration support, or a live walkthrough:

**[your name]**
**[your email]**
**[your company]**
