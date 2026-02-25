//! Heap allocation test program using `Vec`.
//!
//! This program exercises dynamic memory allocation to verify the bump
//! allocator works inside the zkVM. It reads input bytes, collects them
//! into a `Vec`, sorts with a simple selection sort (avoiding Rust's stdlib
//! sort which needs scratch heap allocations that emit `unimp` on failure),
//! and returns summary statistics.

#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

/// Result of heap-based vector operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HeapVecResult {
    pub input_len: u32,
    pub sorted_first: u8,
    pub sorted_last: u8,
    pub sum: u32,
}

/// In-place selection sort — avoids stdlib sort's scratch buffer allocation
/// which emits `unimp` CSR instructions unsupported by the rv32im VM.
fn selection_sort(v: &mut [u8]) {
    let n = v.len();
    for i in 0..n {
        let mut min_idx = i;
        for j in (i + 1)..n {
            if v[j] < v[min_idx] {
                min_idx = j;
            }
        }
        v.swap(i, min_idx);
    }
}

/// Collect input bytes into a `Vec`, sort them, and return summary stats.
pub fn heap_vec(input: &[u8]) -> HeapVecResult {
    let mut v: Vec<u8> = Vec::with_capacity(input.len());
    for &b in input {
        v.push(b);
    }
    selection_sort(&mut v);

    HeapVecResult {
        input_len: v.len() as u32,
        sorted_first: v.first().copied().unwrap_or(0),
        sorted_last: v.last().copied().unwrap_or(0),
        sum: v.iter().map(|&b| b as u32).sum(),
    }
}

/// Standard test entry point for e2e testing.
pub fn test_call() -> HeapVecResult {
    heap_vec(&[0x03, 0x01, 0x04, 0x01, 0x05, 0x09, 0x02, 0x06])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heap_vec_sorts_input() {
        let result = heap_vec(&[0x03, 0x01, 0x04, 0x01, 0x05]);
        // Sorted: [1, 1, 3, 4, 5]
        assert_eq!(result.sorted_first, 0x01);
        assert_eq!(result.sorted_last, 0x05);
    }

    #[test]
    fn test_heap_vec_empty_input() {
        let result = heap_vec(&[]);
        assert_eq!(result.input_len, 0);
        assert_eq!(result.sorted_first, 0);
        assert_eq!(result.sorted_last, 0);
        assert_eq!(result.sum, 0);
    }

    #[test]
    fn test_heap_vec_single_element() {
        let result = heap_vec(&[42]);
        assert_eq!(result.input_len, 1);
        assert_eq!(result.sorted_first, 42);
        assert_eq!(result.sorted_last, 42);
        assert_eq!(result.sum, 42);
    }

    #[test]
    fn test_heap_vec_sum() {
        let result = heap_vec(&[10, 20, 30]);
        assert_eq!(result.sum, 60);
    }

    #[test]
    fn test_call_result() {
        let result = test_call();
        // Input: [3, 1, 4, 1, 5, 9, 2, 6] — sorted: [1, 1, 2, 3, 4, 5, 6, 9]
        assert_eq!(result.input_len, 8);
        assert_eq!(result.sorted_first, 1);
        assert_eq!(result.sorted_last, 9);
        assert_eq!(result.sum, 31);
    }
}
