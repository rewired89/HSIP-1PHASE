# HSIP Live Demo Guide

## Quick start (Windows)

```powershell
cd C:\Users\melas\Desktop\HSIP-1PHASE
.\DEMO_WINDOWS.ps1
```

The script handles everything: builds if needed, starts the API, walks through all 8 demo sections with pauses between each.

---

## Manual steps (if you prefer to run commands one at a time)

### 0 — Start the API

The admin key is only printed and written on **first-time setup** (when no database exists).
Always delete the database before starting to guarantee a fresh key.

```powershell
# Kill any leftover instance
Stop-Process -Name "hsip-api" -ErrorAction SilentlyContinue

# Delete DB and old key file to force first-time setup
Remove-Item .\hsip_api.db        -ErrorAction SilentlyContinue
Remove-Item .\hsip_admin_key.txt -ErrorAction SilentlyContinue

# Start — wait for the key box to appear, then Ctrl+C or leave it running
.\target\release\hsip-api.exe
```

In a second window (after the key box has appeared):

```powershell
$KEY     = Get-Content .\hsip_admin_key.txt
$headers = @{ "Authorization" = "Bearer $KEY"; "Content-Type" = "application/json" }
```

---

### 1 — Identity (offline)

```powershell
.\target\release\hsip-cli.exe keygen        # run twice to show two different keys
.\target\release\hsip-cli.exe init
.\target\release\hsip-cli.exe whoami
.\target\release\hsip-cli.exe revoke --reason "demo only"
```

---

### 2 — Consent

```powershell
"Alice wants to send this to Bob" | Out-File message.txt

.\target\release\hsip-cli.exe consent-request `
    --file message.txt --purpose "Share message" --expires-ms 60000 --out req.json

Get-Content req.json | ConvertFrom-Json | ConvertTo-Json -Depth 5

.\target\release\hsip-cli.exe consent-verify --file req.json

.\target\release\hsip-cli.exe consent-respond `
    --request req.json --decision allow --ttl-ms 60000 --out resp.json

.\target\release\hsip-cli.exe consent-verify-response --request req.json --response resp.json
```

Live UDP consent (two windows):

```powershell
# Window A
.\target\release\hsip-cli.exe consent-listen --addr 127.0.0.1:40405 --decision allow

# Window B
.\target\release\hsip-cli.exe consent-send-request `
    --to 127.0.0.1:40405 --file req.json --wait-reply
```

---

### 3 — Encrypted session (two windows)

```powershell
# Window A
.\target\release\hsip-cli.exe session-listen --addr 127.0.0.1:9002

# Window B
.\target\release\hsip-cli.exe session-send --to 127.0.0.1:9002 --packets 5
```

---

### 4 — REST API

```powershell
Invoke-RestMethod -Uri "http://localhost:3000/health"

Invoke-RestMethod -Uri "http://localhost:3000/v1/identity" -Method POST -Headers $headers

$signed = Invoke-RestMethod `
    -Uri "http://localhost:3000/v1/messages/sign" -Method POST -Headers $headers `
    -Body '{"content":"Hello HSIP World"}'
$signed | ConvertTo-Json -Depth 5

$verifyBody = @{
    content         = "Hello HSIP World"
    signature       = $signed.signature
    peer_verify_key = $signed.peer_verify_key
} | ConvertTo-Json

Invoke-RestMethod `
    -Uri "http://localhost:3000/v1/messages/verify" -Method POST -Headers $headers `
    -Body $verifyBody

Invoke-RestMethod -Uri "http://localhost:3000/v1/audit?limit=20" `
    -Headers @{ "Authorization" = "Bearer $KEY" } | ConvertTo-Json -Depth 5

Start-Process "http://localhost:3000/docs"   # Swagger UI
```

---

### 5 — Capability tokens

```powershell
# Use your pubkey from whoami
$PK = "2db4454012479fd57c860fc928a86fd3b9577762a1a3a4936e4fd03cbed7dfc2"

.\target\release\hsip-cli.exe token-issue `
    --grantee "hsip:ed25519:$PK" --caps "Voice,FileTransfer" --ttl-ms 604800000 --out cap.json

.\target\release\hsip-cli.exe token-verify --file cap.json --issuer-vk-hex $PK
```

---

### 6 — Audit tamper detection

```powershell
.\target\release\hsip-cli.exe audit-export --out audit.json --limit 50
.\target\release\hsip-cli.exe audit-verify        # PASS

# Open audit.json, change one character, save
notepad .\audit.json

.\target\release\hsip-cli.exe audit-verify        # FAIL — tamper detected
```

---

### 7 — Test suite

```powershell
cargo test --workspace 2>&1 | Select-String "test result:"
```

231+ tests, 0 failures, including real ML-KEM-768 + ML-DSA-65 operations.

---

## CLI flag reference

All flags use **dashes**, not underscores:

| Command | Required flags |
|---|---|
| `consent-request` | `--file FILE --purpose TEXT --expires-ms MS` |
| `consent-respond` | `--request FILE --decision allow\|deny --ttl-ms MS` |
| `consent-send-request` | `--to ADDR --file FILE [--wait-reply]` |
| `consent-listen` | `[--addr ADDR] [--decision allow]` |
| `token-issue` | `--grantee PEERID --caps CAPS --ttl-ms MS` |
| `token-verify` | `--file FILE --issuer-vk-hex HEX` |
| `session-listen` | `[--addr ADDR]` |
| `session-send` | `[--to ADDR] [--packets N]` |
| `audit-export` | `[--out FILE] [--limit N]` |
| `audit-verify` | _(no flags)_ |
| `keygen / init / whoami` | _(no flags)_ |
