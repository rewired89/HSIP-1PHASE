import Foundation
import CryptoKit

/// HSIP Message Verifier
///
/// Verifies cryptographically signed messages from banks
/// Shows ✅ VERIFIED or 🔴 UNVERIFIED based on signature validation
public class HSIPVerifier {
    private let bankPublicKey: Curve25519.Signing.PublicKey

    /// Initialize verifier with bank's public key
    ///
    /// - Parameter publicKeyHex: Bank's Ed25519 public key in hex format (64 characters)
    /// - Throws: HSIPError if public key is invalid
    public init(publicKeyHex: String) throws {
        // Convert hex string to Data
        guard let publicKeyData = Data(hexString: publicKeyHex) else {
            throw HSIPError.invalidPublicKey("Public key must be valid hex string")
        }

        guard publicKeyData.count == 32 else {
            throw HSIPError.invalidPublicKey("Public key must be 32 bytes (64 hex characters)")
        }

        do {
            self.bankPublicKey = try Curve25519.Signing.PublicKey(rawRepresentation: publicKeyData)
        } catch {
            throw HSIPError.invalidPublicKey("Failed to create public key: \(error.localizedDescription)")
        }
    }

    /// Verify a signed message
    ///
    /// - Parameters:
    ///   - message: The signed message received from bank
    /// - Returns: true if signature is valid, false otherwise
    public func verify(message: SignedMessage) -> Bool {
        return verify(
            messageId: message.messageId,
            senderDomain: message.senderDomain,
            recipient: message.recipient,
            messageType: message.messageType,
            messageText: message.messageText,
            sentAt: message.sentAt,
            signatureHex: message.signature
        )
    }

    /// Verify message components directly
    ///
    /// - Parameters:
    ///   - messageId: Unique message identifier
    ///   - senderDomain: Bank's domain (e.g., "chase.com")
    ///   - recipient: Customer identifier
    ///   - messageType: Type of message
    ///   - messageText: The actual message content
    ///   - sentAt: ISO 8601 timestamp
    ///   - signatureHex: Ed25519 signature in hex format
    /// - Returns: true if signature is valid, false otherwise
    public func verify(
        messageId: String,
        senderDomain: String,
        recipient: String,
        messageType: String,
        messageText: String,
        sentAt: String,
        signatureHex: String
    ) -> Bool {
        // Convert signature from hex
        guard let signatureData = Data(hexString: signatureHex) else {
            return false
        }

        guard signatureData.count == 64 else {
            return false
        }

        // Build canonical serialization (must match backend Python SDK)
        let canonicalData = buildCanonicalData(
            messageId: messageId,
            senderDomain: senderDomain,
            recipient: recipient,
            messageType: messageType,
            messageText: messageText,
            sentAt: sentAt
        )

        // Verify signature
        return bankPublicKey.isValidSignature(signatureData, for: canonicalData)
    }

    /// Get verification badge for UI display
    ///
    /// - Parameter message: The signed message to verify
    /// - Returns: "✅" if verified, "🔴" if unverified
    public func verificationBadge(for message: SignedMessage) -> String {
        return verify(message: message) ? "✅" : "🔴"
    }

    /// Get verification result with detailed status
    ///
    /// - Parameter message: The signed message to verify
    /// - Returns: VerificationResult with status and badge
    public func verificationResult(for message: SignedMessage) -> VerificationResult {
        let isValid = verify(message: message)
        return VerificationResult(
            isVerified: isValid,
            badge: isValid ? "✅" : "🔴",
            status: isValid ? "VERIFIED" : "UNVERIFIED"
        )
    }

    // MARK: - Private Helpers

    private func buildCanonicalData(
        messageId: String,
        senderDomain: String,
        recipient: String,
        messageType: String,
        messageText: String,
        sentAt: String
    ) -> Data {
        // Must match Python backend SDK serialization format
        let parts = [
            "HSIP_V1",
            messageId,
            senderDomain,
            recipient,
            messageType,
            messageText,
            sentAt
        ]
        let canonical = parts.joined(separator: "|")
        return canonical.data(using: .utf8)!
    }
}

// MARK: - Data Extension for Hex Conversion

extension Data {
    init?(hexString: String) {
        let len = hexString.count / 2
        var data = Data(capacity: len)
        var i = hexString.startIndex
        for _ in 0..<len {
            let j = hexString.index(i, offsetBy: 2)
            let bytes = hexString[i..<j]
            if var num = UInt8(bytes, radix: 16) {
                data.append(&num, count: 1)
            } else {
                return nil
            }
            i = j
        }
        self = data
    }

    var hexString: String {
        return map { String(format: "%02hhx", $0) }.joined()
    }
}
