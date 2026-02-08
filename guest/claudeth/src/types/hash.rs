//! Ethereum hash type (32 bytes)
//!
//! This module provides the [`Hash`] type (also known as H256), a 32-byte
//! hash value commonly used in Ethereum for transaction hashes, block hashes,
//! and other cryptographic digests.

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::{format, string::String};

#[cfg(target_arch = "riscv32")]
use alloc::{format, string::String};

use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash as StdHash, Hasher};
use core::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A 32-byte hash value (H256).
///
/// This type represents a 32-byte hash commonly used in Ethereum for:
/// - Transaction hashes
/// - Block hashes
/// - State roots
/// - Receipt roots
/// - Other cryptographic digests
///
/// # Examples
///
/// ```
/// use claudeth::types::Hash;
/// use core::str::FromStr;
///
/// let hash = Hash::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap();
/// assert_eq!(hash.to_string(), "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");
/// ```
#[derive(Clone, Copy, Default)]
pub struct Hash([u8; 32]);

/// Type alias for Hash (common in Ethereum contexts)
pub type H256 = Hash;

impl Hash {
    /// Zero hash (0x0000...0000)
    pub const ZERO: Self = Hash([0u8; 32]);

    /// Creates a hash from a 32-byte array.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Hash;
    ///
    /// let bytes = [0x42; 32];
    /// let hash = Hash::from(bytes);
    /// ```
    #[inline]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Hash(bytes)
    }

    /// Creates a hash from a slice.
    ///
    /// Returns `None` if the slice length is not exactly 32 bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Hash;
    ///
    /// let bytes = [0x42; 32];
    /// let hash = Hash::from_slice(&bytes).unwrap();
    ///
    /// let invalid = [0x42; 31];
    /// assert!(Hash::from_slice(&invalid).is_none());
    /// ```
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != 32 {
            return None;
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(slice);
        Some(Hash(bytes))
    }

    /// Returns a reference to the underlying byte array.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Hash;
    ///
    /// let hash = Hash::from([0x42; 32]);
    /// assert_eq!(hash.as_bytes(), &[0x42; 32]);
    /// ```
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Returns a mutable reference to the underlying byte array.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Hash;
    ///
    /// let mut hash = Hash::default();
    /// hash.as_bytes_mut()[0] = 0x42;
    /// assert_eq!(hash.as_bytes()[0], 0x42);
    /// ```
    #[inline]
    pub fn as_bytes_mut(&mut self) -> &mut [u8; 32] {
        &mut self.0
    }

    /// Formats the hash as a hex string with 0x prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Hash;
    ///
    /// let hash = Hash::from([0x42; 32]);
    /// let s = hash.to_hex_string();
    /// assert!(s.starts_with("0x"));
    /// assert_eq!(s.len(), 66); // 0x + 64 hex chars
    /// ```
    pub fn to_hex_string(&self) -> String {
        let mut result = String::from("0x");
        for &byte in &self.0 {
            result.push_str(&format!("{byte:02x}"));
        }
        result
    }
}

// =============================================================================
// Trait Implementations
// =============================================================================

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({})", self.to_hex_string())
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex_string())
    }
}

impl PartialEq for Hash {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Hash {}

impl StdHash for Hash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialOrd for Hash {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Hash {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl From<[u8; 32]> for Hash {
    fn from(bytes: [u8; 32]) -> Self {
        Hash(bytes)
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for Hash {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl FromStr for Hash {
    type Err = ParseHashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix("0x").unwrap_or(s);

        if s.len() != 64 {
            return Err(ParseHashError::InvalidLength);
        }

        let mut bytes = [0u8; 32];
        for i in 0..32 {
            let hex_byte = &s[i * 2..i * 2 + 2];
            bytes[i] = u8::from_str_radix(hex_byte, 16)
                .map_err(|_| ParseHashError::InvalidHex)?;
        }

        Ok(Hash(bytes))
    }
}

impl Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex_string())
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Hash::from_str(&s).map_err(serde::de::Error::custom)
    }
}

// =============================================================================
// Error Types
// =============================================================================

/// Error type for hash parsing failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseHashError {
    /// Invalid hex character in hash string
    InvalidHex,
    /// Invalid hash length (must be 64 hex characters)
    InvalidLength,
}

impl fmt::Display for ParseHashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseHashError::InvalidHex => write!(f, "invalid hex character in hash"),
            ParseHashError::InvalidLength => write!(f, "invalid hash length (must be 64 hex characters)"),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Basic Construction Tests
    // =========================================================================

    #[test]
    fn test_zero_hash() {
        let hash = Hash::ZERO;
        assert_eq!(hash.0, [0u8; 32]);
    }

    #[test]
    fn test_new() {
        let bytes = [0x42; 32];
        let hash = Hash::new(bytes);
        assert_eq!(hash.0, bytes);
    }

    #[test]
    fn test_from_array() {
        let bytes = [0x42; 32];
        let hash = Hash::from(bytes);
        assert_eq!(hash.0, bytes);
    }

    #[test]
    fn test_default() {
        let hash = Hash::default();
        assert_eq!(hash.0, [0u8; 32]);
    }

    // =========================================================================
    // Slice Conversion Tests
    // =========================================================================

    #[test]
    fn test_from_slice_valid() {
        let bytes = [0x42; 32];
        let hash = Hash::from_slice(&bytes).unwrap();
        assert_eq!(hash.0, bytes);
    }

    #[test]
    fn test_from_slice_too_short() {
        let bytes = [0x42; 31];
        assert!(Hash::from_slice(&bytes).is_none());
    }

    #[test]
    fn test_from_slice_too_long() {
        let bytes = [0x42; 33];
        assert!(Hash::from_slice(&bytes).is_none());
    }

    #[test]
    fn test_from_slice_empty() {
        let bytes: [u8; 0] = [];
        assert!(Hash::from_slice(&bytes).is_none());
    }

    // =========================================================================
    // Access Tests
    // =========================================================================

    #[test]
    fn test_as_bytes() {
        let bytes = [0x42; 32];
        let hash = Hash::from(bytes);
        assert_eq!(hash.as_bytes(), &bytes);
    }

    #[test]
    fn test_as_bytes_mut() {
        let mut hash = Hash::default();
        hash.as_bytes_mut()[0] = 0x42;
        assert_eq!(hash.0[0], 0x42);
    }

    #[test]
    fn test_as_ref() {
        let bytes = [0x42; 32];
        let hash = Hash::from(bytes);
        let slice: &[u8] = hash.as_ref();
        assert_eq!(slice, &bytes);
    }

    #[test]
    fn test_as_mut() {
        let mut hash = Hash::default();
        let slice: &mut [u8] = hash.as_mut();
        slice[0] = 0x42;
        assert_eq!(hash.0[0], 0x42);
    }

    // =========================================================================
    // Parsing Tests
    // =========================================================================

    #[test]
    fn test_from_str_with_prefix() {
        let hash = Hash::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap();
        assert_eq!(hash.0[0], 0x12);
        assert_eq!(hash.0[31], 0xef);
    }

    #[test]
    fn test_from_str_without_prefix() {
        let hash = Hash::from_str("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap();
        assert_eq!(hash.0[0], 0x12);
        assert_eq!(hash.0[31], 0xef);
    }

    #[test]
    fn test_from_str_uppercase() {
        let hash = Hash::from_str("0x1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF").unwrap();
        assert_eq!(hash.0[0], 0x12);
        assert_eq!(hash.0[31], 0xef);
    }

    #[test]
    fn test_from_str_mixed_case() {
        let hash = Hash::from_str("0x1234567890AbCdEf1234567890aBcDeF1234567890AbCdEf1234567890aBcDeF").unwrap();
        assert_eq!(hash.0[0], 0x12);
        assert_eq!(hash.0[31], 0xef);
    }

    #[test]
    fn test_from_str_too_short() {
        let result = Hash::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcd");
        assert_eq!(result, Err(ParseHashError::InvalidLength));
    }

    #[test]
    fn test_from_str_too_long() {
        let result = Hash::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef00");
        assert_eq!(result, Err(ParseHashError::InvalidLength));
    }

    #[test]
    fn test_from_str_invalid_hex() {
        let result = Hash::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdez");
        assert_eq!(result, Err(ParseHashError::InvalidHex));
    }

    #[test]
    fn test_from_str_empty() {
        let result = Hash::from_str("");
        assert_eq!(result, Err(ParseHashError::InvalidLength));
    }

    #[test]
    fn test_from_str_only_prefix() {
        let result = Hash::from_str("0x");
        assert_eq!(result, Err(ParseHashError::InvalidLength));
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[test]
    fn test_to_hex_string_zero() {
        let hash = Hash::ZERO;
        let s = hash.to_hex_string();
        assert_eq!(s, "0x0000000000000000000000000000000000000000000000000000000000000000");
    }

    #[test]
    fn test_to_hex_string_nonzero() {
        let bytes = [0x42; 32];
        let hash = Hash::from(bytes);
        let s = hash.to_hex_string();
        assert_eq!(s, "0x4242424242424242424242424242424242424242424242424242424242424242");
    }

    #[test]
    fn test_display_zero() {
        let hash = Hash::ZERO;
        let s = hash.to_string();
        assert!(s.starts_with("0x"));
        assert_eq!(s.len(), 66); // 0x + 64 hex chars
    }

    #[test]
    fn test_display_nonzero() {
        let bytes = [0x42; 32];
        let hash = Hash::from(bytes);
        let s = hash.to_string();
        assert!(s.starts_with("0x"));
        assert_eq!(s.len(), 66);
    }

    #[test]
    fn test_debug() {
        let hash = Hash::ZERO;
        let s = format!("{hash:?}");
        assert!(s.contains("Hash"));
        assert!(s.contains("0x"));
    }

    // =========================================================================
    // Equality and Ordering Tests
    // =========================================================================

    #[test]
    fn test_eq_same() {
        let hash1 = Hash::from([0x42; 32]);
        let hash2 = Hash::from([0x42; 32]);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_eq_different() {
        let hash1 = Hash::from([0x42; 32]);
        let hash2 = Hash::from([0x43; 32]);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_ord_less() {
        let hash1 = Hash::from([0x41; 32]);
        let hash2 = Hash::from([0x42; 32]);
        assert!(hash1 < hash2);
    }

    #[test]
    fn test_ord_greater() {
        let hash1 = Hash::from([0x43; 32]);
        let hash2 = Hash::from([0x42; 32]);
        assert!(hash1 > hash2);
    }

    #[test]
    fn test_ord_equal() {
        let hash1 = Hash::from([0x42; 32]);
        let hash2 = Hash::from([0x42; 32]);
        assert_eq!(hash1.cmp(&hash2), Ordering::Equal);
    }

    // =========================================================================
    // Hash Tests
    // =========================================================================

    #[test]
    fn test_hash_same() {
        use std::collections::hash_map::DefaultHasher;

        let hash1 = Hash::from([0x42; 32]);
        let hash2 = Hash::from([0x42; 32]);

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();

        hash1.hash(&mut hasher1);
        hash2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_hash_different() {
        use std::collections::hash_map::DefaultHasher;

        let hash1 = Hash::from([0x42; 32]);
        let hash2 = Hash::from([0x43; 32]);

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();

        hash1.hash(&mut hasher1);
        hash2.hash(&mut hasher2);

        assert_ne!(hasher1.finish(), hasher2.finish());
    }

    // =========================================================================
    // Clone and Copy Tests
    // =========================================================================

    #[test]
    fn test_clone() {
        let hash1 = Hash::from([0x42; 32]);
        let hash2 = hash1;
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_copy() {
        let hash1 = Hash::from([0x42; 32]);
        let hash2 = hash1;
        assert_eq!(hash1, hash2);
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[test]
    fn test_serialize() {
        let hash = Hash::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap();
        let json = serde_json::to_string(&hash).unwrap();
        assert!(json.starts_with("\"0x"));
        assert!(json.ends_with("\""));
    }

    #[test]
    fn test_deserialize() {
        let json = "\"0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef\"";
        let hash: Hash = serde_json::from_str(json).unwrap();
        assert_eq!(hash.0[0], 0x12);
        assert_eq!(hash.0[31], 0xef);
    }

    #[test]
    fn test_roundtrip_serialize() {
        let hash1 = Hash::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap();
        let json = serde_json::to_string(&hash1).unwrap();
        let hash2: Hash = serde_json::from_str(&json).unwrap();
        assert_eq!(hash1, hash2);
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn test_all_zeros() {
        let hash = Hash::from([0x00; 32]);
        let s = hash.to_string();
        assert_eq!(s, "0x0000000000000000000000000000000000000000000000000000000000000000");
    }

    #[test]
    fn test_all_ones() {
        let hash = Hash::from([0xff; 32]);
        let s = hash.to_string();
        assert_eq!(s, "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    }

    #[test]
    fn test_alternating_pattern() {
        let mut bytes = [0u8; 32];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = if i.is_multiple_of(2) { 0xaa } else { 0x55 };
        }
        let hash = Hash::from(bytes);
        let s = hash.to_string();
        assert!(s.starts_with("0x"));
        assert_eq!(s.len(), 66);
    }

    // =========================================================================
    // Type Alias Tests
    // =========================================================================

    #[test]
    fn test_h256_alias() {
        let hash: H256 = Hash::from([0x42; 32]);
        assert_eq!(hash.0, [0x42; 32]);
    }

    #[test]
    fn test_h256_roundtrip() {
        let h256: H256 = H256::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap();
        let hash: Hash = h256;
        assert_eq!(hash.0[0], 0x12);
        assert_eq!(hash.0[31], 0xef);
    }

    // =========================================================================
    // Known Hash Tests
    // =========================================================================

    #[test]
    fn test_known_hash_parsing() {
        let hashes = [
            "0x0000000000000000000000000000000000000000000000000000000000000000",
            "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
        ];

        for hash_str in &hashes {
            let hash = Hash::from_str(hash_str);
            assert!(hash.is_ok(), "Failed to parse {hash_str}");
        }
    }

    #[test]
    fn test_case_insensitive_parsing() {
        let lower = Hash::from_str("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap();
        let upper = Hash::from_str("0x1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF").unwrap();
        let mixed = Hash::from_str("0x1234567890AbCdEf1234567890aBcDeF1234567890AbCdEf1234567890aBcDeF").unwrap();

        assert_eq!(lower, upper);
        assert_eq!(upper, mixed);
        assert_eq!(lower, mixed);
    }

    #[test]
    fn test_parse_display_roundtrip() {
        let original = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let hash = Hash::from_str(original).unwrap();
        let displayed = hash.to_string();
        assert_eq!(original, displayed);
    }

    #[test]
    fn test_parse_display_roundtrip_zeros() {
        let original = "0x0000000000000000000000000000000000000000000000000000000000000000";
        let hash = Hash::from_str(original).unwrap();
        let displayed = hash.to_string();
        assert_eq!(original, displayed);
    }

    #[test]
    fn test_parse_display_roundtrip_all_f() {
        let original = "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let hash = Hash::from_str(original).unwrap();
        let displayed = hash.to_string();
        assert_eq!(original, displayed);
    }
}
