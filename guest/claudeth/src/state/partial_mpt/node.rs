//! Merkle Patricia Trie node types and encoding
//!
//! This module implements the three node types used in Ethereum's Modified Merkle Patricia Trie:
//! - **Leaf**: Stores a key-value pair at the end of a path
//! - **Extension**: Compresses a common path prefix
//! - **Branch**: Represents a branching point with up to 16 children (one for each nibble 0-F)
//!
//! ## Path Encoding
//!
//! Paths in the MPT are sequences of nibbles (4-bit values). Ethereum uses "compact encoding"
//! (also called "hex-prefix encoding") to distinguish leaf vs extension nodes and handle
//! odd-length paths:
//!
//! - Leaf with even-length path: `[0x20, ...nibbles_packed...]`
//! - Leaf with odd-length path: `[0x3X, ...nibbles_packed...]` where X is first nibble
//! - Extension with even-length path: `[0x00, ...nibbles_packed...]`
//! - Extension with odd-length path: `[0x1X, ...nibbles_packed...]` where X is first nibble
//!
//! ## RLP Encoding
//!
//! - Leaf: `RLP([compact_path, value])`
//! - Extension: `RLP([compact_path, child_hash])`
//! - Branch: `RLP([child0, ..., child15, value])` (17 elements)
//!
//! See: https://ethereum.org/en/developers/docs/data-structures-and-encoding/patricia-merkle-trie/

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::{boxed::Box, vec, vec::Vec};

use crate::crypto::{keccak256, rlp};
use crate::types::Hash;

// =============================================================================
// Node Types
// =============================================================================

/// A node in the Merkle Patricia Trie
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    /// Leaf node: stores a key-value pair
    ///
    /// The key_suffix is the remaining nibbles from the path that led to this leaf.
    /// The value is the RLP-encoded data stored at this key.
    Leaf {
        key_suffix: Vec<u8>,
        value: Vec<u8>,
    },

    /// Extension node: compresses a common path prefix
    ///
    /// The prefix is the shared nibbles between multiple keys.
    /// The child_hash is the hash of the next node in the path.
    Extension {
        prefix: Vec<u8>,
        child_hash: Hash,
    },

    /// Branch node: represents a branching point with up to 16 children
    ///
    /// Each child corresponds to one nibble value (0-F).
    /// The optional value is stored if this branch is also a key endpoint.
    /// The children array is boxed to reduce the size of the enum variant.
    Branch {
        children: Box<[Option<Hash>; 16]>,
        value: Option<Vec<u8>>,
    },
}

impl Node {
    /// Creates a new leaf node
    pub fn new_leaf(key_suffix: Vec<u8>, value: Vec<u8>) -> Self {
        Node::Leaf { key_suffix, value }
    }

    /// Creates a new extension node
    pub fn new_extension(prefix: Vec<u8>, child_hash: Hash) -> Self {
        Node::Extension { prefix, child_hash }
    }

    /// Creates a new empty branch node
    pub fn new_branch() -> Self {
        Node::Branch {
            children: Box::new([None; 16]),
            value: None,
        }
    }

    /// Creates a new branch node with a value
    pub fn new_branch_with_value(value: Vec<u8>) -> Self {
        Node::Branch {
            children: Box::new([None; 16]),
            value: Some(value),
        }
    }

    /// Computes the hash of this node using Keccak-256
    ///
    /// If the RLP encoding is less than 32 bytes, the raw RLP is returned as a hash.
    /// Otherwise, the Keccak-256 hash of the RLP encoding is returned.
    pub fn compute_hash(&self) -> Hash {
        let encoded = self.encode_rlp();

        // If the RLP encoding is less than 32 bytes, use it directly
        // This is an optimization in Ethereum to avoid hashing small nodes
        if encoded.len() < 32 {
            // Pad with zeros to make it 32 bytes
            let mut hash_bytes = [0u8; 32];
            hash_bytes[..encoded.len()].copy_from_slice(&encoded);
            Hash::from(hash_bytes)
        } else {
            keccak256(&encoded)
        }
    }

    /// Encodes the node as RLP
    pub fn encode_rlp(&self) -> Vec<u8> {
        match self {
            Node::Leaf { key_suffix, value } => {
                let compact_path = encode_compact_path(key_suffix, true);
                let items = vec![
                    rlp::encode_bytes(&compact_path),
                    rlp::encode_bytes(value),
                ];
                rlp::encode_list(&items)
            }
            Node::Extension { prefix, child_hash } => {
                let compact_path = encode_compact_path(prefix, false);
                let items = vec![
                    rlp::encode_bytes(&compact_path),
                    rlp::encode_hash(child_hash),
                ];
                rlp::encode_list(&items)
            }
            Node::Branch { children, value } => {
                let mut items = Vec::with_capacity(17);

                // Encode all 16 children
                for child in children.iter() {
                    if let Some(hash) = child {
                        items.push(rlp::encode_hash(hash));
                    } else {
                        items.push(rlp::encode_bytes(&[]));
                    }
                }

                // Encode the value (17th element)
                if let Some(val) = value {
                    items.push(rlp::encode_bytes(val));
                } else {
                    items.push(rlp::encode_bytes(&[]));
                }

                rlp::encode_list(&items)
            }
        }
    }

    /// Decodes a node from RLP
    pub fn decode_rlp(data: &[u8]) -> Result<Self, NodeError> {
        let (items, _rest) = rlp::decode_list(data)
            .map_err(|_| NodeError::InvalidRlp)?;

        if items.len() == 2 {
            // Either Leaf or Extension
            let (compact_path, _) = rlp::decode_bytes(&items[0])
                .map_err(|_| NodeError::InvalidRlp)?;
            let (second_item, _) = rlp::decode_bytes(&items[1])
                .map_err(|_| NodeError::InvalidRlp)?;

            let (nibbles, is_leaf) = decode_compact_path(&compact_path)?;

            if is_leaf {
                Ok(Node::Leaf {
                    key_suffix: nibbles,
                    value: second_item,
                })
            } else {
                // Extension node - second item should be a hash
                if second_item.len() != 32 {
                    return Err(NodeError::InvalidHashLength);
                }
                let mut hash_bytes = [0u8; 32];
                hash_bytes.copy_from_slice(&second_item);
                Ok(Node::Extension {
                    prefix: nibbles,
                    child_hash: Hash::from(hash_bytes),
                })
            }
        } else if items.len() == 17 {
            // Branch node
            let mut children = Box::new([None; 16]);

            for (i, item) in items.iter().take(16).enumerate() {
                let (bytes, _) = rlp::decode_bytes(item)
                    .map_err(|_| NodeError::InvalidRlp)?;

                if !bytes.is_empty() {
                    if bytes.len() != 32 {
                        return Err(NodeError::InvalidHashLength);
                    }
                    let mut hash_bytes = [0u8; 32];
                    hash_bytes.copy_from_slice(&bytes);
                    children[i] = Some(Hash::from(hash_bytes));
                }
            }

            // Decode value (17th element)
            let (value_bytes, _) = rlp::decode_bytes(&items[16])
                .map_err(|_| NodeError::InvalidRlp)?;

            let value = if value_bytes.is_empty() {
                None
            } else {
                Some(value_bytes)
            };

            Ok(Node::Branch { children, value })
        } else {
            Err(NodeError::InvalidNodeStructure)
        }
    }
}

// =============================================================================
// Path Encoding
// =============================================================================

/// Encodes a nibble path using compact (hex-prefix) encoding
///
/// # Compact Encoding Rules
///
/// - Leaf with even-length path: `[0x20, ...nibbles_packed...]`
/// - Leaf with odd-length path: `[0x3X, ...nibbles_packed...]` where X is first nibble
/// - Extension with even-length path: `[0x00, ...nibbles_packed...]`
/// - Extension with odd-length path: `[0x1X, ...nibbles_packed...]` where X is first nibble
///
/// # Examples
///
/// ```
/// use claudeth::state::partial_mpt::node::encode_compact_path;
///
/// // Leaf with even-length path [0, 1, 2, 3]
/// let encoded = encode_compact_path(&[0, 1, 2, 3], true);
/// assert_eq!(encoded, vec![0x20, 0x01, 0x23]);
///
/// // Extension with odd-length path [1, 2, 3]
/// let encoded = encode_compact_path(&[1, 2, 3], false);
/// assert_eq!(encoded, vec![0x11, 0x23]);
/// ```
pub fn encode_compact_path(nibbles: &[u8], is_leaf: bool) -> Vec<u8> {
    let is_odd = nibbles.len() % 2 == 1;

    let mut output = Vec::new();

    if is_odd {
        // Odd length: flags and first nibble in first byte
        let flags = if is_leaf { 0x30 } else { 0x10 };
        output.push(flags | nibbles[0]);

        // Pack remaining nibbles
        for chunk in nibbles[1..].chunks(2) {
            let high = chunk[0];
            let low = chunk.get(1).copied().unwrap_or(0);
            output.push((high << 4) | low);
        }
    } else {
        // Even length: flags in first byte
        let flags = if is_leaf { 0x20 } else { 0x00 };
        output.push(flags);

        // Pack all nibbles
        for chunk in nibbles.chunks(2) {
            let high = chunk[0];
            let low = chunk.get(1).copied().unwrap_or(0);
            output.push((high << 4) | low);
        }
    }

    output
}

/// Decodes a compact (hex-prefix) encoded path
///
/// Returns (nibbles, is_leaf)
pub fn decode_compact_path(data: &[u8]) -> Result<(Vec<u8>, bool), NodeError> {
    if data.is_empty() {
        return Err(NodeError::EmptyCompactPath);
    }

    let first_byte = data[0];
    let flags = first_byte >> 4;

    let is_leaf = (flags & 0x02) != 0;
    let is_odd = (flags & 0x01) != 0;

    let mut nibbles = Vec::new();

    if is_odd {
        // First nibble is in the first byte
        nibbles.push(first_byte & 0x0F);
    }

    // Unpack remaining bytes
    for &byte in &data[1..] {
        nibbles.push(byte >> 4);
        nibbles.push(byte & 0x0F);
    }

    Ok((nibbles, is_leaf))
}

// =============================================================================
// Nibble Utilities
// =============================================================================

/// Converts bytes to nibbles (one nibble = 4 bits)
///
/// # Example
///
/// ```
/// use claudeth::state::partial_mpt::node::bytes_to_nibbles;
///
/// let bytes = vec![0x12, 0x34, 0x56];
/// let nibbles = bytes_to_nibbles(&bytes);
/// assert_eq!(nibbles, vec![1, 2, 3, 4, 5, 6]);
/// ```
pub fn bytes_to_nibbles(bytes: &[u8]) -> Vec<u8> {
    let mut nibbles = Vec::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        nibbles.push(byte >> 4);
        nibbles.push(byte & 0x0F);
    }
    nibbles
}

/// Converts nibbles back to bytes
///
/// If the number of nibbles is odd, the last nibble is padded with 0.
///
/// # Example
///
/// ```
/// use claudeth::state::partial_mpt::node::nibbles_to_bytes;
///
/// let nibbles = vec![1, 2, 3, 4, 5, 6];
/// let bytes = nibbles_to_bytes(&nibbles);
/// assert_eq!(bytes, vec![0x12, 0x34, 0x56]);
/// ```
pub fn nibbles_to_bytes(nibbles: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(nibbles.len().div_ceil(2));
    for chunk in nibbles.chunks(2) {
        let high = chunk[0];
        let low = chunk.get(1).copied().unwrap_or(0);
        bytes.push((high << 4) | low);
    }
    bytes
}

/// Returns the length of the common prefix between two nibble slices
///
/// # Example
///
/// ```
/// use claudeth::state::partial_mpt::node::common_prefix_length;
///
/// let a = vec![1, 2, 3, 4, 5];
/// let b = vec![1, 2, 3, 9, 8];
/// assert_eq!(common_prefix_length(&a, &b), 3);
/// ```
pub fn common_prefix_length(a: &[u8], b: &[u8]) -> usize {
    let mut count = 0;
    for (byte_a, byte_b) in a.iter().zip(b.iter()) {
        if byte_a != byte_b {
            break;
        }
        count += 1;
    }
    count
}

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during node operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeError {
    /// Invalid RLP encoding
    InvalidRlp,
    /// Invalid node structure (not 2 or 17 items)
    InvalidNodeStructure,
    /// Invalid hash length (not 32 bytes)
    InvalidHashLength,
    /// Empty compact path
    EmptyCompactPath,
}

impl core::fmt::Display for NodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NodeError::InvalidRlp => write!(f, "invalid RLP encoding"),
            NodeError::InvalidNodeStructure => write!(f, "invalid node structure"),
            NodeError::InvalidHashLength => write!(f, "invalid hash length"),
            NodeError::EmptyCompactPath => write!(f, "empty compact path"),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Node Creation Tests
    // =========================================================================

    #[test]
    fn test_new_leaf() {
        let node = Node::new_leaf(vec![1, 2, 3], vec![0x42]);
        match node {
            Node::Leaf { key_suffix, value } => {
                assert_eq!(key_suffix, vec![1, 2, 3]);
                assert_eq!(value, vec![0x42]);
            }
            _ => panic!("Expected Leaf node"),
        }
    }

    #[test]
    fn test_new_extension() {
        let hash = Hash::from([0x42; 32]);
        let node = Node::new_extension(vec![1, 2], hash);
        match node {
            Node::Extension { prefix, child_hash } => {
                assert_eq!(prefix, vec![1, 2]);
                assert_eq!(child_hash, hash);
            }
            _ => panic!("Expected Extension node"),
        }
    }

    #[test]
    fn test_new_branch() {
        let node = Node::new_branch();
        match node {
            Node::Branch { children, value } => {
                assert_eq!(*children, [None; 16]);
                assert_eq!(value, None);
            }
            _ => panic!("Expected Branch node"),
        }
    }

    #[test]
    fn test_new_branch_with_value() {
        let node = Node::new_branch_with_value(vec![0x42]);
        match node {
            Node::Branch { children, value } => {
                assert_eq!(*children, [None; 16]);
                assert_eq!(value, Some(vec![0x42]));
            }
            _ => panic!("Expected Branch node"),
        }
    }

    // =========================================================================
    // Nibble Conversion Tests
    // =========================================================================

    #[test]
    fn test_bytes_to_nibbles_empty() {
        assert_eq!(bytes_to_nibbles(&[]), Vec::<u8>::new());
    }

    #[test]
    fn test_bytes_to_nibbles_single() {
        assert_eq!(bytes_to_nibbles(&[0x12]), vec![1, 2]);
    }

    #[test]
    fn test_bytes_to_nibbles_multiple() {
        assert_eq!(bytes_to_nibbles(&[0x12, 0x34, 0x56]), vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_bytes_to_nibbles_zeros() {
        assert_eq!(bytes_to_nibbles(&[0x00, 0x00]), vec![0, 0, 0, 0]);
    }

    #[test]
    fn test_bytes_to_nibbles_max() {
        assert_eq!(bytes_to_nibbles(&[0xFF]), vec![15, 15]);
    }

    #[test]
    fn test_nibbles_to_bytes_empty() {
        assert_eq!(nibbles_to_bytes(&[]), Vec::<u8>::new());
    }

    #[test]
    fn test_nibbles_to_bytes_even() {
        assert_eq!(nibbles_to_bytes(&[1, 2, 3, 4]), vec![0x12, 0x34]);
    }

    #[test]
    fn test_nibbles_to_bytes_odd() {
        assert_eq!(nibbles_to_bytes(&[1, 2, 3]), vec![0x12, 0x30]);
    }

    #[test]
    fn test_nibbles_to_bytes_single() {
        assert_eq!(nibbles_to_bytes(&[5]), vec![0x50]);
    }

    #[test]
    fn test_nibbles_roundtrip() {
        let original = vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];
        let nibbles = bytes_to_nibbles(&original);
        let recovered = nibbles_to_bytes(&nibbles);
        assert_eq!(recovered, original);
    }

    // =========================================================================
    // Common Prefix Tests
    // =========================================================================

    #[test]
    fn test_common_prefix_empty() {
        assert_eq!(common_prefix_length(&[], &[]), 0);
        assert_eq!(common_prefix_length(&[1, 2, 3], &[]), 0);
        assert_eq!(common_prefix_length(&[], &[1, 2, 3]), 0);
    }

    #[test]
    fn test_common_prefix_identical() {
        assert_eq!(common_prefix_length(&[1, 2, 3], &[1, 2, 3]), 3);
    }

    #[test]
    fn test_common_prefix_partial() {
        assert_eq!(common_prefix_length(&[1, 2, 3, 4], &[1, 2, 5, 6]), 2);
    }

    #[test]
    fn test_common_prefix_none() {
        assert_eq!(common_prefix_length(&[1, 2, 3], &[4, 5, 6]), 0);
    }

    #[test]
    fn test_common_prefix_different_lengths() {
        assert_eq!(common_prefix_length(&[1, 2, 3], &[1, 2]), 2);
        assert_eq!(common_prefix_length(&[1, 2], &[1, 2, 3]), 2);
    }

    // =========================================================================
    // Compact Path Encoding Tests
    // =========================================================================

    #[test]
    fn test_encode_compact_leaf_even() {
        // Leaf with even-length path [0, 1, 2, 3]
        let encoded = encode_compact_path(&[0, 1, 2, 3], true);
        assert_eq!(encoded, vec![0x20, 0x01, 0x23]);
    }

    #[test]
    fn test_encode_compact_leaf_odd() {
        // Leaf with odd-length path [1, 2, 3]
        let encoded = encode_compact_path(&[1, 2, 3], true);
        assert_eq!(encoded, vec![0x31, 0x23]);
    }

    #[test]
    fn test_encode_compact_extension_even() {
        // Extension with even-length path [0, 1, 2, 3]
        let encoded = encode_compact_path(&[0, 1, 2, 3], false);
        assert_eq!(encoded, vec![0x00, 0x01, 0x23]);
    }

    #[test]
    fn test_encode_compact_extension_odd() {
        // Extension with odd-length path [1, 2, 3]
        let encoded = encode_compact_path(&[1, 2, 3], false);
        assert_eq!(encoded, vec![0x11, 0x23]);
    }

    #[test]
    fn test_encode_compact_empty() {
        let encoded = encode_compact_path(&[], true);
        assert_eq!(encoded, vec![0x20]);
    }

    #[test]
    fn test_encode_compact_single_nibble() {
        let encoded = encode_compact_path(&[5], false);
        assert_eq!(encoded, vec![0x15]);
    }

    #[test]
    fn test_decode_compact_leaf_even() {
        let (nibbles, is_leaf) = decode_compact_path(&[0x20, 0x01, 0x23]).unwrap();
        assert_eq!(nibbles, vec![0, 1, 2, 3]);
        assert!(is_leaf);
    }

    #[test]
    fn test_decode_compact_leaf_odd() {
        let (nibbles, is_leaf) = decode_compact_path(&[0x31, 0x23]).unwrap();
        assert_eq!(nibbles, vec![1, 2, 3]);
        assert!(is_leaf);
    }

    #[test]
    fn test_decode_compact_extension_even() {
        let (nibbles, is_leaf) = decode_compact_path(&[0x00, 0x01, 0x23]).unwrap();
        assert_eq!(nibbles, vec![0, 1, 2, 3]);
        assert!(!is_leaf);
    }

    #[test]
    fn test_decode_compact_extension_odd() {
        let (nibbles, is_leaf) = decode_compact_path(&[0x11, 0x23]).unwrap();
        assert_eq!(nibbles, vec![1, 2, 3]);
        assert!(!is_leaf);
    }

    #[test]
    fn test_decode_compact_empty() {
        assert!(decode_compact_path(&[]).is_err());
    }

    #[test]
    fn test_compact_path_roundtrip() {
        let test_cases = vec![
            (vec![0, 1, 2, 3], true),
            (vec![1, 2, 3], true),
            (vec![0, 1, 2, 3], false),
            (vec![1, 2, 3], false),
            (vec![], true),
            (vec![5], false),
        ];

        for (nibbles, is_leaf) in test_cases {
            let encoded = encode_compact_path(&nibbles, is_leaf);
            let (decoded_nibbles, decoded_is_leaf) = decode_compact_path(&encoded).unwrap();
            assert_eq!(decoded_nibbles, nibbles);
            assert_eq!(decoded_is_leaf, is_leaf);
        }
    }

    // =========================================================================
    // RLP Encoding Tests
    // =========================================================================

    #[test]
    fn test_encode_leaf() {
        let node = Node::new_leaf(vec![1, 2, 3], vec![0x42]);
        let encoded = node.encode_rlp();

        // Should be RLP list with 2 items
        assert!(!encoded.is_empty());
        assert!(encoded[0] >= 0xc0); // List marker
    }

    #[test]
    fn test_encode_extension() {
        let hash = Hash::from([0x42; 32]);
        let node = Node::new_extension(vec![1, 2], hash);
        let encoded = node.encode_rlp();

        // Should be RLP list with 2 items
        assert!(!encoded.is_empty());
        assert!(encoded[0] >= 0xc0); // List marker
    }

    #[test]
    fn test_encode_branch_empty() {
        let node = Node::new_branch();
        let encoded = node.encode_rlp();

        // Should be RLP list with 17 items
        assert!(!encoded.is_empty());
        assert!(encoded[0] >= 0xc0); // List marker (could be short or long form)
    }

    #[test]
    fn test_encode_branch_with_value() {
        let node = Node::new_branch_with_value(vec![0x42]);
        let encoded = node.encode_rlp();

        // Should be RLP list with 17 items
        assert!(!encoded.is_empty());
        assert!(encoded[0] >= 0xc0); // List marker (could be short or long form)
    }

    #[test]
    fn test_decode_leaf() {
        let original = Node::new_leaf(vec![1, 2, 3], vec![0x42]);
        let encoded = original.encode_rlp();
        let decoded = Node::decode_rlp(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_decode_extension() {
        let hash = Hash::from([0x42; 32]);
        let original = Node::new_extension(vec![1, 2], hash);
        let encoded = original.encode_rlp();
        let decoded = Node::decode_rlp(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_decode_branch() {
        let original = Node::new_branch();
        let encoded = original.encode_rlp();
        let decoded = Node::decode_rlp(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_decode_branch_with_value() {
        let original = Node::new_branch_with_value(vec![0x42]);
        let encoded = original.encode_rlp();
        let decoded = Node::decode_rlp(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_rlp_roundtrip_leaf() {
        let test_cases = vec![
            Node::new_leaf(vec![], vec![]),
            Node::new_leaf(vec![1, 2, 3], vec![0x42]),
            Node::new_leaf(vec![0, 0, 0], vec![0x00]),
            Node::new_leaf(vec![15, 15, 15], vec![0xFF, 0xFF]),
        ];

        for node in test_cases {
            let encoded = node.encode_rlp();
            let decoded = Node::decode_rlp(&encoded).unwrap();
            assert_eq!(decoded, node);
        }
    }

    #[test]
    fn test_rlp_roundtrip_extension() {
        let test_cases = vec![
            Node::new_extension(vec![1], Hash::ZERO),
            Node::new_extension(vec![1, 2, 3], Hash::from([0x42; 32])),
            Node::new_extension(vec![0, 0, 0], Hash::from([0xFF; 32])),
        ];

        for node in test_cases {
            let encoded = node.encode_rlp();
            let decoded = Node::decode_rlp(&encoded).unwrap();
            assert_eq!(decoded, node);
        }
    }

    #[test]
    fn test_rlp_roundtrip_branch() {
        let mut branch = Node::new_branch();
        if let Node::Branch { ref mut children, .. } = branch {
            children[0] = Some(Hash::from([0x42; 32]));
            children[15] = Some(Hash::from([0xFF; 32]));
        }

        let encoded = branch.encode_rlp();
        let decoded = Node::decode_rlp(&encoded).unwrap();
        assert_eq!(decoded, branch);
    }

    // =========================================================================
    // Node Hashing Tests
    // =========================================================================

    #[test]
    fn test_compute_hash_leaf() {
        let node = Node::new_leaf(vec![1, 2, 3], vec![0x42]);
        let hash = node.compute_hash();
        assert_eq!(hash.as_bytes().len(), 32);
    }

    #[test]
    fn test_compute_hash_extension() {
        let child_hash = Hash::from([0x42; 32]);
        let node = Node::new_extension(vec![1, 2], child_hash);
        let hash = node.compute_hash();
        assert_eq!(hash.as_bytes().len(), 32);
    }

    #[test]
    fn test_compute_hash_branch() {
        let node = Node::new_branch();
        let hash = node.compute_hash();
        assert_eq!(hash.as_bytes().len(), 32);
    }

    #[test]
    fn test_compute_hash_deterministic() {
        let node = Node::new_leaf(vec![1, 2, 3], vec![0x42]);
        let hash1 = node.compute_hash();
        let hash2 = node.compute_hash();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_different_nodes() {
        let node1 = Node::new_leaf(vec![1, 2, 3], vec![0x42]);
        let node2 = Node::new_leaf(vec![1, 2, 3], vec![0x43]);
        let hash1 = node1.compute_hash();
        let hash2 = node2.compute_hash();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_inline_node() {
        // Create a very small node that should be inline (<32 bytes)
        let node = Node::new_leaf(vec![1], vec![0x42]);
        let encoded = node.encode_rlp();

        // Verify it's small enough to be inline
        if encoded.len() < 32 {
            let hash = node.compute_hash();
            // The hash should start with the encoded data
            assert_eq!(&hash.as_bytes()[..encoded.len()], &encoded[..]);
        }
    }

    // =========================================================================
    // Error Handling Tests
    // =========================================================================

    #[test]
    fn test_decode_invalid_rlp() {
        let invalid = vec![0xFF, 0xFF, 0xFF];
        assert!(Node::decode_rlp(&invalid).is_err());
    }

    #[test]
    fn test_decode_wrong_item_count() {
        // Create a list with 3 items (invalid for MPT nodes)
        let items = vec![vec![0x01], vec![0x02], vec![0x03]];
        let encoded = rlp::encode_list(&items);
        assert!(Node::decode_rlp(&encoded).is_err());
    }

    #[test]
    fn test_decode_invalid_hash_length() {
        // Create an extension with wrong hash length
        let compact_path = encode_compact_path(&[1, 2], false);
        let items = vec![
            rlp::encode_bytes(&compact_path),
            rlp::encode_bytes(&[0x42; 16]), // Wrong length (not 32)
        ];
        let encoded = rlp::encode_list(&items);
        assert!(Node::decode_rlp(&encoded).is_err());
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_leaf_empty_key_and_value() {
        let node = Node::new_leaf(vec![], vec![]);
        let encoded = node.encode_rlp();
        let decoded = Node::decode_rlp(&encoded).unwrap();
        assert_eq!(decoded, node);
    }

    #[test]
    fn test_branch_all_children() {
        let mut branch = Node::new_branch();
        if let Node::Branch { ref mut children, .. } = branch {
            for (i, child) in children.iter_mut().enumerate() {
                let mut hash_bytes = [0u8; 32];
                hash_bytes[0] = i as u8;
                *child = Some(Hash::from(hash_bytes));
            }
        }

        let encoded = branch.encode_rlp();
        let decoded = Node::decode_rlp(&encoded).unwrap();
        assert_eq!(decoded, branch);
    }

    #[test]
    fn test_branch_with_value_and_children() {
        let mut branch = Node::new_branch_with_value(vec![0x42]);
        if let Node::Branch { ref mut children, .. } = branch {
            children[0] = Some(Hash::from([0x01; 32]));
            children[5] = Some(Hash::from([0x05; 32]));
            children[15] = Some(Hash::from([0x0F; 32]));
        }

        let encoded = branch.encode_rlp();
        let decoded = Node::decode_rlp(&encoded).unwrap();
        assert_eq!(decoded, branch);
    }

    #[test]
    fn test_nibbles_all_zeros() {
        let nibbles = vec![0, 0, 0, 0];
        let bytes = nibbles_to_bytes(&nibbles);
        assert_eq!(bytes, vec![0x00, 0x00]);
        let recovered = bytes_to_nibbles(&bytes);
        assert_eq!(recovered, nibbles);
    }

    #[test]
    fn test_nibbles_all_max() {
        let nibbles = vec![15, 15, 15, 15];
        let bytes = nibbles_to_bytes(&nibbles);
        assert_eq!(bytes, vec![0xFF, 0xFF]);
        let recovered = bytes_to_nibbles(&bytes);
        assert_eq!(recovered, nibbles);
    }

    #[test]
    fn test_compact_path_max_nibbles() {
        let nibbles = vec![15; 100];
        let encoded = encode_compact_path(&nibbles, true);
        let (decoded, is_leaf) = decode_compact_path(&encoded).unwrap();
        assert_eq!(decoded, nibbles);
        assert!(is_leaf);
    }

    // =========================================================================
    // Clone and Equality Tests
    // =========================================================================

    #[test]
    fn test_node_clone() {
        let node = Node::new_leaf(vec![1, 2, 3], vec![0x42]);
        let cloned = node.clone();
        assert_eq!(node, cloned);
    }

    #[test]
    fn test_node_equality() {
        let node1 = Node::new_leaf(vec![1, 2, 3], vec![0x42]);
        let node2 = Node::new_leaf(vec![1, 2, 3], vec![0x42]);
        let node3 = Node::new_leaf(vec![1, 2, 3], vec![0x43]);

        assert_eq!(node1, node2);
        assert_ne!(node1, node3);
    }
}
