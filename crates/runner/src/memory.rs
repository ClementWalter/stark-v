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
        let lo = self.read_u8(addr) as u16;
        let hi = self.read_u8(addr.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    /// Read a word (32-bit, little-endian).
    #[inline]
    pub fn read_u32(&self, addr: u32) -> u32 {
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
        self.write_u8(addr, val as u8);
        self.write_u8(addr.wrapping_add(1), (val >> 8) as u8);
    }

    /// Write a word (32-bit, little-endian).
    #[inline]
    pub fn write_u32(&mut self, addr: u32, val: u32) {
        self.write_u8(addr, val as u8);
        self.write_u8(addr.wrapping_add(1), (val >> 8) as u8);
        self.write_u8(addr.wrapping_add(2), (val >> 16) as u8);
        self.write_u8(addr.wrapping_add(3), (val >> 24) as u8);
    }

    // =========================================================================
    // Traced access methods
    // =========================================================================

    /// Read a byte with trace tracking.
    /// Intermediate catch-ups are stored in `tracer.mem_clk_update`.
    /// Returns the final access record.
    #[inline]
    pub fn read_u8_traced(&self, addr: u32, tracer: &mut Tracer) -> Access {
        let value = self.read_u8(addr) as u32;
        tracer.trace_mem_access(addr, value, value)
    }

    /// Trace multiple bytes with clock synchronization.
    /// 1. Find max clk_prev across all bytes
    /// 2. Catch up all bytes to that max (with intermediates stored in mem_clk_update)
    /// 3. Do final access at tracer.clk for all bytes (byte accesses stored in mem_clk_update)
    /// 4. Return a single Access representing the whole operation (base addr, full prev/next values)
    fn trace_multi_byte_access(
        &self,
        base_addr: u32,
        byte_values: &[u8],
        next_values: &[u8],
        prev_value: u32,
        next_value: u32,
        tracer: &mut Tracer,
    ) -> Access {
        let byte_count = byte_values.len();

        // Step 1: Find max clk_prev across all bytes
        let mut max_clk_prev = 0u32;
        for i in 0..byte_count {
            let byte_addr = base_addr.wrapping_add(i as u32);
            let clk_prev = tracer.mem_clk.get(&byte_addr).copied().unwrap_or(0);
            max_clk_prev = max_clk_prev.max(clk_prev);
        }

        // Step 2: Catch up all bytes to max_clk_prev
        // Save original tracer.clk and temporarily set to max_clk_prev for catch-up
        let original_clk = tracer.clk;
        tracer.clk = max_clk_prev;

        for (i, &byte_value) in byte_values.iter().enumerate() {
            let byte_addr = base_addr.wrapping_add(i as u32);
            let byte_value = byte_value as u32;
            let clk_prev = tracer.mem_clk.get(&byte_addr).copied().unwrap_or(0);

            // If this byte isn't at max_clk_prev, generate catch-up accesses
            if clk_prev < max_clk_prev {
                // trace_mem_access pushes intermediates to mem_clk_update and returns final
                let catch_up = tracer.trace_mem_access(byte_addr, byte_value, byte_value);
                // Store the catch-up access too
                tracer.mem_clk_update.push(catch_up);
            }
        }

        // Step 3: Restore original clk and do final accesses for each byte
        tracer.clk = original_clk;

        for (i, (&prev, &next)) in byte_values.iter().zip(next_values).enumerate() {
            let byte_addr = base_addr.wrapping_add(i as u32);
            let byte_access = tracer.trace_mem_access(byte_addr, prev as u32, next as u32);
            // Store individual byte accesses in mem_clk_update
            tracer.mem_clk_update.push(byte_access);
        }

        // Return a single Access representing the whole multi-byte operation
        Access {
            addr: base_addr,
            prev: prev_value,
            clk_prev: max_clk_prev,
            next: next_value,
            clk: original_clk,
        }
    }

    /// Read a half-word with trace tracking.
    /// All bytes are synchronized to the max clk_prev before final access.
    /// Byte-level accesses are stored in `tracer.mem_clk_update`.
    /// Returns a single Access for the whole operation.
    #[inline]
    pub fn read_u16_traced(&self, addr: u32, tracer: &mut Tracer) -> Access {
        let bytes: [u8; 2] = [self.read_u8(addr), self.read_u8(addr.wrapping_add(1))];
        let value = self.read_u16(addr) as u32;
        self.trace_multi_byte_access(addr, &bytes, &bytes, value, value, tracer)
    }

    /// Read a word with trace tracking.
    /// All bytes are synchronized to the max clk_prev before final access.
    /// Byte-level accesses are stored in `tracer.mem_clk_update`.
    /// Returns a single Access for the whole operation.
    #[inline]
    pub fn read_u32_traced(&self, addr: u32, tracer: &mut Tracer) -> Access {
        let bytes: [u8; 4] = [
            self.read_u8(addr),
            self.read_u8(addr.wrapping_add(1)),
            self.read_u8(addr.wrapping_add(2)),
            self.read_u8(addr.wrapping_add(3)),
        ];
        let value = self.read_u32(addr);
        self.trace_multi_byte_access(addr, &bytes, &bytes, value, value, tracer)
    }

    /// Write a byte with trace tracking.
    /// Intermediate catch-ups are stored in `tracer.mem_clk_update`.
    /// Returns the final access record.
    #[inline]
    pub fn write_u8_traced(&mut self, addr: u32, val: u8, tracer: &mut Tracer) -> Access {
        let prev = self.read_u8(addr) as u32;
        let access = tracer.trace_mem_access(addr, prev, val as u32);
        self.write_u8(addr, val);
        access
    }

    /// Write a half-word with trace tracking.
    /// All bytes are synchronized to the max clk_prev before final access.
    /// Byte-level accesses are stored in `tracer.mem_clk_update`.
    /// Returns a single Access for the whole operation.
    #[inline]
    pub fn write_u16_traced(&mut self, addr: u32, val: u16, tracer: &mut Tracer) -> Access {
        let prev_bytes: [u8; 2] = [self.read_u8(addr), self.read_u8(addr.wrapping_add(1))];
        let next_bytes = val.to_le_bytes();
        let prev_value = self.read_u16(addr) as u32;
        let access = self.trace_multi_byte_access(
            addr,
            &prev_bytes,
            &next_bytes,
            prev_value,
            val as u32,
            tracer,
        );
        self.write_u16(addr, val);
        access
    }

    /// Write a word with trace tracking.
    /// All bytes are synchronized to the max clk_prev before final access.
    /// Byte-level accesses are stored in `tracer.mem_clk_update`.
    /// Returns a single Access for the whole operation.
    #[inline]
    pub fn write_u32_traced(&mut self, addr: u32, val: u32, tracer: &mut Tracer) -> Access {
        let prev_bytes: [u8; 4] = [
            self.read_u8(addr),
            self.read_u8(addr.wrapping_add(1)),
            self.read_u8(addr.wrapping_add(2)),
            self.read_u8(addr.wrapping_add(3)),
        ];
        let next_bytes = val.to_le_bytes();
        let prev_value = self.read_u32(addr);
        let access =
            self.trace_multi_byte_access(addr, &prev_bytes, &next_bytes, prev_value, val, tracer);
        self.write_u32(addr, val);
        access
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
    fn test_read_u32_traced_syncs_clocks() {
        let mut mem = Memory::new();
        mem.write_u32(100, 0x44332211);
        let mut tracer = Tracer::default();

        // Set different clk_prev for each byte
        tracer.mem_clk.insert(100, 5);
        tracer.mem_clk.insert(101, 10);
        tracer.mem_clk.insert(102, 3);
        tracer.mem_clk.insert(103, 8);
        tracer.clk = 20;

        let access = mem.read_u32_traced(100, &mut tracer);

        // All bytes should end with clk=20 in tracer
        assert_eq!(tracer.mem_clk.get(&100), Some(&20));
        assert_eq!(tracer.mem_clk.get(&101), Some(&20));
        assert_eq!(tracer.mem_clk.get(&102), Some(&20));
        assert_eq!(tracer.mem_clk.get(&103), Some(&20));

        // Returned access should have base addr, full value, and clk_prev = max (10)
        assert_eq!(access.addr, 100);
        assert_eq!(access.prev, 0x44332211);
        assert_eq!(access.next, 0x44332211);
        assert_eq!(access.clk_prev, 10); // max of [5, 10, 3, 8]
        assert_eq!(access.clk, 20);

        // Should have catch-up accesses for 3 bytes not at max_clk_prev (10)
        // plus final accesses for all 4 bytes = 7 byte-level accesses in mem_clk_update
        assert_eq!(tracer.mem_clk_update.len(), 3 + 4);
    }

    #[test]
    fn test_write_u16_traced_syncs_and_writes() {
        let mut mem = Memory::new();
        let mut tracer = Tracer::default();

        // Different clk_prev for bytes
        tracer.mem_clk.insert(100, 2);
        tracer.mem_clk.insert(101, 5);
        tracer.clk = 10;

        let access = mem.write_u16_traced(100, 0xABCD, &mut tracer);

        // Verify memory written correctly
        assert_eq!(mem.read_u16(100), 0xABCD);

        // Verify clocks updated
        assert_eq!(tracer.mem_clk.get(&100), Some(&10));
        assert_eq!(tracer.mem_clk.get(&101), Some(&10));

        // Returned access should have the full u16 value
        assert_eq!(access.addr, 100);
        assert_eq!(access.prev, 0); // was uninitialized
        assert_eq!(access.next, 0xABCD);

        // Should have catch-up for byte 0 (from 2 to 5) + final accesses for 2 bytes
        assert_eq!(tracer.mem_clk_update.len(), 1 + 2);
    }

    // =========================================================================
    // Intermediate Gap-Filling Accesses (uses Tracer's max_clock_diff)
    // =========================================================================

    #[test]
    fn test_gap_filling_single_byte() {
        let mut mem = Memory::new();
        mem.write_u8(100, 0x42);
        let mut tracer = Tracer::with_max_clock_diff(100);

        // First access at clk=0
        tracer.clk = 0;
        mem.read_u8_traced(100, &mut tracer);

        // Access with gap > max_clock_diff (100)
        tracer.clk = 350;
        let access = mem.read_u8_traced(100, &mut tracer);

        // Should have 3 intermediate accesses to bridge the gap
        assert_eq!(
            tracer.mem_clk_update.len(),
            3,
            "Expected 3 intermediates, got {}",
            tracer.mem_clk_update.len()
        );

        // Verify all intermediate clock diffs are within max_clock_diff
        for intermediate in &tracer.mem_clk_update {
            let diff = intermediate.clk.saturating_sub(intermediate.clk_prev);
            assert!(
                diff <= 100,
                "Clock diff {} exceeds max_clock_diff 100",
                diff
            );
        }

        // Verify final access clock diff is within max_clock_diff
        let diff = access.clk.saturating_sub(access.clk_prev);
        assert!(
            diff <= 100,
            "Final clock diff {} exceeds max_clock_diff 100",
            diff
        );
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
    fn test_gap_filling_multi_byte() {
        let mut mem = Memory::new();
        mem.write_u32(100, 0x44332211);
        let mut tracer = Tracer::with_max_clock_diff(100);

        // Set one byte with old clock, others recent
        tracer.mem_clk.insert(100, 0);
        tracer.mem_clk.insert(101, 400);
        tracer.mem_clk.insert(102, 400);
        tracer.mem_clk.insert(103, 400);
        tracer.clk = 500;

        let access = mem.read_u32_traced(100, &mut tracer);

        // All byte-level clock diffs in mem_clk_update should be within max_clock_diff
        for byte_access in &tracer.mem_clk_update {
            let diff = byte_access.clk.saturating_sub(byte_access.clk_prev);
            assert!(
                diff <= 100,
                "Clock diff {} at addr {} exceeds max_clock_diff 100",
                diff,
                byte_access.addr
            );
        }

        // The returned Access represents the whole operation
        assert_eq!(access.addr, 100);
        assert_eq!(access.prev, 0x44332211);
        assert_eq!(access.next, 0x44332211);
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

        let mut tracer = Tracer::default();
        tracer.clk = 1;
        let access = mem.read_u32_traced(0, &mut tracer);
        // Should have byte-level accesses in mem_clk_update
        assert!(!tracer.mem_clk_update.is_empty());
        assert_eq!(access.prev, 0xDEADBEEF);
    }

    #[test]
    fn test_address_wrap_around() {
        let mut mem = Memory::new();
        // Write at max address - should wrap when reading u32
        mem.write_u8(0xFFFFFFFF, 0x11);
        mem.write_u8(0x00000000, 0x22); // Wraps to 0
        mem.write_u8(0x00000001, 0x33);
        mem.write_u8(0x00000002, 0x44);

        // Read u32 starting at 0xFFFFFFFF wraps around
        let val = mem.read_u32(0xFFFFFFFF);
        assert_eq!(val, 0x44332211); // First byte from 0xFFFFFFFF
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

        // Verify each intermediate step is exactly 1
        for intermediate in &tracer.mem_clk_update {
            let diff = intermediate.clk - intermediate.clk_prev;
            assert_eq!(diff, 1);
        }
        // Verify final step is exactly 1
        let diff = access.clk - access.clk_prev;
        assert_eq!(diff, 1);
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
}
