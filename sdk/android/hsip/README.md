# HSIP Android SDK

**Cryptographic message verification for Android banking apps**

Verify that messages from your bank are cryptographically authentic. Shows ✅ **VERIFIED** or 🔴 **UNVERIFIED** badges.

## Installation

### Gradle (Recommended)

Add to your app's `build.gradle`:

```gradle
dependencies {
    implementation 'com.hsip:hsip-android:1.0.0'
}
```

### Manual Installation

1. Copy the `hsip/` directory into your project's `libs/` folder
2. Add to `settings.gradle`:
```gradle
include ':hsip'
project(':hsip').projectDir = new File('libs/hsip')
```
3. Add to app's `build.gradle`:
```gradle
dependencies {
    implementation project(':hsip')
}
```

## Quick Start

### 1. Initialize Verifier (One Time)

```kotlin
import com.hsip.HSIPVerifier
import com.hsip.SignedMessage

class BankApplication : Application() {
    companion object {
        // Store verifier globally
        lateinit var hsipVerifier: HSIPVerifier
    }

    override fun onCreate() {
        super.onCreate()

        try {
            // Initialize with your bank's public key
            hsipVerifier = HSIPVerifier(
                bankPublicKeyHex = "YOUR_BANK_PUBLIC_KEY_HEX_HERE"
            )
        } catch (e: Exception) {
            Log.e("HSIP", "Failed to initialize HSIP", e)
        }
    }
}
```

### 2. Verify Push Notifications

```kotlin
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import com.hsip.SignedMessage

class BankMessagingService : FirebaseMessagingService() {

    override fun onMessageReceived(message: RemoteMessage) {
        val data = message.data

        // Parse the signed message
        val signedMessage = SignedMessage(
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
        val result = BankApplication.hsipVerifier.verificationResult(signedMessage)

        if (result.isVerified) {
            // ✅ VERIFIED - Show notification with green badge
            showNotification(
                title = "${result.badge} ${signedMessage.senderDomain}",
                body = signedMessage.messageText,
                verified = true
            )
        } else {
            // 🔴 UNVERIFIED - Show warning or block
            showNotification(
                title = "${result.badge} Unverified Message",
                body = "This message could not be verified. Do not trust it.",
                verified = false
            )
        }
    }

    private fun showNotification(title: String, body: String, verified: Boolean) {
        val notification = NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(if (verified) R.drawable.ic_verified else R.drawable.ic_warning)
            .setContentTitle(title)
            .setContentText(body)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .build()

        notificationManager.notify(System.currentTimeMillis().toInt(), notification)
    }
}
```

### 3. Verify In-App Messages

```kotlin
import com.hsip.SignedMessage
import com.hsip.VerificationResult

class MessagesActivity : AppCompatActivity() {

    fun displayMessage(message: SignedMessage) {
        val result = BankApplication.hsipVerifier.verificationResult(message)

        // Update UI based on verification
        if (result.isVerified) {
            badgeTextView.text = result.badge  // ✅
            badgeTextView.setTextColor(Color.GREEN)
            messageTextView.text = message.messageText
        } else {
            badgeTextView.text = result.badge  // 🔴
            badgeTextView.setTextColor(Color.RED)
            messageTextView.text = "⚠️ Unverified message - do not trust"
        }
    }
}
```

## API Reference

### `HSIPVerifier`

Main verification class.

#### Constructor

```kotlin
HSIPVerifier(bankPublicKeyHex: String)
```

Creates a verifier with the bank's public key.

**Parameters:**
- `bankPublicKeyHex`: Bank's Ed25519 public key as hex string (64 characters)

**Throws:** `HSIPException.InvalidPublicKey` if key is invalid

**Example:**
```kotlin
val verifier = HSIPVerifier(bankPublicKeyHex = "a1b2c3d4...")
```

#### Methods

**`verify(message: SignedMessage)`**
```kotlin
fun verify(message: SignedMessage): Boolean
```
Verify a signed message. Returns `true` if valid, `false` otherwise.

**`verificationBadge(message: SignedMessage)`**
```kotlin
fun verificationBadge(message: SignedMessage): String
```
Returns "✅" if verified, "🔴" if unverified.

**`verificationResult(message: SignedMessage)`**
```kotlin
fun verificationResult(message: SignedMessage): VerificationResult
```
Returns detailed verification result with status and badge.

### `SignedMessage`

Represents a signed message from the bank.

```kotlin
data class SignedMessage(
    val messageId: String,        // Unique message identifier
    val senderDomain: String,     // Bank's domain (e.g., "chase.com")
    val recipient: String,        // Customer identifier
    val messageType: String,      // Type of message
    val messageText: String,      // The actual message text
    val sentAt: String,          // ISO 8601 timestamp
    val signature: String,       // Ed25519 signature (hex)
    val publicKey: String        // Bank's public key (hex)
)
```

### `VerificationResult`

Result of message verification.

```kotlin
data class VerificationResult(
    val isVerified: Boolean,    // true if signature is valid
    val badge: String,          // "✅" or "🔴"
    val status: String          // "VERIFIED" or "UNVERIFIED"
)
```

### `MessageType`

Standard message types.

```kotlin
enum class MessageType(val value: String) {
    TRANSACTION_ALERT("transaction_alert"),
    SECURITY_ALERT("security_alert"),
    ACCOUNT_UPDATE("account_update"),
    MARKETING("marketing"),
    GENERAL("general")
}
```

## Security Best Practices

### 1. Pin the Public Key

**Hardcode the bank's public key in your app** - do not fetch it from a server:

```kotlin
// ✅ Good - hardcoded
const val BANK_PUBLIC_KEY = "a1b2c3d4e5f6..."

// ❌ Bad - fetched from server (can be spoofed)
val publicKey = fetchPublicKey()
```

### 2. Block Unverified Messages

For critical message types (transaction alerts, security alerts), **block unverified messages**:

```kotlin
if (!result.isVerified && message.messageType == "transaction_alert") {
    // Don't show the message at all
    return
}
```

### 3. Update Public Keys Carefully

When rotating keys:
1. Ship new app version with new public key
2. Wait for 90% of users to update
3. Only then start using new private key on backend

### 4. Validate Message Fields

Always validate message fields before displaying:

```kotlin
if (message.messageText.isEmpty() || message.senderDomain.isEmpty()) {
    // Invalid message - reject it
    return
}
```

## Example Integration

### Complete Firebase Cloud Messaging Example

```kotlin
class BankMessagingService : FirebaseMessagingService() {

    override fun onMessageReceived(remoteMessage: RemoteMessage) {
        val data = remoteMessage.data

        // Parse signed message
        val message = SignedMessage(
            messageId = data["message_id"] ?: return,
            senderDomain = data["sender_domain"] ?: return,
            recipient = data["recipient"] ?: return,
            messageType = data["message_type"] ?: return,
            messageText = data["message_text"] ?: return,
            sentAt = data["sent_at"] ?: return,
            signature = data["signature"] ?: return,
            publicKey = data["public_key"] ?: return
        )

        // Verify signature
        val result = BankApplication.hsipVerifier.verificationResult(message)

        // Only show verified transaction/security alerts
        if (message.messageType in listOf("transaction_alert", "security_alert")) {
            if (!result.isVerified) {
                Log.w("HSIP", "Blocked unverified ${message.messageType}")
                return  // Don't show unverified critical messages
            }
        }

        // Show notification
        showSecureNotification(message, result)
    }
}
```

### ProGuard Rules

If using ProGuard/R8, add to `proguard-rules.pro`:

```proguard
# Keep HSIP classes
-keep class com.hsip.** { *; }

# Keep BouncyCastle crypto
-keep class org.bouncycastle.** { *; }
-dontwarn org.bouncycastle.**
```

## Dependencies

The SDK includes:
- **BouncyCastle** (org.bouncycastle:bcprov-jdk15on:1.70) - Ed25519 cryptography
- **Gson** (com.google.code.gson:gson:2.10.1) - JSON parsing

## Requirements

- Android 6.0+ (API level 23+)
- Kotlin 1.9.0+
- Gradle 7.0+

## Troubleshooting

### Signature Verification Always Fails

**Problem:** All messages show 🔴 UNVERIFIED

**Solution:**
1. Verify public key matches the one used on backend
2. Check that message fields are in correct order
3. Ensure no extra whitespace in signature/public key hex strings

```kotlin
// Debug: Print canonical data
val verifier = HSIPVerifier(publicKey)
Log.d("HSIP", "Public key: ${verifier.getPublicKeyHex()}")
```

### Build Errors with BouncyCastle

**Problem:** Duplicate class errors

**Solution:** Exclude conflicting dependencies:
```gradle
configurations {
    all {
        exclude group: 'org.bouncycastle', module: 'bcprov-jdk15to18'
    }
}
```

## Support

- Documentation: https://docs.hsip.io
- Issues: https://github.com/nyxsystems/hsip-android/issues
- Email: support@nyxsystems.com

## License

Proprietary - HSIP Enterprise License
