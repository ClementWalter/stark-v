//! Ethereum address type (20 bytes) with EIP-55 checksumming
//!
//! This module provides the [`Address`] type, a 20-byte Ethereum address
//! with full support for EIP-55 mixed-case checksumming for display.

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::{format, string::String};

#[cfg(target_arch = "riscv32")]
use alloc::{format, string::String};

use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use crate::crypto::keccak256;

/// A 20-byte Ethereum address.
///
/// This type represents an Ethereum address with full support for:
/// - EIP-55 mixed-case checksumming for display
/// - Hex encoding/decoding
/// - Serialization/deserialization
///
/// # Examples
///
/// ```
/// use claudeth::types::Address;
/// use core::str::FromStr;
///
/// let addr = Address::from_str("0x5aAeb6053f3E94C9b9A09f33669435E7Ef1BeAed").unwrap();
/// // Address can be parsed and displayed
/// assert!(addr.to_string().starts_with("0x"));
/// assert_eq!(addr.to_string().len(), 42);
/// ```
#[derive(Clone, Copy, Default)]
pub struct Address([u8; 20]);

impl Address {
    /// Zero address (0x0000000000000000000000000000000000000000)
    pub const ZERO: Self = Address([0u8; 20]);

    /// Creates an address from a 20-byte array.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Address;
    ///
    /// let bytes = [0x42; 20];
    /// let addr = Address::from(bytes);
    /// ```
    #[inline]
    pub const fn new(bytes: [u8; 20]) -> Self {
        Address(bytes)
    }

    /// Creates an address from a slice.
    ///
    /// Returns `None` if the slice length is not exactly 20 bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Address;
    ///
    /// let bytes = [0x42; 20];
    /// let addr = Address::from_slice(&bytes).unwrap();
    ///
    /// let invalid = [0x42; 19];
    /// assert!(Address::from_slice(&invalid).is_none());
    /// ```
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != 20 {
            return None;
        }
        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(slice);
        Some(Address(bytes))
    }

    /// Returns a reference to the underlying byte array.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Address;
    ///
    /// let addr = Address::from([0x42; 20]);
    /// assert_eq!(addr.as_bytes(), &[0x42; 20]);
    /// ```
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    /// Returns a mutable reference to the underlying byte array.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Address;
    ///
    /// let mut addr = Address::default();
    /// addr.as_bytes_mut()[0] = 0x42;
    /// assert_eq!(addr.as_bytes()[0], 0x42);
    /// ```
    #[inline]
    pub fn as_bytes_mut(&mut self) -> &mut [u8; 20] {
        &mut self.0
    }

    /// Returns a copy of the address as a 20-byte array.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Address;
    ///
    /// let addr = Address::new([0x42; 20]);
    /// let bytes = addr.to_bytes();
    /// assert_eq!(bytes, [0x42; 20]);
    /// ```
    #[inline]
    pub fn to_bytes(&self) -> [u8; 20] {
        self.0
    }

    /// Computes the Keccak-256 hash of the address in hex (without 0x prefix).
    ///
    /// This is used internally for EIP-55 checksumming.
    fn keccak256_hex(&self) -> [u8; 32] {
        // Convert to hex string (lowercase, no 0x prefix)
        let hex = hex_encode(&self.0);

        // Compute Keccak-256 of the hex string
        let hash = keccak256(hex.as_bytes());
        *hash.as_bytes()
    }

    /// Formats the address with EIP-55 mixed-case checksumming.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Address;
    /// use core::str::FromStr;
    ///
    /// let addr = Address::from_str("0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed").unwrap();
    /// let checksummed = addr.to_checksum_string();
    /// // Checksum string starts with 0x and has 42 chars
    /// assert!(checksummed.starts_with("0x"));
    /// assert_eq!(checksummed.len(), 42);
    /// ```
    pub fn to_checksum_string(&self) -> String {
        let hash = self.keccak256_hex();
        let hex = hex_encode(&self.0);

        let mut result = String::from("0x");
        for (i, c) in hex.chars().enumerate() {
            if c.is_ascii_alphabetic() {
                // Get the corresponding nibble from the hash
                let hash_byte = hash[i / 2];
                let nibble = if i.is_multiple_of(2) {
                    hash_byte >> 4
                } else {
                    hash_byte & 0x0f
                };

                // If the nibble is >= 8, uppercase the character
                if nibble >= 8 {
                    result.push(c.to_ascii_uppercase());
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }

        result
    }
}

// =============================================================================
// Trait Implementations
// =============================================================================

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Address({})", self.to_checksum_string())
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_checksum_string())
    }
}

impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Address {}

impl Hash for Address {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialOrd for Address {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Address {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl From<[u8; 20]> for Address {
    fn from(bytes: [u8; 20]) -> Self {
        Address(bytes)
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for Address {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl FromStr for Address {
    type Err = ParseAddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix("0x").unwrap_or(s);

        if s.len() != 40 {
            return Err(ParseAddressError::InvalidLength);
        }

        let mut bytes = [0u8; 20];
        for i in 0..20 {
            let hex_byte = &s[i * 2..i * 2 + 2];
            bytes[i] = u8::from_str_radix(hex_byte, 16)
                .map_err(|_| ParseAddressError::InvalidHex)?;
        }

        Ok(Address(bytes))
    }
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_checksum_string())
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Address::from_str(&s).map_err(serde::de::Error::custom)
    }
}

// =============================================================================
// Error Types
// =============================================================================

/// Error type for address parsing failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseAddressError {
    /// Invalid hex character in address string
    InvalidHex,
    /// Invalid address length (must be 40 hex characters)
    InvalidLength,
}

impl fmt::Display for ParseAddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseAddressError::InvalidHex => write!(f, "invalid hex character in address"),
            ParseAddressError::InvalidLength => write!(f, "invalid address length (must be 40 hex characters)"),
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Encodes bytes as lowercase hex string (without 0x prefix).
fn hex_encode(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        result.push_str(&format!("{byte:02x}"));
    }
    result
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
    fn test_zero_address() {
        let addr = Address::ZERO;
        assert_eq!(addr.0, [0u8; 20]);
    }

    #[test]
    fn test_new() {
        let bytes = [0x42; 20];
        let addr = Address::new(bytes);
        assert_eq!(addr.0, bytes);
    }

    #[test]
    fn test_from_array() {
        let bytes = [0x42; 20];
        let addr = Address::from(bytes);
        assert_eq!(addr.0, bytes);
    }

    #[test]
    fn test_default() {
        let addr = Address::default();
        assert_eq!(addr.0, [0u8; 20]);
    }

    // =========================================================================
    // Slice Conversion Tests
    // =========================================================================

    #[test]
    fn test_from_slice_valid() {
        let bytes = [0x42; 20];
        let addr = Address::from_slice(&bytes).unwrap();
        assert_eq!(addr.0, bytes);
    }

    #[test]
    fn test_from_slice_too_short() {
        let bytes = [0x42; 19];
        assert!(Address::from_slice(&bytes).is_none());
    }

    #[test]
    fn test_from_slice_too_long() {
        let bytes = [0x42; 21];
        assert!(Address::from_slice(&bytes).is_none());
    }

    #[test]
    fn test_from_slice_empty() {
        let bytes: [u8; 0] = [];
        assert!(Address::from_slice(&bytes).is_none());
    }

    // =========================================================================
    // Access Tests
    // =========================================================================

    #[test]
    fn test_as_bytes() {
        let bytes = [0x42; 20];
        let addr = Address::from(bytes);
        assert_eq!(addr.as_bytes(), &bytes);
    }

    #[test]
    fn test_as_bytes_mut() {
        let mut addr = Address::default();
        addr.as_bytes_mut()[0] = 0x42;
        assert_eq!(addr.0[0], 0x42);
    }

    #[test]
    fn test_as_ref() {
        let bytes = [0x42; 20];
        let addr = Address::from(bytes);
        let slice: &[u8] = addr.as_ref();
        assert_eq!(slice, &bytes);
    }

    #[test]
    fn test_as_mut() {
        let mut addr = Address::default();
        let slice: &mut [u8] = addr.as_mut();
        slice[0] = 0x42;
        assert_eq!(addr.0[0], 0x42);
    }

    // =========================================================================
    // Parsing Tests
    // =========================================================================

    #[test]
    fn test_from_str_with_prefix() {
        let addr = Address::from_str("0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed").unwrap();
        assert_eq!(addr.0[0], 0x5a);
        assert_eq!(addr.0[19], 0xed);
    }

    #[test]
    fn test_from_str_without_prefix() {
        let addr = Address::from_str("5aaeb6053f3e94c9b9a09f33669435e7ef1beaed").unwrap();
        assert_eq!(addr.0[0], 0x5a);
        assert_eq!(addr.0[19], 0xed);
    }

    #[test]
    fn test_from_str_uppercase() {
        let addr = Address::from_str("0x5AAEB6053F3E94C9B9A09F33669435E7EF1BEAED").unwrap();
        assert_eq!(addr.0[0], 0x5a);
        assert_eq!(addr.0[19], 0xed);
    }

    #[test]
    fn test_from_str_mixed_case() {
        let addr = Address::from_str("0x5aAeb6053f3E94C9b9A09f33669435E7Ef1BeAed").unwrap();
        assert_eq!(addr.0[0], 0x5a);
        assert_eq!(addr.0[19], 0xed);
    }

    #[test]
    fn test_from_str_too_short() {
        let result = Address::from_str("0x5aaeb6053f3e94c9b9a09f33669435e7ef1bea");
        assert_eq!(result, Err(ParseAddressError::InvalidLength));
    }

    #[test]
    fn test_from_str_too_long() {
        let result = Address::from_str("0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed00");
        assert_eq!(result, Err(ParseAddressError::InvalidLength));
    }

    #[test]
    fn test_from_str_invalid_hex() {
        let result = Address::from_str("0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaez");
        assert_eq!(result, Err(ParseAddressError::InvalidHex));
    }

    #[test]
    fn test_from_str_empty() {
        let result = Address::from_str("");
        assert_eq!(result, Err(ParseAddressError::InvalidLength));
    }

    #[test]
    fn test_from_str_only_prefix() {
        let result = Address::from_str("0x");
        assert_eq!(result, Err(ParseAddressError::InvalidLength));
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[test]
    fn test_display_zero() {
        let addr = Address::ZERO;
        let s = addr.to_string();
        assert!(s.starts_with("0x"));
        assert_eq!(s.len(), 42); // 0x + 40 hex chars
    }

    #[test]
    fn test_display_nonzero() {
        let bytes = [0x42; 20];
        let addr = Address::from(bytes);
        let s = addr.to_string();
        assert!(s.starts_with("0x"));
        assert_eq!(s.len(), 42);
    }

    #[test]
    fn test_debug() {
        let addr = Address::ZERO;
        let s = format!("{addr:?}");
        assert!(s.contains("Address"));
        assert!(s.contains("0x"));
    }

    // =========================================================================
    // Equality and Ordering Tests
    // =========================================================================

    #[test]
    fn test_eq_same() {
        let addr1 = Address::from([0x42; 20]);
        let addr2 = Address::from([0x42; 20]);
        assert_eq!(addr1, addr2);
    }

    #[test]
    fn test_eq_different() {
        let addr1 = Address::from([0x42; 20]);
        let addr2 = Address::from([0x43; 20]);
        assert_ne!(addr1, addr2);
    }

    #[test]
    fn test_ord_less() {
        let addr1 = Address::from([0x41; 20]);
        let addr2 = Address::from([0x42; 20]);
        assert!(addr1 < addr2);
    }

    #[test]
    fn test_ord_greater() {
        let addr1 = Address::from([0x43; 20]);
        let addr2 = Address::from([0x42; 20]);
        assert!(addr1 > addr2);
    }

    #[test]
    fn test_ord_equal() {
        let addr1 = Address::from([0x42; 20]);
        let addr2 = Address::from([0x42; 20]);
        assert_eq!(addr1.cmp(&addr2), Ordering::Equal);
    }

    // =========================================================================
    // Hash Tests
    // =========================================================================

    #[test]
    fn test_hash_same() {
        use std::collections::hash_map::DefaultHasher;

        let addr1 = Address::from([0x42; 20]);
        let addr2 = Address::from([0x42; 20]);

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();

        addr1.hash(&mut hasher1);
        addr2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_hash_different() {
        use std::collections::hash_map::DefaultHasher;

        let addr1 = Address::from([0x42; 20]);
        let addr2 = Address::from([0x43; 20]);

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();

        addr1.hash(&mut hasher1);
        addr2.hash(&mut hasher2);

        assert_ne!(hasher1.finish(), hasher2.finish());
    }

    // =========================================================================
    // Clone and Copy Tests
    // =========================================================================

    #[test]
    fn test_clone() {
        let addr1 = Address::from([0x42; 20]);
        let addr2 = addr1;
        assert_eq!(addr1, addr2);
    }

    #[test]
    fn test_copy() {
        let addr1 = Address::from([0x42; 20]);
        let addr2 = addr1;
        assert_eq!(addr1, addr2);
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[test]
    fn test_serialize() {
        let addr = Address::from_str("0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed").unwrap();
        let json = serde_json::to_string(&addr).unwrap();
        assert!(json.starts_with("\"0x"));
        assert!(json.ends_with("\""));
    }

    #[test]
    fn test_deserialize() {
        let json = "\"0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed\"";
        let addr: Address = serde_json::from_str(json).unwrap();
        assert_eq!(addr.0[0], 0x5a);
        assert_eq!(addr.0[19], 0xed);
    }

    #[test]
    fn test_roundtrip_serialize() {
        let addr1 = Address::from_str("0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed").unwrap();
        let json = serde_json::to_string(&addr1).unwrap();
        let addr2: Address = serde_json::from_str(&json).unwrap();
        assert_eq!(addr1, addr2);
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn test_all_zeros() {
        let addr = Address::from([0x00; 20]);
        let s = addr.to_string();
        assert!(s.starts_with("0x"));
        assert_eq!(s.len(), 42);
    }

    #[test]
    fn test_all_ones() {
        let addr = Address::from([0xff; 20]);
        let s = addr.to_string();
        assert!(s.starts_with("0x"));
        assert_eq!(s.len(), 42);
    }

    #[test]
    fn test_alternating_pattern() {
        let mut bytes = [0u8; 20];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = if i.is_multiple_of(2) { 0xaa } else { 0x55 };
        }
        let addr = Address::from(bytes);
        let s = addr.to_string();
        assert!(s.starts_with("0x"));
        assert_eq!(s.len(), 42);
    }

    // =========================================================================
    // Known Address Tests (EIP-55 Examples)
    // =========================================================================

    #[test]
    fn test_known_address_parsing() {
        let addresses = [
            "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed",
            "0xfB6916095ca1df60bB79Ce92cE3Ea74c37c5d359",
            "0xdbF03B407c01E7cD3CBea99509d93f8DDDC8C6FB",
            "0xD1220A0cf47c7B9Be7A2E6BA89F429762e7b9aDb",
        ];

        for addr_str in &addresses {
            let addr = Address::from_str(addr_str);
            assert!(addr.is_ok(), "Failed to parse {addr_str}");
        }
    }

    #[test]
    fn test_eip55_checksum_examples() {
        let addresses = [
            "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed",
            "0xfB6916095ca1df60bB79Ce92cE3Ea74c37c5d359",
            "0xdbF03B407c01E7cD3CBea99509d93f8DDDC8C6FB",
            "0xD1220A0cf47c7B9Be7A2E6BA89F429762e7b9aDb",
        ];

        for addr_str in &addresses {
            let addr = Address::from_str(addr_str).expect("parse checksummed address");
            assert_eq!(addr.to_checksum_string(), *addr_str);
        }
    }

    #[test]
    fn test_case_insensitive_parsing() {
        let lower = Address::from_str("0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed").unwrap();
        let upper = Address::from_str("0x5AAEB6053F3E94C9B9A09F33669435E7EF1BEAED").unwrap();
        let mixed = Address::from_str("0x5aAeb6053f3E94C9b9A09f33669435E7Ef1BeAed").unwrap();

        assert_eq!(lower, upper);
        assert_eq!(upper, mixed);
        assert_eq!(lower, mixed);
    }
}
