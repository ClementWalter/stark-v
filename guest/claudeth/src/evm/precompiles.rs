//! EVM precompile implementations (subset).
//!
//! Currently implements:
//! - 0x01: ECRECOVER
//! - 0x04: IDENTITY

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

use crate::crypto::secp256k1::recover_address;
use crate::crypto::secp256k1_math::secp256k1_n;
use crate::crypto::{ripemd160, sha256};
use crate::evm::gas::{GAS_ECRECOVER, GAS_IDENTITY_BASE, GAS_IDENTITY_WORD};
use crate::evm::gas::{GAS_RIPEMD160_BASE, GAS_RIPEMD160_WORD, GAS_SHA256_BASE, GAS_SHA256_WORD};
use crate::types::{Address, Hash, U256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecompileResult {
    pub output: Vec<u8>,
    pub gas_used: u64,
}

pub fn execute_precompile(address: &Address, input: &[u8]) -> Option<PrecompileResult> {
    let id = precompile_id(address)?;
    match id {
        1 => Some(ecrecover_precompile(input)),
        2 => Some(sha256_precompile(input)),
        3 => Some(ripemd160_precompile(input)),
        4 => Some(identity_precompile(input)),
        _ => None,
    }
}

fn precompile_id(address: &Address) -> Option<u8> {
    let bytes = address.as_bytes();
    if bytes[..19].iter().any(|&b| b != 0) {
        return None;
    }
    let id = bytes[19];
    if id == 0 {
        None
    } else {
        Some(id)
    }
}

fn identity_precompile(input: &[u8]) -> PrecompileResult {
    let word_count = (input.len() as u64).div_ceil(32);
    PrecompileResult {
        output: input.to_vec(),
        gas_used: GAS_IDENTITY_BASE + GAS_IDENTITY_WORD * word_count,
    }
}

fn sha256_precompile(input: &[u8]) -> PrecompileResult {
    // Precompile gas depends on input length, so we round up to 32-byte words.
    let word_count = (input.len() as u64).div_ceil(32);
    let digest = sha256(input);
    PrecompileResult {
        output: digest.to_vec(),
        gas_used: GAS_SHA256_BASE + GAS_SHA256_WORD * word_count,
    }
}

fn ripemd160_precompile(input: &[u8]) -> PrecompileResult {
    // Precompile gas depends on input length, so we round up to 32-byte words.
    let word_count = (input.len() as u64).div_ceil(32);
    let digest = ripemd160(input);
    let mut output = vec![0u8; 32];
    // RIPEMD-160 returns 20 bytes, so we left-pad to 32 bytes to match EVM output rules.
    output[12..].copy_from_slice(&digest);
    PrecompileResult {
        output,
        gas_used: GAS_RIPEMD160_BASE + GAS_RIPEMD160_WORD * word_count,
    }
}

fn ecrecover_precompile(input: &[u8]) -> PrecompileResult {
    let message_hash = Hash::from(read_padded_word(input, 0));
    let v = U256::from_be_bytes(read_padded_word(input, 32));
    let r = U256::from_be_bytes(read_padded_word(input, 64));
    let s = U256::from_be_bytes(read_padded_word(input, 96));

    let output = if v == U256::from_u64(27) || v == U256::from_u64(28) {
        let n = secp256k1_n();
        if r.is_zero() || s.is_zero() || r >= n || s >= n {
            Vec::new()
        } else {
            let mut signature = [0u8; 64];
            signature[..32].copy_from_slice(&r.to_be_bytes());
            signature[32..].copy_from_slice(&s.to_be_bytes());
            let recovery_id = (v - U256::from_u64(27)).as_u64() as u8;
            match recover_address(&message_hash, &signature, recovery_id) {
                Ok(address) => {
                    let mut out = vec![0u8; 32];
                    out[12..].copy_from_slice(address.as_bytes());
                    out
                }
                Err(_) => Vec::new(),
            }
        }
    } else {
        Vec::new()
    };

    PrecompileResult {
        output,
        gas_used: GAS_ECRECOVER,
    }
}

fn read_padded_word(input: &[u8], offset: usize) -> [u8; 32] {
    let mut word = [0u8; 32];
    if offset >= input.len() {
        return word;
    }
    let end = (offset + 32).min(input.len());
    word[..(end - offset)].copy_from_slice(&input[offset..end]);
    word
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::secp256k1::{address_from_secret_key, sign_recoverable};

    #[test]
    fn test_identity_precompile_output_and_gas() {
        let input = vec![0x01, 0x02, 0x03];
        let addr = precompile_address(4);
        let result = execute_precompile(&addr, &input).expect("precompile result");
        assert_eq!(result.output, input);
        assert_eq!(result.gas_used, GAS_IDENTITY_BASE + GAS_IDENTITY_WORD);
    }

    #[test]
    fn test_ecrecover_precompile_success() {
        let message_hash = Hash::from([0x11u8; 32]);
        let secret = U256::from_u64(42);
        let (r, s, recid) = sign_recoverable(&message_hash, secret).expect("signature");
        let v = U256::from_u64(recid as u64 + 27);

        let mut input = vec![0u8; 128];
        input[..32].copy_from_slice(message_hash.as_bytes());
        input[32..64].copy_from_slice(&v.to_be_bytes());
        input[64..96].copy_from_slice(&r.to_be_bytes());
        input[96..128].copy_from_slice(&s.to_be_bytes());

        let addr = precompile_address(1);
        let result = execute_precompile(&addr, &input).expect("precompile result");

        let expected = address_from_secret_key(secret).expect("address");
        let mut expected_output = vec![0u8; 32];
        expected_output[12..].copy_from_slice(expected.as_bytes());

        assert_eq!(result.output, expected_output);
        assert_eq!(result.gas_used, GAS_ECRECOVER);
    }

    #[test]
    fn test_ecrecover_precompile_invalid_v_returns_empty() {
        let input = vec![0u8; 128];
        let addr = precompile_address(1);
        let result = execute_precompile(&addr, &input).expect("precompile result");
        assert!(result.output.is_empty());
        assert_eq!(result.gas_used, GAS_ECRECOVER);
    }

    #[test]
    fn test_sha256_precompile_output_and_gas() {
        let input = b"abc".to_vec();
        let addr = precompile_address(2);
        let result = execute_precompile(&addr, &input).expect("precompile result");
        let expected = vec![
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d,
            0xae, 0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10,
            0xff, 0x61, 0xf2, 0x00, 0x15, 0xad,
        ];
        assert_eq!(result.output, expected);
        assert_eq!(result.gas_used, GAS_SHA256_BASE + GAS_SHA256_WORD);
    }

    #[test]
    fn test_ripemd160_precompile_output_and_gas() {
        let input = b"abc".to_vec();
        let addr = precompile_address(3);
        let result = execute_precompile(&addr, &input).expect("precompile result");
        let expected = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8e,
            0xb2, 0x08, 0xf7, 0xe0, 0x5d, 0x98, 0x7a, 0x9b, 0x04, 0x4a, 0x8e, 0x98, 0xc6,
            0xb0, 0x87, 0xf1, 0x5a, 0x0b, 0xfc,
        ];
        assert_eq!(result.output, expected);
        assert_eq!(result.gas_used, GAS_RIPEMD160_BASE + GAS_RIPEMD160_WORD);
    }

    fn precompile_address(id: u8) -> Address {
        let mut bytes = [0u8; 20];
        bytes[19] = id;
        Address::from(bytes)
    }
}
