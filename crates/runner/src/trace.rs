//! Trace capture for zkVM execution.
//!
//! Each opcode defines its own columnar trace table.
//! Registers and memory use a unified Access structure that gets flattened into columns.

use simd::AlignedVec;

/// Default maximum clock difference allowed between accesses.
/// Must be consistent with max range-check in the prover.
pub const DEFAULT_MAX_CLOCK_DIFF: u32 = 1 << 20; // ~1M cycles

// =============================================================================
// Generate all trace tables, Tracer struct, and trace_op! macro
// =============================================================================

runner_macros::define_trace_tables! {
    // R-type ALU
    add: { clk, pc, rd, rs1, rs2 },
    sub: { clk, pc, rd, rs1, rs2 },
    sll: { clk, pc, rd, rs1, rs2 },
    slt: { clk, pc, rd, rs1, rs2 },
    sltu: { clk, pc, rd, rs1, rs2 },
    xor: { clk, pc, rd, rs1, rs2 },
    srl: { clk, pc, rd, rs1, rs2 },
    sra: { clk, pc, rd, rs1, rs2 },
    or: { clk, pc, rd, rs1, rs2 },
    and: { clk, pc, rd, rs1, rs2 },

    // I-type ALU
    addi: { clk, pc, rd, rs1 },
    slti: { clk, pc, rd, rs1 },
    sltiu: { clk, pc, rd, rs1 },
    xori: { clk, pc, rd, rs1 },
    ori: { clk, pc, rd, rs1 },
    andi: { clk, pc, rd, rs1 },
    slli: { clk, pc, rd, rs1 },
    srli: { clk, pc, rd, rs1 },
    srai: { clk, pc, rd, rs1 },

    // Load
    lb: { clk, pc, rd, rs1, mem },
    lh: { clk, pc, rd, rs1, mem },
    lw: { clk, pc, rd, rs1, mem },
    lbu: { clk, pc, rd, rs1, mem },
    lhu: { clk, pc, rd, rs1, mem },

    // Store
    sb: { clk, pc, rs1, rs2, mem },
    sh: { clk, pc, rs1, rs2, mem },
    sw: { clk, pc, rs1, rs2, mem },

    // Branch
    beq: { clk, pc, rs1, rs2 },
    bne: { clk, pc, rs1, rs2 },
    blt: { clk, pc, rs1, rs2 },
    bge: { clk, pc, rs1, rs2 },
    bltu: { clk, pc, rs1, rs2 },
    bgeu: { clk, pc, rs1, rs2 },

    // Jump
    jal: { clk, pc, rd },
    jalr: { clk, pc, rd, rs1 },

    // Upper immediate
    lui: { clk, pc, rd },
    auipc: { clk, pc, rd },

    // M-extension
    mul: { clk, pc, rd, rs1, rs2 },
    mulh: { clk, pc, rd, rs1, rs2 },
    mulhsu: { clk, pc, rd, rs1, rs2 },
    mulhu: { clk, pc, rd, rs1, rs2 },
    div: { clk, pc, rd, rs1, rs2 },
    divu: { clk, pc, rd, rs1, rs2 },
    rem: { clk, pc, rd, rs1, rs2 },
    remu: { clk, pc, rd, rs1, rs2 },
}

// =============================================================================
// Tracer memory access methods and utils
// =============================================================================

/// Unified access record for both registers and memory.
///
/// - For registers: `addr` is the register index (0-31)
/// - For memory: `addr` is the byte address
#[derive(Debug, Clone, Copy, Default)]
pub struct Access {
    pub addr: u32,
    pub prev: u32,
    pub clk_prev: u32,
    pub next: u32,
    pub clk: u32,
}

// =============================================================================
// Columnar AccessTable (for gap-filling)
// =============================================================================

/// Columnar storage for Access records (used for gap-filling).
#[derive(Debug, Clone, Default)]
pub struct AccessTable {
    pub addr: AlignedVec<u32>,
    pub prev: AlignedVec<u32>,
    pub clk_prev: AlignedVec<u32>,
    pub next: AlignedVec<u32>,
    pub clk: AlignedVec<u32>,
}

impl AccessTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            addr: AlignedVec::with_capacity(cap),
            prev: AlignedVec::with_capacity(cap),
            clk_prev: AlignedVec::with_capacity(cap),
            next: AlignedVec::with_capacity(cap),
            clk: AlignedVec::with_capacity(cap),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.addr.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.addr.is_empty()
    }

    #[inline]
    pub fn push(&mut self, access: Access) {
        self.addr.push(access.addr);
        self.prev.push(access.prev);
        self.clk_prev.push(access.clk_prev);
        self.next.push(access.next);
        self.clk.push(access.clk);
    }
}

impl Tracer {
    /// Generate and store intermediate accesses for gap-filling.
    fn fill_gap(
        &mut self,
        table: GapTable,
        addr: u32,
        value: u32,
        clk_prev: u32,
        target_clk: u32,
    ) -> u32 {
        let mut current_clk = clk_prev;

        while target_clk.saturating_sub(current_clk) > self.max_clock_diff {
            let next_clk = current_clk.saturating_add(self.max_clock_diff);
            let access = Access {
                addr,
                prev: value,
                clk_prev: current_clk,
                next: value,
                clk: next_clk,
            };
            match table {
                GapTable::Reg => self.reg_clk_update.push(access),
                GapTable::Mem => self.mem_clk_update.push(access),
            }
            current_clk = next_clk;
        }

        current_clk
    }

    /// Trace a register access with gap-filling.
    /// Intermediate accesses are pushed to `reg_clk_update`.
    /// Returns only the final access.
    pub fn trace_reg_access(&mut self, idx: u8, prev: u32, next: u32) -> Access {
        let clk_prev = self.reg_clk[idx as usize];
        let addr = idx as u32;

        // Generate intermediate catch-up accesses and get final clk_prev
        let final_clk_prev = self.fill_gap(GapTable::Reg, addr, prev, clk_prev, self.clk);

        // Update the register's clock after gap-filling
        if final_clk_prev != clk_prev {
            self.reg_clk[idx as usize] = final_clk_prev;
        }

        // Create the final access
        let final_access = Access {
            addr,
            prev,
            clk_prev: final_clk_prev,
            next,
            clk: self.clk,
        };

        // Update the register's clock
        self.reg_clk[idx as usize] = self.clk;

        final_access
    }

    /// Trace a memory byte access with gap-filling.
    /// Intermediate accesses are pushed to `mem_clk_update`.
    /// Returns only the final access.
    pub fn trace_mem_access(&mut self, addr: u32, prev: u32, next: u32) -> Access {
        let clk_prev = self.mem_clk.get(&addr).copied().unwrap_or(0);

        // Generate intermediate catch-up accesses and get final clk_prev
        let final_clk_prev = self.fill_gap(GapTable::Mem, addr, prev, clk_prev, self.clk);

        // Update mem_clk after gap-filling
        if final_clk_prev != clk_prev {
            self.mem_clk.insert(addr, final_clk_prev);
        }

        // Create the final access
        let final_access = Access {
            addr,
            prev,
            clk_prev: final_clk_prev,
            next,
            clk: self.clk,
        };

        // Update the memory byte's clock
        self.mem_clk.insert(addr, self.clk);

        final_access
    }
}

/// Helper enum for gap-filling table selection.
enum GapTable {
    Reg,
    Mem,
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    impl AccessTable {
        /// Returns an iterator over Access values (for backward compatibility).
        pub fn iter(&self) -> AccessTableIter<'_> {
            AccessTableIter {
                table: self,
                idx: 0,
            }
        }
    }

    /// Iterator over AccessTable that yields Access values.
    pub struct AccessTableIter<'a> {
        table: &'a AccessTable,
        idx: usize,
    }

    impl Iterator for AccessTableIter<'_> {
        type Item = Access;

        fn next(&mut self) -> Option<Self::Item> {
            if self.idx >= self.table.len() {
                None
            } else {
                let access = Access {
                    addr: self.table.addr[self.idx],
                    prev: self.table.prev[self.idx],
                    clk_prev: self.table.clk_prev[self.idx],
                    next: self.table.next[self.idx],
                    clk: self.table.clk[self.idx],
                };
                self.idx += 1;
                Some(access)
            }
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let remaining = self.table.len() - self.idx;
            (remaining, Some(remaining))
        }
    }

    impl ExactSizeIterator for AccessTableIter<'_> {}

    impl<'a> IntoIterator for &'a AccessTable {
        type Item = Access;
        type IntoIter = AccessTableIter<'a>;

        fn into_iter(self) -> Self::IntoIter {
            self.iter()
        }
    }

    // =========================================================================
    // Tracer Construction
    // =========================================================================

    #[test]
    fn test_default_tracer() {
        let tracer = Tracer::default();
        assert_eq!(tracer.clk, 0);
        assert_eq!(tracer.max_clock_diff, DEFAULT_MAX_CLOCK_DIFF);
        assert_eq!(tracer.reg_clk, [0; 32]);
        assert!(tracer.mem_clk.is_empty());
    }

    #[test]
    fn test_with_max_clock_diff() {
        let tracer = Tracer::with_max_clock_diff(100);
        assert_eq!(tracer.max_clock_diff, 100);
        assert_eq!(tracer.clk, 0);
    }

    // =========================================================================
    // Memory Access Tracing
    // =========================================================================

    #[test]
    fn test_trace_mem_access_first_access() {
        let mut tracer = Tracer::default();
        tracer.clk = 10;

        let access = tracer.trace_mem_access(100, 0x42, 0x42);

        assert_eq!(access.addr, 100);
        assert_eq!(access.prev, 0x42);
        assert_eq!(access.next, 0x42);
        assert_eq!(access.clk_prev, 0);
        assert_eq!(access.clk, 10);
        assert!(tracer.mem_clk_update.is_empty());
    }

    #[test]
    fn test_trace_mem_access_consecutive() {
        let mut tracer = Tracer::default();

        tracer.clk = 1;
        tracer.trace_mem_access(100, 0x11, 0x11);

        tracer.clk = 2;
        let access = tracer.trace_mem_access(100, 0x11, 0x22);

        assert_eq!(access.clk_prev, 1);
        assert_eq!(access.clk, 2);
        assert_eq!(access.prev, 0x11);
        assert_eq!(access.next, 0x22);
        assert!(tracer.mem_clk_update.is_empty());
    }

    #[test]
    fn test_trace_mem_access_gap_filling() {
        let mut tracer = Tracer::with_max_clock_diff(100);

        tracer.clk = 0;
        tracer.trace_mem_access(100, 0x42, 0x42);

        tracer.clk = 350;
        let access = tracer.trace_mem_access(100, 0x42, 0x42);

        // Gap of 350 with max_diff 100 needs 3 intermediates
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
    fn test_trace_mem_access_exact_max_diff() {
        let mut tracer = Tracer::with_max_clock_diff(100);

        tracer.clk = 0;
        tracer.trace_mem_access(100, 0, 0);

        tracer.clk = 100;
        let access = tracer.trace_mem_access(100, 0, 0);

        // Exactly at max_clock_diff - no intermediate needed
        assert!(tracer.mem_clk_update.is_empty());
        assert_eq!(access.clk_prev, 0);
        assert_eq!(access.clk, 100);
    }

    #[test]
    fn test_trace_mem_access_preserves_value() {
        let mut tracer = Tracer::with_max_clock_diff(50);

        tracer.clk = 0;
        tracer.trace_mem_access(100, 0xAB, 0xAB);

        tracer.clk = 200;
        let access = tracer.trace_mem_access(100, 0xAB, 0xAB);

        // All intermediate accesses should preserve the value
        for intermediate in &tracer.mem_clk_update {
            assert_eq!(intermediate.prev, 0xAB);
            assert_eq!(intermediate.next, 0xAB);
        }
        // Final access should also preserve value
        assert_eq!(access.prev, 0xAB);
        assert_eq!(access.next, 0xAB);
    }

    #[test]
    fn test_trace_mem_access_updates_mem_clk() {
        let mut tracer = Tracer::default();
        tracer.clk = 10;

        tracer.trace_mem_access(100, 0, 0);

        assert_eq!(tracer.mem_clk.get(&100), Some(&10));
    }

    // =========================================================================
    // Register Access Tracing
    // =========================================================================

    #[test]
    fn test_trace_reg_access_first_access() {
        let mut tracer = Tracer::default();
        tracer.clk = 10;

        let access = tracer.trace_reg_access(5, 0x42, 0x42);

        assert_eq!(access.addr, 5);
        assert_eq!(access.prev, 0x42);
        assert_eq!(access.next, 0x42);
        assert_eq!(access.clk_prev, 0);
        assert_eq!(access.clk, 10);
        assert!(tracer.reg_clk_update.is_empty());
    }

    #[test]
    fn test_trace_reg_access_consecutive() {
        let mut tracer = Tracer::default();

        tracer.clk = 1;
        tracer.trace_reg_access(5, 0x11, 0x11);

        tracer.clk = 2;
        let access = tracer.trace_reg_access(5, 0x11, 0x22);

        assert_eq!(access.clk_prev, 1);
        assert_eq!(access.clk, 2);
        assert_eq!(access.prev, 0x11);
        assert_eq!(access.next, 0x22);
        assert!(tracer.reg_clk_update.is_empty());
    }

    #[test]
    fn test_trace_reg_access_gap_filling() {
        let mut tracer = Tracer::with_max_clock_diff(100);

        tracer.clk = 0;
        tracer.trace_reg_access(5, 0x42, 0x42);

        tracer.clk = 350;
        let access = tracer.trace_reg_access(5, 0x42, 0x42);

        // Gap of 350 with max_diff 100 needs 3 intermediates
        assert_eq!(
            tracer.reg_clk_update.len(),
            3,
            "Expected 3 intermediates, got {}",
            tracer.reg_clk_update.len()
        );

        // Verify all intermediate clock diffs are within max_clock_diff
        for intermediate in &tracer.reg_clk_update {
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
    fn test_trace_reg_access_x0() {
        let mut tracer = Tracer::default();
        tracer.clk = 10;

        // x0 can still be traced - the caller handles x0 semantics
        let access = tracer.trace_reg_access(0, 0, 0);

        assert_eq!(access.addr, 0);
        assert_eq!(access.prev, 0);
        assert_eq!(access.next, 0);
        assert!(tracer.reg_clk_update.is_empty());
    }

    #[test]
    fn test_trace_reg_access_updates_reg_clk() {
        let mut tracer = Tracer::default();
        tracer.clk = 10;

        tracer.trace_reg_access(5, 0, 0);

        assert_eq!(tracer.reg_clk[5], 10);
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_max_clock_diff_one() {
        let mut tracer = Tracer::with_max_clock_diff(1);

        tracer.clk = 0;
        tracer.trace_mem_access(100, 0, 0);

        tracer.clk = 5;
        let access = tracer.trace_mem_access(100, 0, 0);

        // With max_clock_diff=1, gap of 5 needs 4 intermediates + 1 final
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
        let mut tracer = Tracer::with_max_clock_diff(u32::MAX);

        tracer.clk = 0;
        tracer.trace_mem_access(100, 0, 0);

        tracer.clk = u32::MAX - 1;
        tracer.trace_mem_access(100, 0, 0);

        // No intermediate ever needed
        assert!(tracer.mem_clk_update.is_empty());
    }

    // =========================================================================
    // Columnar Table Tests
    // =========================================================================

    #[test]
    fn test_add_table_push() {
        let mut table = AddTable::new();

        let rd = Access {
            addr: 1,
            prev: 0,
            clk_prev: 0,
            next: 10,
            clk: 1,
        };
        let rs1 = Access {
            addr: 2,
            prev: 5,
            clk_prev: 0,
            next: 5,
            clk: 1,
        };
        let rs2 = Access {
            addr: 3,
            prev: 5,
            clk_prev: 0,
            next: 5,
            clk: 1,
        };

        table.push(1, 0x1000, rd, rs1, rs2);

        assert_eq!(table.len(), 1);
        assert_eq!(table.clk[0], 1);
        assert_eq!(table.pc[0], 0x1000);
        assert_eq!(table.rd_addr[0], 1);
        assert_eq!(table.rd_next[0], 10);
        assert_eq!(table.rs1_addr[0], 2);
        assert_eq!(table.rs2_addr[0], 3);
    }

    #[test]
    fn test_access_table_push() {
        let mut table = AccessTable::new();

        let access = Access {
            addr: 100,
            prev: 0,
            clk_prev: 0,
            next: 42,
            clk: 1,
        };
        table.push(access);

        assert_eq!(table.len(), 1);
        assert_eq!(table.addr[0], 100);
        assert_eq!(table.next[0], 42);
    }

    #[test]
    fn test_total_traces() {
        let mut tracer = Tracer::default();

        // Push some traces
        let rd = Access::default();
        let rs1 = Access::default();
        let rs2 = Access::default();

        tracer.add.push(0, 0, rd, rs1, rs2);
        tracer.add.push(1, 4, rd, rs1, rs2);
        tracer.sub.push(2, 8, rd, rs1, rs2);

        assert_eq!(tracer.total_traces(), 3);
    }

    #[test]
    fn test_trace_op_macro() {
        let mut tracer = Tracer::default();
        tracer.clk = 1;

        let rd = Access::default();
        let rs1 = Access::default();
        let rs2 = Access::default();

        trace_op!(add: tracer, 0x1000, rd, rs1, rs2);

        assert_eq!(tracer.add.len(), 1);
        assert_eq!(tracer.add.clk[0], 1);
        assert_eq!(tracer.add.pc[0], 0x1000);
    }
}
