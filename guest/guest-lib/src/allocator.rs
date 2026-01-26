//! Bump allocator for zkVM guest programs.
//!
//! This module provides a minimal bump allocator optimized for zkVM execution
//! where every cycle counts and memory is never reclaimed.
//!
//! Only compiled for riscv32 target with the `alloc` feature enabled.
//! Native builds use the system allocator.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr;

unsafe extern "C" {
    static __heap_start: u8;
    static __heap_end: u8;
}

/// Bump allocator that never frees memory.
///
/// This allocator is optimized for zkVM where:
/// - Every cycle counts (minimal overhead)
/// - Programs are short-lived (no need to reclaim memory)
/// - Single-threaded execution (no synchronization needed)
pub struct ZkvmBumpAlloc {
    head: UnsafeCell<usize>,
}

// SAFETY: zkVM is single-threaded, no concurrent access possible
unsafe impl Sync for ZkvmBumpAlloc {}

impl ZkvmBumpAlloc {
    /// Create a new uninitialized allocator.
    pub const fn new() -> Self {
        Self {
            head: UnsafeCell::new(0),
        }
    }

    /// Initialize the allocator with heap bounds from linker script.
    ///
    /// # Safety
    ///
    /// Must be called exactly once before any allocations.
    /// Called automatically by `guest_main!` macro when alloc feature is enabled.
    pub unsafe fn init(&self) {
        // SAFETY: Caller guarantees this is called once before any allocations
        unsafe {
            *self.head.get() = ptr::addr_of!(__heap_start) as usize;
        }
    }
}

unsafe impl GlobalAlloc for ZkvmBumpAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: We are the only accessor of head (single-threaded zkVM)
        unsafe {
            let head = self.head.get();
            let heap_end = ptr::addr_of!(__heap_end) as usize;

            // Align current position up to required alignment
            let align = layout.align();
            let start = (*head + align - 1) & !(align - 1);
            let new_head = match start.checked_add(layout.size()) {
                Some(new_head) => new_head,
                None => return ptr::null_mut(),
            };

            if new_head > heap_end {
                // Out of memory
                ptr::null_mut()
            } else {
                *head = new_head;
                start as *mut u8
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator: deallocation is a no-op
        // Memory is never reclaimed - appropriate for short-lived zkVM programs
    }
}

#[global_allocator]
static ALLOCATOR: ZkvmBumpAlloc = ZkvmBumpAlloc::new();

#[alloc_error_handler]
fn alloc_error(_: Layout) -> ! {
    #[allow(clippy::empty_loop)]
    loop {}
}

/// Initialize the heap allocator.
///
/// Called automatically by `guest_main!` macro when alloc feature is enabled.
///
/// # Safety
///
/// Must be called exactly once before any heap allocations.
#[inline(never)]
pub unsafe fn init_heap() {
    // SAFETY: Caller guarantees this is called once at program start
    unsafe {
        ALLOCATOR.init();
    }
}
