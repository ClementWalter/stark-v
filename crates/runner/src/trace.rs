#![allow(clippy::too_many_arguments)]
//! Trace capture for zkVM execution.
//!
//! Each opcode defines its own columnar trace table.
//! Registers and memory use a unified Access structure that gets flattened into columns.

use simd::AlignedVec;

/// Default maximum clock difference allowed between accesses.
/// Must be consistent with max range-check in the prover.
/// RangeCheck20 is an array of from 0 to u20::MAX, i.e. to 2^20 - 1.
pub const DEFAULT_MAX_CLOCK_DIFF: u32 = (1 << 20) - 1;

// =============================================================================
// Generate all trace tables, Tracer struct, and trace_op! macro
// =============================================================================

runner_macros::define_trace_tables! {
    // ==========================================================================
    // 1. Base ALU Reg (add/sub/xor/or/and) - airs.md Section 1
    // ==========================================================================
    base_alu_reg: {
        clk, pc, rd, rs1, rs2,
        opcode_add_flag, opcode_sub_flag, opcode_xor_flag, opcode_or_flag, opcode_and_flag
    },

    // ==========================================================================
    // 2. Base ALU Imm (addi/xori/ori/andi) - airs.md Section 2
    // ==========================================================================
    base_alu_imm: {
        clk, pc, rd, rs1,
        imm_0, imm_1, imm_msb,
        opcode_add_flag, opcode_xor_flag, opcode_or_flag, opcode_and_flag
    },

    // ==========================================================================
    // 3. Shifts Reg (sll/srl/sra) - airs.md Section 3
    // ==========================================================================
    shifts_reg: {
        clk, pc, rd, rs1, rs2,
        rs1_sign,
        opcode_sll_flag, opcode_srl_flag, opcode_sra_flag,
        bit_multiplier_left, bit_multiplier_right,
        bit_shift_marker_0, bit_shift_marker_1, bit_shift_marker_2, bit_shift_marker_3,
        bit_shift_marker_4, bit_shift_marker_5, bit_shift_marker_6, bit_shift_marker_7,
        limb_shift_marker_0, limb_shift_marker_1, limb_shift_marker_2, limb_shift_marker_3,
        bit_shift_carry_0, bit_shift_carry_1, bit_shift_carry_2, bit_shift_carry_3
    },

    // ==========================================================================
    // 4. Shifts Imm (slli/srli/srai) - airs.md Section 4
    // ==========================================================================
    shifts_imm: {
        clk, pc, rd, rs1,
        rs1_sign, imm_truncated,
        opcode_sll_flag, opcode_srl_flag, opcode_sra_flag,
        bit_multiplier_left, bit_multiplier_right,
        bit_shift_marker_0, bit_shift_marker_1, bit_shift_marker_2, bit_shift_marker_3,
        bit_shift_marker_4, bit_shift_marker_5, bit_shift_marker_6, bit_shift_marker_7,
        limb_shift_marker_0, limb_shift_marker_1, limb_shift_marker_2, limb_shift_marker_3,
        bit_shift_carry_0, bit_shift_carry_1, bit_shift_carry_2, bit_shift_carry_3
    },

    // ==========================================================================
    // 5. Less Than Reg (slt/sltu) - airs.md Section 5
    // ==========================================================================
    lt_reg: {
        clk, pc, rd, rs1, rs2,
        cmp_result, rs1_msl_felt, rs2_msl_felt,
        opcode_slt_flag, opcode_sltu_flag,
        diff_marker_0, diff_marker_1, diff_marker_2, diff_marker_3,
        diff_val
    },

    // ==========================================================================
    // 6. Less Than Imm (slti/sltiu) - airs.md Section 6
    // ==========================================================================
    lt_imm: {
        clk, pc, rd, rs1,
        cmp_result, rs1_msl_felt,
        imm_0, imm_1, imm_msb,
        opcode_slti_flag, opcode_sltiu_flag,
        diff_marker_0, diff_marker_1, diff_marker_2, diff_marker_3,
        diff_val
    },

    // ==========================================================================
    // 7. Branch Equal (beq/bne) - airs.md Section 7
    // ==========================================================================
    branch_eq: {
        clk, pc, rs1, rs2,
        imm_felt, cmp_result,
        diff_inv_marker_0, diff_inv_marker_1, diff_inv_marker_2, diff_inv_marker_3,
        opcode_beq_flag, opcode_bne_flag
    },

    // ==========================================================================
    // 8. Branch Less Than (blt/bltu/bge/bgeu) - airs.md Section 8
    // ==========================================================================
    branch_lt: {
        clk, pc, rs1, rs2,
        rs1_msl_felt, rs2_msl_felt,
        imm_felt, cmp_result, cmp_lt,
        diff_marker_0, diff_marker_1, diff_marker_2, diff_marker_3,
        diff_val, branch_target,
        opcode_blt_flag, opcode_bltu_flag, opcode_bge_flag, opcode_bgeu_flag
    },

    // ==========================================================================
    // 9. LUI - airs.md Section 9
    // ==========================================================================
    lui: {
        clk, pc, rd,
        imm_0, imm_1, imm_2
    },

    // ==========================================================================
    // 10. AUIPC - airs.md Section 10
    // ==========================================================================
    auipc: {
        clk, pc, rd,
        imm_felt
    },

    // ==========================================================================
    // 11. JALR - airs.md Section 11
    // ==========================================================================
    jalr: {
        clk, pc, rd, rs1,
        to_pc_over_two, to_pc_lsb,
        imm_felt
    },

    // ==========================================================================
    // 12. JAL - airs.md Section 12
    // ==========================================================================
    jal: {
        clk, pc, rd,
        imm_felt
    },

    // ==========================================================================
    // 13. Load/Store (lb/lbu/lh/lhu/lw/sb/sh/sw) - airs.md Section 13
    // ==========================================================================
    load_store: {
        clk, pc, dst, rs1, src,
        r2_idx, imm_felt, src_msb,
        shift_amount,
        src_addr_selector, dst_addr_selector,
        marker_0, marker_1, marker_2, marker_3,
        opcode_lb_flag, opcode_lh_flag, opcode_lbu_flag, opcode_lhu_flag, opcode_lw_flag,
        opcode_sb_flag, opcode_sh_flag, opcode_sw_flag
    },

    // ==========================================================================
    // 14. MUL - airs.md Section 14
    // ==========================================================================
    mul: {
        clk, pc, rd, rs1, rs2
    },

    // ==========================================================================
    // 15. MULH (mulh/mulhsu/mulhu) - airs.md Section 15
    // ==========================================================================
    mulh: {
        clk, pc, rd, rs1, rs2,
        rd_high_0, rd_high_1, rd_high_2, rd_high_3,
        rs1_sign, rs2_sign,
        opcode_mulh_flag, opcode_mulhsu_flag, opcode_mulhu_flag
    },

    // ==========================================================================
    // 16. DIV (div/divu/rem/remu) - airs.md Section 16
    // ==========================================================================
    div: {
        clk, pc, rd, rs1, rs2,
        zero_divisor, r_zero,
        q_0, q_1, q_2, q_3,
        r_0, r_1, r_2, r_3,
        b_sign, c_sign, q_sign, sign_xor,
        c_sum_inv, r_sum_inv,
        r_abs_0, r_abs_1, r_abs_2, r_abs_3,
        r_inv_0, r_inv_1, r_inv_2, r_inv_3,
        lt_marker_0, lt_marker_1, lt_marker_2, lt_marker_3,
        lt_diff,
        opcode_div_flag, opcode_divu_flag, opcode_rem_flag, opcode_remu_flag
    },
}

// =============================================================================
// Tracer memory access methods and utils
// =============================================================================

/// Unified access record for both registers and memory.
///
/// - For registers: `addr` is the register index (0-31)
/// - For memory: `addr` is the byte address
/// - Values stored as `[u8; 4]` little-endian limbs (1-4 bytes meaningful)
///
/// Note: The current clock (`clk`) is not stored here because it's redundant
/// with the VM's `tracer.clk` at the time of the access.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Access {
    pub addr: u32,
    pub prev: u32,
    pub clk_prev: u32,
    pub next: u32,
}

impl std::fmt::Debug for Access {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Access")
            .field("addr", &format_args!("{:#x}", self.addr))
            .field("prev", &format_args!("{:#x}", self.prev))
            .field("clk_prev", &self.clk_prev)
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
/// - `clk == clk_prev + max_clock_diff` (fixed increment)
#[derive(Clone)]
pub struct AccessTable {
    pub addr: AlignedVec<u32>,
    pub value: AlignedVec<u32>,
    pub clk_prev: AlignedVec<u32>,
    pub max_clock_diff: u32,
}

impl Default for AccessTable {
    fn default() -> Self {
        Self {
            addr: AlignedVec::new(),
            value: AlignedVec::new(),
            clk_prev: AlignedVec::new(),
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
                clk_prev: self.clk_prev[i],
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
            clk_prev: AlignedVec::with_capacity(cap),
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
        self.clk_prev.push(access.clk_prev);
    }
}

impl Tracer {
    /// Generate and store intermediate accesses for clock catch-up.
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

        // Create the final access (clk is available from tracer.clk at call site)
        let final_access = Access {
            addr,
            prev,
            clk_prev: final_clk_prev,
            next,
        };

        // Update the register's clock
        self.reg_clk[idx as usize] = self.clk;

        final_access
    }

    /// Trace a memory access with gap-filling.
    /// All memory accesses are traced at 4-byte aligned addresses.
    /// Intermediate accesses are pushed to `mem_clk_update`.
    /// Returns only the final access.
    pub fn trace_mem_access(&mut self, addr: u32, prev: u32, next: u32) -> Access {
        // Always use 4-byte aligned address
        let aligned_addr = addr & !3;

        let clk_prev = self.mem_clk.get(&aligned_addr).copied().unwrap_or(0);

        // Generate intermediate catch-up accesses and get final clk_prev
        let final_clk_prev = self.fill_gap(GapTable::Mem, aligned_addr, prev, clk_prev, self.clk);

        // Update mem_clk after gap-filling
        if final_clk_prev != clk_prev {
            self.mem_clk.insert(aligned_addr, final_clk_prev);
        }

        // Create the final access (clk is available from tracer.clk at call site)
        let final_access = Access {
            addr: aligned_addr,
            prev,
            clk_prev: final_clk_prev,
            next,
        };

        // Update the memory word's clock
        self.mem_clk.insert(aligned_addr, self.clk);

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
                let clk_prev = self.table.clk_prev[self.idx];
                let value = self.table.value[self.idx];
                let access = Access {
                    addr: self.table.addr[self.idx],
                    prev: value,
                    clk_prev,
                    next: value, // For gap-filling, prev == next
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
        // Note: access.clk is no longer stored; use tracer.clk at call site
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
        // Note: access.clk is no longer stored; current clk is tracer.clk=2
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

        // Verify intermediates have correct clk_prev progression
        // Each intermediate's clk was clk_prev + max_clock_diff (now implicit)
        // Sequence: 0 -> 100 -> 200 -> 300 -> 350 (final)
        assert_eq!(tracer.mem_clk_update.clk_prev[0], 0);
        assert_eq!(tracer.mem_clk_update.clk_prev[1], 100);
        assert_eq!(tracer.mem_clk_update.clk_prev[2], 200);

        // Final access's clk_prev should be 300 (after 3 intermediates)
        assert_eq!(access.clk_prev, 300);
        // Final access's clk is tracer.clk=350, diff is 50 which is <= 100
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
        // Note: access.clk is no longer stored; current clk is tracer.clk=100
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
        // Note: access.clk is no longer stored; use tracer.clk at call site
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
        // Note: access.clk is no longer stored; current clk is tracer.clk=2
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

        // Verify intermediates have correct clk_prev progression
        // Each intermediate's clk was clk_prev + max_clock_diff (now implicit)
        // Sequence: 0 -> 100 -> 200 -> 300 -> 350 (final)
        assert_eq!(tracer.reg_clk_update.clk_prev[0], 0);
        assert_eq!(tracer.reg_clk_update.clk_prev[1], 100);
        assert_eq!(tracer.reg_clk_update.clk_prev[2], 200);

        // Final access's clk_prev should be 300 (after 3 intermediates)
        assert_eq!(access.clk_prev, 300);
        // Final access's clk is tracer.clk=350, diff is 50 which is <= 100
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

        // Verify intermediates have correct clk_prev progression: 0, 1, 2, 3
        // Each intermediate's clk was clk_prev + 1 (now implicit)
        assert_eq!(tracer.mem_clk_update.clk_prev[0], 0);
        assert_eq!(tracer.mem_clk_update.clk_prev[1], 1);
        assert_eq!(tracer.mem_clk_update.clk_prev[2], 2);
        assert_eq!(tracer.mem_clk_update.clk_prev[3], 3);

        // Final access's clk_prev is 4, and tracer.clk=5, so diff is 1
        assert_eq!(access.clk_prev, 4);
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
    fn test_base_alu_reg_table_push() {
        let mut table = BaseAluRegTable::new();

        let rd = Access {
            addr: 1,
            prev: 0,
            clk_prev: 0,
            next: 10,
        };
        let rs1 = Access {
            addr: 2,
            prev: 5,
            clk_prev: 0,
            next: 5,
        };
        let rs2 = Access {
            addr: 3,
            prev: 5,
            clk_prev: 0,
            next: 5,
        };

        // Push with opcode flags: add=1, sub=0, xor=0, or=0, and=0
        table.push(1, 0x1000, rd, rs1, rs2, 1, 0, 0, 0, 0);

        assert_eq!(table.len(), 1);
        assert_eq!(table.clk[0], 1);
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
            clk_prev: 0,
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
        tracer.clk = 1;

        let rd = Access::default();
        let rs1 = Access::default();
        let rs2 = Access::default();

        trace_op!(base_alu_reg: tracer, 0x1000, rd, rs1, rs2, 1, 0, 0, 0, 0);

        assert_eq!(tracer.base_alu_reg.len(), 1);
        assert_eq!(tracer.base_alu_reg.clk[0], 1);
        assert_eq!(tracer.base_alu_reg.pc[0], 0x1000);
    }

    // Test prover column generation for new family tables
    mod prover_column_tests {
        use super::prover_columns::*;

        #[test]
        fn test_base_alu_reg_columns_size() {
            // base_alu_reg: clk, pc, rd (10), rs1 (10), rs2 (10),
            // + 5 opcode flags = 37 total (no enabler - has flags)
            assert_eq!(BaseAluRegColumns::<()>::SIZE, 37);
        }

        #[test]
        fn test_base_alu_imm_columns_size() {
            // base_alu_imm: clk, pc, rd (10), rs1 (10),
            // + imm_0, imm_1, imm_msb (3) + 4 opcode flags = 29 total (no enabler - has flags)
            assert_eq!(BaseAluImmColumns::<()>::SIZE, 29);
        }

        #[test]
        fn test_lui_columns_size() {
            // LUI: enabler (1), clk, pc, rd (10), imm_0, imm_1, imm_2 = 16 total
            assert_eq!(LuiColumns::<()>::SIZE, 16);
        }

        #[test]
        fn test_load_store_columns_size() {
            // load_store: clk (1), pc (1), dst (10), rs1 (10), src (10),
            // + r2_idx, imm_felt, src_msb, shift_amount (4)
            // + src_addr_selector, dst_addr_selector (2)
            // + marker_0..3 (4) + 8 opcode flags = 50 total (no enabler - has flags)
            assert_eq!(LoadStoreColumns::<()>::SIZE, 50);
        }

        #[test]
        fn test_branch_eq_columns_size() {
            // branch_eq: clk (1), pc (1), rs1 (10), rs2 (10),
            // + imm_felt (1), cmp_result (1) + diff_inv_marker_0..3 (4) + 2 opcode flags = 30 total (no enabler - has flags)
            assert_eq!(BranchEqColumns::<()>::SIZE, 30);
        }

        #[test]
        fn test_jal_columns_size() {
            // JAL: enabler (1), clk, pc, rd (10), imm_felt = 14 total
            assert_eq!(JalColumns::<()>::SIZE, 14);
        }

        #[test]
        fn test_mul_columns_size() {
            // MUL: enabler (1), clk, pc, rd (10), rs1 (10), rs2 (10) = 33 total
            assert_eq!(MulColumns::<()>::SIZE, 33);
        }
    }
}
