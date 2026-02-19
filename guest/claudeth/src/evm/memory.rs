//! EVM Memory implementation
//!
//! This module implements dynamic memory with gas-based expansion following the Ethereum
//! Virtual Machine specification. Memory is a byte-addressable linear array that expands
//! in 32-byte words when accessed beyond its current size.
//!
//! ## Memory Expansion
//!
//! Memory expansion follows Ethereum's quadratic gas cost model:
//! - `memory_size_word = (size + 31) / 32`
//! - `gas_cost = memory_size_word² / 512 + 3 * memory_size_word`
//!
//! ## Operations
//!
//! - `mload`: Load a 32-byte word from memory
//! - `mstore`: Store a 32-byte word to memory
//! - `mstore8`: Store a single byte to memory
//! - `copy`: Copy memory region (used by MCOPY, CALLDATACOPY, etc.)
//! - `msize`: Get current memory size in bytes

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

use crate::types::U256;

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during memory operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryError {
    /// Memory offset is invalid or out of bounds
    InvalidOffset,
    /// Operation would cause integer overflow
    Overflow,
}

// =============================================================================
// Memory Implementation
// =============================================================================

/// EVM Memory - byte-addressable linear array with dynamic expansion
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    /// Create a new empty memory instance
    pub fn new() -> Self {
        Memory { data: Vec::new() }
    }

    /// Get the current memory size in bytes
    pub fn msize(&self) -> usize {
        self.data.len()
    }

    /// Load a 32-byte word from memory at the given offset
    ///
    /// If the offset + 32 exceeds current memory size, the memory is expanded
    /// with zeroes. Returns the loaded value as U256 in big-endian format.
    pub fn mload(&mut self, offset: usize) -> Result<U256, MemoryError> {
        // Check for overflow: offset + 32
        let end = offset.checked_add(32).ok_or(MemoryError::Overflow)?;

        // Expand memory if necessary (expansion is handled internally)
        self.ensure_capacity(end)?;

        // Load 32 bytes as big-endian U256
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&self.data[offset..end]);
        Ok(U256::from_be_bytes(bytes))
    }

    /// Store a 32-byte word to memory at the given offset
    ///
    /// If the offset + 32 exceeds current memory size, the memory is expanded
    /// with zeroes. The value is stored in big-endian format.
    pub fn mstore(&mut self, offset: usize, value: U256) -> Result<(), MemoryError> {
        // Check for overflow: offset + 32
        let end = offset.checked_add(32).ok_or(MemoryError::Overflow)?;

        // Expand memory if necessary
        self.ensure_capacity(end)?;

        // Store value as big-endian bytes
        let bytes = value.to_be_bytes();
        self.data[offset..end].copy_from_slice(&bytes);
        Ok(())
    }

    /// Store a single byte to memory at the given offset
    ///
    /// If the offset + 1 exceeds current memory size, the memory is expanded.
    /// Only the least significant byte of the input is stored.
    pub fn mstore8(&mut self, offset: usize, byte: u8) -> Result<(), MemoryError> {
        // Check for overflow: offset + 1
        let end = offset.checked_add(1).ok_or(MemoryError::Overflow)?;

        // Expand memory if necessary
        self.ensure_capacity(end)?;

        // Store single byte
        self.data[offset] = byte;
        Ok(())
    }

    /// Copy memory from src to dest for len bytes
    ///
    /// This operation is used by MCOPY, CODECOPY, CALLDATACOPY, etc.
    /// Both source and destination regions are bounds-checked and memory
    /// is expanded as needed. Overlapping regions are handled correctly.
    pub fn copy(&mut self, dest: usize, src: usize, len: usize) -> Result<(), MemoryError> {
        if len == 0 {
            return Ok(());
        }

        // Check for overflow in end positions
        let src_end = src.checked_add(len).ok_or(MemoryError::Overflow)?;
        let dest_end = dest.checked_add(len).ok_or(MemoryError::Overflow)?;

        // Expand memory to accommodate both src and dest regions
        let max_end = src_end.max(dest_end);
        self.ensure_capacity(max_end)?;

        // Handle overlapping regions correctly with copy_within
        if src < self.data.len() && src_end <= self.data.len() {
            self.data.copy_within(src..src_end, dest);
        } else {
            // Source region is at least partially beyond current memory
            // This should not happen after ensure_capacity, but handle defensively
            return Err(MemoryError::InvalidOffset);
        }

        Ok(())
    }

    /// Expand memory to the given size and return the gas cost for expansion
    ///
    /// Memory is expanded in 32-byte words. The gas cost follows Ethereum's
    /// quadratic pricing model:
    /// - `words = (new_size + 31) / 32`
    /// - `gas = words² / 512 + 3 * words`
    ///
    /// Returns the **total** gas cost for the new memory size (not the delta).
    pub fn expand(&mut self, new_size: usize) -> Result<u64, MemoryError> {
        if new_size <= self.data.len() {
            // No expansion needed, return current cost
            return Ok(Self::memory_gas_cost(self.data.len()));
        }

        // Resize memory
        self.data.resize(new_size, 0);

        // Calculate and return new total gas cost
        Ok(Self::memory_gas_cost(new_size))
    }

    /// Ensure memory has at least the given capacity
    ///
    /// This is an internal method that expands memory as needed.
    /// It does not return gas cost (use `expand` for that).
    fn ensure_capacity(&mut self, size: usize) -> Result<(), MemoryError> {
        if size > self.data.len() {
            self.data.resize(size, 0);
        }
        Ok(())
    }

    /// Calculate the total gas cost for a given memory size
    ///
    /// Formula: memory_cost(size) = words² / 512 + 3 * words
    /// where words = (size + 31) / 32
    fn memory_gas_cost(size: usize) -> u64 {
        if size == 0 {
            return 0;
        }

        // Calculate number of 32-byte words (rounded up)
        let words = size.div_ceil(32) as u64;

        // Gas cost formula: words² / 512 + 3 * words
        let quadratic_cost = (words * words) / 512;
        let linear_cost = 3 * words;

        quadratic_cost + linear_cost
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // Basic Operations Tests
    // =============================================================================

    #[test]
    fn test_memory_new() {
        let mem = Memory::new();
        assert_eq!(mem.msize(), 0);
    }

    #[test]
    fn test_memory_default() {
        let mem = Memory::default();
        assert_eq!(mem.msize(), 0);
    }

    #[test]
    fn test_mstore_and_mload() {
        let mut mem = Memory::new();
        let value = U256::from(0x123456789abcdef0u64);

        mem.mstore(0, value).unwrap();
        assert_eq!(mem.msize(), 32);

        let loaded = mem.mload(0).unwrap();
        assert_eq!(loaded, value);
    }

    #[test]
    fn test_mstore_at_offset() {
        let mut mem = Memory::new();
        let value = U256::from(42u64);

        mem.mstore(64, value).unwrap();
        assert_eq!(mem.msize(), 96); // 64 + 32

        let loaded = mem.mload(64).unwrap();
        assert_eq!(loaded, value);
    }

    #[test]
    fn test_mstore8_basic() {
        let mut mem = Memory::new();

        mem.mstore8(0, 0xFF).unwrap();
        assert_eq!(mem.msize(), 1);

        let loaded = mem.mload(0).unwrap();
        let bytes = loaded.to_be_bytes();
        assert_eq!(bytes[0], 0xFF);
        assert_eq!(bytes[1], 0x00); // Rest should be zero
    }

    #[test]
    fn test_mstore8_multiple_bytes() {
        let mut mem = Memory::new();

        mem.mstore8(0, 0xAA).unwrap();
        mem.mstore8(1, 0xBB).unwrap();
        mem.mstore8(2, 0xCC).unwrap();

        let loaded = mem.mload(0).unwrap();
        let bytes = loaded.to_be_bytes();
        assert_eq!(bytes[0], 0xAA);
        assert_eq!(bytes[1], 0xBB);
        assert_eq!(bytes[2], 0xCC);
    }

    #[test]
    fn test_mload_expands_with_zeros() {
        let mut mem = Memory::new();

        // Load from uninitialized memory
        let loaded = mem.mload(0).unwrap();
        assert_eq!(loaded, U256::ZERO);
        assert_eq!(mem.msize(), 32);
    }

    #[test]
    fn test_mload_partial_overlap() {
        let mut mem = Memory::new();
        let value = U256::from(0xFFFFFFFFFFFFFFFFu64);

        mem.mstore(0, value).unwrap();

        // Load overlapping with first store (offset 16)
        // The U256 value has 0xFF in the last 8 bytes (little-endian in value, but stored big-endian)
        // When stored at offset 0, bytes 24-31 will have 0xFF
        // When we load at offset 16, we read bytes 16-47
        // Bytes 16-23 should be 0x00 (from the stored value's high bytes)
        // Bytes 24-31 should be 0xFF (from the stored value's low bytes)
        // Bytes 32-47 should be 0x00 (expanded memory)
        let loaded = mem.mload(16).unwrap();
        let bytes = loaded.to_be_bytes();

        // Bytes 0-7 of loaded value come from bytes 16-23 of memory (zeros from high part of U256)
        assert_eq!(bytes[0], 0x00);
        // Bytes 8-15 of loaded value come from bytes 24-31 of memory (0xFF from U256 low bytes)
        assert_eq!(bytes[8], 0xFF);
        // Bytes 16-31 of loaded value come from bytes 32-47 of memory (expanded zeros)
        assert_eq!(bytes[16], 0x00);
    }

    // =============================================================================
    // Memory Expansion Tests
    // =============================================================================

    #[test]
    fn test_expand_from_zero() {
        let mut mem = Memory::new();

        let gas = mem.expand(32).unwrap();
        assert_eq!(mem.msize(), 32);

        // Expected: words=1, cost = 1²/512 + 3*1 = 0 + 3 = 3
        assert_eq!(gas, 3);
    }

    #[test]
    fn test_expand_multiple_words() {
        let mut mem = Memory::new();

        // Expand to 64 bytes (2 words)
        let gas = mem.expand(64).unwrap();
        assert_eq!(mem.msize(), 64);

        // Expected: words=2, cost = 2²/512 + 3*2 = 0 + 6 = 6
        assert_eq!(gas, 6);
    }

    #[test]
    fn test_expand_with_rounding() {
        let mut mem = Memory::new();

        // Expand to 33 bytes (should round up to 2 words = 64 bytes of gas cost)
        let gas = mem.expand(33).unwrap();
        assert_eq!(mem.msize(), 33);

        // Expected: words=2 (rounded up), cost = 2²/512 + 3*2 = 0 + 6 = 6
        assert_eq!(gas, 6);
    }

    #[test]
    fn test_expand_large_memory() {
        let mut mem = Memory::new();

        // Expand to 1024 bytes (32 words)
        let gas = mem.expand(1024).unwrap();
        assert_eq!(mem.msize(), 1024);

        // Expected: words=32, cost = 32²/512 + 3*32 = 1024/512 + 96 = 2 + 96 = 98
        assert_eq!(gas, 98);
    }

    #[test]
    fn test_expand_no_shrink() {
        let mut mem = Memory::new();

        mem.expand(64).unwrap();
        let gas = mem.expand(32).unwrap();

        // Memory should not shrink
        assert_eq!(mem.msize(), 64);

        // Gas cost is for current size (64 bytes)
        assert_eq!(gas, 6);
    }

    #[test]
    fn test_memory_gas_cost_formula() {
        // Test the gas cost formula directly
        assert_eq!(Memory::memory_gas_cost(0), 0);
        assert_eq!(Memory::memory_gas_cost(32), 3); // 1 word
        assert_eq!(Memory::memory_gas_cost(64), 6); // 2 words
        assert_eq!(Memory::memory_gas_cost(96), 9); // 3 words
        assert_eq!(Memory::memory_gas_cost(1024), 98); // 32 words
    }

    #[test]
    fn test_memory_gas_cost_quadratic() {
        // Verify quadratic growth
        let cost_1k = Memory::memory_gas_cost(1024); // 32 words
        let cost_2k = Memory::memory_gas_cost(2048); // 64 words

        // Cost should more than double (quadratic component)
        assert!(cost_2k > cost_1k * 2);

        // Exact values:
        // 32 words: 32²/512 + 3*32 = 2 + 96 = 98
        // 64 words: 64²/512 + 3*64 = 8 + 192 = 200
        assert_eq!(cost_1k, 98);
        assert_eq!(cost_2k, 200);
    }

    // =============================================================================
    // Memory Copy Tests
    // =============================================================================

    #[test]
    fn test_copy_basic() {
        let mut mem = Memory::new();

        // Write data to source
        mem.mstore(0, U256::from(0xDEADBEEFu64)).unwrap();

        // Copy to different location
        mem.copy(64, 0, 32).unwrap();

        // Verify source is unchanged
        assert_eq!(mem.mload(0).unwrap(), U256::from(0xDEADBEEFu64));

        // Verify destination has copied data
        assert_eq!(mem.mload(64).unwrap(), U256::from(0xDEADBEEFu64));
    }

    #[test]
    fn test_copy_zero_length() {
        let mut mem = Memory::new();

        mem.mstore(0, U256::from(42u64)).unwrap();
        mem.copy(32, 0, 0).unwrap();

        // Memory size should not change beyond initial store
        assert_eq!(mem.msize(), 32);
    }

    #[test]
    fn test_copy_overlapping_forward() {
        let mut mem = Memory::new();

        // Set up initial data
        mem.mstore8(0, 0xAA).unwrap();
        mem.mstore8(1, 0xBB).unwrap();
        mem.mstore8(2, 0xCC).unwrap();
        mem.mstore8(3, 0xDD).unwrap();

        // Copy forward (overlapping)
        mem.copy(2, 0, 4).unwrap();

        // Check result
        assert_eq!(mem.data[0], 0xAA);
        assert_eq!(mem.data[1], 0xBB);
        assert_eq!(mem.data[2], 0xAA); // Copied from offset 0
        assert_eq!(mem.data[3], 0xBB); // Copied from offset 1
        assert_eq!(mem.data[4], 0xCC); // Copied from offset 2
        assert_eq!(mem.data[5], 0xDD); // Copied from offset 3
    }

    #[test]
    fn test_copy_overlapping_backward() {
        let mut mem = Memory::new();

        // Set up initial data
        // After these operations memory is: [0x00, 0x00, 0xAA, 0xBB, 0xCC, 0xDD]
        mem.mstore8(2, 0xAA).unwrap();
        mem.mstore8(3, 0xBB).unwrap();
        mem.mstore8(4, 0xCC).unwrap();
        mem.mstore8(5, 0xDD).unwrap();

        // Copy backward (overlapping): copy bytes 2-5 to positions 0-3
        mem.copy(0, 2, 4).unwrap();

        // Check result - should be: [0xAA, 0xBB, 0xCC, 0xDD, 0xCC, 0xDD]
        assert_eq!(mem.data[0], 0xAA); // Copied from offset 2
        assert_eq!(mem.data[1], 0xBB); // Copied from offset 3
        assert_eq!(mem.data[2], 0xCC); // Copied from offset 4 (overwrites original 0xAA)
        assert_eq!(mem.data[3], 0xDD); // Copied from offset 5 (overwrites original 0xBB)
        assert_eq!(mem.data[4], 0xCC); // Original
        assert_eq!(mem.data[5], 0xDD); // Original
    }

    #[test]
    fn test_copy_expands_memory() {
        let mut mem = Memory::new();

        mem.mstore8(0, 0xFF).unwrap();

        // Copy to high offset
        mem.copy(100, 0, 1).unwrap();

        // Memory should expand
        assert!(mem.msize() >= 101);
        assert_eq!(mem.data[100], 0xFF);
    }

    // =============================================================================
    // Edge Cases and Error Tests
    // =============================================================================

    #[test]
    fn test_mstore_at_max_safe_offset() {
        let mut mem = Memory::new();

        // Use a large but safe offset
        let offset = 1_000_000;
        mem.mstore(offset, U256::from(42u64)).unwrap();

        assert_eq!(mem.mload(offset).unwrap(), U256::from(42u64));
    }

    #[test]
    fn test_mload_at_offset_zero() {
        let mut mem = Memory::new();

        let loaded = mem.mload(0).unwrap();
        assert_eq!(loaded, U256::ZERO);
    }

    #[test]
    fn test_mstore8_at_word_boundary() {
        let mut mem = Memory::new();

        mem.mstore8(31, 0xFF).unwrap();
        mem.mstore8(32, 0xAA).unwrap();

        let word0 = mem.mload(0).unwrap();
        let word1 = mem.mload(32).unwrap();

        let bytes0 = word0.to_be_bytes();
        let bytes1 = word1.to_be_bytes();

        assert_eq!(bytes0[31], 0xFF);
        assert_eq!(bytes1[0], 0xAA);
    }

    #[test]
    fn test_multiple_mstore_operations() {
        let mut mem = Memory::new();

        for i in 0..10 {
            mem.mstore(i * 32, U256::from((i + 1) as u64)).unwrap();
        }

        for i in 0..10 {
            assert_eq!(mem.mload(i * 32).unwrap(), U256::from((i + 1) as u64));
        }
    }

    #[test]
    fn test_mstore_overwrite() {
        let mut mem = Memory::new();

        mem.mstore(0, U256::from(111u64)).unwrap();
        mem.mstore(0, U256::from(222u64)).unwrap();

        assert_eq!(mem.mload(0).unwrap(), U256::from(222u64));
    }

    #[test]
    fn test_copy_full_memory() {
        let mut mem = Memory::new();

        // Fill first 64 bytes
        for i in 0..64 {
            mem.mstore8(i, i as u8).unwrap();
        }

        // Copy to second 64 bytes
        mem.copy(64, 0, 64).unwrap();

        // Verify
        for i in 0..64 {
            assert_eq!(mem.data[i], mem.data[64 + i]);
        }
    }

    #[test]
    fn test_memory_expansion_preserves_data() {
        let mut mem = Memory::new();

        mem.mstore(0, U256::from(0xDEADBEEFu64)).unwrap();
        let before = mem.mload(0).unwrap();

        mem.expand(1000).unwrap();

        let after = mem.mload(0).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn test_msize_after_operations() {
        let mut mem = Memory::new();
        assert_eq!(mem.msize(), 0);

        mem.mstore(0, U256::ZERO).unwrap();
        assert_eq!(mem.msize(), 32);

        mem.mstore8(100, 0).unwrap();
        assert_eq!(mem.msize(), 101);

        mem.copy(200, 0, 32).unwrap();
        assert_eq!(mem.msize(), 232);
    }

    // =============================================================================
    // Integration Tests
    // =============================================================================

    #[test]
    fn test_realistic_evm_scenario() {
        let mut mem = Memory::new();

        // Simulate a contract storing data
        mem.mstore(0x40, U256::from(0x80u64)).unwrap(); // Free memory pointer
        mem.mstore(0x80, U256::from(0x20u64)).unwrap(); // Array length
        mem.mstore(0xa0, U256::from(0xDEADBEEFu64)).unwrap(); // Array data

        // Verify all stores
        assert_eq!(mem.mload(0x40).unwrap(), U256::from(0x80u64));
        assert_eq!(mem.mload(0x80).unwrap(), U256::from(0x20u64));
        assert_eq!(mem.mload(0xa0).unwrap(), U256::from(0xDEADBEEFu64));
    }

    #[test]
    fn test_memory_clone() {
        let mut mem1 = Memory::new();
        mem1.mstore(0, U256::from(42u64)).unwrap();

        let mut mem2 = mem1.clone();

        assert_eq!(mem1.mload(0).unwrap(), mem2.mload(0).unwrap());
    }

    #[test]
    fn test_memory_equality() {
        let mut mem1 = Memory::new();
        let mut mem2 = Memory::new();

        mem1.mstore(0, U256::from(42u64)).unwrap();
        mem2.mstore(0, U256::from(42u64)).unwrap();

        assert_eq!(mem1, mem2);
    }

    #[test]
    fn test_large_offset_with_small_copy() {
        let mut mem = Memory::new();

        // Write at high offset
        mem.mstore(10000, U256::from(123u64)).unwrap();

        // Copy small amount
        mem.copy(20000, 10000, 32).unwrap();

        assert_eq!(mem.mload(20000).unwrap(), U256::from(123u64));
    }

    // =============================================================================
    // Gas Calculation Verification Tests
    // =============================================================================

    #[test]
    fn test_gas_cost_edge_cases() {
        assert_eq!(Memory::memory_gas_cost(1), 3); // 1 byte = 1 word
        assert_eq!(Memory::memory_gas_cost(31), 3); // 31 bytes = 1 word
        assert_eq!(Memory::memory_gas_cost(32), 3); // 32 bytes = 1 word
        assert_eq!(Memory::memory_gas_cost(33), 6); // 33 bytes = 2 words
    }

    #[test]
    fn test_expand_returns_total_cost_not_delta() {
        let mut mem = Memory::new();

        let cost1 = mem.expand(32).unwrap();
        assert_eq!(cost1, 3);

        let cost2 = mem.expand(64).unwrap();
        assert_eq!(cost2, 6); // Total cost, not delta
    }
}
