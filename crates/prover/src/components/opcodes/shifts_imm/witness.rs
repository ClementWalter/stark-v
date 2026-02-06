//! Witness generation for shifts_imm component.

use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::ColumnVec;
use stwo::core::fields::m31::BaseField;
use stwo::core::fields::qm31::QM31;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::backend::simd::m31::PackedM31;
use stwo::prover::backend::simd::qm31::PackedQM31;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;
use stwo_constraint_framework::LogupTraceGenerator;

use super::columns::ShiftsImmColumns;

/// Generate interaction trace for LogUp.
pub fn gen_interaction_trace(
    trace: &[CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>],
    relations: &crate::relations::Relations,
) -> (
    ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    QM31,
) {
    if trace.is_empty() {
        return (vec![], QM31::zero());
    }

    let cols = ShiftsImmColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clk.len();

    let log_size = trace[0].domain.log_size();
    let mut logup_gen = LogupTraceGenerator::new(log_size);

    // Constants
    let zero = PackedM31::zero();
    let one = PackedM31::broadcast(BaseField::one());
    let four = PackedM31::broadcast(BaseField::from_u32_unchecked(4));

    let opcode_slli = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Slli as u32));
    let opcode_srli = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Srli as u32));
    let opcode_srai = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Srai as u32));

    let zero_col: Vec<PackedM31> = vec![zero; simd_size];

    // Compute derived columns
    let enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.opcode_sll_flag[i] + cols.opcode_srl_flag[i] + cols.opcode_sra_flag[i])
        .collect();

    let expected_opcode_id: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.opcode_sll_flag[i] * opcode_slli
                + cols.opcode_srl_flag[i] * opcode_srli
                + cols.opcode_sra_flag[i] * opcode_srai
        })
        .collect();

    let pc_plus_4: Vec<PackedM31> = (0..simd_size).map(|i| cols.pc[i] + four).collect();
    let clk_plus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.clk[i] + one).collect();
    let clk_minus_rs1_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rs1_clk_prev[i])
        .collect();
    let clk_minus_rd_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rd_clk_prev[i])
        .collect();

    // Compute bit_multiplier for shift carry range checks
    let bit_multiplier: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            let mut mult = PackedM31::zero();
            mult +=
                cols.bit_shift_marker_0[i] * PackedM31::broadcast(BaseField::from_u32_unchecked(1));
            mult +=
                cols.bit_shift_marker_1[i] * PackedM31::broadcast(BaseField::from_u32_unchecked(2));
            mult +=
                cols.bit_shift_marker_2[i] * PackedM31::broadcast(BaseField::from_u32_unchecked(4));
            mult +=
                cols.bit_shift_marker_3[i] * PackedM31::broadcast(BaseField::from_u32_unchecked(8));
            mult += cols.bit_shift_marker_4[i]
                * PackedM31::broadcast(BaseField::from_u32_unchecked(16));
            mult += cols.bit_shift_marker_5[i]
                * PackedM31::broadcast(BaseField::from_u32_unchecked(32));
            mult += cols.bit_shift_marker_6[i]
                * PackedM31::broadcast(BaseField::from_u32_unchecked(64));
            mult += cols.bit_shift_marker_7[i]
                * PackedM31::broadcast(BaseField::from_u32_unchecked(128));
            mult
        })
        .collect();

    // Shift carry range check values: bit_multiplier - enabler - bit_shift_carry[i]
    let carry_rc_0: Vec<PackedM31> = (0..simd_size)
        .map(|i| bit_multiplier[i] - enabler[i] - cols.bit_shift_carry_0[i])
        .collect();
    let carry_rc_1: Vec<PackedM31> = (0..simd_size)
        .map(|i| bit_multiplier[i] - enabler[i] - cols.bit_shift_carry_1[i])
        .collect();
    let carry_rc_2: Vec<PackedM31> = (0..simd_size)
        .map(|i| bit_multiplier[i] - enabler[i] - cols.bit_shift_carry_2[i])
        .collect();
    let carry_rc_3: Vec<PackedM31> = (0..simd_size)
        .map(|i| bit_multiplier[i] - enabler[i] - cols.bit_shift_carry_3[i])
        .collect();

    // Numerators
    let neg_enabler: Vec<PackedQM31> = enabler.iter().map(|&e| -PackedQM31::from(e)).collect();
    let pos_enabler: Vec<PackedQM31> = enabler.iter().map(|&e| PackedQM31::from(e)).collect();

    // =====================================================================
    // LogUp entries (same order as AIR)
    // =====================================================================

    // 1. program_access: -enabler * (pc, expected_opcode_id, rd_addr, rs1_addr, imm_truncated)
    let program_denom = combine!(
        relations.program_access,
        [
            cols.pc,
            &expected_opcode_id,
            cols.rd_addr,
            cols.rs1_addr,
            cols.imm_truncated
        ]
    );

    // 2. registers_state: -enabler * (pc, clk)
    let registers_read_denom = combine!(relations.registers_state, [cols.pc, cols.clk]);

    write_pair!(
        &neg_enabler,
        &program_denom,
        &neg_enabler,
        &registers_read_denom,
        logup_gen
    );

    // 3. registers_state: +enabler * (pc + 4, clk + 1)
    let registers_write_denom = combine!(relations.registers_state, [&pc_plus_4, &clk_plus_1]);

    // 4. memory_access: -enabler * (0, rs1_addr, rs1_clk_prev, rs1_prev_0..3)
    let rs1_read_denom = combine!(
        relations.memory_access,
        [
            &zero_col,
            cols.rs1_addr,
            cols.rs1_clk_prev,
            cols.rs1_prev_0,
            cols.rs1_prev_1,
            cols.rs1_prev_2,
            cols.rs1_prev_3
        ]
    );

    write_pair!(
        &pos_enabler,
        &registers_write_denom,
        &neg_enabler,
        &rs1_read_denom,
        logup_gen
    );

    // 5. memory_access: +enabler * (0, rs1_addr, clk, rs1_next_0..3)
    let rs1_write_denom = combine!(
        relations.memory_access,
        [
            &zero_col,
            cols.rs1_addr,
            cols.clk,
            cols.rs1_next_0,
            cols.rs1_next_1,
            cols.rs1_next_2,
            cols.rs1_next_3
        ]
    );

    // 6. range_check_20: -1 * (clk - rs1_clk_prev)
    let rc_20_rs1_denom = combine!(relations.range_check_20, [&clk_minus_rs1_clk_prev]);

    write_pair!(
        &pos_enabler,
        &rs1_write_denom,
        &neg_enabler,
        &rc_20_rs1_denom,
        logup_gen
    );

    // 7. range_check_8_8: -1 * (bit_multiplier - enabler - carry[0], bit_multiplier - enabler - carry[1])
    let rc_8_8_carry_0_denom = combine!(relations.range_check_8_8, [&carry_rc_0, &carry_rc_1]);

    // 8. range_check_8_8: -1 * (bit_multiplier - enabler - carry[2], bit_multiplier - enabler - carry[3])
    let rc_8_8_carry_1_denom = combine!(relations.range_check_8_8, [&carry_rc_2, &carry_rc_3]);

    write_pair!(
        &neg_enabler,
        &rc_8_8_carry_0_denom,
        &neg_enabler,
        &rc_8_8_carry_1_denom,
        logup_gen
    );

    // 9. range_check_8_8: -1 * (rd[0], rd[1])
    let rc_8_8_0_denom = combine!(relations.range_check_8_8, [cols.rd_next_0, cols.rd_next_1]);

    // 10. range_check_8_8: -1 * (rd[2], rd[3])
    let rc_8_8_1_denom = combine!(relations.range_check_8_8, [cols.rd_next_2, cols.rd_next_3]);

    write_pair!(
        &neg_enabler,
        &rc_8_8_0_denom,
        &neg_enabler,
        &rc_8_8_1_denom,
        logup_gen
    );

    // 11. memory_access: -enabler * (0, rd_addr, rd_clk_prev, rd_prev_0..3)
    let rd_read_denom = combine!(
        relations.memory_access,
        [
            &zero_col,
            cols.rd_addr,
            cols.rd_clk_prev,
            cols.rd_prev_0,
            cols.rd_prev_1,
            cols.rd_prev_2,
            cols.rd_prev_3
        ]
    );

    // 12. memory_access: +enabler * (0, rd_addr, clk, rd_next_0..3)
    let rd_write_denom = combine!(
        relations.memory_access,
        [
            &zero_col,
            cols.rd_addr,
            cols.clk,
            cols.rd_next_0,
            cols.rd_next_1,
            cols.rd_next_2,
            cols.rd_next_3
        ]
    );

    write_pair!(
        &neg_enabler,
        &rd_read_denom,
        &pos_enabler,
        &rd_write_denom,
        logup_gen
    );

    // 13. range_check_20: -1 * (clk - rd_clk_prev)
    let rc_20_rd_denom = combine!(relations.range_check_20, [&clk_minus_rd_clk_prev]);

    write_col!(&neg_enabler, &rc_20_rd_denom, logup_gen);

    logup_gen.finalize_last()
}

/// Register multiplicities for preprocessed lookups.
/// Uses the same column access pattern as gen_interaction_trace.
pub fn register_multiplicities(
    trace: &[CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>],
    counters: &mut crate::relations::Counters,
) {
    if trace.is_empty() {
        return;
    }

    let cols = ShiftsImmColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clk.len();

    // Numerator: negated enabler (to match gen_interaction_trace)
    let neg_enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| -(cols.opcode_sll_flag[i] + cols.opcode_srl_flag[i] + cols.opcode_sra_flag[i]))
        .collect();

    let clk_minus_rs1_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rs1_clk_prev[i])
        .collect();
    let clk_minus_rd_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rd_clk_prev[i])
        .collect();

    // Compute bit_multiplier for shift carry range checks
    let bit_multiplier: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.bit_shift_marker_0[i]
                + cols.bit_shift_marker_1[i]
                    * PackedM31::broadcast(BaseField::from_u32_unchecked(2))
                + cols.bit_shift_marker_2[i]
                    * PackedM31::broadcast(BaseField::from_u32_unchecked(4))
                + cols.bit_shift_marker_3[i]
                    * PackedM31::broadcast(BaseField::from_u32_unchecked(8))
                + cols.bit_shift_marker_4[i]
                    * PackedM31::broadcast(BaseField::from_u32_unchecked(16))
                + cols.bit_shift_marker_5[i]
                    * PackedM31::broadcast(BaseField::from_u32_unchecked(32))
                + cols.bit_shift_marker_6[i]
                    * PackedM31::broadcast(BaseField::from_u32_unchecked(64))
                + cols.bit_shift_marker_7[i]
                    * PackedM31::broadcast(BaseField::from_u32_unchecked(128))
        })
        .collect();

    let enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.opcode_sll_flag[i] + cols.opcode_srl_flag[i] + cols.opcode_sra_flag[i])
        .collect();

    // Shift carry range check values: bit_multiplier - enabler - bit_shift_carry[i]
    let carry_rc_0: Vec<PackedM31> = (0..simd_size)
        .map(|i| bit_multiplier[i] - enabler[i] - cols.bit_shift_carry_0[i])
        .collect();
    let carry_rc_1: Vec<PackedM31> = (0..simd_size)
        .map(|i| bit_multiplier[i] - enabler[i] - cols.bit_shift_carry_1[i])
        .collect();
    let carry_rc_2: Vec<PackedM31> = (0..simd_size)
        .map(|i| bit_multiplier[i] - enabler[i] - cols.bit_shift_carry_2[i])
        .collect();
    let carry_rc_3: Vec<PackedM31> = (0..simd_size)
        .map(|i| bit_multiplier[i] - enabler[i] - cols.bit_shift_carry_3[i])
        .collect();

    // Register range_check_20 for rs1 clock diff with negated multiplicity
    counters
        .range_check_20
        .register_many(&neg_enabler, &[&clk_minus_rs1_clk_prev]);

    // Register range_check_8_8 for shift carries with negated multiplicity
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[&carry_rc_0, &carry_rc_1]);
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[&carry_rc_2, &carry_rc_3]);

    // Register range_check_8_8 for rd limbs with negated multiplicity
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[cols.rd_next_0, cols.rd_next_1]);
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[cols.rd_next_2, cols.rd_next_3]);

    // Register range_check_20 for rd clock diff with negated multiplicity
    counters
        .range_check_20
        .register_many(&neg_enabler, &[&clk_minus_rd_clk_prev]);
}
