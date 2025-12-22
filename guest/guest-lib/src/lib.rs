//! Guest library for zkVM programs.
//!
//! This library provides:
//! - Result types for guest-host communication
//! - The `guest_main!` macro for minimal boilerplate
//! - Example computations for testing

#![cfg_attr(target_arch = "riscv32", no_std)]

// =============================================================================
// Modules
// =============================================================================

pub mod examples;

// =============================================================================
// Re-exports
// =============================================================================

pub use examples::{
    branch, constant, fact, fib, memory, muldiv, BranchResult, ConstantResult, FactorialResult,
    FibResult, MemoryTestResult, MulDivResult,
};

// =============================================================================
// Guest glue and entry macro
// =============================================================================

#[cfg(target_arch = "riscv32")]
pub mod glue;

/// Macro to define the guest entry point with minimal boilerplate.
///
/// # Example
///
/// ```ignore
/// #![no_std]
/// #![no_main]
///
/// guest_lib::guest_main!(guest_lib::fib(20));
/// ```
#[cfg(target_arch = "riscv32")]
#[macro_export]
macro_rules! guest_main {
    ($expr:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn __zkvm_start() -> ! {
            $crate::glue::output(&$expr)
        }
    };
}
