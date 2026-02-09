//! Partial Merkle Patricia Trie implementation
//!
//! This module implements a partial MPT for Ethereum state verification in zkVM environments.
//! It supports proving state transitions without requiring the full trie.

pub mod node;
pub mod proof;
pub mod trie;

pub use node::{
    Node, NodeError, bytes_to_nibbles, common_prefix_length, decode_compact_path,
    encode_compact_path, nibbles_to_bytes,
};
pub use proof::{Proof, ProofError, verify_proof};
pub use trie::{EMPTY_TRIE_ROOT, Trie};
