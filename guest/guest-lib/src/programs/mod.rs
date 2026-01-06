//! Example computations for testing the VM.

pub mod branch;
pub mod constant;
pub mod factorial;
pub mod fib;
pub mod load_merge;
pub mod memory;
pub mod muldiv;
pub mod sha2;

pub use branch::{BranchResult, branch};
pub use constant::{ConstantResult, constant};
pub use factorial::{FactorialResult, fact};
pub use fib::{FibResult, fib};
pub use load_merge::{LoadMergeResult, load_merge};
pub use memory::{MemoryTestResult, memory};
pub use muldiv::{MulDivResult, muldiv};
pub use sha2::{Sha2Result, sha256};
