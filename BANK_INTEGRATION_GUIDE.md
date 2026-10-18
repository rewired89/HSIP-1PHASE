# HSIP Bank Integration Guide

**Complete guide for integrating HSIP secure messaging into your banking platform**

This guide shows you how to integrate HSIP to eliminate phishing attacks by cryptographically signing all messages sent to customers.

## What You'll Build

After integration, your customers will see:
- ✅ **VERIFIED** badge on authentic messages from your bank
- 🔴 **UNVERIFIED** warning on phishing attempts

**Example:** When you send "Transaction: $50.00 at Starbucks", customers see your message with a green ✅ badge proving it's really from your bank.

---

## Integration Overview

HSIP requires three components:

1. **Backend SDK (Python)** - Signs messages on your servers
2. **iOS SDK (Swift)** - Customers verify messages in your iOS app
3. **Android SDK (Kotlin)** - Customers verify messages in your Android app

**Timeline:** Complete integration typically takes 1-2 sprints.

---

## Step 1: Generate Bank Identity Keys

### Install Python SDK

On your backend server:

```bash
pip install hsip-bank-sdk
```

### Generate Keypair

```bash
hsip-keygen --domain yourbank.com --output ./keys
```

This creates:
```
keys/
├── yourbank.com_private_key.pem  # Keep this SECRET
├── yourbank.com_public_key.pem   # Distribute to mobile apps
└── metadata.json                 # Contains public key hex
```

### Secure the Private Key

**CRITICAL:** Store the private key in an HSM (Hardware Security Module) or secure key vault:

```bash
# AWS KMS example
aws kms import-key-material \
  --key-id your-key-id \
  --encrypted-key-material fileb://yourbank.com_private_key.pem

# Azure Key Vault example
az keyvault secret set \
  --vault-name YourBankVault \
  --name hsip-private-key \
  --file yourbank.com_private_key.pem
```

**Never commit private keys to git or store in plaintext.**

---

## Step 2: Integrate Backend Signing

### Initialize Signer

Add to your backend application (Python):

```python
from hsip_bank import BankMessageSigner, MessageType

# Initialize once at startup
signer = BankMessageSigner(
    private_key_path="/secure/path/to/yourbank.com_private_key.pem",
    domain="yourbank.com"
)
```

### Sign Messages Before Sending

**Example 1: Transaction Alert**

```python
# Your existing code sends SMS/push notifications
customer_email = "customer@example.com"
transaction_text = "Transaction: $50.00 at Starbucks on Main St"

# Sign the message
signed_message = signer.sign_message(
    message_text=transaction_text,
    message_type=MessageType.TRANSACTION_ALERT,
    recipient=customer_email
)

# Send via your push notification system
send_push_notification(
    user_id=customer_id,
    payload=signed_message.to_notification_payload()
)
```

**Example 2: Security Alert**

```python
security_text = "New login from Chrome on Windows - San Francisco, CA"

signed_message = signer.sign_message(
    message_text=security_text,
    message_type=MessageType.SECURITY_ALERT,
    recipient=customer_email
)

send_push_notification(
    user_id=customer_id,
    payload=signed_message.to_notification_payload()
)
```

### Notification Payload Format

The `to_notification_payload()` method returns:

```json
{
  "title": "yourbank.com",
  "body": "Transaction: $50.00 at Starbucks",
  "data": {
    "message_id": "msg_abc123",
    "sender_domain": "yourbank.com",
    "recipient": "customer@example.com",
    "message_type": "transaction_alert",
    "message_text": "Transaction: $50.00 at Starbucks",
    "sent_at": "2025-01-04T12:30:00Z",
    "signature": "a1b2c3d4e5f6...",
    "public_key": "789abcdef..."
  }
}
```

Send this payload via FCM (Firebase Cloud Messaging) or APNS (Apple Push Notification Service).

---

## Step 3: Integrate iOS App Verification

### Add HSIP SDK to iOS App

**Using Swift Package Manager** (recommended):

1. Open Xcode
2. File → Add Packages
3. Enter: `https://github.com/nyxsystems/hsip-ios`
4. Version: `1.0.0`

Or add to `Package.swift`:

```swift
dependencies: [
    .package(url: "https://github.com/nyxsystems/hsip-ios", from: "1.0.0")
]
```

### Initialize Verifier (One Time)

In your `AppDelegate.swift`:

```swift
import HSIP

class AppDelegate: UIApplicationDelegate {
    static var hsipVerifier: HSIPVerifier!

    func application(_ application: UIApplication,
                     didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {

        do {
            // Get public key from keys/metadata.json (from Step 1)
            let publicKeyHex = "YOUR_BANK_PUBLIC_KEY_HEX_HERE"

            AppDelegate.hsipVerifier = try HSIPVerifier(
                publicKeyHex: publicKeyHex
            )
        } catch {
            print("Failed to initialize HSIP: \(error)")
        }

        return true
    }
}
```

**IMPORTANT:** Hardcode the public key in your app. Do NOT fetch it from a server (phishers could spoof it).

### Verify Push Notifications

In your notification service extension:

```swift
import UserNotifications
import HSIP

extension NotificationService: UNNotificationServiceExtension {
    func didReceive(_ request: UNNotificationRequest,
                    withContentHandler contentHandler: @escaping (UNNotificationContent) -> Void) {

        guard let content = request.content.mutableCopy() as? UNMutableNotificationContent,
              let messageData = content.userInfo["data"] as? [String: Any] else {
            return
        }

        // Parse the signed message
        let message = SignedMessage(
            messageId: messageData["message_id"] as? String ?? "",
            senderDomain: messageData["sender_domain"] as? String ?? "",
            recipient: messageData["recipient"] as? String ?? "",
            messageType: messageData["message_type"] as? String ?? "",
            messageText: messageData["message_text"] as? String ?? "",
            sentAt: messageData["sent_at"] as? String ?? "",
            signature: messageData["signature"] as? String ?? "",
            publicKey: messageData["public_key"] as? String ?? ""
        )

        // Verify the message
        let result = AppDelegate.hsipVerifier.verificationResult(for: message)

        if result.isVerified {
            // ✅ VERIFIED - Show with green badge
            content.title = "\(result.badge) \(message.senderDomain)"
            content.body = message.messageText
        } else {
            // 🔴 UNVERIFIED - Block or warn
            content.title = "\(result.badge) Unverified Message"
            content.body = "This message could not be verified."
        }

        contentHandler(content)
    }
}
```

---

## Step 4: Integrate Android App Verification

### Add HSIP SDK to Android App

In your app's `build.gradle`:

```gradle
dependencies {
    implementation 'com.hsip:hsip-android:1.0.0'
}
```

Or for manual installation, copy the SDK into your `libs/` directory.

### Initialize Verifier (One Time)

In your `Application` class:

```kotlin
import com.hsip.HSIPVerifier

class BankApplication : Application() {
    companion object {
        lateinit var hsipVerifier: HSIPVerifier
    }

    override fun onCreate() {
        super.onCreate()

        try {
            // Get public key from keys/metadata.json (from Step 1)
            val publicKeyHex = "YOUR_BANK_PUBLIC_KEY_HEX_HERE"

            hsipVerifier = HSIPVerifier(
                bankPublicKeyHex = publicKeyHex
            )
        } catch (e: Exception) {
            Log.e("HSIP", "Failed to initialize HSIP", e)
        }
    }
}
```

### Verify Push Notifications

In your Firebase messaging service:

```kotlin
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import com.hsip.SignedMessage

class BankMessagingService : FirebaseMessagingService() {

    override fun onMessageReceived(remoteMessage: RemoteMessage) {
        val data = remoteMessage.data

        // Parse the signed message
        val message = SignedMessage(
            messageId = data["message_id"] ?: "",
            senderDomain = data["sender_domain"] ?: "",
            recipient = data["recipient"] ?: "",
            messageType = data["message_type"] ?: "",
            messageText = data["message_text"] ?: "",
            sentAt = data["sent_at"] ?: "",
            signature = data["signature"] ?: "",
            publicKey = data["public_key"] ?: ""
        )

        // Verify the message
        val result = BankApplication.hsipVerifier.verificationResult(message)

        if (result.isVerified) {
            // ✅ VERIFIED - Show notification
            showNotification(
                title = "${result.badge} ${message.senderDomain}",
                body = message.messageText,
                verified = true
            )
        } else {
            // 🔴 UNVERIFIED - Block or warn
            showNotification(
                title = "${result.badge} Unverified Message",
                body = "This message could not be verified.",
                verified = false
            )
        }
    }
}
```

---

## Step 5: Testing

### Test Key Generation

```bash
# Verify keypair was generated
ls -la keys/
# Should show: private_key.pem, public_key.pem, metadata.json

# Check public key hex
cat keys/metadata.json
```

### Test Backend Signing

Create `test_signing.py`:

```python
from hsip_bank import BankMessageSigner, MessageType

signer = BankMessageSigner(
    private_key_path="./keys/yourbank.com_private_key.pem",
    domain="yourbank.com"
)

msg = signer.sign_message(
    message_text="Test transaction alert",
    message_type=MessageType.TRANSACTION_ALERT,
    recipient="test@example.com"
)

print("✅ Message signed successfully!")
print(f"Signature: {msg.signature[:32]}...")
print(f"Payload: {msg.to_notification_payload()}")
```

Run:
```bash
python test_signing.py
```

### Test Mobile App Verification

1. **Send test notification** from your backend
2. **Verify badge appears** on customer's phone
3. **Try tampering** with message (change text) - should show 🔴

**Test cases:**
- ✅ Valid signature → Shows green badge
- 🔴 Modified message text → Shows red warning
- 🔴 Wrong public key → Shows red warning
- 🔴 Expired timestamp → (Optional) Shows red warning

---

## Step 6: Production Deployment

### Checklist

- [ ] Private key stored in HSM/secure vault (not in code)
- [ ] Public key hardcoded in mobile apps (not fetched from server)
- [ ] All message types (transaction, security, account) are signed
- [ ] Mobile apps block unverified critical messages
- [ ] Monitoring/alerts for signature failures
- [ ] Key rotation plan documented

### Deploy Backend Changes

```bash
# Deploy to production
git add yourbank_backend/
git commit -m "Add HSIP message signing"
git push origin main

# Deploy via your CI/CD pipeline
```

### Deploy Mobile Apps

1. **iOS:**
   ```bash
   # Update version in Xcode
   # Archive and submit to App Store
   ```

2. **Android:**
   ```bash
   # Update versionCode in build.gradle
   # Build release APK/AAB
   ./gradlew bundleRelease
   ```

### Monitor Deployment

Track adoption:
- % of users on new app version
- Signature verification success rate
- User reports of phishing attempts

---

## Security Best Practices

### 1. Key Management

**DO:**
- ✅ Store private keys in HSM or secure vault
- ✅ Restrict access to private key (least privilege)
- ✅ Hardcode public key in mobile apps
- ✅ Plan for key rotation (every 1-2 years)

**DON'T:**
- ❌ Store private keys in code or config files
- ❌ Fetch public keys from server at runtime
- ❌ Share private keys across environments
- ❌ Leave private keys unencrypted

### 2. Message Handling

**Critical messages** (transaction_alert, security_alert):
- Block completely if unverified
- Log failed verifications
- Alert security team

**Non-critical messages** (marketing, general):
- Show red 🔴 badge but don't block
- Let user decide whether to trust

### 3. Key Rotation

When rotating keys:

1. Generate new keypair
2. Ship new mobile app version with NEW public key
3. Wait for 90% user adoption (check analytics)
4. Switch backend to sign with NEW private key
5. Keep OLD key for 30 days (for users on old app versions)
6. Deprecate OLD key

---

## Troubleshooting

### Signature Verification Always Fails

**Problem:** All messages show 🔴 UNVERIFIED

**Causes:**
1. Public key mismatch (mobile app has different key than backend)
2. Message tampering by proxy/firewall
3. Timestamp issues (clock drift)

**Solution:**
```bash
# Verify public keys match
# On backend:
python -c "from hsip_bank import load_keypair; print(load_keypair('keys/yourbank.com_private_key.pem')[1])"

# Compare with public key hex in mobile apps
```

### Backend Signing Errors

**Problem:** Python SDK throws errors when signing

**Error:** `FileNotFoundError: private_key.pem`

**Solution:**
```python
# Use absolute path
signer = BankMessageSigner(
    private_key_path="/var/secure/keys/yourbank.com_private_key.pem",
    domain="yourbank.com"
)
```

### Mobile App Build Errors

**iOS:** `Cannot find 'HSIPVerifier' in scope`

**Solution:**
```swift
// Ensure import statement exists
import HSIP

// Rebuild project: Cmd+Shift+K, then Cmd+B
```

**Android:** `Duplicate class org.bouncycastle...`

**Solution:**
```gradle
configurations {
    all {
        exclude group: 'org.bouncycastle', module: 'bcprov-jdk15to18'
    }
}
```

---

## Example: Full Integration

See `examples/` directory for:
- **Python backend** integration with Django/Flask
- **iOS app** complete notification service
- **Android app** complete FCM integration

---

## Support

### Documentation
- Python SDK: `sdk/python/README.md`

### Contact
- Email: support@nyxsystems.com
- Issues: https://github.com/nyxsystems/hsip-bank-sdk/issues
- Documentation: https://docs.hsip.io

### Enterprise Support
For priority support, key rotation assistance, and compliance consulting:
- Contact: enterprise@nyxsystems.com
- SLA: 24-hour response time
- Includes: Security audits, integration assistance, training

---

## Pricing

**HSIP Enterprise License:**
- Free pilot integration (3 months)
- $15,000/year after pilot
- Includes: All SDKs, updates, security patches, email support

---

## Next Steps

1. **Schedule integration kickoff** - Email enterprise@nyxsystems.com
2. **Generate keys** - Use `hsip-keygen` CLI
3. **Integrate backend** - Add signing to message pipeline
4. **Integrate mobile apps** - Add verification SDKs
5. **Test thoroughly** - Verify signatures work end-to-end
6. **Deploy to production** - Roll out to customers

**Questions?** Contact support@nyxsystems.com
