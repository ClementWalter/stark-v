//! SIMD utilities for aligned vector operations.
//!
//! Provides 64-byte aligned vectors for optimal SIMD performance (AVX-512 / 16x u32).
//!
//! # Example
//!
//! ```
//! #![feature(allocator_api)]
//! #![feature(portable_simd)]
//! use std::simd::u32x16;
//! use simd::aligned_vec;
//!
//! // Create an aligned vector
//! let vec = aligned_vec![0u32; 32];
//!
//! // Convert to SIMD slices for vectorized operations
//! let simd_slice: &[u32x16] = vec.as_ref();
//! assert_eq!(simd_slice.len(), 2); // 32 elements / 16 lanes = 2 SIMD vectors
//! ```

#![feature(allocator_api)]
#![feature(portable_simd)]

mod aligned_vec;
mod allocator;
mod macros;

// Re-export public API
pub use aligned_vec::{AlignedVec, U32X16_LANES};
pub use allocator::{AlignedAllocator, SIMD_ALIGNMENT};
