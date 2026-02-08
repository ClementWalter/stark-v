//! EVM Opcodes implementation
//!
//! This module provides implementations for all Ethereum Virtual Machine opcodes,
//! organized by category.

pub mod arithmetic;

// Re-export all opcode functions
pub use arithmetic::*;
