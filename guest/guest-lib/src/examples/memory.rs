//! Memory stress test example.

use crate::types::MemoryTestResult;

/// Memory stress test returning a result struct.
pub fn memory() -> MemoryTestResult {
    MemoryTestResult {
        sum: memory_test_impl(),
    }
}

/// Memory stress test implementation: write and read back array values.
pub fn memory_test_impl() -> u32 {
    let mut arr = [0u32; 16];

    // Write pattern
    let mut i = 0usize;
    while i < 16 {
        arr[i] = (i as u32).wrapping_mul(7).wrapping_add(3);
        i += 1;
    }

    // Read and sum
    let mut sum = 0u32;
    i = 0;
    while i < 16 {
        sum = sum.wrapping_add(arr[i]);
        i += 1;
    }

    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory() {
        assert_eq!(memory(), MemoryTestResult { sum: 888 });
    }

    #[test]
    fn test_memory_impl() {
        // Sum of (i*7+3) for i=0..15 = 7*(0+1+...+15) + 3*16 = 7*120 + 48 = 888
        assert_eq!(memory_test_impl(), 888);
    }
}
