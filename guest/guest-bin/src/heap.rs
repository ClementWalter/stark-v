//! Bump allocator for guest heap memory.
//!
//! Uses the free space between `__heap_start` (after static data) and
//! `__heap_end` (end of the DATA region) defined in `linker.ld`.
//! Deallocation is a no-op — standard for zkVM bump allocators since
//! programs run once and never need to reclaim memory.

use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

// Linker symbols marking heap bounds (defined in linker.ld)
unsafe extern "C" {
    static __heap_start: u8;
    static __heap_end: u8;
}

struct BumpAllocator;

#[global_allocator]
static HEAP: BumpAllocator = BumpAllocator;

/// Next free address in the heap. Zero means uninitialized — the first
/// allocation lazily reads `__heap_start` from the linker symbol.
static mut NEXT: usize = 0;

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut next = unsafe { NEXT };
        if next == 0 {
            // Lazy init from linker symbol on first allocation
            next = ptr::addr_of!(__heap_start) as usize;
        }

        let aligned = (next + layout.align() - 1) & !(layout.align() - 1);
        let new_next = aligned + layout.size();
        let end = ptr::addr_of!(__heap_end) as usize;

        if new_next > end {
            // Out of heap memory
            ptr::null_mut()
        } else {
            unsafe { NEXT = new_next };
            aligned as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // No-op: bump allocators never free in single-run zkVM programs
    }
}
