//! SHA256 hash computation example.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Result of SHA256 computation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Sha2Result {
    pub input_len: u32,
    pub hash: [u8; 32],
}

/// Compute SHA256 hash of input bytes returning a result struct.
pub fn sha256(input: &[u8]) -> Sha2Result {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let hash: [u8; 32] = hasher.finalize().into();
    Sha2Result {
        input_len: input.len() as u32,
        hash,
    }
}

/// Standard test entry point for e2e testing.
pub fn test_call() -> Sha2Result {
    sha256(b"hello world")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        let result = sha256(b"hello world");
        assert_eq!(result.input_len, 11);
        // Expected SHA256 of "hello world"
        let expected_hash = [
            0xb9, 0x4d, 0x27, 0xb9, 0x93, 0x4d, 0x3e, 0x08, 0xa5, 0x2e, 0x52, 0xd7, 0xda, 0x7d,
            0xab, 0xfa, 0xc4, 0x84, 0xef, 0xe3, 0x7a, 0x53, 0x80, 0xee, 0x90, 0x88, 0xf7, 0xac,
            0xe2, 0xef, 0xcd, 0xe9,
        ];
        assert_eq!(result.hash, expected_hash);
    }

    #[test]
    fn test_sha256_empty() {
        let result = sha256(b"");
        assert_eq!(result.input_len, 0);
        // Expected SHA256 of empty string
        let expected_hash = [
            0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
            0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
            0x78, 0x52, 0xb8, 0x55,
        ];
        assert_eq!(result.hash, expected_hash);
    }

    #[test]
    fn test_call_result() {
        let result = test_call();
        assert_eq!(result.input_len, 11);
    }
}
