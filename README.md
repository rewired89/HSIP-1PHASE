# HSIP — Your Personal Privacy Layer

**Block trackers. Send tamper-proof messages. Control your AI assistants. All on your own computer.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Build](https://github.com/rewired89/HSIP-1PHASE/actions/workflows/release.yml/badge.svg)](https://github.com/rewired89/HSIP-1PHASE/actions)

---

## Download

| Platform | File |
|----------|------|
| **Windows** | [`hsip-windows-x64.exe`](https://github.com/rewired89/HSIP-1PHASE/releases/tag/latest) |
| macOS Apple Silicon | [`hsip-macos-arm64`](https://github.com/rewired89/HSIP-1PHASE/releases/tag/latest) |
| macOS Intel | [`hsip-macos-x64`](https://github.com/rewired89/HSIP-1PHASE/releases/tag/latest) |
| Linux | [`hsip-linux-x64`](https://github.com/rewired89/HSIP-1PHASE/releases/tag/latest) |

> **Windows:** Double-click the exe. It installs itself, creates a Desktop shortcut, and opens in your browser automatically. No setup required.
>
> **Mac / Linux:** `chmod +x hsip-macos-arm64 && ./hsip-macos-arm64` — your browser opens automatically.

---

## What is HSIP?

HSIP is a privacy and identity layer that runs entirely on your computer. No cloud. No subscription. No company storing your data.

It gives you five things most people don't have:

### 1. Block trackers — system-wide, every app
Turn on the DNS Blocker and HSIP intercepts tracking requests before they reach your computer — not just in one browser, but across every app you run. Google Analytics, Facebook Pixel, Hotjar, TikTok, DoubleClick, and 200+ more never load.

### 2. Send tamper-proof messages
Every message you send through HSIP is signed with your personal cryptographic key. The signature is mathematical proof of exactly what was said and when — timestamped in a way that cannot be faked or altered. Useful in contracts, disputes, court filings, or any situation where "he said / she said" isn't good enough.

### 3. Connect your AI assistants
Connect Siri, Claude, ChatGPT, or any AI tool to HSIP. Once connected, your AI can send signed messages on your behalf by voice command:

> *"Hey Siri, send HSIP message: I confirm we agreed to these terms today."*

The message is signed with your key and timestamped instantly.

### 4. Control who has access to you
Your Consent Wallet tracks every party that has your permission to contact you or access your data. You see exactly what they can do and can revoke it in one click — no emails, no waiting, no excuses.

### 5. Cryptographic proof of everything
Every operation in HSIP — message signed, consent granted, key created — writes to a tamper-proof audit log. Your identity key is an Ed25519 keypair generated on your machine and never leaves it.

---

## Features

| Feature | What it does |
|---------|-------------|
| **DNS Tracker Blocker** | Blocks 200+ tracker domains at the network level — every app, every browser |
| **Signed Messages** | Ed25519-signed messages with cryptographic timestamps — legally verifiable proof |
| **AI Integration** | Connect Siri, Claude, any AI via API keys — send signed messages by voice |
| **Consent Wallet** | Grant and revoke access to your data from a single screen |
| **Tracker Inspector** | Look up any domain and see exactly what it does to you |
| **Message History** | Full timestamped log of every signed message — printable for court records |
| **AI Watch** | See every AI system connected to your account and disconnect any of them |
| **Identity Key** | Your own Ed25519 keypair — generated locally, never transmitted |
| **Audit Trail** | Tamper-proof log of every operation |

---

## How to connect your AI assistant

After opening HSIP, go to **AI Watch** → **Connect an AI**. Give the connection a name and copy the key that appears.

Then follow the built-in setup guide for your platform:

**Siri (iPhone / Mac)**
The setup guide walks you through creating a Siri Shortcut in 4 steps. Once done, say *"Hey Siri, Send HSIP Message"* — Siri asks what you want to say, signs it with your key, and stores it with a timestamp.

**Claude Desktop**
Copy the pre-written system prompt from the setup guide and paste it into any Claude conversation. Claude will know how to call HSIP when you ask it to send or record a message.

**Any AI with HTTP support**
Point your AI at `GET http://127.0.0.1:7777/v1/agent/capabilities` (with your Bearer key) — it returns a full machine-readable description of everything HSIP can do. Paste that into your AI's system prompt.

---

## API

HSIP runs a REST API at `http://127.0.0.1:7777`. Your API key is saved to:
- **Windows:** `%LOCALAPPDATA%\HSIP\admin.key`
- **Mac / Linux:** `~/.hsip/admin.key`

```bash
export KEY="hsip_your_key_here"

# Sign a message (creates a timestamped proof)
curl -X POST http://127.0.0.1:7777/v1/messages/sign \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{"content": "I agree to the terms discussed on this date."}'

# Get message history
curl http://127.0.0.1:7777/v1/messages \
  -H "Authorization: Bearer $KEY"

# Get AI agent capabilities spec (inject into AI system prompts)
curl http://127.0.0.1:7777/v1/agent/capabilities \
  -H "Authorization: Bearer $KEY"

# Enable the DNS tracker blocker
curl -X POST http://127.0.0.1:7777/v1/dns/enable \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{"port": 5300}'

# Grant consent to another party
curl -X POST http://127.0.0.1:7777/v1/consent/grant \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d '{"peer_verify_key": "...", "scope": "contact"}'
```

Full reference at `http://127.0.0.1:7777/docs` when HSIP is running.

---

## Build from source

```bash
# 1. Build the dashboard
cd dashboard && npm install && npm run build && cd ..

# 2. Build the binary with embedded dashboard
cargo build --release -p hsip-api --features hsip-api/embed-dashboard

# 3. Run
./target/release/hsip-api
```

Development mode (dashboard hot-reloads):
```bash
cargo run -p hsip-api          # API on :7777
cd dashboard && npm run dev    # UI on :5173
```

---

## How it's built

Everything runs locally. No data ever leaves your machine unless you explicitly send a message to someone.

```
┌──────────────────────────────────────────────────┐
│  Rust / Axum / Tokio    — async REST API          │
│  SQLite                 — local storage           │
│  Ed25519                — identity & signatures   │
│  ChaCha20-Poly1305      — key encryption at rest  │
│  hsip-dns (UDP :5300)   — DNS tracker blocker     │
│  React dashboard        — embedded in the binary  │
└──────────────────────────────────────────────────┘
```

All cryptography uses audited [RustCrypto](https://github.com/RustCrypto) libraries. The Ed25519 signing key is generated on your machine and stored encrypted — the master encryption key never leaves memory.

---

## Security

- Private keys encrypted with ChaCha20-Poly1305 + HKDF before storage
- API keys stored as SHA-256 hashes, never plaintext
- Rate limiting on all endpoints
- Full audit trail of every operation
- No telemetry, no analytics, no phone-home

To report a vulnerability: **sanchezleal1989@gmail.com**

---

## License

MIT © 2025–2026 Dayana Sanchez — use it freely. See [LICENSE](LICENSE).

---

**HSIP: Your data stays yours.**
