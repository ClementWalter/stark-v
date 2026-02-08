//! Keccak-256 cryptographic hash function
//!
//! This module provides a wrapper around the sha3 crate's Keccak-256
//! implementation, providing a convenient interface for Ethereum hashing.

#[cfg(target_arch = "riscv32")]
extern crate alloc;

use sha3::{Digest, Keccak256};

use crate::types::Hash;

/// Computes the Keccak-256 hash of the input data.
///
/// # Examples
///
/// ```
/// use claudeth::crypto::keccak256;
/// use claudeth::types::Hash;
///
/// let data = b"hello world";
/// let hash = keccak256(data);
/// assert_eq!(hash.as_bytes().len(), 32);
/// ```
///
/// # Test Vector
///
/// ```
/// use claudeth::crypto::keccak256;
///
/// // Official Ethereum test vector: keccak256("") = c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470
/// let empty_hash = keccak256(b"");
/// let expected = [
///     0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c,
///     0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7, 0x03, 0xc0,
///     0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b,
///     0x7b, 0xfa, 0xd8, 0x04, 0x5d, 0x85, 0xa4, 0x70,
/// ];
/// assert_eq!(empty_hash.as_bytes(), &expected);
/// ```
pub fn keccak256(input: &[u8]) -> Hash {
    let mut hasher = Keccak256::new();
    hasher.update(input);
    let result = hasher.finalize();

    // Convert GenericArray to [u8; 32]
    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(&result);

    Hash::from(hash_bytes)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Basic Functionality Tests
    // =========================================================================

    #[test]
    fn test_keccak256_empty() {
        // Official Ethereum test vector: keccak256("") = c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470
        let hash = keccak256(b"");
        let expected = [
            0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c,
            0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7, 0x03, 0xc0,
            0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b,
            0x7b, 0xfa, 0xd8, 0x04, 0x5d, 0x85, 0xa4, 0x70,
        ];
        assert_eq!(hash.as_bytes(), &expected);
    }

    #[test]
    fn test_keccak256_hello_world() {
        // Test vector from Ethereum tests
        let hash = keccak256(b"hello world");
        let expected = [
            0x47, 0x17, 0x32, 0x85, 0xa8, 0xd7, 0x34, 0x1e,
            0x5e, 0x97, 0x2f, 0xc6, 0x77, 0x28, 0x63, 0x84,
            0xf8, 0x02, 0xf8, 0xef, 0x42, 0xa5, 0xec, 0x5f,
            0x03, 0xbb, 0xfa, 0x25, 0x4c, 0xb0, 0x1f, 0xad,
        ];
        assert_eq!(hash.as_bytes(), &expected);
    }

    #[test]
    fn test_keccak256_single_byte() {
        // Test with single byte 0x00
        let hash = keccak256(&[0x00]);
        let expected = [
            0xbc, 0x36, 0x78, 0x9e, 0x7a, 0x1e, 0x28, 0x14,
            0x36, 0x46, 0x42, 0x29, 0x82, 0x8f, 0x81, 0x7d,
            0x66, 0x12, 0xf7, 0xb4, 0x77, 0xd6, 0x65, 0x91,
            0xff, 0x96, 0xa9, 0xe0, 0x64, 0xbc, 0xc9, 0x8a,
        ];
        assert_eq!(hash.as_bytes(), &expected);
    }

    #[test]
    fn test_keccak256_ethereum_address() {
        // Test hashing an Ethereum address (20 bytes)
        let address = [0x42u8; 20];
        let hash = keccak256(&address);
        // Just verify it produces a 32-byte hash
        assert_eq!(hash.as_bytes().len(), 32);
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_keccak256_large_input() {
        // Test with 1KB of data
        let data = vec![0x42u8; 1024];
        let hash = keccak256(&data);
        assert_eq!(hash.as_bytes().len(), 32);
    }

    #[test]
    fn test_keccak256_very_large_input() {
        // Test with 1MB of data
        let data = vec![0x42u8; 1024 * 1024];
        let hash = keccak256(&data);
        assert_eq!(hash.as_bytes().len(), 32);
    }

    // =========================================================================
    // Determinism Tests
    // =========================================================================

    #[test]
    fn test_keccak256_deterministic() {
        let data = b"test data";
        let hash1 = keccak256(data);
        let hash2 = keccak256(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_keccak256_different_inputs() {
        let hash1 = keccak256(b"input1");
        let hash2 = keccak256(b"input2");
        assert_ne!(hash1, hash2);
    }

    // =========================================================================
    // Ethereum Compatibility Tests
    // =========================================================================

    #[test]
    fn test_keccak256_ethereum_function_selector() {
        // Function selector for "transfer(address,uint256)" should be 0xa9059cbb...
        let hash = keccak256(b"transfer(address,uint256)");
        let selector = &hash.as_bytes()[0..4];
        assert_eq!(selector, &[0xa9, 0x05, 0x9c, 0xbb]);
    }

    #[test]
    fn test_keccak256_ethereum_event_signature() {
        // Event signature for "Transfer(address,address,uint256)"
        let hash = keccak256(b"Transfer(address,address,uint256)");
        let expected = [
            0xdd, 0xf2, 0x52, 0xad, 0x1b, 0xe2, 0xc8, 0x9b,
            0x69, 0xc2, 0xb0, 0x68, 0xfc, 0x37, 0x8d, 0xaa,
            0x95, 0x2b, 0xa7, 0xf1, 0x63, 0xc4, 0xa1, 0x16,
            0x28, 0xf5, 0x5a, 0x4d, 0xf5, 0x23, 0xb3, 0xef,
        ];
        assert_eq!(hash.as_bytes(), &expected);
    }

    // =========================================================================
    // Additional Test Vectors
    // =========================================================================

    #[test]
    fn test_keccak256_abc() {
        // Test vector for "abc"
        let hash = keccak256(b"abc");
        let expected = [
            0x4e, 0x03, 0x65, 0x7a, 0xea, 0x45, 0xa9, 0x4f,
            0xc7, 0xd4, 0x7b, 0xa8, 0x26, 0xc8, 0xd6, 0x67,
            0xc0, 0xd1, 0xe6, 0xe3, 0x3a, 0x64, 0xa0, 0x36,
            0xec, 0x44, 0xf5, 0x8f, 0xa1, 0x2d, 0x6c, 0x45,
        ];
        assert_eq!(hash.as_bytes(), &expected);
    }

    #[test]
    fn test_keccak256_alphanumeric() {
        // Test vector for alphanumeric string
        let hash = keccak256(b"The quick brown fox jumps over the lazy dog");
        let expected = [
            0x4d, 0x74, 0x1b, 0x6f, 0x1e, 0xb2, 0x9c, 0xb2,
            0xa9, 0xb9, 0x91, 0x1c, 0x82, 0xf5, 0x6f, 0xa8,
            0xd7, 0x3b, 0x04, 0x95, 0x9d, 0x3d, 0x9d, 0x22,
            0x28, 0x95, 0xdf, 0x6c, 0x0b, 0x28, 0xaa, 0x15,
        ];
        assert_eq!(hash.as_bytes(), &expected);
    }

    #[test]
    fn test_keccak256_repeated_pattern() {
        // Test with repeated pattern
        let data = b"abababababababababababababababab";
        let hash = keccak256(data);
        // Just verify it produces a valid hash
        assert_eq!(hash.as_bytes().len(), 32);
        assert_ne!(hash, Hash::ZERO);
    }
}
