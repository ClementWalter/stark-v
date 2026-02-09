//! Execution state interface and implementations
//!
//! This module provides the State trait for EVM execution and implementations
//! including InMemoryState for testing and simulation.

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::collections::{HashMap, HashSet};
#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::collections::{BTreeMap as HashMap, BTreeSet as HashSet};
#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

use crate::crypto::keccak256;
use crate::state::{Account, EMPTY_CODE_HASH, EMPTY_TRIE_ROOT, Storage, Trie};
use crate::types::{Address, Hash, U256};

/// EVM execution state interface
///
/// Provides methods to read and write account state, storage, and transient storage
/// during EVM execution.
pub trait State {
    /// Gets the balance of an account
    fn get_balance(&self, address: &Address) -> U256;

    /// Sets the balance of an account
    fn set_balance(&mut self, address: &Address, balance: U256);

    /// Gets the nonce of an account
    fn get_nonce(&self, address: &Address) -> U256;

    /// Sets the nonce of an account
    fn set_nonce(&mut self, address: &Address, nonce: U256);

    /// Increments the nonce of an account by 1
    fn increment_nonce(&mut self, address: &Address);

    /// Gets the code of an account (empty slice for EOAs)
    fn get_code(&self, address: &Address) -> &[u8];

    /// Sets the code of an account
    fn set_code(&mut self, address: &Address, code: Vec<u8>);

    /// Gets the code hash of an account
    fn get_code_hash(&self, address: &Address) -> Hash;

    /// Loads a value from permanent storage (SLOAD)
    fn sload(&self, address: &Address, key: &U256) -> U256;

    /// Stores a value to permanent storage (SSTORE)
    fn sstore(&mut self, address: &Address, key: &U256, value: U256);

    /// Loads a value from transient storage (TLOAD - EIP-1153)
    fn tload(&self, address: &Address, key: &U256) -> U256;

    /// Stores a value to transient storage (TSTORE - EIP-1153)
    fn tstore(&mut self, address: &Address, key: &U256, value: U256);

    /// Returns true if the account exists (has code, nonce > 0, or balance > 0)
    fn account_exists(&self, address: &Address) -> bool;

    /// Returns true if the account is empty (no code, nonce = 0, balance = 0)
    fn is_empty(&self, address: &Address) -> bool;

    /// Marks an account for self-destruction (SELFDESTRUCT)
    fn selfdestruct(&mut self, address: &Address, beneficiary: &Address);

    /// Returns the list of self-destructed accounts and their beneficiaries
    fn get_selfdestructs(&self) -> &[(Address, Address)];

    /// Clears transient storage (called at transaction end)
    fn clear_transient_storage(&mut self);

    /// Clears self-destruct list (called at transaction end)
    fn clear_selfdestructs(&mut self);

    /// Marks an account as created during the current transaction
    fn mark_created(&mut self, address: &Address);

    /// Returns true if the account was created during the current transaction
    fn was_created(&self, address: &Address) -> bool;

    /// Clears the list of accounts created during the current transaction
    fn clear_created_accounts(&mut self);

    /// Clears an account from the state (code, storage, and account data)
    fn clear_account(&mut self, address: &Address);

    /// Marks an account as touched (accessed) during execution
    fn touch_account(&mut self, address: &Address);

    /// Deletes empty touched accounts (EIP-161)
    fn delete_empty_touched_accounts(&mut self);

    /// Clears the list of touched accounts
    fn clear_touched_accounts(&mut self);

    /// Computes the current state root
    fn compute_state_root(&self) -> Hash;
}

/// In-memory implementation of State for testing and simulation
///
/// Uses HashMaps to store accounts, code, storage, and transient storage.
/// Provides lazy account creation (accounts are created on first access).
#[derive(Clone, Debug)]
pub struct InMemoryState {
    /// Account state (nonce, balance, storage_root, code_hash)
    accounts: HashMap<Address, Account>,
    /// Contract code storage
    code: HashMap<Address, Vec<u8>>,
    /// Permanent storage (account -> storage trie)
    storage: HashMap<Address, Storage>,
    /// Transient storage (EIP-1153) - cleared after transaction
    transient_storage: HashMap<(Address, U256), U256>,
    /// Self-destructed accounts and their beneficiaries
    selfdestructs: Vec<(Address, Address)>,
    /// Accounts created during the current transaction (EIP-6780 tracking)
    created_accounts: Vec<Address>,
    /// Accounts touched (accessed) during the current transaction (EIP-161)
    touched_accounts: HashSet<Address>,
    /// Controls whether mutating operations mark accounts as touched
    track_touched_accounts: bool,
}

impl InMemoryState {
    /// Creates a new empty in-memory state
    pub fn new() -> Self {
        InMemoryState {
            accounts: HashMap::new(),
            code: HashMap::new(),
            storage: HashMap::new(),
            transient_storage: HashMap::new(),
            selfdestructs: Vec::new(),
            created_accounts: Vec::new(),
            touched_accounts: HashSet::new(),
            track_touched_accounts: true,
        }
    }

    /// Ensures an account exists, creating an empty one if needed
    fn ensure_account(&mut self, address: &Address) {
        #[cfg(not(target_arch = "riscv32"))]
        self.accounts.entry(*address).or_insert_with(Account::empty);

        #[cfg(target_arch = "riscv32")]
        if !self.accounts.contains_key(address) {
            self.accounts.insert(*address, Account::empty());
        }
    }

    /// Gets a reference to an account, or returns a default empty account
    fn get_account(&self, address: &Address) -> Account {
        #[cfg(not(target_arch = "riscv32"))]
        return self
            .accounts
            .get(address)
            .cloned()
            .unwrap_or_else(Account::empty);

        #[cfg(target_arch = "riscv32")]
        return self
            .accounts
            .get(address)
            .cloned()
            .unwrap_or_else(Account::empty);
    }

    /// Gets the storage trie for an account, creating an empty one if needed
    fn get_storage_mut(&mut self, address: &Address) -> &mut Storage {
        #[cfg(not(target_arch = "riscv32"))]
        return self.storage.entry(*address).or_default();

        #[cfg(target_arch = "riscv32")]
        {
            if !self.storage.contains_key(address) {
                self.storage.insert(*address, Storage::new());
            }
            self.storage.get_mut(address).unwrap()
        }
    }

    /// Clears transient storage (called at transaction end)
    pub fn clear_transient_storage(&mut self) {
        self.transient_storage.clear();
    }

    /// Clears self-destruct list (called at transaction end)
    pub fn clear_selfdestructs(&mut self) {
        self.selfdestructs.clear();
    }

    /// Enables or disables touch tracking for mutating operations
    pub fn set_touch_tracking(&mut self, enabled: bool) {
        self.track_touched_accounts = enabled;
    }

    /// Returns all known account addresses in stable order
    pub fn account_addresses(&self) -> Vec<Address> {
        let mut addresses: Vec<Address> = self.accounts.keys().copied().collect();
        addresses.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        addresses
    }

    /// Computes the storage root for an account
    pub fn storage_root(&self, address: &Address) -> Hash {
        self.storage
            .get(address)
            .map(Storage::compute_root)
            .unwrap_or(EMPTY_TRIE_ROOT)
    }
}

impl Default for InMemoryState {
    fn default() -> Self {
        Self::new()
    }
}

impl State for InMemoryState {
    fn get_balance(&self, address: &Address) -> U256 {
        self.get_account(address).balance
    }

    fn set_balance(&mut self, address: &Address, balance: U256) {
        self.ensure_account(address);
        self.touch_account(address);
        #[cfg(not(target_arch = "riscv32"))]
        if let Some(account) = self.accounts.get_mut(address) {
            account.balance = balance;
        }

        #[cfg(target_arch = "riscv32")]
        if let Some(account) = self.accounts.get_mut(address) {
            account.balance = balance;
        }
    }

    fn get_nonce(&self, address: &Address) -> U256 {
        self.get_account(address).nonce
    }

    fn set_nonce(&mut self, address: &Address, nonce: U256) {
        self.ensure_account(address);
        self.touch_account(address);
        #[cfg(not(target_arch = "riscv32"))]
        if let Some(account) = self.accounts.get_mut(address) {
            account.nonce = nonce;
        }

        #[cfg(target_arch = "riscv32")]
        if let Some(account) = self.accounts.get_mut(address) {
            account.nonce = nonce;
        }
    }

    fn increment_nonce(&mut self, address: &Address) {
        self.ensure_account(address);
        self.touch_account(address);
        #[cfg(not(target_arch = "riscv32"))]
        if let Some(account) = self.accounts.get_mut(address) {
            account.nonce += U256::from(1u64);
        }

        #[cfg(target_arch = "riscv32")]
        if let Some(account) = self.accounts.get_mut(address) {
            account.nonce += U256::from(1u64);
        }
    }

    fn get_code(&self, address: &Address) -> &[u8] {
        #[cfg(not(target_arch = "riscv32"))]
        return self.code.get(address).map(|c| c.as_slice()).unwrap_or(&[]);

        #[cfg(target_arch = "riscv32")]
        return self.code.get(address).map(|c| c.as_slice()).unwrap_or(&[]);
    }

    fn set_code(&mut self, address: &Address, code: Vec<u8>) {
        self.ensure_account(address);
        self.touch_account(address);

        // Compute code hash
        let code_hash = if code.is_empty() {
            EMPTY_CODE_HASH
        } else {
            keccak256(&code)
        };

        // Update account code hash
        #[cfg(not(target_arch = "riscv32"))]
        if let Some(account) = self.accounts.get_mut(address) {
            account.code_hash = code_hash;
        }

        #[cfg(target_arch = "riscv32")]
        if let Some(account) = self.accounts.get_mut(address) {
            account.code_hash = code_hash;
        }

        // Store code
        if code.is_empty() {
            self.code.remove(address);
        } else {
            self.code.insert(*address, code);
        }
    }

    fn get_code_hash(&self, address: &Address) -> Hash {
        self.get_account(address).code_hash
    }

    fn sload(&self, address: &Address, key: &U256) -> U256 {
        #[cfg(not(target_arch = "riscv32"))]
        return self
            .storage
            .get(address)
            .map(|s| s.get(key))
            .unwrap_or(U256::ZERO);

        #[cfg(target_arch = "riscv32")]
        return self
            .storage
            .get(address)
            .map(|s| s.get(key))
            .unwrap_or(U256::ZERO);
    }

    fn sstore(&mut self, address: &Address, key: &U256, value: U256) {
        self.ensure_account(address);
        self.touch_account(address);

        let storage_root = {
            let storage = self.get_storage_mut(address);
            storage.set(key, value);
            storage.compute_root()
        };

        if let Some(account) = self.accounts.get_mut(address) {
            account.storage_root = storage_root;
        }

        // Note: We do NOT remove empty storage from the HashMap because:
        // 1. The storage trie still needs to be accessible for sload operations
        // 2. The account.storage_root is the source of truth for the root hash
        // 3. Removing it would cause sload to return 0 for all keys
    }

    fn tload(&self, address: &Address, key: &U256) -> U256 {
        #[cfg(not(target_arch = "riscv32"))]
        return *self
            .transient_storage
            .get(&(*address, *key))
            .unwrap_or(&U256::ZERO);

        #[cfg(target_arch = "riscv32")]
        return *self
            .transient_storage
            .get(&(*address, *key))
            .unwrap_or(&U256::ZERO);
    }

    fn tstore(&mut self, address: &Address, key: &U256, value: U256) {
        if value == U256::ZERO {
            self.transient_storage.remove(&(*address, *key));
        } else {
            self.transient_storage.insert((*address, *key), value);
        }
    }

    fn account_exists(&self, address: &Address) -> bool {
        let account = self.get_account(address);
        !account.is_empty()
    }

    fn is_empty(&self, address: &Address) -> bool {
        let account = self.get_account(address);
        account.is_empty()
    }

    fn selfdestruct(&mut self, address: &Address, beneficiary: &Address) {
        self.touch_account(address);
        self.touch_account(beneficiary);
        self.selfdestructs.push((*address, *beneficiary));
    }

    fn get_selfdestructs(&self) -> &[(Address, Address)] {
        &self.selfdestructs
    }

    fn clear_transient_storage(&mut self) {
        self.transient_storage.clear();
    }

    fn clear_selfdestructs(&mut self) {
        self.selfdestructs.clear();
    }

    fn mark_created(&mut self, address: &Address) {
        if !self.created_accounts.contains(address) {
            self.created_accounts.push(*address);
        }
    }

    fn was_created(&self, address: &Address) -> bool {
        self.created_accounts.contains(address)
    }

    fn clear_created_accounts(&mut self) {
        self.created_accounts.clear();
    }

    fn clear_account(&mut self, address: &Address) {
        self.accounts.remove(address);
        self.code.remove(address);
        self.storage.remove(address);
    }

    fn touch_account(&mut self, address: &Address) {
        if self.track_touched_accounts {
            self.touched_accounts.insert(*address);
        }
    }

    fn delete_empty_touched_accounts(&mut self) {
        // EIP-161: Delete accounts that were touched and are now empty
        let addresses_to_delete: Vec<Address> = self
            .touched_accounts
            .iter()
            .filter(|addr| self.is_empty(addr))
            .copied()
            .collect();

        for address in addresses_to_delete {
            self.clear_account(&address);
        }
    }

    fn clear_touched_accounts(&mut self) {
        self.touched_accounts.clear();
    }

    fn compute_state_root(&self) -> Hash {
        if self.accounts.is_empty() {
            return EMPTY_TRIE_ROOT;
        }

        let mut trie = Trie::new();
        let mut addresses: Vec<Address> = self.accounts.keys().copied().collect();
        addresses.sort();

        for address in addresses {
            let account = self
                .accounts
                .get(&address)
                .cloned()
                .unwrap_or_else(Account::empty);
            // Note: account.storage_root is already maintained by sstore()
            // We do NOT recompute it here to preserve the root even when storage HashMap entry is removed

            if account.is_empty() {
                continue;
            }

            // Ethereum state trie uses keccak256(address) as key, not raw address bytes
            let key = keccak256(address.as_bytes());
            trie.insert(key.as_bytes(), account.encode_rlp());
        }

        trie.compute_root()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keccak256;

    // =========================================================================
    // Balance Tests
    // =========================================================================

    #[test]
    fn test_get_balance_nonexistent_account() {
        let state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        assert_eq!(state.get_balance(&addr), U256::ZERO);
    }

    #[test]
    fn test_set_and_get_balance() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let balance = U256::from(1000u64);

        state.set_balance(&addr, balance);
        assert_eq!(state.get_balance(&addr), balance);
    }

    #[test]
    fn test_set_balance_multiple_times() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);

        state.set_balance(&addr, U256::from(100u64));
        assert_eq!(state.get_balance(&addr), U256::from(100u64));

        state.set_balance(&addr, U256::from(200u64));
        assert_eq!(state.get_balance(&addr), U256::from(200u64));
    }

    #[test]
    fn test_set_balance_multiple_accounts() {
        let mut state = InMemoryState::new();
        let addr1 = Address::from([0x01; 20]);
        let addr2 = Address::from([0x02; 20]);

        state.set_balance(&addr1, U256::from(100u64));
        state.set_balance(&addr2, U256::from(200u64));

        assert_eq!(state.get_balance(&addr1), U256::from(100u64));
        assert_eq!(state.get_balance(&addr2), U256::from(200u64));
    }

    #[test]
    fn test_set_balance_zero() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);

        state.set_balance(&addr, U256::from(100u64));
        state.set_balance(&addr, U256::ZERO);

        assert_eq!(state.get_balance(&addr), U256::ZERO);
    }

    // =========================================================================
    // Nonce Tests
    // =========================================================================

    #[test]
    fn test_get_nonce_nonexistent_account() {
        let state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        assert_eq!(state.get_nonce(&addr), U256::ZERO);
    }

    #[test]
    fn test_increment_nonce() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);

        assert_eq!(state.get_nonce(&addr), U256::ZERO);

        state.increment_nonce(&addr);
        assert_eq!(state.get_nonce(&addr), U256::from(1u64));

        state.increment_nonce(&addr);
        assert_eq!(state.get_nonce(&addr), U256::from(2u64));
    }

    #[test]
    fn test_increment_nonce_multiple_accounts() {
        let mut state = InMemoryState::new();
        let addr1 = Address::from([0x01; 20]);
        let addr2 = Address::from([0x02; 20]);

        state.increment_nonce(&addr1);
        state.increment_nonce(&addr1);
        state.increment_nonce(&addr2);

        assert_eq!(state.get_nonce(&addr1), U256::from(2u64));
        assert_eq!(state.get_nonce(&addr2), U256::from(1u64));
    }

    #[test]
    fn test_increment_nonce_many_times() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);

        for i in 1..=100u64 {
            state.increment_nonce(&addr);
            assert_eq!(state.get_nonce(&addr), U256::from(i));
        }
    }

    // =========================================================================
    // Code Tests
    // =========================================================================

    #[test]
    fn test_get_code_nonexistent_account() {
        let state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let empty: &[u8] = &[];
        assert_eq!(state.get_code(&addr), empty);
    }

    #[test]
    fn test_set_and_get_code() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let code = vec![0x60, 0x00, 0x60, 0x00, 0xf3]; // Simple RETURN

        state.set_code(&addr, code.clone());
        assert_eq!(state.get_code(&addr), code.as_slice());
    }

    #[test]
    fn test_set_code_updates_code_hash() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let code = vec![0x60, 0x00];

        let hash_before = state.get_code_hash(&addr);
        assert_eq!(hash_before, EMPTY_CODE_HASH);

        state.set_code(&addr, code);
        let hash_after = state.get_code_hash(&addr);
        let expected = keccak256(&[0x60, 0x00]);
        assert_eq!(hash_after, expected);
    }

    #[test]
    fn test_set_empty_code() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let empty: &[u8] = &[];

        state.set_code(&addr, vec![0x60, 0x00]);
        assert_ne!(state.get_code(&addr), empty);

        state.set_code(&addr, vec![]);
        assert_eq!(state.get_code(&addr), empty);
        assert_eq!(state.get_code_hash(&addr), EMPTY_CODE_HASH);
    }

    #[test]
    fn test_set_code_multiple_accounts() {
        let mut state = InMemoryState::new();
        let addr1 = Address::from([0x01; 20]);
        let addr2 = Address::from([0x02; 20]);

        let code1 = vec![0x60, 0x00];
        let code2 = vec![0x60, 0x01];

        state.set_code(&addr1, code1.clone());
        state.set_code(&addr2, code2.clone());

        assert_eq!(state.get_code(&addr1), code1.as_slice());
        assert_eq!(state.get_code(&addr2), code2.as_slice());
    }

    // =========================================================================
    // Permanent Storage Tests (SLOAD/SSTORE)
    // =========================================================================

    #[test]
    fn test_sload_nonexistent_account() {
        let state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let key = U256::from(0u64);
        assert_eq!(state.sload(&addr, &key), U256::ZERO);
    }

    #[test]
    fn test_sstore_and_sload() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let key = U256::from(0u64);
        let value = U256::from(42u64);

        state.sstore(&addr, &key, value);
        assert_eq!(state.sload(&addr, &key), value);
    }

    #[test]
    fn test_sstore_multiple_slots() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);

        state.sstore(&addr, &U256::from(0u64), U256::from(10u64));
        state.sstore(&addr, &U256::from(1u64), U256::from(20u64));
        state.sstore(&addr, &U256::from(2u64), U256::from(30u64));

        assert_eq!(state.sload(&addr, &U256::from(0u64)), U256::from(10u64));
        assert_eq!(state.sload(&addr, &U256::from(1u64)), U256::from(20u64));
        assert_eq!(state.sload(&addr, &U256::from(2u64)), U256::from(30u64));
    }

    #[test]
    fn test_sstore_overwrite_value() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let key = U256::from(0u64);

        state.sstore(&addr, &key, U256::from(10u64));
        state.sstore(&addr, &key, U256::from(20u64));

        assert_eq!(state.sload(&addr, &key), U256::from(20u64));
    }

    #[test]
    fn test_sstore_zero_deletes_slot() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let key = U256::from(0u64);

        state.sstore(&addr, &key, U256::from(42u64));
        assert_eq!(state.sload(&addr, &key), U256::from(42u64));

        state.sstore(&addr, &key, U256::ZERO);
        assert_eq!(state.sload(&addr, &key), U256::ZERO);
    }

    #[test]
    fn test_sstore_multiple_accounts() {
        let mut state = InMemoryState::new();
        let addr1 = Address::from([0x01; 20]);
        let addr2 = Address::from([0x02; 20]);
        let key = U256::from(0u64);

        state.sstore(&addr1, &key, U256::from(100u64));
        state.sstore(&addr2, &key, U256::from(200u64));

        assert_eq!(state.sload(&addr1, &key), U256::from(100u64));
        assert_eq!(state.sload(&addr2, &key), U256::from(200u64));
    }

    // =========================================================================
    // Transient Storage Tests (TLOAD/TSTORE - EIP-1153)
    // =========================================================================

    #[test]
    fn test_tload_nonexistent_slot() {
        let state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let key = U256::from(0u64);
        assert_eq!(state.tload(&addr, &key), U256::ZERO);
    }

    #[test]
    fn test_tstore_and_tload() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let key = U256::from(0u64);
        let value = U256::from(99u64);

        state.tstore(&addr, &key, value);
        assert_eq!(state.tload(&addr, &key), value);
    }

    #[test]
    fn test_tstore_multiple_slots() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);

        state.tstore(&addr, &U256::from(0u64), U256::from(10u64));
        state.tstore(&addr, &U256::from(1u64), U256::from(20u64));
        state.tstore(&addr, &U256::from(2u64), U256::from(30u64));

        assert_eq!(state.tload(&addr, &U256::from(0u64)), U256::from(10u64));
        assert_eq!(state.tload(&addr, &U256::from(1u64)), U256::from(20u64));
        assert_eq!(state.tload(&addr, &U256::from(2u64)), U256::from(30u64));
    }

    #[test]
    fn test_tstore_zero_deletes_slot() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let key = U256::from(0u64);

        state.tstore(&addr, &key, U256::from(42u64));
        assert_eq!(state.tload(&addr, &key), U256::from(42u64));

        state.tstore(&addr, &key, U256::ZERO);
        assert_eq!(state.tload(&addr, &key), U256::ZERO);
    }

    #[test]
    fn test_clear_transient_storage() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let key = U256::from(0u64);

        state.tstore(&addr, &key, U256::from(42u64));
        assert_eq!(state.tload(&addr, &key), U256::from(42u64));

        state.clear_transient_storage();
        assert_eq!(state.tload(&addr, &key), U256::ZERO);
    }

    #[test]
    fn test_transient_storage_independent_from_permanent() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let key = U256::from(0u64);

        state.sstore(&addr, &key, U256::from(100u64));
        state.tstore(&addr, &key, U256::from(200u64));

        assert_eq!(state.sload(&addr, &key), U256::from(100u64));
        assert_eq!(state.tload(&addr, &key), U256::from(200u64));

        state.clear_transient_storage();
        assert_eq!(state.sload(&addr, &key), U256::from(100u64));
        assert_eq!(state.tload(&addr, &key), U256::ZERO);
    }

    // =========================================================================
    // Account Existence Tests
    // =========================================================================

    #[test]
    fn test_account_exists_nonexistent() {
        let state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        assert!(!state.account_exists(&addr));
    }

    #[test]
    fn test_account_exists_with_balance() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);

        state.set_balance(&addr, U256::from(1u64));
        assert!(state.account_exists(&addr));
    }

    #[test]
    fn test_account_exists_with_nonce() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);

        state.increment_nonce(&addr);
        assert!(state.account_exists(&addr));
    }

    #[test]
    fn test_account_exists_with_code() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);

        state.set_code(&addr, vec![0x60, 0x00]);
        assert!(state.account_exists(&addr));
    }

    #[test]
    fn test_account_exists_with_storage() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);

        state.sstore(&addr, &U256::from(1u64), U256::from(2u64));
        assert!(state.account_exists(&addr));
        assert!(!state.is_empty(&addr));
    }

    #[test]
    fn test_is_empty_nonexistent_account() {
        let state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        assert!(state.is_empty(&addr));
    }

    #[test]
    fn test_is_empty_with_balance() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);

        state.set_balance(&addr, U256::from(1u64));
        assert!(!state.is_empty(&addr));
    }

    #[test]
    fn test_is_empty_with_nonce() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);

        state.increment_nonce(&addr);
        assert!(!state.is_empty(&addr));
    }

    #[test]
    fn test_is_empty_with_code() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);

        state.set_code(&addr, vec![0x60, 0x00]);
        assert!(!state.is_empty(&addr));
    }

    #[test]
    fn test_is_empty_after_reset() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);

        state.set_balance(&addr, U256::from(100u64));
        assert!(!state.is_empty(&addr));

        state.set_balance(&addr, U256::ZERO);
        assert!(state.is_empty(&addr));
    }

    // =========================================================================
    // Self-Destruct Tests
    // =========================================================================

    #[test]
    fn test_selfdestruct_empty_list() {
        let state = InMemoryState::new();
        assert_eq!(state.get_selfdestructs().len(), 0);
    }

    #[test]
    fn test_selfdestruct_single_account() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let beneficiary = Address::from([0x02; 20]);

        state.selfdestruct(&addr, &beneficiary);

        let selfdestructs = state.get_selfdestructs();
        assert_eq!(selfdestructs.len(), 1);
        assert_eq!(selfdestructs[0], (addr, beneficiary));
    }

    #[test]
    fn test_selfdestruct_multiple_accounts() {
        let mut state = InMemoryState::new();
        let addr1 = Address::from([0x01; 20]);
        let addr2 = Address::from([0x02; 20]);
        let beneficiary1 = Address::from([0x03; 20]);
        let beneficiary2 = Address::from([0x04; 20]);

        state.selfdestruct(&addr1, &beneficiary1);
        state.selfdestruct(&addr2, &beneficiary2);

        let selfdestructs = state.get_selfdestructs();
        assert_eq!(selfdestructs.len(), 2);
        assert_eq!(selfdestructs[0], (addr1, beneficiary1));
        assert_eq!(selfdestructs[1], (addr2, beneficiary2));
    }

    #[test]
    fn test_clear_selfdestructs() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let beneficiary = Address::from([0x02; 20]);

        state.selfdestruct(&addr, &beneficiary);
        assert_eq!(state.get_selfdestructs().len(), 1);

        state.clear_selfdestructs();
        assert_eq!(state.get_selfdestructs().len(), 0);
    }

    #[test]
    fn test_selfdestruct_same_beneficiary() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let beneficiary = Address::from([0x01; 20]); // Self-destruct to self

        state.selfdestruct(&addr, &beneficiary);

        let selfdestructs = state.get_selfdestructs();
        assert_eq!(selfdestructs.len(), 1);
        assert_eq!(selfdestructs[0], (addr, beneficiary));
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_complete_account_lifecycle() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);

        // Start with empty account
        assert!(state.is_empty(&addr));

        // Add balance
        state.set_balance(&addr, U256::from(1000u64));
        assert_eq!(state.get_balance(&addr), U256::from(1000u64));
        assert!(!state.is_empty(&addr));

        // Increment nonce
        state.increment_nonce(&addr);
        assert_eq!(state.get_nonce(&addr), U256::from(1u64));

        // Add code
        let code = vec![0x60, 0x00, 0x60, 0x00, 0xf3];
        state.set_code(&addr, code.clone());
        assert_eq!(state.get_code(&addr), code.as_slice());

        // Add storage
        state.sstore(&addr, &U256::from(0u64), U256::from(42u64));
        assert_eq!(state.sload(&addr, &U256::from(0u64)), U256::from(42u64));

        // Add transient storage
        state.tstore(&addr, &U256::from(1u64), U256::from(99u64));
        assert_eq!(state.tload(&addr, &U256::from(1u64)), U256::from(99u64));

        // Verify account exists
        assert!(state.account_exists(&addr));
    }

    #[test]
    fn test_multiple_accounts_independence() {
        let mut state = InMemoryState::new();
        let addr1 = Address::from([0x01; 20]);
        let addr2 = Address::from([0x02; 20]);

        // Set different balances
        state.set_balance(&addr1, U256::from(100u64));
        state.set_balance(&addr2, U256::from(200u64));

        // Set different nonces
        state.increment_nonce(&addr1);
        state.increment_nonce(&addr2);
        state.increment_nonce(&addr2);

        // Set different storage
        state.sstore(&addr1, &U256::from(0u64), U256::from(10u64));
        state.sstore(&addr2, &U256::from(0u64), U256::from(20u64));

        // Verify independence
        assert_eq!(state.get_balance(&addr1), U256::from(100u64));
        assert_eq!(state.get_balance(&addr2), U256::from(200u64));
        assert_eq!(state.get_nonce(&addr1), U256::from(1u64));
        assert_eq!(state.get_nonce(&addr2), U256::from(2u64));
        assert_eq!(state.sload(&addr1, &U256::from(0u64)), U256::from(10u64));
        assert_eq!(state.sload(&addr2, &U256::from(0u64)), U256::from(20u64));
    }

    #[test]
    fn test_transaction_boundaries() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);
        let beneficiary = Address::from([0x02; 20]);

        // Set transient storage and selfdestruct
        state.tstore(&addr, &U256::from(0u64), U256::from(42u64));
        state.selfdestruct(&addr, &beneficiary);

        assert_eq!(state.tload(&addr, &U256::from(0u64)), U256::from(42u64));
        assert_eq!(state.get_selfdestructs().len(), 1);

        // Clear transaction-scoped state
        state.clear_transient_storage();
        state.clear_selfdestructs();

        assert_eq!(state.tload(&addr, &U256::from(0u64)), U256::ZERO);
        assert_eq!(state.get_selfdestructs().len(), 0);
    }

    // =========================================================================
    // State Root Tests
    // =========================================================================

    #[test]
    fn test_compute_state_root_empty() {
        let state = InMemoryState::new();
        assert_eq!(state.compute_state_root(), EMPTY_TRIE_ROOT);
    }

    #[test]
    fn test_compute_state_root_with_account() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x11; 20]);

        state.set_balance(&addr, U256::from(100u64));
        state.increment_nonce(&addr);

        let computed = state.compute_state_root();

        let mut trie = Trie::new();
        let account = Account::new_eoa(U256::from(1u64), U256::from(100u64));
        // State trie uses keccak256(address) as key
        let key = keccak256(addr.as_bytes());
        trie.insert(key.as_bytes(), account.encode_rlp());

        assert_eq!(computed, trie.compute_root());
    }

    #[test]
    fn test_compute_state_root_with_storage() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x22; 20]);

        state.sstore(&addr, &U256::from(1u64), U256::from(2u64));
        let computed = state.compute_state_root();

        let mut storage = Storage::new();
        storage.set(&U256::from(1u64), U256::from(2u64));
        let account = Account::new(
            U256::ZERO,
            U256::ZERO,
            storage.compute_root(),
            EMPTY_CODE_HASH,
        );

        let mut trie = Trie::new();
        // State trie uses keccak256(address) as key
        let key = keccak256(addr.as_bytes());
        trie.insert(key.as_bytes(), account.encode_rlp());

        assert_eq!(computed, trie.compute_root());
    }

    #[test]
    fn test_clone_state() {
        let mut state = InMemoryState::new();
        let addr = Address::from([0x01; 20]);

        state.set_balance(&addr, U256::from(1000u64));
        state.increment_nonce(&addr);
        state.sstore(&addr, &U256::from(0u64), U256::from(42u64));

        let cloned = state.clone();

        assert_eq!(cloned.get_balance(&addr), U256::from(1000u64));
        assert_eq!(cloned.get_nonce(&addr), U256::from(1u64));
        assert_eq!(cloned.sload(&addr, &U256::from(0u64)), U256::from(42u64));
    }

    #[test]
    fn test_clone_independence() {
        let mut state1 = InMemoryState::new();
        let addr = Address::from([0x01; 20]);

        state1.set_balance(&addr, U256::from(1000u64));

        let mut state2 = state1.clone();
        state2.set_balance(&addr, U256::from(2000u64));

        assert_eq!(state1.get_balance(&addr), U256::from(1000u64));
        assert_eq!(state2.get_balance(&addr), U256::from(2000u64));
    }

    #[test]
    fn test_default_state() {
        let state = InMemoryState::default();
        let addr = Address::from([0x01; 20]);
        let empty: &[u8] = &[];

        assert_eq!(state.get_balance(&addr), U256::ZERO);
        assert_eq!(state.get_nonce(&addr), U256::ZERO);
        assert_eq!(state.get_code(&addr), empty);
        assert!(state.is_empty(&addr));
    }
}
