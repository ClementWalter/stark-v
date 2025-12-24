use std::alloc::{AllocError, Allocator, Global, Layout};
use std::ptr::NonNull;

/// Alignment for SIMD operations (64 bytes = 16 × u32 for AVX-512).
pub const SIMD_ALIGNMENT: usize = 64;

// ============================================================================
// Aligned Allocator
// ============================================================================

/// A custom allocator that guarantees 64-byte alignment for SIMD operations.
#[derive(Debug, Clone, Copy, Default)]
pub struct AlignedAllocator;

unsafe impl Allocator for AlignedAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let aligned_layout = layout
            .align_to(SIMD_ALIGNMENT)
            .map_err(|_| AllocError)?
            .pad_to_align();
        Global.allocate(aligned_layout)
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let aligned_layout = layout
            .align_to(SIMD_ALIGNMENT)
            .map_err(|_| AllocError)?
            .pad_to_align();
        Global.allocate_zeroed(aligned_layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let aligned_layout = layout
            .align_to(SIMD_ALIGNMENT)
            .expect("invalid layout")
            .pad_to_align();
        unsafe { Global.deallocate(ptr, aligned_layout) }
    }

    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let old_aligned = old_layout
            .align_to(SIMD_ALIGNMENT)
            .map_err(|_| AllocError)?
            .pad_to_align();
        let new_aligned = new_layout
            .align_to(SIMD_ALIGNMENT)
            .map_err(|_| AllocError)?
            .pad_to_align();
        unsafe { Global.grow(ptr, old_aligned, new_aligned) }
    }

    unsafe fn grow_zeroed(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let old_aligned = old_layout
            .align_to(SIMD_ALIGNMENT)
            .map_err(|_| AllocError)?
            .pad_to_align();
        let new_aligned = new_layout
            .align_to(SIMD_ALIGNMENT)
            .map_err(|_| AllocError)?
            .pad_to_align();
        unsafe { Global.grow_zeroed(ptr, old_aligned, new_aligned) }
    }

    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let old_aligned = old_layout
            .align_to(SIMD_ALIGNMENT)
            .map_err(|_| AllocError)?
            .pad_to_align();
        let new_aligned = new_layout
            .align_to(SIMD_ALIGNMENT)
            .map_err(|_| AllocError)?
            .pad_to_align();
        unsafe { Global.shrink(ptr, old_aligned, new_aligned) }
    }
}
