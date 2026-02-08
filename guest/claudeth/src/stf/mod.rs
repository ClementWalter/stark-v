//! State Transition Function (STF) module
//!
//! This module implements the Ethereum state transition function, including
//! transaction validation, execution, and receipt generation.

pub mod executor;
pub mod receipt;
pub mod transaction;

pub use executor::{execute_transaction, ExecutionError, TransactionExecutionResult};
pub use receipt::{calculate_receipts_root, Bloom, Log, TransactionReceipt};
pub use transaction::{
    calculate_intrinsic_gas, validate_balance, validate_chain_id, validate_gas, validate_nonce,
    validate_signature, validate_transaction, ValidationError,
};
