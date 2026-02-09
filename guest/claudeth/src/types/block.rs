//! Ethereum block header type
//!
//! This module provides the [`BlockHeader`] type for Ethereum blocks,
//! with support for all Fusaka fork fields including EIP-1559, EIP-4895,
//! EIP-4844, EIP-4788, and EIP-7685.

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::{vec, vec::Vec};

#[cfg(target_arch = "riscv32")]
use alloc::{vec, vec::Vec};

use core::fmt;
use core::hash::{Hash as StdHash, Hasher};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::crypto::keccak256;
use crate::crypto::rlp::{self, RlpError};
use crate::types::{Address, Bytes, Hash, U256};

/// Ethereum empty ommers hash: keccak256(rlp([]))
///
/// This is 0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347.
pub const EMPTY_OMMERS_HASH: Hash = Hash::new([
    0x1d, 0xcc, 0x4d, 0xe8, 0xde, 0xc7, 0x5d, 0x7a, 0xab, 0x85, 0xb5, 0x67, 0xb6, 0xcc, 0xd4, 0x1a,
    0xd3, 0x12, 0x45, 0x1b, 0x94, 0x8a, 0x74, 0x13, 0xf0, 0xa1, 0x42, 0xfd, 0x40, 0xd4, 0x93, 0x47,
]);

// Helper functions for serializing/deserializing [u8; 256]
fn serialize_logs_bloom<S>(bloom: &[u8; 256], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bytes(bloom)
}

fn deserialize_logs_bloom<'de, D>(deserializer: D) -> Result<[u8; 256], D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
    if bytes.len() != 256 {
        return Err(D::Error::custom("logs bloom must be exactly 256 bytes"));
    }
    let mut bloom = [0u8; 256];
    bloom.copy_from_slice(&bytes);
    Ok(bloom)
}

/// Ethereum block header supporting all Fusaka fork fields.
///
/// This structure contains all 20 fields required for a complete
/// Ethereum block header post-Fusaka fork, including support for:
/// - EIP-1559 (base_fee_per_gas)
/// - EIP-4895 (withdrawals_root)
/// - EIP-4844 (blob_gas_used, excess_blob_gas)
/// - EIP-4788 (parent_beacon_block_root)
/// - EIP-7685 (requests_hash)
///
/// # Examples
///
/// ```
/// use claudeth::types::{BlockHeader, Address, Hash, U256, Bytes, EMPTY_OMMERS_HASH};
///
/// let header = BlockHeader {
///     parent_hash: Hash::ZERO,
///     ommers_hash: EMPTY_OMMERS_HASH,
///     coinbase: Address::ZERO,
///     state_root: Hash::ZERO,
///     transactions_root: Hash::ZERO,
///     receipts_root: Hash::ZERO,
///     logs_bloom: [0u8; 256],
///     difficulty: U256::ZERO,
///     number: 0,
///     gas_limit: 30_000_000,
///     gas_used: 0,
///     timestamp: 0,
///     extra_data: Bytes::new(),
///     mix_hash: Hash::ZERO,
///     nonce: 0,
///     base_fee_per_gas: Some(1_000_000_000),
///     withdrawals_root: None,
///     blob_gas_used: None,
///     excess_blob_gas: None,
///     parent_beacon_block_root: None,
///     requests_hash: None,
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Hash of the parent block
    pub parent_hash: Hash,
    /// Hash of the ommers (uncles) list
    pub ommers_hash: Hash,
    /// Address of the miner/validator (coinbase)
    pub coinbase: Address,
    /// Root hash of the state trie
    pub state_root: Hash,
    /// Root hash of the transactions trie
    pub transactions_root: Hash,
    /// Root hash of the receipts trie
    pub receipts_root: Hash,
    /// Bloom filter for logs (256 bytes)
    #[serde(
        serialize_with = "serialize_logs_bloom",
        deserialize_with = "deserialize_logs_bloom"
    )]
    pub logs_bloom: [u8; 256],
    /// Difficulty (always 0 post-merge)
    pub difficulty: U256,
    /// Block number
    pub number: u64,
    /// Gas limit for this block
    pub gas_limit: u64,
    /// Gas used by transactions in this block
    pub gas_used: u64,
    /// Block timestamp (Unix epoch seconds)
    pub timestamp: u64,
    /// Extra data (max 32 bytes)
    pub extra_data: Bytes,
    /// Mix hash (always 0 post-merge)
    pub mix_hash: Hash,
    /// Nonce (always 0 post-merge)
    pub nonce: u64,
    /// Base fee per gas (EIP-1559, London fork)
    pub base_fee_per_gas: Option<u64>,
    /// Withdrawals root (EIP-4895, Shanghai fork)
    pub withdrawals_root: Option<Hash>,
    /// Blob gas used (EIP-4844, Cancun fork)
    pub blob_gas_used: Option<u64>,
    /// Excess blob gas (EIP-4844, Cancun fork)
    pub excess_blob_gas: Option<u64>,
    /// Parent beacon block root (EIP-4788, Cancun fork)
    pub parent_beacon_block_root: Option<Hash>,
    /// Requests hash (EIP-7685, Prague fork)
    pub requests_hash: Option<Hash>,
}

impl BlockHeader {
    /// Maximum allowed extra data size (32 bytes)
    pub const MAX_EXTRA_DATA_SIZE: usize = 32;
    /// Maximum gas limit change per block (1/1024 of parent gas limit)
    pub const GAS_LIMIT_BOUND_DIVISOR: u64 = 1024;
    /// Minimum gas limit per block
    pub const MIN_GAS_LIMIT: u64 = 5000;

    /// Validates gas fields (gas_used <= gas_limit).
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::BlockHeader;
    ///
    /// let mut header = BlockHeader::default();
    /// header.gas_limit = 30_000_000;
    /// header.gas_used = 15_000_000;
    /// assert!(header.validate_gas_fields().is_ok());
    ///
    /// header.gas_used = 35_000_000;
    /// assert!(header.validate_gas_fields().is_err());
    /// ```
    pub fn validate_gas_fields(&self) -> Result<(), ValidationError> {
        if self.gas_used > self.gas_limit {
            return Err(ValidationError::GasUsedExceedsLimit {
                gas_used: self.gas_used,
                gas_limit: self.gas_limit,
            });
        }
        Ok(())
    }

    /// Validates extra data size (must be <= 32 bytes).
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::{BlockHeader, Bytes};
    ///
    /// let mut header = BlockHeader::default();
    /// header.extra_data = Bytes::from_slice(&[0x42; 32]);
    /// assert!(header.validate_extra_data().is_ok());
    ///
    /// header.extra_data = Bytes::from_slice(&[0x42; 33]);
    /// assert!(header.validate_extra_data().is_err());
    /// ```
    pub fn validate_extra_data(&self) -> Result<(), ValidationError> {
        if self.extra_data.len() > Self::MAX_EXTRA_DATA_SIZE {
            return Err(ValidationError::ExtraDataTooLarge {
                size: self.extra_data.len(),
                max_size: Self::MAX_EXTRA_DATA_SIZE,
            });
        }
        Ok(())
    }

    /// Validates post-merge fields (difficulty, mix_hash, nonce should be zero).
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::{BlockHeader, U256, Hash};
    ///
    /// let mut header = BlockHeader::default();
    /// header.difficulty = U256::ZERO;
    /// header.mix_hash = Hash::ZERO;
    /// header.nonce = 0;
    /// assert!(header.validate_post_merge_fields().is_ok());
    ///
    /// header.difficulty = U256::from(1u64);
    /// assert!(header.validate_post_merge_fields().is_err());
    /// ```
    pub fn validate_post_merge_fields(&self) -> Result<(), ValidationError> {
        if !self.difficulty.is_zero() {
            return Err(ValidationError::NonZeroDifficulty);
        }
        if self.mix_hash != Hash::ZERO {
            return Err(ValidationError::NonZeroMixHash);
        }
        if self.nonce != 0 {
            return Err(ValidationError::NonZeroNonce);
        }
        Ok(())
    }

    /// Validates all fields.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::BlockHeader;
    ///
    /// let header = BlockHeader::default();
    /// assert!(header.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.validate_gas_fields()?;
        self.validate_extra_data()?;
        self.validate_post_merge_fields()?;
        Ok(())
    }

    /// Validates this header against a parent header.
    ///
    /// This checks:
    /// - parent hash matches the parent header hash
    /// - block number is parent.number + 1
    /// - timestamp is strictly greater than parent timestamp
    /// - gas limit is within allowed bounds and above minimum
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::BlockHeader;
    ///
    /// let parent = BlockHeader::default();
    /// let mut child = BlockHeader::default();
    /// child.parent_hash = parent.compute_hash();
    /// child.number = parent.number + 1;
    /// child.timestamp = parent.timestamp + 1;
    ///
    /// assert!(child.validate_against_parent(&parent).is_ok());
    /// ```
    pub fn validate_against_parent(&self, parent: &BlockHeader) -> Result<(), ValidationError> {
        let parent_hash = parent.compute_hash();
        if self.parent_hash != parent_hash {
            return Err(ValidationError::ParentHashMismatch);
        }

        let expected_number = parent.number.saturating_add(1);
        if self.number != expected_number {
            return Err(ValidationError::InvalidBlockNumber {
                expected: expected_number,
                actual: self.number,
            });
        }

        if self.timestamp <= parent.timestamp {
            return Err(ValidationError::InvalidTimestamp {
                parent: parent.timestamp,
                actual: self.timestamp,
            });
        }

        if self.gas_limit < Self::MIN_GAS_LIMIT {
            return Err(ValidationError::GasLimitBelowMinimum {
                gas_limit: self.gas_limit,
                min_gas_limit: Self::MIN_GAS_LIMIT,
            });
        }

        let max_change = parent.gas_limit / Self::GAS_LIMIT_BOUND_DIVISOR;
        let min_gas_limit = parent.gas_limit.saturating_sub(max_change);
        let max_gas_limit = parent.gas_limit.saturating_add(max_change);

        if self.gas_limit < min_gas_limit {
            return Err(ValidationError::GasLimitTooLow {
                gas_limit: self.gas_limit,
                min_gas_limit,
            });
        }

        if self.gas_limit > max_gas_limit {
            return Err(ValidationError::GasLimitTooHigh {
                gas_limit: self.gas_limit,
                max_gas_limit,
            });
        }

        Ok(())
    }

    /// Computes the block hash (Keccak-256 of RLP encoding).
    ///
    /// The block hash is computed by first encoding the header as RLP,
    /// then computing the Keccak-256 hash of the RLP-encoded data.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::BlockHeader;
    ///
    /// let header = BlockHeader::default();
    /// let hash = header.compute_hash();
    /// assert_eq!(hash.as_bytes().len(), 32);
    /// ```
    pub fn compute_hash(&self) -> Hash {
        let rlp_encoded = self.encode_rlp();
        keccak256(&rlp_encoded)
    }

    /// Encodes the block header as RLP.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::BlockHeader;
    ///
    /// let header = BlockHeader::default();
    /// let encoded = header.encode_rlp();
    /// assert!(!encoded.is_empty());
    /// ```
    pub fn encode_rlp(&self) -> Vec<u8> {
        let mut items = vec![
            rlp::encode_hash(&self.parent_hash),
            rlp::encode_hash(&self.ommers_hash),
            rlp::encode_address(&self.coinbase),
            rlp::encode_hash(&self.state_root),
            rlp::encode_hash(&self.transactions_root),
            rlp::encode_hash(&self.receipts_root),
            rlp::encode_bytes(&self.logs_bloom),
            rlp::encode_u256(&self.difficulty),
            rlp::encode_u64(self.number),
            rlp::encode_u64(self.gas_limit),
            rlp::encode_u64(self.gas_used),
            rlp::encode_u64(self.timestamp),
            rlp::encode_bytes(self.extra_data.as_ref()),
            rlp::encode_hash(&self.mix_hash),
            rlp::encode_bytes(&self.nonce.to_be_bytes()),
        ];

        // Add optional fields if present
        if let Some(base_fee) = self.base_fee_per_gas {
            items.push(rlp::encode_u64(base_fee));
        }

        if let Some(withdrawals_root) = self.withdrawals_root {
            items.push(rlp::encode_hash(&withdrawals_root));
        }

        if let Some(blob_gas_used) = self.blob_gas_used {
            items.push(rlp::encode_u64(blob_gas_used));
        }

        if let Some(excess_blob_gas) = self.excess_blob_gas {
            items.push(rlp::encode_u64(excess_blob_gas));
        }

        if let Some(parent_beacon_block_root) = self.parent_beacon_block_root {
            items.push(rlp::encode_hash(&parent_beacon_block_root));
        }

        if let Some(requests_hash) = self.requests_hash {
            items.push(rlp::encode_hash(&requests_hash));
        }

        rlp::encode_list(&items)
    }

    /// Decodes a block header from RLP encoding.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::BlockHeader;
    ///
    /// let header = BlockHeader::default();
    /// let encoded = header.encode_rlp();
    /// let decoded = BlockHeader::decode_rlp(&encoded).unwrap();
    /// assert_eq!(header, decoded);
    /// ```
    pub fn decode_rlp(input: &[u8]) -> Result<Self, RlpError> {
        let (items, _rest) = rlp::decode_list(input)?;

        // Minimum 15 fields required (pre-London)
        if items.len() < 15 {
            return Err(RlpError::InvalidEncoding);
        }

        // Decode mandatory fields
        let (parent_hash, _) = rlp::decode_hash(&items[0])?;
        let (ommers_hash, _) = rlp::decode_hash(&items[1])?;
        let (coinbase, _) = rlp::decode_address(&items[2])?;
        let (state_root, _) = rlp::decode_hash(&items[3])?;
        let (transactions_root, _) = rlp::decode_hash(&items[4])?;
        let (receipts_root, _) = rlp::decode_hash(&items[5])?;

        let (logs_bloom_bytes, _) = rlp::decode_bytes(&items[6])?;
        if logs_bloom_bytes.len() != 256 {
            return Err(RlpError::InvalidLength);
        }
        let mut logs_bloom = [0u8; 256];
        logs_bloom.copy_from_slice(&logs_bloom_bytes);

        let (difficulty, _) = rlp::decode_u256(&items[7])?;
        let (number, _) = rlp::decode_u64(&items[8])?;
        let (gas_limit, _) = rlp::decode_u64(&items[9])?;
        let (gas_used, _) = rlp::decode_u64(&items[10])?;
        let (timestamp, _) = rlp::decode_u64(&items[11])?;

        let (extra_data_bytes, _) = rlp::decode_bytes(&items[12])?;
        let extra_data = Bytes::from_slice(&extra_data_bytes);

        let (mix_hash, _) = rlp::decode_hash(&items[13])?;
        let (nonce_bytes, _) = rlp::decode_bytes(&items[14])?;
        if nonce_bytes.len() > 8 {
            return Err(RlpError::InvalidLength);
        }
        let mut nonce_buf = [0u8; 8];
        nonce_buf[8 - nonce_bytes.len()..].copy_from_slice(&nonce_bytes);
        let nonce = u64::from_be_bytes(nonce_buf);

        // Decode optional fields if present
        let base_fee_per_gas = if items.len() > 15 {
            let (base_fee, _) = rlp::decode_u64(&items[15])?;
            Some(base_fee)
        } else {
            None
        };

        let withdrawals_root = if items.len() > 16 {
            let (root, _) = rlp::decode_hash(&items[16])?;
            Some(root)
        } else {
            None
        };

        let blob_gas_used = if items.len() > 17 {
            let (blob_gas, _) = rlp::decode_u64(&items[17])?;
            Some(blob_gas)
        } else {
            None
        };

        let excess_blob_gas = if items.len() > 18 {
            let (excess, _) = rlp::decode_u64(&items[18])?;
            Some(excess)
        } else {
            None
        };

        let parent_beacon_block_root = if items.len() > 19 {
            let (root, _) = rlp::decode_hash(&items[19])?;
            Some(root)
        } else {
            None
        };

        let requests_hash = if items.len() > 20 {
            let (hash, _) = rlp::decode_hash(&items[20])?;
            Some(hash)
        } else {
            None
        };

        Ok(BlockHeader {
            parent_hash,
            ommers_hash,
            coinbase,
            state_root,
            transactions_root,
            receipts_root,
            logs_bloom,
            difficulty,
            number,
            gas_limit,
            gas_used,
            timestamp,
            extra_data,
            mix_hash,
            nonce,
            base_fee_per_gas,
            withdrawals_root,
            blob_gas_used,
            excess_blob_gas,
            parent_beacon_block_root,
            requests_hash,
        })
    }
}

impl Default for BlockHeader {
    fn default() -> Self {
        BlockHeader {
            parent_hash: Hash::ZERO,
            ommers_hash: EMPTY_OMMERS_HASH,
            coinbase: Address::ZERO,
            state_root: Hash::ZERO,
            transactions_root: Hash::ZERO,
            receipts_root: Hash::ZERO,
            logs_bloom: [0u8; 256],
            difficulty: U256::ZERO,
            number: 0,
            gas_limit: 30_000_000,
            gas_used: 0,
            timestamp: 0,
            extra_data: Bytes::new(),
            mix_hash: Hash::ZERO,
            nonce: 0,
            base_fee_per_gas: Some(1_000_000_000),
            withdrawals_root: None,
            blob_gas_used: None,
            excess_blob_gas: None,
            parent_beacon_block_root: None,
            requests_hash: None,
        }
    }
}

impl StdHash for BlockHeader {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.parent_hash.hash(state);
        self.ommers_hash.hash(state);
        self.coinbase.hash(state);
        self.state_root.hash(state);
        self.transactions_root.hash(state);
        self.receipts_root.hash(state);
        self.logs_bloom.hash(state);
        self.difficulty.hash(state);
        self.number.hash(state);
        self.gas_limit.hash(state);
        self.gas_used.hash(state);
        self.timestamp.hash(state);
        self.extra_data.hash(state);
        self.mix_hash.hash(state);
        self.nonce.hash(state);
        self.base_fee_per_gas.hash(state);
        self.withdrawals_root.hash(state);
        self.blob_gas_used.hash(state);
        self.excess_blob_gas.hash(state);
        self.parent_beacon_block_root.hash(state);
        self.requests_hash.hash(state);
    }
}

impl fmt::Display for BlockHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Block #{} (hash computation pending Phase 1)",
            self.number
        )
    }
}

// =============================================================================
// Error Types
// =============================================================================

/// Validation errors for block headers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Gas used exceeds gas limit
    GasUsedExceedsLimit { gas_used: u64, gas_limit: u64 },
    /// Extra data exceeds maximum size
    ExtraDataTooLarge { size: usize, max_size: usize },
    /// Non-zero difficulty in post-merge block
    NonZeroDifficulty,
    /// Non-zero mix hash in post-merge block
    NonZeroMixHash,
    /// Non-zero nonce in post-merge block
    NonZeroNonce,
    /// Parent hash does not match provided parent header hash
    ParentHashMismatch,
    /// Block number is not parent.number + 1
    InvalidBlockNumber { expected: u64, actual: u64 },
    /// Timestamp is not strictly greater than parent timestamp
    InvalidTimestamp { parent: u64, actual: u64 },
    /// Gas limit below minimum allowed
    GasLimitBelowMinimum { gas_limit: u64, min_gas_limit: u64 },
    /// Gas limit below allowed bound (parent - parent/1024)
    GasLimitTooLow { gas_limit: u64, min_gas_limit: u64 },
    /// Gas limit above allowed bound (parent + parent/1024)
    GasLimitTooHigh { gas_limit: u64, max_gas_limit: u64 },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::GasUsedExceedsLimit {
                gas_used,
                gas_limit,
            } => {
                write!(f, "gas used ({gas_used}) exceeds gas limit ({gas_limit})")
            }
            ValidationError::ExtraDataTooLarge { size, max_size } => {
                write!(f, "extra data size ({size}) exceeds maximum ({max_size})")
            }
            ValidationError::NonZeroDifficulty => {
                write!(f, "difficulty must be zero in post-merge blocks")
            }
            ValidationError::NonZeroMixHash => {
                write!(f, "mix hash must be zero in post-merge blocks")
            }
            ValidationError::NonZeroNonce => {
                write!(f, "nonce must be zero in post-merge blocks")
            }
            ValidationError::ParentHashMismatch => {
                write!(f, "parent hash does not match provided parent header")
            }
            ValidationError::InvalidBlockNumber { expected, actual } => {
                write!(
                    f,
                    "block number {actual} does not match expected {expected}"
                )
            }
            ValidationError::InvalidTimestamp { parent, actual } => {
                write!(
                    f,
                    "timestamp {actual} is not greater than parent timestamp {parent}"
                )
            }
            ValidationError::GasLimitBelowMinimum {
                gas_limit,
                min_gas_limit,
            } => {
                write!(f, "gas limit {gas_limit} below minimum {min_gas_limit}")
            }
            ValidationError::GasLimitTooLow {
                gas_limit,
                min_gas_limit,
            } => {
                write!(
                    f,
                    "gas limit {gas_limit} below allowed minimum {min_gas_limit}"
                )
            }
            ValidationError::GasLimitTooHigh {
                gas_limit,
                max_gas_limit,
            } => {
                write!(
                    f,
                    "gas limit {gas_limit} exceeds allowed maximum {max_gas_limit}"
                )
            }
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    // =========================================================================
    // Construction Tests
    // =========================================================================

    #[test]
    fn test_default_header() {
        let header = BlockHeader::default();
        assert_eq!(header.parent_hash, Hash::ZERO);
        assert_eq!(header.number, 0);
        assert_eq!(header.gas_limit, 30_000_000);
        assert_eq!(header.gas_used, 0);
        assert_eq!(header.difficulty, U256::ZERO);
        assert_eq!(header.base_fee_per_gas, Some(1_000_000_000));
    }

    #[test]
    fn test_custom_header() {
        let mut header = BlockHeader::default();
        header.number = 12345;
        header.timestamp = 1234567890;
        header.gas_used = 15_000_000;

        assert_eq!(header.number, 12345);
        assert_eq!(header.timestamp, 1234567890);
        assert_eq!(header.gas_used, 15_000_000);
    }

    // =========================================================================
    // Validation Tests
    // =========================================================================

    #[test]
    fn test_validate_gas_fields_valid() {
        let mut header = BlockHeader::default();
        header.gas_limit = 30_000_000;
        header.gas_used = 15_000_000;
        assert!(header.validate_gas_fields().is_ok());
    }

    #[test]
    fn test_validate_gas_fields_equal() {
        let mut header = BlockHeader::default();
        header.gas_limit = 30_000_000;
        header.gas_used = 30_000_000;
        assert!(header.validate_gas_fields().is_ok());
    }

    #[test]
    fn test_validate_gas_fields_invalid() {
        let mut header = BlockHeader::default();
        header.gas_limit = 30_000_000;
        header.gas_used = 35_000_000;
        assert!(header.validate_gas_fields().is_err());
    }

    #[test]
    fn test_validate_extra_data_valid() {
        let mut header = BlockHeader::default();
        header.extra_data = Bytes::from_slice(&[0x42; 32]);
        assert!(header.validate_extra_data().is_ok());
    }

    #[test]
    fn test_validate_extra_data_empty() {
        let mut header = BlockHeader::default();
        header.extra_data = Bytes::new();
        assert!(header.validate_extra_data().is_ok());
    }

    #[test]
    fn test_validate_extra_data_invalid() {
        let mut header = BlockHeader::default();
        header.extra_data = Bytes::from_slice(&[0x42; 33]);
        assert!(header.validate_extra_data().is_err());
    }

    #[test]
    fn test_validate_post_merge_valid() {
        let mut header = BlockHeader::default();
        header.difficulty = U256::ZERO;
        header.mix_hash = Hash::ZERO;
        header.nonce = 0;
        assert!(header.validate_post_merge_fields().is_ok());
    }

    #[test]
    fn test_validate_post_merge_invalid_difficulty() {
        let mut header = BlockHeader::default();
        header.difficulty = U256::from(1u64);
        assert!(header.validate_post_merge_fields().is_err());
    }

    #[test]
    fn test_validate_post_merge_invalid_mix_hash() {
        let mut header = BlockHeader::default();
        header.mix_hash = Hash::from([0x42; 32]);
        assert!(header.validate_post_merge_fields().is_err());
    }

    #[test]
    fn test_validate_post_merge_invalid_nonce() {
        let mut header = BlockHeader::default();
        header.nonce = 1;
        assert!(header.validate_post_merge_fields().is_err());
    }

    #[test]
    fn test_validate_all_valid() {
        let header = BlockHeader::default();
        assert!(header.validate().is_ok());
    }

    #[test]
    fn test_validate_against_parent_valid() {
        let parent = BlockHeader::default();
        let mut child = BlockHeader::default();
        child.parent_hash = parent.compute_hash();
        child.number = parent.number + 1;
        child.timestamp = parent.timestamp + 1;
        child.gas_limit = parent.gas_limit;
        assert!(child.validate_against_parent(&parent).is_ok());
    }

    #[test]
    fn test_validate_against_parent_hash_mismatch() {
        let parent = BlockHeader::default();
        let mut child = BlockHeader::default();
        child.parent_hash = Hash::from([0x11; 32]);
        child.number = parent.number + 1;
        child.timestamp = parent.timestamp + 1;
        let err = child.validate_against_parent(&parent).unwrap_err();
        assert_eq!(err, ValidationError::ParentHashMismatch);
    }

    #[test]
    fn test_validate_against_parent_number_invalid() {
        let parent = BlockHeader::default();
        let mut child = BlockHeader::default();
        child.parent_hash = parent.compute_hash();
        child.number = parent.number + 2;
        child.timestamp = parent.timestamp + 1;
        let err = child.validate_against_parent(&parent).unwrap_err();
        assert_eq!(
            err,
            ValidationError::InvalidBlockNumber {
                expected: parent.number + 1,
                actual: parent.number + 2
            }
        );
    }

    #[test]
    fn test_validate_against_parent_timestamp_invalid() {
        let parent = BlockHeader::default();
        let mut child = BlockHeader::default();
        child.parent_hash = parent.compute_hash();
        child.number = parent.number + 1;
        child.timestamp = parent.timestamp;
        let err = child.validate_against_parent(&parent).unwrap_err();
        assert_eq!(
            err,
            ValidationError::InvalidTimestamp {
                parent: parent.timestamp,
                actual: parent.timestamp
            }
        );
    }

    #[test]
    fn test_validate_against_parent_gas_limit_below_minimum() {
        let mut parent = BlockHeader::default();
        parent.gas_limit = BlockHeader::MIN_GAS_LIMIT;

        let mut child = BlockHeader::default();
        child.parent_hash = parent.compute_hash();
        child.number = parent.number + 1;
        child.timestamp = parent.timestamp + 1;
        child.gas_limit = BlockHeader::MIN_GAS_LIMIT - 1;

        let err = child.validate_against_parent(&parent).unwrap_err();
        assert_eq!(
            err,
            ValidationError::GasLimitBelowMinimum {
                gas_limit: BlockHeader::MIN_GAS_LIMIT - 1,
                min_gas_limit: BlockHeader::MIN_GAS_LIMIT
            }
        );
    }

    #[test]
    fn test_validate_against_parent_gas_limit_too_low() {
        let mut parent = BlockHeader::default();
        parent.gas_limit = 100_000;

        let max_change = parent.gas_limit / BlockHeader::GAS_LIMIT_BOUND_DIVISOR;
        let min_allowed = parent.gas_limit - max_change;

        let mut child = BlockHeader::default();
        child.parent_hash = parent.compute_hash();
        child.number = parent.number + 1;
        child.timestamp = parent.timestamp + 1;
        child.gas_limit = min_allowed - 1;

        let err = child.validate_against_parent(&parent).unwrap_err();
        assert_eq!(
            err,
            ValidationError::GasLimitTooLow {
                gas_limit: min_allowed - 1,
                min_gas_limit: min_allowed
            }
        );
    }

    #[test]
    fn test_validate_against_parent_gas_limit_too_high() {
        let mut parent = BlockHeader::default();
        parent.gas_limit = 100_000;

        let max_change = parent.gas_limit / BlockHeader::GAS_LIMIT_BOUND_DIVISOR;
        let max_allowed = parent.gas_limit + max_change;

        let mut child = BlockHeader::default();
        child.parent_hash = parent.compute_hash();
        child.number = parent.number + 1;
        child.timestamp = parent.timestamp + 1;
        child.gas_limit = max_allowed + 1;

        let err = child.validate_against_parent(&parent).unwrap_err();
        assert_eq!(
            err,
            ValidationError::GasLimitTooHigh {
                gas_limit: max_allowed + 1,
                max_gas_limit: max_allowed
            }
        );
    }

    // =========================================================================
    // RLP Encoding Tests
    // =========================================================================

    #[test]
    fn test_encode_rlp_basic() {
        let header = BlockHeader::default();
        let encoded = header.encode_rlp();
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_roundtrip_rlp_default() {
        let header = BlockHeader::default();
        let encoded = header.encode_rlp();
        let decoded = BlockHeader::decode_rlp(&encoded).unwrap();
        assert_eq!(header, decoded);
    }

    #[test]
    fn test_roundtrip_rlp_custom() {
        let mut header = BlockHeader::default();
        header.number = 12345;
        header.timestamp = 1234567890;
        header.gas_used = 15_000_000;
        header.extra_data = Bytes::from_slice(&[0x42, 0x43, 0x44]);

        let encoded = header.encode_rlp();
        let decoded = BlockHeader::decode_rlp(&encoded).unwrap();
        assert_eq!(header, decoded);
    }

    #[test]
    fn test_roundtrip_rlp_with_all_optional_fields() {
        let mut header = BlockHeader::default();
        header.base_fee_per_gas = Some(2_000_000_000);
        header.withdrawals_root = Some(Hash::from([0x42; 32]));
        header.blob_gas_used = Some(131072);
        header.excess_blob_gas = Some(262144);
        header.parent_beacon_block_root = Some(Hash::from([0x43; 32]));
        header.requests_hash = Some(Hash::from([0x44; 32]));

        let encoded = header.encode_rlp();
        let decoded = BlockHeader::decode_rlp(&encoded).unwrap();
        assert_eq!(header, decoded);
    }

    #[test]
    fn test_roundtrip_rlp_without_optional_fields() {
        let mut header = BlockHeader::default();
        header.base_fee_per_gas = None;
        header.withdrawals_root = None;
        header.blob_gas_used = None;
        header.excess_blob_gas = None;
        header.parent_beacon_block_root = None;
        header.requests_hash = None;

        let encoded = header.encode_rlp();
        let decoded = BlockHeader::decode_rlp(&encoded).unwrap();
        assert_eq!(header, decoded);
    }

    #[test]
    fn test_roundtrip_rlp_partial_optional_fields() {
        let mut header = BlockHeader::default();
        header.base_fee_per_gas = Some(1_000_000_000);
        header.withdrawals_root = Some(Hash::from([0x42; 32]));
        header.blob_gas_used = None;
        header.excess_blob_gas = None;
        header.parent_beacon_block_root = None;
        header.requests_hash = None;

        let encoded = header.encode_rlp();
        let decoded = BlockHeader::decode_rlp(&encoded).unwrap();
        assert_eq!(header, decoded);
    }

    #[test]
    fn test_decode_rlp_invalid_too_short() {
        let items = vec![vec![0x00]; 14]; // Only 14 fields
        let encoded = rlp::encode_list(&items);
        assert!(BlockHeader::decode_rlp(&encoded).is_err());
    }

    #[test]
    fn test_decode_rlp_invalid_logs_bloom_size() {
        let items = vec![
            rlp::encode_hash(&Hash::ZERO),
            rlp::encode_hash(&Hash::ZERO),
            rlp::encode_address(&Address::ZERO),
            rlp::encode_hash(&Hash::ZERO),
            rlp::encode_hash(&Hash::ZERO),
            rlp::encode_hash(&Hash::ZERO),
            rlp::encode_bytes(&[0u8; 100]), // Wrong size
            rlp::encode_u256(&U256::ZERO),
            rlp::encode_u64(0),
            rlp::encode_u64(30_000_000),
            rlp::encode_u64(0),
            rlp::encode_u64(0),
            rlp::encode_bytes(&[]),
            rlp::encode_hash(&Hash::ZERO),
            rlp::encode_bytes(&[0u8; 8]),
        ];
        let encoded = rlp::encode_list(&items);
        assert!(BlockHeader::decode_rlp(&encoded).is_err());
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[test]
    fn test_display() {
        let mut header = BlockHeader::default();
        header.number = 12345;
        let s = header.to_string();
        assert!(s.contains("12345"));
    }

    // =========================================================================
    // Hash Tests
    // =========================================================================

    #[test]
    fn test_std_hash() {
        use std::collections::hash_map::DefaultHasher;

        let header1 = BlockHeader::default();
        let header2 = BlockHeader::default();

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();

        header1.hash(&mut hasher1);
        header2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_std_hash_different() {
        use std::collections::hash_map::DefaultHasher;

        let header1 = BlockHeader::default();
        let mut header2 = BlockHeader::default();
        header2.number = 12345;

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();

        header1.hash(&mut hasher1);
        header2.hash(&mut hasher2);

        assert_ne!(hasher1.finish(), hasher2.finish());
    }

    // =========================================================================
    // Clone and Equality Tests
    // =========================================================================

    #[test]
    fn test_clone() {
        let header1 = BlockHeader::default();
        let header2 = header1.clone();
        assert_eq!(header1, header2);
    }

    #[test]
    fn test_equality() {
        let header1 = BlockHeader::default();
        let header2 = BlockHeader::default();
        assert_eq!(header1, header2);
    }

    #[test]
    fn test_inequality() {
        let header1 = BlockHeader::default();
        let mut header2 = BlockHeader::default();
        header2.number = 12345;
        assert_ne!(header1, header2);
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn test_max_gas_limit() {
        let mut header = BlockHeader::default();
        header.gas_limit = u64::MAX;
        header.gas_used = u64::MAX;
        assert!(header.validate_gas_fields().is_ok());
    }

    #[test]
    fn test_max_extra_data() {
        let mut header = BlockHeader::default();
        header.extra_data = Bytes::from_slice(&[0xff; 32]);
        assert!(header.validate_extra_data().is_ok());
    }

    #[test]
    fn test_zero_gas_limit() {
        let mut header = BlockHeader::default();
        header.gas_limit = 0;
        header.gas_used = 0;
        assert!(header.validate_gas_fields().is_ok());
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[test]
    fn test_serialize() {
        let header = BlockHeader::default();
        let json = serde_json::to_string(&header).unwrap();
        assert!(!json.is_empty());
    }

    #[test]
    fn test_deserialize() {
        let header1 = BlockHeader::default();
        let json = serde_json::to_string(&header1).unwrap();
        let header2: BlockHeader = serde_json::from_str(&json).unwrap();
        assert_eq!(header1, header2);
    }

    #[test]
    fn test_roundtrip_serialize() {
        let mut header1 = BlockHeader::default();
        header1.number = 12345;
        header1.timestamp = 1234567890;
        header1.gas_used = 15_000_000;

        let json = serde_json::to_string(&header1).unwrap();
        let header2: BlockHeader = serde_json::from_str(&json).unwrap();
        assert_eq!(header1, header2);
    }

    // =========================================================================
    // Block Hash Tests
    // =========================================================================

    #[test]
    fn test_compute_hash_default_header() {
        let header = BlockHeader::default();
        let hash = header.compute_hash();
        // Just verify it produces a 32-byte hash
        assert_eq!(hash.as_bytes().len(), 32);
        assert_ne!(hash, Hash::ZERO);
    }

    #[test]
    fn test_compute_hash_deterministic() {
        let header = BlockHeader::default();
        let hash1 = header.compute_hash();
        let hash2 = header.compute_hash();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_different_headers() {
        let header1 = BlockHeader::default();
        let mut header2 = BlockHeader::default();
        header2.number = 1;

        let hash1 = header1.compute_hash();
        let hash2 = header2.compute_hash();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_with_custom_fields() {
        let mut header = BlockHeader::default();
        header.number = 12345;
        header.timestamp = 1234567890;
        header.gas_used = 15_000_000;

        let hash = header.compute_hash();
        assert_eq!(hash.as_bytes().len(), 32);
        assert_ne!(hash, Hash::ZERO);
    }

    #[test]
    fn test_compute_hash_with_all_optional_fields() {
        let mut header = BlockHeader::default();
        header.base_fee_per_gas = Some(2_000_000_000);
        header.withdrawals_root = Some(Hash::from([0x42; 32]));
        header.blob_gas_used = Some(131072);
        header.excess_blob_gas = Some(262144);
        header.parent_beacon_block_root = Some(Hash::from([0x43; 32]));
        header.requests_hash = Some(Hash::from([0x44; 32]));

        let hash = header.compute_hash();
        assert_eq!(hash.as_bytes().len(), 32);
        assert_ne!(hash, Hash::ZERO);
    }

    #[test]
    fn test_compute_hash_without_optional_fields() {
        let mut header = BlockHeader::default();
        header.base_fee_per_gas = None;
        header.withdrawals_root = None;
        header.blob_gas_used = None;
        header.excess_blob_gas = None;
        header.parent_beacon_block_root = None;
        header.requests_hash = None;

        let hash = header.compute_hash();
        assert_eq!(hash.as_bytes().len(), 32);
        assert_ne!(hash, Hash::ZERO);
    }

    #[test]
    fn test_compute_hash_sensitivity() {
        // Test that hash changes with each field modification
        let base_header = BlockHeader::default();
        let base_hash = base_header.compute_hash();

        // Modify each field and verify hash changes
        let mut modified = base_header.clone();
        modified.parent_hash = Hash::from([0x01; 32]);
        assert_ne!(modified.compute_hash(), base_hash);

        let mut modified = base_header.clone();
        modified.number = 1;
        assert_ne!(modified.compute_hash(), base_hash);

        let mut modified = base_header.clone();
        modified.timestamp = 1;
        assert_ne!(modified.compute_hash(), base_hash);

        let mut modified = base_header.clone();
        modified.gas_used = 1;
        assert_ne!(modified.compute_hash(), base_hash);

        let mut modified = base_header.clone();
        modified.extra_data = Bytes::from_slice(&[0x42]);
        assert_ne!(modified.compute_hash(), base_hash);
    }

    #[test]
    fn test_compute_hash_roundtrip_consistency() {
        // Verify that hash of a decoded header matches the original
        let header1 = BlockHeader::default();
        let hash1 = header1.compute_hash();

        let encoded = header1.encode_rlp();
        let header2 = BlockHeader::decode_rlp(&encoded).unwrap();
        let hash2 = header2.compute_hash();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_ethereum_genesis() {
        // Ethereum mainnet genesis block header (simplified test)
        let mut header = BlockHeader::default();
        header.number = 0;
        header.timestamp = 0;
        header.gas_limit = 5000;
        header.gas_used = 0;
        header.base_fee_per_gas = None;

        let hash = header.compute_hash();
        // Just verify it produces a valid hash
        assert_eq!(hash.as_bytes().len(), 32);
    }

    #[test]
    fn test_compute_hash_post_merge_block() {
        // Test a typical post-merge block
        let mut header = BlockHeader::default();
        header.number = 15537394; // First post-merge block
        header.difficulty = U256::ZERO;
        header.mix_hash = Hash::ZERO;
        header.nonce = 0;
        header.base_fee_per_gas = Some(12_000_000_000);

        let hash = header.compute_hash();
        assert_eq!(hash.as_bytes().len(), 32);
        assert_ne!(hash, Hash::ZERO);
    }

    #[test]
    fn test_compute_hash_cancun_block() {
        // Test a Cancun fork block with blob fields
        let mut header = BlockHeader::default();
        header.number = 19426589; // Example Cancun block
        header.base_fee_per_gas = Some(10_000_000_000);
        header.withdrawals_root = Some(Hash::from([0x11; 32]));
        header.blob_gas_used = Some(131072);
        header.excess_blob_gas = Some(262144);
        header.parent_beacon_block_root = Some(Hash::from([0x22; 32]));
        header.requests_hash = Some(Hash::from([0x33; 32]));

        let hash = header.compute_hash();
        assert_eq!(hash.as_bytes().len(), 32);
        assert_ne!(hash, Hash::ZERO);
    }

    #[test]
    fn test_compute_hash_with_extra_data() {
        // Test with various extra_data sizes
        let mut header = BlockHeader::default();

        // Empty extra_data
        header.extra_data = Bytes::new();
        let hash1 = header.compute_hash();

        // Small extra_data
        header.extra_data = Bytes::from_slice(b"Geth/v1.0.0");
        let hash2 = header.compute_hash();

        // Max extra_data
        header.extra_data = Bytes::from_slice(&[0x42; 32]);
        let hash3 = header.compute_hash();

        // All hashes should be different
        assert_ne!(hash1, hash2);
        assert_ne!(hash2, hash3);
        assert_ne!(hash1, hash3);
    }
}
