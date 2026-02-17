# SDK Integration Guide

This guide covers how to integrate HSIP into your platform using the provided SDKs or by generating a client in any language from the OpenAPI spec.

---

## Ready-made SDKs

Three SDKs are included in the `sdks/` folder. Each wraps the HSIP REST API with idiomatic code for its language.

---

### Python

**Location:** `sdks/python/`

**Install:**
```bash
pip install ./sdks/python
```

**Usage:**
```python
from hsip import HSIPClient

client = HSIPClient(
    base_url="http://localhost:3000",
    api_key="hsip_your_key_here"
)

# Create an identity
identity = client.create_identity()
print(identity["verify_key"])

# Issue a credential
cred = client.issue_credential(
    claim="age_over_18",
    user_token="user_abc123",
    ttl_seconds=3600
)

# Verify a credential
result = client.verify_credential(
    credential=cred["credential"],
    signature=cred["signature"]
)
print(result["valid"])  # True

# Grant consent to a peer
client.grant_consent(peer_verify_key="<peer_verify_key>")

# Get audit log
entries = client.get_audit_log()
```

---

### Node.js / TypeScript

**Location:** `sdks/node/`

**Install:**
```bash
npm install ./sdks/node
```

**Usage (JavaScript):**
```javascript
const { HSIPClient } = require('hsip-sdk');

const client = new HSIPClient({
  baseUrl: 'http://localhost:3000',
  apiKey: 'hsip_your_key_here'
});

// Create identity
const identity = await client.createIdentity();
console.log(identity.verify_key);

// Issue credential
const cred = await client.issueCredential({
  claim: 'age_over_18',
  user_token: 'user_abc123',
  ttl_seconds: 3600
});

// Verify credential
const result = await client.verifyCredential(cred.credential, cred.signature);
console.log(result.valid); // true

// Audit log
const entries = await client.getAuditLog();
```

**Usage (TypeScript):**
```typescript
import { HSIPClient, CredentialRecord } from 'hsip-sdk';

const client = new HSIPClient({
  baseUrl: 'http://localhost:3000',
  apiKey: 'hsip_your_key_here'
});

const cred: CredentialRecord = await client.issueCredential({
  claim: 'kyc_verified',
  user_token: 'user_xyz',
  ttl_seconds: 86400
});
```

---

### Go

**Location:** `sdks/go/`

**Install:**
```bash
go get github.com/nyxsystems/hsip-sdk-go
```

Or copy the `sdks/go/` folder into your project.

**Usage:**
```go
package main

import (
    "fmt"
    "github.com/nyxsystems/hsip-sdk-go/hsip"
)

func main() {
    client := hsip.NewClient("http://localhost:3000", "hsip_your_key_here")

    // Create identity
    identity, err := client.CreateIdentity()
    if err != nil {
        panic(err)
    }
    fmt.Println(identity.VerifyKey)

    // Grant consent
    err = client.GrantConsent("<peer_verify_key>", 0)
    if err != nil {
        panic(err)
    }

    // Get audit log
    entries, err := client.GetAuditLog()
    fmt.Printf("Audit entries: %d\n", len(entries))
}
```

---

## Generate a client in any other language

HSIP exposes a full OpenAPI 3.0 spec. You can generate a client library in **any language** in one command using `openapi-generator`.

### Step 1 — Get the spec

While the server is running:
```bash
curl http://localhost:3000/openapi.json -o hsip-openapi.json
```

Or open `http://localhost:3000/openapi.json` in your browser and save the file.

### Step 2 — Install openapi-generator

```bash
# macOS
brew install openapi-generator

# Windows (via npm)
npm install -g @openapitools/openapi-generator-cli

# Any OS (via Java JAR)
# Download from: https://openapi-generator.tech/docs/installation
```

### Step 3 — Generate your client

**Java:**
```bash
openapi-generator generate -i hsip-openapi.json -g java -o ./hsip-client-java
```

**C# / .NET:**
```bash
openapi-generator generate -i hsip-openapi.json -g csharp -o ./hsip-client-csharp
```

**PHP:**
```bash
openapi-generator generate -i hsip-openapi.json -g php -o ./hsip-client-php
```

**Ruby:**
```bash
openapi-generator generate -i hsip-openapi.json -g ruby -o ./hsip-client-ruby
```

**Swift (iOS):**
```bash
openapi-generator generate -i hsip-openapi.json -g swift5 -o ./hsip-client-swift
```

**Kotlin (Android):**
```bash
openapi-generator generate -i hsip-openapi.json -g kotlin -o ./hsip-client-kotlin
```

The generated client includes full documentation, models, and API classes ready to use.

---

## Manual integration (no SDK)

HSIP is a standard REST API. Any HTTP client works.

**Authentication:** All requests require a `Bearer` token header.

```
Authorization: Bearer hsip_your_key_here
Content-Type: application/json
```

**Base URL:** `http://your-server:3000`

### Core endpoints

| Method | Endpoint | What it does |
|---|---|---|
| `POST` | `/v1/identity` | Create or retrieve your cryptographic identity |
| `GET` | `/v1/identity` | Get your current identity and verify key |
| `POST` | `/v1/consent/grant` | Grant consent to a peer |
| `POST` | `/v1/consent/revoke` | Revoke consent from a peer |
| `GET` | `/v1/consent` | List all consent records |
| `POST` | `/v1/credentials/issue` | Issue a signed credential claim |
| `POST` | `/v1/credentials/verify` | Verify a credential signature |
| `DELETE` | `/v1/credentials/{id}/revoke` | Revoke a credential |
| `POST` | `/v1/messages/sign` | Sign a message |
| `POST` | `/v1/messages/verify` | Verify a signed message |
| `GET` | `/v1/audit` | Retrieve the audit log |
| `POST` | `/v1/keys` | Create an API key |
| `DELETE` | `/v1/keys/{id}` | Revoke an API key |
| `POST` | `/v1/tenant/erase` | GDPR Article 17 — erase all tenant data |

### Example: Python with requests

```python
import requests

BASE = "http://localhost:3000"
HEADERS = {
    "Authorization": "Bearer hsip_your_key_here",
    "Content-Type": "application/json"
}

# Create identity
r = requests.post(f"{BASE}/v1/identity", headers=HEADERS)
print(r.json())

# Issue a credential
r = requests.post(f"{BASE}/v1/credentials/issue", headers=HEADERS, json={
    "claim": "age_over_18",
    "user_token": "user_abc123",
    "ttl_seconds": 3600
})
cred = r.json()

# Verify the credential
r = requests.post(f"{BASE}/v1/credentials/verify", headers=HEADERS, json={
    "credential": cred["credential"],
    "signature": cred["signature"]
})
print(r.json()["valid"])  # True
```

### Example: Node.js with fetch

```javascript
const BASE = 'http://localhost:3000';
const HEADERS = {
  'Authorization': 'Bearer hsip_your_key_here',
  'Content-Type': 'application/json'
};

// Issue a credential
const resp = await fetch(`${BASE}/v1/credentials/issue`, {
  method: 'POST',
  headers: HEADERS,
  body: JSON.stringify({ claim: 'kyc_verified', user_token: 'u123', ttl_seconds: 86400 })
});
const cred = await resp.json();
console.log(cred.credential.id);
```

### Example: curl

```bash
KEY="hsip_your_key_here"
BASE="http://localhost:3000"

# Create identity
curl -X POST $BASE/v1/identity \
  -H "Authorization: Bearer $KEY"

# Issue credential
curl -X POST $BASE/v1/credentials/issue \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{"claim":"age_over_18","user_token":"user123","ttl_seconds":3600}'

# Audit log
curl $BASE/v1/audit \
  -H "Authorization: Bearer $KEY"
```

---

## API key types

When creating keys for different parts of your platform, use the `agent_type` field:

| Type | Use case |
|---|---|
| `human` | End users or admins accessing the API directly |
| `service` | Backend microservices or automated pipelines |
| `ai_agent` | AI/ML workloads — velocity anomaly detection is applied automatically |

```json
POST /v1/keys
{
  "name": "payments-service",
  "agent_type": "service",
  "expires_in_days": 90
}
```

---

## Interactive API explorer

The full OpenAPI spec is available in your browser while the server is running:

```
http://localhost:3000/docs
```

Every endpoint can be tested interactively without writing any code.
