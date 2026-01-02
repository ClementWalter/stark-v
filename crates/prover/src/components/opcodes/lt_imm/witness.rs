//! Witness generation for lt_imm component.

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

use super::columns::LtImmColumns;
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

    let cols = LtImmColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clk.len();

    let log_size = trace[0].domain.log_size();
    let mut logup_gen = LogupTraceGenerator::new(log_size);

    // Constants
    let zero = PackedM31::zero();
    let one = PackedM31::broadcast(BaseField::one());
    let two = PackedM31::broadcast(BaseField::from_u32_unchecked(2));
    let four = PackedM31::broadcast(BaseField::from_u32_unchecked(4));
    let pow2_7 = PackedM31::broadcast(BaseField::from_u32_unchecked(128));
    let pow2_8 = PackedM31::broadcast(BaseField::from_u32_unchecked(256));
    let pow2_11 = PackedM31::broadcast(BaseField::from_u32_unchecked(2048));

    let opcode_slti = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Slti as u32));
    let opcode_sltiu = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Sltiu as u32));

    let zero_col: Vec<PackedM31> = vec![zero; simd_size];

    // Compute derived columns
    let enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.opcode_slti_flag[i] + cols.opcode_sltiu_flag[i])
        .collect();

    let expected_opcode_id: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.opcode_slti_flag[i] * opcode_slti + cols.opcode_sltiu_flag[i] * opcode_sltiu)
        .collect();

    let imm: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.imm_0[i] + pow2_8 * cols.imm_1[i] + pow2_11 * cols.imm_msb[i])
        .collect();

    let prefix_sum_final: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.diff_marker_0[i]
                + cols.diff_marker_1[i]
                + cols.diff_marker_2[i]
                + cols.diff_marker_3[i]
        })
        .collect();

    // rs1_msl_felt + opcode_slti_flag * 2^7
    let rs1_msl_adjusted: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.rs1_msl_felt[i] + cols.opcode_slti_flag[i] * pow2_7)
        .collect();

    // 2 * imm_1
    let imm_1_times_2: Vec<PackedM31> = (0..simd_size).map(|i| two * cols.imm_1[i]).collect();

    let pc_plus_4: Vec<PackedM31> = (0..simd_size).map(|i| cols.pc[i] + four).collect();
    let clk_plus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.clk[i] + one).collect();
    let clk_minus_rs1_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rs1_clk_prev[i])
        .collect();
    let diff_val_minus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.diff_val[i] - one).collect();
    let clk_minus_rd_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rd_clk_prev[i])
        .collect();

    // Numerators
    let neg_enabler: Vec<PackedQM31> = enabler.iter().map(|&e| -PackedQM31::from(e)).collect();
    let pos_enabler: Vec<PackedQM31> = enabler.iter().map(|&e| PackedQM31::from(e)).collect();
    let neg_prefix_sum: Vec<PackedQM31> = prefix_sum_final
        .iter()
        .map(|&p| -PackedQM31::from(p))
        .collect();

    // =====================================================================
    // LogUp entries (same order as AIR)
    // =====================================================================

    // 1. program_access: -enabler * (pc, expected_opcode_id, rd_addr, rs1_addr, imm)
    let program_denom = combine!(
        relations.program_access,
        [
            cols.pc,
            &expected_opcode_id,
            cols.rd_addr,
            cols.rs1_addr,
            &imm
        ]
    );

    // 2. range_check_8_8_4: -1 * (rs1_msl_adjusted, imm_0, 2*imm_1)
    let rc_8_8_4_denom = combine!(
        relations.range_check_8_8_4,
        [&rs1_msl_adjusted, cols.imm_0, &imm_1_times_2]
    );

    write_pair!(
        &neg_enabler,
        &program_denom,
        &neg_enabler,
        &rc_8_8_4_denom,
        logup_gen
    );

    // 3. registers_state: -enabler * (pc, clk)
    let registers_read_denom = combine!(relations.registers_state, [cols.pc, cols.clk]);

    // 4. registers_state: +enabler * (pc + 4, clk + 1)
    let registers_write_denom = combine!(relations.registers_state, [&pc_plus_4, &clk_plus_1]);

    write_pair!(
        &neg_enabler,
        &registers_read_denom,
        &pos_enabler,
        &registers_write_denom,
        logup_gen
    );

    // 5. memory_access: -enabler * (0, rs1_addr, rs1_clk_prev, rs1_prev_0..3)
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

    // 6. memory_access: +enabler * (0, rs1_addr, clk, rs1_next_0..3)
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

    write_pair!(
        &neg_enabler,
        &rs1_read_denom,
        &pos_enabler,
        &rs1_write_denom,
        logup_gen
    );

    // 7. range_check_20: -1 * (clk - rs1_clk_prev)
    let rc_20_rs1_denom = combine!(relations.range_check_20, [&clk_minus_rs1_clk_prev]);

    // 8. range_check_20: -prefix_sum * (diff_val - 1)
    let rc_20_diff_denom = combine!(relations.range_check_20, [&diff_val_minus_1]);

    write_pair!(
        &neg_enabler,
        &rc_20_rs1_denom,
        &neg_prefix_sum,
        &rc_20_diff_denom,
        logup_gen
    );

    // 9. memory_access: -enabler * (0, rd_addr, rd_clk_prev, rd_prev_0..3)
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

    // 10. memory_access: +enabler * (0, rd_addr, clk, cmp_result, 0, 0, 0)
    let rd_write_denom = combine!(
        relations.memory_access,
        [
            &zero_col,
            cols.rd_addr,
            cols.clk,
            cols.cmp_result,
            &zero_col,
            &zero_col,
            &zero_col
        ]
    );

    write_pair!(
        &neg_enabler,
        &rd_read_denom,
        &pos_enabler,
        &rd_write_denom,
        logup_gen
    );

    // 11. range_check_20: -1 * (clk - rd_clk_prev)
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

    let cols = LtImmColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clk.len();

    let one = PackedM31::broadcast(BaseField::one());
    let two = PackedM31::broadcast(BaseField::from_u32_unchecked(2));
    let pow2_7 = PackedM31::broadcast(BaseField::from_u32_unchecked(128));

    // Numerator: enabler (sum of opcode flags)
    let enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.opcode_slti_flag[i] + cols.opcode_sltiu_flag[i])
        .collect();

    let clk_minus_rs1_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rs1_clk_prev[i])
        .collect();
    let clk_minus_rd_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rd_clk_prev[i])
        .collect();
    let diff_val_minus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.diff_val[i] - one).collect();

    // prefix_sum_final = sum of diff_markers
    let prefix_sum_final: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.diff_marker_0[i]
                + cols.diff_marker_1[i]
                + cols.diff_marker_2[i]
                + cols.diff_marker_3[i]
        })
        .collect();

    // rs1_msl_adjusted = rs1_msl_felt + opcode_slti_flag * 128
    let rs1_msl_adjusted: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.rs1_msl_felt[i] + cols.opcode_slti_flag[i] * pow2_7)
        .collect();

    // imm_1_times_2 = 2 * imm_1
    let imm_1_times_2: Vec<PackedM31> = (0..simd_size).map(|i| two * cols.imm_1[i]).collect();

    // Register range_check_8_8_4: (rs1_msl_adjusted, imm_0, 2*imm_1)
    counters
        .range_check_8_8_4
        .register_many(&enabler, &[&rs1_msl_adjusted, cols.imm_0, &imm_1_times_2]);

    // Register range_check_20: (clk - rs1_clk_prev)
    counters
        .range_check_20
        .register_many(&enabler, &[&clk_minus_rs1_clk_prev]);

    // Register range_check_20: (clk - rd_clk_prev)
    counters
        .range_check_20
        .register_many(&enabler, &[&clk_minus_rd_clk_prev]);

    // Register range_check_20: (diff_val - 1) with multiplicity prefix_sum_final
    counters
        .range_check_20
        .register_many(&prefix_sum_final, &[&diff_val_minus_1]);
}
