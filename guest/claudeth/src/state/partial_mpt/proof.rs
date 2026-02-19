//! Merkle proof generation and verification
//!
//! This module implements Merkle proofs for the Patricia Trie, allowing verification
//! of key-value inclusion or exclusion without requiring the full trie.
//!
//! A Merkle proof consists of a sequence of RLP-encoded nodes from the root to the
//! target key. The verifier can reconstruct the root hash from the proof and compare
//! it with the expected root.

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

use super::node::{Node, bytes_to_nibbles};
use super::trie::EMPTY_TRIE_ROOT;
use super::trie::Trie;
use crate::types::Hash;

// =============================================================================
// Proof Types
// =============================================================================

/// A Merkle proof for a key-value pair in the trie
///
/// The proof contains a list of RLP-encoded nodes from the root to the target key.
/// For inclusion proofs, the last node should contain the value.
/// For exclusion proofs, the path should end at a non-matching node or empty child.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proof {
    /// RLP-encoded nodes from root to target
    pub nodes: Vec<Vec<u8>>,
}

impl Proof {
    /// Creates a new empty proof
    pub fn new() -> Self {
        Proof { nodes: Vec::new() }
    }

    /// Creates a proof from a list of RLP-encoded nodes
    pub fn from_nodes(nodes: Vec<Vec<u8>>) -> Self {
        Proof { nodes }
    }

    /// Returns true if the proof is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the number of nodes in the proof
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

impl Default for Proof {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Proof Errors
// =============================================================================

/// Errors that can occur during proof generation or verification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofError {
    /// Key not found in trie (for inclusion proofs)
    KeyNotFound,
    /// Node not found in storage
    NodeNotFound,
    /// Invalid proof structure
    InvalidProof,
    /// Proof verification failed
    VerificationFailed,
}

impl core::fmt::Display for ProofError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProofError::KeyNotFound => write!(f, "key not found in trie"),
            ProofError::NodeNotFound => write!(f, "node not found in storage"),
            ProofError::InvalidProof => write!(f, "invalid proof structure"),
            ProofError::VerificationFailed => write!(f, "proof verification failed"),
        }
    }
}

// =============================================================================
// Proof Generation
// =============================================================================

impl Trie {
    /// Generates a Merkle proof for a key
    ///
    /// Returns a proof that can be used to verify the key's value (inclusion)
    /// or absence (exclusion) in the trie.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::state::partial_mpt::{Trie, proof::Proof};
    ///
    /// let mut trie = Trie::new();
    /// trie.insert(b"key", b"value".to_vec());
    ///
    /// let proof = trie.generate_proof(b"key").unwrap();
    /// assert!(!proof.is_empty());
    /// ```
    pub fn generate_proof(&self, key: &[u8]) -> Result<Proof, ProofError> {
        let nibbles = bytes_to_nibbles(key);
        let mut proof_nodes = Vec::new();

        // If trie is empty, return empty proof for exclusion
        let Some(root_hash) = self.root() else {
            return Ok(Proof::new());
        };

        // Traverse the trie and collect all nodes on the path
        self.collect_proof_nodes(root_hash, &nibbles, &mut proof_nodes)?;

        Ok(Proof::from_nodes(proof_nodes))
    }

    /// Recursively collects proof nodes from root to target key
    fn collect_proof_nodes(
        &self,
        node_hash: Hash,
        path: &[u8],
        proof_nodes: &mut Vec<Vec<u8>>,
    ) -> Result<(), ProofError> {
        // Get the node from storage
        let node = self.get_node(&node_hash).ok_or(ProofError::NodeNotFound)?;

        // Add this node's RLP encoding to the proof
        let rlp_encoded = node.encode_rlp();
        proof_nodes.push(rlp_encoded);

        // Continue traversal based on node type
        match node {
            Node::Leaf { .. } => {
                // Reached a leaf - proof is complete (inclusion or exclusion)
                // For inclusion: key_suffix should match path
                // For exclusion: key_suffix doesn't match path
                // In both cases, we include this leaf in the proof
                Ok(())
            }
            Node::Extension { prefix, child_hash } => {
                // Check if path matches prefix
                if path.len() >= prefix.len() && &path[..prefix.len()] == prefix.as_slice() {
                    // Path matches - continue to child
                    self.collect_proof_nodes(*child_hash, &path[prefix.len()..], proof_nodes)
                } else {
                    // Path doesn't match - this is an exclusion proof
                    Ok(())
                }
            }
            Node::Branch { children, .. } => {
                if path.is_empty() {
                    // Reached the target (branch with value)
                    Ok(())
                } else {
                    let nibble = path[0] as usize;
                    if let Some(child_hash) = children[nibble] {
                        // Continue to child
                        self.collect_proof_nodes(child_hash, &path[1..], proof_nodes)
                    } else {
                        // No child at this nibble - exclusion proof
                        Ok(())
                    }
                }
            }
        }
    }
}

// =============================================================================
// Proof Verification
// =============================================================================

/// Verifies a Merkle proof for a key-value pair
///
/// Returns true if the proof is valid and matches the expected root hash.
///
/// # Arguments
///
/// * `root` - Expected root hash of the trie
/// * `key` - The key being proved
/// * `value` - Expected value (Some for inclusion, None for exclusion)
/// * `proof` - The Merkle proof to verify
///
/// # Examples
///
/// ```
/// use claudeth::state::partial_mpt::{Trie, proof::verify_proof};
///
/// let mut trie = Trie::new();
/// trie.insert(b"key", b"value".to_vec());
/// let root = trie.compute_root();
///
/// let proof = trie.generate_proof(b"key").unwrap();
/// assert!(verify_proof(root, b"key", Some(b"value"), &proof));
/// ```
pub fn verify_proof(root: Hash, key: &[u8], value: Option<&[u8]>, proof: &Proof) -> bool {
    let nibbles = bytes_to_nibbles(key);

    // Empty trie case
    if root == EMPTY_TRIE_ROOT {
        // Empty trie should have empty proof and no value
        return proof.is_empty() && value.is_none();
    }

    // Empty proof but non-empty root is invalid
    if proof.is_empty() {
        return false;
    }

    // Verify the proof by reconstructing the root hash
    match verify_proof_internal(&proof.nodes, &nibbles, value) {
        Ok(computed_root) => computed_root == root,
        Err(_) => false,
    }
}

/// Internal proof verification that reconstructs the root hash
fn verify_proof_internal(
    proof_nodes: &[Vec<u8>],
    path: &[u8],
    expected_value: Option<&[u8]>,
) -> Result<Hash, ProofError> {
    if proof_nodes.is_empty() {
        return Err(ProofError::InvalidProof);
    }

    // Start from the first node (root)
    verify_node_chain(proof_nodes, 0, path, expected_value)
}

/// Verifies a chain of nodes and returns the hash of the current node
fn verify_node_chain(
    proof_nodes: &[Vec<u8>],
    index: usize,
    path: &[u8],
    expected_value: Option<&[u8]>,
) -> Result<Hash, ProofError> {
    if index >= proof_nodes.len() {
        return Err(ProofError::InvalidProof);
    }

    // Decode the current node
    let node_rlp = &proof_nodes[index];
    let node = Node::decode_rlp(node_rlp).map_err(|_| ProofError::InvalidProof)?;

    // Compute hash of current node
    let node_hash = node.compute_hash();

    match node {
        Node::Leaf { key_suffix, value } => {
            // This should be the last node in the proof
            if index != proof_nodes.len() - 1 {
                return Err(ProofError::InvalidProof);
            }

            // Check if this is an inclusion or exclusion proof
            if key_suffix == path {
                // Inclusion proof - value should match
                if expected_value == Some(&value[..]) {
                    Ok(node_hash)
                } else {
                    Err(ProofError::VerificationFailed)
                }
            } else {
                // Exclusion proof - key doesn't match
                if expected_value.is_none() {
                    Ok(node_hash)
                } else {
                    Err(ProofError::VerificationFailed)
                }
            }
        }
        Node::Extension { prefix, child_hash } => {
            // Check if path matches prefix
            if path.len() >= prefix.len() && &path[..prefix.len()] == prefix.as_slice() {
                // Path matches - verify child
                let child_node_hash = verify_node_chain(
                    proof_nodes,
                    index + 1,
                    &path[prefix.len()..],
                    expected_value,
                )?;

                // Verify that the child hash matches
                if child_node_hash == child_hash {
                    Ok(node_hash)
                } else {
                    Err(ProofError::VerificationFailed)
                }
            } else {
                // Path doesn't match - this is an exclusion proof
                if index == proof_nodes.len() - 1 && expected_value.is_none() {
                    Ok(node_hash)
                } else {
                    Err(ProofError::VerificationFailed)
                }
            }
        }
        Node::Branch { children, value } => {
            if path.is_empty() {
                // Reached the target - check value
                if index != proof_nodes.len() - 1 {
                    return Err(ProofError::InvalidProof);
                }

                match (value, expected_value) {
                    (Some(v), Some(expected)) if v == expected => Ok(node_hash),
                    (None, None) => Ok(node_hash),
                    _ => Err(ProofError::VerificationFailed),
                }
            } else {
                let nibble = path[0] as usize;
                if let Some(child_hash) = children[nibble] {
                    // Verify child exists in proof
                    let child_node_hash =
                        verify_node_chain(proof_nodes, index + 1, &path[1..], expected_value)?;

                    // Verify that the child hash matches
                    if child_node_hash == child_hash {
                        Ok(node_hash)
                    } else {
                        Err(ProofError::VerificationFailed)
                    }
                } else {
                    // No child at this nibble - exclusion proof
                    if index == proof_nodes.len() - 1 && expected_value.is_none() {
                        Ok(node_hash)
                    } else {
                        Err(ProofError::VerificationFailed)
                    }
                }
            }
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
    // Proof Construction Tests
    // =========================================================================

    #[test]
    fn test_proof_new() {
        let proof = Proof::new();
        assert!(proof.is_empty());
        assert_eq!(proof.len(), 0);
    }

    #[test]
    fn test_proof_from_nodes() {
        let nodes = vec![vec![0x01, 0x02], vec![0x03, 0x04]];
        let proof = Proof::from_nodes(nodes.clone());
        assert_eq!(proof.nodes, nodes);
        assert_eq!(proof.len(), 2);
        assert!(!proof.is_empty());
    }

    #[test]
    fn test_proof_default() {
        let proof = Proof::default();
        assert!(proof.is_empty());
    }

    // =========================================================================
    // Empty Trie Tests
    // =========================================================================

    #[test]
    fn test_generate_proof_empty_trie() {
        let trie = Trie::new();
        let proof = trie.generate_proof(b"key").unwrap();
        assert!(proof.is_empty());
    }

    #[test]
    fn test_verify_proof_empty_trie() {
        let trie = Trie::new();
        let root = trie.compute_root();
        let proof = trie.generate_proof(b"key").unwrap();

        // Empty trie, no value - should pass
        assert!(verify_proof(root, b"key", None, &proof));
    }

    #[test]
    fn test_verify_proof_empty_trie_with_value_fails() {
        let trie = Trie::new();
        let root = trie.compute_root();
        let proof = trie.generate_proof(b"key").unwrap();

        // Empty trie but expecting value - should fail
        assert!(!verify_proof(root, b"key", Some(b"value"), &proof));
    }

    // =========================================================================
    // Single Key Tests
    // =========================================================================

    #[test]
    fn test_generate_proof_single_key() {
        let mut trie = Trie::new();
        trie.insert(b"key", b"value".to_vec());

        let proof = trie.generate_proof(b"key").unwrap();
        assert!(!proof.is_empty());
        assert_eq!(proof.len(), 1); // Just the leaf node
    }

    #[test]
    fn test_verify_proof_single_key_inclusion() {
        let mut trie = Trie::new();
        trie.insert(b"key", b"value".to_vec());
        let root = trie.compute_root();

        let proof = trie.generate_proof(b"key").unwrap();
        assert!(verify_proof(root, b"key", Some(b"value"), &proof));
    }

    #[test]
    fn test_verify_proof_single_key_wrong_value() {
        let mut trie = Trie::new();
        trie.insert(b"key", b"value".to_vec());
        let root = trie.compute_root();

        let proof = trie.generate_proof(b"key").unwrap();
        assert!(!verify_proof(root, b"key", Some(b"wrong"), &proof));
    }

    #[test]
    fn test_verify_proof_single_key_exclusion() {
        let mut trie = Trie::new();
        trie.insert(b"key", b"value".to_vec());
        let root = trie.compute_root();

        let proof = trie.generate_proof(b"other").unwrap();
        assert!(verify_proof(root, b"other", None, &proof));
    }

    // =========================================================================
    // Multiple Keys Tests
    // =========================================================================

    #[test]
    fn test_generate_proof_multiple_keys() {
        let mut trie = Trie::new();
        trie.insert(b"key1", b"value1".to_vec());
        trie.insert(b"key2", b"value2".to_vec());
        trie.insert(b"key3", b"value3".to_vec());

        let proof1 = trie.generate_proof(b"key1").unwrap();
        let proof2 = trie.generate_proof(b"key2").unwrap();
        let proof3 = trie.generate_proof(b"key3").unwrap();

        assert!(!proof1.is_empty());
        assert!(!proof2.is_empty());
        assert!(!proof3.is_empty());
    }

    #[test]
    fn test_verify_proof_multiple_keys() {
        let mut trie = Trie::new();
        trie.insert(b"key1", b"value1".to_vec());
        trie.insert(b"key2", b"value2".to_vec());
        trie.insert(b"key3", b"value3".to_vec());
        let root = trie.compute_root();

        // Test inclusion proofs for all keys
        let proof1 = trie.generate_proof(b"key1").unwrap();
        assert!(verify_proof(root, b"key1", Some(b"value1"), &proof1));

        let proof2 = trie.generate_proof(b"key2").unwrap();
        assert!(verify_proof(root, b"key2", Some(b"value2"), &proof2));

        let proof3 = trie.generate_proof(b"key3").unwrap();
        assert!(verify_proof(root, b"key3", Some(b"value3"), &proof3));
    }

    #[test]
    fn test_verify_proof_exclusion_with_multiple_keys() {
        let mut trie = Trie::new();
        trie.insert(b"key1", b"value1".to_vec());
        trie.insert(b"key2", b"value2".to_vec());
        let root = trie.compute_root();

        // Generate and verify exclusion proof for non-existent key
        let proof = trie.generate_proof(b"key3").unwrap();
        assert!(verify_proof(root, b"key3", None, &proof));
    }

    // =========================================================================
    // Prefix Tests
    // =========================================================================

    #[test]
    fn test_proof_with_common_prefix() {
        let mut trie = Trie::new();
        trie.insert(b"test", b"value1".to_vec());
        trie.insert(b"testing", b"value2".to_vec());
        let root = trie.compute_root();

        let proof1 = trie.generate_proof(b"test").unwrap();
        assert!(verify_proof(root, b"test", Some(b"value1"), &proof1));

        let proof2 = trie.generate_proof(b"testing").unwrap();
        assert!(verify_proof(root, b"testing", Some(b"value2"), &proof2));
    }

    #[test]
    fn test_proof_exclusion_prefix_mismatch() {
        let mut trie = Trie::new();
        trie.insert(b"testing", b"value".to_vec());
        let root = trie.compute_root();

        // "test" is a prefix but not present
        let proof = trie.generate_proof(b"test").unwrap();
        assert!(verify_proof(root, b"test", None, &proof));
    }

    #[test]
    fn test_proof_exclusion_extension_mismatch() {
        let mut trie = Trie::new();
        trie.insert(b"test", b"value".to_vec());
        let root = trie.compute_root();

        // "testing" extends existing key
        let proof = trie.generate_proof(b"testing").unwrap();
        assert!(verify_proof(root, b"testing", None, &proof));
    }

    // =========================================================================
    // Branch Node Tests
    // =========================================================================

    #[test]
    fn test_proof_with_branch_nodes() {
        let mut trie = Trie::new();
        trie.insert(&[0x10, 0x00], vec![1]);
        trie.insert(&[0x20, 0x00], vec![2]);
        trie.insert(&[0x30, 0x00], vec![3]);
        let root = trie.compute_root();

        let proof1 = trie.generate_proof(&[0x10, 0x00]).unwrap();
        assert!(verify_proof(root, &[0x10, 0x00], Some(&[1]), &proof1));

        let proof2 = trie.generate_proof(&[0x20, 0x00]).unwrap();
        assert!(verify_proof(root, &[0x20, 0x00], Some(&[2]), &proof2));

        let proof3 = trie.generate_proof(&[0x30, 0x00]).unwrap();
        assert!(verify_proof(root, &[0x30, 0x00], Some(&[3]), &proof3));
    }

    #[test]
    fn test_proof_branch_with_value() {
        let mut trie = Trie::new();
        trie.insert(b"test", b"value1".to_vec());
        trie.insert(b"testing", b"value2".to_vec());
        let root = trie.compute_root();

        // "test" should be stored as a branch value
        let proof = trie.generate_proof(b"test").unwrap();
        assert!(verify_proof(root, b"test", Some(b"value1"), &proof));
    }

    #[test]
    fn test_proof_branch_empty_child_exclusion() {
        let mut trie = Trie::new();
        trie.insert(&[0x10, 0x00], vec![1]);
        trie.insert(&[0x30, 0x00], vec![3]);
        let root = trie.compute_root();

        // Key with nibble 0x20 doesn't exist
        let proof = trie.generate_proof(&[0x20, 0x00]).unwrap();
        assert!(verify_proof(root, &[0x20, 0x00], None, &proof));
    }

    // =========================================================================
    // Deep Path Tests
    // =========================================================================

    #[test]
    fn test_proof_deep_path() {
        let mut trie = Trie::new();
        let long_key = vec![0x42; 50];
        trie.insert(&long_key, b"deep_value".to_vec());
        let root = trie.compute_root();

        let proof = trie.generate_proof(&long_key).unwrap();
        assert!(verify_proof(root, &long_key, Some(b"deep_value"), &proof));
    }

    #[test]
    fn test_proof_multiple_levels() {
        let mut trie = Trie::new();
        // Create a deeper tree structure
        trie.insert(&[0x12, 0x34, 0x56], vec![1]);
        trie.insert(&[0x12, 0x34, 0x78], vec![2]);
        trie.insert(&[0x12, 0x44], vec![3]);
        let root = trie.compute_root();

        let proof1 = trie.generate_proof(&[0x12, 0x34, 0x56]).unwrap();
        assert!(verify_proof(root, &[0x12, 0x34, 0x56], Some(&[1]), &proof1));
        assert!(proof1.len() >= 2); // Should have multiple nodes
    }

    // =========================================================================
    // Invalid Proof Tests
    // =========================================================================

    #[test]
    fn test_verify_tampered_proof_fails() {
        let mut trie = Trie::new();
        trie.insert(b"key", b"value".to_vec());
        let root = trie.compute_root();

        let mut proof = trie.generate_proof(b"key").unwrap();

        // Tamper with the proof
        if !proof.nodes.is_empty() {
            proof.nodes[0][0] ^= 0xFF;
        }

        assert!(!verify_proof(root, b"key", Some(b"value"), &proof));
    }

    #[test]
    fn test_verify_wrong_root_fails() {
        let mut trie = Trie::new();
        trie.insert(b"key", b"value".to_vec());

        let proof = trie.generate_proof(b"key").unwrap();
        let wrong_root = Hash::from([0x42; 32]);

        assert!(!verify_proof(wrong_root, b"key", Some(b"value"), &proof));
    }

    #[test]
    fn test_verify_truncated_proof_fails() {
        let mut trie = Trie::new();
        trie.insert(b"key1", b"value1".to_vec());
        trie.insert(b"key2", b"value2".to_vec());
        let root = trie.compute_root();

        let mut proof = trie.generate_proof(b"key1").unwrap();

        // Remove last node if proof has multiple nodes
        if proof.len() > 1 {
            proof.nodes.pop();
            assert!(!verify_proof(root, b"key1", Some(b"value1"), &proof));
        }
    }

    #[test]
    fn test_verify_empty_proof_non_empty_root_fails() {
        let mut trie = Trie::new();
        trie.insert(b"key", b"value".to_vec());
        let root = trie.compute_root();

        let empty_proof = Proof::new();
        assert!(!verify_proof(root, b"key", Some(b"value"), &empty_proof));
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_proof_empty_key() {
        let mut trie = Trie::new();
        trie.insert(b"", b"empty_key_value".to_vec());
        let root = trie.compute_root();

        let proof = trie.generate_proof(b"").unwrap();
        assert!(verify_proof(root, b"", Some(b"empty_key_value"), &proof));
    }

    #[test]
    fn test_proof_empty_value() {
        let mut trie = Trie::new();
        trie.insert(b"key", vec![]);
        let root = trie.compute_root();

        let proof = trie.generate_proof(b"key").unwrap();
        assert!(verify_proof(root, b"key", Some(&[]), &proof));
    }

    #[test]
    fn test_proof_binary_data() {
        let mut trie = Trie::new();
        let key = vec![0x00, 0xFF, 0x00, 0xFF];
        let value = vec![0xDE, 0xAD, 0xBE, 0xEF];
        trie.insert(&key, value.clone());
        let root = trie.compute_root();

        let proof = trie.generate_proof(&key).unwrap();
        assert!(verify_proof(root, &key, Some(&value), &proof));
    }

    #[test]
    fn test_proof_after_delete() {
        let mut trie = Trie::new();
        trie.insert(b"key1", b"value1".to_vec());
        trie.insert(b"key2", b"value2".to_vec());
        trie.delete(b"key1");
        let root = trie.compute_root();

        // key1 should have exclusion proof
        let proof1 = trie.generate_proof(b"key1").unwrap();
        assert!(verify_proof(root, b"key1", None, &proof1));

        // key2 should still have inclusion proof
        let proof2 = trie.generate_proof(b"key2").unwrap();
        assert!(verify_proof(root, b"key2", Some(b"value2"), &proof2));
    }

    #[test]
    fn test_proof_consistency() {
        let mut trie = Trie::new();
        trie.insert(b"key", b"value".to_vec());

        // Generate same proof multiple times
        let proof1 = trie.generate_proof(b"key").unwrap();
        let proof2 = trie.generate_proof(b"key").unwrap();

        assert_eq!(proof1, proof2);
    }

    #[test]
    fn test_proof_different_keys_different_proofs() {
        let mut trie = Trie::new();
        trie.insert(b"key1", b"value1".to_vec());
        trie.insert(b"key2", b"value2".to_vec());

        let proof1 = trie.generate_proof(b"key1").unwrap();
        let proof2 = trie.generate_proof(b"key2").unwrap();

        // Proofs for different keys should be different
        assert_ne!(proof1, proof2);
    }

    // =========================================================================
    // Large Trie Tests
    // =========================================================================

    #[test]
    fn test_proof_large_trie() {
        let mut trie = Trie::new();

        // Insert 100 keys
        for i in 0..100 {
            let key = format!("key{i}");
            let value = format!("value{i}");
            trie.insert(key.as_bytes(), value.as_bytes().to_vec());
        }

        let root = trie.compute_root();

        // Verify proofs for several keys
        for i in [0, 25, 50, 75, 99] {
            let key = format!("key{i}");
            let value = format!("value{i}");
            let proof = trie.generate_proof(key.as_bytes()).unwrap();
            assert!(verify_proof(
                root,
                key.as_bytes(),
                Some(value.as_bytes()),
                &proof
            ));
        }
    }

    #[test]
    fn test_proof_all_single_byte_keys() {
        let mut trie = Trie::new();

        for i in 0..16u8 {
            trie.insert(&[i], vec![i]);
        }

        let root = trie.compute_root();

        // Verify all keys
        for i in 0..16u8 {
            let proof = trie.generate_proof(&[i]).unwrap();
            assert!(verify_proof(root, &[i], Some(&[i]), &proof));
        }
    }
}
