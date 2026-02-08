//! State management and Merkle Patricia Trie

pub mod partial_mpt;

pub use partial_mpt::{
    Node, NodeError,
    bytes_to_nibbles, nibbles_to_bytes, common_prefix_length,
    encode_compact_path, decode_compact_path,
};
