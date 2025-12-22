use std::collections::BTreeMap;

use crate::trace::{Access, Tracer};

/// Default maximum clock difference allowed between accesses.
/// Need to be consistent with max range-check in the prover
pub const DEFAULT_MAX_CLOCK_DIFF: u32 = 1 << 20; // ~1M cycles

/// Sparse byte-addressable memory using BTreeMap.
pub struct Memory {
    data: BTreeMap<u32, u8>,
    /// Maximum allowed clock difference between consecutive accesses to the same address.
    /// If exceeded, intermediate "catch-up" accesses are generated.
    max_clock_diff: u32,
}

impl Memory {
    /// Create empty memory with default max clock diff.
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            max_clock_diff: DEFAULT_MAX_CLOCK_DIFF,
        }
    }

    /// Create empty memory with custom max clock diff.
    pub fn with_max_clock_diff(max_clock_diff: u32) -> Self {
        Self {
            data: BTreeMap::new(),
            max_clock_diff,
        }
    }

    /// Set the maximum clock difference.
    pub fn set_max_clock_diff(&mut self, max_diff: u32) {
        self.max_clock_diff = max_diff;
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

    /// Generate intermediate accesses for a single byte to bring its clock up.
    /// Returns the list of intermediate accesses (not including the final one).
    fn generate_intermediate_accesses(
        &self,
        addr: u32,
        value: u32,
        clk_prev: u32,
        target_clk: u32,
        tracer: &mut Tracer,
    ) -> Vec<Access> {
        let mut accesses = Vec::new();
        let mut current_clk_prev = clk_prev;

        // Generate intermediate accesses until we're within max_clock_diff of target
        while target_clk.saturating_sub(current_clk_prev) > self.max_clock_diff {
            let intermediate_clk = current_clk_prev.saturating_add(self.max_clock_diff);
            accesses.push(Access {
                addr,
                prev: value,
                clk_prev: current_clk_prev,
                next: value, // No change, just clock catch-up
                clk: intermediate_clk,
            });
            // Update the byte's clock in tracer
            tracer.mem_clk.insert(addr, intermediate_clk);
            current_clk_prev = intermediate_clk;
        }

        accesses
    }

    /// Trace a single byte access, generating intermediate accesses if needed.
    fn trace_byte_access(
        &self,
        addr: u32,
        prev_value: u32,
        next_value: u32,
        tracer: &mut Tracer,
    ) -> Vec<Access> {
        let clk_prev = tracer.mem_clk.get(&addr).copied().unwrap_or(0);

        // Generate intermediate accesses to catch up the clock
        let mut accesses =
            self.generate_intermediate_accesses(addr, prev_value, clk_prev, tracer.clk, tracer);

        // The final clk_prev after intermediates
        let final_clk_prev = if accesses.is_empty() {
            clk_prev
        } else {
            accesses.last().map(|a| a.clk).unwrap_or(clk_prev)
        };

        // Add the actual access
        accesses.push(Access {
            addr,
            prev: prev_value,
            clk_prev: final_clk_prev,
            next: next_value,
            clk: tracer.clk,
        });

        // Update the byte's clock
        tracer.mem_clk.insert(addr, tracer.clk);

        accesses
    }

    /// Read a byte with trace tracking.
    /// Returns a list of accesses (intermediate catch-ups + final read).
    #[inline]
    pub fn read_u8_traced(&self, addr: u32, tracer: &mut Tracer) -> Vec<Access> {
        let value = self.read_u8(addr) as u32;
        self.trace_byte_access(addr, value, value, tracer)
    }

    /// Trace multiple bytes with clock synchronization.
    /// 1. Find max clk_prev across all bytes
    /// 2. Catch up all bytes to that max (with intermediates if needed)
    /// 3. Do final access at tracer.clk for all bytes
    fn trace_multi_byte_access(
        &self,
        base_addr: u32,
        byte_count: usize,
        byte_values: &[u8],
        next_values: &[u8],
        tracer: &mut Tracer,
    ) -> Vec<Access> {
        let mut accesses = Vec::new();

        // Step 1: Find max clk_prev across all bytes
        let mut max_clk_prev = 0u32;
        for i in 0..byte_count {
            let byte_addr = base_addr.wrapping_add(i as u32);
            let clk_prev = tracer.mem_clk.get(&byte_addr).copied().unwrap_or(0);
            max_clk_prev = max_clk_prev.max(clk_prev);
        }

        // Step 2: Catch up all bytes to max_clk_prev (with intermediates if needed)
        for (i, &byte_value) in byte_values.iter().enumerate() {
            let byte_addr = base_addr.wrapping_add(i as u32);
            let byte_value = byte_value as u32;
            let clk_prev = tracer.mem_clk.get(&byte_addr).copied().unwrap_or(0);

            // Generate intermediates from clk_prev to max_clk_prev
            accesses.extend(self.generate_intermediate_accesses(
                byte_addr,
                byte_value,
                clk_prev,
                max_clk_prev,
                tracer,
            ));

            // If this byte wasn't already at max_clk_prev, add a catch-up access
            if clk_prev < max_clk_prev {
                // Get the current clock after intermediates
                let current_clk = tracer.mem_clk.get(&byte_addr).copied().unwrap_or(clk_prev);
                if current_clk < max_clk_prev {
                    accesses.push(Access {
                        addr: byte_addr,
                        prev: byte_value,
                        clk_prev: current_clk,
                        next: byte_value,
                        clk: max_clk_prev,
                    });
                    tracer.mem_clk.insert(byte_addr, max_clk_prev);
                }
            }
        }

        // Step 3: Generate intermediates from max_clk_prev to tracer.clk and final access
        for (i, (&prev, &next)) in byte_values.iter().zip(next_values).enumerate() {
            let byte_addr = base_addr.wrapping_add(i as u32);
            let prev_value = prev as u32;
            let next_value = next as u32;

            // Generate intermediates from max_clk_prev to tracer.clk
            accesses.extend(self.generate_intermediate_accesses(
                byte_addr,
                prev_value,
                max_clk_prev,
                tracer.clk,
                tracer,
            ));

            // Get the final clk_prev after all intermediates
            let final_clk_prev = tracer
                .mem_clk
                .get(&byte_addr)
                .copied()
                .unwrap_or(max_clk_prev);

            // Add the actual access
            accesses.push(Access {
                addr: byte_addr,
                prev: prev_value,
                clk_prev: final_clk_prev,
                next: next_value,
                clk: tracer.clk,
            });

            tracer.mem_clk.insert(byte_addr, tracer.clk);
        }

        accesses
    }

    /// Read a half-word with trace tracking.
    /// All bytes are synchronized to the max clk_prev before final access.
    #[inline]
    pub fn read_u16_traced(&self, addr: u32, tracer: &mut Tracer) -> Vec<Access> {
        let bytes: [u8; 2] = [self.read_u8(addr), self.read_u8(addr.wrapping_add(1))];
        self.trace_multi_byte_access(addr, 2, &bytes, &bytes, tracer)
    }

    /// Read a word with trace tracking.
    /// All bytes are synchronized to the max clk_prev before final access.
    #[inline]
    pub fn read_u32_traced(&self, addr: u32, tracer: &mut Tracer) -> Vec<Access> {
        let bytes: [u8; 4] = [
            self.read_u8(addr),
            self.read_u8(addr.wrapping_add(1)),
            self.read_u8(addr.wrapping_add(2)),
            self.read_u8(addr.wrapping_add(3)),
        ];
        self.trace_multi_byte_access(addr, 4, &bytes, &bytes, tracer)
    }

    /// Write a byte with trace tracking.
    /// Returns a list of accesses (intermediate catch-ups + final write).
    #[inline]
    pub fn write_u8_traced(&mut self, addr: u32, val: u8, tracer: &mut Tracer) -> Vec<Access> {
        let prev = self.read_u8(addr) as u32;
        let accesses = self.trace_byte_access(addr, prev, val as u32, tracer);
        self.write_u8(addr, val);
        accesses
    }

    /// Write a half-word with trace tracking.
    /// All bytes are synchronized to the max clk_prev before final access.
    #[inline]
    pub fn write_u16_traced(&mut self, addr: u32, val: u16, tracer: &mut Tracer) -> Vec<Access> {
        let prev_bytes: [u8; 2] = [self.read_u8(addr), self.read_u8(addr.wrapping_add(1))];
        let next_bytes = val.to_le_bytes();
        let accesses = self.trace_multi_byte_access(addr, 2, &prev_bytes, &next_bytes, tracer);
        self.write_u16(addr, val);
        accesses
    }

    /// Write a word with trace tracking.
    /// All bytes are synchronized to the max clk_prev before final access.
    #[inline]
    pub fn write_u32_traced(&mut self, addr: u32, val: u32, tracer: &mut Tracer) -> Vec<Access> {
        let prev_bytes: [u8; 4] = [
            self.read_u8(addr),
            self.read_u8(addr.wrapping_add(1)),
            self.read_u8(addr.wrapping_add(2)),
            self.read_u8(addr.wrapping_add(3)),
        ];
        let next_bytes = val.to_le_bytes();
        let accesses = self.trace_multi_byte_access(addr, 4, &prev_bytes, &next_bytes, tracer);
        self.write_u32(addr, val);
        accesses
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
            max_clock_diff: DEFAULT_MAX_CLOCK_DIFF,
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
        assert_eq!(mem.max_clock_diff, DEFAULT_MAX_CLOCK_DIFF);
    }

    #[test]
    fn test_default_same_as_new() {
        let mem = Memory::default();
        assert_eq!(mem.read_u8(0), 0);
        assert_eq!(mem.max_clock_diff, DEFAULT_MAX_CLOCK_DIFF);
    }

    #[test]
    fn test_with_max_clock_diff() {
        let mem = Memory::with_max_clock_diff(100);
        assert_eq!(mem.max_clock_diff, 100);
    }

    #[test]
    fn test_set_max_clock_diff() {
        let mut mem = Memory::new();
        mem.set_max_clock_diff(500);
        assert_eq!(mem.max_clock_diff, 500);
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

        let accesses = mem.read_u8_traced(100, &mut tracer);

        assert_eq!(accesses.len(), 1);
        assert_eq!(accesses[0].addr, 100);
        assert_eq!(accesses[0].prev, 0);
        assert_eq!(accesses[0].next, 0);
        assert_eq!(accesses[0].clk_prev, 0);
        assert_eq!(accesses[0].clk, tracer.clk);
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

        let accesses = mem.write_u8_traced(100, 0xFF, &mut tracer);

        assert_eq!(accesses.len(), 1);
        assert_eq!(accesses[0].addr, 100);
        assert_eq!(accesses[0].prev, 0x42);
        assert_eq!(accesses[0].next, 0xFF);
        assert_eq!(accesses[0].clk_prev, 0);
        assert_eq!(accesses[0].clk, tracer.clk);

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
        let accesses = mem.write_u8_traced(100, 0x22, &mut tracer);

        assert_eq!(accesses.len(), 1);
        assert_eq!(accesses[0].clk_prev, 1);
        assert_eq!(accesses[0].clk, tracer.clk);
        assert_eq!(accesses[0].prev, 0x11);
        assert_eq!(accesses[0].next, 0x22);
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

        let accesses = mem.read_u32_traced(100, &mut tracer);

        // All bytes should end with clk=20 in tracer
        assert_eq!(tracer.mem_clk.get(&100), Some(&tracer.clk));
        assert_eq!(tracer.mem_clk.get(&101), Some(&tracer.clk));
        assert_eq!(tracer.mem_clk.get(&102), Some(&tracer.clk));
        assert_eq!(tracer.mem_clk.get(&103), Some(&tracer.clk));

        // Should have catch-up accesses for 3 bytes not at max_clk_prev (10)
        // plus final accesses for all 4 bytes
        assert!(accesses.len() == 3 + 4);
    }

    #[test]
    fn test_write_u16_traced_syncs_and_writes() {
        let mut mem = Memory::new();
        let mut tracer = Tracer::default();

        // Different clk_prev for bytes
        tracer.mem_clk.insert(100, 2);
        tracer.mem_clk.insert(101, 5);
        tracer.clk = 10;

        let accesses = mem.write_u16_traced(100, 0xABCD, &mut tracer);

        // Verify memory written correctly
        assert_eq!(mem.read_u16(100), 0xABCD);

        // Verify clocks updated
        assert_eq!(tracer.mem_clk.get(&100), Some(&10));
        assert_eq!(tracer.mem_clk.get(&101), Some(&10));

        // Should have catch-up for byte 0 (from 2 to 5) + final accesses
        assert!(accesses.len() >= 2);
    }

    // =========================================================================
    // Intermediate Gap-Filling Accesses
    // =========================================================================

    #[test]
    fn test_gap_filling_single_byte() {
        let mut mem = Memory::with_max_clock_diff(100);
        mem.write_u8(100, 0x42);
        let mut tracer = Tracer::default();

        // First access at clk=0
        tracer.clk = 0;
        mem.read_u8_traced(100, &mut tracer);

        // Access with gap > max_clock_diff (100)
        tracer.clk = 350;
        let accesses = mem.read_u8_traced(100, &mut tracer);

        // Should have intermediate accesses to bridge the gap
        // Gap of 350 with max_diff 100 needs at least 3 intermediates + 1 final
        assert!(
            accesses.len() == 4,
            "Expected at 4 accesses, got {}",
            accesses.len()
        );

        // Verify all clock diffs are within max_clock_diff
        for access in &accesses {
            let diff = access.clk.saturating_sub(access.clk_prev);
            assert!(
                diff <= 100,
                "Clock diff {} exceeds max_clock_diff 100",
                diff
            );
        }
    }

    #[test]
    fn test_gap_filling_preserves_value() {
        let mut mem = Memory::with_max_clock_diff(50);
        mem.write_u8(100, 0xAB);
        let mut tracer = Tracer::default();

        tracer.clk = 0;
        mem.read_u8_traced(100, &mut tracer);

        tracer.clk = 200; // Large gap
        let accesses = mem.read_u8_traced(100, &mut tracer);

        // All intermediate accesses should preserve the value (read, not write)
        for access in &accesses {
            assert_eq!(access.prev, 0xAB);
            assert_eq!(access.next, 0xAB);
        }
    }

    #[test]
    fn test_gap_filling_multi_byte() {
        let mut mem = Memory::with_max_clock_diff(100);
        mem.write_u32(100, 0x44332211);
        let mut tracer = Tracer::default();

        // Set one byte with old clock, others recent
        tracer.mem_clk.insert(100, 0);
        tracer.mem_clk.insert(101, 400);
        tracer.mem_clk.insert(102, 400);
        tracer.mem_clk.insert(103, 400);
        tracer.clk = 500;

        let accesses = mem.read_u32_traced(100, &mut tracer);

        // All clock diffs should be within max_clock_diff
        for access in &accesses {
            let diff = access.clk.saturating_sub(access.clk_prev);
            assert!(
                diff <= 100,
                "Clock diff {} at addr {} exceeds max_clock_diff 100",
                diff,
                access.addr
            );
        }
    }

    #[test]
    fn test_exact_max_clock_diff_no_intermediate() {
        let mem = Memory::with_max_clock_diff(100);
        let mut tracer = Tracer::default();

        tracer.clk = 0;
        mem.read_u8_traced(100, &mut tracer);

        tracer.clk = 100; // Exactly at max_clock_diff
        let accesses = mem.read_u8_traced(100, &mut tracer);

        // Should be exactly 1 access (no intermediate needed)
        assert_eq!(accesses.len(), 1);
        assert_eq!(accesses[0].clk_prev, 0);
        assert_eq!(accesses[0].clk, 100);
    }

    #[test]
    fn test_just_over_max_clock_diff_one_intermediate() {
        let mem = Memory::with_max_clock_diff(100);
        let mut tracer = Tracer::default();

        tracer.clk = 0;
        mem.read_u8_traced(100, &mut tracer);

        tracer.clk = 101; // Just over max_clock_diff
        let accesses = mem.read_u8_traced(100, &mut tracer);

        // Should have 1 intermediate + 1 final = 2 accesses
        assert_eq!(accesses.len(), 2);
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
        let accesses = mem.read_u32_traced(0, &mut tracer);
        assert!(!accesses.is_empty());
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
        let mem = Memory::with_max_clock_diff(1);
        let mut tracer = Tracer::default();

        tracer.clk = 0;
        mem.read_u8_traced(100, &mut tracer);

        tracer.clk = 5; // Gap of 5, need 4 intermediates
        let accesses = mem.read_u8_traced(100, &mut tracer);

        // With max_clock_diff=1, gap of 5 needs 5 accesses total
        assert_eq!(accesses.len(), 5);

        // Verify each step is exactly 1
        for access in &accesses {
            let diff = access.clk - access.clk_prev;
            assert_eq!(diff, 1);
        }
    }

    #[test]
    fn test_max_clock_diff_max() {
        let mem = Memory::with_max_clock_diff(u32::MAX);
        let mut tracer = Tracer::default();

        tracer.clk = 0;
        mem.read_u8_traced(100, &mut tracer);

        tracer.clk = u32::MAX - 1;
        let accesses = mem.read_u8_traced(100, &mut tracer);

        // Should be exactly 1 access (no intermediate ever needed)
        assert_eq!(accesses.len(), 1);
    }

    #[test]
    fn test_no_gap_sequential_clocks() {
        let mem = Memory::new();
        let mut tracer = Tracer::default();

        for clk in 0..10 {
            tracer.clk = clk;
            let accesses = mem.read_u8_traced(100, &mut tracer);
            // Each access should be exactly 1 (no intermediates)
            assert_eq!(accesses.len(), 1);
        }
    }
}
