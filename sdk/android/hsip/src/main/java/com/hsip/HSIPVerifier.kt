package com.hsip

import org.bouncycastle.crypto.params.Ed25519PublicKeyParameters
import org.bouncycastle.crypto.signers.Ed25519Signer

/**
 * HSIP Message Verifier for Android
 *
 * Verifies cryptographically signed messages from banks.
 * Shows ✅ VERIFIED or 🔴 UNVERIFIED based on signature validation.
 *
 * Example:
 * ```
 * val verifier = HSIPVerifier("a1b2c3d4...")
 * val result = verifier.verify(message)
 * if (result.isVerified) {
 *     showBadge("✅")
 * } else {
 *     showBadge("🔴")
 * }
 * ```
 */
class HSIPVerifier(bankPublicKeyHex: String) {
    private val publicKey: Ed25519PublicKeyParameters

    init {
        // Convert hex string to bytes
        val publicKeyBytes = hexToBytes(bankPublicKeyHex)
            ?: throw HSIPException.InvalidPublicKey("Public key must be valid hex string")

        if (publicKeyBytes.size != 32) {
            throw HSIPException.InvalidPublicKey("Public key must be 32 bytes (64 hex characters)")
        }

        try {
            publicKey = Ed25519PublicKeyParameters(publicKeyBytes, 0)
        } catch (e: Exception) {
            throw HSIPException.InvalidPublicKey("Failed to create public key: ${e.message}")
        }
    }

    /**
     * Verify a signed message
     *
     * @param message The signed message received from bank
     * @return true if signature is valid, false otherwise
     */
    fun verify(message: SignedMessage): Boolean {
        return verify(
            messageId = message.messageId,
            senderDomain = message.senderDomain,
            recipient = message.recipient,
            messageType = message.messageType,
            messageText = message.messageText,
            sentAt = message.sentAt,
            signatureHex = message.signature
        )
    }

    /**
     * Verify message components directly
     *
     * @param messageId Unique message identifier
     * @param senderDomain Bank's domain (e.g., "chase.com")
     * @param recipient Customer identifier
     * @param messageType Type of message
     * @param messageText The actual message content
     * @param sentAt ISO 8601 timestamp
     * @param signatureHex Ed25519 signature in hex format
     * @return true if signature is valid, false otherwise
     */
    fun verify(
        messageId: String,
        senderDomain: String,
        recipient: String,
        messageType: String,
        messageText: String,
        sentAt: String,
        signatureHex: String
    ): Boolean {
        // Convert signature from hex
        val signatureBytes = hexToBytes(signatureHex) ?: return false

        if (signatureBytes.size != 64) {
            return false
        }

        // Build canonical serialization (must match backend Python SDK)
        val canonicalData = buildCanonicalData(
            messageId = messageId,
            senderDomain = senderDomain,
            recipient = recipient,
            messageType = messageType,
            messageText = messageText,
            sentAt = sentAt
        )

        // Verify signature using Ed25519
        return try {
            val verifier = Ed25519Signer()
            verifier.init(false, publicKey)
            verifier.update(canonicalData, 0, canonicalData.size)
            verifier.verifySignature(signatureBytes)
        } catch (e: Exception) {
            false
        }
    }

    /**
     * Get verification badge for UI display
     *
     * @param message The signed message to verify
     * @return "✅" if verified, "🔴" if unverified
     */
    fun verificationBadge(message: SignedMessage): String {
        return if (verify(message)) "✅" else "🔴"
    }

    /**
     * Get verification result with detailed status
     *
     * @param message The signed message to verify
     * @return VerificationResult with status and badge
     */
    fun verificationResult(message: SignedMessage): VerificationResult {
        val isValid = verify(message)
        return VerificationResult(
            isVerified = isValid,
            badge = if (isValid) "✅" else "🔴",
            status = if (isValid) "VERIFIED" else "UNVERIFIED"
        )
    }

    // Private helpers

    private fun buildCanonicalData(
        messageId: String,
        senderDomain: String,
        recipient: String,
        messageType: String,
        messageText: String,
        sentAt: String
    ): ByteArray {
        // Must match Python backend SDK serialization format
        val parts = listOf(
            "HSIP_V1",
            messageId,
            senderDomain,
            recipient,
            messageType,
            messageText,
            sentAt
        )
        val canonical = parts.joinToString("|")
        return canonical.toByteArray(Charsets.UTF_8)
    }

    private fun hexToBytes(hex: String): ByteArray? {
        if (hex.length % 2 != 0) return null

        return try {
            ByteArray(hex.length / 2) { i ->
                hex.substring(i * 2, i * 2 + 2).toInt(16).toByte()
            }
        } catch (e: NumberFormatException) {
            null
        }
    }
}
