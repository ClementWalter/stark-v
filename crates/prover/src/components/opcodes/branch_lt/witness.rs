//! Witness generation for branch_lt component.

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

use super::columns::BranchLtColumns;
use crate::{combine, write_pair};

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

    let cols = BranchLtColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clk.len();

    let log_size = trace[0].domain.log_size();
    let mut logup_gen = LogupTraceGenerator::new(log_size);

    // Constants
    let zero = PackedM31::zero();
    let one = PackedM31::broadcast(BaseField::one());
    let pow2_7 = PackedM31::broadcast(BaseField::from_u32_unchecked(128));

    let opcode_blt = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Blt as u32));
    let opcode_bltu = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Bltu as u32));
    let opcode_bge = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Bge as u32));
    let opcode_bgeu = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Bgeu as u32));

    let zero_col: Vec<PackedM31> = vec![zero; simd_size];

    // Compute derived columns
    let enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.opcode_blt_flag[i]
                + cols.opcode_bltu_flag[i]
                + cols.opcode_bge_flag[i]
                + cols.opcode_bgeu_flag[i]
        })
        .collect();

    let expected_opcode_id: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.opcode_blt_flag[i] * opcode_blt
                + cols.opcode_bltu_flag[i] * opcode_bltu
                + cols.opcode_bge_flag[i] * opcode_bge
                + cols.opcode_bgeu_flag[i] * opcode_bgeu
        })
        .collect();

    let signed: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.opcode_blt_flag[i] + cols.opcode_bge_flag[i])
        .collect();

    // msl_felt + signed * 2^7
    let rs1_msl_adjusted: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.rs1_msl_felt[i] + signed[i] * pow2_7)
        .collect();
    let rs2_msl_adjusted: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.rs2_msl_felt[i] + signed[i] * pow2_7)
        .collect();

    let clk_plus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.clk[i] + one).collect();
    let clk_minus_rs1_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rs1_clk_prev[i])
        .collect();
    let clk_minus_rs2_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rs2_clk_prev[i])
        .collect();
    let diff_val_minus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.diff_val[i] - one).collect();

    // prefix_sum = sum of diff_markers
    let prefix_sum: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.diff_marker_0[i]
                + cols.diff_marker_1[i]
                + cols.diff_marker_2[i]
                + cols.diff_marker_3[i]
        })
        .collect();

    // Numerators
    let neg_enabler: Vec<PackedQM31> = enabler.iter().map(|&e| -PackedQM31::from(e)).collect();
    let pos_enabler: Vec<PackedQM31> = enabler.iter().map(|&e| PackedQM31::from(e)).collect();
    let neg_one = vec![-PackedQM31::one(); simd_size];

    // =====================================================================
    // LogUp entries (same order as AIR)
    // =====================================================================

    // 1. program_access: -enabler * (pc, expected_opcode_id, rs1_addr, rs2_addr, imm_felt)
    let program_denom = combine!(
        relations.program_access,
        [
            cols.pc,
            &expected_opcode_id,
            cols.rs1_addr,
            cols.rs2_addr,
            cols.imm_felt
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

    // 3. registers_state: +enabler * (branch_target, clk + 1)
    let registers_write_denom =
        combine!(relations.registers_state, [cols.branch_target, &clk_plus_1]);

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
        &neg_one,
        &rc_20_rs1_denom,
        logup_gen
    );

    // 7. memory_access: -enabler * (0, rs2_addr, rs2_clk_prev, rs2_prev_0..3)
    let rs2_read_denom = combine!(
        relations.memory_access,
        [
            &zero_col,
            cols.rs2_addr,
            cols.rs2_clk_prev,
            cols.rs2_prev_0,
            cols.rs2_prev_1,
            cols.rs2_prev_2,
            cols.rs2_prev_3
        ]
    );

    // 8. memory_access: +enabler * (0, rs2_addr, clk, rs2_next_0..3)
    let rs2_write_denom = combine!(
        relations.memory_access,
        [
            &zero_col,
            cols.rs2_addr,
            cols.clk,
            cols.rs2_next_0,
            cols.rs2_next_1,
            cols.rs2_next_2,
            cols.rs2_next_3
        ]
    );

    write_pair!(
        &neg_enabler,
        &rs2_read_denom,
        &pos_enabler,
        &rs2_write_denom,
        logup_gen
    );

    // 9. range_check_20: -1 * (clk - rs2_clk_prev)
    let rc_20_rs2_denom = combine!(relations.range_check_20, [&clk_minus_rs2_clk_prev]);

    // 10. range_check_8_8: -1 * (rs1_msl_adjusted, rs2_msl_adjusted)
    let rc_8_8_msl_denom = combine!(
        relations.range_check_8_8,
        [&rs1_msl_adjusted, &rs2_msl_adjusted]
    );

    write_pair!(
        &neg_one,
        &rc_20_rs2_denom,
        &neg_one,
        &rc_8_8_msl_denom,
        logup_gen
    );

    // 11. range_check_20: -prefix_sum * (diff_val - 1)
    let rc_20_diff_denom = combine!(relations.range_check_20, [&diff_val_minus_1]);
    let neg_prefix_sum: Vec<PackedQM31> =
        prefix_sum.iter().map(|&p| -PackedQM31::from(p)).collect();

    crate::write_col!(&neg_prefix_sum, &rc_20_diff_denom, logup_gen);

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

    let cols = BranchLtColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clk.len();

    let one = PackedM31::broadcast(BaseField::one());
    let pow2_7 = PackedM31::broadcast(BaseField::from_u32_unchecked(128));

    // Numerator: enabler (sum of opcode flags)
    let enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.opcode_blt_flag[i]
                + cols.opcode_bltu_flag[i]
                + cols.opcode_bge_flag[i]
                + cols.opcode_bgeu_flag[i]
        })
        .collect();

    let clk_minus_rs1_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rs1_clk_prev[i])
        .collect();
    let clk_minus_rs2_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rs2_clk_prev[i])
        .collect();
    let diff_val_minus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.diff_val[i] - one).collect();

    // prefix_sum = sum of diff_markers
    let prefix_sum: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.diff_marker_0[i]
                + cols.diff_marker_1[i]
                + cols.diff_marker_2[i]
                + cols.diff_marker_3[i]
        })
        .collect();

    // signed = opcode_blt_flag + opcode_bge_flag
    let signed: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.opcode_blt_flag[i] + cols.opcode_bge_flag[i])
        .collect();

    // rs1_msl_adjusted = rs1_msl_felt + signed * 128
    // rs2_msl_adjusted = rs2_msl_felt + signed * 128
    let rs1_msl_adjusted: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.rs1_msl_felt[i] + signed[i] * pow2_7)
        .collect();
    let rs2_msl_adjusted: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.rs2_msl_felt[i] + signed[i] * pow2_7)
        .collect();

    // Register range_check_20: (clk - rs1_clk_prev)
    counters
        .range_check_20
        .register_many(&enabler, &[&clk_minus_rs1_clk_prev]);

    // Register range_check_20: (clk - rs2_clk_prev)
    counters
        .range_check_20
        .register_many(&enabler, &[&clk_minus_rs2_clk_prev]);

    // Register range_check_8_8: (rs1_msl_adjusted, rs2_msl_adjusted)
    counters
        .range_check_8_8
        .register_many(&enabler, &[&rs1_msl_adjusted, &rs2_msl_adjusted]);

    // Register range_check_20: (diff_val - 1) with multiplicity prefix_sum
    counters
        .range_check_20
        .register_many(&prefix_sum, &[&diff_val_minus_1]);
}
