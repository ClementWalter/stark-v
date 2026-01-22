//! Keccak-256 hash computation example.

use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

/// Result of Keccak-256 computation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeccakResult {
    pub input_len: u32,
    pub hash: [u8; 32],
}

/// Compute Keccak-256 hash of input bytes returning a result struct.
pub fn keccak256(input: &[u8]) -> KeccakResult {
    let mut hasher = Keccak256::new();
    hasher.update(input);
    let hash: [u8; 32] = hasher.finalize().into();
    KeccakResult {
        input_len: input.len() as u32,
        hash,
    }
}

/// Standard test entry point for e2e testing.
pub fn test_call() -> KeccakResult {
    keccak256(b"hello world")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keccak256() {
        let result = keccak256(b"hello world");
        assert_eq!(result.input_len, 11);
        // Expected Keccak-256 of "hello world"
        let expected_hash = [
            0x47, 0x17, 0x32, 0x85, 0xa8, 0xd7, 0x34, 0x1e, 0x5e, 0x97, 0x2f, 0xc6, 0x77, 0x28,
            0x63, 0x84, 0xf8, 0x02, 0xf8, 0xef, 0x42, 0xa5, 0xec, 0x5f, 0x03, 0xbb, 0xfa, 0x25,
            0x4c, 0xb0, 0x1f, 0xad,
        ];
        assert_eq!(result.hash, expected_hash);
    }

    #[test]
    fn test_keccak256_empty() {
        let result = keccak256(b"");
        assert_eq!(result.input_len, 0);
        // Expected Keccak-256 of empty string
        let expected_hash = [
            0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c, 0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7,
            0x03, 0xc0, 0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b, 0x7b, 0xfa, 0xd8, 0x04,
            0x5d, 0x85, 0xa4, 0x70,
        ];
        assert_eq!(result.hash, expected_hash);
    }

    #[test]
    fn test_call_result() {
        let result = test_call();
        assert_eq!(result.input_len, 11);
    }
}
