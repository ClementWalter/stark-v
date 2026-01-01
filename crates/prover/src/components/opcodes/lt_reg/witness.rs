//! Witness generation for lt_reg component.

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

use super::columns::LtRegColumns;
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

    let cols = LtRegColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clk.len();

    let log_size = trace[0].domain.log_size();
    let mut logup_gen = LogupTraceGenerator::new(log_size);

    // Constants
    let zero = PackedM31::zero();
    let one = PackedM31::broadcast(BaseField::one());
    let four = PackedM31::broadcast(BaseField::from_u32_unchecked(4));
    let pow2_7 = PackedM31::broadcast(BaseField::from_u32_unchecked(128));

    let opcode_slt = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Slt as u32));
    let opcode_sltu = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Sltu as u32));

    let zero_col: Vec<PackedM31> = vec![zero; simd_size];

    // Compute derived columns
    let enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.opcode_slt_flag[i] + cols.opcode_sltu_flag[i])
        .collect();

    let expected_opcode_id: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.opcode_slt_flag[i] * opcode_slt + cols.opcode_sltu_flag[i] * opcode_sltu)
        .collect();

    let prefix_sum_final: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.diff_marker_0[i]
                + cols.diff_marker_1[i]
                + cols.diff_marker_2[i]
                + cols.diff_marker_3[i]
        })
        .collect();

    // msl_felt + opcode_slt_flag * 2^7
    let rs1_msl_adjusted: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.rs1_msl_felt[i] + cols.opcode_slt_flag[i] * pow2_7)
        .collect();
    let rs2_msl_adjusted: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.rs2_msl_felt[i] + cols.opcode_slt_flag[i] * pow2_7)
        .collect();

    let pc_plus_4: Vec<PackedM31> = (0..simd_size).map(|i| cols.pc[i] + four).collect();
    let clk_plus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.clk[i] + one).collect();
    let clk_minus_rs1_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rs1_clk_prev[i])
        .collect();
    let clk_minus_rs2_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rs2_clk_prev[i])
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
    let neg_one = vec![-PackedQM31::one(); simd_size];

    // =====================================================================
    // LogUp entries (same order as AIR)
    // =====================================================================

    // 1. program_access: -enabler * (pc, expected_opcode_id, rd_addr, rs1_addr, rs2_addr)
    let program_denom = combine!(
        relations.program_access,
        [
            cols.pc,
            &expected_opcode_id,
            cols.rd_addr,
            cols.rs1_addr,
            cols.rs2_addr
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

    // 12. memory_access: -enabler * (0, rd_addr, rd_clk_prev, rd_prev_0..3)
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

    write_pair!(
        &neg_prefix_sum,
        &rc_20_diff_denom,
        &neg_enabler,
        &rd_read_denom,
        logup_gen
    );

    // 13. memory_access: +enabler * (0, rd_addr, clk, cmp_result, 0, 0, 0)
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

    // 14. range_check_20: -1 * (clk - rd_clk_prev)
    let rc_20_rd_denom = combine!(relations.range_check_20, [&clk_minus_rd_clk_prev]);

    write_pair!(
        &pos_enabler,
        &rd_write_denom,
        &neg_one,
        &rc_20_rd_denom,
        logup_gen
    );

    logup_gen.finalize_last()
}

/// Register multiplicities for preprocessed lookups.
pub fn register_multiplicities(
    trace: &runner::trace::LtRegTable,
    counters: &mut crate::relations::Counters,
) {
    // Compute clock differences for rs1
    let clk_minus_rs1_clk_prev: Vec<u32> = trace
        .clk
        .iter()
        .zip(trace.rs1_clk_prev.iter())
        .map(|(clk, prev)| clk.wrapping_sub(*prev))
        .collect();

    // Compute clock differences for rs2
    let clk_minus_rs2_clk_prev: Vec<u32> = trace
        .clk
        .iter()
        .zip(trace.rs2_clk_prev.iter())
        .map(|(clk, prev)| clk.wrapping_sub(*prev))
        .collect();

    // Compute clock differences for rd
    let clk_minus_rd_clk_prev: Vec<u32> = trace
        .clk
        .iter()
        .zip(trace.rd_clk_prev.iter())
        .map(|(clk, prev)| clk.wrapping_sub(*prev))
        .collect();

    // Compute diff_val - 1 (used for range check)
    let diff_val_minus_1: Vec<u32> = trace.diff_val.iter().map(|v| v.wrapping_sub(1)).collect();

    // Compute prefix_sum_final for each row (sum of diff_markers)
    // Each row contributes prefix_sum_final lookups to range_check_20
    let prefix_sum_final: Vec<u32> = (0..trace.clk.len())
        .map(|i| {
            trace.diff_marker_0[i]
                + trace.diff_marker_1[i]
                + trace.diff_marker_2[i]
                + trace.diff_marker_3[i]
        })
        .collect();

    // Register range_check_20 multiplicities:
    // - clk - rs1_clk_prev (1 per row)
    // - clk - rs2_clk_prev (1 per row)
    // - clk - rd_clk_prev (1 per row)
    // - (diff_val - 1) with multiplicity prefix_sum_final
    counters
        .range_check_20
        .register_many([&clk_minus_rs1_clk_prev]);
    counters
        .range_check_20
        .register_many([&clk_minus_rs2_clk_prev]);
    counters
        .range_check_20
        .register_many([&clk_minus_rd_clk_prev]);

    // Register (diff_val - 1) with variable multiplicity
    for (i, &count) in prefix_sum_final.iter().enumerate() {
        for _ in 0..count {
            counters.range_check_20.register([diff_val_minus_1[i]]);
        }
    }

    // Compute adjusted msl values for range_check_8_8
    // rs1_msl_adjusted = rs1_msl_felt + opcode_slt_flag * 128
    // rs2_msl_adjusted = rs2_msl_felt + opcode_slt_flag * 128
    let rs1_msl_adjusted: Vec<u32> = trace
        .rs1_msl_felt
        .iter()
        .zip(trace.opcode_slt_flag.iter())
        .map(|(msl, flag)| msl + flag * 128)
        .collect();

    let rs2_msl_adjusted: Vec<u32> = trace
        .rs2_msl_felt
        .iter()
        .zip(trace.opcode_slt_flag.iter())
        .map(|(msl, flag)| msl + flag * 128)
        .collect();

    // Register range_check_8_8 multiplicities: (rs1_msl_adjusted, rs2_msl_adjusted)
    counters
        .range_check_8_8
        .register_many([&rs1_msl_adjusted, &rs2_msl_adjusted]);
}
