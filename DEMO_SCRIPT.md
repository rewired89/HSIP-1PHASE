# HSIP 3-Minute Demo Script

**For recording a video demonstration of HSIP consent flow**

**Total Time:** 3 minutes
**Goal:** Show cryptographic consent issuance, verification, and revocation
**Audience:** Technical decision-makers (CTOs, security architects, engineering leads)

---

## ⏱️ **Timeline**

- **0:00-0:30** — Introduction (30 seconds)
- **0:30-1:30** — Consent Issuance (60 seconds)
- **1:30-2:15** — Consent Verification (45 seconds)
- **2:15-2:45** — Consent Revocation (30 seconds)
- **2:45-3:00** — Call to Action (15 seconds)

---

## 📝 **Full Script**

### **[0:00-0:30] INTRODUCTION (30 seconds)**

**[Screen: HSIP README on GitHub]**

> "Hi, I'm [Your Name], creator of HSIP — an enterprise cryptographic consent management system."
>
> "Today's problem: AI agents like ChatGPT or Copilot need to prove users consented before accessing their data. Traditional consent systems rely on policy, not mathematics. They can be tampered with."
>
> "HSIP solves this with Ed25519 cryptographic signatures. Let me show you how it works in 3 minutes."

**[Transition to terminal/Postman]**

---

### **[0:30-1:30] CONSENT ISSUANCE (60 seconds)**

**[Screen: Terminal with running HSIP server showing startup logs]**

> "First, HSIP is running on localhost. It's a Rust-based REST API with PostgreSQL backend, TLS enabled, and multi-tenant support."
>
> "Let's create a cryptographic identity."

**[Execute command:]**

```powershell
# Create identity (shows tenant_id and verify_key)
Invoke-RestMethod -Method POST `
  -Uri http://localhost:3000/v1/identity `
  -Headers @{"Authorization"="Bearer $adminKey"}
```

**[Result displayed on screen - highlight the verify_key]**

> "This tenant now has an Ed25519 keypair. The verify_key is their public identity. The signing key is encrypted at rest with ChaCha20-Poly1305."
>
> "Now let's issue a consent credential. Imagine this is ChatGPT requesting access to a user's calendar."

**[Execute command:]**

```powershell
$body = '{"claim":"access_calendar","user_token":"user_alice","expires_in_days":30}'
Invoke-RestMethod -Method POST `
  -Uri http://localhost:3000/v1/credentials/issue `
  -Headers @{"Authorization"="Bearer $adminKey"; "Content-Type"="application/json"} `
  -Body $body
```

**[Result displayed - highlight the signature field]**

> "Look at this: We get a credential with an Ed25519 signature. This signature is mathematically non-repudiable — it proves Alice consented. No one can forge this, not even us."

**[Zoom in on signature field for 2 seconds]**

---

### **[1:30-2:15] CONSENT VERIFICATION (45 seconds)**

**[Screen: Same terminal or Postman]**

> "Now let's verify this credential. Imagine ChatGPT is checking if it's allowed to access Alice's calendar."

**[Execute command:]**

```powershell
$body = '{"credential_id":"<credential_id>","signature":"<signature>"}'
Invoke-RestMethod -Method POST `
  -Uri http://localhost:3000/v1/credentials/verify `
  -Headers @{"Authorization"="Bearer $adminKey"; "Content-Type"="application/json"} `
  -Body $body
```

**[Result: {"valid":true,"claim":"access_calendar",...}]**

> "Valid! Cryptographically verified in under 100 milliseconds. This credential can be verified billions of times — the signature is always valid until we explicitly revoke it."
>
> "Let's check the audit log."

**[Execute command:]**

```powershell
Invoke-RestMethod -Uri http://localhost:3000/v1/audit `
  -Headers @{"Authorization"="Bearer $adminKey"}
```

**[Result: Shows audit entries for identity creation, credential issuance, verification]**

> "Every operation is logged with timestamps. Immutable audit trail for compliance — GDPR, SOC 2, HIPAA."

---

### **[2:15-2:45] CONSENT REVOCATION (30 seconds)**

**[Screen: Same terminal]**

> "Now the critical part: What if Alice changes her mind and revokes consent?"

**[Execute command:]**

```powershell
Invoke-RestMethod -Method POST `
  -Uri "http://localhost:3000/v1/credentials/<credential_id>/revoke" `
  -Headers @{"Authorization"="Bearer $adminKey"}
```

**[Result: {"revoked":true}]**

> "Revoked. Instantly."
>
> "Let's verify again."

**[Re-run the verification command from earlier]**

**[Result: {"valid":false,"revoked":true}]**

> "Now it's invalid. ChatGPT can no longer access Alice's calendar. This revocation propagates in real-time to all API instances."

---

### **[2:45-3:00] CALL TO ACTION (15 seconds)**

**[Screen: Back to README or contact slide]**

> "That's HSIP: Cryptographic consent issuance, verification, and revocation in under 3 minutes."
>
> "Built in Rust. Production-ready. Scales to 10,000 requests per second."
>
> "If you need consent management for AI agents, APIs, or compliance, let's talk."
>
> "Email me at nyxsystemsllc@gmail.com or check out the full documentation at github.com/nyxsystems/HSIP-1PHASE."
>
> "Thanks for watching."

**[End screen with contact info:]**

```
HSIP — Cryptographic Consent Management

Email: nyxsystemsllc@gmail.com
GitHub: github.com/nyxsystems/HSIP-1PHASE
Demo available: 30-day POC trial
```

---

## 🎥 **Recording Tips**

### **Visual Setup**

1. **Terminal Font:** Use a large, readable font (18-20pt)
2. **Theme:** Use a high-contrast theme (white text on dark background)
3. **Screen Resolution:** 1920x1080 or 1280x720
4. **Zoom:** Zoom in when showing API responses
5. **Highlight:** Use cursor to point at important fields (signature, verify_key, etc.)

### **Audio**

- **Microphone:** Use a decent USB microphone (not laptop mic)
- **Quiet space:** Minimal background noise
- **Script:** Read naturally, not robotic
- **Pace:** Speak slightly slower than normal conversation

### **Editing**

- **Add captions** at key moments:
  - "Ed25519 Signature = Non-Repudiable Proof"
  - "Verified in 100ms"
  - "Revoked Instantly"

- **Add visual callouts** (arrows, boxes) pointing to:
  - Signature field in credential response
  - "valid": true in verification response
  - "revoked": true after revocation

- **Background music:** Optional light background music (keep volume low)

### **Alternative Tools**

If you don't want to use terminal commands, you can use:

1. **Postman** — Visual REST API client (easier to follow)
2. **Swagger UI** — Built-in at http://localhost:3000/docs
3. **Screen recording** — OBS Studio (free), Camtasia (paid), or Loom (online)

---

## 📤 **Where to Post**

Once recorded, post the demo video on:

1. **YouTube** — "HSIP: Enterprise Cryptographic Consent Management Demo"
2. **LinkedIn** — Share with caption targeting CTOs/security professionals
3. **Twitter/X** — Share with hashtags: #CyberSecurity #AI #Compliance #ZeroTrust
4. **Email campaigns** — Embed YouTube link in sales emails
5. **GitHub README** — Add video link to README.md

---

## 🎬 **Short vs. Long Version**

### **3-Minute Version (Above)**
- Core consent flow only
- For busy executives

### **10-Minute Extended Version** (Optional)
Add these sections:
- Multi-tenant architecture (2 min)
- High availability deployment (2 min)
- Performance benchmarks (1 min)
- Security audit results (2 min)

---

## ✅ **Pre-Recording Checklist**

- [ ] HSIP server running on localhost
- [ ] Admin key saved in `$adminKey` variable
- [ ] Terminal font size increased (18-20pt)
- [ ] High-contrast theme enabled
- [ ] Microphone tested
- [ ] Screen recording software ready
- [ ] Script printed or on second screen
- [ ] All commands tested and working
- [ ] Browser tabs closed (no distractions)
- [ ] Notifications disabled

---

## 📊 **Expected Viewer Response**

After watching, viewers should:
1. ✅ Understand what HSIP does (consent management)
2. ✅ See it working (live demo)
3. ✅ Know it's production-ready (Rust, TLS, PostgreSQL)
4. ✅ Have a clear next step (email for demo/POC)

---

**Good luck with your recording! 🎬**

If you need help or want feedback on the video, send it to nyxsystemsllc@gmail.com.
