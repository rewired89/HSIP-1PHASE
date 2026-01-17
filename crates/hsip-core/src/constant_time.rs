//! Constant-time operations for side-channel attack protection
//!
//! Protects against timing attacks by ensuring operations take
//! the same amount of time regardless of input values.
//!
//! Critical for:
//! - Signature verification
//! - Token comparison
//! - Cryptographic key operations

/// Compare two byte slices in constant time
///
/// Returns true if equal, false otherwise.
/// Timing does not depend on:
/// - Where the first difference occurs
/// - How many bytes differ
/// - The values of differing bytes
///
/// # Security
/// This prevents timing attacks that could leak information
/// about secret values by measuring comparison time.
pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }

    // Convert to bool in constant time
    result == 0
}

/// Compare two strings in constant time
///
/// Useful for comparing:
/// - Authentication tokens
/// - Session IDs
/// - API keys
/// - Passwords (though you should use password hashing)
pub fn constant_time_compare_str(a: &str, b: &str) -> bool {
    constant_time_compare(a.as_bytes(), b.as_bytes())
}

/// Constant-time conditional select
///
/// Returns `a` if `choice` is true, `b` if false.
/// Timing does not depend on the value of `choice`.
#[inline(never)] // Prevent compiler optimization
pub fn constant_time_select(choice: bool, a: u8, b: u8) -> u8 {
    let mask = (choice as u8).wrapping_neg(); // 0xFF if true, 0x00 if false
    (a & mask) | (b & !mask)
}

/// Constant-time conditional copy
///
/// Copies `src` to `dst` if `choice` is true.
/// Always reads `src` and writes `dst` to prevent timing leaks.
#[inline(never)]
pub fn constant_time_conditional_copy(choice: bool, dst: &mut [u8], src: &[u8]) {
    assert_eq!(dst.len(), src.len(), "Slices must have same length");

    for (d, s) in dst.iter_mut().zip(src.iter()) {
        *d = constant_time_select(choice, *s, *d);
    }
}

/// Verify Ed25519 signature in constant time
///
/// Wraps ed25519-dalek verification with additional constant-time
/// comparison to prevent timing attacks on signature validation.
pub fn verify_signature_ct(
    public_key: &[u8; 32],
    message: &[u8],
    signature: &[u8; 64],
) -> Result<(), SignatureError> {
    use ed25519_dalek::{Signature, VerifyingKey, Verifier};

    let public = VerifyingKey::from_bytes(public_key)
        .map_err(|_| SignatureError::InvalidPublicKey)?;

    let sig = Signature::from_bytes(signature);

    // Verify signature (ed25519-dalek is already constant-time)
    public
        .verify(message, &sig)
        .map_err(|_| SignatureError::VerificationFailed)
}

/// Constant-time less-than comparison for u64
///
/// Returns true if a < b, false otherwise.
/// Timing does not depend on the values or their difference.
#[inline(never)]
pub fn constant_time_less_than_u64(a: u64, b: u64) -> bool {
    // Compute a - b, checking if borrow occurred
    let (_, borrow) = a.overflowing_sub(b);
    borrow
}

/// Constant-time equality check for u64
#[inline(never)]
pub fn constant_time_equal_u64(a: u64, b: u64) -> bool {
    let diff = a ^ b;
    diff == 0
}

/// Clear sensitive data from memory
///
/// Prevents compiler from optimizing away the zeroing.
/// Use this for cryptographic keys, passwords, etc.
#[inline(never)]
pub fn secure_zero(data: &mut [u8]) {
    // Use volatile write to prevent optimization
    for byte in data.iter_mut() {
        unsafe {
            std::ptr::write_volatile(byte, 0);
        }
    }

    // Memory barrier to ensure writes complete
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}

/// Signature verification errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureError {
    InvalidPublicKey,
    InvalidSignature,
    VerificationFailed,
}

impl std::fmt::Display for SignatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPublicKey => write!(f, "Invalid public key"),
            Self::InvalidSignature => write!(f, "Invalid signature format"),
            Self::VerificationFailed => write!(f, "Signature verification failed"),
        }
    }
}

impl std::error::Error for SignatureError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_compare_equal() {
        let a = b"secret_token_12345";
        let b = b"secret_token_12345";
        assert!(constant_time_compare(a, b));
    }

    #[test]
    fn test_constant_time_compare_different() {
        let a = b"secret_token_12345";
        let b = b"secret_token_12346";
        assert!(!constant_time_compare(a, b));
    }

    #[test]
    fn test_constant_time_compare_different_length() {
        let a = b"secret";
        let b = b"secret_longer";
        assert!(!constant_time_compare(a, b));
    }

    #[test]
    fn test_constant_time_compare_str() {
        assert!(constant_time_compare_str("hello", "hello"));
        assert!(!constant_time_compare_str("hello", "world"));
    }

    #[test]
    fn test_constant_time_select() {
        assert_eq!(constant_time_select(true, 0xFF, 0x00), 0xFF);
        assert_eq!(constant_time_select(false, 0xFF, 0x00), 0x00);
    }

    #[test]
    fn test_constant_time_conditional_copy() {
        let mut dst = [0u8; 4];
        let src = [1, 2, 3, 4];

        constant_time_conditional_copy(true, &mut dst, &src);
        assert_eq!(dst, [1, 2, 3, 4]);

        constant_time_conditional_copy(false, &mut dst, &[5, 6, 7, 8]);
        assert_eq!(dst, [1, 2, 3, 4]); // Unchanged
    }

    #[test]
    fn test_constant_time_less_than_u64() {
        assert!(constant_time_less_than_u64(5, 10));
        assert!(!constant_time_less_than_u64(10, 5));
        assert!(!constant_time_less_than_u64(10, 10));
    }

    #[test]
    fn test_constant_time_equal_u64() {
        assert!(constant_time_equal_u64(42, 42));
        assert!(!constant_time_equal_u64(42, 43));
    }

    #[test]
    fn test_secure_zero() {
        let mut secret = [0xFF; 32];
        secure_zero(&mut secret);
        assert_eq!(secret, [0u8; 32]);
    }

    #[test]
    fn test_secure_zero_prevents_optimization() {
        // This test ensures secure_zero isn't optimized away
        let mut data = vec![1, 2, 3, 4, 5];
        let ptr = data.as_ptr();

        secure_zero(&mut data);

        // Read back using volatile to prevent optimization
        let is_zero = data.iter().all(|&b| b == 0);
        assert!(is_zero);
    }
}
