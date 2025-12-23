//! SIMD utilities for aligned vector operations.
//!
//! Provides 64-byte aligned vectors for optimal SIMD performance (AVX-512 / 16x u32).

use std::alloc::{Layout, alloc, alloc_zeroed, realloc};
use std::fmt;
use std::mem;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::ptr::NonNull;
use std::slice;

/// Alignment for SIMD operations (64 bytes = 16 x u32 for AVX-512).
pub const SIMD_ALIGNMENT: usize = 64;

/// A vector with guaranteed 64-byte alignment for SIMD operations.
///
/// This is a thin wrapper around a raw allocation that maintains alignment.
/// It implements `Deref<Target = [T]>` for convenient slice access.
pub struct AlignedVec<T> {
    ptr: NonNull<T>,
    len: usize,
    capacity: usize,
}

impl<T: fmt::Debug> fmt::Debug for AlignedVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display as a slice, like a regular Vec
        f.debug_list().entries(self.iter()).finish()
    }
}

// SAFETY: AlignedVec owns its data and T: Send implies the vec is Send
unsafe impl<T: Send> Send for AlignedVec<T> {}

impl<T> AlignedVec<T> {
    /// Creates a new empty aligned vector.
    #[inline]
    pub fn new() -> Self {
        Self {
            ptr: NonNull::dangling(),
            len: 0,
            capacity: 0,
        }
    }

    /// Creates a new aligned vector with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        if capacity == 0 || mem::size_of::<T>() == 0 {
            return Self::new();
        }

        let layout = Self::layout_for_capacity(capacity);
        // SAFETY: layout has non-zero size (checked above)
        let ptr = unsafe { alloc(layout) as *mut T };
        let ptr = NonNull::new(ptr).expect("allocation failed");

        Self {
            ptr,
            len: 0,
            capacity,
        }
    }

    /// Creates an aligned vector filled with `value` repeated `len` times.
    pub fn from_elem(value: T, len: usize) -> Self
    where
        T: Clone,
    {
        let mut vec = Self::with_capacity(len);
        for _ in 0..len {
            vec.push(value.clone());
        }
        vec
    }

    /// Creates an aligned vector from a slice.
    pub fn from_slice(slice: &[T]) -> Self
    where
        T: Clone,
    {
        let mut vec = Self::with_capacity(slice.len());
        for item in slice {
            vec.push(item.clone());
        }
        vec
    }

    /// Creates an aligned vector with `len` elements, all initialized to zero.
    pub fn zeroed(len: usize) -> Self
    where
        T: Copy,
    {
        if len == 0 || mem::size_of::<T>() == 0 {
            return Self::new();
        }

        let layout = Self::layout_for_capacity(len);
        // SAFETY: layout has non-zero size
        let ptr = unsafe { alloc_zeroed(layout) as *mut T };
        let ptr = NonNull::new(ptr).expect("allocation failed");

        Self {
            ptr,
            len,
            capacity: len,
        }
    }

    fn layout_for_capacity(capacity: usize) -> Layout {
        let size = capacity * mem::size_of::<T>();
        // SAFETY: alignment is always a power of 2 and size won't overflow
        Layout::from_size_align(size, SIMD_ALIGNMENT).expect("invalid layout")
    }

    /// Returns the number of elements in the vector.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the vector contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the capacity of the vector.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns a raw pointer to the vector's buffer.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.ptr.as_ptr()
    }

    /// Returns a mutable raw pointer to the vector's buffer.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Appends an element to the back of the vector.
    #[inline]
    pub fn push(&mut self, value: T) {
        if self.len == self.capacity {
            self.grow();
        }
        // SAFETY: we just ensured there's capacity
        unsafe {
            self.ptr.as_ptr().add(self.len).write(value);
        }
        self.len += 1;
    }

    /// Removes and returns the last element, or `None` if empty.
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            // SAFETY: len was > 0, so this is valid
            Some(unsafe { self.ptr.as_ptr().add(self.len).read() })
        }
    }

    /// Clears the vector, removing all elements.
    pub fn clear(&mut self) {
        // Drop all elements
        while self.pop().is_some() {}
    }

    /// Extends the vector with elements from an iterator.
    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.push(item);
        }
    }

    /// Reserves capacity for at least `additional` more elements.
    pub fn reserve(&mut self, additional: usize) {
        let required = self.len.saturating_add(additional);
        if required > self.capacity {
            self.grow_to(required);
        }
    }

    fn grow(&mut self) {
        let new_capacity = if self.capacity == 0 {
            // Start with enough for one SIMD lane (16 u32s)
            (SIMD_ALIGNMENT / mem::size_of::<T>()).max(1)
        } else {
            self.capacity * 2
        };
        self.grow_to(new_capacity);
    }

    fn grow_to(&mut self, new_capacity: usize) {
        if mem::size_of::<T>() == 0 {
            self.capacity = new_capacity;
            return;
        }

        let new_layout = Self::layout_for_capacity(new_capacity);

        let new_ptr = if self.capacity == 0 {
            // SAFETY: new_layout has non-zero size
            unsafe { alloc(new_layout) as *mut T }
        } else {
            let old_layout = Self::layout_for_capacity(self.capacity);
            // SAFETY: ptr was allocated with old_layout, new_layout has same alignment
            unsafe {
                realloc(self.ptr.as_ptr() as *mut u8, old_layout, new_layout.size()) as *mut T
            }
        };

        self.ptr = NonNull::new(new_ptr).expect("allocation failed");
        self.capacity = new_capacity;
    }
}

impl<T> Default for AlignedVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for AlignedVec<T> {
    fn drop(&mut self) {
        if self.capacity == 0 || mem::size_of::<T>() == 0 {
            return;
        }

        // Drop all elements
        for i in 0..self.len {
            // SAFETY: i < len, so this is valid
            unsafe {
                self.ptr.as_ptr().add(i).drop_in_place();
            }
        }

        // Deallocate
        let layout = Self::layout_for_capacity(self.capacity);
        // SAFETY: ptr was allocated with this layout
        unsafe {
            std::alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
        }
    }
}

impl<T: Clone> Clone for AlignedVec<T> {
    fn clone(&self) -> Self {
        Self::from_slice(self)
    }
}

impl<T> Deref for AlignedVec<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        if self.len == 0 {
            &[]
        } else {
            // SAFETY: ptr is valid for len elements
            unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
        }
    }
}

impl<T> DerefMut for AlignedVec<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        if self.len == 0 {
            &mut []
        } else {
            // SAFETY: ptr is valid for len elements
            unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
        }
    }
}

impl<T> Index<usize> for AlignedVec<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &T {
        &(**self)[index]
    }
}

impl<T> IndexMut<usize> for AlignedVec<T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut T {
        &mut (**self)[index]
    }
}

impl<T> AsRef<[T]> for AlignedVec<T> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T> AsMut<[T]> for AlignedVec<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

/// Creates an aligned vector.
///
/// - `aligned_vec![value; len]` creates a vector with `len` copies of `value`
/// - `aligned_vec![v1, v2, ...]` creates a vector from the given elements
#[macro_export]
macro_rules! aligned_vec {
    ($value:expr; $len:expr) => {
        $crate::AlignedVec::from_elem($value, $len)
    };
    ($($elem:expr),+ $(,)?) => {
        $crate::AlignedVec::from_slice(&[$($elem),+])
    };
    () => {
        $crate::AlignedVec::new()
    };
}

/// Helper function for creating aligned vectors (used by macro).
#[inline]
pub fn aligned_vec<T: Clone>(value: T, len: usize) -> AlignedVec<T> {
    AlignedVec::from_elem(value, len)
}

/// Helper function for creating aligned vectors from slices (used by macro).
#[inline]
pub fn aligned_vec_from_slice<T: Clone>(slice: &[T]) -> AlignedVec<T> {
    AlignedVec::from_slice(slice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment() {
        let vec: AlignedVec<u32> = AlignedVec::with_capacity(16);
        assert_eq!(vec.as_ptr() as usize % SIMD_ALIGNMENT, 0);
    }

    #[test]
    fn test_push_and_access() {
        let mut vec = AlignedVec::new();
        vec.push(1u32);
        vec.push(2);
        vec.push(3);

        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0], 1);
        assert_eq!(vec[1], 2);
        assert_eq!(vec[2], 3);
    }

    #[test]
    fn test_from_elem() {
        let vec = AlignedVec::from_elem(42u32, 16);
        assert_eq!(vec.len(), 16);
        assert!(vec.iter().all(|&x| x == 42));
    }

    #[test]
    fn test_aligned_vec_macro() {
        let vec1 = aligned_vec![0u32; 16];
        assert_eq!(vec1.len(), 16);

        let vec2 = aligned_vec![1u32, 2, 3, 4];
        assert_eq!(vec2.len(), 4);
        assert_eq!(&*vec2, &[1, 2, 3, 4]);
    }

    #[test]
    fn test_zeroed() {
        let vec: AlignedVec<u32> = AlignedVec::zeroed(16);
        assert_eq!(vec.len(), 16);
        assert!(vec.iter().all(|&x| x == 0));
    }

    #[test]
    fn test_grow() {
        let mut vec = AlignedVec::new();
        for i in 0..1000u32 {
            vec.push(i);
        }
        assert_eq!(vec.len(), 1000);
        for i in 0..1000 {
            assert_eq!(vec[i], i as u32);
        }
        // Check alignment is preserved after growth
        assert_eq!(vec.as_ptr() as usize % SIMD_ALIGNMENT, 0);
    }
}
