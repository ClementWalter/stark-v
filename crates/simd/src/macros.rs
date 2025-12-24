//! Macros for creating aligned vectors.

/// Creates an aligned vector.
///
/// # Examples
///
/// ```
/// #![feature(allocator_api)]
/// use simd::aligned_vec;
///
/// // Create with repeated value
/// let vec = aligned_vec![0u32; 16];
/// assert_eq!(vec.len(), 16);
///
/// // Create from elements
/// let vec = aligned_vec![1u32, 2, 3, 4];
/// assert_eq!(vec.as_slice(), &[1, 2, 3, 4]);
///
/// // Create empty
/// let vec: simd::AlignedVec<u32> = aligned_vec![];
/// assert!(vec.is_empty());
/// ```
#[macro_export]
macro_rules! aligned_vec {
    ($value:expr; $len:expr) => {{
        let mut vec = Vec::with_capacity_in($len, $crate::AlignedAllocator);
        vec.resize($len, $value);
        $crate::AlignedVec(vec)
    }};
    ($($elem:expr),+ $(,)?) => {{
        let mut vec = Vec::new_in($crate::AlignedAllocator);
        $(vec.push($elem);)+
        $crate::AlignedVec(vec)
    }};
    () => {
        $crate::AlignedVec(Vec::new_in($crate::AlignedAllocator))
    };
}

#[cfg(test)]
mod tests {
    use crate::SIMD_ALIGNMENT;

    #[test]
    fn macro_repeat() {
        let v = aligned_vec![7u32; 10];
        assert_eq!(v.as_ptr() as usize % SIMD_ALIGNMENT, 0);
        assert_eq!(v.len(), 10);
        assert!(v.iter().all(|&x| x == 7));
    }

    #[test]
    fn macro_list() {
        let v = aligned_vec![1u32, 2, 3, 4];
        assert_eq!(v.as_ptr() as usize % SIMD_ALIGNMENT, 0);
        assert_eq!(v.as_slice(), &[1, 2, 3, 4]);
    }
}
