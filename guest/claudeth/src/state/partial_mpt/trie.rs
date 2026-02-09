//! Merkle Patricia Trie implementation
//!
//! This module implements a full Merkle Patricia Trie with insert, get, and delete operations.
//! The trie stores key-value pairs and maintains the cryptographic integrity through hashing.

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::collections::HashMap;
#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::{collections::BTreeMap as HashMap, vec, vec::Vec};

use super::node::{Node, bytes_to_nibbles, common_prefix_length};
use crate::types::Hash;

/// Ethereum empty trie root hash: keccak256(rlp([]))
/// This is 0x56e81f171bcc55a6ff8345e692c0f86e5b96e01b996cadc001622fb5e363b421
pub const EMPTY_TRIE_ROOT: Hash = Hash::new([
    0x56, 0xe8, 0x1f, 0x17, 0x1b, 0xcc, 0x55, 0xa6, 0xff, 0x83, 0x45, 0xe6, 0x92, 0xc0, 0xf8, 0x6e,
    0x5b, 0x96, 0xe0, 0x1b, 0x99, 0x6c, 0xad, 0xc0, 0x01, 0x62, 0x2f, 0xb5, 0xe3, 0x63, 0xb4, 0x21,
]);

/// A Merkle Patricia Trie for storing key-value pairs
#[derive(Clone, Debug)]
pub struct Trie {
    /// Root hash of the trie
    root: Option<Hash>,
    /// In-memory node storage: hash -> node
    nodes: HashMap<Hash, Node>,
}

impl Trie {
    /// Creates a new empty trie
    pub fn new() -> Self {
        Trie {
            root: None,
            nodes: HashMap::new(),
        }
    }

    /// Returns the root hash of the trie
    pub fn root(&self) -> Option<Hash> {
        self.root
    }

    /// Computes the root hash of the trie
    ///
    /// For an empty trie, returns EMPTY_TRIE_ROOT (keccak256 of RLP empty bytes).
    /// For a non-empty trie, returns the hash of the root node.
    ///
    /// This method verifies the trie structure by computing the hash from the root,
    /// ensuring cryptographic integrity.
    pub fn compute_root(&self) -> Hash {
        if let Some(root_hash) = self.root {
            root_hash
        } else {
            EMPTY_TRIE_ROOT
        }
    }

    /// Gets a node from storage by its hash (used internally for proof generation)
    pub(crate) fn get_node(&self, hash: &Hash) -> Option<&Node> {
        self.nodes.get(hash)
    }

    /// Inserts a key-value pair into the trie
    ///
    /// If the key already exists, its value is updated and the old value is returned.
    pub fn insert(&mut self, key: &[u8], value: Vec<u8>) -> Option<Vec<u8>> {
        let nibbles = bytes_to_nibbles(key);
        let (new_root, old_value) = if let Some(root_hash) = self.root {
            self.insert_at(root_hash, &nibbles, value)
        } else {
            // Empty trie - create a leaf
            let node = Node::new_leaf(nibbles, value);
            let hash = node.compute_hash();
            self.nodes.insert(hash, node);
            (hash, None)
        };
        self.root = Some(new_root);
        old_value
    }

    /// Gets a value from the trie by key
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let nibbles = bytes_to_nibbles(key);
        self.root
            .and_then(|root_hash| self.get_at(root_hash, &nibbles))
    }

    /// Deletes a key from the trie
    ///
    /// Returns the deleted value if the key existed.
    pub fn delete(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        let nibbles = bytes_to_nibbles(key);
        if let Some(root_hash) = self.root
            && let Some((new_root, deleted_value)) = self.delete_at(root_hash, &nibbles)
        {
            self.root = new_root;
            return Some(deleted_value);
        }
        None
    }

    /// Recursive insert helper
    ///
    /// Returns (new_node_hash, old_value)
    fn insert_at(
        &mut self,
        node_hash: Hash,
        path: &[u8],
        value: Vec<u8>,
    ) -> (Hash, Option<Vec<u8>>) {
        let node = self
            .nodes
            .get(&node_hash)
            .cloned()
            .expect("Node not found in storage");

        match node {
            Node::Leaf {
                key_suffix,
                value: old_value,
            } => {
                if key_suffix == path {
                    // Exact match - update value
                    let new_node = Node::new_leaf(key_suffix, value);
                    let new_hash = new_node.compute_hash();
                    self.nodes.insert(new_hash, new_node);
                    (new_hash, Some(old_value))
                } else {
                    // Split the leaf
                    let common_len = common_prefix_length(&key_suffix, path);

                    if common_len == path.len() && common_len < key_suffix.len() {
                        // New key is a prefix of the existing key
                        // Create branch with value
                        let mut branch = Node::new_branch_with_value(value);
                        let next_nibble = key_suffix[common_len];

                        // Create leaf for remainder
                        let remainder_leaf =
                            Node::new_leaf(key_suffix[common_len + 1..].to_vec(), old_value);
                        let remainder_hash = remainder_leaf.compute_hash();
                        self.nodes.insert(remainder_hash, remainder_leaf);

                        if let Node::Branch {
                            ref mut children, ..
                        } = branch
                        {
                            children[next_nibble as usize] = Some(remainder_hash);
                        }

                        let branch_hash = branch.compute_hash();
                        self.nodes.insert(branch_hash, branch);

                        // Wrap in extension if needed
                        if common_len > 0 {
                            let ext = Node::new_extension(path[..common_len].to_vec(), branch_hash);
                            let ext_hash = ext.compute_hash();
                            self.nodes.insert(ext_hash, ext);
                            (ext_hash, None)
                        } else {
                            (branch_hash, None)
                        }
                    } else if common_len == key_suffix.len() && common_len < path.len() {
                        // Existing key is a prefix of new key
                        // Create branch with old value
                        let mut branch = Node::new_branch_with_value(old_value);
                        let next_nibble = path[common_len];

                        // Create leaf for remainder
                        let remainder_leaf = Node::new_leaf(path[common_len + 1..].to_vec(), value);
                        let remainder_hash = remainder_leaf.compute_hash();
                        self.nodes.insert(remainder_hash, remainder_leaf);

                        if let Node::Branch {
                            ref mut children, ..
                        } = branch
                        {
                            children[next_nibble as usize] = Some(remainder_hash);
                        }

                        let branch_hash = branch.compute_hash();
                        self.nodes.insert(branch_hash, branch);

                        // Wrap in extension if needed
                        if common_len > 0 {
                            let ext = Node::new_extension(path[..common_len].to_vec(), branch_hash);
                            let ext_hash = ext.compute_hash();
                            self.nodes.insert(ext_hash, ext);
                            (ext_hash, None)
                        } else {
                            (branch_hash, None)
                        }
                    } else {
                        // Create branch node
                        let mut branch = Node::new_branch();

                        let old_nibble = key_suffix[common_len];
                        let new_nibble = path[common_len];

                        // Create leaves for both paths
                        let old_leaf =
                            Node::new_leaf(key_suffix[common_len + 1..].to_vec(), old_value);
                        let old_hash = old_leaf.compute_hash();
                        self.nodes.insert(old_hash, old_leaf);

                        let new_leaf = Node::new_leaf(path[common_len + 1..].to_vec(), value);
                        let new_hash = new_leaf.compute_hash();
                        self.nodes.insert(new_hash, new_leaf);

                        if let Node::Branch {
                            ref mut children, ..
                        } = branch
                        {
                            children[old_nibble as usize] = Some(old_hash);
                            children[new_nibble as usize] = Some(new_hash);
                        }

                        let branch_hash = branch.compute_hash();
                        self.nodes.insert(branch_hash, branch);

                        // Wrap in extension if there's a common prefix
                        if common_len > 0 {
                            let ext = Node::new_extension(path[..common_len].to_vec(), branch_hash);
                            let ext_hash = ext.compute_hash();
                            self.nodes.insert(ext_hash, ext);
                            (ext_hash, None)
                        } else {
                            (branch_hash, None)
                        }
                    }
                }
            }
            Node::Extension { prefix, child_hash } => {
                let common_len = common_prefix_length(&prefix, path);

                if common_len == prefix.len() {
                    // Path matches entire prefix - recurse to child
                    let (new_child_hash, old_value) =
                        self.insert_at(child_hash, &path[common_len..], value);

                    let new_ext = Node::new_extension(prefix, new_child_hash);
                    let new_hash = new_ext.compute_hash();
                    self.nodes.insert(new_hash, new_ext);
                    (new_hash, old_value)
                } else {
                    // Split the extension
                    let mut branch = Node::new_branch();

                    let ext_nibble = prefix[common_len];
                    let path_nibble = path[common_len];

                    // Create extension or child for the remainder of the original extension
                    let remainder_path = &prefix[common_len + 1..];
                    let child_to_insert = if remainder_path.is_empty() {
                        child_hash
                    } else {
                        let remainder_ext =
                            Node::new_extension(remainder_path.to_vec(), child_hash);
                        let remainder_hash = remainder_ext.compute_hash();
                        self.nodes.insert(remainder_hash, remainder_ext);
                        remainder_hash
                    };

                    // Create leaf for new path
                    let new_leaf = Node::new_leaf(path[common_len + 1..].to_vec(), value);
                    let new_leaf_hash = new_leaf.compute_hash();
                    self.nodes.insert(new_leaf_hash, new_leaf);

                    if let Node::Branch {
                        ref mut children, ..
                    } = branch
                    {
                        children[ext_nibble as usize] = Some(child_to_insert);
                        children[path_nibble as usize] = Some(new_leaf_hash);
                    }

                    let branch_hash = branch.compute_hash();
                    self.nodes.insert(branch_hash, branch);

                    // Wrap in extension if there's a common prefix
                    if common_len > 0 {
                        let new_ext = Node::new_extension(path[..common_len].to_vec(), branch_hash);
                        let new_ext_hash = new_ext.compute_hash();
                        self.nodes.insert(new_ext_hash, new_ext);
                        (new_ext_hash, None)
                    } else {
                        (branch_hash, None)
                    }
                }
            }
            Node::Branch {
                mut children,
                value: branch_value,
            } => {
                if path.is_empty() {
                    // Update branch value
                    let old_value = branch_value;
                    let new_branch = Node::Branch {
                        children,
                        value: Some(value),
                    };
                    let new_hash = new_branch.compute_hash();
                    self.nodes.insert(new_hash, new_branch);
                    (new_hash, old_value)
                } else {
                    // Recurse to child
                    let nibble = path[0] as usize;
                    let (new_child_hash, old_value) = if let Some(child_hash) = children[nibble] {
                        self.insert_at(child_hash, &path[1..], value)
                    } else {
                        // Create new leaf
                        let leaf = Node::new_leaf(path[1..].to_vec(), value);
                        let leaf_hash = leaf.compute_hash();
                        self.nodes.insert(leaf_hash, leaf);
                        (leaf_hash, None)
                    };

                    children[nibble] = Some(new_child_hash);
                    let new_branch = Node::Branch {
                        children,
                        value: branch_value,
                    };
                    let new_hash = new_branch.compute_hash();
                    self.nodes.insert(new_hash, new_branch);
                    (new_hash, old_value)
                }
            }
        }
    }

    /// Recursive get helper
    fn get_at(&self, node_hash: Hash, path: &[u8]) -> Option<Vec<u8>> {
        let node = self.nodes.get(&node_hash)?;

        match node {
            Node::Leaf { key_suffix, value } => {
                if key_suffix == path {
                    Some(value.clone())
                } else {
                    None
                }
            }
            Node::Extension { prefix, child_hash } => {
                if path.len() >= prefix.len() && &path[..prefix.len()] == prefix.as_slice() {
                    self.get_at(*child_hash, &path[prefix.len()..])
                } else {
                    None
                }
            }
            Node::Branch { children, value } => {
                if path.is_empty() {
                    value.clone()
                } else {
                    let nibble = path[0] as usize;
                    children[nibble].and_then(|child_hash| self.get_at(child_hash, &path[1..]))
                }
            }
        }
    }

    /// Recursive delete helper
    ///
    /// Returns Some((new_node_hash, deleted_value)) or None if key not found
    fn delete_at(&mut self, node_hash: Hash, path: &[u8]) -> Option<(Option<Hash>, Vec<u8>)> {
        let node = self.nodes.get(&node_hash).cloned()?;

        match node {
            Node::Leaf { key_suffix, value } => {
                if key_suffix == path {
                    // Delete this leaf
                    Some((None, value))
                } else {
                    None
                }
            }
            Node::Extension { prefix, child_hash } => {
                if path.len() >= prefix.len() && &path[..prefix.len()] == prefix.as_slice() {
                    let (new_child, deleted_value) =
                        self.delete_at(child_hash, &path[prefix.len()..])?;

                    if let Some(child) = new_child {
                        // Child still exists - recreate extension
                        let child_node = self.nodes.get(&child).cloned()?;

                        // Collapse extension + extension or extension + leaf
                        match child_node {
                            Node::Extension {
                                prefix: child_prefix,
                                child_hash: grandchild,
                            } => {
                                // Merge extensions
                                let mut merged_prefix = prefix.clone();
                                merged_prefix.extend_from_slice(&child_prefix);
                                let merged_ext = Node::new_extension(merged_prefix, grandchild);
                                let merged_hash = merged_ext.compute_hash();
                                self.nodes.insert(merged_hash, merged_ext);
                                Some((Some(merged_hash), deleted_value))
                            }
                            Node::Leaf {
                                key_suffix,
                                value: leaf_value,
                            } => {
                                // Merge extension + leaf
                                let mut merged_key = prefix.clone();
                                merged_key.extend_from_slice(&key_suffix);
                                let merged_leaf = Node::new_leaf(merged_key, leaf_value);
                                let merged_hash = merged_leaf.compute_hash();
                                self.nodes.insert(merged_hash, merged_leaf);
                                Some((Some(merged_hash), deleted_value))
                            }
                            Node::Branch { .. } => {
                                // Keep extension
                                let new_ext = Node::new_extension(prefix, child);
                                let new_hash = new_ext.compute_hash();
                                self.nodes.insert(new_hash, new_ext);
                                Some((Some(new_hash), deleted_value))
                            }
                        }
                    } else {
                        // Child was deleted
                        Some((None, deleted_value))
                    }
                } else {
                    None
                }
            }
            Node::Branch {
                mut children,
                value: branch_value,
            } => {
                if path.is_empty() {
                    // Delete branch value
                    if let Some(val) = branch_value {
                        // Check if we can collapse the branch
                        let child_count: usize = children.iter().filter(|c| c.is_some()).count();

                        if child_count == 0 {
                            // No children - delete branch
                            Some((None, val))
                        } else if child_count == 1 {
                            // One child - collapse to extension or leaf
                            let (nibble, child_hash) = children
                                .iter()
                                .enumerate()
                                .find_map(|(i, c)| c.map(|h| (i, h)))?;

                            let child_node = self.nodes.get(&child_hash).cloned()?;
                            match child_node {
                                Node::Leaf {
                                    key_suffix,
                                    value: leaf_value,
                                } => {
                                    let mut new_key = vec![nibble as u8];
                                    new_key.extend_from_slice(&key_suffix);
                                    let new_leaf = Node::new_leaf(new_key, leaf_value);
                                    let new_hash = new_leaf.compute_hash();
                                    self.nodes.insert(new_hash, new_leaf);
                                    Some((Some(new_hash), val))
                                }
                                Node::Extension {
                                    prefix,
                                    child_hash: grandchild,
                                } => {
                                    let mut new_prefix = vec![nibble as u8];
                                    new_prefix.extend_from_slice(&prefix);
                                    let new_ext = Node::new_extension(new_prefix, grandchild);
                                    let new_hash = new_ext.compute_hash();
                                    self.nodes.insert(new_hash, new_ext);
                                    Some((Some(new_hash), val))
                                }
                                Node::Branch { .. } => {
                                    // Create extension to branch
                                    let new_ext =
                                        Node::new_extension(vec![nibble as u8], child_hash);
                                    let new_hash = new_ext.compute_hash();
                                    self.nodes.insert(new_hash, new_ext);
                                    Some((Some(new_hash), val))
                                }
                            }
                        } else {
                            // Multiple children - just remove value
                            let new_branch = Node::Branch {
                                children,
                                value: None,
                            };
                            let new_hash = new_branch.compute_hash();
                            self.nodes.insert(new_hash, new_branch);
                            Some((Some(new_hash), val))
                        }
                    } else {
                        None
                    }
                } else {
                    let nibble = path[0] as usize;
                    if let Some(child_hash) = children[nibble] {
                        let (new_child, deleted_value) = self.delete_at(child_hash, &path[1..])?;

                        children[nibble] = new_child;

                        // Check if we need to collapse the branch
                        let child_count: usize = children.iter().filter(|c| c.is_some()).count();

                        if child_count == 0 && branch_value.is_none() {
                            // No children or value - delete branch
                            Some((None, deleted_value))
                        } else if child_count == 1 && branch_value.is_none() {
                            // One child and no value - collapse
                            let (child_nibble, child_hash) = children
                                .iter()
                                .enumerate()
                                .find_map(|(i, c)| c.map(|h| (i, h)))?;

                            let child_node = self.nodes.get(&child_hash).cloned()?;
                            match child_node {
                                Node::Leaf {
                                    key_suffix,
                                    value: leaf_value,
                                } => {
                                    let mut new_key = vec![child_nibble as u8];
                                    new_key.extend_from_slice(&key_suffix);
                                    let new_leaf = Node::new_leaf(new_key, leaf_value);
                                    let new_hash = new_leaf.compute_hash();
                                    self.nodes.insert(new_hash, new_leaf);
                                    Some((Some(new_hash), deleted_value))
                                }
                                Node::Extension {
                                    prefix,
                                    child_hash: grandchild,
                                } => {
                                    let mut new_prefix = vec![child_nibble as u8];
                                    new_prefix.extend_from_slice(&prefix);
                                    let new_ext = Node::new_extension(new_prefix, grandchild);
                                    let new_hash = new_ext.compute_hash();
                                    self.nodes.insert(new_hash, new_ext);
                                    Some((Some(new_hash), deleted_value))
                                }
                                Node::Branch { .. } => {
                                    // Create extension
                                    let new_ext =
                                        Node::new_extension(vec![child_nibble as u8], child_hash);
                                    let new_hash = new_ext.compute_hash();
                                    self.nodes.insert(new_hash, new_ext);
                                    Some((Some(new_hash), deleted_value))
                                }
                            }
                        } else {
                            // Keep branch
                            let new_branch = Node::Branch {
                                children,
                                value: branch_value,
                            };
                            let new_hash = new_branch.compute_hash();
                            self.nodes.insert(new_hash, new_branch);
                            Some((Some(new_hash), deleted_value))
                        }
                    } else {
                        None
                    }
                }
            }
        }
    }
}

impl Default for Trie {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Construction Tests
    // =========================================================================

    #[test]
    fn test_new_trie() {
        let trie = Trie::new();
        assert_eq!(trie.root(), None);
    }

    #[test]
    fn test_default_trie() {
        let trie = Trie::default();
        assert_eq!(trie.root(), None);
    }

    // =========================================================================
    // Basic Insert Tests
    // =========================================================================

    #[test]
    fn test_insert_single_key() {
        let mut trie = Trie::new();
        let old = trie.insert(b"key", b"value".to_vec());
        assert_eq!(old, None);
        assert!(trie.root().is_some());
    }

    #[test]
    fn test_insert_and_get() {
        let mut trie = Trie::new();
        trie.insert(b"key", b"value".to_vec());
        assert_eq!(trie.get(b"key"), Some(b"value".to_vec()));
    }

    #[test]
    fn test_insert_overwrite() {
        let mut trie = Trie::new();
        trie.insert(b"key", b"value1".to_vec());
        let old = trie.insert(b"key", b"value2".to_vec());
        assert_eq!(old, Some(b"value1".to_vec()));
        assert_eq!(trie.get(b"key"), Some(b"value2".to_vec()));
    }

    #[test]
    fn test_insert_multiple_keys() {
        let mut trie = Trie::new();
        trie.insert(b"key1", b"value1".to_vec());
        trie.insert(b"key2", b"value2".to_vec());
        trie.insert(b"key3", b"value3".to_vec());

        assert_eq!(trie.get(b"key1"), Some(b"value1".to_vec()));
        assert_eq!(trie.get(b"key2"), Some(b"value2".to_vec()));
        assert_eq!(trie.get(b"key3"), Some(b"value3".to_vec()));
    }

    #[test]
    fn test_insert_empty_key() {
        let mut trie = Trie::new();
        trie.insert(b"", b"empty_key".to_vec());
        assert_eq!(trie.get(b""), Some(b"empty_key".to_vec()));
    }

    #[test]
    fn test_insert_empty_value() {
        let mut trie = Trie::new();
        trie.insert(b"key", vec![]);
        assert_eq!(trie.get(b"key"), Some(vec![]));
    }

    // =========================================================================
    // Get Tests
    // =========================================================================

    #[test]
    fn test_get_nonexistent() {
        let trie = Trie::new();
        assert_eq!(trie.get(b"key"), None);
    }

    #[test]
    fn test_get_after_multiple_inserts() {
        let mut trie = Trie::new();
        trie.insert(b"apple", b"fruit".to_vec());
        trie.insert(b"application", b"software".to_vec());
        trie.insert(b"apply", b"verb".to_vec());

        assert_eq!(trie.get(b"apple"), Some(b"fruit".to_vec()));
        assert_eq!(trie.get(b"application"), Some(b"software".to_vec()));
        assert_eq!(trie.get(b"apply"), Some(b"verb".to_vec()));
        assert_eq!(trie.get(b"app"), None);
    }

    #[test]
    fn test_get_prefix_key() {
        let mut trie = Trie::new();
        trie.insert(b"test", b"value1".to_vec());
        trie.insert(b"testing", b"value2".to_vec());

        assert_eq!(trie.get(b"test"), Some(b"value1".to_vec()));
        assert_eq!(trie.get(b"testing"), Some(b"value2".to_vec()));
        assert_eq!(trie.get(b"te"), None);
    }

    // =========================================================================
    // Delete Tests
    // =========================================================================

    #[test]
    fn test_delete_from_empty() {
        let mut trie = Trie::new();
        assert_eq!(trie.delete(b"key"), None);
    }

    #[test]
    fn test_delete_single_key() {
        let mut trie = Trie::new();
        trie.insert(b"key", b"value".to_vec());
        let deleted = trie.delete(b"key");
        assert_eq!(deleted, Some(b"value".to_vec()));
        assert_eq!(trie.get(b"key"), None);
        assert_eq!(trie.root(), None);
    }

    #[test]
    fn test_delete_nonexistent() {
        let mut trie = Trie::new();
        trie.insert(b"key1", b"value1".to_vec());
        assert_eq!(trie.delete(b"key2"), None);
        assert_eq!(trie.get(b"key1"), Some(b"value1".to_vec()));
    }

    #[test]
    fn test_delete_one_of_many() {
        let mut trie = Trie::new();
        trie.insert(b"key1", b"value1".to_vec());
        trie.insert(b"key2", b"value2".to_vec());
        trie.insert(b"key3", b"value3".to_vec());

        let deleted = trie.delete(b"key2");
        assert_eq!(deleted, Some(b"value2".to_vec()));
        assert_eq!(trie.get(b"key1"), Some(b"value1".to_vec()));
        assert_eq!(trie.get(b"key2"), None);
        assert_eq!(trie.get(b"key3"), Some(b"value3".to_vec()));
    }

    #[test]
    fn test_delete_and_reinsert() {
        let mut trie = Trie::new();
        trie.insert(b"key", b"value1".to_vec());
        trie.delete(b"key");
        trie.insert(b"key", b"value2".to_vec());
        assert_eq!(trie.get(b"key"), Some(b"value2".to_vec()));
    }

    // =========================================================================
    // Path Splitting Tests
    // =========================================================================

    #[test]
    fn test_split_leaf_common_prefix() {
        let mut trie = Trie::new();
        trie.insert(b"test", b"value1".to_vec());
        trie.insert(b"team", b"value2".to_vec());

        assert_eq!(trie.get(b"test"), Some(b"value1".to_vec()));
        assert_eq!(trie.get(b"team"), Some(b"value2".to_vec()));
    }

    #[test]
    fn test_split_leaf_no_common_prefix() {
        let mut trie = Trie::new();
        trie.insert(&[0x10], vec![1]);
        trie.insert(&[0x20], vec![2]);

        assert_eq!(trie.get(&[0x10]), Some(vec![1]));
        assert_eq!(trie.get(&[0x20]), Some(vec![2]));
    }

    #[test]
    fn test_insert_prefix_of_existing() {
        let mut trie = Trie::new();
        trie.insert(b"testing", b"value1".to_vec());
        trie.insert(b"test", b"value2".to_vec());

        assert_eq!(trie.get(b"test"), Some(b"value2".to_vec()));
        assert_eq!(trie.get(b"testing"), Some(b"value1".to_vec()));
    }

    #[test]
    fn test_insert_extends_existing() {
        let mut trie = Trie::new();
        trie.insert(b"test", b"value1".to_vec());
        trie.insert(b"testing", b"value2".to_vec());

        assert_eq!(trie.get(b"test"), Some(b"value1".to_vec()));
        assert_eq!(trie.get(b"testing"), Some(b"value2".to_vec()));
    }

    // =========================================================================
    // Branch Node Tests
    // =========================================================================

    #[test]
    fn test_branch_with_multiple_children() {
        let mut trie = Trie::new();
        trie.insert(&[0x10, 0x00], vec![1]);
        trie.insert(&[0x20, 0x00], vec![2]);
        trie.insert(&[0x30, 0x00], vec![3]);

        assert_eq!(trie.get(&[0x10, 0x00]), Some(vec![1]));
        assert_eq!(trie.get(&[0x20, 0x00]), Some(vec![2]));
        assert_eq!(trie.get(&[0x30, 0x00]), Some(vec![3]));
    }

    #[test]
    fn test_branch_with_value() {
        let mut trie = Trie::new();
        trie.insert(b"test", b"value1".to_vec());
        trie.insert(b"testing", b"value2".to_vec());

        // "test" should be stored as a branch value
        assert_eq!(trie.get(b"test"), Some(b"value1".to_vec()));
        assert_eq!(trie.get(b"testing"), Some(b"value2".to_vec()));
    }

    #[test]
    fn test_delete_branch_value() {
        let mut trie = Trie::new();
        trie.insert(b"test", b"value1".to_vec());
        trie.insert(b"testing", b"value2".to_vec());

        trie.delete(b"test");
        assert_eq!(trie.get(b"test"), None);
        assert_eq!(trie.get(b"testing"), Some(b"value2".to_vec()));
    }

    // =========================================================================
    // Extension Node Tests
    // =========================================================================

    #[test]
    fn test_extension_creation() {
        let mut trie = Trie::new();
        // These should create an extension node for the common prefix
        trie.insert(&[0x12, 0x34, 0x00], vec![1]);
        trie.insert(&[0x12, 0x34, 0x10], vec![2]);

        assert_eq!(trie.get(&[0x12, 0x34, 0x00]), Some(vec![1]));
        assert_eq!(trie.get(&[0x12, 0x34, 0x10]), Some(vec![2]));
    }

    #[test]
    fn test_split_extension() {
        let mut trie = Trie::new();
        trie.insert(&[0x12, 0x34, 0x56], vec![1]);
        trie.insert(&[0x12, 0x34, 0x78], vec![2]);
        trie.insert(&[0x12, 0x44], vec![3]);

        assert_eq!(trie.get(&[0x12, 0x34, 0x56]), Some(vec![1]));
        assert_eq!(trie.get(&[0x12, 0x34, 0x78]), Some(vec![2]));
        assert_eq!(trie.get(&[0x12, 0x44]), Some(vec![3]));
    }

    // =========================================================================
    // Complex Sequence Tests
    // =========================================================================

    #[test]
    fn test_insert_delete_sequence() {
        let mut trie = Trie::new();

        // Insert multiple keys
        trie.insert(b"key1", b"value1".to_vec());
        trie.insert(b"key2", b"value2".to_vec());
        trie.insert(b"key3", b"value3".to_vec());

        // Delete one
        trie.delete(b"key2");

        // Insert new key
        trie.insert(b"key4", b"value4".to_vec());

        // Verify state
        assert_eq!(trie.get(b"key1"), Some(b"value1".to_vec()));
        assert_eq!(trie.get(b"key2"), None);
        assert_eq!(trie.get(b"key3"), Some(b"value3".to_vec()));
        assert_eq!(trie.get(b"key4"), Some(b"value4".to_vec()));
    }

    #[test]
    fn test_overwrite_sequence() {
        let mut trie = Trie::new();

        trie.insert(b"key", b"value1".to_vec());
        assert_eq!(trie.get(b"key"), Some(b"value1".to_vec()));

        trie.insert(b"key", b"value2".to_vec());
        assert_eq!(trie.get(b"key"), Some(b"value2".to_vec()));

        trie.insert(b"key", b"value3".to_vec());
        assert_eq!(trie.get(b"key"), Some(b"value3".to_vec()));
    }

    #[test]
    fn test_delete_all_keys() {
        let mut trie = Trie::new();

        trie.insert(b"key1", b"value1".to_vec());
        trie.insert(b"key2", b"value2".to_vec());
        trie.insert(b"key3", b"value3".to_vec());

        trie.delete(b"key1");
        trie.delete(b"key2");
        trie.delete(b"key3");

        assert_eq!(trie.root(), None);
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_single_byte_keys() {
        let mut trie = Trie::new();

        for i in 0..16u8 {
            trie.insert(&[i], vec![i]);
        }

        for i in 0..16u8 {
            assert_eq!(trie.get(&[i]), Some(vec![i]));
        }
    }

    #[test]
    fn test_long_keys() {
        let mut trie = Trie::new();
        let key = vec![0x42; 100];
        trie.insert(&key, b"long_key".to_vec());
        assert_eq!(trie.get(&key), Some(b"long_key".to_vec()));
    }

    #[test]
    fn test_similar_keys() {
        let mut trie = Trie::new();

        trie.insert(b"abc", b"value1".to_vec());
        trie.insert(b"abd", b"value2".to_vec());
        trie.insert(b"abe", b"value3".to_vec());

        assert_eq!(trie.get(b"abc"), Some(b"value1".to_vec()));
        assert_eq!(trie.get(b"abd"), Some(b"value2".to_vec()));
        assert_eq!(trie.get(b"abe"), Some(b"value3".to_vec()));
        assert_eq!(trie.get(b"ab"), None);
    }

    #[test]
    fn test_binary_keys() {
        let mut trie = Trie::new();

        trie.insert(&[0x00, 0x00], vec![1]);
        trie.insert(&[0x00, 0xFF], vec![2]);
        trie.insert(&[0xFF, 0x00], vec![3]);
        trie.insert(&[0xFF, 0xFF], vec![4]);

        assert_eq!(trie.get(&[0x00, 0x00]), Some(vec![1]));
        assert_eq!(trie.get(&[0x00, 0xFF]), Some(vec![2]));
        assert_eq!(trie.get(&[0xFF, 0x00]), Some(vec![3]));
        assert_eq!(trie.get(&[0xFF, 0xFF]), Some(vec![4]));
    }

    // =========================================================================
    // Node Collapse Tests
    // =========================================================================

    #[test]
    fn test_collapse_after_delete() {
        let mut trie = Trie::new();

        trie.insert(b"test1", b"value1".to_vec());
        trie.insert(b"test2", b"value2".to_vec());

        // After deleting one, the structure should collapse
        trie.delete(b"test2");

        assert_eq!(trie.get(b"test1"), Some(b"value1".to_vec()));
        assert_eq!(trie.get(b"test2"), None);
    }

    #[test]
    fn test_branch_collapse_to_leaf() {
        let mut trie = Trie::new();

        trie.insert(&[0x10, 0x00], vec![1]);
        trie.insert(&[0x20, 0x00], vec![2]);

        trie.delete(&[0x20, 0x00]);

        assert_eq!(trie.get(&[0x10, 0x00]), Some(vec![1]));
    }

    // =========================================================================
    // Hash Consistency Tests
    // =========================================================================

    #[test]
    fn test_same_content_same_hash() {
        let mut trie1 = Trie::new();
        let mut trie2 = Trie::new();

        trie1.insert(b"key1", b"value1".to_vec());
        trie1.insert(b"key2", b"value2".to_vec());

        trie2.insert(b"key1", b"value1".to_vec());
        trie2.insert(b"key2", b"value2".to_vec());

        assert_eq!(trie1.root(), trie2.root());
    }

    #[test]
    fn test_different_order_same_hash() {
        let mut trie1 = Trie::new();
        let mut trie2 = Trie::new();

        trie1.insert(b"key1", b"value1".to_vec());
        trie1.insert(b"key2", b"value2".to_vec());

        trie2.insert(b"key2", b"value2".to_vec());
        trie2.insert(b"key1", b"value1".to_vec());

        assert_eq!(trie1.root(), trie2.root());
    }

    #[test]
    fn test_different_content_different_hash() {
        let mut trie1 = Trie::new();
        let mut trie2 = Trie::new();

        trie1.insert(b"key", b"value1".to_vec());
        trie2.insert(b"key", b"value2".to_vec());

        assert_ne!(trie1.root(), trie2.root());
    }

    // =========================================================================
    // Large Trie Tests
    // =========================================================================

    #[test]
    fn test_many_sequential_inserts() {
        let mut trie = Trie::new();

        for i in 0..100 {
            let key = format!("key{i}");
            let value = format!("value{i}");
            trie.insert(key.as_bytes(), value.as_bytes().to_vec());
        }

        for i in 0..100 {
            let key = format!("key{i}");
            let value = format!("value{i}");
            assert_eq!(trie.get(key.as_bytes()), Some(value.as_bytes().to_vec()));
        }
    }

    #[test]
    fn test_many_random_operations() {
        let mut trie = Trie::new();

        // Insert
        for i in 0..50 {
            trie.insert(&[i], vec![i]);
        }

        // Update
        for i in 0..25 {
            trie.insert(&[i], vec![i + 100]);
        }

        // Delete
        for i in 25..50 {
            trie.delete(&[i]);
        }

        // Verify
        for i in 0..25 {
            assert_eq!(trie.get(&[i]), Some(vec![i + 100]));
        }
        for i in 25..50 {
            assert_eq!(trie.get(&[i]), None);
        }
    }

    // =========================================================================
    // Root Computation Tests
    // =========================================================================

    #[test]
    fn test_compute_root_empty_trie() {
        let trie = Trie::new();
        assert_eq!(trie.compute_root(), EMPTY_TRIE_ROOT);
    }

    #[test]
    fn test_compute_root_single_leaf() {
        let mut trie = Trie::new();
        trie.insert(b"key", b"value".to_vec());

        let root = trie.compute_root();
        assert_ne!(root, Hash::ZERO);
        assert_eq!(root, trie.root().unwrap());
    }

    #[test]
    fn test_compute_root_deterministic() {
        let mut trie = Trie::new();
        trie.insert(b"key1", b"value1".to_vec());
        trie.insert(b"key2", b"value2".to_vec());

        let root1 = trie.compute_root();
        let root2 = trie.compute_root();

        assert_eq!(root1, root2);
    }

    #[test]
    fn test_compute_root_same_trie_same_root() {
        let mut trie1 = Trie::new();
        let mut trie2 = Trie::new();

        trie1.insert(b"key1", b"value1".to_vec());
        trie1.insert(b"key2", b"value2".to_vec());

        trie2.insert(b"key1", b"value1".to_vec());
        trie2.insert(b"key2", b"value2".to_vec());

        assert_eq!(trie1.compute_root(), trie2.compute_root());
    }

    #[test]
    fn test_compute_root_different_tries_different_roots() {
        let mut trie1 = Trie::new();
        let mut trie2 = Trie::new();

        trie1.insert(b"key", b"value1".to_vec());
        trie2.insert(b"key", b"value2".to_vec());

        assert_ne!(trie1.compute_root(), trie2.compute_root());
    }

    #[test]
    fn test_compute_root_after_insert() {
        let mut trie = Trie::new();

        let root_empty = trie.compute_root();
        assert_eq!(root_empty, EMPTY_TRIE_ROOT);

        trie.insert(b"key", b"value".to_vec());
        let root_after_insert = trie.compute_root();
        assert_ne!(root_after_insert, EMPTY_TRIE_ROOT);
        assert_ne!(root_after_insert, root_empty);
    }

    #[test]
    fn test_compute_root_after_delete() {
        let mut trie = Trie::new();

        trie.insert(b"key1", b"value1".to_vec());
        trie.insert(b"key2", b"value2".to_vec());
        let root_before_delete = trie.compute_root();

        trie.delete(b"key1");
        let root_after_delete = trie.compute_root();

        assert_ne!(root_before_delete, root_after_delete);
    }

    #[test]
    fn test_compute_root_after_delete_all() {
        let mut trie = Trie::new();

        trie.insert(b"key", b"value".to_vec());
        assert_ne!(trie.compute_root(), Hash::ZERO);

        trie.delete(b"key");
        assert_eq!(trie.compute_root(), EMPTY_TRIE_ROOT);
    }

    #[test]
    fn test_compute_root_after_update() {
        let mut trie = Trie::new();

        trie.insert(b"key", b"value1".to_vec());
        let root_before = trie.compute_root();

        trie.insert(b"key", b"value2".to_vec());
        let root_after = trie.compute_root();

        assert_ne!(root_before, root_after);
    }

    #[test]
    fn test_compute_root_branch_node() {
        let mut trie = Trie::new();

        // Insert keys that will create a branch node
        trie.insert(&[0x10, 0x00], vec![1]);
        trie.insert(&[0x20, 0x00], vec![2]);
        trie.insert(&[0x30, 0x00], vec![3]);

        let root = trie.compute_root();
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn test_compute_root_extension_node() {
        let mut trie = Trie::new();

        // Insert keys that will create an extension node
        trie.insert(&[0x12, 0x34, 0x00], vec![1]);
        trie.insert(&[0x12, 0x34, 0x10], vec![2]);

        let root = trie.compute_root();
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn test_compute_root_with_prefix_keys() {
        let mut trie = Trie::new();

        trie.insert(b"test", b"value1".to_vec());
        trie.insert(b"testing", b"value2".to_vec());

        let root = trie.compute_root();
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn test_compute_root_insertion_order_invariant() {
        let mut trie1 = Trie::new();
        let mut trie2 = Trie::new();

        // Insert in different order
        trie1.insert(b"apple", b"fruit".to_vec());
        trie1.insert(b"banana", b"fruit".to_vec());
        trie1.insert(b"cherry", b"fruit".to_vec());

        trie2.insert(b"cherry", b"fruit".to_vec());
        trie2.insert(b"apple", b"fruit".to_vec());
        trie2.insert(b"banana", b"fruit".to_vec());

        assert_eq!(trie1.compute_root(), trie2.compute_root());
    }

    #[test]
    fn test_compute_root_empty_key() {
        let mut trie = Trie::new();

        trie.insert(b"", b"empty_key".to_vec());

        let root = trie.compute_root();
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn test_compute_root_empty_value() {
        let mut trie = Trie::new();

        trie.insert(b"key", vec![]);

        let root = trie.compute_root();
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn test_compute_root_multiple_operations() {
        let mut trie = Trie::new();

        // Series of operations
        trie.insert(b"key1", b"value1".to_vec());
        let root1 = trie.compute_root();

        trie.insert(b"key2", b"value2".to_vec());
        let root2 = trie.compute_root();

        trie.insert(b"key3", b"value3".to_vec());
        let root3 = trie.compute_root();

        trie.delete(b"key2");
        let root4 = trie.compute_root();

        // All roots should be different (except possibly root1 and root4 if structure collapsed the same way)
        assert_ne!(root1, root2);
        assert_ne!(root2, root3);
        assert_ne!(root3, root4);
    }

    #[test]
    fn test_compute_root_long_keys() {
        let mut trie = Trie::new();

        let key = vec![0x42; 100];
        trie.insert(&key, b"long_key".to_vec());

        let root = trie.compute_root();
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn test_compute_root_many_keys() {
        let mut trie = Trie::new();

        for i in 0..50 {
            let key = format!("key{i}");
            let value = format!("value{i}");
            trie.insert(key.as_bytes(), value.as_bytes().to_vec());
        }

        let root = trie.compute_root();
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn test_compute_root_binary_keys() {
        let mut trie = Trie::new();

        trie.insert(&[0x00, 0x00], vec![1]);
        trie.insert(&[0x00, 0xFF], vec![2]);
        trie.insert(&[0xFF, 0x00], vec![3]);
        trie.insert(&[0xFF, 0xFF], vec![4]);

        let root = trie.compute_root();
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn test_compute_root_similar_keys() {
        let mut trie = Trie::new();

        trie.insert(b"abc", b"value1".to_vec());
        trie.insert(b"abd", b"value2".to_vec());
        trie.insert(b"abe", b"value3".to_vec());

        let root = trie.compute_root();
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn test_compute_root_delete_and_reinsert() {
        let mut trie = Trie::new();

        trie.insert(b"key", b"value1".to_vec());
        let root1 = trie.compute_root();

        trie.delete(b"key");
        assert_eq!(trie.compute_root(), EMPTY_TRIE_ROOT);

        trie.insert(b"key", b"value1".to_vec());
        let root2 = trie.compute_root();

        // Should have the same root after delete and reinsert with same value
        assert_eq!(root1, root2);
    }

    #[test]
    fn test_compute_root_single_byte_keys() {
        let mut trie = Trie::new();

        for i in 0..16u8 {
            trie.insert(&[i], vec![i]);
        }

        let root = trie.compute_root();
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn test_compute_root_overwrite_multiple_times() {
        let mut trie = Trie::new();

        trie.insert(b"key", b"value1".to_vec());
        let root1 = trie.compute_root();

        trie.insert(b"key", b"value2".to_vec());
        let root2 = trie.compute_root();

        trie.insert(b"key", b"value3".to_vec());
        let root3 = trie.compute_root();

        assert_ne!(root1, root2);
        assert_ne!(root2, root3);
        assert_ne!(root1, root3);
    }

    #[test]
    fn test_compute_root_consistency_with_root_method() {
        let mut trie = Trie::new();

        // Empty trie
        assert_eq!(trie.compute_root(), EMPTY_TRIE_ROOT);
        assert_eq!(trie.root(), None);

        // After insert
        trie.insert(b"key", b"value".to_vec());
        assert_eq!(trie.compute_root(), trie.root().unwrap());

        // After multiple inserts
        trie.insert(b"key2", b"value2".to_vec());
        assert_eq!(trie.compute_root(), trie.root().unwrap());

        // After delete
        trie.delete(b"key");
        assert_eq!(trie.compute_root(), trie.root().unwrap());

        // After all deletes
        trie.delete(b"key2");
        assert_eq!(trie.compute_root(), EMPTY_TRIE_ROOT);
        assert_eq!(trie.root(), None);
    }

    #[test]
    fn test_compute_root_clone_has_same_root() {
        let mut trie = Trie::new();

        trie.insert(b"key1", b"value1".to_vec());
        trie.insert(b"key2", b"value2".to_vec());

        let cloned = trie.clone();

        assert_eq!(trie.compute_root(), cloned.compute_root());
    }

    #[test]
    fn test_compute_root_modified_clone_different_root() {
        let mut trie = Trie::new();

        trie.insert(b"key1", b"value1".to_vec());

        let mut cloned = trie.clone();
        cloned.insert(b"key2", b"value2".to_vec());

        assert_ne!(trie.compute_root(), cloned.compute_root());
    }

    #[test]
    fn test_compute_root_branch_collapse() {
        let mut trie = Trie::new();

        trie.insert(&[0x10, 0x00], vec![1]);
        trie.insert(&[0x20, 0x00], vec![2]);
        let root_before = trie.compute_root();

        trie.delete(&[0x20, 0x00]);
        let root_after = trie.compute_root();

        assert_ne!(root_before, root_after);
    }

    #[test]
    fn test_compute_root_extension_merge() {
        let mut trie = Trie::new();

        trie.insert(&[0x12, 0x34, 0x56], vec![1]);
        trie.insert(&[0x12, 0x34, 0x78], vec![2]);
        trie.insert(&[0x12, 0x44], vec![3]);
        let root_with_all = trie.compute_root();

        trie.delete(&[0x12, 0x44]);
        let root_after_delete = trie.compute_root();

        assert_ne!(root_with_all, root_after_delete);
    }

    #[test]
    fn test_compute_root_incremental_build() {
        let mut trie = Trie::new();
        let mut roots = Vec::new();

        // Build trie incrementally and collect roots
        for i in 0..10 {
            trie.insert(&[i], vec![i]);
            roots.push(trie.compute_root());
        }

        // All roots should be different
        for i in 0..roots.len() {
            for j in (i + 1)..roots.len() {
                assert_ne!(roots[i], roots[j]);
            }
        }
    }
}
