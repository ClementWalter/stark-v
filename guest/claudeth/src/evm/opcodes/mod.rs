//! EVM Opcodes implementation
//!
//! This module provides implementations for all Ethereum Virtual Machine opcodes,
//! organized by category.

pub mod arithmetic;
pub mod control;
pub mod environment;
pub mod exec;
pub mod log;
pub mod utils;

// Re-export all opcode functions and types
pub use arithmetic::*;
pub use control::*;
pub use environment::*;
pub use exec::*;
pub use log::*;
pub use utils::*;
