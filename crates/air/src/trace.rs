//! Trace helpers and tracer methods.
//!
//! Table definitions and the `Tracer` struct are generated in [`crate::schema::trace`]
//! by `define_air!` and re-exported here.

use simd::AlignedVec;

/// Default maximum clock difference allowed between accesses.
/// Must be consistent with max range-check in the prover.
pub const DEFAULT_MAX_CLOCK_DIFF: u32 = (1 << 20) - 1;

// =============================================================================
// Tracer memory access methods and utils
// =============================================================================

/// Unified access record for both registers and memory.
///
/// - For registers: `addr` is the register index (0-31)
/// - For memory: `addr` is the byte address
/// - Values stored as `[u8; 4]` little-endian limbs (1-4 bytes meaningful)
///
/// Note: The current clock (`clock`) is not stored here because it's redundant
/// with the VM's `tracer.clock` at the time of the access.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Access {
    pub addr: u32,
    pub prev: u32,
    pub clock_prev: u32,
    pub next: u32,
}

impl std::fmt::Debug for Access {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Access")
            .field("addr", &format_args!("{:#x}", self.addr))
            .field("prev", &format_args!("{:#x}", self.prev))
            .field("clock_prev", &self.clock_prev)
            .field("next", &format_args!("{:#x}", self.next))
            .finish()
    }
}

// =============================================================================
// Columnar AccessTable (for clock update)
// =============================================================================

/// Columnar storage for Access records.
///
/// Simplified storage since for clock catch-up:
/// - `prev == next` (value unchanged)
/// - `clock == clock_prev + max_clock_diff` (fixed increment)
#[derive(Clone)]
pub struct AccessTable {
    pub addr: AlignedVec<u32>,
    pub value: AlignedVec<u32>,
    pub clock_prev: AlignedVec<u32>,
    pub max_clock_diff: u32,
}

impl Default for AccessTable {
    fn default() -> Self {
        Self {
            addr: AlignedVec::new(),
            value: AlignedVec::new(),
            clock_prev: AlignedVec::new(),
            max_clock_diff: DEFAULT_MAX_CLOCK_DIFF,
        }
    }
}

impl std::fmt::Debug for AccessTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut list = f.debug_list();
        for i in 0..self.len() {
            list.entry(&Access {
                addr: self.addr[i],
                prev: self.value[i],
                clock_prev: self.clock_prev[i],
                next: self.value[i],
            });
        }
        list.finish()
    }
}

impl AccessTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            addr: AlignedVec::with_capacity(cap),
            value: AlignedVec::with_capacity(cap),
            clock_prev: AlignedVec::with_capacity(cap),
            max_clock_diff: DEFAULT_MAX_CLOCK_DIFF,
        }
    }

    pub fn with_max_clock_diff(max_clock_diff: u32) -> Self {
        Self {
            max_clock_diff,
            ..Default::default()
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
        debug_assert_eq!(
            access.prev, access.next,
            "clock catch-up must not change value"
        );
        self.addr.push(access.addr);
        self.value.push(access.prev);
        self.clock_prev.push(access.clock_prev);
    }

    /// Consumes the table and returns columns in canonical order.
    /// Order matches the ClockUpdateColumns layout in the prover.
    pub fn into_columns(self) -> Vec<AlignedVec<u32>> {
        let len = self.len();
        let mut enabler = AlignedVec::with_capacity(len);
        for _ in 0..len {
            enabler.push(1);
        }

        let mut value_0 = AlignedVec::with_capacity(len);
        let mut value_1 = AlignedVec::with_capacity(len);
        let mut value_2 = AlignedVec::with_capacity(len);
        let mut value_3 = AlignedVec::with_capacity(len);
        for val in self.value.iter() {
            let val = *val;
            value_0.push(val & 0xFF);
            value_1.push((val >> 8) & 0xFF);
            value_2.push((val >> 16) & 0xFF);
            value_3.push((val >> 24) & 0xFF);
        }

        vec![
            enabler,
            self.addr,
            self.clock_prev,
            value_0,
            value_1,
            value_2,
            value_3,
        ]
    }

    /// Convert table to trace columns, padding to power of 2.
    /// Always produces columns with minimum log_size of 4 (16 rows),
    /// even for empty tables.
    pub fn into_witness(
        self,
    ) -> Vec<
        stwo::prover::poly::circle::CircleEvaluation<
            stwo::prover::backend::simd::SimdBackend,
            stwo::core::fields::m31::BaseField,
            stwo::prover::poly::BitReversedOrder,
        >,
    > {
        use stwo::core::poly::circle::CanonicCoset;
        use stwo::prover::backend::simd::column::BaseColumn;
        use stwo::prover::poly::circle::CircleEvaluation;

        let len = self.len() as u32;
        let log_size = len.next_power_of_two().ilog2().max(4);
        let padded_len = 1 << log_size;
        let columns = self.into_columns();
        let domain = CanonicCoset::new(log_size).circle_domain();

        columns
            .into_iter()
            .map(|mut col| {
                col.resize(padded_len, 0);
                let base_col: BaseColumn = col.into();
                CircleEvaluation::new(domain, base_col)
            })
            .collect()
    }

    pub fn to_witness(
        &self,
    ) -> Vec<
        stwo::prover::poly::circle::CircleEvaluation<
            stwo::prover::backend::simd::SimdBackend,
            stwo::core::fields::m31::BaseField,
            stwo::prover::poly::BitReversedOrder,
        >,
    > {
        self.clone().into_witness()
    }

    /// Returns an iterator over Access values stored in columnar form.
    pub fn iter(&self) -> AccessTableIter<'_> {
        AccessTableIter {
            table: self,
            idx: 0,
        }
    }
}

/// Iterator over [`AccessTable`] that yields [`Access`] values.
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
            let clock_prev = self.table.clock_prev[self.idx];
            let value = self.table.value[self.idx];
            let access = Access {
                addr: self.table.addr[self.idx],
                prev: value,
                clock_prev,
                next: value,
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

pub use crate::schema::trace::*;

impl Tracer {
    /// Generate and store intermediate accesses for clock catch-up.
    fn fill_gap(
        &mut self,
        table: GapTable,
        addr: u32,
        value: u32,
        clock_prev: u32,
        target_clock: u32,
    ) -> u32 {
        let mut current_clock = clock_prev;

        while target_clock.saturating_sub(current_clock) > self.max_clock_diff {
            let next_clock = current_clock.saturating_add(self.max_clock_diff);
            let access = Access {
                addr,
                prev: value,
                clock_prev: current_clock,
                next: value,
            };
            match table {
                GapTable::Reg => self.reg_clock_update.push(access),
                GapTable::Mem => self.mem_clock_update.push(access),
            }
            current_clock = next_clock;
        }

        current_clock
    }

    /// Trace a register access with gap-filling.
    /// Intermediate accesses are pushed to `reg_clock_update`.
    /// Returns only the final access.
    pub fn trace_reg_access(&mut self, idx: u8, prev: u32, next: u32) -> Access {
        let clock_prev = self.reg_clock[idx as usize];
        let addr = idx as u32;

        // Generate intermediate catch-up accesses and get final clock_prev
        let final_clock_prev = self.fill_gap(GapTable::Reg, addr, prev, clock_prev, self.clock);

        // Update the register's clock after gap-filling
        if final_clock_prev != clock_prev {
            self.reg_clock[idx as usize] = final_clock_prev;
        }

        // Create the final access (clock is available from tracer.clock at call site)
        let final_access = Access {
            addr,
            prev,
            clock_prev: final_clock_prev,
            next,
        };

        // Update the register's clock
        self.reg_clock[idx as usize] = self.clock;

        final_access
    }

    /// Trace a memory access with gap-filling.
    /// All memory accesses are traced at 4-byte aligned addresses.
    /// Intermediate accesses are pushed to `mem_clock_update`.
    /// Returns only the final access.
    pub fn trace_mem_access(&mut self, addr: u32, prev: u32, next: u32) -> Access {
        // Always use 4-byte aligned address
        let aligned_addr = addr & !3;

        self.mem_initial.entry(aligned_addr).or_insert(prev);

        let clock_prev = self.mem_clock.get(&aligned_addr).copied().unwrap_or(0);

        // Generate intermediate catch-up accesses and get final clock_prev
        let final_clock_prev =
            self.fill_gap(GapTable::Mem, aligned_addr, prev, clock_prev, self.clock);

        // Update mem_clock after gap-filling
        if final_clock_prev != clock_prev {
            self.mem_clock.insert(aligned_addr, final_clock_prev);
        }

        // Create the final access (clock is available from tracer.clock at call site)
        let final_access = Access {
            addr: aligned_addr,
            prev,
            clock_prev: final_clock_prev,
            next,
        };

        // Update the memory word's clock
        self.mem_clock.insert(aligned_addr, self.clock);

        final_access
    }

    pub fn trace_instr_access(&mut self, pc: u32) {
        *self.program_reads.entry(pc).or_insert(0) += 1;
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
    const MEM_ADDR: u32 = 0x2000;

    // =========================================================================
    // Tracer Construction
    // =========================================================================

    #[test]
    fn test_default_tracer() {
        let tracer = Tracer::default();
        assert_eq!(tracer.clock, 0);
        assert_eq!(tracer.max_clock_diff, DEFAULT_MAX_CLOCK_DIFF);
        assert_eq!(tracer.reg_clock, [0; 32]);
        assert!(tracer.mem_clock.is_empty());
        assert!(tracer.mem_initial.is_empty());
        assert!(tracer.program_reads.is_empty());
    }

    #[test]
    fn test_with_max_clock_diff() {
        let tracer = Tracer::with_max_clock_diff(100);
        assert_eq!(tracer.max_clock_diff, 100);
        assert_eq!(tracer.clock, 0);
    }

    // =========================================================================
    // Memory Access Tracing
    // =========================================================================

    #[test]
    fn test_trace_mem_access_first_access() {
        let mut tracer = Tracer::default();
        tracer.clock = 10;

        let access = tracer.trace_mem_access(MEM_ADDR, 0x42, 0x42);

        assert_eq!(access.addr, MEM_ADDR);
        assert_eq!(access.prev, 0x42);
        assert_eq!(access.next, 0x42);
        assert_eq!(access.clock_prev, 0);
        // Note: access.clock is no longer stored; use tracer.clock at call site
        assert!(tracer.mem_clock_update.is_empty());
    }

    #[test]
    fn test_trace_mem_access_consecutive() {
        let mut tracer = Tracer::default();

        tracer.clock = 1;
        tracer.trace_mem_access(MEM_ADDR, 0x11, 0x11);

        tracer.clock = 2;
        let access = tracer.trace_mem_access(MEM_ADDR, 0x11, 0x22);

        assert_eq!(access.clock_prev, 1);
        // Note: access.clock is no longer stored; current clock is tracer.clock=2
        assert_eq!(access.prev, 0x11);
        assert_eq!(access.next, 0x22);
        assert!(tracer.mem_clock_update.is_empty());
    }

    #[test]
    fn test_trace_mem_access_gap_filling() {
        let mut tracer = Tracer::with_max_clock_diff(100);

        tracer.clock = 0;
        tracer.trace_mem_access(MEM_ADDR, 0x42, 0x42);

        tracer.clock = 350;
        let access = tracer.trace_mem_access(MEM_ADDR, 0x42, 0x42);

        // Gap of 350 with max_diff 100 needs 3 intermediates
        assert_eq!(
            tracer.mem_clock_update.len(),
            3,
            "Expected 3 intermediates, got {}",
            tracer.mem_clock_update.len()
        );

        // Verify intermediates have correct clock_prev progression
        // Each intermediate's clock was clock_prev + max_clock_diff (now implicit)
        // Sequence: 0 -> 100 -> 200 -> 300 -> 350 (final)
        assert_eq!(tracer.mem_clock_update.clock_prev[0], 0);
        assert_eq!(tracer.mem_clock_update.clock_prev[1], 100);
        assert_eq!(tracer.mem_clock_update.clock_prev[2], 200);

        // Final access's clock_prev should be 300 (after 3 intermediates)
        assert_eq!(access.clock_prev, 300);
        // Final access's clock is tracer.clock=350, diff is 50 which is <= 100
    }

    #[test]
    fn test_trace_mem_access_exact_max_diff() {
        let mut tracer = Tracer::with_max_clock_diff(100);

        tracer.clock = 0;
        tracer.trace_mem_access(MEM_ADDR, 0, 0);

        tracer.clock = 100;
        let access = tracer.trace_mem_access(MEM_ADDR, 0, 0);

        // Exactly at max_clock_diff - no intermediate needed
        assert!(tracer.mem_clock_update.is_empty());
        assert_eq!(access.clock_prev, 0);
        // Note: access.clock is no longer stored; current clock is tracer.clock=100
    }

    #[test]
    fn test_trace_mem_access_preserves_value() {
        let mut tracer = Tracer::with_max_clock_diff(50);

        tracer.clock = 0;
        tracer.trace_mem_access(MEM_ADDR, 0xAB, 0xAB);

        tracer.clock = 200;
        let access = tracer.trace_mem_access(MEM_ADDR, 0xAB, 0xAB);

        // All intermediate accesses should preserve the value
        for intermediate in &tracer.mem_clock_update {
            assert_eq!(intermediate.prev, 0xAB);
            assert_eq!(intermediate.next, 0xAB);
        }
        // Final access should also preserve value
        assert_eq!(access.prev, 0xAB);
        assert_eq!(access.next, 0xAB);
    }

    #[test]
    fn test_trace_mem_access_updates_mem_clock() {
        let mut tracer = Tracer::default();
        tracer.clock = 10;

        tracer.trace_mem_access(MEM_ADDR, 0, 0);

        assert_eq!(tracer.mem_clock.get(&MEM_ADDR), Some(&10));
    }

    // =========================================================================
    // Register Access Tracing
    // =========================================================================

    #[test]
    fn test_trace_reg_access_first_access() {
        let mut tracer = Tracer::default();
        tracer.clock = 10;

        let access = tracer.trace_reg_access(5, 0x42, 0x42);

        assert_eq!(access.addr, 5);
        assert_eq!(access.prev, 0x42);
        assert_eq!(access.next, 0x42);
        assert_eq!(access.clock_prev, 0);
        // Note: access.clock is no longer stored; use tracer.clock at call site
        assert!(tracer.reg_clock_update.is_empty());
    }

    #[test]
    fn test_trace_reg_access_consecutive() {
        let mut tracer = Tracer::default();

        tracer.clock = 1;
        tracer.trace_reg_access(5, 0x11, 0x11);

        tracer.clock = 2;
        let access = tracer.trace_reg_access(5, 0x11, 0x22);

        assert_eq!(access.clock_prev, 1);
        // Note: access.clock is no longer stored; current clock is tracer.clock=2
        assert_eq!(access.prev, 0x11);
        assert_eq!(access.next, 0x22);
        assert!(tracer.reg_clock_update.is_empty());
    }

    #[test]
    fn test_trace_reg_access_gap_filling() {
        let mut tracer = Tracer::with_max_clock_diff(100);

        tracer.clock = 0;
        tracer.trace_reg_access(5, 0x42, 0x42);

        tracer.clock = 350;
        let access = tracer.trace_reg_access(5, 0x42, 0x42);

        // Gap of 350 with max_diff 100 needs 3 intermediates
        assert_eq!(
            tracer.reg_clock_update.len(),
            3,
            "Expected 3 intermediates, got {}",
            tracer.reg_clock_update.len()
        );

        // Verify intermediates have correct clock_prev progression
        // Each intermediate's clock was clock_prev + max_clock_diff (now implicit)
        // Sequence: 0 -> 100 -> 200 -> 300 -> 350 (final)
        assert_eq!(tracer.reg_clock_update.clock_prev[0], 0);
        assert_eq!(tracer.reg_clock_update.clock_prev[1], 100);
        assert_eq!(tracer.reg_clock_update.clock_prev[2], 200);

        // Final access's clock_prev should be 300 (after 3 intermediates)
        assert_eq!(access.clock_prev, 300);
        // Final access's clock is tracer.clock=350, diff is 50 which is <= 100
    }

    #[test]
    fn test_trace_reg_access_x0() {
        let mut tracer = Tracer::default();
        tracer.clock = 10;

        // x0 can still be traced - the caller handles x0 semantics
        let access = tracer.trace_reg_access(0, 0, 0);

        assert_eq!(access.addr, 0);
        assert_eq!(access.prev, 0);
        assert_eq!(access.next, 0);
        assert!(tracer.reg_clock_update.is_empty());
    }

    #[test]
    fn test_trace_reg_access_updates_reg_clock() {
        let mut tracer = Tracer::default();
        tracer.clock = 10;

        tracer.trace_reg_access(5, 0, 0);

        assert_eq!(tracer.reg_clock[5], 10);
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_max_clock_diff_one() {
        let mut tracer = Tracer::with_max_clock_diff(1);

        tracer.clock = 0;
        tracer.trace_mem_access(MEM_ADDR, 0, 0);

        tracer.clock = 5;
        let access = tracer.trace_mem_access(MEM_ADDR, 0, 0);

        // With max_clock_diff=1, gap of 5 needs 4 intermediates + 1 final
        assert_eq!(tracer.mem_clock_update.len(), 4);

        // Verify intermediates have correct clock_prev progression: 0, 1, 2, 3
        // Each intermediate's clock was clock_prev + 1 (now implicit)
        assert_eq!(tracer.mem_clock_update.clock_prev[0], 0);
        assert_eq!(tracer.mem_clock_update.clock_prev[1], 1);
        assert_eq!(tracer.mem_clock_update.clock_prev[2], 2);
        assert_eq!(tracer.mem_clock_update.clock_prev[3], 3);

        // Final access's clock_prev is 4, and tracer.clock=5, so diff is 1
        assert_eq!(access.clock_prev, 4);
    }

    #[test]
    fn test_max_clock_diff_max() {
        let mut tracer = Tracer::with_max_clock_diff(u32::MAX);

        tracer.clock = 0;
        tracer.trace_mem_access(MEM_ADDR, 0, 0);

        tracer.clock = u32::MAX - 1;
        tracer.trace_mem_access(MEM_ADDR, 0, 0);

        // No intermediate ever needed
        assert!(tracer.mem_clock_update.is_empty());
    }

    // =========================================================================
    // Columnar Table Tests
    // =========================================================================

    #[test]
    fn test_base_alu_reg_table_push() {
        let mut table = BaseAluRegTable::new();

        let rd = Access {
            addr: 1,
            prev: 0,
            clock_prev: 0,
            next: 10,
        };
        let rs1 = Access {
            addr: 2,
            prev: 5,
            clock_prev: 0,
            next: 5,
        };
        let rs2 = Access {
            addr: 3,
            prev: 5,
            clock_prev: 0,
            next: 5,
        };

        // Push with opcode flags: add=1, sub=0, xor=0, or=0, and=0
        table.push(1, 0x1000, rd, rs1, rs2, 1, 0, 0, 0, 0);

        assert_eq!(table.len(), 1);
        assert_eq!(table.clock[0], 1);
        assert_eq!(table.pc[0], 0x1000);
        assert_eq!(table.rd_addr[0], 1);
        assert_eq!(table.rd_next[0], 10);
        assert_eq!(table.rs1_addr[0], 2);
        assert_eq!(table.rs2_addr[0], 3);
        assert_eq!(table.opcode_add_flag[0], 1);
        assert_eq!(table.opcode_sub_flag[0], 0);
    }

    #[test]
    fn test_access_table_push() {
        let mut table = AccessTable::with_max_clock_diff(100);

        // AccessTable is for gap-filling: prev == next
        let value = 42u32;
        let access = Access {
            addr: 100,
            prev: value,
            clock_prev: 0,
            next: value,
        };
        table.push(access);

        assert_eq!(table.len(), 1);
        assert_eq!(table.addr[0], 100);
        assert_eq!(table.value[0], value);
    }

    #[test]
    fn test_total_traces() {
        let mut tracer = Tracer::default();

        // Push some traces
        let rd = Access::default();
        let rs1 = Access::default();
        let rs2 = Access::default();

        // base_alu_reg with add flag
        tracer.base_alu_reg.push(0, 0, rd, rs1, rs2, 1, 0, 0, 0, 0);
        tracer.base_alu_reg.push(1, 4, rd, rs1, rs2, 1, 0, 0, 0, 0);
        // base_alu_reg with sub flag
        tracer.base_alu_reg.push(2, 8, rd, rs1, rs2, 0, 1, 0, 0, 0);

        assert_eq!(tracer.total_traces(), 3);
    }

    #[test]
    fn test_trace_op_macro() {
        let mut tracer = Tracer::default();
        tracer.clock = 1;

        let rd = Access::default();
        let rs1 = Access::default();
        let rs2 = Access::default();

        trace_op!(base_alu_reg: tracer, 0x1000, rd, rs1, rs2, 1, 0, 0, 0, 0);

        assert_eq!(tracer.base_alu_reg.len(), 1);
        assert_eq!(tracer.base_alu_reg.clock[0], 1);
        assert_eq!(tracer.base_alu_reg.pc[0], 0x1000);
    }

    // Test prover column generation for new family tables
    mod prover_column_tests {
        use super::prover_columns::*;

        #[test]
        fn test_base_alu_reg_columns_size() {
            // base_alu_reg: clock, pc, rd (10), rs1 (10), rs2 (10),
            // + 5 opcode flags = 37 total (no enabler - has flags)
            assert_eq!(BaseAluRegColumns::<()>::SIZE, 37);
        }

        #[test]
        fn test_base_alu_imm_columns_size() {
            // base_alu_imm: clock, pc, rd (10), rs1 (10),
            // + imm_0, imm_1, imm_msb (3) + 4 opcode flags = 29 total (no enabler - has flags)
            assert_eq!(BaseAluImmColumns::<()>::SIZE, 29);
        }

        #[test]
        fn test_lui_columns_size() {
            // LUI: enabler (1), clock, pc, rd (10), imm_0, imm_1, imm_2 = 16 total
            assert_eq!(LuiColumns::<()>::SIZE, 16);
        }

        #[test]
        fn test_load_store_columns_size() {
            // load_store: clock (1), pc (1), dst (10), rs1 (10), src (10),
            // + r2_idx, imm_felt, src_msb, shift_amount (4)
            // + src_addr_selector, dst_addr_selector (2)
            // + marker_0..3 (4) + 8 opcode flags = 50 total (no enabler - has flags)
            assert_eq!(LoadStoreColumns::<()>::SIZE, 50);
        }

        #[test]
        fn test_branch_eq_columns_size() {
            // branch_eq: clock (1), pc (1), rs1 (10), rs2 (10),
            // + imm_felt (1), cmp_result (1) + diff_inv_marker_0..3 (4) + 2 opcode flags = 30 total (no enabler - has flags)
            assert_eq!(BranchEqColumns::<()>::SIZE, 30);
        }

        #[test]
        fn test_jal_columns_size() {
            // JAL: enabler (1), clock, pc, rd (10), imm_felt = 14 total
            assert_eq!(JalColumns::<()>::SIZE, 14);
        }

        #[test]
        fn test_mul_columns_size() {
            // MUL: enabler (1), clock, pc, rd (10), rs1 (10), rs2 (10) = 33 total
            assert_eq!(MulColumns::<()>::SIZE, 33);
        }
    }

    // Test derived columns and constraints declared in define_trace_tables!
    mod derived_column_tests {
        use super::prover_columns::*;
        use stwo::core::fields::m31::BaseField;

        fn f(v: u32) -> BaseField {
            BaseField::from_u32_unchecked(v)
        }

        /// All-zero LUI columns, mutated per test.
        fn zero_lui_cols() -> LuiColumns<BaseField> {
            LuiColumns::from_iter(std::iter::repeat_n(f(0), LuiColumns::<()>::SIZE))
        }

        /// All-zero Base ALU Imm columns, mutated per test.
        fn zero_base_alu_imm_cols() -> BaseAluImmColumns<BaseField> {
            BaseAluImmColumns::from_iter(std::iter::repeat_n(f(0), BaseAluImmColumns::<()>::SIZE))
        }

        #[test]
        fn test_lui_imm_combines_limbs() {
            let mut cols = zero_lui_cols();
            cols.imm_0 = f(3);
            cols.imm_1 = f(5);
            cols.imm_2 = f(7);
            assert_eq!(cols.imm(), f(3 + 5 * (1 << 4) + 7 * (1 << 12)));
        }

        #[test]
        fn test_lui_pc_next_adds_four() {
            let mut cols = zero_lui_cols();
            cols.pc = f(0x1000);
            assert_eq!(cols.pc_next(), f(0x1004));
        }

        #[test]
        fn test_lui_rd_clock_diff() {
            let mut cols = zero_lui_cols();
            cols.clock = f(10);
            cols.rd_clock_prev = f(4);
            assert_eq!(cols.rd_clock_diff(), f(6));
        }

        #[test]
        fn test_lui_enabler_booleanity_holds_for_one() {
            let mut cols = zero_lui_cols();
            cols.enabler = f(1);
            assert_eq!(cols.constraints()[0], f(0));
        }

        #[test]
        fn test_lui_enabler_booleanity_fails_for_two() {
            let mut cols = zero_lui_cols();
            cols.enabler = f(2);
            assert_ne!(cols.constraints()[0], f(0));
        }

        #[test]
        fn test_base_alu_imm_enabler_sums_flags() {
            let mut cols = zero_base_alu_imm_cols();
            cols.opcode_add_flag = f(1);
            cols.opcode_or_flag = f(1);
            assert_eq!(cols.enabler(), f(2));
        }

        #[test]
        fn test_base_alu_imm_expected_opcode_id_selects_active_flag() {
            let mut cols = zero_base_alu_imm_cols();
            cols.opcode_xor_flag = f(1);
            assert_eq!(
                cols.expected_opcode_id(),
                f(crate::decode::Opcode::Xori as u32)
            );
        }

        #[test]
        fn test_base_alu_imm_carry_0_detects_limb_overflow() {
            let mut cols = zero_base_alu_imm_cols();
            // 255 + 1 = 256 = 0 with carry 1 over an 8-bit limb
            cols.rs1_next_0 = f(255);
            cols.imm_0 = f(1);
            cols.rd_next_0 = f(0);
            assert_eq!(cols.carry_0(), f(1));
        }

        #[test]
        fn test_base_alu_imm_carry_1_chains_carry_0() {
            let mut cols = zero_base_alu_imm_cols();
            // Limb 0 overflows; limb 1 receives the carry and overflows too
            cols.rs1_next_0 = f(255);
            cols.imm_0 = f(1);
            cols.rd_next_0 = f(0);
            cols.rs1_next_1 = f(255);
            cols.rd_next_1 = f(0);
            assert_eq!(cols.carry_1(), f(1));
        }

        #[test]
        fn test_base_alu_imm_carry_booleanity_holds_for_valid_add() {
            let mut cols = zero_base_alu_imm_cols();
            cols.opcode_add_flag = f(1);
            // rs1 = 255, imm = 1: rd = 256, i.e. limb 0 wraps to 0 and limb 1 is 1
            cols.rs1_next_0 = f(255);
            cols.imm_0 = f(1);
            cols.rd_next_0 = f(0);
            cols.rd_next_1 = f(1);
            assert!(cols.constraints().iter().all(|c| *c == f(0)));
        }

        #[test]
        fn test_at_extracts_row_values() {
            // Column c holds [c, c + 100]; pc is the third column (index 2)
            let data: Vec<Vec<BaseField>> = (0..LuiColumns::<()>::SIZE as u32)
                .map(|c| vec![f(c), f(c + 100)])
                .collect();
            let cols = LuiColumns::from_iter(data.iter());
            assert_eq!(cols.at(1).pc, f(102));
        }
    }

    // =========================================================================
    // Table Debug Tests
    // =========================================================================

    mod debug_table_tests {
        use super::*;

        #[test]
        fn test_base_alu_reg_table_to_table() {
            let mut table = BaseAluRegTable::new();

            let rd = Access {
                addr: 1,
                prev: 0,
                clock_prev: 0,
                next: 10,
            };
            let rs1 = Access {
                addr: 2,
                prev: 5,
                clock_prev: 1,
                next: 5,
            };
            let rs2 = Access {
                addr: 3,
                prev: 7,
                clock_prev: 2,
                next: 7,
            };

            // Push two rows
            table.push(1, 0x1000, rd, rs1, rs2, 1, 0, 0, 0, 0);
            table.push(2, 0x1004, rd, rs1, rs2, 0, 1, 0, 0, 0);

            table.to_table().to_string();
        }

        #[test]
        fn test_lui_table_to_table_with_enabler() {
            // LUI has an enabler column (no opcode flags)
            let mut table = LuiTable::new();

            let rd = Access {
                addr: 10,
                prev: 0,
                clock_prev: 0,
                next: 0x12345000,
            };

            table.push(1, 0x1000, rd, 0x12, 0x34, 0x50);

            // Inspect header cells, not the rendered string: the dynamic
            // arrangement truncates wide headers to the terminal width.
            let headers: Vec<String> = table
                .to_table()
                .header()
                .expect("headers are always set")
                .cell_iter()
                .map(|cell| cell.content())
                .collect();
            assert!(headers.contains(&"enabler".to_string()));
        }

        #[test]
        fn test_empty_table_to_table() {
            let table = BaseAluRegTable::new();

            // An empty table still carries its headers; inspect the cells,
            // not the rendered string, which truncates to the terminal width.
            let headers: Vec<String> = table
                .to_table()
                .header()
                .expect("headers are always set")
                .cell_iter()
                .map(|cell| cell.content())
                .collect();
            assert!(headers.contains(&"clock".to_string()));
        }

        #[test]
        fn test_tracer_print_tables() {
            let mut tracer = Tracer::default();

            // Add some traces
            let rd = Access::default();
            let rs1 = Access::default();
            let rs2 = Access::default();

            tracer.base_alu_reg.push(0, 0, rd, rs1, rs2, 1, 0, 0, 0, 0);
            tracer.base_alu_reg.push(1, 4, rd, rs1, rs2, 1, 0, 0, 0, 0);

            // This should not panic
            tracer.print_tables(Some(10), Some(10));
        }

        #[test]
        fn test_tracer_print_tables_empty() {
            let tracer = Tracer::default();

            // Empty tracer should not panic
            tracer.print_tables(None, None);
        }

        #[test]
        fn test_multiple_tables_to_table() {
            let mut tracer = Tracer::default();

            // Add traces to different tables
            let rd = Access::default();
            let rs1 = Access::default();
            let rs2 = Access::default();

            tracer.base_alu_reg.push(0, 0, rd, rs1, rs2, 1, 0, 0, 0, 0);
            tracer.lui.push(1, 4, rd, 0, 0, 0);
            tracer.jal.push(2, 8, rd, 100);

            // Each table should produce valid output
            let base_alu_output = tracer.base_alu_reg.to_table().to_string();
            let lui_output = tracer.lui.to_table().to_string();
            let jal_output = tracer.jal.to_table().to_string();

            // LUI and JAL have enabler columns, BaseAluReg doesn't
            assert!(lui_output.contains("enabler"));
            assert!(jal_output.contains("enabler"));
            assert!(!base_alu_output.contains("enabler"));
        }
    }
}
