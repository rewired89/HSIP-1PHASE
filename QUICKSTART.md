# HSIP Quick Start Guide

**Get HSIP working in 15 minutes**

Eliminate phishing attacks by showing ✅ **VERIFIED** or 🔴 **UNVERIFIED** badges on all bank messages.

---

## 30-Second Overview

**What HSIP does:** Banks sign messages → Customers verify signatures → Phishing attempts show red 🔴 warning

**What you need:**
1. Python backend to sign messages
2. iOS/Android apps to verify signatures

---

## Step 1: Generate Keys (2 minutes)

```bash
# Install Python SDK
pip install hsip-bank-sdk

# Generate bank keypair
hsip-keygen --domain yourbank.com --output ./keys
```

**Output:**
```
✅ Generated keypair for yourbank.com
   Private key: keys/yourbank.com_private_key.pem  # Keep SECRET
   Public key:  keys/yourbank.com_public_key.pem   # Give to mobile apps
```

**⚠️ CRITICAL:** Store private key in secure vault, NOT in git.

---

## Step 2: Sign Messages (Backend) (5 minutes)

**Install:**
```bash
pip install hsip-bank-sdk
```

**Sign a message:**

```python
from hsip_bank import BankMessageSigner, MessageType

# Initialize signer
signer = BankMessageSigner(
    private_key_path="./keys/yourbank.com_private_key.pem",
    domain="yourbank.com"
)

# Sign a transaction alert
msg = signer.sign_message(
    message_text="Transaction: $50.00 at Starbucks",
    message_type=MessageType.TRANSACTION_ALERT,
    recipient="customer@email.com"
)

# Get push notification payload
payload = msg.to_notification_payload()
# Send via FCM/APNS: send_push(payload)
```

**That's it!** Messages are now cryptographically signed.

---

## Step 3: Verify in iOS App (4 minutes)

**Add SDK** (Swift Package Manager):
```
File → Add Packages → https://github.com/nyxsystems/hsip-ios → 1.0.0
```

**Initialize in AppDelegate.swift:**

```swift
import HSIP

class AppDelegate: UIApplicationDelegate {
    static var hsipVerifier: HSIPVerifier!

    func application(...) -> Bool {
        // Get hex from keys/metadata.json
        AppDelegate.hsipVerifier = try! HSIPVerifier(
            publicKeyHex: "YOUR_PUBLIC_KEY_HEX_HERE"
        )
        return true
    }
}
```

**Verify notifications:**

```swift
import HSIP

// Parse message from push notification
let message = SignedMessage(
    messageId: data["message_id"]!,
    senderDomain: data["sender_domain"]!,
    recipient: data["recipient"]!,
    messageType: data["message_type"]!,
    messageText: data["message_text"]!,
    sentAt: data["sent_at"]!,
    signature: data["signature"]!,
    publicKey: data["public_key"]!
)

// Verify
let result = AppDelegate.hsipVerifier.verificationResult(for: message)

if result.isVerified {
    // Show: ✅ yourbank.com - Transaction: $50.00...
    title = "\(result.badge) \(message.senderDomain)"
} else {
    // Show: 🔴 Unverified Message - Do not trust
    title = "\(result.badge) Unverified Message"
}
```

---

## Step 4: Verify in Android App (4 minutes)

**Add SDK** (build.gradle):
```gradle
dependencies {
    implementation 'com.hsip:hsip-android:1.0.0'
}
```

**Initialize in Application.kt:**

```kotlin
import com.hsip.HSIPVerifier

class BankApp : Application() {
    companion object {
        lateinit var hsipVerifier: HSIPVerifier
    }

    override fun onCreate() {
        super.onCreate()
        // Get hex from keys/metadata.json
        hsipVerifier = HSIPVerifier(
            bankPublicKeyHex = "YOUR_PUBLIC_KEY_HEX_HERE"
        )
    }
}
```

**Verify notifications:**

```kotlin
import com.hsip.SignedMessage

// Parse message from Firebase
val message = SignedMessage(
    messageId = data["message_id"]!!,
    senderDomain = data["sender_domain"]!!,
    recipient = data["recipient"]!!,
    messageType = data["message_type"]!!,
    messageText = data["message_text"]!!,
    sentAt = data["sent_at"]!!,
    signature = data["signature"]!!,
    publicKey = data["public_key"]!!
)

// Verify
val result = BankApp.hsipVerifier.verificationResult(message)

if (result.isVerified) {
    // Show: ✅ yourbank.com - Transaction: $50.00...
    showNotification("${result.badge} ${message.senderDomain}", ...)
} else {
    // Show: 🔴 Unverified Message
    showNotification("${result.badge} Unverified Message", ...)
}
```

---

## You're Done! 🎉

**Test it:**
1. Send a signed message from backend
2. Receive on mobile → See ✅ **VERIFIED** badge
3. Try sending unsigned message → See 🔴 **UNVERIFIED** warning

---

## What Customers See

### Before HSIP (No Protection)
```
📱 Chase Bank
   Transaction: $500 at Amazon
   [No way to verify authenticity]
```

### After HSIP (Protected)
```
📱 ✅ chase.com
   Transaction: $500 at Amazon
   [Cryptographically verified]

📱 🔴 Unverified Message
   Urgent: Update your account
   [Blocked phishing attempt]
```

---

## Key Files

```
sdk/
├── python/              # Backend SDK (signing)
│   ├── hsip_bank/
│   ├── setup.py
│   └── README.md
├── ios/                 # iOS SDK (verification)
│   └── HSIP/
│       ├── Sources/
│       └── README.md
└── android/             # Android SDK (verification)
    └── hsip/
        ├── src/
        └── README.md

BANK_INTEGRATION_GUIDE.md  # Full integration guide
QUICKSTART.md              # This file
```

---

## Security Checklist

Before production:
- [ ] Private key stored in HSM/vault (not in code)
- [ ] Public key hardcoded in mobile apps
- [ ] Critical messages (transaction/security alerts) blocked if unverified
- [ ] Monitoring enabled for signature failures

---

## Common Issues

**"Signature verification always fails"**
→ Check public key matches between backend and mobile apps

**"ImportError: No module named hsip_bank"**
→ Run: `pip install hsip-bank-sdk`

**"Cannot find HSIPVerifier in scope" (iOS)**
→ Add: `import HSIP` at top of file

**"Duplicate BouncyCastle class" (Android)**
→ Exclude conflicting dependency in build.gradle

---

## Next Steps

1. **Read full guide:** `BANK_INTEGRATION_GUIDE.md`
2. **Explore examples:** `sdk/python/examples/`
3. **Get support:** support@nyxsystems.com

---

## Enterprise Support

**HSIP Enterprise License:**
- Free 3-month pilot
- $15K/year licensing
- Includes: Priority support, security audits, compliance consulting

**Contact:** enterprise@nyxsystems.com

---

**Questions?** support@nyxsystems.com | https://docs.hsip.io
