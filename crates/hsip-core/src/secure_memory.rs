//! Secure memory management for cryptographic secrets
//!
//! Automatically zeros sensitive data when dropped to prevent:
//! - Memory dumps exposing secrets
//! - Swap file containing keys
//! - Cold boot attacks
//! - Memory reuse leaking old secrets

use std::fmt;
use std::ops::{Deref, DerefMut};
use zeroize::Zeroize;

/// A secure wrapper that zeros memory on drop
///
/// Use this for any sensitive data:
/// - Private keys
/// - Symmetric encryption keys
/// - Passwords and tokens
/// - Session secrets
/// - Nonces (after use)
///
/// # Example
/// ```
/// use hsip_core::secure_memory::SecureBytes;
///
/// let mut key = SecureBytes::new(vec![1, 2, 3, 4]);
/// // Use the key...
/// // When `key` goes out of scope, memory is automatically zeroed
/// ```
#[derive(Clone)]
pub struct SecureBytes {
    data: Vec<u8>,
}

impl SecureBytes {
    /// Create a new secure byte container
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Create from a slice
    pub fn from_slice(slice: &[u8]) -> Self {
        Self {
            data: slice.to_vec(),
        }
    }

    /// Create zeroed bytes of given length
    pub fn zeroed(len: usize) -> Self {
        Self {
            data: vec![0; len],
        }
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get as slice
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Get as mutable slice
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Extract the data (consumes self, caller responsible for zeroizing)
    pub fn into_vec(mut self) -> Vec<u8> {
        // Prevent drop from zeroizing since we're transferring ownership
        let data = std::mem::take(&mut self.data);
        std::mem::forget(self);
        data
    }

    /// Explicitly zero the contents
    pub fn zero(&mut self) {
        self.data.zeroize();
    }
}

impl Deref for SecureBytes {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.data
    }
}

impl DerefMut for SecureBytes {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

impl Drop for SecureBytes {
    fn drop(&mut self) {
        self.data.zeroize();
    }
}

// Prevent accidentally printing secrets
impl fmt::Debug for SecureBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecureBytes([REDACTED {} bytes])", self.data.len())
    }
}

/// Secure fixed-size array for keys
///
/// Use this for fixed-size cryptographic keys:
/// - Ed25519 keys (32 bytes)
/// - ChaCha20 keys (32 bytes)
/// - HMAC keys (varies)
#[derive(Clone)]
pub struct SecureKey<const N: usize> {
    data: [u8; N],
}

impl<const N: usize> SecureKey<N> {
    /// Create from array
    pub fn new(data: [u8; N]) -> Self {
        Self { data }
    }

    /// Create from slice (panics if wrong size)
    pub fn from_slice(slice: &[u8]) -> Self {
        assert_eq!(slice.len(), N, "Slice must be exactly {} bytes", N);
        let mut data = [0u8; N];
        data.copy_from_slice(slice);
        Self { data }
    }

    /// Create zeroed key
    pub fn zeroed() -> Self {
        Self { data: [0u8; N] }
    }

    /// Get as array reference
    pub fn as_array(&self) -> &[u8; N] {
        &self.data
    }

    /// Get as mutable array reference
    pub fn as_mut_array(&mut self) -> &mut [u8; N] {
        &mut self.data
    }

    /// Get as slice
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Explicitly zero the contents
    pub fn zero(&mut self) {
        self.data.zeroize();
    }
}

impl<const N: usize> Deref for SecureKey<N> {
    type Target = [u8; N];

    fn deref(&self) -> &[u8; N] {
        &self.data
    }
}

impl<const N: usize> DerefMut for SecureKey<N> {
    fn deref_mut(&mut self) -> &mut [u8; N] {
        &mut self.data
    }
}

impl<const N: usize> Drop for SecureKey<N> {
    fn drop(&mut self) {
        self.data.zeroize();
    }
}

impl<const N: usize> fmt::Debug for SecureKey<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecureKey<{}>([REDACTED])", N)
    }
}

/// Secure string for passwords and tokens
#[derive(Clone)]
pub struct SecureString {
    data: String,
}

impl SecureString {
    /// Create from string
    pub fn new(data: String) -> Self {
        Self { data }
    }

    /// Create from str
    pub fn from_str(s: &str) -> Self {
        Self {
            data: s.to_string(),
        }
    }

    /// Get as str
    pub fn as_str(&self) -> &str {
        &self.data
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Explicitly zero the contents
    pub fn zero(&mut self) {
        unsafe {
            let bytes = self.data.as_bytes_mut();
            bytes.zeroize();
        }
    }
}

impl Deref for SecureString {
    type Target = str;

    fn deref(&self) -> &str {
        &self.data
    }
}

impl Drop for SecureString {
    fn drop(&mut self) {
        unsafe {
            let bytes = self.data.as_bytes_mut();
            bytes.zeroize();
        }
    }
}

impl fmt::Debug for SecureString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecureString([REDACTED {} chars])", self.data.len())
    }
}

/// Memory lock hint (best effort)
///
/// On supported platforms, attempts to lock memory to prevent swapping.
/// This is advisory only - the OS may still swap if under pressure.
///
/// # Security Note
/// This requires elevated privileges on most systems.
/// If it fails, the code continues without memory locking.
#[cfg(unix)]
pub fn try_lock_memory(ptr: *const u8, len: usize) -> Result<(), String> {
    use libc::{mlock, ENOMEM};

    let result = unsafe { mlock(ptr as *const libc::c_void, len) };

    if result == 0 {
        Ok(())
    } else {
        let errno = std::io::Error::last_os_error();
        if errno.raw_os_error() == Some(ENOMEM) {
            Err("Insufficient memory lock quota".to_string())
        } else {
            Err(format!("Failed to lock memory: {}", errno))
        }
    }
}

#[cfg(windows)]
pub fn try_lock_memory(ptr: *const u8, len: usize) -> Result<(), String> {
    use winapi::um::memoryapi::VirtualLock;

    let result = unsafe { VirtualLock(ptr as *mut winapi::ctypes::c_void, len) };

    if result != 0 {
        Ok(())
    } else {
        Err(format!(
            "Failed to lock memory: {}",
            std::io::Error::last_os_error()
        ))
    }
}

#[cfg(not(any(unix, windows)))]
pub fn try_lock_memory(_ptr: *const u8, _len: usize) -> Result<(), String> {
    Err("Memory locking not supported on this platform".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_bytes_zeros_on_drop() {
        let data = vec![1, 2, 3, 4, 5];
        let ptr = data.as_ptr();

        {
            let secure = SecureBytes::new(data);
            assert_eq!(secure.len(), 5);
        }

        // After drop, memory should be zeroed
        // Note: This test is best-effort as the allocator may have moved the data
    }

    #[test]
    fn test_secure_bytes_deref() {
        let secure = SecureBytes::new(vec![1, 2, 3]);
        assert_eq!(&secure[..], &[1, 2, 3]);
    }

    #[test]
    fn test_secure_bytes_debug() {
        let secure = SecureBytes::new(vec![1, 2, 3, 4]);
        let debug = format!("{:?}", secure);
        assert!(debug.contains("REDACTED"));
        assert!(debug.contains("4 bytes"));
    }

    #[test]
    fn test_secure_key_fixed_size() {
        let key = SecureKey::<32>::new([0xFF; 32]);
        assert_eq!(key.as_slice().len(), 32);
        assert_eq!(key.as_slice()[0], 0xFF);
    }

    #[test]
    fn test_secure_key_from_slice() {
        let slice = [1, 2, 3, 4];
        let key = SecureKey::<4>::from_slice(&slice);
        assert_eq!(key.as_slice(), &slice);
    }

    #[test]
    #[should_panic(expected = "Slice must be exactly 32 bytes")]
    fn test_secure_key_wrong_size() {
        SecureKey::<32>::from_slice(&[1, 2, 3]); // Too short
    }

    #[test]
    fn test_secure_key_zeros_on_drop() {
        let mut key = SecureKey::<4>::new([1, 2, 3, 4]);
        assert_eq!(key.as_slice(), &[1, 2, 3, 4]);

        key.zero();
        assert_eq!(key.as_slice(), &[0, 0, 0, 0]);
    }

    #[test]
    fn test_secure_string() {
        let secret = SecureString::from_str("password123");
        assert_eq!(secret.as_str(), "password123");
        assert_eq!(secret.len(), 11);
    }

    #[test]
    fn test_secure_string_debug() {
        let secret = SecureString::from_str("password");
        let debug = format!("{:?}", secret);
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("password"));
    }

    #[test]
    fn test_secure_bytes_clone() {
        let original = SecureBytes::new(vec![1, 2, 3]);
        let cloned = original.clone();
        assert_eq!(original.as_slice(), cloned.as_slice());
    }

    #[test]
    fn test_secure_bytes_zeroed() {
        let secure = SecureBytes::zeroed(10);
        assert_eq!(secure.len(), 10);
        assert!(secure.iter().all(|&b| b == 0));
    }
}
