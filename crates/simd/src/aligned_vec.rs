use std::ops::{Deref, DerefMut};
use std::simd::u32x16;
use std::slice;

use stwo::prover::backend::simd::column::BaseColumn;
use stwo::prover::backend::simd::m31::PackedBaseField;

use crate::allocator::AlignedAllocator;

/// Number of u32 elements in a u32x16 SIMD vector.
pub const U32X16_LANES: usize = 16;

/// A vector with guaranteed 64-byte alignment for SIMD operations.
#[derive(Clone)]
pub struct AlignedVec<T>(pub Vec<T, AlignedAllocator>);

impl<T> AlignedVec<T> {
    /// Creates a new empty aligned vector.
    #[inline]
    pub fn new() -> Self {
        Self(Vec::new_in(AlignedAllocator))
    }

    /// Creates a new aligned vector with the specified capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity_in(capacity, AlignedAllocator))
    }
}

impl<T> Default for AlignedVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Deref for AlignedVec<T> {
    type Target = Vec<T, AlignedAllocator>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for AlignedVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<[u32x16]> for AlignedVec<u32> {
    /// Returns the slice as a slice of `u32x16` SIMD vectors.
    ///
    /// # Panics
    ///
    /// Panics if the length is not a multiple of 16.
    #[inline]
    fn as_ref(&self) -> &[u32x16] {
        assert!(
            self.len().is_multiple_of(U32X16_LANES),
            "length must be a multiple of 16, got {}",
            self.len()
        );
        let simd_len = self.len() / U32X16_LANES;
        unsafe { slice::from_raw_parts(self.0.as_ptr() as *const u32x16, simd_len) }
    }
}

impl AsMut<[u32x16]> for AlignedVec<u32> {
    /// Returns the slice as a mutable slice of `u32x16` SIMD vectors.
    ///
    /// # Panics
    ///
    /// Panics if the length is not a multiple of 16.
    #[inline]
    fn as_mut(&mut self) -> &mut [u32x16] {
        assert!(
            self.len().is_multiple_of(U32X16_LANES),
            "length must be a multiple of 16, got {}",
            self.len()
        );
        let simd_len = self.len() / U32X16_LANES;
        unsafe { slice::from_raw_parts_mut(self.0.as_mut_ptr() as *mut u32x16, simd_len) }
    }
}

impl From<AlignedVec<u32>> for BaseColumn {
    /// Converts an `AlignedVec<u32>` into a `BaseColumn` for the SIMD backend.
    ///
    /// # Panics
    ///
    /// Panics if the length is not a multiple of 16.
    fn from(vec: AlignedVec<u32>) -> Self {
        let packed: Vec<PackedBaseField> = vec
            .as_ref()
            .iter()
            .map(|&v| unsafe { PackedBaseField::from_simd_unchecked(v) })
            .collect();
        BaseColumn::from_simd(packed)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use std::simd::u32x16;

    use crate::aligned_vec;
    use crate::allocator::SIMD_ALIGNMENT;

    use super::AlignedVec;

    #[test]
    fn alignment() {
        let mut vec: AlignedVec<u32> = AlignedVec::with_capacity(16);
        vec.push(0);
        assert_eq!(vec.as_ptr() as usize % SIMD_ALIGNMENT, 0);
    }

    #[test]
    fn alignment_preserved_after_growth() {
        let mut vec: AlignedVec<u32> = AlignedVec::new();
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
        let simd_slice: &[u32x16] = vec.as_ref();
        assert_eq!(simd_slice.len(), 2);
    }

    #[test]
    fn simd_slice_mut() {
        let mut vec = aligned_vec![0u32; 16];
        let simd_slice: &mut [u32x16] = vec.as_mut();
        assert_eq!(simd_slice.len(), 1);
        simd_slice[0] = u32x16::splat(42);
        assert!(vec.iter().all(|&x| x == 42));
    }

    #[test]
    #[should_panic(expected = "must be a multiple of 16")]
    fn invalid_length_panics() {
        let vec = aligned_vec![0u32; 10];
        let _: &[u32x16] = vec.as_ref();
    }
}
