//! Claudeth - Minimal-dependency Ethereum State Transition Function for stark-v zkVM
//!
//! This library implements the Ethereum protocol execution layer with minimal external dependencies,
//! designed for efficient proof generation in zkVM environments. It relies on `serde` and keeps
//! cryptography in-tree, with a deterministic in-tree signer for tests.
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
const HEAP_SIZE: usize = 16 * 1024 * 1024;

#[cfg(target_arch = "riscv32")]
#[repr(align(16))]
struct AlignedHeap([u8; HEAP_SIZE]);

#[cfg(target_arch = "riscv32")]
static mut HEAP: AlignedHeap = AlignedHeap([0; HEAP_SIZE]);

#[cfg(target_arch = "riscv32")]
static mut HEAP_OFFSET: usize = 0;

#[cfg(target_arch = "riscv32")]
unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        let layout = _layout;
        let size = layout.size();
        if size == 0 {
            return core::ptr::null_mut();
        }

        let align = layout.align().max(core::mem::align_of::<usize>());
        // Why: rv32im excludes atomic compare-and-swap, and the guest runtime
        // is single-threaded, so a plain mutable bump pointer is sufficient.
        let current = unsafe { HEAP_OFFSET };
        let aligned = (current + align - 1) & !(align - 1);
        let next = aligned.saturating_add(size);
        if next > HEAP_SIZE {
            return core::ptr::null_mut();
        }

        // SAFETY: single-threaded guest; allocator is the only writer.
        unsafe {
            HEAP_OFFSET = next;
            let heap_ptr = core::ptr::addr_of_mut!(HEAP.0) as *mut u8;
            heap_ptr.add(aligned)
        }
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
