import Foundation

/// A signed message from a bank
public struct SignedMessage: Codable {
    /// Unique message identifier
    public let messageId: String

    /// Bank's domain (e.g., "chase.com")
    public let senderDomain: String

    /// Customer identifier (email, phone, user_id)
    public let recipient: String

    /// Type of message (transaction_alert, security_alert, etc.)
    public let messageType: String

    /// The actual message text
    public let messageText: String

    /// ISO 8601 timestamp when message was sent
    public let sentAt: String

    /// Ed25519 signature (hex string)
    public let signature: String

    /// Bank's public key (hex string)
    public let publicKey: String

    enum CodingKeys: String, CodingKey {
        case messageId = "message_id"
        case senderDomain = "sender_domain"
        case recipient
        case messageType = "message_type"
        case messageText = "message_text"
        case sentAt = "sent_at"
        case signature
        case publicKey = "public_key"
    }

    public init(
        messageId: String,
        senderDomain: String,
        recipient: String,
        messageType: String,
        messageText: String,
        sentAt: String,
        signature: String,
        publicKey: String
    ) {
        self.messageId = messageId
        self.senderDomain = senderDomain
        self.recipient = recipient
        self.messageType = messageType
        self.messageText = messageText
        self.sentAt = sentAt
        self.signature = signature
        self.publicKey = publicKey
    }
}

/// Verification result
public struct VerificationResult {
    /// Whether the message signature is valid
    public let isVerified: Bool

    /// UI badge: "✅" or "🔴"
    public let badge: String

    /// Status text: "VERIFIED" or "UNVERIFIED"
    public let status: String

    public init(isVerified: Bool, badge: String, status: String) {
        self.isVerified = isVerified
        self.badge = badge
        self.status = status
    }
}

/// HSIP errors
public enum HSIPError: Error, LocalizedError {
    case invalidPublicKey(String)
    case invalidSignature(String)
    case verificationFailed(String)

    public var errorDescription: String? {
        switch self {
        case .invalidPublicKey(let message):
            return "Invalid public key: \(message)"
        case .invalidSignature(let message):
            return "Invalid signature: \(message)"
        case .verificationFailed(let message):
            return "Verification failed: \(message)"
        }
    }
}

/// Standard message types
public enum MessageType: String, Codable {
    case transactionAlert = "transaction_alert"
    case securityAlert = "security_alert"
    case accountUpdate = "account_update"
    case marketing = "marketing"
    case general = "general"
}
