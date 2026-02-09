//! EVM execution error type (shared by interpreter and opcodes).

#[cfg(target_arch = "riscv32")]
extern crate alloc;

use crate::evm::memory::MemoryError;
use crate::evm::stack::StackError;

#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;
#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

/// Errors that can occur during EVM execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvmError {
    /// Stack error (overflow, underflow, invalid index)
    StackError(StackError),
    /// Memory error (invalid offset, overflow)
    MemoryError(MemoryError),
    /// Insufficient gas for operation
    OutOfGas,
    /// Invalid opcode encountered
    InvalidOpcode(u8),
    /// Invalid JUMP destination
    InvalidJump,
    /// Execution reverted
    Revert(Vec<u8>),
    /// PC out of bounds
    PcOutOfBounds,
    /// Invalid PUSH data (not enough bytes)
    InvalidPush,
    /// Invalid address (e.g. from U256 conversion)
    InvalidAddress,
}

impl From<StackError> for EvmError {
    fn from(err: StackError) -> Self {
        EvmError::StackError(err)
    }
}

impl From<MemoryError> for EvmError {
    fn from(err: MemoryError) -> Self {
        EvmError::MemoryError(err)
    }
}

impl From<crate::evm::opcodes::arithmetic::EvmError> for EvmError {
    fn from(err: crate::evm::opcodes::arithmetic::EvmError) -> Self {
        match err {
            crate::evm::opcodes::arithmetic::EvmError::Stack(e) => EvmError::StackError(e),
            crate::evm::opcodes::arithmetic::EvmError::Memory(e) => EvmError::MemoryError(e),
        }
    }
}
