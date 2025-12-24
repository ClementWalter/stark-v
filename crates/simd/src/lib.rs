//! SIMD utilities for aligned vector operations.
//!
//! Provides 64-byte aligned vectors for optimal SIMD performance (AVX-512 / 16x u32).
//!
//! # Example
//!
//! ```
//! #![feature(allocator_api)]
//! use simd::aligned_vec;
//!
//! // Create an aligned vector
//! let vec = aligned_vec![0u32; 32];
//!
//! // Convert to SIMD slices for vectorized operations
//! let simd_slice = simd::as_simd_slice(&vec);
//! assert_eq!(simd_slice.len(), 2); // 32 elements / 16 lanes = 2 SIMD vectors
//! ```

#![feature(allocator_api)]
#![feature(portable_simd)]

mod aligned_vec;
mod allocator;
mod macros;

// Re-export public API
pub use aligned_vec::{
    AlignedVec, U32X16_LANES, as_simd_slice, as_simd_slice_mut, into_base_column,
};
pub use allocator::{AlignedAllocator, SIMD_ALIGNMENT};
