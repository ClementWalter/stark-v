//! Cryptographic primitives
//!
//! This module provides cryptographic functions required for Ethereum:
//! - Keccak256 hashing
//! - ECDSA signature verification (secp256k1)
//! - RLP encoding/decoding

pub mod keccak;
pub mod rlp;
pub mod secp256k1;

// Re-export Keccak-256 function
pub use keccak::keccak256;

// Re-export secp256k1 functions
pub use secp256k1::{Secp256k1Error, recover_address, recover_public_key, verify_signature};

// Re-export RLP functions
pub use rlp::{
    RlpError, decode_address, decode_byte, decode_bytes, decode_hash, decode_list, decode_u64,
    decode_u256, decode_u512, encode_address, encode_byte, encode_bytes, encode_hash, encode_list,
    encode_u64, encode_u256, encode_u512,
};
