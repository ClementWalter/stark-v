use std::simd::u32x16;
use std::slice;

use stwo::prover::backend::simd::column::BaseColumn;
use stwo::prover::backend::simd::m31::PackedBaseField;

use crate::allocator::AlignedAllocator;

/// Number of u32 elements in a u32x16 SIMD vector.
pub const U32X16_LANES: usize = 16;

/// A vector with guaranteed 64-byte alignment for SIMD operations.
pub type AlignedVec<T> = Vec<T, AlignedAllocator>;

// ============================================================================
// SIMD Conversion Functions
// ============================================================================

/// Returns the slice as a slice of `u32x16` SIMD vectors.
///
/// # Panics
///
/// Panics if the length is not a multiple of 16.
#[inline]
pub fn as_simd_slice(vec: &AlignedVec<u32>) -> &[u32x16] {
    assert!(
        vec.len().is_multiple_of(U32X16_LANES),
        "length must be a multiple of 16, got {}",
        vec.len()
    );
    let simd_len = vec.len() / U32X16_LANES;
    unsafe { slice::from_raw_parts(vec.as_ptr() as *const u32x16, simd_len) }
}

/// Returns the slice as a mutable slice of `u32x16` SIMD vectors.
///
/// # Panics
///
/// Panics if the length is not a multiple of 16.
#[inline]
pub fn as_simd_slice_mut(vec: &mut AlignedVec<u32>) -> &mut [u32x16] {
    assert!(
        vec.len().is_multiple_of(U32X16_LANES),
        "length must be a multiple of 16, got {}",
        vec.len()
    );
    let simd_len = vec.len() / U32X16_LANES;
    unsafe { slice::from_raw_parts_mut(vec.as_mut_ptr() as *mut u32x16, simd_len) }
}

/// Converts an `AlignedVec<u32>` to a `BaseColumn` for the SIMD backend.
///
/// # Panics
///
/// Panics if the length is not a multiple of 16.
pub fn into_base_column(vec: AlignedVec<u32>) -> BaseColumn {
    let packed: Vec<PackedBaseField> = as_simd_slice(&vec)
        .iter()
        .map(|&v| unsafe { PackedBaseField::from_simd_unchecked(v) })
        .collect();
    BaseColumn::from_simd(packed)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use crate::AlignedAllocator;
    use crate::aligned_vec;
    use crate::allocator::SIMD_ALIGNMENT;

    use super::*;

    #[test]
    fn alignment() {
        let mut vec: AlignedVec<u32> = Vec::with_capacity_in(16, AlignedAllocator);
        vec.push(0);
        assert_eq!(vec.as_ptr() as usize % SIMD_ALIGNMENT, 0);
    }

    #[test]
    fn alignment_preserved_after_growth() {
        let mut vec: AlignedVec<u32> = Vec::new_in(AlignedAllocator);
        for i in 0..1000u32 {
            vec.push(i);
        }
        assert_eq!(vec.as_ptr() as usize % SIMD_ALIGNMENT, 0);
    }

    #[test]
    fn from_elem() {
        let vec = aligned_vec![42u32; 16];
        assert_eq!(vec.len(), 16);
        assert!(vec.iter().all(|&x| x == 42));
    }

    #[test]
    fn simd_slice() {
        let vec = aligned_vec![0u32; 32];
        let simd_slice = as_simd_slice(&vec);
        assert_eq!(simd_slice.len(), 2);
    }

    #[test]
    fn simd_slice_mut() {
        let mut vec = aligned_vec![0u32; 16];
        let simd_slice = as_simd_slice_mut(&mut vec);
        assert_eq!(simd_slice.len(), 1);
        simd_slice[0] = u32x16::splat(42);
        assert!(vec.iter().all(|&x| x == 42));
    }

    #[test]
    #[should_panic(expected = "must be a multiple of 16")]
    fn invalid_length_panics() {
        let vec = aligned_vec![0u32; 10];
        let _ = as_simd_slice(&vec);
    }
}
