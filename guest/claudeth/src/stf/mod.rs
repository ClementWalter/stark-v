//! State Transition Function (STF) module
//!
//! This module implements the Ethereum state transition function, including
//! transaction validation, execution, receipt generation, and block processing.

pub mod block;
pub mod executor;
pub mod receipt;
pub mod transaction;

pub use block::{BlockProcessingError, BlockProcessingResult, process_block};
pub use executor::{
    BlockHashContext, ExecutionError, TransactionExecutionResult, execute_transaction,
};
pub use receipt::{
    Bloom, Log, TransactionReceipt, calculate_receipts_root, calculate_receipts_root_with_types,
};
pub use transaction::{
    ValidationError, calculate_intrinsic_gas, validate_balance, validate_chain_id, validate_gas,
    validate_nonce, validate_signature, validate_transaction,
};
