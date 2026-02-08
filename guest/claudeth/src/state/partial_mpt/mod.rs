//! Partial Merkle Patricia Trie implementation
//!
//! This module implements a partial MPT for Ethereum state verification in zkVM environments.
//! It supports proving state transitions without requiring the full trie.

pub mod node;

pub use node::{
    Node, NodeError,
    bytes_to_nibbles, nibbles_to_bytes, common_prefix_length,
    encode_compact_path, decode_compact_path,
};
