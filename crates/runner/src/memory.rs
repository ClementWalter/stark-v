use std::collections::BTreeMap;

use crate::trace::{Access, Tracer};

/// Sparse byte-addressable memory using BTreeMap.
pub struct Memory {
    data: BTreeMap<u32, u8>,
}

impl Memory {
    /// Create empty memory.
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    /// Read a single byte.
    #[inline]
    pub fn read_u8(&self, addr: u32) -> u8 {
        self.data.get(&addr).copied().unwrap_or(0)
    }

    /// Read a half-word (16-bit, little-endian).
    #[inline]
    pub fn read_u16(&self, addr: u32) -> u16 {
        debug_assert_eq!(addr & 1, 0, "Address must be 2-byte aligned");
        let lo = self.read_u8(addr) as u16;
        let hi = self.read_u8(addr.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    /// Read a word (32-bit, little-endian).
    #[inline]
    pub fn read_u32(&self, addr: u32) -> u32 {
        debug_assert_eq!(addr & 3, 0, "Address must be 4-byte aligned");
        let b0 = self.read_u8(addr) as u32;
        let b1 = self.read_u8(addr.wrapping_add(1)) as u32;
        let b2 = self.read_u8(addr.wrapping_add(2)) as u32;
        let b3 = self.read_u8(addr.wrapping_add(3)) as u32;
        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }

    /// Write a single byte.
    #[inline]
    pub fn write_u8(&mut self, addr: u32, val: u8) {
        self.data.insert(addr, val);
    }

    /// Write a half-word (16-bit, little-endian).
    #[inline]
    pub fn write_u16(&mut self, addr: u32, val: u16) {
        debug_assert_eq!(addr & 1, 0, "Address must be 2-byte aligned");
        self.write_u8(addr, val as u8);
        self.write_u8(addr.wrapping_add(1), (val >> 8) as u8);
    }

    /// Write a word (32-bit, little-endian).
    #[inline]
    pub fn write_u32(&mut self, addr: u32, val: u32) {
        debug_assert_eq!(addr & 3, 0, "Address must be 4-byte aligned");
        self.write_u8(addr, val as u8);
        self.write_u8(addr.wrapping_add(1), (val >> 8) as u8);
        self.write_u8(addr.wrapping_add(2), (val >> 16) as u8);
        self.write_u8(addr.wrapping_add(3), (val >> 24) as u8);
    }

    // =========================================================================
    // Traced access methods - all use 4-byte aligned word access
    // =========================================================================

    /// Read the aligned word value at the given address.
    /// All traced accesses use 4-byte aligned addresses.
    /// If addr is not 4-byte aligned, the returned value is the value of the 4-byte
    /// aligned word containing the byte at addr.
    ///
    /// All traced read methods use this same helper function to trace the aligned word.
    /// Read methods are kept for maintaining Memory interface consistent with the RISC-V
    /// specification.
    #[inline(always)]
    fn read_aligned_word_traced(&self, addr: u32, tracer: &mut Tracer) -> Access {
        let aligned = addr & !3;
        let word = self.read_u32(aligned);
        tracer.trace_mem_access(aligned, word, word)
    }

    /// Read a byte with trace tracking.
    /// Traces the full 4-byte aligned word containing this byte.
    #[inline]
    pub fn read_u8_traced(&self, addr: u32, tracer: &mut Tracer) -> Access {
        self.read_aligned_word_traced(addr, tracer)
    }

    /// Read a half-word with trace tracking.
    /// Traces the full 4-byte aligned word containing this half-word.
    #[inline]
    pub fn read_u16_traced(&self, addr: u32, tracer: &mut Tracer) -> Access {
        debug_assert_eq!(addr & 1, 0, "Address must be 2-byte aligned");
        self.read_aligned_word_traced(addr, tracer)
    }

    /// Read a word with trace tracking.
    #[inline]
    pub fn read_u32_traced(&self, addr: u32, tracer: &mut Tracer) -> Access {
        debug_assert_eq!(addr & 3, 0, "Address must be 4-byte aligned");
        self.read_aligned_word_traced(addr, tracer)
    }

    /// Write a byte with trace tracking.
    /// Traces the full 4-byte aligned word containing this byte.
    #[inline]
    pub fn write_u8_traced(&mut self, addr: u32, val: u8, tracer: &mut Tracer) -> Access {
        let aligned = addr & !3;
        let prev_word = self.read_u32(aligned);
        self.write_u8(addr, val);
        let next_word = self.read_u32(aligned);
        tracer.trace_mem_access(addr, prev_word, next_word)
    }

    /// Write a half-word with trace tracking.
    /// Traces the full 4-byte aligned word containing this half-word.
    #[inline]
    pub fn write_u16_traced(&mut self, addr: u32, val: u16, tracer: &mut Tracer) -> Access {
        debug_assert_eq!(addr & 1, 0, "Address must be 2-byte aligned");
        let aligned = addr & !3;
        let prev_word = self.read_u32(aligned);
        self.write_u16(addr, val);
        let next_word = self.read_u32(aligned);
        tracer.trace_mem_access(addr, prev_word, next_word)
    }

    /// Write a word with trace tracking.
    #[inline]
    pub fn write_u32_traced(&mut self, addr: u32, val: u32, tracer: &mut Tracer) -> Access {
        debug_assert_eq!(addr & 3, 0, "Address must be 4-byte aligned");
        let prev_value = self.read_u32(addr);
        self.write_u32(addr, val);
        tracer.trace_mem_access(addr, prev_value, val)
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<(u32, u8)> for Memory {
    fn from_iter<I: IntoIterator<Item = (u32, u8)>>(iter: I) -> Self {
        Self {
            data: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::trace::Tracer;

    // =========================================================================
    // Basic Operations
    // =========================================================================

    #[test]
    fn test_new_creates_empty_memory() {
        let mem = Memory::new();
        assert_eq!(mem.read_u8(0), 0);
        assert_eq!(mem.read_u8(100), 0);
    }

    #[test]
    fn test_default_same_as_new() {
        let mem = Memory::default();
        assert_eq!(mem.read_u8(0), 0);
    }

    #[test]
    fn test_read_write_u8() {
        let mut mem = Memory::new();
        mem.write_u8(100, 0xAB);
        assert_eq!(mem.read_u8(100), 0xAB);
        assert_eq!(mem.read_u8(101), 0); // Adjacent is still zero
    }

    #[test]
    fn test_read_write_u16_little_endian() {
        let mut mem = Memory::new();
        mem.write_u16(100, 0x1234);
        // Little-endian: low byte first
        assert_eq!(mem.read_u8(100), 0x34);
        assert_eq!(mem.read_u8(101), 0x12);
        assert_eq!(mem.read_u16(100), 0x1234);
    }

    #[test]
    fn test_read_write_u32_little_endian() {
        let mut mem = Memory::new();
        mem.write_u32(100, 0xDEADBEEF);
        // Little-endian: low byte first
        assert_eq!(mem.read_u8(100), 0xEF);
        assert_eq!(mem.read_u8(101), 0xBE);
        assert_eq!(mem.read_u8(102), 0xAD);
        assert_eq!(mem.read_u8(103), 0xDE);
        assert_eq!(mem.read_u32(100), 0xDEADBEEF);
    }

    #[test]
    fn test_from_iterator() {
        let mem: Memory = vec![(0u32, 0x11u8), (1, 0x22), (2, 0x33), (3, 0x44)]
            .into_iter()
            .collect();
        assert_eq!(mem.read_u32(0), 0x44332211);
    }

    // =========================================================================
    // Traced Single-Byte Access
    // =========================================================================

    #[test]
    fn test_read_u8_traced_first_access() {
        let mem = Memory::new();
        let mut tracer = Tracer::default();
        tracer.clk = 10;

        let access = mem.read_u8_traced(100, &mut tracer);

        assert_eq!(access.addr, 100);
        assert_eq!(access.prev, 0);
        assert_eq!(access.next, 0);
        assert_eq!(access.clk_prev, 0);
        assert_eq!(access.clk, 10);
        assert!(tracer.mem_clk_update.is_empty());
    }

    #[test]
    fn test_read_u8_traced_updates_mem_clk() {
        let mem = Memory::new();
        let mut tracer = Tracer::default();
        tracer.clk = 10;

        mem.read_u8_traced(100, &mut tracer);
        assert_eq!(tracer.mem_clk.get(&100), Some(&10));
    }

    #[test]
    fn test_write_u8_traced_records_change() {
        let mut mem = Memory::new();
        mem.write_u8(100, 0x42);
        let mut tracer = Tracer::default();
        tracer.clk = 5;

        let access = mem.write_u8_traced(100, 0xFF, &mut tracer);

        assert_eq!(access.addr, 100);
        assert_eq!(access.prev, 0x42);
        assert_eq!(access.next, 0xFF);
        assert_eq!(access.clk_prev, 0);
        assert_eq!(access.clk, 5);
        assert!(tracer.mem_clk_update.is_empty());

        // Verify memory was updated
        assert_eq!(mem.read_u8(100), 0xFF);
    }

    #[test]
    fn test_traced_consecutive_accesses() {
        let mut mem = Memory::new();
        let mut tracer = Tracer::default();

        // First write at clk=1
        tracer.clk = 1;
        mem.write_u8_traced(100, 0x11, &mut tracer);

        // Second write at clk=2
        tracer.clk = 2;
        let access = mem.write_u8_traced(100, 0x22, &mut tracer);

        assert_eq!(access.clk_prev, 1);
        assert_eq!(access.clk, 2);
        assert_eq!(access.prev, 0x11);
        assert_eq!(access.next, 0x22);
        assert!(tracer.mem_clk_update.is_empty());
    }

    // =========================================================================
    // Traced Multi-Byte Access (Clock Synchronization)
    // =========================================================================

    #[test]
    fn test_read_u32_traced() {
        let mut mem = Memory::new();
        mem.write_u32(100, 0x44332211);
        let mut tracer = Tracer::default();

        tracer.clk = 20;
        let access = mem.read_u32_traced(100, &mut tracer);

        // Aligned address should have the clock
        assert_eq!(tracer.mem_clk.get(&100), Some(&20));

        // Returned access should have full value
        assert_eq!(access.addr, 100);
        assert_eq!(access.prev, 0x44332211);
        assert_eq!(access.next, 0x44332211);
        assert_eq!(access.clk, 20);
    }

    #[test]
    fn test_write_u16_traced() {
        let mut mem = Memory::new();
        let mut tracer = Tracer::default();

        tracer.clk = 10;
        let access = mem.write_u16_traced(100, 0xABCD, &mut tracer);

        // Verify memory written correctly
        assert_eq!(mem.read_u16(100), 0xABCD);

        // Aligned address should have the clock (100 is 4-byte aligned)
        assert_eq!(tracer.mem_clk.get(&100), Some(&10));

        // Returned access should have the full word value
        assert_eq!(access.addr, 100);
        assert_eq!(access.prev, 0); // was uninitialized
        assert_eq!(access.next, 0xABCD); // written u16 in low bytes
    }

    // =========================================================================
    // Intermediate Gap-Filling Accesses (uses Tracer's max_clock_diff)
    // =========================================================================

    #[test]
    fn test_gap_filling_single_byte() {
        let mut mem = Memory::new();
        mem.write_u8(100, 0x42);
        let mut tracer = Tracer::with_max_clock_diff(100);

        // First access at clk=1
        tracer.clk = 1;
        mem.read_u8_traced(100, &mut tracer);

        // Access with gap > max_clock_diff (100)
        tracer.clk = 350;
        let access = mem.read_u8_traced(100, &mut tracer);

        // Should have 3 intermediate accesses (101, 201, 301) to bridge the gap
        assert_eq!(
            tracer.mem_clk_update.len(),
            3,
            "Expected 3 intermediates, got {}",
            tracer.mem_clk_update.len()
        );

        // Verify all intermediate clock diffs are within max_clock_diff
        for intermediate in &tracer.mem_clk_update {
            let diff = intermediate.clk.saturating_sub(intermediate.clk_prev);
            assert_eq!(diff, 100, "Clock diff {} should be 100", diff);
        }

        // Verify final access clock diff is within max_clock_diff
        let diff = access.clk.saturating_sub(access.clk_prev);
        assert_eq!(diff, 49, "Final clock diff {} should be 49", diff);
    }

    #[test]
    fn test_gap_filling_preserves_value() {
        let mut mem = Memory::new();
        mem.write_u8(100, 0xAB);
        let mut tracer = Tracer::with_max_clock_diff(50);

        tracer.clk = 0;
        mem.read_u8_traced(100, &mut tracer);

        tracer.clk = 200; // Large gap
        let access = mem.read_u8_traced(100, &mut tracer);

        // All intermediate accesses should preserve the value (read, not write)
        for intermediate in &tracer.mem_clk_update {
            assert_eq!(intermediate.prev, 0xAB);
            assert_eq!(intermediate.next, 0xAB);
        }
        // Final access should also preserve value
        assert_eq!(access.prev, 0xAB);
        assert_eq!(access.next, 0xAB);
    }

    #[test]
    fn test_exact_max_clock_diff_no_intermediate() {
        let mem = Memory::new();
        let mut tracer = Tracer::with_max_clock_diff(100);

        tracer.clk = 0;
        mem.read_u8_traced(100, &mut tracer);

        tracer.clk = 100; // Exactly at max_clock_diff
        let access = mem.read_u8_traced(100, &mut tracer);

        // Should be no intermediates needed
        assert!(tracer.mem_clk_update.is_empty());
        assert_eq!(access.clk_prev, 0);
        assert_eq!(access.clk, 100);
    }

    #[test]
    fn test_just_over_max_clock_diff_one_intermediate() {
        let mem = Memory::new();
        let mut tracer = Tracer::with_max_clock_diff(100);

        tracer.clk = 0;
        mem.read_u8_traced(100, &mut tracer);

        tracer.clk = 101; // Just over max_clock_diff
        mem.read_u8_traced(100, &mut tracer);

        // Should have 1 intermediate stored
        assert_eq!(tracer.mem_clk_update.len(), 1);
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_address_zero() {
        let mut mem = Memory::new();
        mem.write_u32(0, 0xDEADBEEF);
        assert_eq!(mem.read_u32(0), 0xDEADBEEF);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn test_address_u32_not_4_byte_aligned() {
        let mem = Memory::new();
        mem.read_u32(1);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn test_address_u16_not_2_byte_aligned() {
        let mem = Memory::new();
        mem.read_u16(1);
    }

    #[test]
    fn test_max_clock_diff_one() {
        let mem = Memory::new();
        let mut tracer = Tracer::with_max_clock_diff(1);

        tracer.clk = 0;
        mem.read_u8_traced(100, &mut tracer);

        tracer.clk = 5; // Gap of 5, need 4 intermediates
        let access = mem.read_u8_traced(100, &mut tracer);

        // With max_clock_diff=1, gap of 5 needs 4 intermediates
        assert_eq!(tracer.mem_clk_update.len(), 4);

        // Verify clk_prev values increase by 1 (max_clock_diff) each step
        for (i, intermediate) in tracer.mem_clk_update.iter().enumerate() {
            assert_eq!(intermediate.clk_prev, i as u32);
        }
        // Final access should have clk_prev = 4 (last intermediate's clk)
        assert_eq!(access.clk_prev, 4);
        assert_eq!(access.clk, 5);
    }

    #[test]
    fn test_max_clock_diff_max() {
        let mem = Memory::new();
        let mut tracer = Tracer::with_max_clock_diff(u32::MAX);

        tracer.clk = 0;
        mem.read_u8_traced(100, &mut tracer);

        tracer.clk = u32::MAX - 1;
        mem.read_u8_traced(100, &mut tracer);

        // No intermediate ever needed with max clock diff
        assert!(tracer.mem_clk_update.is_empty());
    }

    #[test]
    fn test_no_gap_sequential_clocks() {
        let mem = Memory::new();
        let mut tracer = Tracer::default();

        for clk in 0..10 {
            tracer.clk = clk;
            mem.read_u8_traced(100, &mut tracer);
        }
        // No intermediates needed for sequential clocks
        assert!(tracer.mem_clk_update.is_empty());
    }

    // =========================================================================
    // 4-Byte Aligned Tracing Tests
    // =========================================================================

    #[test]
    fn test_aligned_tracing_u8() {
        let mut mem = Memory::new();
        mem.write_u32(100, 0x12345678);
        let mut tracer = Tracer::default();

        tracer.clk = 10;
        let access = mem.read_u8_traced(101, &mut tracer);

        // Should trace the 4-byte aligned address
        assert_eq!(access.addr, 100); // 101 & !3 = 100
        assert_eq!(access.prev, 0x12345678);
        assert_eq!(access.next, 0x12345678);
        assert_eq!(tracer.mem_clk.get(&100), Some(&10));
    }

    #[test]
    fn test_aligned_tracing_u16() {
        let mut mem = Memory::new();
        mem.write_u32(100, 0x12345678);
        let mut tracer = Tracer::default();

        tracer.clk = 10;
        let access = mem.read_u16_traced(102, &mut tracer);

        // Should trace the 4-byte aligned address
        assert_eq!(access.addr, 100); // 102 & !3 = 100
        assert_eq!(access.prev, 0x12345678);
        assert_eq!(access.next, 0x12345678);
        assert_eq!(tracer.mem_clk.get(&100), Some(&10));
    }

    #[test]
    fn test_aligned_tracing_write_u8() {
        let mut mem = Memory::new();
        mem.write_u32(100, 0x12345678);
        let mut tracer = Tracer::default();

        tracer.clk = 10;
        let access = mem.write_u8_traced(101, 0xFF, &mut tracer);

        // Should trace the 4-byte aligned address with prev/next word values
        assert_eq!(access.addr, 100); // 101 & !3 = 100
        assert_eq!(access.prev, 0x12345678);
        assert_eq!(access.next, 0x1234FF78); // byte 1 changed to 0xFF
        assert_eq!(mem.read_u32(100), 0x1234FF78);
    }
}
