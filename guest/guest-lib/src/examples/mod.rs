//! Example computations for testing the VM.

pub mod branch;
pub mod constant;
pub mod factorial;
pub mod fib;
pub mod memory;
pub mod muldiv;

pub use branch::{branch, BranchResult};
pub use constant::{constant, ConstantResult};
pub use factorial::{fact, FactorialResult};
pub use fib::{fib, FibResult};
pub use memory::{memory, MemoryTestResult};
pub use muldiv::{muldiv, MulDivResult};
