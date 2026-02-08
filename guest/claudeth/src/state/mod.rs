//! State management and Merkle Patricia Trie

pub mod account;
pub mod partial_mpt;
pub mod storage;

pub use account::{Account, EMPTY_CODE_HASH};
pub use partial_mpt::{
    Node, NodeError, Trie,
    bytes_to_nibbles, nibbles_to_bytes, common_prefix_length,
    encode_compact_path, decode_compact_path,
    proof::{Proof, ProofError, verify_proof},
};
pub use storage::Storage;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::types::{Hash, U256, Address};

    // =========================================================================
    // Account State Integration Tests
    // =========================================================================

    #[test]
    fn test_account_state_trie_integration() {
        let mut state_trie = Trie::new();

        // Create accounts
        let account1 = Account::new_eoa(U256::from(5u64), U256::from(1000u64));
        let account2 = Account::new_contract(
            U256::from(1u64),
            U256::from(500u64),
            Hash::from([0x11; 32]),
            Hash::from([0x22; 32]),
        );

        // Insert accounts into state trie (using address as key)
        let addr1 = Address::from([0x01; 20]);
        let addr2 = Address::from([0x02; 20]);

        state_trie.insert(addr1.as_bytes(), account1.encode_rlp());
        state_trie.insert(addr2.as_bytes(), account2.encode_rlp());

        // Compute state root
        let state_root = state_trie.compute_root();
        assert_ne!(state_root, Hash::ZERO);

        // Retrieve accounts
        let retrieved1_rlp = state_trie.get(addr1.as_bytes()).unwrap();
        let retrieved1 = Account::decode_rlp(&retrieved1_rlp).unwrap();
        assert_eq!(retrieved1, account1);

        let retrieved2_rlp = state_trie.get(addr2.as_bytes()).unwrap();
        let retrieved2 = Account::decode_rlp(&retrieved2_rlp).unwrap();
        assert_eq!(retrieved2, account2);
    }

    #[test]
    fn test_account_state_proof_generation() {
        let mut state_trie = Trie::new();

        // Create and insert account
        let account = Account::new_eoa(U256::from(10u64), U256::from(5000u64));
        let addr = Address::from([0xAA; 20]);
        state_trie.insert(addr.as_bytes(), account.encode_rlp());

        let state_root = state_trie.compute_root();

        // Generate proof for account
        let proof = state_trie.generate_proof(addr.as_bytes()).unwrap();
        assert!(!proof.is_empty());

        // Verify proof
        let account_rlp = account.encode_rlp();
        assert!(verify_proof(state_root, addr.as_bytes(), Some(&account_rlp), &proof));
    }

    #[test]
    fn test_account_state_update() {
        let mut state_trie = Trie::new();
        let addr = Address::from([0xBB; 20]);

        // Insert initial account
        let account1 = Account::new_eoa(U256::from(1u64), U256::from(100u64));
        state_trie.insert(addr.as_bytes(), account1.encode_rlp());
        let root1 = state_trie.compute_root();

        // Update account (increase nonce and balance)
        let account2 = Account::new_eoa(U256::from(2u64), U256::from(200u64));
        state_trie.insert(addr.as_bytes(), account2.encode_rlp());
        let root2 = state_trie.compute_root();

        assert_ne!(root1, root2);

        // Verify final state
        let retrieved_rlp = state_trie.get(addr.as_bytes()).unwrap();
        let retrieved = Account::decode_rlp(&retrieved_rlp).unwrap();
        assert_eq!(retrieved, account2);
    }

    // =========================================================================
    // Storage Integration Tests
    // =========================================================================

    #[test]
    fn test_storage_trie_integration() {
        let mut storage = Storage::new();

        // Set multiple storage slots
        storage.set(&U256::from(0u64), U256::from(42u64));
        storage.set(&U256::from(1u64), U256::from(99u64));
        storage.set(&U256::from(2u64), U256::from(1000u64));

        let storage_root = storage.compute_root();
        assert_ne!(storage_root, Hash::ZERO);

        // Verify values
        assert_eq!(storage.get(&U256::from(0u64)), U256::from(42u64));
        assert_eq!(storage.get(&U256::from(1u64)), U256::from(99u64));
        assert_eq!(storage.get(&U256::from(2u64)), U256::from(1000u64));
    }

    #[test]
    fn test_storage_proof_generation() {
        let mut storage = Storage::new();

        // Set storage value
        let key = U256::from(5u64);
        let value = U256::from(12345u64);
        storage.set(&key, value);

        let _storage_root = storage.compute_root();

        // Generate proof
        let proof = storage.generate_proof(&key).unwrap();
        assert!(!proof.is_empty());

        // Note: Full verification would require decoding the RLP-encoded value
        // This demonstrates proof generation works with the storage trie
    }

    // =========================================================================
    // Complete State Transition Tests
    // =========================================================================

    #[test]
    fn test_complete_contract_state() {
        // Create a contract account with storage
        let mut storage = Storage::new();
        storage.set(&U256::from(0u64), U256::from(100u64));
        storage.set(&U256::from(1u64), U256::from(200u64));

        let storage_root = storage.compute_root();
        let code_hash = Hash::from([0x33; 32]);

        let contract = Account::new_contract(
            U256::from(1u64),
            U256::from(10000u64),
            storage_root,
            code_hash,
        );

        // Insert contract into state trie
        let mut state_trie = Trie::new();
        let addr = Address::from([0xCC; 20]);
        state_trie.insert(addr.as_bytes(), contract.encode_rlp());

        let state_root = state_trie.compute_root();
        assert_ne!(state_root, Hash::ZERO);

        // Verify contract retrieval
        let retrieved_rlp = state_trie.get(addr.as_bytes()).unwrap();
        let retrieved = Account::decode_rlp(&retrieved_rlp).unwrap();
        assert_eq!(retrieved.storage_root, storage_root);
        assert_eq!(retrieved.code_hash, code_hash);
    }

    #[test]
    fn test_multiple_accounts_with_storage() {
        let mut state_trie = Trie::new();

        // Account 1: EOA
        let eoa = Account::new_eoa(U256::from(5u64), U256::from(1_000_000u64));
        let eoa_addr = Address::from([0x01; 20]);
        state_trie.insert(eoa_addr.as_bytes(), eoa.encode_rlp());

        // Account 2: Contract with storage
        let mut storage1 = Storage::new();
        storage1.set(&U256::from(0u64), U256::from(42u64));
        let contract1 = Account::new_contract(
            U256::from(1u64),
            U256::from(500_000u64),
            storage1.compute_root(),
            Hash::from([0x11; 32]),
        );
        let contract1_addr = Address::from([0x02; 20]);
        state_trie.insert(contract1_addr.as_bytes(), contract1.encode_rlp());

        // Account 3: Another contract with different storage
        let mut storage2 = Storage::new();
        storage2.set(&U256::from(0u64), U256::from(99u64));
        storage2.set(&U256::from(1u64), U256::from(88u64));
        let contract2 = Account::new_contract(
            U256::from(2u64),
            U256::from(750_000u64),
            storage2.compute_root(),
            Hash::from([0x22; 32]),
        );
        let contract2_addr = Address::from([0x03; 20]);
        state_trie.insert(contract2_addr.as_bytes(), contract2.encode_rlp());

        // Compute final state root
        let state_root = state_trie.compute_root();
        assert_ne!(state_root, Hash::ZERO);

        // Verify all accounts
        let eoa_rlp = state_trie.get(eoa_addr.as_bytes()).unwrap();
        let retrieved_eoa = Account::decode_rlp(&eoa_rlp).unwrap();
        assert_eq!(retrieved_eoa.nonce, U256::from(5u64));
        assert!(retrieved_eoa.is_eoa());

        let contract1_rlp = state_trie.get(contract1_addr.as_bytes()).unwrap();
        let retrieved_contract1 = Account::decode_rlp(&contract1_rlp).unwrap();
        assert!(retrieved_contract1.is_contract());
        assert_eq!(retrieved_contract1.storage_root, storage1.compute_root());

        let contract2_rlp = state_trie.get(contract2_addr.as_bytes()).unwrap();
        let retrieved_contract2 = Account::decode_rlp(&contract2_rlp).unwrap();
        assert!(retrieved_contract2.is_contract());
        assert_eq!(retrieved_contract2.storage_root, storage2.compute_root());
    }

    #[test]
    fn test_state_transition_with_storage_update() {
        let addr = Address::from([0xDD; 20]);

        // Initial state
        let mut storage = Storage::new();
        storage.set(&U256::from(0u64), U256::from(10u64));
        let initial_storage_root = storage.compute_root();

        let mut state_trie = Trie::new();
        let account = Account::new_contract(
            U256::from(1u64),
            U256::from(1000u64),
            initial_storage_root,
            Hash::from([0x44; 32]),
        );
        state_trie.insert(addr.as_bytes(), account.encode_rlp());
        let state_root1 = state_trie.compute_root();

        // Update storage
        storage.set(&U256::from(0u64), U256::from(20u64));
        storage.set(&U256::from(1u64), U256::from(30u64));
        let updated_storage_root = storage.compute_root();
        assert_ne!(initial_storage_root, updated_storage_root);

        // Update account with new storage root
        let updated_account = Account::new_contract(
            U256::from(2u64),
            U256::from(1000u64),
            updated_storage_root,
            Hash::from([0x44; 32]),
        );
        state_trie.insert(addr.as_bytes(), updated_account.encode_rlp());
        let state_root2 = state_trie.compute_root();

        assert_ne!(state_root1, state_root2);
    }

    #[test]
    fn test_account_deletion_workflow() {
        let mut state_trie = Trie::new();
        let addr1 = Address::from([0x01; 20]);
        let addr2 = Address::from([0x02; 20]);

        // Insert two accounts
        let account1 = Account::new_eoa(U256::from(1u64), U256::from(100u64));
        let account2 = Account::new_eoa(U256::from(2u64), U256::from(200u64));
        state_trie.insert(addr1.as_bytes(), account1.encode_rlp());
        state_trie.insert(addr2.as_bytes(), account2.encode_rlp());

        let root_before = state_trie.compute_root();

        // Delete account1
        state_trie.delete(addr1.as_bytes());
        let root_after = state_trie.compute_root();

        assert_ne!(root_before, root_after);
        assert!(state_trie.get(addr1.as_bytes()).is_none());
        assert!(state_trie.get(addr2.as_bytes()).is_some());
    }

    #[test]
    fn test_proof_verification_after_state_change() {
        let mut state_trie = Trie::new();
        let addr1 = Address::from([0xEE; 20]);
        let addr2 = Address::from([0xFF; 20]);

        // Insert accounts
        let account1 = Account::new_eoa(U256::from(1u64), U256::from(1000u64));
        let account2 = Account::new_eoa(U256::from(2u64), U256::from(2000u64));
        state_trie.insert(addr1.as_bytes(), account1.encode_rlp());
        state_trie.insert(addr2.as_bytes(), account2.encode_rlp());

        let state_root = state_trie.compute_root();

        // Generate proofs
        let proof1 = state_trie.generate_proof(addr1.as_bytes()).unwrap();
        let proof2 = state_trie.generate_proof(addr2.as_bytes()).unwrap();

        // Verify proofs
        assert!(verify_proof(
            state_root,
            addr1.as_bytes(),
            Some(&account1.encode_rlp()),
            &proof1
        ));
        assert!(verify_proof(
            state_root,
            addr2.as_bytes(),
            Some(&account2.encode_rlp()),
            &proof2
        ));
    }

    #[test]
    fn test_storage_clear_updates_account() {
        let addr = Address::from([0x11; 20]);

        // Create contract with storage
        let mut storage = Storage::new();
        storage.set(&U256::from(0u64), U256::from(42u64));
        let storage_root1 = storage.compute_root();

        let mut state_trie = Trie::new();
        let account1 = Account::new_contract(
            U256::from(1u64),
            U256::from(1000u64),
            storage_root1,
            Hash::from([0x55; 32]),
        );
        state_trie.insert(addr.as_bytes(), account1.encode_rlp());

        // Clear storage
        storage.clear();
        assert_eq!(storage.compute_root(), Hash::ZERO);

        // Update account with empty storage root
        let account2 = Account::new_contract(
            U256::from(1u64),
            U256::from(1000u64),
            Hash::ZERO,
            Hash::from([0x55; 32]),
        );
        state_trie.insert(addr.as_bytes(), account2.encode_rlp());

        // Verify
        let retrieved_rlp = state_trie.get(addr.as_bytes()).unwrap();
        let retrieved = Account::decode_rlp(&retrieved_rlp).unwrap();
        assert_eq!(retrieved.storage_root, Hash::ZERO);
    }

    #[test]
    fn test_eoa_to_contract_conversion() {
        let addr = Address::from([0x22; 20]);
        let mut state_trie = Trie::new();

        // Start as EOA
        let eoa = Account::new_eoa(U256::from(5u64), U256::from(10000u64));
        state_trie.insert(addr.as_bytes(), eoa.encode_rlp());
        let root1 = state_trie.compute_root();

        // Convert to contract (deploy code)
        let mut storage = Storage::new();
        storage.set(&U256::from(0u64), U256::from(123u64));

        let contract = Account::new_contract(
            U256::from(6u64), // Nonce incremented
            U256::from(10000u64),
            storage.compute_root(),
            Hash::from([0x66; 32]),
        );
        state_trie.insert(addr.as_bytes(), contract.encode_rlp());
        let root2 = state_trie.compute_root();

        assert_ne!(root1, root2);

        // Verify conversion
        let retrieved_rlp = state_trie.get(addr.as_bytes()).unwrap();
        let retrieved = Account::decode_rlp(&retrieved_rlp).unwrap();
        assert!(retrieved.is_contract());
        assert_eq!(retrieved.nonce, U256::from(6u64));
    }

    #[test]
    fn test_large_state_with_many_accounts() {
        let mut state_trie = Trie::new();

        // Create 50 accounts
        for i in 0..50u8 {
            let mut addr_bytes = [0u8; 20];
            addr_bytes[19] = i;
            let addr = Address::from(addr_bytes);

            let account = if i.is_multiple_of(2) {
                // EOA
                Account::new_eoa(U256::from(i as u64), U256::from(i as u64 * 1000))
            } else {
                // Contract with storage
                let mut storage = Storage::new();
                storage.set(&U256::from(0u64), U256::from(i as u64));
                Account::new_contract(
                    U256::from(i as u64),
                    U256::from(i as u64 * 1000),
                    storage.compute_root(),
                    Hash::from([i; 32]),
                )
            };

            state_trie.insert(addr.as_bytes(), account.encode_rlp());
        }

        let state_root = state_trie.compute_root();
        assert_ne!(state_root, Hash::ZERO);

        // Verify a few accounts
        for i in [0u8, 10, 25, 49] {
            let mut addr_bytes = [0u8; 20];
            addr_bytes[19] = i;
            let addr = Address::from(addr_bytes);

            let account_rlp = state_trie.get(addr.as_bytes()).unwrap();
            let account = Account::decode_rlp(&account_rlp).unwrap();
            assert_eq!(account.nonce, U256::from(i as u64));
        }
    }

    #[test]
    fn test_storage_slot_deletion() {
        let mut storage = Storage::new();

        // Set values
        storage.set(&U256::from(0u64), U256::from(10u64));
        storage.set(&U256::from(1u64), U256::from(20u64));
        storage.set(&U256::from(2u64), U256::from(30u64));
        let root1 = storage.compute_root();

        // Delete middle slot
        let old_value = storage.set(&U256::from(1u64), U256::ZERO);
        assert_eq!(old_value, U256::from(20u64));
        let root2 = storage.compute_root();

        assert_ne!(root1, root2);
        assert_eq!(storage.get(&U256::from(0u64)), U256::from(10u64));
        assert_eq!(storage.get(&U256::from(1u64)), U256::ZERO);
        assert_eq!(storage.get(&U256::from(2u64)), U256::from(30u64));
    }

    #[test]
    fn test_account_nonce_increment() {
        let addr = Address::from([0x33; 20]);
        let mut state_trie = Trie::new();

        // Initial account with nonce 0
        let mut account = Account::new_eoa(U256::ZERO, U256::from(5000u64));
        state_trie.insert(addr.as_bytes(), account.encode_rlp());

        // Simulate 10 transactions
        for i in 1..=10u64 {
            account.nonce = U256::from(i);
            state_trie.insert(addr.as_bytes(), account.encode_rlp());
        }

        // Verify final state
        let retrieved_rlp = state_trie.get(addr.as_bytes()).unwrap();
        let retrieved = Account::decode_rlp(&retrieved_rlp).unwrap();
        assert_eq!(retrieved.nonce, U256::from(10u64));
    }

    #[test]
    fn test_balance_transfer() {
        let mut state_trie = Trie::new();
        let addr1 = Address::from([0x44; 20]);
        let addr2 = Address::from([0x55; 20]);

        // Initial balances
        let mut account1 = Account::new_eoa(U256::from(1u64), U256::from(1000u64));
        let mut account2 = Account::new_eoa(U256::from(0u64), U256::from(0u64));
        state_trie.insert(addr1.as_bytes(), account1.encode_rlp());
        state_trie.insert(addr2.as_bytes(), account2.encode_rlp());

        // Transfer 300 wei from account1 to account2
        account1.balance = U256::from(700u64);
        account1.nonce = U256::from(2u64); // Increment nonce
        account2.balance = U256::from(300u64);

        state_trie.insert(addr1.as_bytes(), account1.encode_rlp());
        state_trie.insert(addr2.as_bytes(), account2.encode_rlp());

        // Verify final state
        let retrieved1_rlp = state_trie.get(addr1.as_bytes()).unwrap();
        let retrieved1 = Account::decode_rlp(&retrieved1_rlp).unwrap();
        assert_eq!(retrieved1.balance, U256::from(700u64));
        assert_eq!(retrieved1.nonce, U256::from(2u64));

        let retrieved2_rlp = state_trie.get(addr2.as_bytes()).unwrap();
        let retrieved2 = Account::decode_rlp(&retrieved2_rlp).unwrap();
        assert_eq!(retrieved2.balance, U256::from(300u64));
    }
}
