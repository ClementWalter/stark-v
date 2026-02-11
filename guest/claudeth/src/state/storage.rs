//! Contract storage management
//!
//! This module implements Ethereum contract storage using a Merkle Patricia Trie.
//! Each contract has its own storage trie that maps 256-bit keys to 256-bit values.

#[cfg(target_arch = "riscv32")]
extern crate alloc;

use crate::crypto::{keccak256, rlp};
use crate::state::partial_mpt::{Trie, proof::Proof};
use crate::types::{Hash, U256};

#[cfg(test)]
use crate::state::partial_mpt::EMPTY_TRIE_ROOT;

/// Contract storage trie
///
/// Maps 256-bit storage keys to 256-bit values using a Merkle Patricia Trie.
/// Storage keys are hashed with Keccak-256 before insertion, matching Ethereum.
/// Non-zero values are RLP-encoded before being stored in the trie.
/// Zero values are treated as deletions (key is removed from trie).
#[derive(Clone, Debug)]
pub struct Storage {
    /// Underlying MPT for storage
    trie: Trie,
}

impl Storage {
    /// Creates a new empty storage
    pub fn new() -> Self {
        Storage { trie: Trie::new() }
    }

    /// Gets a value from storage by key
    ///
    /// Returns U256::ZERO if the key doesn't exist (Ethereum behavior).
    pub fn get(&self, key: &U256) -> U256 {
        let key_hash = keccak256(&key.to_be_bytes());

        if let Some(value_rlp) = self.trie.get(key_hash.as_bytes()) {
            // Decode RLP-encoded U256
            if let Ok((value, _)) = rlp::decode_u256(&value_rlp) {
                value
            } else {
                U256::ZERO
            }
        } else {
            U256::ZERO
        }
    }

    /// Sets a value in storage
    ///
    /// If the value is zero, the key is deleted (Ethereum behavior).
    /// Returns the previous value.
    pub fn set(&mut self, key: &U256, value: U256) -> U256 {
        let key_hash = keccak256(&key.to_be_bytes());

        if value == U256::ZERO {
            // Delete the key if value is zero
            if let Some(old_rlp) = self.trie.delete(key_hash.as_bytes())
                && let Ok((old_value, _)) = rlp::decode_u256(&old_rlp)
            {
                return old_value;
            }
            U256::ZERO
        } else {
            // RLP-encode the value and insert
            let value_rlp = rlp::encode_u256(&value);

            if let Some(old_rlp) = self.trie.insert(key_hash.as_bytes(), value_rlp)
                && let Ok((old_value, _)) = rlp::decode_u256(&old_rlp)
            {
                return old_value;
            }
            U256::ZERO
        }
    }

    /// Computes the storage root hash
    ///
    /// Returns the root hash of the underlying trie.
    /// An empty storage returns Hash::ZERO.
    pub fn compute_root(&self) -> Hash {
        self.trie.compute_root()
    }

    /// Returns the current root hash
    pub fn root(&self) -> Option<Hash> {
        self.trie.root()
    }

    /// Generates a Merkle proof for a storage key
    ///
    /// The proof can be used to verify the value at this key.
    pub fn generate_proof(
        &self,
        key: &U256,
    ) -> Result<Proof, crate::state::partial_mpt::proof::ProofError> {
        let key_hash = keccak256(&key.to_be_bytes());
        self.trie.generate_proof(key_hash.as_bytes())
    }

    /// Returns true if the storage is empty
    pub fn is_empty(&self) -> bool {
        self.trie.root().is_none()
    }

    /// Clears all storage (removes all keys)
    pub fn clear(&mut self) {
        self.trie = Trie::new();
    }
}

impl Default for Storage {
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
    fn test_new_storage() {
        let storage = Storage::new();
        assert!(storage.is_empty());
        assert_eq!(storage.root(), None);
        assert_eq!(storage.compute_root(), EMPTY_TRIE_ROOT);
    }

    #[test]
    fn test_default_storage() {
        let storage = Storage::default();
        assert!(storage.is_empty());
        assert_eq!(storage.compute_root(), EMPTY_TRIE_ROOT);
    }

    // =========================================================================
    // Get/Set Tests
    // =========================================================================

    #[test]
    fn test_get_nonexistent_returns_zero() {
        let storage = Storage::new();
        let key = U256::from(42u64);
        assert_eq!(storage.get(&key), U256::ZERO);
    }

    #[test]
    fn test_set_and_get() {
        let mut storage = Storage::new();
        let key = U256::from(1u64);
        let value = U256::from(100u64);

        let old = storage.set(&key, value);
        assert_eq!(old, U256::ZERO);
        assert_eq!(storage.get(&key), value);
    }

    #[test]
    fn test_set_multiple_keys() {
        let mut storage = Storage::new();

        storage.set(&U256::from(1u64), U256::from(10u64));
        storage.set(&U256::from(2u64), U256::from(20u64));
        storage.set(&U256::from(3u64), U256::from(30u64));

        assert_eq!(storage.get(&U256::from(1u64)), U256::from(10u64));
        assert_eq!(storage.get(&U256::from(2u64)), U256::from(20u64));
        assert_eq!(storage.get(&U256::from(3u64)), U256::from(30u64));
    }

    #[test]
    fn test_set_overwrites_value() {
        let mut storage = Storage::new();
        let key = U256::from(5u64);

        storage.set(&key, U256::from(100u64));
        let old = storage.set(&key, U256::from(200u64));

        assert_eq!(old, U256::from(100u64));
        assert_eq!(storage.get(&key), U256::from(200u64));
    }

    #[test]
    fn test_set_zero_deletes_key() {
        let mut storage = Storage::new();
        let key = U256::from(10u64);

        storage.set(&key, U256::from(999u64));
        assert_eq!(storage.get(&key), U256::from(999u64));

        let old = storage.set(&key, U256::ZERO);
        assert_eq!(old, U256::from(999u64));
        assert_eq!(storage.get(&key), U256::ZERO);
    }

    #[test]
    fn test_set_zero_on_nonexistent_key() {
        let mut storage = Storage::new();
        let key = U256::from(42u64);

        let old = storage.set(&key, U256::ZERO);
        assert_eq!(old, U256::ZERO);
        assert_eq!(storage.get(&key), U256::ZERO);
    }

    // =========================================================================
    // Root Computation Tests
    // =========================================================================

    #[test]
    fn test_compute_root_empty() {
        let storage = Storage::new();
        assert_eq!(storage.compute_root(), EMPTY_TRIE_ROOT);
    }

    #[test]
    fn test_compute_root_single_value() {
        let mut storage = Storage::new();
        storage.set(&U256::from(1u64), U256::from(100u64));

        let root = storage.compute_root();
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn test_compute_root_multiple_values() {
        let mut storage = Storage::new();
        storage.set(&U256::from(1u64), U256::from(10u64));
        storage.set(&U256::from(2u64), U256::from(20u64));
        storage.set(&U256::from(3u64), U256::from(30u64));

        let root = storage.compute_root();
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn test_compute_root_deterministic() {
        let mut storage = Storage::new();
        storage.set(&U256::from(1u64), U256::from(100u64));

        let root1 = storage.compute_root();
        let root2 = storage.compute_root();
        assert_eq!(root1, root2);
    }

    #[test]
    fn test_compute_root_same_data_same_root() {
        let mut storage1 = Storage::new();
        let mut storage2 = Storage::new();

        storage1.set(&U256::from(1u64), U256::from(10u64));
        storage1.set(&U256::from(2u64), U256::from(20u64));

        storage2.set(&U256::from(1u64), U256::from(10u64));
        storage2.set(&U256::from(2u64), U256::from(20u64));

        assert_eq!(storage1.compute_root(), storage2.compute_root());
    }

    #[test]
    fn test_compute_root_order_independent() {
        let mut storage1 = Storage::new();
        let mut storage2 = Storage::new();

        storage1.set(&U256::from(1u64), U256::from(10u64));
        storage1.set(&U256::from(2u64), U256::from(20u64));

        storage2.set(&U256::from(2u64), U256::from(20u64));
        storage2.set(&U256::from(1u64), U256::from(10u64));

        assert_eq!(storage1.compute_root(), storage2.compute_root());
    }

    #[test]
    fn test_compute_root_different_values_different_roots() {
        let mut storage1 = Storage::new();
        let mut storage2 = Storage::new();

        storage1.set(&U256::from(1u64), U256::from(10u64));
        storage2.set(&U256::from(1u64), U256::from(20u64));

        assert_ne!(storage1.compute_root(), storage2.compute_root());
    }

    #[test]
    fn test_compute_root_after_delete() {
        let mut storage = Storage::new();
        storage.set(&U256::from(1u64), U256::from(10u64));
        storage.set(&U256::from(2u64), U256::from(20u64));
        let root_before = storage.compute_root();

        storage.set(&U256::from(1u64), U256::ZERO);
        let root_after = storage.compute_root();

        assert_ne!(root_before, root_after);
    }

    #[test]
    fn test_compute_root_delete_all_returns_zero() {
        let mut storage = Storage::new();
        storage.set(&U256::from(1u64), U256::from(10u64));
        storage.set(&U256::from(2u64), U256::from(20u64));

        storage.set(&U256::from(1u64), U256::ZERO);
        storage.set(&U256::from(2u64), U256::ZERO);

        assert_eq!(storage.compute_root(), EMPTY_TRIE_ROOT);
        assert!(storage.is_empty());
    }

    // =========================================================================
    // Proof Tests
    // =========================================================================

    #[test]
    fn test_generate_proof_empty_storage() {
        let storage = Storage::new();
        let key = U256::from(1u64);
        let proof = storage.generate_proof(&key).unwrap();
        assert!(proof.is_empty());
    }

    #[test]
    fn test_generate_proof_existing_key() {
        let mut storage = Storage::new();
        let key = U256::from(1u64);
        storage.set(&key, U256::from(100u64));

        let proof = storage.generate_proof(&key).unwrap();
        assert!(!proof.is_empty());
    }

    #[test]
    fn test_generate_proof_nonexistent_key() {
        let mut storage = Storage::new();
        storage.set(&U256::from(1u64), U256::from(100u64));

        let proof = storage.generate_proof(&U256::from(999u64)).unwrap();
        assert!(!proof.is_empty());
    }

    #[test]
    fn test_generate_proof_multiple_keys() {
        let mut storage = Storage::new();
        storage.set(&U256::from(1u64), U256::from(10u64));
        storage.set(&U256::from(2u64), U256::from(20u64));
        storage.set(&U256::from(3u64), U256::from(30u64));

        let proof1 = storage.generate_proof(&U256::from(1u64)).unwrap();
        let proof2 = storage.generate_proof(&U256::from(2u64)).unwrap();
        let proof3 = storage.generate_proof(&U256::from(3u64)).unwrap();

        assert!(!proof1.is_empty());
        assert!(!proof2.is_empty());
        assert!(!proof3.is_empty());
    }

    // =========================================================================
    // Clear Tests
    // =========================================================================

    #[test]
    fn test_clear_empty_storage() {
        let mut storage = Storage::new();
        storage.clear();
        assert!(storage.is_empty());
        assert_eq!(storage.compute_root(), EMPTY_TRIE_ROOT);
    }

    #[test]
    fn test_clear_non_empty_storage() {
        let mut storage = Storage::new();
        storage.set(&U256::from(1u64), U256::from(10u64));
        storage.set(&U256::from(2u64), U256::from(20u64));
        assert!(!storage.is_empty());

        storage.clear();
        assert!(storage.is_empty());
        assert_eq!(storage.compute_root(), EMPTY_TRIE_ROOT);
        assert_eq!(storage.get(&U256::from(1u64)), U256::ZERO);
        assert_eq!(storage.get(&U256::from(2u64)), U256::ZERO);
    }

    #[test]
    fn test_reuse_after_clear() {
        let mut storage = Storage::new();
        storage.set(&U256::from(1u64), U256::from(10u64));
        storage.clear();

        storage.set(&U256::from(2u64), U256::from(20u64));
        assert_eq!(storage.get(&U256::from(1u64)), U256::ZERO);
        assert_eq!(storage.get(&U256::from(2u64)), U256::from(20u64));
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_large_key_values() {
        let mut storage = Storage::new();
        let key = U256::MAX;
        let value = U256::MAX;

        storage.set(&key, value);
        assert_eq!(storage.get(&key), value);
    }

    #[test]
    fn test_zero_key() {
        let mut storage = Storage::new();
        let key = U256::ZERO;
        let value = U256::from(42u64);

        storage.set(&key, value);
        assert_eq!(storage.get(&key), value);
    }

    #[test]
    fn test_sequential_keys() {
        let mut storage = Storage::new();

        for i in 0..50u64 {
            storage.set(&U256::from(i), U256::from(i * 10));
        }

        for i in 0..50u64 {
            assert_eq!(storage.get(&U256::from(i)), U256::from(i * 10));
        }
    }

    #[test]
    fn test_sparse_keys() {
        let mut storage = Storage::new();

        storage.set(&U256::from(1u64), U256::from(10u64));
        storage.set(&U256::from(1000u64), U256::from(20u64));
        storage.set(&U256::from(1_000_000u64), U256::from(30u64));

        assert_eq!(storage.get(&U256::from(1u64)), U256::from(10u64));
        assert_eq!(storage.get(&U256::from(1000u64)), U256::from(20u64));
        assert_eq!(storage.get(&U256::from(1_000_000u64)), U256::from(30u64));
        assert_eq!(storage.get(&U256::from(500u64)), U256::ZERO);
    }

    #[test]
    fn test_clone_storage() {
        let mut storage = Storage::new();
        storage.set(&U256::from(1u64), U256::from(100u64));
        storage.set(&U256::from(2u64), U256::from(200u64));

        let cloned = storage.clone();
        assert_eq!(storage.compute_root(), cloned.compute_root());
        assert_eq!(cloned.get(&U256::from(1u64)), U256::from(100u64));
        assert_eq!(cloned.get(&U256::from(2u64)), U256::from(200u64));
    }

    #[test]
    fn test_clone_independence() {
        let mut storage1 = Storage::new();
        storage1.set(&U256::from(1u64), U256::from(100u64));

        let mut storage2 = storage1.clone();
        storage2.set(&U256::from(2u64), U256::from(200u64));

        assert_eq!(storage1.get(&U256::from(2u64)), U256::ZERO);
        assert_eq!(storage2.get(&U256::from(2u64)), U256::from(200u64));
        assert_ne!(storage1.compute_root(), storage2.compute_root());
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_storage_lifecycle() {
        let mut storage = Storage::new();
        assert_eq!(storage.compute_root(), EMPTY_TRIE_ROOT);

        // Add values
        storage.set(&U256::from(1u64), U256::from(10u64));
        let root1 = storage.compute_root();
        assert_ne!(root1, Hash::ZERO);

        storage.set(&U256::from(2u64), U256::from(20u64));
        let root2 = storage.compute_root();
        assert_ne!(root2, root1);

        // Update value
        storage.set(&U256::from(1u64), U256::from(15u64));
        let root3 = storage.compute_root();
        assert_ne!(root3, root2);

        // Delete value
        storage.set(&U256::from(1u64), U256::ZERO);
        let root4 = storage.compute_root();
        assert_ne!(root4, root3);

        // Verify final state
        assert_eq!(storage.get(&U256::from(1u64)), U256::ZERO);
        assert_eq!(storage.get(&U256::from(2u64)), U256::from(20u64));
    }

    #[test]
    fn test_many_operations() {
        let mut storage = Storage::new();

        // Insert many values
        for i in 0..100u64 {
            storage.set(&U256::from(i), U256::from(i * 2));
        }

        // Update some values
        for i in 0..50u64 {
            storage.set(&U256::from(i), U256::from(i * 3));
        }

        // Delete some values
        for i in 25..75u64 {
            storage.set(&U256::from(i), U256::ZERO);
        }

        // Verify final state
        for i in 0..25u64 {
            assert_eq!(storage.get(&U256::from(i)), U256::from(i * 3));
        }
        for i in 25..75u64 {
            assert_eq!(storage.get(&U256::from(i)), U256::ZERO);
        }
        for i in 75..100u64 {
            assert_eq!(storage.get(&U256::from(i)), U256::from(i * 2));
        }
    }
}
