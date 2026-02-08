//! Ethereum Virtual Machine implementation
//!
//! This module provides the core EVM execution components:
//! - Memory: Dynamic memory with gas-based expansion
//! - Stack: 256-bit word stack with depth limits
//! - Gas metering: Gas cost calculation and tracking
//! - Opcodes: EVM instruction execution
//! - Interpreter: Bytecode execution engine

pub mod gas;
pub mod host;
pub mod interpreter;
pub mod memory;
pub mod opcodes;
pub mod stack;

// Re-export Gas functions and constants
pub use gas::{
    call_gas_cost, code_deposit_cost, copy_gas_cost, create2_hash_cost, exp_gas_cost,
    init_code_gas_cost, keccak256_gas_cost, log_gas_cost, memory_expansion_cost, opcode_gas_cost,
    // Export commonly used constants
    GAS_ADD, GAS_CALL_COLD, GAS_CALL_NEW_ACCOUNT, GAS_CALL_STIPEND, GAS_CALL_VALUE_TRANSFER,
    GAS_CALL_WARM, GAS_CREATE, GAS_CREATE2, GAS_DIV, GAS_EQ, GAS_JUMP, GAS_JUMPI, GAS_KECCAK256,
    GAS_MUL, GAS_SLOAD_COLD, GAS_SLOAD_WARM, GAS_SSTORE_SET, GAS_SUB, MAX_CODE_SIZE,
    MAX_INIT_CODE_SIZE,
};

// Re-export Memory types
pub use memory::{Memory, MemoryError};

// Re-export Stack types
pub use stack::{Stack, StackError, MAX_STACK_SIZE};

// Re-export Opcode types and functions
// pub use opcodes::{EvmError, arithmetic::*};

// Re-export Interpreter types and functions
pub use host::{
    CallKind, CallMessage, CallResult, CreateMessage, CreateResult, Host, NullHost,
};
pub use interpreter::{
    execute_bytecode, execute_bytecode_with_host, BlockContext, EvmError, ExecutionResult, TxContext,
};
