//! Transaction receipts for Ethereum
//!
//! This module implements transaction receipts, bloom filters, and receipt root calculation
//! according to the Ethereum Yellow Paper specification.
//!
//! ## Receipt Structure (Post-EIP-658)
//!
//! A transaction receipt contains:
//! - `status`: true for success, false for failure
//! - `cumulative_gas_used`: Total gas used in block up to this transaction
//! - `logs_bloom`: 2048-bit bloom filter for fast log searching
//! - `logs`: Event logs emitted by the transaction
//!
//! ## Bloom Filter
//!
//! The bloom filter is a 2048-bit (256-byte) probabilistic data structure for testing
//! set membership. For each input (log address or topic), three bits are set in the filter
//! based on the Keccak-256 hash of the input.

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::{vec, vec::Vec};

use crate::crypto::{encode_address, encode_bytes, encode_list, encode_u256, keccak256, RlpError};
use crate::types::{Address, Bytes, Hash, U256};

// =============================================================================
// Log Structure
// =============================================================================

/// Event log emitted by a transaction.
///
/// Logs are created when smart contracts execute LOG0, LOG1, LOG2, LOG3, or LOG4 opcodes.
/// Each log contains:
/// - The address of the contract that emitted it
/// - Up to 4 indexed topics (for efficient filtering)
/// - Arbitrary non-indexed data
///
/// # Examples
///
/// ```
/// use claudeth::types::{Address, Bytes};
/// use claudeth::stf::receipt::Log;
///
/// let address = Address::from([0x42; 20]);
/// let topics = vec![];
/// let data = Bytes::from(vec![0x01, 0x02, 0x03]);
///
/// let log = Log::new(address, topics, data);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Log {
    /// Contract address that emitted the log
    pub address: Address,
    /// Indexed event parameters (maximum 4 topics)
    pub topics: Vec<Hash>,
    /// Non-indexed event data
    pub data: Bytes,
}

impl Log {
    /// Creates a new log.
    ///
    /// # Panics
    ///
    /// Panics if more than 4 topics are provided (Ethereum limit).
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::{Address, Bytes, Hash};
    /// use claudeth::stf::receipt::Log;
    ///
    /// let address = Address::from([0x42; 20]);
    /// let topics = vec![Hash::from([0x01; 32])];
    /// let data = Bytes::from(vec![0xaa, 0xbb]);
    ///
    /// let log = Log::new(address, topics, data);
    /// assert_eq!(log.address, address);
    /// assert_eq!(log.topics.len(), 1);
    /// ```
    pub fn new(address: Address, topics: Vec<Hash>, data: Bytes) -> Self {
        assert!(topics.len() <= 4, "maximum 4 topics allowed per log");
        Self {
            address,
            topics,
            data,
        }
    }

    /// Encodes the log as RLP.
    ///
    /// RLP encoding: `[address, [topic1, topic2, ...], data]`
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::{Address, Bytes};
    /// use claudeth::stf::receipt::Log;
    ///
    /// let log = Log::new(Address::ZERO, vec![], Bytes::new());
    /// let encoded = log.encode_rlp();
    /// assert!(!encoded.is_empty());
    /// ```
    pub fn encode_rlp(&self) -> Vec<u8> {
        let address_rlp = encode_address(&self.address);

        let topics_rlp: Vec<Vec<u8>> = self.topics.iter()
            .map(crate::crypto::encode_hash)
            .collect();
        let topics_list = encode_list(&topics_rlp);

        let data_rlp = encode_bytes(self.data.as_ref());

        encode_list(&[address_rlp, topics_list, data_rlp])
    }

    /// Decodes a log from RLP.
    ///
    /// # Errors
    ///
    /// Returns `RlpError` if the input is not a valid RLP-encoded log.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::{Address, Bytes};
    /// use claudeth::stf::receipt::Log;
    ///
    /// let log = Log::new(Address::ZERO, vec![], Bytes::new());
    /// let encoded = log.encode_rlp();
    /// let decoded = Log::decode_rlp(&encoded).unwrap();
    /// assert_eq!(log, decoded);
    /// ```
    pub fn decode_rlp(data: &[u8]) -> Result<Self, RlpError> {
        let (items, _rest) = crate::crypto::decode_list(data)?;

        if items.len() != 3 {
            return Err(RlpError::InvalidEncoding);
        }

        let (address, _) = crate::crypto::decode_address(&items[0])?;

        let (topic_items, _) = crate::crypto::decode_list(&items[1])?;
        let mut topics = Vec::new();
        for topic_bytes in topic_items {
            let (topic, _) = crate::crypto::decode_hash(&topic_bytes)?;
            topics.push(topic);
        }

        if topics.len() > 4 {
            return Err(RlpError::InvalidEncoding);
        }

        let (data_bytes, _) = crate::crypto::decode_bytes(&items[2])?;
        let data = Bytes::from(data_bytes);

        Ok(Self {
            address,
            topics,
            data,
        })
    }
}

// =============================================================================
// Bloom Filter
// =============================================================================

/// 2048-bit (256-byte) bloom filter for efficient log searching.
///
/// The bloom filter allows fast probabilistic testing of whether a log
/// with a specific address or topic might exist in a transaction receipt.
///
/// ## Algorithm (Ethereum Yellow Paper)
///
/// For each input:
/// 1. Compute h = keccak256(input)
/// 2. For i in 0..3:
///    - Extract m = (h[2*i] << 8) | h[2*i+1]
///    - bit_index = m & 0x7FF (11 bits, 0-2047)
///    - Set bit at bit_index in the 256-byte array
///
/// # Examples
///
/// ```
/// use claudeth::stf::receipt::Bloom;
///
/// let mut bloom = Bloom::new();
/// bloom.add(b"hello");
/// assert!(bloom.contains(b"hello"));
/// assert!(!bloom.contains(b"world"));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bloom {
    data: [u8; 256],
}

impl Bloom {
    /// Creates an empty bloom filter (all bits zero).
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::stf::receipt::Bloom;
    ///
    /// let bloom = Bloom::new();
    /// assert!(!bloom.contains(b"anything"));
    /// ```
    pub fn new() -> Self {
        Self { data: [0u8; 256] }
    }

    /// Adds data to the bloom filter.
    ///
    /// Sets 3 bits based on the Keccak-256 hash of the input.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::stf::receipt::Bloom;
    ///
    /// let mut bloom = Bloom::new();
    /// bloom.add(b"test");
    /// assert!(bloom.contains(b"test"));
    /// ```
    pub fn add(&mut self, input: &[u8]) {
        let hash = keccak256(input);
        let hash_bytes = hash.as_bytes();

        // Set 3 bits according to the execution-specs bloom definition
        for i in 0..3 {
            let m = ((hash_bytes[2 * i] as u16) << 8) | (hash_bytes[2 * i + 1] as u16);
            let bit_to_set = (m & 0x07FF) as usize;
            let bit_index = 0x07FF - bit_to_set;
            let byte_index = bit_index / 8;
            let bit_offset = bit_index % 8;

            self.data[byte_index] |= 1 << (7 - bit_offset);
        }
    }

    /// Checks if the bloom filter might contain the data.
    ///
    /// Returns true if all 3 bits for this input are set.
    /// False positives are possible, but false negatives are not.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::stf::receipt::Bloom;
    ///
    /// let mut bloom = Bloom::new();
    /// bloom.add(b"exists");
    ///
    /// assert!(bloom.contains(b"exists"));
    /// assert!(!bloom.contains(b"definitely not there"));
    /// ```
    pub fn contains(&self, input: &[u8]) -> bool {
        let hash = keccak256(input);
        let hash_bytes = hash.as_bytes();

        // Check all 3 bits using the execution-specs ordering
        for i in 0..3 {
            let m = ((hash_bytes[2 * i] as u16) << 8) | (hash_bytes[2 * i + 1] as u16);
            let bit_to_set = (m & 0x07FF) as usize;
            let bit_index = 0x07FF - bit_to_set;
            let byte_index = bit_index / 8;
            let bit_offset = bit_index % 8;

            if (self.data[byte_index] & (1 << (7 - bit_offset))) == 0 {
                return false;
            }
        }

        true
    }

    /// Adds a log to the bloom filter.
    ///
    /// Adds the log's address and all topics to the filter.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::{Address, Bytes, Hash};
    /// use claudeth::stf::receipt::{Bloom, Log};
    ///
    /// let mut bloom = Bloom::new();
    /// let log = Log::new(
    ///     Address::from([0x42; 20]),
    ///     vec![Hash::from([0x01; 32])],
    ///     Bytes::new()
    /// );
    ///
    /// bloom.add_log(&log);
    /// assert!(bloom.contains(log.address.as_ref()));
    /// assert!(bloom.contains(log.topics[0].as_ref()));
    /// ```
    pub fn add_log(&mut self, log: &Log) {
        // Add address
        self.add(log.address.as_ref());

        // Add all topics
        for topic in &log.topics {
            self.add(topic.as_ref());
        }
    }

    /// Combines two bloom filters with bitwise OR.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::stf::receipt::Bloom;
    ///
    /// let mut bloom1 = Bloom::new();
    /// bloom1.add(b"test1");
    ///
    /// let mut bloom2 = Bloom::new();
    /// bloom2.add(b"test2");
    ///
    /// bloom1.combine(&bloom2);
    /// assert!(bloom1.contains(b"test1"));
    /// assert!(bloom1.contains(b"test2"));
    /// ```
    pub fn combine(&mut self, other: &Bloom) {
        for i in 0..256 {
            self.data[i] |= other.data[i];
        }
    }

    /// Returns a reference to the underlying 256-byte array.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::stf::receipt::Bloom;
    ///
    /// let bloom = Bloom::new();
    /// assert_eq!(bloom.as_bytes().len(), 256);
    /// ```
    pub fn as_bytes(&self) -> &[u8; 256] {
        &self.data
    }

    /// Encodes the bloom filter as RLP (256-byte array).
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::stf::receipt::Bloom;
    ///
    /// let bloom = Bloom::new();
    /// let encoded = bloom.encode_rlp();
    /// assert!(!encoded.is_empty());
    /// ```
    pub fn encode_rlp(&self) -> Vec<u8> {
        encode_bytes(&self.data)
    }

    /// Decodes a bloom filter from RLP.
    ///
    /// # Errors
    ///
    /// Returns `RlpError` if the input is not a valid 256-byte RLP encoding.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::stf::receipt::Bloom;
    ///
    /// let bloom = Bloom::new();
    /// let encoded = bloom.encode_rlp();
    /// let decoded = Bloom::decode_rlp(&encoded).unwrap();
    /// assert_eq!(bloom, decoded);
    /// ```
    pub fn decode_rlp(data: &[u8]) -> Result<Self, RlpError> {
        let (bytes, _rest) = crate::crypto::decode_bytes(data)?;

        if bytes.len() != 256 {
            return Err(RlpError::InvalidLength);
        }

        let mut bloom_data = [0u8; 256];
        bloom_data.copy_from_slice(&bytes);

        Ok(Self { data: bloom_data })
    }
}

impl Default for Bloom {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Transaction Receipt
// =============================================================================

/// Transaction receipt (post-EIP-658 format).
///
/// A receipt contains the outcome of a transaction execution:
/// - Whether it succeeded or failed
/// - How much gas was used (cumulative in the block)
/// - What events (logs) were emitted
/// - A bloom filter for efficient log searching
///
/// # Examples
///
/// ```
/// use claudeth::types::{Address, Bytes, U256};
/// use claudeth::stf::receipt::{Log, TransactionReceipt};
///
/// let logs = vec![
///     Log::new(Address::from([0x42; 20]), vec![], Bytes::new())
/// ];
///
/// let receipt = TransactionReceipt::new(true, U256::from(21000u64), logs);
/// assert!(receipt.status);
/// assert_eq!(receipt.logs.len(), 1);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransactionReceipt {
    /// Transaction execution status (true = success, false = failure)
    pub status: bool,
    /// Cumulative gas used in the block up to and including this transaction
    pub cumulative_gas_used: U256,
    /// Event logs emitted by this transaction
    pub logs: Vec<Log>,
    /// Bloom filter for efficient log searching
    pub logs_bloom: Bloom,
}

impl TransactionReceipt {
    /// Creates a new transaction receipt.
    ///
    /// The logs bloom filter is automatically generated from the logs.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::{Address, Bytes, Hash, U256};
    /// use claudeth::stf::receipt::{Log, TransactionReceipt};
    ///
    /// let address = Address::from([0x42; 20]);
    /// let topic = Hash::from([0x01; 32]);
    /// let logs = vec![
    ///     Log::new(address, vec![topic], Bytes::new())
    /// ];
    ///
    /// let receipt = TransactionReceipt::new(
    ///     true,
    ///     U256::from(50000u64),
    ///     logs
    /// );
    ///
    /// assert!(receipt.status);
    /// assert_eq!(receipt.cumulative_gas_used, U256::from(50000u64));
    /// assert!(receipt.logs_bloom.contains(address.as_ref()));
    /// assert!(receipt.logs_bloom.contains(topic.as_ref()));
    /// ```
    pub fn new(status: bool, cumulative_gas_used: U256, logs: Vec<Log>) -> Self {
        // Generate bloom filter from logs
        let mut logs_bloom = Bloom::new();
        for log in &logs {
            logs_bloom.add_log(log);
        }

        Self {
            status,
            cumulative_gas_used,
            logs,
            logs_bloom,
        }
    }

    /// Encodes the receipt as RLP (post-EIP-658 format).
    ///
    /// RLP encoding: `[status, cumulative_gas_used, logs_bloom, logs]`
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::U256;
    /// use claudeth::stf::receipt::TransactionReceipt;
    ///
    /// let receipt = TransactionReceipt::new(true, U256::from(21000u64), vec![]);
    /// let encoded = receipt.encode_rlp();
    /// assert!(!encoded.is_empty());
    /// ```
    pub fn encode_rlp(&self) -> Vec<u8> {
        // Status: 1 for success, 0 for failure (as single byte or 0x80 for empty)
        let status_rlp = if self.status {
            vec![0x01]
        } else {
            vec![0x80] // Empty bytes = 0
        };

        let gas_rlp = encode_u256(&self.cumulative_gas_used);
        let bloom_rlp = self.logs_bloom.encode_rlp();

        let logs_rlp: Vec<Vec<u8>> = self.logs.iter()
            .map(|log| log.encode_rlp())
            .collect();
        let logs_list = encode_list(&logs_rlp);

        encode_list(&[status_rlp, gas_rlp, bloom_rlp, logs_list])
    }

    /// Decodes a receipt from RLP.
    ///
    /// # Errors
    ///
    /// Returns `RlpError` if the input is not a valid RLP-encoded receipt.
    ///
    /// # Examples
    ///
    /// ```
    /// use claudeth::types::U256;
    /// use claudeth::stf::receipt::TransactionReceipt;
    ///
    /// let receipt = TransactionReceipt::new(true, U256::from(21000u64), vec![]);
    /// let encoded = receipt.encode_rlp();
    /// let decoded = TransactionReceipt::decode_rlp(&encoded).unwrap();
    /// assert_eq!(receipt, decoded);
    /// ```
    pub fn decode_rlp(data: &[u8]) -> Result<Self, RlpError> {
        let (items, _rest) = crate::crypto::decode_list(data)?;

        if items.len() != 4 {
            return Err(RlpError::InvalidEncoding);
        }

        // Decode status
        let (status_bytes, _) = crate::crypto::decode_bytes(&items[0])?;
        let status = if status_bytes.is_empty() {
            false
        } else if status_bytes.len() == 1 && status_bytes[0] == 1 {
            true
        } else {
            return Err(RlpError::InvalidEncoding);
        };

        let (cumulative_gas_used, _) = crate::crypto::decode_u256(&items[1])?;
        let logs_bloom = Bloom::decode_rlp(&items[2])?;

        let (log_items, _) = crate::crypto::decode_list(&items[3])?;
        let mut logs = Vec::new();
        for log_bytes in log_items {
            let log = Log::decode_rlp(&log_bytes)?;
            logs.push(log);
        }

        Ok(Self {
            status,
            cumulative_gas_used,
            logs,
            logs_bloom,
        })
    }
}

// =============================================================================
// Receipt Root Calculation
// =============================================================================

/// Calculates the receipts root hash from a list of transaction receipts and transactions.
///
/// Uses a Merkle Patricia Trie where:
/// - Keys are RLP(transaction_index) for index = 0, 1, 2, ...
/// - Values are receipt encodings (with type prefix for typed transactions per EIP-2718)
///
/// For legacy transactions, the receipt is just RLP-encoded.
/// For typed transactions (EIP-2930, EIP-1559, EIP-4844), the receipt is:
/// TransactionType || RLP(receipt)
///
/// # Examples
///
/// ```
/// use claudeth::types::{U256, Hash, Transaction};
/// use claudeth::stf::receipt::{calculate_receipts_root_with_types, TransactionReceipt};
///
/// let receipts = vec![
///     TransactionReceipt::new(true, U256::from(21000u64), vec![]),
///     TransactionReceipt::new(true, U256::from(42000u64), vec![]),
/// ];
/// let transactions = vec![]; // Would contain actual transactions
///
/// let root = calculate_receipts_root_with_types(&receipts, &transactions);
/// assert_ne!(root, Hash::ZERO);
/// ```
pub fn calculate_receipts_root_with_types(
    receipts: &[TransactionReceipt],
    transactions: &[&crate::types::Transaction],
) -> Hash {
    use crate::state::partial_mpt::Trie;
    use crate::types::Transaction;

    let mut trie = Trie::new();

    for (index, receipt) in receipts.iter().enumerate() {
        // Key: RLP(index)
        let key = encode_u64(index as u64);

        // Value: Receipt encoding (with type prefix for typed transactions)
        let value = if index < transactions.len() {
            match transactions[index] {
                Transaction::Legacy(_) => {
                    // Legacy: just RLP-encoded receipt
                    receipt.encode_rlp()
                }
                Transaction::Eip2930(_) => {
                    // EIP-2930: 0x01 || RLP(receipt)
                    let mut encoded = vec![0x01];
                    encoded.extend(receipt.encode_rlp());
                    encoded
                }
                Transaction::Eip1559(_) => {
                    // EIP-1559: 0x02 || RLP(receipt)
                    let mut encoded = vec![0x02];
                    encoded.extend(receipt.encode_rlp());
                    encoded
                }
                Transaction::Blob(_) => {
                    // EIP-4844: 0x03 || RLP(receipt)
                    let mut encoded = vec![0x03];
                    encoded.extend(receipt.encode_rlp());
                    encoded
                }
            }
        } else {
            // Fallback: shouldn't happen if receipts and transactions match
            receipt.encode_rlp()
        };

        trie.insert(&key, value);
    }

    // Return the root hash (EMPTY_TRIE_ROOT for empty trie)
    trie.compute_root()
}

/// Calculates the receipts root hash from a list of transaction receipts.
///
/// This is a legacy function that doesn't handle typed transaction receipts correctly.
/// Use `calculate_receipts_root_with_types` instead for correct EIP-2718 compliance.
///
/// # Examples
///
/// ```
/// use claudeth::types::{U256, Hash};
/// use claudeth::stf::receipt::{calculate_receipts_root, TransactionReceipt};
///
/// let receipts = vec![
///     TransactionReceipt::new(true, U256::from(21000u64), vec![]),
///     TransactionReceipt::new(true, U256::from(42000u64), vec![]),
/// ];
///
/// let root = calculate_receipts_root(&receipts);
/// assert_ne!(root, Hash::ZERO);
/// ```
pub fn calculate_receipts_root(receipts: &[TransactionReceipt]) -> Hash {
    use crate::state::partial_mpt::Trie;

    let mut trie = Trie::new();

    for (index, receipt) in receipts.iter().enumerate() {
        // Key: RLP(index)
        let key = encode_u64(index as u64);

        // Value: RLP-encoded receipt (without type prefix)
        let value = receipt.encode_rlp();

        trie.insert(&key, value);
    }

    // Return the root hash (EMPTY_TRIE_ROOT for empty trie)
    trie.compute_root()
}

// Helper function to encode u64 for trie keys
fn encode_u64(n: u64) -> Vec<u8> {
    crate::crypto::encode_u64(n)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Log Tests
    // =========================================================================

    #[test]
    fn test_log_new() {
        let address = Address::from([0x42; 20]);
        let topics = vec![Hash::from([0x01; 32])];
        let data = Bytes::from(vec![0xaa, 0xbb]);

        let log = Log::new(address, topics.clone(), data.clone());

        assert_eq!(log.address, address);
        assert_eq!(log.topics, topics);
        assert_eq!(log.data, data);
    }

    #[test]
    fn test_log_empty_topics() {
        let log = Log::new(Address::ZERO, vec![], Bytes::new());
        assert_eq!(log.topics.len(), 0);
    }

    #[test]
    fn test_log_max_topics() {
        let topics = vec![
            Hash::from([0x01; 32]),
            Hash::from([0x02; 32]),
            Hash::from([0x03; 32]),
            Hash::from([0x04; 32]),
        ];
        let log = Log::new(Address::ZERO, topics.clone(), Bytes::new());
        assert_eq!(log.topics.len(), 4);
    }

    #[test]
    #[should_panic(expected = "maximum 4 topics")]
    fn test_log_too_many_topics() {
        let topics = vec![
            Hash::from([0x01; 32]),
            Hash::from([0x02; 32]),
            Hash::from([0x03; 32]),
            Hash::from([0x04; 32]),
            Hash::from([0x05; 32]),
        ];
        Log::new(Address::ZERO, topics, Bytes::new());
    }

    #[test]
    fn test_log_empty_data() {
        let log = Log::new(Address::ZERO, vec![], Bytes::new());
        assert!(log.data.is_empty());
    }

    #[test]
    fn test_log_rlp_roundtrip() {
        let address = Address::from([0x42; 20]);
        let topics = vec![Hash::from([0x01; 32]), Hash::from([0x02; 32])];
        let data = Bytes::from(vec![0xaa, 0xbb, 0xcc]);

        let log = Log::new(address, topics, data);
        let encoded = log.encode_rlp();
        let decoded = Log::decode_rlp(&encoded).unwrap();

        assert_eq!(log, decoded);
    }

    #[test]
    fn test_log_rlp_empty() {
        let log = Log::new(Address::ZERO, vec![], Bytes::new());
        let encoded = log.encode_rlp();
        let decoded = Log::decode_rlp(&encoded).unwrap();
        assert_eq!(log, decoded);
    }

    // =========================================================================
    // Bloom Filter Tests
    // =========================================================================

    #[test]
    fn test_bloom_new() {
        let bloom = Bloom::new();
        assert_eq!(bloom.as_bytes(), &[0u8; 256]);
    }

    #[test]
    fn test_bloom_add_single() {
        let mut bloom = Bloom::new();
        bloom.add(b"test");

        // Bloom should not be all zeros
        assert_ne!(bloom.as_bytes(), &[0u8; 256]);
    }

    #[test]
    fn test_bloom_add_multiple() {
        let mut bloom = Bloom::new();
        bloom.add(b"test1");
        bloom.add(b"test2");
        bloom.add(b"test3");

        assert!(bloom.contains(b"test1"));
        assert!(bloom.contains(b"test2"));
        assert!(bloom.contains(b"test3"));
    }

    #[test]
    fn test_bloom_contains_positive() {
        let mut bloom = Bloom::new();
        bloom.add(b"hello");
        assert!(bloom.contains(b"hello"));
    }

    #[test]
    fn test_bloom_contains_negative() {
        let mut bloom = Bloom::new();
        bloom.add(b"hello");
        assert!(!bloom.contains(b"world"));
    }

    #[test]
    fn test_bloom_add_log() {
        let mut bloom = Bloom::new();
        let address = Address::from([0x42; 20]);
        let topic = Hash::from([0x01; 32]);
        let log = Log::new(address, vec![topic], Bytes::new());

        bloom.add_log(&log);

        assert!(bloom.contains(address.as_ref()));
        assert!(bloom.contains(topic.as_ref()));
    }

    #[test]
    fn test_bloom_add_log_multiple_topics() {
        let mut bloom = Bloom::new();
        let address = Address::from([0x42; 20]);
        let topics = vec![
            Hash::from([0x01; 32]),
            Hash::from([0x02; 32]),
            Hash::from([0x03; 32]),
        ];
        let log = Log::new(address, topics.clone(), Bytes::new());

        bloom.add_log(&log);

        assert!(bloom.contains(address.as_ref()));
        for topic in &topics {
            assert!(bloom.contains(topic.as_ref()));
        }
    }

    #[test]
    fn test_bloom_combine() {
        let mut bloom1 = Bloom::new();
        bloom1.add(b"test1");

        let mut bloom2 = Bloom::new();
        bloom2.add(b"test2");

        bloom1.combine(&bloom2);

        assert!(bloom1.contains(b"test1"));
        assert!(bloom1.contains(b"test2"));
    }

    #[test]
    fn test_bloom_rlp_roundtrip() {
        let mut bloom = Bloom::new();
        bloom.add(b"test");

        let encoded = bloom.encode_rlp();
        let decoded = Bloom::decode_rlp(&encoded).unwrap();

        assert_eq!(bloom, decoded);
    }

    #[test]
    fn test_bloom_rlp_empty() {
        let bloom = Bloom::new();
        let encoded = bloom.encode_rlp();
        let decoded = Bloom::decode_rlp(&encoded).unwrap();
        assert_eq!(bloom, decoded);
    }

    #[test]
    fn test_bloom_hash_algorithm() {
        // Test that bloom filter sets exactly 3 bits per input
        let mut bloom = Bloom::new();

        // Start with empty bloom
        let initial_ones = bloom.as_bytes().iter()
            .map(|b| b.count_ones())
            .sum::<u32>();
        assert_eq!(initial_ones, 0);

        // Add one item
        bloom.add(b"test");

        // Should have at most 3 bits set (might be fewer if bits overlap)
        let final_ones = bloom.as_bytes().iter()
            .map(|b| b.count_ones())
            .sum::<u32>();
        assert!(final_ones > 0 && final_ones <= 3);
    }

    // =========================================================================
    // Receipt Tests
    // =========================================================================

    #[test]
    fn test_receipt_success() {
        let receipt = TransactionReceipt::new(true, U256::from(21000u64), vec![]);
        assert!(receipt.status);
        assert_eq!(receipt.cumulative_gas_used, U256::from(21000u64));
        assert_eq!(receipt.logs.len(), 0);
    }

    #[test]
    fn test_receipt_failure() {
        let receipt = TransactionReceipt::new(false, U256::from(21000u64), vec![]);
        assert!(!receipt.status);
    }

    #[test]
    fn test_receipt_with_logs() {
        let logs = vec![
            Log::new(Address::from([0x42; 20]), vec![], Bytes::new()),
            Log::new(Address::from([0x43; 20]), vec![], Bytes::new()),
        ];

        let receipt = TransactionReceipt::new(true, U256::from(50000u64), logs.clone());
        assert_eq!(receipt.logs.len(), 2);
        assert_eq!(receipt.logs, logs);
    }

    #[test]
    fn test_receipt_bloom_auto_generated() {
        let address = Address::from([0x42; 20]);
        let topic = Hash::from([0x01; 32]);
        let logs = vec![
            Log::new(address, vec![topic], Bytes::new())
        ];

        let receipt = TransactionReceipt::new(true, U256::from(21000u64), logs);

        // Bloom should contain address and topic
        assert!(receipt.logs_bloom.contains(address.as_ref()));
        assert!(receipt.logs_bloom.contains(topic.as_ref()));
    }

    #[test]
    fn test_receipt_empty_logs() {
        let receipt = TransactionReceipt::new(true, U256::from(21000u64), vec![]);
        assert_eq!(receipt.logs.len(), 0);
        assert_eq!(receipt.logs_bloom, Bloom::new());
    }

    #[test]
    fn test_receipt_rlp_roundtrip_success() {
        let receipt = TransactionReceipt::new(true, U256::from(21000u64), vec![]);
        let encoded = receipt.encode_rlp();
        let decoded = TransactionReceipt::decode_rlp(&encoded).unwrap();
        assert_eq!(receipt, decoded);
    }

    #[test]
    fn test_receipt_rlp_roundtrip_failure() {
        let receipt = TransactionReceipt::new(false, U256::from(21000u64), vec![]);
        let encoded = receipt.encode_rlp();
        let decoded = TransactionReceipt::decode_rlp(&encoded).unwrap();
        assert_eq!(receipt, decoded);
    }

    #[test]
    fn test_receipt_rlp_with_logs() {
        let logs = vec![
            Log::new(
                Address::from([0x42; 20]),
                vec![Hash::from([0x01; 32])],
                Bytes::from(vec![0xaa, 0xbb])
            )
        ];

        let receipt = TransactionReceipt::new(true, U256::from(50000u64), logs);
        let encoded = receipt.encode_rlp();
        let decoded = TransactionReceipt::decode_rlp(&encoded).unwrap();

        assert_eq!(receipt, decoded);
    }

    #[test]
    fn test_receipt_cumulative_gas() {
        let receipt1 = TransactionReceipt::new(true, U256::from(21000u64), vec![]);
        let receipt2 = TransactionReceipt::new(true, U256::from(42000u64), vec![]);
        let receipt3 = TransactionReceipt::new(true, U256::from(63000u64), vec![]);

        assert_eq!(receipt1.cumulative_gas_used, U256::from(21000u64));
        assert_eq!(receipt2.cumulative_gas_used, U256::from(42000u64));
        assert_eq!(receipt3.cumulative_gas_used, U256::from(63000u64));
    }

    // =========================================================================
    // Receipt Root Tests
    // =========================================================================

    #[test]
    fn test_receipts_root_single() {
        let receipts = vec![
            TransactionReceipt::new(true, U256::from(21000u64), vec![])
        ];

        let root = calculate_receipts_root(&receipts);
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn test_receipts_root_multiple() {
        let receipts = vec![
            TransactionReceipt::new(true, U256::from(21000u64), vec![]),
            TransactionReceipt::new(true, U256::from(42000u64), vec![]),
            TransactionReceipt::new(false, U256::from(50000u64), vec![]),
        ];

        let root = calculate_receipts_root(&receipts);
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn test_receipts_root_empty() {
        use crate::state::EMPTY_TRIE_ROOT;

        let receipts: Vec<TransactionReceipt> = vec![];
        let root = calculate_receipts_root(&receipts);

        // Empty trie returns EMPTY_TRIE_ROOT
        assert_eq!(root, EMPTY_TRIE_ROOT);
    }

    #[test]
    fn test_receipts_root_deterministic() {
        let receipts = vec![
            TransactionReceipt::new(true, U256::from(21000u64), vec![]),
            TransactionReceipt::new(true, U256::from(42000u64), vec![]),
        ];

        let root1 = calculate_receipts_root(&receipts);
        let root2 = calculate_receipts_root(&receipts);

        assert_eq!(root1, root2);
    }

    #[test]
    fn test_receipts_root_order_matters() {
        let receipt1 = TransactionReceipt::new(true, U256::from(21000u64), vec![]);
        let receipt2 = TransactionReceipt::new(true, U256::from(42000u64), vec![]);

        let root1 = calculate_receipts_root(&[receipt1.clone(), receipt2.clone()]);
        let root2 = calculate_receipts_root(&[receipt2, receipt1]);

        // Different order should give different root
        assert_ne!(root1, root2);
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_complex_receipt_with_multiple_logs() {
        let logs = vec![
            Log::new(
                Address::from([0x42; 20]),
                vec![Hash::from([0x01; 32]), Hash::from([0x02; 32])],
                Bytes::from(vec![0xaa, 0xbb])
            ),
            Log::new(
                Address::from([0x43; 20]),
                vec![Hash::from([0x03; 32])],
                Bytes::from(vec![0xcc])
            ),
            Log::new(
                Address::from([0x44; 20]),
                vec![],
                Bytes::from(vec![0xdd, 0xee, 0xff])
            ),
        ];

        let receipt = TransactionReceipt::new(true, U256::from(100000u64), logs.clone());

        // Verify logs
        assert_eq!(receipt.logs, logs);

        // Verify bloom contains all addresses and topics
        for log in &logs {
            assert!(receipt.logs_bloom.contains(log.address.as_ref()));
            for topic in &log.topics {
                assert!(receipt.logs_bloom.contains(topic.as_ref()));
            }
        }

        // RLP roundtrip
        let encoded = receipt.encode_rlp();
        let decoded = TransactionReceipt::decode_rlp(&encoded).unwrap();
        assert_eq!(receipt, decoded);
    }

    #[test]
    fn test_receipt_root_with_complex_receipts() {
        let logs1 = vec![
            Log::new(
                Address::from([0x42; 20]),
                vec![Hash::from([0x01; 32])],
                Bytes::new()
            )
        ];

        let logs2 = vec![
            Log::new(
                Address::from([0x43; 20]),
                vec![Hash::from([0x02; 32]), Hash::from([0x03; 32])],
                Bytes::from(vec![0xaa])
            )
        ];

        let receipts = vec![
            TransactionReceipt::new(true, U256::from(21000u64), logs1),
            TransactionReceipt::new(false, U256::from(42000u64), logs2),
            TransactionReceipt::new(true, U256::from(63000u64), vec![]),
        ];

        let root = calculate_receipts_root(&receipts);
        assert_ne!(root, Hash::ZERO);

        // Should be deterministic
        let root2 = calculate_receipts_root(&receipts);
        assert_eq!(root, root2);
    }

    #[test]
    fn test_bloom_false_positive_rate() {
        // This is a probabilistic test to ensure bloom filter works correctly
        let mut bloom = Bloom::new();

        // Add 10 known items
        for i in 0..10 {
            bloom.add(format!("item{i}").as_bytes());
        }

        // Check all known items are found
        for i in 0..10 {
            assert!(bloom.contains(format!("item{i}").as_bytes()));
        }

        // Check that we don't have 100% false positive rate on unknown items
        let mut false_positives = 0;
        for i in 10..110 {
            if bloom.contains(format!("item{i}").as_bytes()) {
                false_positives += 1;
            }
        }

        // False positive rate should be reasonable (not 100%)
        assert!(false_positives < 100);
    }
}
