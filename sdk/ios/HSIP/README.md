# HSIP iOS SDK

**Cryptographic message verification for iOS banking apps**

Verify that messages from your bank are cryptographically authentic. Shows ✅ **VERIFIED** or 🔴 **UNVERIFIED** badges.

## Installation

### Swift Package Manager (Recommended)

In Xcode:
1. File → Add Packages
2. Enter repository URL: `https://github.com/nyxsystems/hsip-ios`
3. Select version `1.0.0`
4. Add to your target

Or add to `Package.swift`:
```swift
dependencies: [
    .package(url: "https://github.com/nyxsystems/hsip-ios", from: "1.0.0")
]
```

### CocoaPods

```ruby
pod 'HSIP', '~> 1.0.0'
```

## Quick Start

### 1. Initialize Verifier (One Time)

```swift
import HSIP

class AppDelegate: UIApplicationDelegate {
    // Store verifier globally
    static var hsipVerifier: HSIPVerifier!

    func application(_ application: UIApplication,
                     didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {

        do {
            // Initialize with your bank's public key
            AppDelegate.hsipVerifier = try HSIPVerifier(
                publicKeyHex: "YOUR_BANK_PUBLIC_KEY_HEX_HERE"
            )
        } catch {
            print("Failed to initialize HSIP: \(error)")
        }

        return true
    }
}
```

### 2. Verify Push Notifications

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
        let messageText = messageData["message_text"] as? String ?? ""
        let signature = messageData["signature"] as? String ?? ""
        let messageId = messageData["message_id"] as? String ?? ""
        let senderDomain = messageData["sender_domain"] as? String ?? ""
        let recipient = messageData["recipient"] as? String ?? ""
        let messageType = messageData["message_type"] as? String ?? ""
        let sentAt = messageData["sent_at"] as? String ?? ""
        let publicKey = messageData["public_key"] as? String ?? ""

        let message = SignedMessage(
            messageId: messageId,
            senderDomain: senderDomain,
            recipient: recipient,
            messageType: messageType,
            messageText: messageText,
            sentAt: sentAt,
            signature: signature,
            publicKey: publicKey
        )

        // Verify the message
        let result = AppDelegate.hsipVerifier.verificationResult(for: message)

        if result.isVerified {
            // ✅ VERIFIED - Show green badge
            content.title = "\(result.badge) \(senderDomain)"
            content.body = messageText
            content.badge = 1
            content.sound = .default
        } else {
            // 🔴 UNVERIFIED - Show red warning or block
            content.title = "\(result.badge) Unverified Message"
            content.body = "This message could not be verified. Do not trust it."
            content.badge = 1
            content.sound = .default
        }

        contentHandler(content)
    }
}
```

### 3. Verify In-App Messages

```swift
import HSIP

class MessagesViewController: UIViewController {
    func displayMessage(_ message: SignedMessage) {
        let result = AppDelegate.hsipVerifier.verificationResult(for: message)

        // Update UI based on verification
        if result.isVerified {
            badgeLabel.text = result.badge  // ✅
            badgeLabel.textColor = .systemGreen
            messageLabel.text = message.messageText
        } else {
            badgeLabel.text = result.badge  // 🔴
            badgeLabel.textColor = .systemRed
            messageLabel.text = "⚠️ Unverified message - do not trust"
        }
    }
}
```

## API Reference

### `HSIPVerifier`

Main verification class.

#### Initialization

```swift
init(publicKeyHex: String) throws
```

Creates a verifier with the bank's public key.

**Parameters:**
- `publicKeyHex`: Bank's Ed25519 public key as hex string (64 characters)

**Throws:** `HSIPError.invalidPublicKey` if key is invalid

**Example:**
```swift
let verifier = try HSIPVerifier(publicKeyHex: "a1b2c3d4...")
```

#### Methods

**`verify(message:)`**
```swift
func verify(message: SignedMessage) -> Bool
```
Verify a signed message. Returns `true` if valid, `false` otherwise.

**`verificationBadge(for:)`**
```swift
func verificationBadge(for message: SignedMessage) -> String
```
Returns "✅" if verified, "🔴" if unverified.

**`verificationResult(for:)`**
```swift
func verificationResult(for message: SignedMessage) -> VerificationResult
```
Returns detailed verification result with status and badge.

### `SignedMessage`

Represents a signed message from the bank.

```swift
struct SignedMessage: Codable {
    let messageId: String
    let senderDomain: String
    let recipient: String
    let messageType: String
    let messageText: String
    let sentAt: String
    let signature: String
    let publicKey: String
}
```

### `VerificationResult`

Result of message verification.

```swift
struct VerificationResult {
    let isVerified: Bool    // true if signature is valid
    let badge: String       // "✅" or "🔴"
    let status: String      // "VERIFIED" or "UNVERIFIED"
}
```

## Security Best Practices

### 1. Pin the Public Key

**Hardcode the bank's public key in your app** - do not fetch it from a server:

```swift
// ✅ Good - hardcoded
let PUBLIC_KEY = "a1b2c3d4e5f6..."

// ❌ Bad - fetched from server (can be spoofed)
let publicKey = await fetchPublicKey()
```

### 2. Block Unverified Messages

For critical message types (transaction alerts, security alerts), **block unverified messages**:

```swift
if !result.isVerified && message.messageType == "transaction_alert" {
    // Don't show the message at all
    return
}
```

### 3. Update Public Keys Carefully

When rotating keys:
1. Ship new app version with new public key
2. Wait for 90% of users to update
3. Only then start using new private key on backend

## Example Integration

See `Examples/` directory for:
- Complete notification service extension
- In-app message display
- Key management

## Requirements

- iOS 13.0+
- Swift 5.9+
- Xcode 15.0+

## Support

- Documentation: https://docs.hsip.io
- Issues: https://github.com/nyxsystems/hsip-ios/issues
- Email: support@nyxsystems.com

## License

Proprietary - HSIP Enterprise License
