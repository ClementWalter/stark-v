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
use crate::crypto::{keccak256, rlp};
use crate::evm::host::RecursiveHost;
use crate::evm::interpreter::{BlockContext, CallContext, Evm, TxContext};
use crate::state::{EMPTY_TRIE_ROOT, State, Trie, bytes_to_nibbles, encode_compact_path};
use crate::stf::transaction::{MAX_BLOB_GAS_PER_BLOCK, blob_gas_used};
use crate::stf::{
    Bloom, ExecutionError, TransactionExecutionResult, TransactionReceipt,
    calculate_receipts_root_with_types, execute_transaction,
};
use crate::types::{Address, BlockHeader, Hash, Transaction, U256, Withdrawal};

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
    /// Blob gas used exceeds the block blob gas limit
    BlobGasLimitExceeded {
        /// Block blob gas limit
        blob_gas_limit: u64,
        /// Cumulative blob gas used
        blob_gas_used: u64,
        /// Partial transaction results (for debugging)
        transaction_results: Vec<TransactionExecutionResult>,
    },
    /// Blob gas used doesn't match header
    BlobGasUsedMismatch {
        /// Expected blob gas used from header
        expected: u64,
        /// Computed blob gas used
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
    /// Withdrawals were provided but the header has no withdrawals root
    UnexpectedWithdrawals {
        /// Number of withdrawals provided
        count: usize,
        /// Partial transaction results (for debugging)
        transaction_results: Vec<TransactionExecutionResult>,
    },
    /// Computed withdrawals root doesn't match header
    WithdrawalsRootMismatch {
        /// Expected withdrawals root from header
        expected: Hash,
        /// Computed withdrawals root
        computed: Hash,
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

fn evm_difficulty_or_prevrandao(block: &BlockHeader) -> U256 {
    if block.difficulty.is_zero() {
        // Why: post-merge headers carry PREVRANDAO in the legacy mix-hash
        // field, and opcode 0x44 must expose that value instead of 0.
        U256::from_be_bytes(*block.mix_hash.as_bytes())
    } else {
        block.difficulty
    }
}

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

/// Computes the withdrawals root from a list of withdrawals.
fn calculate_withdrawals_root(withdrawals: &[Withdrawal]) -> Hash {
    if withdrawals.is_empty() {
        return EMPTY_TRIE_ROOT;
    }

    // Why: execution-specs embeds child node RLP when it is <32 bytes; always
    // hashing children changes trie structure and diverges from fixture roots.
    #[derive(Clone)]
    struct WithdrawalTrieEntry {
        key_nibbles: Vec<u8>,
        value_rlp: Vec<u8>,
    }

    enum TrieNodeRef {
        InlineRlp(Vec<u8>),
        Hashed(Hash),
    }

    impl TrieNodeRef {
        fn into_rlp_item(self) -> Vec<u8> {
            match self {
                TrieNodeRef::InlineRlp(encoded) => encoded,
                TrieNodeRef::Hashed(hash) => rlp::encode_hash(&hash),
            }
        }
    }

    fn to_node_ref(encoded: Vec<u8>) -> TrieNodeRef {
        if encoded.len() < 32 {
            TrieNodeRef::InlineRlp(encoded)
        } else {
            TrieNodeRef::Hashed(keccak256(&encoded))
        }
    }

    fn shared_prefix_len(entries: &[WithdrawalTrieEntry], level: usize) -> usize {
        if entries.len() < 2 {
            return 0;
        }

        let min_remaining = entries
            .iter()
            .map(|entry| entry.key_nibbles.len().saturating_sub(level))
            .min()
            .unwrap_or(0);

        let mut prefix_len = 0usize;
        while prefix_len < min_remaining {
            let candidate = entries[0].key_nibbles[level + prefix_len];
            if entries
                .iter()
                .all(|entry| entry.key_nibbles[level + prefix_len] == candidate)
            {
                prefix_len += 1;
            } else {
                break;
            }
        }
        prefix_len
    }

    fn encode_node(entries: &[WithdrawalTrieEntry], level: usize) -> TrieNodeRef {
        if entries.len() == 1 {
            let entry = &entries[0];
            let compact_path = encode_compact_path(&entry.key_nibbles[level..], true);
            let leaf_items = vec![
                rlp::encode_bytes(&compact_path),
                rlp::encode_bytes(&entry.value_rlp),
            ];
            return to_node_ref(rlp::encode_list(&leaf_items));
        }

        let prefix_len = shared_prefix_len(entries, level);
        if prefix_len > 0 {
            let child_ref = encode_node(entries, level + prefix_len);
            let compact_path =
                encode_compact_path(&entries[0].key_nibbles[level..level + prefix_len], false);
            let extension_items = vec![rlp::encode_bytes(&compact_path), child_ref.into_rlp_item()];
            return to_node_ref(rlp::encode_list(&extension_items));
        }

        let mut children: [Vec<WithdrawalTrieEntry>; 16] = core::array::from_fn(|_| Vec::new());
        let mut branch_value: Option<Vec<u8>> = None;

        for entry in entries {
            if level == entry.key_nibbles.len() {
                branch_value = Some(entry.value_rlp.clone());
                continue;
            }

            let child_index = usize::from(entry.key_nibbles[level]);
            children[child_index].push(entry.clone());
        }

        let mut branch_items = Vec::with_capacity(17);
        for child_entries in &children {
            if child_entries.is_empty() {
                branch_items.push(rlp::encode_bytes(&[]));
            } else {
                branch_items.push(encode_node(child_entries, level + 1).into_rlp_item());
            }
        }
        branch_items.push(match branch_value {
            Some(value) => rlp::encode_bytes(&value),
            None => rlp::encode_bytes(&[]),
        });

        to_node_ref(rlp::encode_list(&branch_items))
    }

    let entries: Vec<WithdrawalTrieEntry> = withdrawals
        .iter()
        .enumerate()
        .map(|(index, withdrawal)| WithdrawalTrieEntry {
            key_nibbles: bytes_to_nibbles(&encode_u256(&U256::from(index as u64))),
            value_rlp: withdrawal.encode_rlp(),
        })
        .collect();

    match encode_node(&entries, 0) {
        // Why: the block header root is always a 32-byte hash, even if the
        // internal root node itself is short enough to be inlined.
        TrieNodeRef::InlineRlp(encoded) => keccak256(&encoded),
        TrieNodeRef::Hashed(hash) => hash,
    }
}

/// Applies withdrawals to the state (credits balances).
fn apply_withdrawals<S: State>(state: &mut S, withdrawals: &[Withdrawal]) {
    for withdrawal in withdrawals {
        let balance = state.get_balance(&withdrawal.address);
        state.set_balance(&withdrawal.address, balance + withdrawal.amount_wei());
    }
}

// =============================================================================
// EIP-4788: Beacon Block Root System Call
// =============================================================================

/// The address of the beacon roots contract (EIP-4788).
/// `0x000F3df6D732807Ef1319fB7B8bB8522d0Beac02`
const BEACON_ROOTS_ADDRESS: Address = Address::new([
    0x00, 0x0f, 0x3d, 0xf6, 0xd7, 0x32, 0x80, 0x7e, 0xf1, 0x31, 0x9f, 0xb7, 0xb8, 0xbb, 0x85, 0x22,
    0xd0, 0xbe, 0xac, 0x02,
]);

/// The system address used as caller for EIP-4788 system calls.
/// `0xfffffffffffffffffffffffffffffffffffffffe`
const SYSTEM_ADDRESS: Address = Address::new([
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xfe,
]);

/// Gas limit for the beacon root system call (30 million).
const SYSTEM_CALL_GAS_LIMIT: u64 = 30_000_000;

/// Executes the EIP-4788 beacon root system call.
///
/// At the start of processing any execution block where `parent_beacon_block_root`
/// is present, call the beacon roots contract as `SYSTEM_ADDRESS` with the
/// 32-byte `parent_beacon_block_root` as input. The call does not count against
/// the block's gas limit. If no code exists at the beacon roots address, the
/// call fails silently.
fn apply_beacon_root_system_call<S: State + Clone>(
    state: &mut S,
    block: &BlockHeader,
    block_ctx: &BlockContext,
    parent_hash: Hash,
    block_hashes: &[Hash],
) {
    let parent_beacon_block_root = match block.parent_beacon_block_root {
        Some(root) => root,
        None => return,
    };

    // If no code exists at the beacon roots address, fail silently
    let code = state.get_code(&BEACON_ROOTS_ADDRESS).to_vec();
    if code.is_empty() {
        return;
    }

    // Build the 32-byte calldata: the parent beacon block root
    let calldata = parent_beacon_block_root.as_bytes().to_vec();

    // Set up EVM execution context with SYSTEM_ADDRESS as caller
    let call_ctx = CallContext {
        address: BEACON_ROOTS_ADDRESS,
        caller: SYSTEM_ADDRESS,
        call_value: U256::ZERO,
        call_data: calldata,
    };

    let tx_ctx = TxContext {
        origin: SYSTEM_ADDRESS,
        gas_price: U256::ZERO,
        blob_versioned_hashes: Vec::new(),
    };

    let host = RecursiveHost::new()
        .with_block_context(block_ctx.clone())
        .with_parent_hash(parent_hash)
        .with_recent_block_hashes(block_hashes)
        .with_tx_context(tx_ctx.clone());

    let mut evm = Evm::new(code, SYSTEM_CALL_GAS_LIMIT, state.clone(), host)
        .with_block_context(block_ctx.clone())
        .with_tx_context(tx_ctx)
        .with_call_context(call_ctx);

    // Execute the system call and apply state changes on success.
    // On failure, state changes are discarded (fail silently).
    if let Ok(result) = evm.run()
        && result.success
    {
        *state = evm.into_state();
        state.clear_original_storage();
    }
}

// =============================================================================
// EIP-2935: Historical Block Hashes System Call
// =============================================================================

/// The address of the historical block hashes contract (EIP-2935).
/// `0x0000F90827F1C53a10cb7A02335B175320002935`
const HISTORY_STORAGE_ADDRESS: Address = Address::new([
    0x00, 0x00, 0xf9, 0x08, 0x27, 0xf1, 0xc5, 0x3a, 0x10, 0xcb, 0x7a, 0x02, 0x33, 0x5b, 0x17, 0x53,
    0x20, 0x00, 0x29, 0x35,
]);

/// Executes the EIP-2935 historical block hashes system call.
///
/// At the start of processing any execution block, call the history storage
/// contract as `SYSTEM_ADDRESS` with the 32-byte parent block hash as input.
/// The call does not count against the block's gas limit. If no code exists at
/// the history storage address, the call fails silently.
fn apply_history_storage_system_call<S: State + Clone>(
    state: &mut S,
    block_ctx: &BlockContext,
    parent_hash: Hash,
    block_hashes: &[Hash],
) {
    let code = state.get_code(&HISTORY_STORAGE_ADDRESS).to_vec();
    if code.is_empty() {
        return;
    }

    let calldata = parent_hash.as_bytes().to_vec();

    let call_ctx = CallContext {
        address: HISTORY_STORAGE_ADDRESS,
        caller: SYSTEM_ADDRESS,
        call_value: U256::ZERO,
        call_data: calldata,
    };

    let tx_ctx = TxContext {
        origin: SYSTEM_ADDRESS,
        gas_price: U256::ZERO,
        blob_versioned_hashes: Vec::new(),
    };

    let host = RecursiveHost::new()
        .with_block_context(block_ctx.clone())
        .with_parent_hash(parent_hash)
        .with_recent_block_hashes(block_hashes)
        .with_tx_context(tx_ctx.clone());

    let mut evm = Evm::new(code, SYSTEM_CALL_GAS_LIMIT, state.clone(), host)
        .with_block_context(block_ctx.clone())
        .with_tx_context(tx_ctx)
        .with_call_context(call_ctx);

    if let Ok(result) = evm.run()
        && result.success
    {
        *state = evm.into_state();
        state.clear_original_storage();
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
/// * `withdrawals` - The withdrawals in the block
/// * `block_hashes` - Recent block hashes (oldest -> newest) for BLOCKHASH
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
///     gas_used: 15_000_000,
///     timestamp: 1000,
///     extra_data: Default::default(),
///     mix_hash: Default::default(),
///     nonce: 0,
///     base_fee_per_gas: Some(1_000_000_000),
///     withdrawals_root: None,
///     blob_gas_used: None,
///     excess_blob_gas: None,
///     parent_beacon_block_root: None,
///     requests_hash: None,
/// };
///
/// let mut block = parent.clone();
/// block.number = 1;
/// block.timestamp = 2000;
/// block.parent_hash = parent.compute_hash();
/// block.gas_used = 0;
///
/// let transactions = vec![];
/// let withdrawals = vec![];
/// let mut state = InMemoryState::new();
///
/// // Process empty block (should succeed)
/// let result =
///     process_block(
///         &block,
///         &parent,
///         &transactions,
///         &withdrawals,
///         &[],
///         &mut state,
///         U256::ONE,
///     );
/// assert!(result.is_ok());
/// ```
pub fn process_block<S: State + Clone>(
    block: &BlockHeader,
    parent: &BlockHeader,
    transactions: &[Transaction],
    withdrawals: &[Withdrawal],
    block_hashes: &[Hash],
    state: &mut S,
    chain_id: U256,
) -> Result<BlockProcessingResult, BlockProcessingError> {
    // Step 1: Validate block header fields and validate against parent
    block
        .validate()
        .map_err(|e| BlockProcessingError::InvalidHeader(format!("{e}")))?;
    block
        .validate_against_parent(parent)
        .map_err(|e| BlockProcessingError::InvalidHeader(format!("{e}")))?;

    // Step 2: Create block context for EVM execution
    let block_ctx = BlockContext {
        number: U256::from_u64(block.number),
        timestamp: U256::from_u64(block.timestamp),
        coinbase: block.coinbase,
        difficulty: evm_difficulty_or_prevrandao(block),
        gas_limit: U256::from_u64(block.gas_limit),
        chain_id,
        base_fee: U256::from_u64(block.base_fee_per_gas.unwrap_or(0)),
        excess_blob_gas: block.excess_blob_gas.map(U256::from_u64),
        // Why: Prague fixtures enable `requests_hash`, and execution-specs
        // treat precompiles `0x01..0x11` as transaction-warm at that fork.
        max_precompile_address: if block.requests_hash.is_some() { 0x11 } else { 0x0a },
    };

    // Step 3: Apply EIP-4788 beacon root system call (before executing transactions)
    let parent_hash = parent.compute_hash();
    apply_beacon_root_system_call(state, block, &block_ctx, parent_hash, block_hashes);

    // Step 3b: Apply EIP-2935 historical block hashes system call
    apply_history_storage_system_call(state, &block_ctx, parent_hash, block_hashes);

    // Step 4: Execute all transactions
    let mut cumulative_gas_used = 0u64;
    let mut cumulative_blob_gas_used = 0u64;
    let mut receipts = Vec::with_capacity(transactions.len());
    let mut transaction_results = Vec::with_capacity(transactions.len());

    let tx_exec_ctx = crate::stf::executor::TransactionExecutionContext {
        block_ctx: &block_ctx,
        parent_hash,
        block_hashes,
        chain_id,
        block_gas_limit: U256::from_u64(block.gas_limit),
        // Why: EIP-7623 activates from Prague onward. In execution-spec
        // fixtures, Prague headers include requests_hash.
        enforce_calldata_floor: block.requests_hash.is_some(),
    };

    for tx in transactions {
        let tx_blob_gas_used = blob_gas_used(tx);
        if block.blob_gas_used.is_some() {
            let updated_blob_gas_used = cumulative_blob_gas_used.saturating_add(tx_blob_gas_used);
            if updated_blob_gas_used > MAX_BLOB_GAS_PER_BLOCK {
                return Err(BlockProcessingError::BlobGasLimitExceeded {
                    blob_gas_limit: MAX_BLOB_GAS_PER_BLOCK,
                    blob_gas_used: updated_blob_gas_used,
                    transaction_results: transaction_results.clone(),
                });
            }
        }

        // Execute transaction
        let mut exec_result = execute_transaction(tx, state, &tx_exec_ctx, cumulative_gas_used)?;

        // Update cumulative gas
        cumulative_gas_used += exec_result.gas_used;
        cumulative_blob_gas_used += tx_blob_gas_used;

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

    if block.withdrawals_root.is_none() && !withdrawals.is_empty() {
        return Err(BlockProcessingError::UnexpectedWithdrawals {
            count: withdrawals.len(),
            transaction_results: transaction_results.clone(),
        });
    }

    // Step 4: Apply withdrawals and compute roots and bloom
    apply_withdrawals(state, withdrawals);

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

    if let Some(expected_blob_gas_used) = block.blob_gas_used
        && cumulative_blob_gas_used != expected_blob_gas_used
    {
        return Err(BlockProcessingError::BlobGasUsedMismatch {
            expected: expected_blob_gas_used,
            computed: cumulative_blob_gas_used,
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

    // Validate withdrawals root
    if let Some(expected_withdrawals_root) = block.withdrawals_root {
        let computed_withdrawals_root = calculate_withdrawals_root(withdrawals);
        if computed_withdrawals_root != expected_withdrawals_root {
            return Err(BlockProcessingError::WithdrawalsRootMismatch {
                expected: expected_withdrawals_root,
                computed: computed_withdrawals_root,
                transaction_results: transaction_results.clone(),
            });
        }
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
    use crate::types::{Address, Bytes, EMPTY_OMMERS_HASH, Hash, Withdrawal};

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
            gas_used: 15_000_000,
            timestamp: 1000,
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

    fn create_test_block(parent: &BlockHeader) -> BlockHeader {
        let mut block = parent.clone();
        block.number = parent.number + 1;
        block.timestamp = parent.timestamp + 12;
        block.parent_hash = parent.compute_hash();
        block.gas_used = 0;
        block
    }

    #[test]
    fn test_process_empty_block() {
        let parent = create_test_parent();
        let block = create_test_block(&parent);
        let transactions = vec![];
        let withdrawals = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        );
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
        let withdrawals = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        );
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
        let withdrawals = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        );
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
        let withdrawals = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        );
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
        let withdrawals = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        );
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
        let withdrawals = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        );
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
        let withdrawals = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        );
        assert!(matches!(
            result,
            Err(BlockProcessingError::GasUsedMismatch { .. })
        ));
    }

    #[test]
    fn test_process_block_blob_gas_used_mismatch() {
        let parent = create_test_parent();
        let mut block = create_test_block(&parent);
        block.blob_gas_used = Some(1);
        block.excess_blob_gas = Some(0);
        let transactions = vec![];
        let withdrawals = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        );
        assert!(matches!(
            result,
            Err(BlockProcessingError::BlobGasUsedMismatch { .. })
        ));
    }

    #[test]
    fn test_process_block_receipts_root_mismatch() {
        let parent = create_test_parent();
        let mut block = create_test_block(&parent);

        // First compute the correct receipts root for an empty block
        let transactions = vec![];
        let withdrawals = vec![];
        let mut state = InMemoryState::new();
        let correct_root = calculate_receipts_root_with_types(&[], &[]);

        // Now set a DIFFERENT receipts root that is definitely wrong
        // We'll use a non-zero hash that's different from the correct root
        block.receipts_root = if correct_root == Hash::ZERO {
            Hash::from([1u8; 32])
        } else {
            Hash::ZERO
        };

        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        );
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
        let withdrawals = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        );
        assert!(result.is_ok());

        // Test maximum valid decrease
        let mut block = create_test_block(&parent);
        block.gas_limit = parent.gas_limit - (parent.gas_limit / 1024);
        let transactions = vec![];
        let withdrawals = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_block_transactions_root_mismatch() {
        let parent = create_test_parent();
        let mut block = create_test_block(&parent);

        // Set transactions_root to a non-zero value that won't match
        block.transactions_root = Hash::from([1u8; 32]);

        let transactions = vec![];
        let withdrawals = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        );
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
        let withdrawals = vec![];
        let mut state = InMemoryState::new();

        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        );
        assert!(matches!(
            result,
            Err(BlockProcessingError::LogsBloomMismatch { .. })
        ));
    }

    #[test]
    fn test_process_block_unexpected_withdrawals() {
        let parent = create_test_parent();
        let mut block = create_test_block(&parent);
        block.withdrawals_root = None;
        let transactions = vec![];
        let withdrawals = vec![Withdrawal {
            index: 0,
            validator_index: 1,
            address: Address::from([0x10; 20]),
            amount_gwei: 1,
        }];
        let mut state = InMemoryState::new();

        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        );
        assert!(matches!(
            result,
            Err(BlockProcessingError::UnexpectedWithdrawals { .. })
        ));
    }

    #[test]
    fn test_process_block_empty_withdrawals_root_allows_empty_list() {
        let parent = create_test_parent();
        let mut block = create_test_block(&parent);
        let transactions = vec![];
        let withdrawals = vec![];
        let mut state = InMemoryState::new();

        block.withdrawals_root = Some(calculate_withdrawals_root(&withdrawals));

        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_block_withdrawals_root_mismatch() {
        let parent = create_test_parent();
        let mut block = create_test_block(&parent);
        block.withdrawals_root = Some(Hash::from([0x99; 32]));

        let transactions = vec![];
        let withdrawals = vec![Withdrawal {
            index: 0,
            validator_index: 1,
            address: Address::from([0x10; 20]),
            amount_gwei: 1,
        }];
        let mut state = InMemoryState::new();

        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        );
        assert!(matches!(
            result,
            Err(BlockProcessingError::WithdrawalsRootMismatch { .. })
        ));
    }

    #[test]
    fn test_process_block_withdrawals_applied() {
        let parent = create_test_parent();
        let mut block = create_test_block(&parent);

        let transactions = vec![];
        let withdrawals = vec![Withdrawal {
            index: 0,
            validator_index: 1,
            address: Address::from([0x10; 20]),
            amount_gwei: 2,
        }];

        let mut expected_state = InMemoryState::new();
        let amount_wei = withdrawals[0].amount_wei();
        expected_state.set_balance(&withdrawals[0].address, amount_wei);

        block.withdrawals_root = Some(calculate_withdrawals_root(&withdrawals));
        block.state_root = expected_state.compute_state_root();

        let mut state = InMemoryState::new();
        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        )
        .expect("process block");

        assert_eq!(result.state_root, expected_state.compute_state_root());
        assert_eq!(
            state.get_balance(&withdrawals[0].address),
            withdrawals[0].amount_wei()
        );
    }

    #[test]
    fn test_calculate_withdrawals_root_matches_fixture_with_duplicate_indices() {
        // Why: this fixture uses duplicate/non-monotonic withdrawal.index
        // values and catches trie-reference encoding mismatches immediately.
        let withdrawals = vec![
            Withdrawal {
                index: 0,
                validator_index: 0,
                address: Address::from([
                    0xc9, 0x4f, 0x53, 0x74, 0xfc, 0xe5, 0xed, 0xbc, 0x8e, 0x2a, 0x86, 0x97, 0xc1,
                    0x53, 0x31, 0x67, 0x7e, 0x6e, 0xbf, 0x0b,
                ]),
                amount_gwei: 10_000,
            },
            Withdrawal {
                index: 2,
                validator_index: 0,
                address: Address::from([
                    0xc9, 0x4f, 0x53, 0x74, 0xfc, 0xe5, 0xed, 0xbc, 0x8e, 0x2a, 0x86, 0x97, 0xc1,
                    0x53, 0x31, 0x67, 0x7e, 0x6e, 0xbf, 0x0b,
                ]),
                amount_gwei: 10_000,
            },
            Withdrawal {
                index: 1,
                validator_index: 0,
                address: Address::from([
                    0xc9, 0x4f, 0x53, 0x74, 0xfc, 0xe5, 0xed, 0xbc, 0x8e, 0x2a, 0x86, 0x97, 0xc1,
                    0x53, 0x31, 0x67, 0x7e, 0x6e, 0xbf, 0x0b,
                ]),
                amount_gwei: 10_000,
            },
            Withdrawal {
                index: 2,
                validator_index: 0,
                address: Address::from([
                    0xc9, 0x4f, 0x53, 0x74, 0xfc, 0xe5, 0xed, 0xbc, 0x8e, 0x2a, 0x86, 0x97, 0xc1,
                    0x53, 0x31, 0x67, 0x7e, 0x6e, 0xbf, 0x0b,
                ]),
                amount_gwei: 10_000,
            },
        ];

        let expected = Hash::from([
            0xa9, 0x5b, 0x9a, 0x7b, 0x58, 0xa6, 0xb3, 0xcb, 0x40, 0x01, 0xeb, 0x0b, 0xe6, 0x79,
            0x51, 0xc5, 0x51, 0x71, 0x41, 0xcb, 0x01, 0x83, 0xa2, 0x55, 0xb5, 0xca, 0xe0, 0x27,
            0xa7, 0xb1, 0x0b, 0x36,
        ]);

        assert_eq!(calculate_withdrawals_root(&withdrawals), expected);
    }

    #[test]
    fn test_calculate_withdrawals_root_matches_fixture_same_address_diff_validators() {
        // Why: this fixture confirms validator_index participates in value
        // encoding while trie keys still come from list position.
        let withdrawals = vec![
            Withdrawal {
                index: 0,
                validator_index: 0,
                address: Address::from([
                    0xc9, 0x4f, 0x53, 0x74, 0xfc, 0xe5, 0xed, 0xbc, 0x8e, 0x2a, 0x86, 0x97, 0xc1,
                    0x53, 0x31, 0x67, 0x7e, 0x6e, 0xbf, 0x0b,
                ]),
                amount_gwei: 10_000,
            },
            Withdrawal {
                index: 2,
                validator_index: 1,
                address: Address::from([
                    0xc9, 0x4f, 0x53, 0x74, 0xfc, 0xe5, 0xed, 0xbc, 0x8e, 0x2a, 0x86, 0x97, 0xc1,
                    0x53, 0x31, 0x67, 0x7e, 0x6e, 0xbf, 0x0b,
                ]),
                amount_gwei: 10_000,
            },
            Withdrawal {
                index: 1,
                validator_index: 3,
                address: Address::from([
                    0xc9, 0x4f, 0x53, 0x74, 0xfc, 0xe5, 0xed, 0xbc, 0x8e, 0x2a, 0x86, 0x97, 0xc1,
                    0x53, 0x31, 0x67, 0x7e, 0x6e, 0xbf, 0x0b,
                ]),
                amount_gwei: 10_000,
            },
        ];

        let expected = Hash::from([
            0x36, 0x19, 0xf8, 0xd0, 0x3e, 0x31, 0x4a, 0x2c, 0xbe, 0x21, 0x7b, 0x3a, 0x45, 0x8f,
            0xb5, 0xe4, 0x12, 0x5b, 0xdf, 0x37, 0x0f, 0xa9, 0x0d, 0xb4, 0x9e, 0x34, 0xb2, 0x0a,
            0x0b, 0x1a, 0x93, 0xa0,
        ]);

        assert_eq!(calculate_withdrawals_root(&withdrawals), expected);
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
        let log1 = Log::new(
            Address::from([1u8; 20]),
            vec![Hash::from([2u8; 32])],
            Bytes::new(),
        );
        let receipt1 = TransactionReceipt::new(true, U256::from(100u64), vec![log1.clone()]);

        let log2 = Log::new(
            Address::from([3u8; 20]),
            vec![Hash::from([4u8; 32])],
            Bytes::new(),
        );
        let receipt2 = TransactionReceipt::new(true, U256::from(200u64), vec![log2.clone()]);

        let receipts = vec![receipt1.clone(), receipt2.clone()];
        let combined_bloom_bytes = calculate_logs_bloom(&receipts);

        // Manually create expected bloom by combining both receipt blooms
        let mut expected_bloom = receipt1.logs_bloom;
        expected_bloom.combine(&receipt2.logs_bloom);

        assert_eq!(combined_bloom_bytes, *expected_bloom.as_bytes());
    }

    // =========================================================================
    // EIP-4788 Beacon Root System Call Tests
    // =========================================================================

    /// The EIP-4788 beacon roots contract runtime bytecode.
    const BEACON_ROOTS_BYTECODE: [u8; 97] = [
        0x33, 0x73, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe, 0x14, 0x60, 0x4d, 0x57, 0x60, 0x20, 0x36, 0x14,
        0x60, 0x24, 0x57, 0x5f, 0x5f, 0xfd, 0x5b, 0x5f, 0x35, 0x80, 0x15, 0x60, 0x49, 0x57, 0x62,
        0x00, 0x1f, 0xff, 0x81, 0x06, 0x90, 0x81, 0x54, 0x14, 0x60, 0x3c, 0x57, 0x5f, 0x5f, 0xfd,
        0x5b, 0x62, 0x00, 0x1f, 0xff, 0x01, 0x54, 0x5f, 0x52, 0x60, 0x20, 0x5f, 0xf3, 0x5b, 0x5f,
        0x5f, 0xfd, 0x5b, 0x62, 0x00, 0x1f, 0xff, 0x42, 0x06, 0x42, 0x81, 0x55, 0x5f, 0x35, 0x90,
        0x62, 0x00, 0x1f, 0xff, 0x01, 0x55, 0x00,
    ];

    /// The EIP-2935 historical block hashes contract runtime bytecode.
    const HISTORY_STORAGE_BYTECODE: [u8; 83] = [
        0x33, 0x73, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe, 0x14, 0x60, 0x46, 0x57, 0x60, 0x20, 0x36, 0x03,
        0x60, 0x42, 0x57, 0x5f, 0x35, 0x60, 0x01, 0x43, 0x03, 0x81, 0x11, 0x60, 0x42, 0x57, 0x61,
        0x1f, 0xff, 0x81, 0x43, 0x03, 0x11, 0x60, 0x42, 0x57, 0x61, 0x1f, 0xff, 0x90, 0x06, 0x54,
        0x5f, 0x52, 0x60, 0x20, 0x5f, 0xf3, 0x5b, 0x5f, 0x5f, 0xfd, 0x5b, 0x5f, 0x35, 0x61, 0x1f,
        0xff, 0x60, 0x01, 0x43, 0x03, 0x06, 0x55, 0x00,
    ];

    #[test]
    fn test_beacon_root_system_call_no_code_silent() {
        // When no code is deployed at BEACON_ROOTS_ADDRESS, the system call should
        // be a no-op (fail silently).
        let mut state = InMemoryState::new();
        let parent = create_test_parent();
        let mut block = create_test_block(&parent);
        block.parent_beacon_block_root = Some(Hash::from([0xaa; 32]));

        let block_ctx = BlockContext {
            number: U256::from_u64(block.number),
            timestamp: U256::from_u64(block.timestamp),
            coinbase: block.coinbase,
            difficulty: block.difficulty,
            gas_limit: U256::from_u64(block.gas_limit),
            chain_id: U256::ONE,
            base_fee: U256::from_u64(block.base_fee_per_gas.unwrap_or(0)),
            excess_blob_gas: None,
            max_precompile_address: 0x0a,
        };
        let parent_hash = parent.compute_hash();

        // Should not panic or modify state
        apply_beacon_root_system_call(&mut state, &block, &block_ctx, parent_hash, &[]);

        // State should remain empty
        assert_eq!(state.compute_state_root(), EMPTY_TRIE_ROOT);
    }

    #[test]
    fn test_beacon_root_system_call_no_beacon_root() {
        // When parent_beacon_block_root is None, the system call should be a no-op.
        let mut state = InMemoryState::new();
        state.set_code(&BEACON_ROOTS_ADDRESS, BEACON_ROOTS_BYTECODE.to_vec());

        let parent = create_test_parent();
        let block = create_test_block(&parent);
        // block.parent_beacon_block_root is None

        let block_ctx = BlockContext {
            number: U256::from_u64(block.number),
            timestamp: U256::from_u64(block.timestamp),
            coinbase: block.coinbase,
            difficulty: block.difficulty,
            gas_limit: U256::from_u64(block.gas_limit),
            chain_id: U256::ONE,
            base_fee: U256::from_u64(block.base_fee_per_gas.unwrap_or(0)),
            excess_blob_gas: None,
            max_precompile_address: 0x0a,
        };
        let parent_hash = parent.compute_hash();

        let state_root_before = state.compute_state_root();
        apply_beacon_root_system_call(&mut state, &block, &block_ctx, parent_hash, &[]);

        // State should be unchanged (only the contract code, no storage writes)
        assert_eq!(state.compute_state_root(), state_root_before);
    }

    #[test]
    fn test_beacon_root_system_call_debug() {
        // Debug: execute a minimal SSTORE test to verify EVM+state works
        use crate::evm::host::NullHost;
        use crate::evm::interpreter::{CallContext, Evm, TxContext};

        let mut state = InMemoryState::new();
        let test_addr = Address::from([0xaa; 20]);

        // Bytecode: PUSH1 0x42 PUSH1 0x01 SSTORE STOP
        // This stores value 0x42 at storage slot 1 for the call_ctx.address
        let code = vec![0x60, 0x42, 0x60, 0x01, 0x55, 0x00];

        let call_ctx = CallContext {
            address: test_addr,
            caller: Address::from([0xbb; 20]),
            call_value: U256::ZERO,
            call_data: vec![],
        };

        let mut evm = Evm::new(code, 100_000, state.clone(), NullHost).with_call_context(call_ctx);

        let result = evm.run();
        eprintln!("Simple SSTORE test: result={result:?}");
        assert!(result.is_ok());

        let final_state = evm.into_state();
        let v = final_state.sload(&test_addr, &U256::from_u64(1));
        eprintln!("Simple SSTORE test: storage[1] = {v:?}");
        assert_eq!(v, U256::from_u64(0x42), "Simple SSTORE should work");

        // Test SSTORE with large key (like 1012)
        // Bytecode: PUSH2 0x03f4 PUSH2 0x03f4 SSTORE STOP
        // store value 0x03f4 at key 0x03f4
        let code2 = vec![
            0x61, 0x03, 0xf4, // PUSH2 1012
            0x61, 0x03, 0xf4, // PUSH2 1012
            0x55, // SSTORE
            0x00, // STOP
        ];
        let call_ctx_big = CallContext {
            address: test_addr,
            caller: Address::from([0xbb; 20]),
            call_value: U256::ZERO,
            call_data: vec![],
        };
        let mut evm_big = Evm::new(code2, 100_000, InMemoryState::new(), NullHost)
            .with_call_context(call_ctx_big);
        let result_big = evm_big.run();
        eprintln!(
            "Big key SSTORE test: result={:?}",
            result_big.as_ref().map(|r| (r.success, r.gas_used))
        );
        assert!(result_big.is_ok());
        let big_state = evm_big.into_state();
        let v_big = big_state.sload(&test_addr, &U256::from_u64(1012));
        eprintln!("Big key SSTORE test: storage[1012] = {v_big:?}");
        assert_eq!(v_big, U256::from_u64(1012), "Large key SSTORE should work");

        // Now test the beacon roots bytecode
        state.set_code(&BEACON_ROOTS_ADDRESS, BEACON_ROOTS_BYTECODE.to_vec());

        let parent = create_test_parent();
        let block = create_test_block(&parent);
        let beacon_root = Hash::from([0xbb; 32]);

        let block_ctx = BlockContext {
            number: U256::from_u64(block.number),
            timestamp: U256::from_u64(block.timestamp),
            coinbase: block.coinbase,
            difficulty: block.difficulty,
            gas_limit: U256::from_u64(block.gas_limit),
            chain_id: U256::ONE,
            base_fee: U256::from_u64(block.base_fee_per_gas.unwrap_or(0)),
            excess_blob_gas: None,
            max_precompile_address: 0x0a,
        };

        let calldata = beacon_root.as_bytes().to_vec();
        let call_ctx2 = CallContext {
            address: BEACON_ROOTS_ADDRESS,
            caller: SYSTEM_ADDRESS,
            call_value: U256::ZERO,
            call_data: calldata,
        };

        let tx_ctx = TxContext {
            origin: SYSTEM_ADDRESS,
            gas_price: U256::ZERO,
            blob_versioned_hashes: Vec::new(),
        };

        let host = RecursiveHost::new()
            .with_block_context(block_ctx.clone())
            .with_parent_hash(parent.compute_hash())
            .with_tx_context(tx_ctx.clone());

        // Instead of using the full bytecode, let's test the set path directly
        // The set path starts at offset 0x4d (77) in the contract
        // It does: PUSH3 0x1fff, TIMESTAMP, MOD, TIMESTAMP, DUP2, SSTORE, PUSH0, CALLDATALOAD, SWAP1, PUSH3 0x1fff, ADD, SSTORE, STOP
        // Let's test with a simple bytecode that should do the same thing:
        // PUSH3 0x1fff TIMESTAMP MOD TIMESTAMP DUP2 SSTORE PUSH0 CALLDATALOAD SWAP1 PUSH3 0x1fff ADD SSTORE STOP
        let set_bytecode = vec![
            0x62, 0x00, 0x1f, 0xff, // PUSH3 0x1fff
            0x42, // TIMESTAMP
            0x06, // MOD
            0x42, // TIMESTAMP
            0x81, // DUP2
            0x55, // SSTORE
            0x5f, // PUSH0
            0x35, // CALLDATALOAD
            0x90, // SWAP1
            0x62, 0x00, 0x1f, 0xff, // PUSH3 0x1fff
            0x01, // ADD
            0x55, // SSTORE
            0x00, // STOP
        ];

        let call_ctx3 = CallContext {
            address: BEACON_ROOTS_ADDRESS,
            caller: SYSTEM_ADDRESS,
            call_value: U256::ZERO,
            call_data: beacon_root.as_bytes().to_vec(),
        };

        let host2 = RecursiveHost::new()
            .with_block_context(block_ctx.clone())
            .with_parent_hash(parent.compute_hash())
            .with_tx_context(tx_ctx.clone());

        let mut evm3 = Evm::new(set_bytecode, SYSTEM_CALL_GAS_LIMIT, state.clone(), host2)
            .with_block_context(block_ctx.clone())
            .with_tx_context(tx_ctx.clone())
            .with_call_context(call_ctx3);

        let result3 = evm3.run();
        eprintln!(
            "Direct set path: result={:?}",
            result3.as_ref().map(|r| (r.success, r.gas_used))
        );
        assert!(result3.is_ok());

        let history_buffer_length = 8191u64;
        let timestamp_idx = block.timestamp % history_buffer_length;
        let root_idx = timestamp_idx + history_buffer_length;

        let direct_state = evm3.into_state();
        let stored_ts_direct =
            direct_state.sload(&BEACON_ROOTS_ADDRESS, &U256::from_u64(timestamp_idx));
        let stored_root_direct =
            direct_state.sload(&BEACON_ROOTS_ADDRESS, &U256::from_u64(root_idx));
        eprintln!("Direct set: stored_ts={stored_ts_direct:?}, stored_root={stored_root_direct:?}");

        // Now test the full bytecode
        let mut evm2 = Evm::new(
            BEACON_ROOTS_BYTECODE.to_vec(),
            SYSTEM_CALL_GAS_LIMIT,
            state.clone(),
            host,
        )
        .with_block_context(block_ctx)
        .with_tx_context(tx_ctx)
        .with_call_context(call_ctx2);

        let result2 = evm2.run();
        match &result2 {
            Ok(r) => eprintln!(
                "Beacon root EVM: success={}, gas_used={}",
                r.success, r.gas_used
            ),
            Err(e) => eprintln!("Beacon root EVM error: {e:?}"),
        }
        assert!(result2.is_ok());
        let exec_result = result2.unwrap();

        // Check stack/memory for debugging
        eprintln!("Stack after execution: {:?}", exec_result.stack);

        let final_state2 = evm2.into_state();
        let history_buffer_length = 8191u64;
        let timestamp_idx = block.timestamp % history_buffer_length;
        let root_idx = timestamp_idx + history_buffer_length;

        let stored_ts = final_state2.sload(&BEACON_ROOTS_ADDRESS, &U256::from_u64(timestamp_idx));
        let stored_root = final_state2.sload(&BEACON_ROOTS_ADDRESS, &U256::from_u64(root_idx));
        eprintln!(
            "timestamp_idx={timestamp_idx} (ts={}), root_idx={root_idx}",
            block.timestamp
        );
        eprintln!("stored_ts={stored_ts:?}");
        eprintln!("stored_root={stored_root:?}");

        // Also check if SSTORE wrote somewhere unexpected
        for key in 0..30u64 {
            let v = final_state2.sload(&BEACON_ROOTS_ADDRESS, &U256::from_u64(key));
            if !v.is_zero() {
                eprintln!("  BEACON storage[{key}] = {v:?}");
            }
        }
        for key in 1000..1020u64 {
            let v = final_state2.sload(&BEACON_ROOTS_ADDRESS, &U256::from_u64(key));
            if !v.is_zero() {
                eprintln!("  BEACON storage[{key}] = {v:?}");
            }
        }

        assert_eq!(stored_ts, U256::from_u64(block.timestamp));
    }

    #[test]
    fn test_beacon_root_system_call_stores_root() {
        // When beacon roots contract is deployed and parent_beacon_block_root is
        // present, the system call should store the timestamp and root in the
        // contract's ring buffer storage.
        let mut state = InMemoryState::new();
        state.set_code(&BEACON_ROOTS_ADDRESS, BEACON_ROOTS_BYTECODE.to_vec());

        let parent = create_test_parent();
        let mut block = create_test_block(&parent);
        let beacon_root = Hash::from([0xbb; 32]);
        block.parent_beacon_block_root = Some(beacon_root);

        let block_ctx = BlockContext {
            number: U256::from_u64(block.number),
            timestamp: U256::from_u64(block.timestamp),
            coinbase: block.coinbase,
            difficulty: block.difficulty,
            gas_limit: U256::from_u64(block.gas_limit),
            chain_id: U256::ONE,
            base_fee: U256::from_u64(block.base_fee_per_gas.unwrap_or(0)),
            excess_blob_gas: None,
            max_precompile_address: 0x0a,
        };
        let parent_hash = parent.compute_hash();

        apply_beacon_root_system_call(&mut state, &block, &block_ctx, parent_hash, &[]);

        // HISTORY_BUFFER_LENGTH = 0x1fff = 8191
        let history_buffer_length = 8191u64;
        let timestamp_idx = block.timestamp % history_buffer_length;
        let root_idx = timestamp_idx + history_buffer_length;

        // Check that timestamp was stored at timestamp_idx
        let stored_timestamp = state.sload(&BEACON_ROOTS_ADDRESS, &U256::from_u64(timestamp_idx));
        assert_eq!(
            stored_timestamp,
            U256::from_u64(block.timestamp),
            "timestamp should be stored at timestamp_idx"
        );

        // Check that beacon root was stored at root_idx
        let stored_root = state.sload(&BEACON_ROOTS_ADDRESS, &U256::from_u64(root_idx));
        assert_eq!(
            stored_root,
            U256::from_be_bytes(*beacon_root.as_bytes()),
            "beacon root should be stored at root_idx"
        );
    }

    #[test]
    fn test_history_storage_system_call_stores_parent_hash() {
        let mut state = InMemoryState::new();
        state.set_code(&HISTORY_STORAGE_ADDRESS, HISTORY_STORAGE_BYTECODE.to_vec());

        let parent = create_test_parent();
        let block = create_test_block(&parent);
        let block_ctx = BlockContext {
            number: U256::from_u64(block.number),
            timestamp: U256::from_u64(block.timestamp),
            coinbase: block.coinbase,
            difficulty: block.difficulty,
            gas_limit: U256::from_u64(block.gas_limit),
            chain_id: U256::ONE,
            base_fee: U256::from_u64(block.base_fee_per_gas.unwrap_or(0)),
            excess_blob_gas: None,
            max_precompile_address: 0x0a,
        };
        let parent_hash = parent.compute_hash();

        apply_history_storage_system_call(&mut state, &block_ctx, parent_hash, &[]);

        let history_buffer_length = 8191u64;
        let slot = (block.number - 1) % history_buffer_length;
        let stored_hash = state.sload(&HISTORY_STORAGE_ADDRESS, &U256::from_u64(slot));
        assert_eq!(
            stored_hash,
            U256::from_be_bytes(*parent_hash.as_bytes()),
            "parent hash should be stored at ring buffer slot"
        );
    }

    #[test]
    fn test_process_block_with_beacon_root() {
        // End-to-end test: process an empty block with parent_beacon_block_root set.
        // The beacon root system call should modify state before state root computation.
        let mut state = InMemoryState::new();
        state.set_code(&BEACON_ROOTS_ADDRESS, BEACON_ROOTS_BYTECODE.to_vec());

        let parent = create_test_parent();
        let mut block = create_test_block(&parent);
        let beacon_root = Hash::from([0xcc; 32]);
        block.parent_beacon_block_root = Some(beacon_root);

        // Pre-compute the expected state root by simulating what process_block does:
        let mut expected_state = state.clone();
        let block_ctx = BlockContext {
            number: U256::from_u64(block.number),
            timestamp: U256::from_u64(block.timestamp),
            coinbase: block.coinbase,
            difficulty: block.difficulty,
            gas_limit: U256::from_u64(block.gas_limit),
            chain_id: U256::ONE,
            base_fee: U256::from_u64(block.base_fee_per_gas.unwrap_or(0)),
            excess_blob_gas: None,
            max_precompile_address: 0x0a,
        };
        let parent_hash = parent.compute_hash();
        apply_beacon_root_system_call(&mut expected_state, &block, &block_ctx, parent_hash, &[]);
        block.state_root = expected_state.compute_state_root();

        let transactions = vec![];
        let withdrawals = vec![];

        let result = process_block(
            &block,
            &parent,
            &transactions,
            &withdrawals,
            &[],
            &mut state,
            U256::ONE,
        );
        if let Err(ref e) = result {
            eprintln!("Error processing block with beacon root: {e:?}");
        }
        assert!(result.is_ok());

        // Verify the beacon root was stored
        let history_buffer_length = 8191u64;
        let timestamp_idx = block.timestamp % history_buffer_length;
        let root_idx = timestamp_idx + history_buffer_length;
        let stored_root = state.sload(&BEACON_ROOTS_ADDRESS, &U256::from_u64(root_idx));
        assert_eq!(stored_root, U256::from_be_bytes(*beacon_root.as_bytes()),);
    }
}
