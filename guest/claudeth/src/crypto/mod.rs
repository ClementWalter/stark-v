//! Cryptographic primitives
//!
//! This module provides cryptographic functions required for Ethereum:
//! - Keccak256 hashing
//! - ECDSA signature verification
//! - RLP encoding/decoding

pub mod rlp;

// Re-export RLP functions
pub use rlp::{
    decode_address, decode_byte, decode_bytes, decode_hash, decode_list, decode_u256, decode_u512,
    decode_u64, encode_address, encode_byte, encode_bytes, encode_hash, encode_list, encode_u256,
    encode_u512, encode_u64, RlpError,
};
