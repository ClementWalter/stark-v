//! Cryptographic primitives
//!
//! This module provides cryptographic functions required for Ethereum:
//! - Keccak256 hashing
//! - ECDSA signature verification (secp256k1)
//! - RLP encoding/decoding

pub mod keccak;
pub mod rlp;
pub mod secp256k1;
pub(crate) mod secp256k1_math;
pub mod secp256k1_point;

// Re-export Keccak-256 function
pub use keccak::keccak256;

// Re-export secp256k1 functions
pub use secp256k1::{
    address_from_public_key, address_from_secret_key, public_key_from_secret, recover_address,
    recover_public_key, sign_recoverable, verify_signature, Secp256k1Error,
};

// Re-export RLP functions
pub use rlp::{
    decode_address, decode_byte, decode_bytes, decode_hash, decode_list, decode_u256, decode_u512,
    decode_u64, encode_address, encode_byte, encode_bytes, encode_hash, encode_list, encode_u256,
    encode_u512, encode_u64, RlpError,
};
