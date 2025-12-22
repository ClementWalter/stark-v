//! Example computations for testing the VM.

pub mod branch;
pub mod constant;
pub mod factorial;
pub mod fib;
pub mod memory;
pub mod muldiv;

pub use branch::{branch, branch_test_impl, BranchResult};
pub use constant::{constant, ConstantResult};
pub use factorial::{fact, factorial_impl, FactorialResult};
pub use fib::{fib, fibonacci_impl, FibResult};
pub use memory::{memory, memory_test_impl, MemoryTestResult};
pub use muldiv::{muldiv, muldiv_test_impl, MulDivResult};
