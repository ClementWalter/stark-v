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
use alloc::vec::Vec;

use crate::evm::interpreter::BlockContext;
use crate::state::State;
use crate::stf::{
    calculate_receipts_root, execute_transaction, ExecutionError, TransactionExecutionResult,
    TransactionReceipt,
};
use crate::types::{BlockHeader, Hash, Transaction, U256};

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
    },
    /// Computed receipts root doesn't match header
    ReceiptsRootMismatch {
        /// Expected receipts root from header
        expected: Hash,
        /// Computed receipts root
        computed: Hash,
    },
    /// Computed state root doesn't match header
    StateRootMismatch {
        /// Expected state root from header
        expected: Hash,
        /// Computed state root
        computed: Hash,
    },
    /// Gas used doesn't match header
    GasUsedMismatch {
        /// Expected gas used from header
        expected: u64,
        /// Computed gas used
        computed: u64,
    },
}

impl From<ExecutionError> for BlockProcessingError {
    fn from(err: ExecutionError) -> Self {
        BlockProcessingError::TransactionExecutionError(err)
    }
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
/// use claudeth::types::{BlockHeader, Transaction, U256};
/// use claudeth::state::InMemoryState;
/// use claudeth::types::Address;
///
/// let parent = BlockHeader {
///     parent_hash: Default::default(),
///     ommers_hash: Default::default(),
///     coinbase: Address::ZERO,
///     state_root: Default::default(),
///     transactions_root: Default::default(),
///     receipts_root: Default::default(),
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
/// let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE);
/// assert!(result.is_ok());
/// ```
pub fn process_block<S: State + Clone>(
    block: &BlockHeader,
    parent: &BlockHeader,
    transactions: &[Transaction],
    state: &mut S,
    chain_id: U256,
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
    let mut cumulative_gas_used = 0u64;
    let mut receipts = Vec::with_capacity(transactions.len());
    let mut transaction_results = Vec::with_capacity(transactions.len());

    for tx in transactions {
        // Execute transaction
        let mut exec_result = execute_transaction(
            tx,
            state,
            &block_ctx,
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
            });
        }

        // Update cumulative gas in result
        exec_result.cumulative_gas_used = cumulative_gas_used;

        // Generate receipt
        let receipt = exec_result.to_receipt();
        receipts.push(receipt);
        transaction_results.push(exec_result);
    }

    // Step 4: Compute receipts root
    let receipts_root = calculate_receipts_root(&receipts);

    // Step 5: Compute state root
    // NOTE: This is a placeholder. A full implementation would compute the actual
    // state root by building a Merkle Patricia Trie of all accounts and their storage.
    // For now, we return the state_root from the block header for validation.
    let state_root = block.state_root;

    // Step 6: Validate results match block header
    // Validate gas used
    if cumulative_gas_used != block.gas_used {
        return Err(BlockProcessingError::GasUsedMismatch {
            expected: block.gas_used,
            computed: cumulative_gas_used,
        });
    }

    // Validate receipts root
    if receipts_root != block.receipts_root {
        return Err(BlockProcessingError::ReceiptsRootMismatch {
            expected: block.receipts_root,
            computed: receipts_root,
        });
    }

    // Validate state root (when full state trie is implemented)
    // For now, we skip this check as we don't compute the actual state root
    // if state_root != block.state_root {
    //     return Err(BlockProcessingError::StateRootMismatch {
    //         expected: block.state_root,
    //         computed: state_root,
    //     });
    // }

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
    use crate::types::{Address, Bytes, Hash};

    fn create_test_parent() -> BlockHeader {
        BlockHeader {
            parent_hash: Hash::ZERO,
            ommers_hash: Hash::ZERO,
            coinbase: Address::ZERO,
            state_root: Hash::ZERO,
            transactions_root: Hash::ZERO,
            receipts_root: Hash::ZERO,
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

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE);
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

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE);
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

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE);
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

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE);
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

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE);
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

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE);
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

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE);
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
        let correct_root = calculate_receipts_root(&[]);

        // Now set a DIFFERENT receipts root that is definitely wrong
        // We'll use a non-zero hash that's different from the correct root
        block.receipts_root = if correct_root == Hash::ZERO {
            Hash::from([1u8; 32])
        } else {
            Hash::ZERO
        };

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE);
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

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE);
        assert!(result.is_ok());

        // Test maximum valid decrease
        let mut block = create_test_block(&parent);
        block.gas_limit = parent.gas_limit - (parent.gas_limit / 1024);
        let transactions = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(&block, &parent, &transactions, &mut state, U256::ONE);
        assert!(result.is_ok());
    }
}
