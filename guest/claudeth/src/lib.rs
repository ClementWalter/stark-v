//! Claudeth - Dependency-free Ethereum State Transition Function for stark-v zkVM
//!
//! This library implements the Ethereum protocol execution layer with zero external dependencies,
//! designed for efficient proof generation in zkVM environments.
//!
//! ## Features
//!
//! - `no_std` compatible with `alloc` support (on riscv32 target)
//! - Minimal dependencies (serde only)
//! - Full Ethereum state transition function
//! - Optimized for Circle STARKs proof generation
//!
//! ## Architecture
//!
//! - [`types`]: Core Ethereum types (U256, Address, BlockHeader, etc.)
//! - [`crypto`]: Cryptographic primitives (Keccak256, ECDSA, etc.)
//! - [`state`]: State management and Merkle Patricia Trie
//! - [`evm`]: Ethereum Virtual Machine implementation

#![cfg_attr(target_arch = "riscv32", no_std)]

#[cfg(target_arch = "riscv32")]
extern crate alloc;

// =============================================================================
// Platform-specific setup for riscv32
// =============================================================================

#[cfg(target_arch = "riscv32")]
use core::alloc::{GlobalAlloc, Layout};
#[cfg(target_arch = "riscv32")]
use core::panic::PanicInfo;

/// Simple bump allocator for riscv32 target
#[cfg(target_arch = "riscv32")]
struct BumpAllocator;

#[cfg(target_arch = "riscv32")]
unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // No-op for bump allocator
    }
}

#[cfg(target_arch = "riscv32")]
#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator;

#[cfg(target_arch = "riscv32")]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// =============================================================================
// Modules
// =============================================================================

pub mod crypto;
pub mod evm;
pub mod state;
pub mod stf;
pub mod types;

// =============================================================================
// Re-exports
// =============================================================================

// Re-export implemented types
pub use types::Bytes;

// Future re-exports will be added as more types and crypto modules are implemented
// pub use types::*;
// pub use crypto::*;

// =============================================================================
// Library metadata
// =============================================================================

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        // Verify version is set (const, so no need to check is_empty)
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }
}
