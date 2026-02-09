//! State Transition Function (STF) module
//!
//! This module implements the Ethereum state transition function, including
//! transaction validation, execution, receipt generation, and block processing.

pub mod block;
pub mod executor;
pub mod receipt;
pub mod transaction;

pub use block::{process_block, BlockProcessingError, BlockProcessingResult};
pub use executor::{
    execute_transaction, ExecutionError, TransactionExecutionContext,
    TransactionExecutionResult,
};
pub use receipt::{
    calculate_receipts_root, calculate_receipts_root_with_types, Bloom, Log, TransactionReceipt,
};
pub use transaction::{
    calculate_intrinsic_gas, validate_balance, validate_chain_id, validate_gas, validate_nonce,
    validate_signature, validate_transaction, ValidationError,
};
