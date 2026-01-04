package com.hsip

import com.google.gson.annotations.SerializedName

/**
 * A signed message from a bank
 */
data class SignedMessage(
    /** Unique message identifier */
    @SerializedName("message_id")
    val messageId: String,

    /** Bank's domain (e.g., "chase.com") */
    @SerializedName("sender_domain")
    val senderDomain: String,

    /** Customer identifier (email, phone, user_id) */
    val recipient: String,

    /** Type of message (transaction_alert, security_alert, etc.) */
    @SerializedName("message_type")
    val messageType: String,

    /** The actual message text */
    @SerializedName("message_text")
    val messageText: String,

    /** ISO 8601 timestamp when message was sent */
    @SerializedName("sent_at")
    val sentAt: String,

    /** Ed25519 signature (hex string) */
    val signature: String,

    /** Bank's public key (hex string) */
    @SerializedName("public_key")
    val publicKey: String
)

/**
 * Verification result
 */
data class VerificationResult(
    /** Whether the message signature is valid */
    val isVerified: Boolean,

    /** UI badge: "✅" or "🔴" */
    val badge: String,

    /** Status text: "VERIFIED" or "UNVERIFIED" */
    val status: String
)

/**
 * HSIP exceptions
 */
sealed class HSIPException(message: String) : Exception(message) {
    class InvalidPublicKey(message: String) : HSIPException(message)
    class InvalidSignature(message: String) : HSIPException(message)
    class VerificationFailed(message: String) : HSIPException(message)
}

/**
 * Standard message types
 */
enum class MessageType(val value: String) {
    TRANSACTION_ALERT("transaction_alert"),
    SECURITY_ALERT("security_alert"),
    ACCOUNT_UPDATE("account_update"),
    MARKETING("marketing"),
    GENERAL("general");

    companion object {
        fun from(value: String): MessageType? =
            values().find { it.value == value }
    }
}
