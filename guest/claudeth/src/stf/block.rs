//! Block processing and validation
//!
//! This module implements the block-level state transition function:
//! 1. Validate block header against parent
//! 2. Execute all transactions in order
//! 3. Track cumulative gas used
//! 4. Generate receipts
//! 5. Compute receipts root
//! 6. Compute state root
//! 7. Validate final block header

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::{boxed::Box, format, string::String, vec::Vec};

use crate::crypto::rlp::encode_u256;
use crate::evm::interpreter::BlockContext;
use crate::state::{State, Trie};
use crate::stf::{
    calculate_receipts_root_with_types, execute_transaction, BlockHashContext, Bloom,
    ExecutionError, TransactionExecutionResult, TransactionReceipt,
};
use crate::types::{BlockHeader, Hash, Transaction, U256};

#[cfg(test)]
use crate::state::EMPTY_TRIE_ROOT;

// =============================================================================
// Block Processing Result
// =============================================================================

/// Result of block processing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockProcessingResult {
    /// Total gas used by all transactions
    pub gas_used: u64,
    /// Receipts for all transactions
    pub receipts: Vec<TransactionReceipt>,
    /// Computed receipts root
    pub receipts_root: Hash,
    /// Computed state root (placeholder - requires full state trie)
    pub state_root: Hash,
    /// Individual transaction execution results
    pub transaction_results: Vec<TransactionExecutionResult>,
}

// =============================================================================
// Block Processing Error
// =============================================================================

/// Errors that can occur during block processing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockProcessingError {
    /// Block header validation failed against parent
    InvalidHeader(String),
    /// Transaction execution failed
    TransactionExecutionError(ExecutionError),
    /// Cumulative gas exceeds block gas limit
    GasLimitExceeded {
        /// Gas limit in block header
        gas_limit: u64,
        /// Cumulative gas used by transactions
        gas_used: u64,
        /// Partial transaction results (for debugging)
        transaction_results: Vec<TransactionExecutionResult>,
    },
    /// Computed receipts root doesn't match header
    ReceiptsRootMismatch {
        /// Expected receipts root from header
        expected: Hash,
        /// Computed receipts root
        computed: Hash,
        /// Partial transaction results (for debugging)
        transaction_results: Vec<TransactionExecutionResult>,
    },
    /// Computed state root doesn't match header
    StateRootMismatch {
        /// Expected state root from header
        expected: Hash,
        /// Computed state root
        computed: Hash,
        /// Partial transaction results (for debugging)
        transaction_results: Vec<TransactionExecutionResult>,
    },
    /// Gas used doesn't match header
    GasUsedMismatch {
        /// Expected gas used from header
        expected: u64,
        /// Computed gas used
        computed: u64,
        /// Partial transaction results (for debugging)
        transaction_results: Vec<TransactionExecutionResult>,
    },
    /// Transactions root doesn't match header
    TransactionsRootMismatch {
        /// Expected transactions root from header
        expected: Hash,
        /// Computed transactions root
        computed: Hash,
        /// Partial transaction results (for debugging)
        transaction_results: Vec<TransactionExecutionResult>,
    },
    /// Logs bloom doesn't match header
    LogsBloomMismatch {
        /// Expected logs bloom from header
        expected: Box<[u8; 256]>,
        /// Computed logs bloom
        computed: Box<[u8; 256]>,
        /// Partial transaction results (for debugging)
        transaction_results: Vec<TransactionExecutionResult>,
    },
}

impl From<ExecutionError> for BlockProcessingError {
    fn from(err: ExecutionError) -> Self {
        BlockProcessingError::TransactionExecutionError(err)
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Computes the transactions root from a list of transactions
///
/// The transactions root is the root of a Merkle Patricia Trie where:
/// - Key: RLP-encoded transaction index (0, 1, 2, ...)
/// - Value: RLP-encoded transaction
///
/// # Arguments
/// * `transactions` - The transactions to include in the trie
///
/// # Returns
/// The computed transactions root hash
fn calculate_transactions_root(transactions: &[Transaction]) -> Hash {
    let mut trie = Trie::new();
    for (index, tx) in transactions.iter().enumerate() {
        let key = encode_u256(&U256::from(index as u64));
        let value = tx.encode_rlp();
        trie.insert(&key, value);
    }
    trie.compute_root()
}

/// Computes the logs bloom from a list of receipts
///
/// The logs bloom is the bitwise OR of all individual receipt blooms.
///
/// # Arguments
/// * `receipts` - The receipts to extract blooms from
///
/// # Returns
/// The combined logs bloom as a 256-byte array
fn calculate_logs_bloom(receipts: &[TransactionReceipt]) -> [u8; 256] {
    if receipts.is_empty() {
        return [0u8; 256];
    }

    let mut combined_bloom = Bloom::new();
    for receipt in receipts {
        combined_bloom.combine(&receipt.logs_bloom);
    }
    *combined_bloom.as_bytes()
}

/// Computes the state root from the current state
///
/// The state root is the root of a Merkle Patricia Trie where:
/// - Key: Address (20 bytes)
/// - Value: RLP-encoded Account
///
/// # Arguments
/// * `state` - The state to compute the root from
///
/// # Returns
/// The computed state root hash
///
/// # Note
/// This requires the State trait to provide access to all accounts.
/// For InMemoryState, we need to iterate over all accounts.
fn calculate_state_root<S: State>(state: &S) -> Hash {
    state.compute_state_root()
}

// =============================================================================
// Block Processor
// =============================================================================

/// Processes a block by executing all transactions and validating the result
///
/// # Arguments
/// * `block` - The block header to process
/// * `parent` - The parent block header
/// * `transactions` - The transactions in the block
/// * `state` - The current state (will be mutated)
/// * `chain_id` - The expected chain ID
/// * `recent_block_hashes` - Recent block hashes for BLOCKHASH lookups (up to 256)
///
/// # Returns
/// The block processing result with gas used, receipts, and roots
///
/// # Errors
/// Returns error if:
/// - Block header is invalid against parent
/// - Any transaction execution fails
/// - Cumulative gas exceeds block gas limit
/// - Computed roots don't match header
///
/// # Example
/// ```
/// use claudeth::stf::process_block;
/// use claudeth::types::{BlockHeader, Transaction, U256, EMPTY_OMMERS_HASH};
/// use claudeth::state::{InMemoryState, EMPTY_TRIE_ROOT};
/// use claudeth::types::Address;
///
/// let parent = BlockHeader {
///     parent_hash: Default::default(),
///     ommers_hash: EMPTY_OMMERS_HASH,
///     coinbase: Address::ZERO,
///     state_root: EMPTY_TRIE_ROOT,
///     transactions_root: EMPTY_TRIE_ROOT,
///     receipts_root: EMPTY_TRIE_ROOT,
///     logs_bloom: [0u8; 256],
///     difficulty: U256::ZERO,
///     number: 0,
///     gas_limit: 30_000_000,
///     gas_used: 0,
///     timestamp: 1000,
///     extra_data: Default::default(),
///     mix_hash: Default::default(),
///     nonce: 0,
///     base_fee_per_gas: Some(1_000_000_000),
///     withdrawals_root: None,
///     blob_gas_used: None,
///     excess_blob_gas: None,
///     parent_beacon_block_root: None,
/// };
///
/// let mut block = parent.clone();
/// block.number = 1;
/// block.timestamp = 2000;
/// block.parent_hash = parent.compute_hash();
///
/// let transactions = vec![];
/// let mut state = InMemoryState::new();
///
/// // Process empty block (should succeed)
/// let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE, &[]);
/// assert!(result.is_ok());
/// ```
pub fn process_block<S: State + Clone>(
    block: &BlockHeader,
    parent: &BlockHeader,
    transactions: &[Transaction],
    state: &mut S,
    chain_id: U256,
    recent_block_hashes: &[(u64, Hash)],
) -> Result<BlockProcessingResult, BlockProcessingError> {
    // Step 1: Validate block header against parent
    block
        .validate_against_parent(parent)
        .map_err(|e| BlockProcessingError::InvalidHeader(format!("{e}")))?;

    // Step 2: Create block context for EVM execution
    let block_ctx = BlockContext {
        number: U256::from_u64(block.number),
        timestamp: U256::from_u64(block.timestamp),
        coinbase: block.coinbase,
        difficulty: block.difficulty,
        gas_limit: U256::from_u64(block.gas_limit),
        chain_id,
        base_fee: U256::from_u64(block.base_fee_per_gas.unwrap_or(0)),
    };

    // Step 3: Execute all transactions
    let parent_hash = parent.compute_hash();
    let block_hash_ctx = BlockHashContext::new(parent_hash, recent_block_hashes.to_vec());
    let mut cumulative_gas_used = 0u64;
    let mut receipts = Vec::with_capacity(transactions.len());
    let mut transaction_results = Vec::with_capacity(transactions.len());

    for tx in transactions {
        // Execute transaction
        let mut exec_result = execute_transaction(
            tx,
            state,
            &block_ctx,
            &block_hash_ctx,
            cumulative_gas_used,
            chain_id,
            U256::from_u64(block.gas_limit),
        )?;

        // Update cumulative gas
        cumulative_gas_used += exec_result.gas_used;

        // Check gas limit
        if cumulative_gas_used > block.gas_limit {
            return Err(BlockProcessingError::GasLimitExceeded {
                gas_limit: block.gas_limit,
                gas_used: cumulative_gas_used,
                transaction_results: transaction_results.clone(),
            });
        }

        // Update cumulative gas in result
        exec_result.cumulative_gas_used = cumulative_gas_used;

        // Generate receipt
        let receipt = exec_result.to_receipt();
        receipts.push(receipt);
        transaction_results.push(exec_result);
    }

    // Step 4: Compute roots and bloom
    let transactions_refs: Vec<&Transaction> = transactions.iter().collect();
    let receipts_root = calculate_receipts_root_with_types(&receipts, &transactions_refs);
    let transactions_root = calculate_transactions_root(transactions);
    let logs_bloom = calculate_logs_bloom(&receipts);
    let state_root = calculate_state_root(state);

    // Step 5: Validate results match block header
    // Validate gas used
    if cumulative_gas_used != block.gas_used {
        return Err(BlockProcessingError::GasUsedMismatch {
            expected: block.gas_used,
            computed: cumulative_gas_used,
            transaction_results: transaction_results.clone(),
        });
    }

    // Validate receipts root
    if receipts_root != block.receipts_root {
        return Err(BlockProcessingError::ReceiptsRootMismatch {
            expected: block.receipts_root,
            computed: receipts_root,
            transaction_results: transaction_results.clone(),
        });
    }

    // Validate transactions root
    if transactions_root != block.transactions_root {
        return Err(BlockProcessingError::TransactionsRootMismatch {
            expected: block.transactions_root,
            computed: transactions_root,
            transaction_results: transaction_results.clone(),
        });
    }

    // Validate logs bloom
    if logs_bloom != block.logs_bloom {
        return Err(BlockProcessingError::LogsBloomMismatch {
            expected: Box::new(block.logs_bloom),
            computed: Box::new(logs_bloom),
            transaction_results: transaction_results.clone(),
        });
    }

    // Validate state root
    if state_root != block.state_root {
        return Err(BlockProcessingError::StateRootMismatch {
            expected: block.state_root,
            computed: state_root,
            transaction_results: transaction_results.clone(),
        });
    }

    Ok(BlockProcessingResult {
        gas_used: cumulative_gas_used,
        receipts,
        receipts_root,
        state_root,
        transaction_results,
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::InMemoryState;
    use crate::types::{Address, Bytes, Hash, EMPTY_OMMERS_HASH};

    fn create_test_parent() -> BlockHeader {
        BlockHeader {
            parent_hash: Hash::ZERO,
            ommers_hash: EMPTY_OMMERS_HASH,
            coinbase: Address::ZERO,
            state_root: EMPTY_TRIE_ROOT,
            transactions_root: EMPTY_TRIE_ROOT,
            receipts_root: EMPTY_TRIE_ROOT,
            logs_bloom: [0u8; 256],
            difficulty: U256::ZERO,
            number: 100,
            gas_limit: 30_000_000,
            gas_used: 0,
            timestamp: 1000,
            extra_data: Bytes::new(),
            mix_hash: Hash::ZERO,
            nonce: 0,
            base_fee_per_gas: Some(1_000_000_000),
            withdrawals_root: None,
            blob_gas_used: None,
            excess_blob_gas: None,
            parent_beacon_block_root: None,
        }
    }

    fn create_test_block(parent: &BlockHeader) -> BlockHeader {
        let mut block = parent.clone();
        block.number = parent.number + 1;
        block.timestamp = parent.timestamp + 12;
        block.parent_hash = parent.compute_hash();
        block
    }

    #[test]
    fn test_process_empty_block() {
        let parent = create_test_parent();
        let block = create_test_block(&parent);
        let transactions = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE, &[]);
        if let Err(ref e) = result {
            eprintln!("Error processing empty block: {e:?}");
        }
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.gas_used, 0);
        assert_eq!(result.receipts.len(), 0);
        assert_eq!(result.transaction_results.len(), 0);
    }

    #[test]
    fn test_process_block_invalid_parent_hash() {
        let parent = create_test_parent();
        let mut block = create_test_block(&parent);
        block.parent_hash = Hash::ZERO; // Wrong parent hash
        let transactions = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE, &[]);
        assert!(matches!(
            result,
            Err(BlockProcessingError::InvalidHeader(_))
        ));
    }

    #[test]
    fn test_process_block_invalid_number() {
        let parent = create_test_parent();
        let mut block = create_test_block(&parent);
        block.number = parent.number + 2; // Wrong number (should be parent + 1)
        let transactions = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE, &[]);
        assert!(matches!(
            result,
            Err(BlockProcessingError::InvalidHeader(_))
        ));
    }

    #[test]
    fn test_process_block_invalid_timestamp() {
        let parent = create_test_parent();
        let mut block = create_test_block(&parent);
        block.timestamp = parent.timestamp; // Invalid: must be > parent timestamp
        let transactions = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE, &[]);
        assert!(matches!(
            result,
            Err(BlockProcessingError::InvalidHeader(_))
        ));
    }

    #[test]
    fn test_process_block_gas_limit_too_high() {
        let parent = create_test_parent();
        let mut block = create_test_block(&parent);
        // Gas limit increase > parent/1024
        block.gas_limit = parent.gas_limit + (parent.gas_limit / 1024) + 1;
        let transactions = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE, &[]);
        assert!(matches!(
            result,
            Err(BlockProcessingError::InvalidHeader(_))
        ));
    }

    #[test]
    fn test_process_block_gas_limit_too_low() {
        let parent = create_test_parent();
        let mut block = create_test_block(&parent);
        // Gas limit decrease > parent/1024
        block.gas_limit = parent.gas_limit - (parent.gas_limit / 1024) - 1;
        let transactions = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE, &[]);
        assert!(matches!(
            result,
            Err(BlockProcessingError::InvalidHeader(_))
        ));
    }

    #[test]
    fn test_process_block_gas_used_mismatch() {
        let parent = create_test_parent();
        let mut block = create_test_block(&parent);
        block.gas_used = 1000; // Should be 0 for empty block
        let transactions = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE, &[]);
        assert!(matches!(
            result,
            Err(BlockProcessingError::GasUsedMismatch { .. })
        ));
    }

    #[test]
    fn test_process_block_receipts_root_mismatch() {
        let parent = create_test_parent();
        let mut block = create_test_block(&parent);

        // First compute the correct receipts root for an empty block
        let transactions = vec![];
        let mut state = InMemoryState::new();
        let correct_root = calculate_receipts_root_with_types(&[], &[]);

        // Now set a DIFFERENT receipts root that is definitely wrong
        // We'll use a non-zero hash that's different from the correct root
        block.receipts_root = if correct_root == Hash::ZERO {
            Hash::from([1u8; 32])
        } else {
            Hash::ZERO
        };

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE, &[]);
        // We expect this to fail since we set an incorrect receipts root
        assert!(matches!(
            result,
            Err(BlockProcessingError::ReceiptsRootMismatch { .. })
        ));
    }

    #[test]
    fn test_process_block_valid_gas_limit_boundaries() {
        let parent = create_test_parent();

        // Test maximum valid increase
        let mut block = create_test_block(&parent);
        block.gas_limit = parent.gas_limit + (parent.gas_limit / 1024);
        let transactions = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE, &[]);
        assert!(result.is_ok());

        // Test maximum valid decrease
        let mut block = create_test_block(&parent);
        block.gas_limit = parent.gas_limit - (parent.gas_limit / 1024);
        let transactions = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_block_transactions_root_mismatch() {
        let parent = create_test_parent();
        let mut block = create_test_block(&parent);

        // Set transactions_root to a non-zero value that won't match
        block.transactions_root = Hash::from([1u8; 32]);

        let transactions = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE, &[]);
        assert!(matches!(
            result,
            Err(BlockProcessingError::TransactionsRootMismatch { .. })
        ));
    }

    #[test]
    fn test_process_block_logs_bloom_mismatch() {
        let parent = create_test_parent();
        let mut block = create_test_block(&parent);

        // Set logs_bloom to a non-zero value that won't match (empty block has all zeros)
        block.logs_bloom = [1u8; 256];

        let transactions = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE, &[]);
        assert!(matches!(
            result,
            Err(BlockProcessingError::LogsBloomMismatch { .. })
        ));
    }

    #[test]
    fn test_calculate_transactions_root_empty() {
        let transactions = vec![];
        let root = calculate_transactions_root(&transactions);
        assert_eq!(root, EMPTY_TRIE_ROOT);
    }

    #[test]
    fn test_calculate_logs_bloom_empty() {
        let receipts = vec![];
        let bloom = calculate_logs_bloom(&receipts);
        assert_eq!(bloom, [0u8; 256]);
    }

    #[test]
    fn test_calculate_logs_bloom_combines_multiple_receipts() {
        use crate::stf::Log;

        // Create two receipts with logs (bloom is auto-generated from logs)
        let log1 = Log::new(Address::from([1u8; 20]), vec![Hash::from([2u8; 32])], Bytes::new());
        let receipt1 =
            TransactionReceipt::new(true, U256::from(100u64), vec![log1.clone()]);

        let log2 = Log::new(Address::from([3u8; 20]), vec![Hash::from([4u8; 32])], Bytes::new());
        let receipt2 = TransactionReceipt::new(true, U256::from(200u64), vec![log2.clone()]);

        let receipts = vec![receipt1.clone(), receipt2.clone()];
        let combined_bloom_bytes = calculate_logs_bloom(&receipts);

        // Manually create expected bloom by combining both receipt blooms
        let mut expected_bloom = receipt1.logs_bloom;
        expected_bloom.combine(&receipt2.logs_bloom);

        assert_eq!(combined_bloom_bytes, *expected_bloom.as_bytes());
    }
}
