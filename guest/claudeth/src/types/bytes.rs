//! Dynamic byte array implementation for Ethereum
//!
//! This module provides the [`Bytes`] type, a wrapper around `Vec<u8>` with
//! convenient methods for working with byte arrays in Ethereum contexts.

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::{string::String, vec::Vec};

#[cfg(target_arch = "riscv32")]
use alloc::{string::String, vec::Vec};

use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::{Index, IndexMut};
use core::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A dynamic byte array wrapper around `Vec<u8>`.
///
/// This type provides a convenient interface for working with byte arrays
/// in Ethereum contexts, including hex encoding/decoding and serialization.
///
/// # Examples
///
/// ```
/// use claudeth::types::Bytes;
///
/// let mut bytes = Bytes::new();
/// bytes.push(0x42);
/// bytes.extend_from_slice(&[0x43, 0x44]);
/// assert_eq!(bytes.len(), 3);
/// ```
#[derive(Clone, Debug, Default, Eq)]
pub struct Bytes {
    inner: Vec<u8>,
}

impl Bytes {
    /// Creates a new empty `Bytes`.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Bytes;
    ///
    /// let bytes = Bytes::new();
    /// assert_eq!(bytes.len(), 0);
    /// ```
    #[inline]
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }

    /// Creates a new `Bytes` from a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Bytes;
    ///
    /// let bytes = Bytes::from_slice(&[0x01, 0x02, 0x03]);
    /// assert_eq!(bytes.len(), 3);
    /// ```
    #[inline]
    pub fn from_slice(slice: &[u8]) -> Self {
        Self {
            inner: slice.to_vec(),
        }
    }

    /// Creates a new empty `Bytes` with the specified capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Bytes;
    ///
    /// let bytes = Bytes::with_capacity(10);
    /// assert_eq!(bytes.len(), 0);
    /// ```
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
        }
    }

    /// Returns the number of bytes in the `Bytes`.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Bytes;
    ///
    /// let bytes = Bytes::from_slice(&[0x01, 0x02]);
    /// assert_eq!(bytes.len(), 2);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the `Bytes` contains no bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Bytes;
    ///
    /// let bytes = Bytes::new();
    /// assert!(bytes.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Appends a byte to the back of the `Bytes`.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Bytes;
    ///
    /// let mut bytes = Bytes::new();
    /// bytes.push(0x42);
    /// assert_eq!(bytes.len(), 1);
    /// ```
    #[inline]
    pub fn push(&mut self, byte: u8) {
        self.inner.push(byte);
    }

    /// Extends the `Bytes` with the contents of a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Bytes;
    ///
    /// let mut bytes = Bytes::new();
    /// bytes.extend_from_slice(&[0x01, 0x02, 0x03]);
    /// assert_eq!(bytes.len(), 3);
    /// ```
    #[inline]
    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        self.inner.extend_from_slice(slice);
    }

    /// Shortens the `Bytes`, keeping the first `len` bytes and dropping the rest.
    ///
    /// If `len` is greater than the current length, this has no effect.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Bytes;
    ///
    /// let mut bytes = Bytes::from_slice(&[0x01, 0x02, 0x03]);
    /// bytes.truncate(2);
    /// assert_eq!(bytes.len(), 2);
    /// ```
    #[inline]
    pub fn truncate(&mut self, len: usize) {
        self.inner.truncate(len);
    }

    /// Clears the `Bytes`, removing all bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Bytes;
    ///
    /// let mut bytes = Bytes::from_slice(&[0x01, 0x02]);
    /// bytes.clear();
    /// assert_eq!(bytes.len(), 0);
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Returns the capacity of the `Bytes`.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Bytes;
    ///
    /// let bytes = Bytes::with_capacity(10);
    /// assert!(bytes.capacity() >= 10);
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Reserves capacity for at least `additional` more bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Bytes;
    ///
    /// let mut bytes = Bytes::new();
    /// bytes.reserve(10);
    /// assert!(bytes.capacity() >= 10);
    /// ```
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }

    /// Removes the last byte from the `Bytes` and returns it, or `None` if it is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::Bytes;
    ///
    /// let mut bytes = Bytes::from_slice(&[0x01, 0x02]);
    /// assert_eq!(bytes.pop(), Some(0x02));
    /// assert_eq!(bytes.len(), 1);
    /// ```
    #[inline]
    pub fn pop(&mut self) -> Option<u8> {
        self.inner.pop()
    }
}

// =============================================================================
// Trait Implementations
// =============================================================================

impl PartialEq for Bytes {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl PartialOrd for Bytes {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Bytes {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl Hash for Bytes {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

// =============================================================================
// Conversions
// =============================================================================

impl From<Vec<u8>> for Bytes {
    #[inline]
    fn from(vec: Vec<u8>) -> Self {
        Self { inner: vec }
    }
}

impl From<&[u8]> for Bytes {
    #[inline]
    fn from(slice: &[u8]) -> Self {
        Self::from_slice(slice)
    }
}

impl<const N: usize> From<[u8; N]> for Bytes {
    #[inline]
    fn from(arr: [u8; N]) -> Self {
        Self {
            inner: arr.to_vec(),
        }
    }
}

impl<const N: usize> From<&[u8; N]> for Bytes {
    #[inline]
    fn from(arr: &[u8; N]) -> Self {
        Self {
            inner: arr.to_vec(),
        }
    }
}

impl From<Bytes> for Vec<u8> {
    #[inline]
    fn from(bytes: Bytes) -> Self {
        bytes.inner
    }
}

impl AsRef<[u8]> for Bytes {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

impl AsMut<[u8]> for Bytes {
    #[inline]
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.inner
    }
}

// =============================================================================
// Indexing
// =============================================================================

impl Index<usize> for Bytes {
    type Output = u8;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.inner[index]
    }
}

impl IndexMut<usize> for Bytes {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.inner[index]
    }
}

impl Index<core::ops::Range<usize>> for Bytes {
    type Output = [u8];

    #[inline]
    fn index(&self, range: core::ops::Range<usize>) -> &Self::Output {
        &self.inner[range]
    }
}

impl IndexMut<core::ops::Range<usize>> for Bytes {
    #[inline]
    fn index_mut(&mut self, range: core::ops::Range<usize>) -> &mut Self::Output {
        &mut self.inner[range]
    }
}

impl Index<core::ops::RangeTo<usize>> for Bytes {
    type Output = [u8];

    #[inline]
    fn index(&self, range: core::ops::RangeTo<usize>) -> &Self::Output {
        &self.inner[range]
    }
}

impl IndexMut<core::ops::RangeTo<usize>> for Bytes {
    #[inline]
    fn index_mut(&mut self, range: core::ops::RangeTo<usize>) -> &mut Self::Output {
        &mut self.inner[range]
    }
}

impl Index<core::ops::RangeFrom<usize>> for Bytes {
    type Output = [u8];

    #[inline]
    fn index(&self, range: core::ops::RangeFrom<usize>) -> &Self::Output {
        &self.inner[range]
    }
}

impl IndexMut<core::ops::RangeFrom<usize>> for Bytes {
    #[inline]
    fn index_mut(&mut self, range: core::ops::RangeFrom<usize>) -> &mut Self::Output {
        &mut self.inner[range]
    }
}

impl Index<core::ops::RangeFull> for Bytes {
    type Output = [u8];

    #[inline]
    fn index(&self, _: core::ops::RangeFull) -> &Self::Output {
        &self.inner[..]
    }
}

impl IndexMut<core::ops::RangeFull> for Bytes {
    #[inline]
    fn index_mut(&mut self, _: core::ops::RangeFull) -> &mut Self::Output {
        &mut self.inner[..]
    }
}

// =============================================================================
// Display and Parsing
// =============================================================================

impl fmt::Display for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x")?;
        for byte in &self.inner {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl FromStr for Bytes {
    type Err = BytesParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix("0x").unwrap_or(s);

        if !s.len().is_multiple_of(2) {
            return Err(BytesParseError::OddLength);
        }

        let mut bytes = Vec::with_capacity(s.len() / 2);
        for i in (0..s.len()).step_by(2) {
            let byte_str = &s[i..i + 2];
            let byte = u8::from_str_radix(byte_str, 16)
                .map_err(|_| BytesParseError::InvalidHex)?;
            bytes.push(byte);
        }

        Ok(Self { inner: bytes })
    }
}

/// Error type for parsing `Bytes` from a string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BytesParseError {
    /// The input string has an odd length.
    OddLength,
    /// The input string contains invalid hex characters.
    InvalidHex,
}

impl fmt::Display for BytesParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OddLength => write!(f, "hex string has odd length"),
            Self::InvalidHex => write!(f, "invalid hex character"),
        }
    }
}

// =============================================================================
// Serde
// =============================================================================

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut hex = String::with_capacity(2 + self.inner.len() * 2);
        hex.push_str("0x");
        for byte in &self.inner {
            use core::fmt::Write;
            write!(&mut hex, "{byte:02x}").unwrap();
        }
        serializer.serialize_str(&hex)
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let bytes = Bytes::new();
        assert_eq!(bytes.len(), 0);
        assert!(bytes.is_empty());
    }

    #[test]
    fn test_default() {
        let bytes = Bytes::default();
        assert_eq!(bytes.len(), 0);
        assert!(bytes.is_empty());
    }

    #[test]
    fn test_from_slice() {
        let data = [0x01, 0x02, 0x03];
        let bytes = Bytes::from_slice(&data);
        assert_eq!(bytes.len(), 3);
        assert_eq!(bytes.as_ref(), &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_with_capacity() {
        let bytes = Bytes::with_capacity(10);
        assert_eq!(bytes.len(), 0);
        assert!(bytes.capacity() >= 10);
    }

    #[test]
    fn test_push() {
        let mut bytes = Bytes::new();
        bytes.push(0x42);
        assert_eq!(bytes.len(), 1);
        assert_eq!(bytes[0], 0x42);
    }

    #[test]
    fn test_push_multiple() {
        let mut bytes = Bytes::new();
        bytes.push(0x01);
        bytes.push(0x02);
        bytes.push(0x03);
        assert_eq!(bytes.len(), 3);
        assert_eq!(bytes.as_ref(), &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_extend_from_slice() {
        let mut bytes = Bytes::new();
        bytes.extend_from_slice(&[0x01, 0x02]);
        assert_eq!(bytes.len(), 2);
        bytes.extend_from_slice(&[0x03, 0x04]);
        assert_eq!(bytes.len(), 4);
        assert_eq!(bytes.as_ref(), &[0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_truncate() {
        let mut bytes = Bytes::from_slice(&[0x01, 0x02, 0x03, 0x04]);
        bytes.truncate(2);
        assert_eq!(bytes.len(), 2);
        assert_eq!(bytes.as_ref(), &[0x01, 0x02]);
    }

    #[test]
    fn test_truncate_longer() {
        let mut bytes = Bytes::from_slice(&[0x01, 0x02]);
        bytes.truncate(10);
        assert_eq!(bytes.len(), 2);
        assert_eq!(bytes.as_ref(), &[0x01, 0x02]);
    }

    #[test]
    fn test_clear() {
        let mut bytes = Bytes::from_slice(&[0x01, 0x02, 0x03]);
        bytes.clear();
        assert_eq!(bytes.len(), 0);
        assert!(bytes.is_empty());
    }

    #[test]
    fn test_pop() {
        let mut bytes = Bytes::from_slice(&[0x01, 0x02, 0x03]);
        assert_eq!(bytes.pop(), Some(0x03));
        assert_eq!(bytes.len(), 2);
        assert_eq!(bytes.pop(), Some(0x02));
        assert_eq!(bytes.len(), 1);
        assert_eq!(bytes.pop(), Some(0x01));
        assert_eq!(bytes.len(), 0);
        assert_eq!(bytes.pop(), None);
    }

    #[test]
    fn test_reserve() {
        let mut bytes = Bytes::new();
        bytes.reserve(10);
        assert!(bytes.capacity() >= 10);
    }

    #[test]
    fn test_from_vec() {
        let vec = vec![0x01, 0x02, 0x03];
        let bytes = Bytes::from(vec);
        assert_eq!(bytes.len(), 3);
        assert_eq!(bytes.as_ref(), &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_from_slice_ref() {
        let data = [0x01, 0x02, 0x03];
        let bytes = Bytes::from(&data[..]);
        assert_eq!(bytes.len(), 3);
        assert_eq!(bytes.as_ref(), &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_from_array() {
        let data = [0x01, 0x02, 0x03];
        let bytes = Bytes::from(data);
        assert_eq!(bytes.len(), 3);
        assert_eq!(bytes.as_ref(), &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_from_array_ref() {
        let data = [0x01, 0x02, 0x03];
        let bytes = Bytes::from(&data);
        assert_eq!(bytes.len(), 3);
        assert_eq!(bytes.as_ref(), &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_into_vec() {
        let bytes = Bytes::from_slice(&[0x01, 0x02, 0x03]);
        let vec: Vec<u8> = bytes.into();
        assert_eq!(vec, vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_as_ref() {
        let bytes = Bytes::from_slice(&[0x01, 0x02, 0x03]);
        let slice: &[u8] = bytes.as_ref();
        assert_eq!(slice, &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_as_mut() {
        let mut bytes = Bytes::from_slice(&[0x01, 0x02, 0x03]);
        let slice: &mut [u8] = bytes.as_mut();
        slice[0] = 0xFF;
        assert_eq!(bytes.as_ref(), &[0xFF, 0x02, 0x03]);
    }

    #[test]
    fn test_clone() {
        let bytes = Bytes::from_slice(&[0x01, 0x02, 0x03]);
        let cloned = bytes.clone();
        assert_eq!(bytes, cloned);
    }

    #[test]
    fn test_eq() {
        let bytes1 = Bytes::from_slice(&[0x01, 0x02, 0x03]);
        let bytes2 = Bytes::from_slice(&[0x01, 0x02, 0x03]);
        let bytes3 = Bytes::from_slice(&[0x01, 0x02, 0x04]);
        assert_eq!(bytes1, bytes2);
        assert_ne!(bytes1, bytes3);
    }

    #[test]
    fn test_ord() {
        let bytes1 = Bytes::from_slice(&[0x01, 0x02]);
        let bytes2 = Bytes::from_slice(&[0x01, 0x03]);
        let bytes3 = Bytes::from_slice(&[0x02, 0x01]);
        assert!(bytes1 < bytes2);
        assert!(bytes1 < bytes3);
        assert!(bytes2 < bytes3);
    }

    #[test]
    fn test_hash() {
        use core::hash::{Hash, Hasher};

        struct SimpleHasher {
            state: u64,
        }

        impl Hasher for SimpleHasher {
            fn finish(&self) -> u64 {
                self.state
            }

            fn write(&mut self, bytes: &[u8]) {
                for &byte in bytes {
                    self.state = self.state.wrapping_mul(31).wrapping_add(byte as u64);
                }
            }
        }

        let bytes1 = Bytes::from_slice(&[0x01, 0x02, 0x03]);
        let bytes2 = Bytes::from_slice(&[0x01, 0x02, 0x03]);

        let mut hasher1 = SimpleHasher { state: 0 };
        bytes1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = SimpleHasher { state: 0 };
        bytes2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_index() {
        let bytes = Bytes::from_slice(&[0x01, 0x02, 0x03]);
        assert_eq!(bytes[0], 0x01);
        assert_eq!(bytes[1], 0x02);
        assert_eq!(bytes[2], 0x03);
    }

    #[test]
    fn test_index_mut() {
        let mut bytes = Bytes::from_slice(&[0x01, 0x02, 0x03]);
        bytes[0] = 0xFF;
        assert_eq!(bytes[0], 0xFF);
    }

    #[test]
    fn test_index_range() {
        let bytes = Bytes::from_slice(&[0x01, 0x02, 0x03, 0x04]);
        assert_eq!(&bytes[1..3], &[0x02, 0x03]);
    }

    #[test]
    fn test_index_range_to() {
        let bytes = Bytes::from_slice(&[0x01, 0x02, 0x03, 0x04]);
        assert_eq!(&bytes[..2], &[0x01, 0x02]);
    }

    #[test]
    fn test_index_range_from() {
        let bytes = Bytes::from_slice(&[0x01, 0x02, 0x03, 0x04]);
        assert_eq!(&bytes[2..], &[0x03, 0x04]);
    }

    #[test]
    fn test_index_range_full() {
        let bytes = Bytes::from_slice(&[0x01, 0x02, 0x03, 0x04]);
        assert_eq!(&bytes[..], &[0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_display_empty() {
        let bytes = Bytes::new();
        assert_eq!(bytes.to_string(), "0x");
    }

    #[test]
    fn test_display() {
        let bytes = Bytes::from_slice(&[0x01, 0x02, 0x03]);
        assert_eq!(bytes.to_string(), "0x010203");
    }

    #[test]
    fn test_display_with_leading_zeros() {
        let bytes = Bytes::from_slice(&[0x00, 0x01, 0x0a, 0xff]);
        assert_eq!(bytes.to_string(), "0x00010aff");
    }

    #[test]
    fn test_from_str() {
        let bytes: Bytes = "0x010203".parse().unwrap();
        assert_eq!(bytes.as_ref(), &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_from_str_without_prefix() {
        let bytes: Bytes = "010203".parse().unwrap();
        assert_eq!(bytes.as_ref(), &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_from_str_empty() {
        let bytes: Bytes = "0x".parse().unwrap();
        assert_eq!(bytes.len(), 0);
    }

    #[test]
    fn test_from_str_odd_length() {
        let result: Result<Bytes, _> = "0x123".parse();
        assert_eq!(result, Err(BytesParseError::OddLength));
    }

    #[test]
    fn test_from_str_invalid_hex() {
        let result: Result<Bytes, _> = "0x01zz".parse();
        assert_eq!(result, Err(BytesParseError::InvalidHex));
    }

    #[test]
    fn test_serialize() {
        let bytes = Bytes::from_slice(&[0x01, 0x02, 0x03]);
        let json = serde_json::to_string(&bytes).unwrap();
        assert_eq!(json, "\"0x010203\"");
    }

    #[test]
    fn test_serialize_empty() {
        let bytes = Bytes::new();
        let json = serde_json::to_string(&bytes).unwrap();
        assert_eq!(json, "\"0x\"");
    }

    #[test]
    fn test_deserialize() {
        let json = "\"0x010203\"";
        let bytes: Bytes = serde_json::from_str(json).unwrap();
        assert_eq!(bytes.as_ref(), &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_deserialize_without_prefix() {
        let json = "\"010203\"";
        let bytes: Bytes = serde_json::from_str(json).unwrap();
        assert_eq!(bytes.as_ref(), &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_deserialize_empty() {
        let json = "\"0x\"";
        let bytes: Bytes = serde_json::from_str(json).unwrap();
        assert_eq!(bytes.len(), 0);
    }

    #[test]
    fn test_roundtrip_serde() {
        let original = Bytes::from_slice(&[0x01, 0x02, 0x03, 0xff]);
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Bytes = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_empty_bytes() {
        let bytes = Bytes::new();
        assert_eq!(bytes.len(), 0);
        assert!(bytes.is_empty());
        assert_eq!(bytes.as_ref(), &[] as &[u8]);
        assert_eq!(bytes.to_string(), "0x");
    }

    #[test]
    fn test_large_bytes() {
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let bytes = Bytes::from(data.clone());
        assert_eq!(bytes.len(), 1000);
        assert_eq!(bytes.as_ref(), &data[..]);
    }

    #[test]
    fn test_debug() {
        let bytes = Bytes::from_slice(&[0x01, 0x02, 0x03]);
        let debug = format!("{bytes:?}");
        assert!(debug.contains("Bytes"));
    }

    #[test]
    #[should_panic]
    fn test_index_out_of_bounds() {
        let bytes = Bytes::from_slice(&[0x01, 0x02]);
        let _ = bytes[10];
    }

    #[test]
    #[should_panic]
    fn test_index_mut_out_of_bounds() {
        let mut bytes = Bytes::from_slice(&[0x01, 0x02]);
        bytes[10] = 0xFF;
    }
}
