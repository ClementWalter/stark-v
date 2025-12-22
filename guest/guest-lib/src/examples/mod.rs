//! Example computations for testing the VM.

pub mod branch;
pub mod compute;
pub mod factorial;
pub mod fib;
pub mod memory;
pub mod muldiv;

pub use branch::{branch, branch_test_impl};
pub use compute::compute;
pub use factorial::{fact, factorial_impl};
pub use fib::{fib, fibonacci_impl};
pub use memory::{memory, memory_test_impl};
pub use muldiv::{muldiv, muldiv_test_impl};
