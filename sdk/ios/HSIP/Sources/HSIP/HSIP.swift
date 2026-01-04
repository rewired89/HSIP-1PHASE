/// HSIP iOS SDK
///
/// Cryptographic message verification for banks
///
/// # Overview
/// This SDK allows bank mobile apps to verify that messages are
/// cryptographically signed by the bank's backend servers.
///
/// # Usage
/// ```swift
/// import HSIP
///
/// // Initialize with bank's public key (one time, at app launch)
/// let verifier = try HSIPVerifier(publicKeyHex: "a1b2c3d4...")
///
/// // Verify messages when push notifications arrive
/// func handleNotification(_ data: [String: Any]) {
///     let decoder = JSONDecoder()
///     let message = try decoder.decode(SignedMessage.self, from: jsonData)
///
///     let result = verifier.verificationResult(for: message)
///     if result.isVerified {
///         showNotification(message.messageText, badge: "✅")
///     } else {
///         showWarning("Unverified message", badge: "🔴")
///     }
/// }
/// ```
///
/// # Key Features
/// - Ed25519 signature verification using CryptoKit
/// - Simple 3-line integration into existing banking apps
/// - Returns ✅ VERIFIED or 🔴 UNVERIFIED badges
/// - No external dependencies (uses native CryptoKit)

import Foundation

/// Current SDK version
public let HSIPVersion = "1.0.0"

// Re-export main types
public typealias Verifier = HSIPVerifier
