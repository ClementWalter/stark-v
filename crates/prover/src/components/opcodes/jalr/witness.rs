//! Witness generation for jalr component.

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

use super::columns::JalrColumns;
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

    let cols = JalrColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clk.len();

    let log_size = trace[0].domain.log_size();
    let mut logup_gen = LogupTraceGenerator::new(log_size);

    // Constants
    let opcode_jalr_id = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Jalr as u32));
    let zero = PackedM31::zero();
    let one = PackedM31::broadcast(BaseField::one());
    let two = PackedM31::broadcast(BaseField::from_u32_unchecked(2));

    let opcode_id_col: Vec<PackedM31> = vec![opcode_jalr_id; simd_size];
    let zero_col: Vec<PackedM31> = vec![zero; simd_size];

    // Derived columns
    let to_pc: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.to_pc_over_two[i] * two)
        .collect();
    let clk_plus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.clk[i] + one).collect();
    let clk_minus_rs1_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rs1_clk_prev[i])
        .collect();
    let clk_minus_rd_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rd_clk_prev[i])
        .collect();

    // Numerators
    let neg_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| -PackedQM31::from(cols.enabler[i]))
        .collect();
    let pos_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| PackedQM31::from(cols.enabler[i]))
        .collect();
    let neg_one = vec![-PackedQM31::one(); simd_size];

    // =====================================================================
    // LogUp entries (same order as AIR)
    // =====================================================================

    // 1. program_access: -enabler * (pc, opcode_jalr_id, rd_addr, rs1_addr, imm_felt)
    let program_denom = combine!(
        relations.program_access,
        [
            cols.pc,
            &opcode_id_col,
            cols.rd_addr,
            cols.rs1_addr,
            cols.imm_felt
        ]
    );

    // 2. memory_access: -enabler * (0, rs1_addr, rs1_clk_prev, rs1_prev_0..3)
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
        &neg_enabler,
        &program_denom,
        &neg_enabler,
        &rs1_read_denom,
        logup_gen
    );

    // 3. memory_access: +enabler * (0, rs1_addr, clk, rs1_next_0..3)
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

    // 4. range_check_20: -1 * (clk - rs1_clk_prev)
    let rc_20_rs1_denom = combine!(relations.range_check_20, [&clk_minus_rs1_clk_prev]);

    write_pair!(
        &pos_enabler,
        &rs1_write_denom,
        &neg_one,
        &rc_20_rs1_denom,
        logup_gen
    );

    // 5. range_check_m31: -1 * (rs1_next_0, rs1_next_3)
    let rc_m31_rs1_denom = combine!(
        relations.range_check_m31,
        [cols.rs1_next_0, cols.rs1_next_3]
    );

    // 6. registers_state: -enabler * (pc, clk)
    let registers_read_denom = combine!(relations.registers_state, [cols.pc, cols.clk]);

    write_pair!(
        &neg_one,
        &rc_m31_rs1_denom,
        &neg_enabler,
        &registers_read_denom,
        logup_gen
    );

    // 7. registers_state: +enabler * (2 * to_pc_over_two, clk + 1)
    let registers_write_denom = combine!(relations.registers_state, [&to_pc, &clk_plus_1]);

    // 8. range_check_8_8: -1 * (rd_next_1, rd_next_2)
    let rc_8_8_denom = combine!(relations.range_check_8_8, [cols.rd_next_1, cols.rd_next_2]);

    write_pair!(
        &pos_enabler,
        &registers_write_denom,
        &neg_one,
        &rc_8_8_denom,
        logup_gen
    );

    // 9. range_check_m31: -1 * (rd_next_0, rd_next_3)
    let rc_m31_rd_denom = combine!(relations.range_check_m31, [cols.rd_next_0, cols.rd_next_3]);

    // 10. memory_access: -enabler * (0, rd_addr, rd_clk_prev, rd_prev_0..3)
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
        &neg_one,
        &rc_m31_rd_denom,
        &neg_enabler,
        &rd_read_denom,
        logup_gen
    );

    // 11. memory_access: +enabler * (0, rd_addr, clk, rd_next_0..3)
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

    // 12. range_check_20: -1 * (clk - rd_clk_prev)
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
