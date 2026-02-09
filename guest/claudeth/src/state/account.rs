//! Ethereum account state representation
//!
//! This module implements the Ethereum account structure with RLP encoding/decoding.
//! An account contains:
//! - nonce: number of transactions sent from this account
//! - balance: amount of Wei owned by this account
//! - storage_root: root hash of the account's storage trie
//! - code_hash: hash of the account's bytecode (or empty hash for EOAs)

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::{vec, vec::Vec};

use crate::crypto::rlp::{self, RlpError};
use crate::state::partial_mpt::EMPTY_TRIE_ROOT;
use crate::types::{Hash, U256};

/// Ethereum account state
///
/// An account is RLP-encoded as: `[nonce, balance, storage_root, code_hash]`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    /// Number of transactions sent from this account
    pub nonce: U256,
    /// Amount of Wei owned by this account
    pub balance: U256,
    /// Root hash of the account's storage trie
    pub storage_root: Hash,
    /// Hash of the account's EVM bytecode
    pub code_hash: Hash,
}

impl Account {
    /// Creates a new account
    pub fn new(nonce: U256, balance: U256, storage_root: Hash, code_hash: Hash) -> Self {
        Account {
            nonce,
            balance,
            storage_root,
            code_hash,
        }
    }

    /// Creates a new empty account (EOA with no code)
    ///
    /// Empty accounts have:
    /// - nonce: 0
    /// - balance: 0
    /// - storage_root: EMPTY_TRIE_ROOT (keccak256 of RLP empty bytes)
    /// - code_hash: Keccak256 of empty bytes
    pub fn empty() -> Self {
        Account {
            nonce: U256::ZERO,
            balance: U256::ZERO,
            storage_root: EMPTY_TRIE_ROOT,
            code_hash: EMPTY_CODE_HASH,
        }
    }

    /// Creates a new externally owned account (EOA)
    pub fn new_eoa(nonce: U256, balance: U256) -> Self {
        Account {
            nonce,
            balance,
            storage_root: EMPTY_TRIE_ROOT,
            code_hash: EMPTY_CODE_HASH,
        }
    }

    /// Creates a new contract account
    pub fn new_contract(nonce: U256, balance: U256, storage_root: Hash, code_hash: Hash) -> Self {
        Account {
            nonce,
            balance,
            storage_root,
            code_hash,
        }
    }

    /// Returns true if this is an EOA (no contract code)
    pub fn is_eoa(&self) -> bool {
        self.code_hash == EMPTY_CODE_HASH
    }

    /// Returns true if this is a contract account
    pub fn is_contract(&self) -> bool {
        !self.is_eoa()
    }

    /// Returns true if the account is empty (all fields are zero/empty)
    pub fn is_empty(&self) -> bool {
        self.nonce == U256::ZERO
            && self.balance == U256::ZERO
            && self.storage_root == EMPTY_TRIE_ROOT
            && self.code_hash == EMPTY_CODE_HASH
    }

    /// Encodes the account as RLP
    ///
    /// Format: `[nonce, balance, storage_root, code_hash]`
    pub fn encode_rlp(&self) -> Vec<u8> {
        let items = vec![
            rlp::encode_u256(&self.nonce),
            rlp::encode_u256(&self.balance),
            rlp::encode_hash(&self.storage_root),
            rlp::encode_hash(&self.code_hash),
        ];
        rlp::encode_list(&items)
    }

    /// Decodes an account from RLP
    pub fn decode_rlp(data: &[u8]) -> Result<Self, RlpError> {
        let (items, _rest) = rlp::decode_list(data)?;

        if items.len() != 4 {
            return Err(RlpError::InvalidEncoding);
        }

        let (nonce, _) = rlp::decode_u256(&items[0])?;
        let (balance, _) = rlp::decode_u256(&items[1])?;
        let (storage_root, _) = rlp::decode_hash(&items[2])?;
        let (code_hash, _) = rlp::decode_hash(&items[3])?;

        Ok(Account {
            nonce,
            balance,
            storage_root,
            code_hash,
        })
    }
}

impl Default for Account {
    fn default() -> Self {
        Self::empty()
    }
}

/// Keccak256 hash of empty bytes: keccak256([])
/// This is c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470
pub const EMPTY_CODE_HASH: Hash = Hash::new([
    0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c, 0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7, 0x03, 0xc0,
    0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b, 0x7b, 0xfa, 0xd8, 0x04, 0x5d, 0x85, 0xa4, 0x70,
]);

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Construction Tests
    // =========================================================================

    #[test]
    fn test_new_account() {
        let account = Account::new(
            U256::from(1u64),
            U256::from(100u64),
            Hash::from([0x42; 32]),
            Hash::from([0x99; 32]),
        );
        assert_eq!(account.nonce, U256::from(1u64));
        assert_eq!(account.balance, U256::from(100u64));
        assert_eq!(account.storage_root, Hash::from([0x42; 32]));
        assert_eq!(account.code_hash, Hash::from([0x99; 32]));
    }

    #[test]
    fn test_empty_account() {
        let account = Account::empty();
        assert_eq!(account.nonce, U256::ZERO);
        assert_eq!(account.balance, U256::ZERO);
        assert_eq!(account.storage_root, EMPTY_TRIE_ROOT);
        assert_eq!(account.code_hash, EMPTY_CODE_HASH);
        assert!(account.is_empty());
    }

    #[test]
    fn test_default_account() {
        let account = Account::default();
        assert_eq!(account, Account::empty());
    }

    #[test]
    fn test_new_eoa() {
        let account = Account::new_eoa(U256::from(5u64), U256::from(1000u64));
        assert_eq!(account.nonce, U256::from(5u64));
        assert_eq!(account.balance, U256::from(1000u64));
        assert_eq!(account.storage_root, EMPTY_TRIE_ROOT);
        assert_eq!(account.code_hash, EMPTY_CODE_HASH);
        assert!(account.is_eoa());
        assert!(!account.is_contract());
    }

    #[test]
    fn test_new_contract() {
        let storage_root = Hash::from([0x11; 32]);
        let code_hash = Hash::from([0x22; 32]);
        let account = Account::new_contract(
            U256::from(1u64),
            U256::from(500u64),
            storage_root,
            code_hash,
        );
        assert_eq!(account.nonce, U256::from(1u64));
        assert_eq!(account.balance, U256::from(500u64));
        assert_eq!(account.storage_root, storage_root);
        assert_eq!(account.code_hash, code_hash);
        assert!(!account.is_eoa());
        assert!(account.is_contract());
    }

    // =========================================================================
    // Property Tests
    // =========================================================================

    #[test]
    fn test_is_eoa() {
        let eoa = Account::new_eoa(U256::ZERO, U256::ZERO);
        assert!(eoa.is_eoa());

        let contract =
            Account::new_contract(U256::ZERO, U256::ZERO, Hash::ZERO, Hash::from([0x42; 32]));
        assert!(!contract.is_eoa());
    }

    #[test]
    fn test_is_contract() {
        let eoa = Account::new_eoa(U256::ZERO, U256::ZERO);
        assert!(!eoa.is_contract());

        let contract =
            Account::new_contract(U256::ZERO, U256::ZERO, Hash::ZERO, Hash::from([0x42; 32]));
        assert!(contract.is_contract());
    }

    #[test]
    fn test_is_empty() {
        let empty = Account::empty();
        assert!(empty.is_empty());

        let eoa_with_balance = Account::new_eoa(U256::ZERO, U256::from(1u64));
        assert!(!eoa_with_balance.is_empty());

        let eoa_with_nonce = Account::new_eoa(U256::from(1u64), U256::ZERO);
        assert!(!eoa_with_nonce.is_empty());

        let contract = Account::new_contract(
            U256::ZERO,
            U256::ZERO,
            Hash::from([0x01; 32]),
            EMPTY_CODE_HASH,
        );
        assert!(!contract.is_empty());
    }

    // =========================================================================
    // RLP Encoding Tests
    // =========================================================================

    #[test]
    fn test_encode_empty_account() {
        let account = Account::empty();
        let encoded = account.encode_rlp();
        assert!(!encoded.is_empty());
        assert!(encoded[0] >= 0xc0); // RLP list marker
    }

    #[test]
    fn test_encode_eoa() {
        let account = Account::new_eoa(U256::from(5u64), U256::from(1000u64));
        let encoded = account.encode_rlp();
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_encode_contract() {
        let account = Account::new_contract(
            U256::from(1u64),
            U256::from(500u64),
            Hash::from([0x11; 32]),
            Hash::from([0x22; 32]),
        );
        let encoded = account.encode_rlp();
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_encode_deterministic() {
        let account = Account::new_eoa(U256::from(42u64), U256::from(999u64));
        let encoded1 = account.encode_rlp();
        let encoded2 = account.encode_rlp();
        assert_eq!(encoded1, encoded2);
    }

    // =========================================================================
    // RLP Decoding Tests
    // =========================================================================

    #[test]
    fn test_decode_empty_account() {
        let original = Account::empty();
        let encoded = original.encode_rlp();
        let decoded = Account::decode_rlp(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_decode_eoa() {
        let original = Account::new_eoa(U256::from(5u64), U256::from(1000u64));
        let encoded = original.encode_rlp();
        let decoded = Account::decode_rlp(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_decode_contract() {
        let original = Account::new_contract(
            U256::from(1u64),
            U256::from(500u64),
            Hash::from([0x11; 32]),
            Hash::from([0x22; 32]),
        );
        let encoded = original.encode_rlp();
        let decoded = Account::decode_rlp(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_decode_invalid_field_count() {
        // RLP list with 3 items instead of 4
        let items = vec![
            rlp::encode_u256(&U256::ZERO),
            rlp::encode_u256(&U256::ZERO),
            rlp::encode_hash(&Hash::ZERO),
        ];
        let encoded = rlp::encode_list(&items);
        assert!(Account::decode_rlp(&encoded).is_err());
    }

    #[test]
    fn test_decode_invalid_rlp() {
        let invalid = vec![0xFF, 0xFF, 0xFF];
        assert!(Account::decode_rlp(&invalid).is_err());
    }

    // =========================================================================
    // RLP Roundtrip Tests
    // =========================================================================

    #[test]
    fn test_roundtrip_empty() {
        let account = Account::empty();
        let encoded = account.encode_rlp();
        let decoded = Account::decode_rlp(&encoded).unwrap();
        assert_eq!(decoded, account);
    }

    #[test]
    fn test_roundtrip_eoa() {
        let test_cases = vec![
            Account::new_eoa(U256::ZERO, U256::ZERO),
            Account::new_eoa(U256::from(1u64), U256::from(100u64)),
            Account::new_eoa(U256::from(999u64), U256::from(1_000_000u64)),
            Account::new_eoa(U256::MAX, U256::MAX),
        ];

        for account in test_cases {
            let encoded = account.encode_rlp();
            let decoded = Account::decode_rlp(&encoded).unwrap();
            assert_eq!(decoded, account);
        }
    }

    #[test]
    fn test_roundtrip_contract() {
        let test_cases = vec![
            Account::new_contract(U256::ZERO, U256::ZERO, Hash::ZERO, Hash::from([0x01; 32])),
            Account::new_contract(
                U256::from(1u64),
                U256::from(100u64),
                Hash::from([0x11; 32]),
                Hash::from([0x22; 32]),
            ),
            Account::new_contract(U256::MAX, U256::MAX, Hash::from([0xFF; 32]), Hash::ZERO),
        ];

        for account in test_cases {
            let encoded = account.encode_rlp();
            let decoded = Account::decode_rlp(&encoded).unwrap();
            assert_eq!(decoded, account);
        }
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_account_with_max_values() {
        let account = Account::new(U256::MAX, U256::MAX, Hash::from([0xFF; 32]), Hash::ZERO);
        let encoded = account.encode_rlp();
        let decoded = Account::decode_rlp(&encoded).unwrap();
        assert_eq!(decoded, account);
    }

    #[test]
    fn test_account_with_zero_values() {
        let account = Account::new(U256::ZERO, U256::ZERO, Hash::ZERO, Hash::ZERO);
        let encoded = account.encode_rlp();
        let decoded = Account::decode_rlp(&encoded).unwrap();
        assert_eq!(decoded, account);
    }

    #[test]
    fn test_clone_account() {
        let account = Account::new_eoa(U256::from(42u64), U256::from(999u64));
        let cloned = account.clone();
        assert_eq!(account, cloned);
    }

    #[test]
    fn test_empty_code_hash_constant() {
        // Verify the EMPTY_CODE_HASH constant is correct
        // This is keccak256([]) = c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470
        let expected = [
            0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c, 0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7,
            0x03, 0xc0, 0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b, 0x7b, 0xfa, 0xd8, 0x04,
            0x5d, 0x85, 0xa4, 0x70,
        ];
        assert_eq!(EMPTY_CODE_HASH, Hash::from(expected));
    }
}
