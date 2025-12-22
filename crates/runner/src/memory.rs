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
