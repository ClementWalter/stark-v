#![allow(clippy::too_many_arguments)]
//! Trace capture for zkVM execution.
//!
//! Each opcode defines its own columnar trace table.
//! Registers and memory use a unified Access structure that gets flattened into columns.

use simd::AlignedVec;

use crate::poseidon2::POSEIDON2_TRACE_COLUMNS;

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

    // ==========================================================================
    // 17. Program commitment table
    // ==========================================================================
    program: {
        addr, value_0, value_1, value_2, value_3, multiplicity, root
    },

    // ==========================================================================
    // 18. Memory commitment table (initial/final)
    // ==========================================================================
    memory: {
        addr, clk,
        value_0, value_1, value_2, value_3,
        multiplicity, root
    },

    // ==========================================================================
    // 19. Merkle tree nodes
    // ==========================================================================
    merkle: {
        index, depth,
        left_value, right_value, parent_value,
        left_multiplicity, right_multiplicity, parent_multiplicity,
        root
    },

    // ==========================================================================
    // 20. Poseidon2 hash trace
    // ==========================================================================
    poseidon2: {
        state0, state1, state2, state3, state4, state5, state6, state7, state8, state9, state10,
        state11, state12, state13, state14, state15, full0_sq1_0, full0_sq1_1, full0_sq1_2,
        full0_sq1_3, full0_sq1_4, full0_sq1_5, full0_sq1_6, full0_sq1_7, full0_sq1_8, full0_sq1_9,
        full0_sq1_10, full0_sq1_11, full0_sq1_12, full0_sq1_13, full0_sq1_14, full0_sq1_15,
        full0_sq2_0, full0_sq2_1, full0_sq2_2, full0_sq2_3, full0_sq2_4, full0_sq2_5, full0_sq2_6,
        full0_sq2_7, full0_sq2_8, full0_sq2_9, full0_sq2_10, full0_sq2_11, full0_sq2_12,
        full0_sq2_13, full0_sq2_14, full0_sq2_15, full0_mix_0, full0_mix_1, full0_mix_2,
        full0_mix_3, full0_mix_4, full0_mix_5, full0_mix_6, full0_mix_7, full0_mix_8, full0_mix_9,
        full0_mix_10, full0_mix_11, full0_mix_12, full0_mix_13, full0_mix_14, full0_mix_15,
        full1_sq1_0, full1_sq1_1, full1_sq1_2, full1_sq1_3, full1_sq1_4, full1_sq1_5, full1_sq1_6,
        full1_sq1_7, full1_sq1_8, full1_sq1_9, full1_sq1_10, full1_sq1_11, full1_sq1_12,
        full1_sq1_13, full1_sq1_14, full1_sq1_15, full1_sq2_0, full1_sq2_1, full1_sq2_2,
        full1_sq2_3, full1_sq2_4, full1_sq2_5, full1_sq2_6, full1_sq2_7, full1_sq2_8, full1_sq2_9,
        full1_sq2_10, full1_sq2_11, full1_sq2_12, full1_sq2_13, full1_sq2_14, full1_sq2_15,
        full1_mix_0, full1_mix_1, full1_mix_2, full1_mix_3, full1_mix_4, full1_mix_5, full1_mix_6,
        full1_mix_7, full1_mix_8, full1_mix_9, full1_mix_10, full1_mix_11, full1_mix_12,
        full1_mix_13, full1_mix_14, full1_mix_15, full2_sq1_0, full2_sq1_1, full2_sq1_2,
        full2_sq1_3, full2_sq1_4, full2_sq1_5, full2_sq1_6, full2_sq1_7, full2_sq1_8, full2_sq1_9,
        full2_sq1_10, full2_sq1_11, full2_sq1_12, full2_sq1_13, full2_sq1_14, full2_sq1_15,
        full2_sq2_0, full2_sq2_1, full2_sq2_2, full2_sq2_3, full2_sq2_4, full2_sq2_5, full2_sq2_6,
        full2_sq2_7, full2_sq2_8, full2_sq2_9, full2_sq2_10, full2_sq2_11, full2_sq2_12,
        full2_sq2_13, full2_sq2_14, full2_sq2_15, full2_mix_0, full2_mix_1, full2_mix_2,
        full2_mix_3, full2_mix_4, full2_mix_5, full2_mix_6, full2_mix_7, full2_mix_8, full2_mix_9,
        full2_mix_10, full2_mix_11, full2_mix_12, full2_mix_13, full2_mix_14, full2_mix_15,
        full3_sq1_0, full3_sq1_1, full3_sq1_2, full3_sq1_3, full3_sq1_4, full3_sq1_5, full3_sq1_6,
        full3_sq1_7, full3_sq1_8, full3_sq1_9, full3_sq1_10, full3_sq1_11, full3_sq1_12,
        full3_sq1_13, full3_sq1_14, full3_sq1_15, full3_sq2_0, full3_sq2_1, full3_sq2_2,
        full3_sq2_3, full3_sq2_4, full3_sq2_5, full3_sq2_6, full3_sq2_7, full3_sq2_8, full3_sq2_9,
        full3_sq2_10, full3_sq2_11, full3_sq2_12, full3_sq2_13, full3_sq2_14, full3_sq2_15,
        full3_mix_0, full3_mix_1, full3_mix_2, full3_mix_3, full3_mix_4, full3_mix_5, full3_mix_6,
        full3_mix_7, full3_mix_8, full3_mix_9, full3_mix_10, full3_mix_11, full3_mix_12,
        full3_mix_13, full3_mix_14, full3_mix_15, partial0_sq1, partial0_sq2, partial0_mul,
        partial1_sq1, partial1_sq2, partial1_mul, partial2_sq1, partial2_sq2, partial2_mul,
        partial3_sq1, partial3_sq2, partial3_mul, partial4_sq1, partial4_sq2, partial4_mul,
        partial5_sq1, partial5_sq2, partial5_mul, partial6_sq1, partial6_sq2, partial6_mul,
        partial7_sq1, partial7_sq2, partial7_mul, partial8_sq1, partial8_sq2, partial8_mul,
        partial9_sq1, partial9_sq2, partial9_mul, partial10_sq1, partial10_sq2, partial10_mul,
        partial11_sq1, partial11_sq2, partial11_mul, partial12_sq1, partial12_sq2, partial12_mul,
        partial13_sq1, partial13_sq2, partial13_mul, full4_sq1_0, full4_sq1_1, full4_sq1_2,
        full4_sq1_3, full4_sq1_4, full4_sq1_5, full4_sq1_6, full4_sq1_7, full4_sq1_8, full4_sq1_9,
        full4_sq1_10, full4_sq1_11, full4_sq1_12, full4_sq1_13, full4_sq1_14, full4_sq1_15,
        full4_sq2_0, full4_sq2_1, full4_sq2_2, full4_sq2_3, full4_sq2_4, full4_sq2_5, full4_sq2_6,
        full4_sq2_7, full4_sq2_8, full4_sq2_9, full4_sq2_10, full4_sq2_11, full4_sq2_12,
        full4_sq2_13, full4_sq2_14, full4_sq2_15, full4_mix_0, full4_mix_1, full4_mix_2,
        full4_mix_3, full4_mix_4, full4_mix_5, full4_mix_6, full4_mix_7, full4_mix_8, full4_mix_9,
        full4_mix_10, full4_mix_11, full4_mix_12, full4_mix_13, full4_mix_14, full4_mix_15,
        full5_sq1_0, full5_sq1_1, full5_sq1_2, full5_sq1_3, full5_sq1_4, full5_sq1_5, full5_sq1_6,
        full5_sq1_7, full5_sq1_8, full5_sq1_9, full5_sq1_10, full5_sq1_11, full5_sq1_12,
        full5_sq1_13, full5_sq1_14, full5_sq1_15, full5_sq2_0, full5_sq2_1, full5_sq2_2,
        full5_sq2_3, full5_sq2_4, full5_sq2_5, full5_sq2_6, full5_sq2_7, full5_sq2_8, full5_sq2_9,
        full5_sq2_10, full5_sq2_11, full5_sq2_12, full5_sq2_13, full5_sq2_14, full5_sq2_15,
        full5_mix_0, full5_mix_1, full5_mix_2, full5_mix_3, full5_mix_4, full5_mix_5, full5_mix_6,
        full5_mix_7, full5_mix_8, full5_mix_9, full5_mix_10, full5_mix_11, full5_mix_12,
        full5_mix_13, full5_mix_14, full5_mix_15, full6_sq1_0, full6_sq1_1, full6_sq1_2,
        full6_sq1_3, full6_sq1_4, full6_sq1_5, full6_sq1_6, full6_sq1_7, full6_sq1_8, full6_sq1_9,
        full6_sq1_10, full6_sq1_11, full6_sq1_12, full6_sq1_13, full6_sq1_14, full6_sq1_15,
        full6_sq2_0, full6_sq2_1, full6_sq2_2, full6_sq2_3, full6_sq2_4, full6_sq2_5, full6_sq2_6,
        full6_sq2_7, full6_sq2_8, full6_sq2_9, full6_sq2_10, full6_sq2_11, full6_sq2_12,
        full6_sq2_13, full6_sq2_14, full6_sq2_15, full6_mix_0, full6_mix_1, full6_mix_2,
        full6_mix_3, full6_mix_4, full6_mix_5, full6_mix_6, full6_mix_7, full6_mix_8, full6_mix_9,
        full6_mix_10, full6_mix_11, full6_mix_12, full6_mix_13, full6_mix_14, full6_mix_15,
        full7_sq1_0, full7_sq1_1, full7_sq1_2, full7_sq1_3, full7_sq1_4, full7_sq1_5, full7_sq1_6,
        full7_sq1_7, full7_sq1_8, full7_sq1_9, full7_sq1_10, full7_sq1_11, full7_sq1_12,
        full7_sq1_13, full7_sq1_14, full7_sq1_15, full7_sq2_0, full7_sq2_1, full7_sq2_2,
        full7_sq2_3, full7_sq2_4, full7_sq2_5, full7_sq2_6, full7_sq2_7, full7_sq2_8, full7_sq2_9,
        full7_sq2_10, full7_sq2_11, full7_sq2_12, full7_sq2_13, full7_sq2_14, full7_sq2_15,
        full7_mix_0, full7_mix_1, full7_mix_2, full7_mix_3, full7_mix_4, full7_mix_5, full7_mix_6,
        full7_mix_7, full7_mix_8, full7_mix_9, full7_mix_10, full7_mix_11, full7_mix_12,
        full7_mix_13, full7_mix_14, full7_mix_15,
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
            self.clk_prev,
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
}

impl Poseidon2Table {
    pub fn push_row(&mut self, row: &[u32; POSEIDON2_TRACE_COLUMNS]) {
        self.enabler.push(row[0]);
        self.state0.push(row[1]);
        self.state1.push(row[2]);
        self.state2.push(row[3]);
        self.state3.push(row[4]);
        self.state4.push(row[5]);
        self.state5.push(row[6]);
        self.state6.push(row[7]);
        self.state7.push(row[8]);
        self.state8.push(row[9]);
        self.state9.push(row[10]);
        self.state10.push(row[11]);
        self.state11.push(row[12]);
        self.state12.push(row[13]);
        self.state13.push(row[14]);
        self.state14.push(row[15]);
        self.state15.push(row[16]);
        self.full0_sq1_0.push(row[17]);
        self.full0_sq1_1.push(row[18]);
        self.full0_sq1_2.push(row[19]);
        self.full0_sq1_3.push(row[20]);
        self.full0_sq1_4.push(row[21]);
        self.full0_sq1_5.push(row[22]);
        self.full0_sq1_6.push(row[23]);
        self.full0_sq1_7.push(row[24]);
        self.full0_sq1_8.push(row[25]);
        self.full0_sq1_9.push(row[26]);
        self.full0_sq1_10.push(row[27]);
        self.full0_sq1_11.push(row[28]);
        self.full0_sq1_12.push(row[29]);
        self.full0_sq1_13.push(row[30]);
        self.full0_sq1_14.push(row[31]);
        self.full0_sq1_15.push(row[32]);
        self.full0_sq2_0.push(row[33]);
        self.full0_sq2_1.push(row[34]);
        self.full0_sq2_2.push(row[35]);
        self.full0_sq2_3.push(row[36]);
        self.full0_sq2_4.push(row[37]);
        self.full0_sq2_5.push(row[38]);
        self.full0_sq2_6.push(row[39]);
        self.full0_sq2_7.push(row[40]);
        self.full0_sq2_8.push(row[41]);
        self.full0_sq2_9.push(row[42]);
        self.full0_sq2_10.push(row[43]);
        self.full0_sq2_11.push(row[44]);
        self.full0_sq2_12.push(row[45]);
        self.full0_sq2_13.push(row[46]);
        self.full0_sq2_14.push(row[47]);
        self.full0_sq2_15.push(row[48]);
        self.full0_mix_0.push(row[49]);
        self.full0_mix_1.push(row[50]);
        self.full0_mix_2.push(row[51]);
        self.full0_mix_3.push(row[52]);
        self.full0_mix_4.push(row[53]);
        self.full0_mix_5.push(row[54]);
        self.full0_mix_6.push(row[55]);
        self.full0_mix_7.push(row[56]);
        self.full0_mix_8.push(row[57]);
        self.full0_mix_9.push(row[58]);
        self.full0_mix_10.push(row[59]);
        self.full0_mix_11.push(row[60]);
        self.full0_mix_12.push(row[61]);
        self.full0_mix_13.push(row[62]);
        self.full0_mix_14.push(row[63]);
        self.full0_mix_15.push(row[64]);
        self.full1_sq1_0.push(row[65]);
        self.full1_sq1_1.push(row[66]);
        self.full1_sq1_2.push(row[67]);
        self.full1_sq1_3.push(row[68]);
        self.full1_sq1_4.push(row[69]);
        self.full1_sq1_5.push(row[70]);
        self.full1_sq1_6.push(row[71]);
        self.full1_sq1_7.push(row[72]);
        self.full1_sq1_8.push(row[73]);
        self.full1_sq1_9.push(row[74]);
        self.full1_sq1_10.push(row[75]);
        self.full1_sq1_11.push(row[76]);
        self.full1_sq1_12.push(row[77]);
        self.full1_sq1_13.push(row[78]);
        self.full1_sq1_14.push(row[79]);
        self.full1_sq1_15.push(row[80]);
        self.full1_sq2_0.push(row[81]);
        self.full1_sq2_1.push(row[82]);
        self.full1_sq2_2.push(row[83]);
        self.full1_sq2_3.push(row[84]);
        self.full1_sq2_4.push(row[85]);
        self.full1_sq2_5.push(row[86]);
        self.full1_sq2_6.push(row[87]);
        self.full1_sq2_7.push(row[88]);
        self.full1_sq2_8.push(row[89]);
        self.full1_sq2_9.push(row[90]);
        self.full1_sq2_10.push(row[91]);
        self.full1_sq2_11.push(row[92]);
        self.full1_sq2_12.push(row[93]);
        self.full1_sq2_13.push(row[94]);
        self.full1_sq2_14.push(row[95]);
        self.full1_sq2_15.push(row[96]);
        self.full1_mix_0.push(row[97]);
        self.full1_mix_1.push(row[98]);
        self.full1_mix_2.push(row[99]);
        self.full1_mix_3.push(row[100]);
        self.full1_mix_4.push(row[101]);
        self.full1_mix_5.push(row[102]);
        self.full1_mix_6.push(row[103]);
        self.full1_mix_7.push(row[104]);
        self.full1_mix_8.push(row[105]);
        self.full1_mix_9.push(row[106]);
        self.full1_mix_10.push(row[107]);
        self.full1_mix_11.push(row[108]);
        self.full1_mix_12.push(row[109]);
        self.full1_mix_13.push(row[110]);
        self.full1_mix_14.push(row[111]);
        self.full1_mix_15.push(row[112]);
        self.full2_sq1_0.push(row[113]);
        self.full2_sq1_1.push(row[114]);
        self.full2_sq1_2.push(row[115]);
        self.full2_sq1_3.push(row[116]);
        self.full2_sq1_4.push(row[117]);
        self.full2_sq1_5.push(row[118]);
        self.full2_sq1_6.push(row[119]);
        self.full2_sq1_7.push(row[120]);
        self.full2_sq1_8.push(row[121]);
        self.full2_sq1_9.push(row[122]);
        self.full2_sq1_10.push(row[123]);
        self.full2_sq1_11.push(row[124]);
        self.full2_sq1_12.push(row[125]);
        self.full2_sq1_13.push(row[126]);
        self.full2_sq1_14.push(row[127]);
        self.full2_sq1_15.push(row[128]);
        self.full2_sq2_0.push(row[129]);
        self.full2_sq2_1.push(row[130]);
        self.full2_sq2_2.push(row[131]);
        self.full2_sq2_3.push(row[132]);
        self.full2_sq2_4.push(row[133]);
        self.full2_sq2_5.push(row[134]);
        self.full2_sq2_6.push(row[135]);
        self.full2_sq2_7.push(row[136]);
        self.full2_sq2_8.push(row[137]);
        self.full2_sq2_9.push(row[138]);
        self.full2_sq2_10.push(row[139]);
        self.full2_sq2_11.push(row[140]);
        self.full2_sq2_12.push(row[141]);
        self.full2_sq2_13.push(row[142]);
        self.full2_sq2_14.push(row[143]);
        self.full2_sq2_15.push(row[144]);
        self.full2_mix_0.push(row[145]);
        self.full2_mix_1.push(row[146]);
        self.full2_mix_2.push(row[147]);
        self.full2_mix_3.push(row[148]);
        self.full2_mix_4.push(row[149]);
        self.full2_mix_5.push(row[150]);
        self.full2_mix_6.push(row[151]);
        self.full2_mix_7.push(row[152]);
        self.full2_mix_8.push(row[153]);
        self.full2_mix_9.push(row[154]);
        self.full2_mix_10.push(row[155]);
        self.full2_mix_11.push(row[156]);
        self.full2_mix_12.push(row[157]);
        self.full2_mix_13.push(row[158]);
        self.full2_mix_14.push(row[159]);
        self.full2_mix_15.push(row[160]);
        self.full3_sq1_0.push(row[161]);
        self.full3_sq1_1.push(row[162]);
        self.full3_sq1_2.push(row[163]);
        self.full3_sq1_3.push(row[164]);
        self.full3_sq1_4.push(row[165]);
        self.full3_sq1_5.push(row[166]);
        self.full3_sq1_6.push(row[167]);
        self.full3_sq1_7.push(row[168]);
        self.full3_sq1_8.push(row[169]);
        self.full3_sq1_9.push(row[170]);
        self.full3_sq1_10.push(row[171]);
        self.full3_sq1_11.push(row[172]);
        self.full3_sq1_12.push(row[173]);
        self.full3_sq1_13.push(row[174]);
        self.full3_sq1_14.push(row[175]);
        self.full3_sq1_15.push(row[176]);
        self.full3_sq2_0.push(row[177]);
        self.full3_sq2_1.push(row[178]);
        self.full3_sq2_2.push(row[179]);
        self.full3_sq2_3.push(row[180]);
        self.full3_sq2_4.push(row[181]);
        self.full3_sq2_5.push(row[182]);
        self.full3_sq2_6.push(row[183]);
        self.full3_sq2_7.push(row[184]);
        self.full3_sq2_8.push(row[185]);
        self.full3_sq2_9.push(row[186]);
        self.full3_sq2_10.push(row[187]);
        self.full3_sq2_11.push(row[188]);
        self.full3_sq2_12.push(row[189]);
        self.full3_sq2_13.push(row[190]);
        self.full3_sq2_14.push(row[191]);
        self.full3_sq2_15.push(row[192]);
        self.full3_mix_0.push(row[193]);
        self.full3_mix_1.push(row[194]);
        self.full3_mix_2.push(row[195]);
        self.full3_mix_3.push(row[196]);
        self.full3_mix_4.push(row[197]);
        self.full3_mix_5.push(row[198]);
        self.full3_mix_6.push(row[199]);
        self.full3_mix_7.push(row[200]);
        self.full3_mix_8.push(row[201]);
        self.full3_mix_9.push(row[202]);
        self.full3_mix_10.push(row[203]);
        self.full3_mix_11.push(row[204]);
        self.full3_mix_12.push(row[205]);
        self.full3_mix_13.push(row[206]);
        self.full3_mix_14.push(row[207]);
        self.full3_mix_15.push(row[208]);
        self.partial0_sq1.push(row[209]);
        self.partial0_sq2.push(row[210]);
        self.partial0_mul.push(row[211]);
        self.partial1_sq1.push(row[212]);
        self.partial1_sq2.push(row[213]);
        self.partial1_mul.push(row[214]);
        self.partial2_sq1.push(row[215]);
        self.partial2_sq2.push(row[216]);
        self.partial2_mul.push(row[217]);
        self.partial3_sq1.push(row[218]);
        self.partial3_sq2.push(row[219]);
        self.partial3_mul.push(row[220]);
        self.partial4_sq1.push(row[221]);
        self.partial4_sq2.push(row[222]);
        self.partial4_mul.push(row[223]);
        self.partial5_sq1.push(row[224]);
        self.partial5_sq2.push(row[225]);
        self.partial5_mul.push(row[226]);
        self.partial6_sq1.push(row[227]);
        self.partial6_sq2.push(row[228]);
        self.partial6_mul.push(row[229]);
        self.partial7_sq1.push(row[230]);
        self.partial7_sq2.push(row[231]);
        self.partial7_mul.push(row[232]);
        self.partial8_sq1.push(row[233]);
        self.partial8_sq2.push(row[234]);
        self.partial8_mul.push(row[235]);
        self.partial9_sq1.push(row[236]);
        self.partial9_sq2.push(row[237]);
        self.partial9_mul.push(row[238]);
        self.partial10_sq1.push(row[239]);
        self.partial10_sq2.push(row[240]);
        self.partial10_mul.push(row[241]);
        self.partial11_sq1.push(row[242]);
        self.partial11_sq2.push(row[243]);
        self.partial11_mul.push(row[244]);
        self.partial12_sq1.push(row[245]);
        self.partial12_sq2.push(row[246]);
        self.partial12_mul.push(row[247]);
        self.partial13_sq1.push(row[248]);
        self.partial13_sq2.push(row[249]);
        self.partial13_mul.push(row[250]);
        self.full4_sq1_0.push(row[251]);
        self.full4_sq1_1.push(row[252]);
        self.full4_sq1_2.push(row[253]);
        self.full4_sq1_3.push(row[254]);
        self.full4_sq1_4.push(row[255]);
        self.full4_sq1_5.push(row[256]);
        self.full4_sq1_6.push(row[257]);
        self.full4_sq1_7.push(row[258]);
        self.full4_sq1_8.push(row[259]);
        self.full4_sq1_9.push(row[260]);
        self.full4_sq1_10.push(row[261]);
        self.full4_sq1_11.push(row[262]);
        self.full4_sq1_12.push(row[263]);
        self.full4_sq1_13.push(row[264]);
        self.full4_sq1_14.push(row[265]);
        self.full4_sq1_15.push(row[266]);
        self.full4_sq2_0.push(row[267]);
        self.full4_sq2_1.push(row[268]);
        self.full4_sq2_2.push(row[269]);
        self.full4_sq2_3.push(row[270]);
        self.full4_sq2_4.push(row[271]);
        self.full4_sq2_5.push(row[272]);
        self.full4_sq2_6.push(row[273]);
        self.full4_sq2_7.push(row[274]);
        self.full4_sq2_8.push(row[275]);
        self.full4_sq2_9.push(row[276]);
        self.full4_sq2_10.push(row[277]);
        self.full4_sq2_11.push(row[278]);
        self.full4_sq2_12.push(row[279]);
        self.full4_sq2_13.push(row[280]);
        self.full4_sq2_14.push(row[281]);
        self.full4_sq2_15.push(row[282]);
        self.full4_mix_0.push(row[283]);
        self.full4_mix_1.push(row[284]);
        self.full4_mix_2.push(row[285]);
        self.full4_mix_3.push(row[286]);
        self.full4_mix_4.push(row[287]);
        self.full4_mix_5.push(row[288]);
        self.full4_mix_6.push(row[289]);
        self.full4_mix_7.push(row[290]);
        self.full4_mix_8.push(row[291]);
        self.full4_mix_9.push(row[292]);
        self.full4_mix_10.push(row[293]);
        self.full4_mix_11.push(row[294]);
        self.full4_mix_12.push(row[295]);
        self.full4_mix_13.push(row[296]);
        self.full4_mix_14.push(row[297]);
        self.full4_mix_15.push(row[298]);
        self.full5_sq1_0.push(row[299]);
        self.full5_sq1_1.push(row[300]);
        self.full5_sq1_2.push(row[301]);
        self.full5_sq1_3.push(row[302]);
        self.full5_sq1_4.push(row[303]);
        self.full5_sq1_5.push(row[304]);
        self.full5_sq1_6.push(row[305]);
        self.full5_sq1_7.push(row[306]);
        self.full5_sq1_8.push(row[307]);
        self.full5_sq1_9.push(row[308]);
        self.full5_sq1_10.push(row[309]);
        self.full5_sq1_11.push(row[310]);
        self.full5_sq1_12.push(row[311]);
        self.full5_sq1_13.push(row[312]);
        self.full5_sq1_14.push(row[313]);
        self.full5_sq1_15.push(row[314]);
        self.full5_sq2_0.push(row[315]);
        self.full5_sq2_1.push(row[316]);
        self.full5_sq2_2.push(row[317]);
        self.full5_sq2_3.push(row[318]);
        self.full5_sq2_4.push(row[319]);
        self.full5_sq2_5.push(row[320]);
        self.full5_sq2_6.push(row[321]);
        self.full5_sq2_7.push(row[322]);
        self.full5_sq2_8.push(row[323]);
        self.full5_sq2_9.push(row[324]);
        self.full5_sq2_10.push(row[325]);
        self.full5_sq2_11.push(row[326]);
        self.full5_sq2_12.push(row[327]);
        self.full5_sq2_13.push(row[328]);
        self.full5_sq2_14.push(row[329]);
        self.full5_sq2_15.push(row[330]);
        self.full5_mix_0.push(row[331]);
        self.full5_mix_1.push(row[332]);
        self.full5_mix_2.push(row[333]);
        self.full5_mix_3.push(row[334]);
        self.full5_mix_4.push(row[335]);
        self.full5_mix_5.push(row[336]);
        self.full5_mix_6.push(row[337]);
        self.full5_mix_7.push(row[338]);
        self.full5_mix_8.push(row[339]);
        self.full5_mix_9.push(row[340]);
        self.full5_mix_10.push(row[341]);
        self.full5_mix_11.push(row[342]);
        self.full5_mix_12.push(row[343]);
        self.full5_mix_13.push(row[344]);
        self.full5_mix_14.push(row[345]);
        self.full5_mix_15.push(row[346]);
        self.full6_sq1_0.push(row[347]);
        self.full6_sq1_1.push(row[348]);
        self.full6_sq1_2.push(row[349]);
        self.full6_sq1_3.push(row[350]);
        self.full6_sq1_4.push(row[351]);
        self.full6_sq1_5.push(row[352]);
        self.full6_sq1_6.push(row[353]);
        self.full6_sq1_7.push(row[354]);
        self.full6_sq1_8.push(row[355]);
        self.full6_sq1_9.push(row[356]);
        self.full6_sq1_10.push(row[357]);
        self.full6_sq1_11.push(row[358]);
        self.full6_sq1_12.push(row[359]);
        self.full6_sq1_13.push(row[360]);
        self.full6_sq1_14.push(row[361]);
        self.full6_sq1_15.push(row[362]);
        self.full6_sq2_0.push(row[363]);
        self.full6_sq2_1.push(row[364]);
        self.full6_sq2_2.push(row[365]);
        self.full6_sq2_3.push(row[366]);
        self.full6_sq2_4.push(row[367]);
        self.full6_sq2_5.push(row[368]);
        self.full6_sq2_6.push(row[369]);
        self.full6_sq2_7.push(row[370]);
        self.full6_sq2_8.push(row[371]);
        self.full6_sq2_9.push(row[372]);
        self.full6_sq2_10.push(row[373]);
        self.full6_sq2_11.push(row[374]);
        self.full6_sq2_12.push(row[375]);
        self.full6_sq2_13.push(row[376]);
        self.full6_sq2_14.push(row[377]);
        self.full6_sq2_15.push(row[378]);
        self.full6_mix_0.push(row[379]);
        self.full6_mix_1.push(row[380]);
        self.full6_mix_2.push(row[381]);
        self.full6_mix_3.push(row[382]);
        self.full6_mix_4.push(row[383]);
        self.full6_mix_5.push(row[384]);
        self.full6_mix_6.push(row[385]);
        self.full6_mix_7.push(row[386]);
        self.full6_mix_8.push(row[387]);
        self.full6_mix_9.push(row[388]);
        self.full6_mix_10.push(row[389]);
        self.full6_mix_11.push(row[390]);
        self.full6_mix_12.push(row[391]);
        self.full6_mix_13.push(row[392]);
        self.full6_mix_14.push(row[393]);
        self.full6_mix_15.push(row[394]);
        self.full7_sq1_0.push(row[395]);
        self.full7_sq1_1.push(row[396]);
        self.full7_sq1_2.push(row[397]);
        self.full7_sq1_3.push(row[398]);
        self.full7_sq1_4.push(row[399]);
        self.full7_sq1_5.push(row[400]);
        self.full7_sq1_6.push(row[401]);
        self.full7_sq1_7.push(row[402]);
        self.full7_sq1_8.push(row[403]);
        self.full7_sq1_9.push(row[404]);
        self.full7_sq1_10.push(row[405]);
        self.full7_sq1_11.push(row[406]);
        self.full7_sq1_12.push(row[407]);
        self.full7_sq1_13.push(row[408]);
        self.full7_sq1_14.push(row[409]);
        self.full7_sq1_15.push(row[410]);
        self.full7_sq2_0.push(row[411]);
        self.full7_sq2_1.push(row[412]);
        self.full7_sq2_2.push(row[413]);
        self.full7_sq2_3.push(row[414]);
        self.full7_sq2_4.push(row[415]);
        self.full7_sq2_5.push(row[416]);
        self.full7_sq2_6.push(row[417]);
        self.full7_sq2_7.push(row[418]);
        self.full7_sq2_8.push(row[419]);
        self.full7_sq2_9.push(row[420]);
        self.full7_sq2_10.push(row[421]);
        self.full7_sq2_11.push(row[422]);
        self.full7_sq2_12.push(row[423]);
        self.full7_sq2_13.push(row[424]);
        self.full7_sq2_14.push(row[425]);
        self.full7_sq2_15.push(row[426]);
        self.full7_mix_0.push(row[427]);
        self.full7_mix_1.push(row[428]);
        self.full7_mix_2.push(row[429]);
        self.full7_mix_3.push(row[430]);
        self.full7_mix_4.push(row[431]);
        self.full7_mix_5.push(row[432]);
        self.full7_mix_6.push(row[433]);
        self.full7_mix_7.push(row[434]);
        self.full7_mix_8.push(row[435]);
        self.full7_mix_9.push(row[436]);
        self.full7_mix_10.push(row[437]);
        self.full7_mix_11.push(row[438]);
        self.full7_mix_12.push(row[439]);
        self.full7_mix_13.push(row[440]);
        self.full7_mix_14.push(row[441]);
        self.full7_mix_15.push(row[442]);
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

        self.mem_initial.entry(aligned_addr).or_insert(prev);

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
    use crate::commitment::RW_MEMORY_BASE;

    const MEM_ADDR: u32 = RW_MEMORY_BASE + 0x100;

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
        assert!(tracer.mem_initial.is_empty());
        assert!(tracer.program_reads.is_empty());
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

        let access = tracer.trace_mem_access(MEM_ADDR, 0x42, 0x42);

        assert_eq!(access.addr, MEM_ADDR);
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
        tracer.trace_mem_access(MEM_ADDR, 0x11, 0x11);

        tracer.clk = 2;
        let access = tracer.trace_mem_access(MEM_ADDR, 0x11, 0x22);

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
        tracer.trace_mem_access(MEM_ADDR, 0x42, 0x42);

        tracer.clk = 350;
        let access = tracer.trace_mem_access(MEM_ADDR, 0x42, 0x42);

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
        tracer.trace_mem_access(MEM_ADDR, 0, 0);

        tracer.clk = 100;
        let access = tracer.trace_mem_access(MEM_ADDR, 0, 0);

        // Exactly at max_clock_diff - no intermediate needed
        assert!(tracer.mem_clk_update.is_empty());
        assert_eq!(access.clk_prev, 0);
        // Note: access.clk is no longer stored; current clk is tracer.clk=100
    }

    #[test]
    fn test_trace_mem_access_preserves_value() {
        let mut tracer = Tracer::with_max_clock_diff(50);

        tracer.clk = 0;
        tracer.trace_mem_access(MEM_ADDR, 0xAB, 0xAB);

        tracer.clk = 200;
        let access = tracer.trace_mem_access(MEM_ADDR, 0xAB, 0xAB);

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

        tracer.trace_mem_access(MEM_ADDR, 0, 0);

        assert_eq!(tracer.mem_clk.get(&MEM_ADDR), Some(&10));
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
        tracer.trace_mem_access(MEM_ADDR, 0, 0);

        tracer.clk = 5;
        let access = tracer.trace_mem_access(MEM_ADDR, 0, 0);

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
        tracer.trace_mem_access(MEM_ADDR, 0, 0);

        tracer.clk = u32::MAX - 1;
        tracer.trace_mem_access(MEM_ADDR, 0, 0);

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
                clk_prev: 0,
                next: 10,
            };
            let rs1 = Access {
                addr: 2,
                prev: 5,
                clk_prev: 1,
                next: 5,
            };
            let rs2 = Access {
                addr: 3,
                prev: 7,
                clk_prev: 2,
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
                clk_prev: 0,
                next: 0x12345000,
            };

            table.push(1, 0x1000, rd, 0x12, 0x34, 0x50);

            let output = table.to_table().to_string();

            // Check enabler column exists
            assert!(output.contains("enabler"));
        }

        #[test]
        fn test_empty_table_to_table() {
            let table = BaseAluRegTable::new();
            let output = table.to_table().to_string();

            // Empty table should still have headers
            assert!(output.contains("clk"));
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
