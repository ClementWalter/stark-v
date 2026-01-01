//! Witness generation for base_alu_reg component.

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

use super::columns::BaseAluRegColumns;
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

    let cols = BaseAluRegColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clk.len();

    let log_size = trace[0].domain.log_size();
    let mut logup_gen = LogupTraceGenerator::new(log_size);

    // Constants
    let zero = PackedM31::zero();
    let one = PackedM31::broadcast(BaseField::one());
    let two = PackedM31::broadcast(BaseField::from_u32_unchecked(2));
    let three = PackedM31::broadcast(BaseField::from_u32_unchecked(3));
    let four = PackedM31::broadcast(BaseField::from_u32_unchecked(4));

    let opcode_add = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Add as u32));
    let opcode_sub = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Sub as u32));
    let opcode_xor = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Xor as u32));
    let opcode_or = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Or as u32));
    let opcode_and = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::And as u32));

    let zero_col: Vec<PackedM31> = vec![zero; simd_size];

    // Compute derived columns
    let enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.opcode_add_flag[i]
                + cols.opcode_sub_flag[i]
                + cols.opcode_xor_flag[i]
                + cols.opcode_or_flag[i]
                + cols.opcode_and_flag[i]
        })
        .collect();

    let expected_opcode_id: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.opcode_add_flag[i] * opcode_add
                + cols.opcode_sub_flag[i] * opcode_sub
                + cols.opcode_xor_flag[i] * opcode_xor
                + cols.opcode_or_flag[i] * opcode_or
                + cols.opcode_and_flag[i] * opcode_and
        })
        .collect();

    let is_bitwise: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.opcode_xor_flag[i] + cols.opcode_or_flag[i] + cols.opcode_and_flag[i])
        .collect();

    let bitwise_id: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.opcode_xor_flag[i]
                + two * cols.opcode_or_flag[i]
                + three * cols.opcode_and_flag[i]
                + four * (cols.opcode_add_flag[i] + cols.opcode_sub_flag[i])
        })
        .collect();

    let pc_plus_4: Vec<PackedM31> = (0..simd_size).map(|i| cols.pc[i] + four).collect();
    let clk_plus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.clk[i] + one).collect();
    let clk_minus_rs1_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rs1_clk_prev[i])
        .collect();
    let clk_minus_rs2_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rs2_clk_prev[i])
        .collect();
    let clk_minus_rd_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rd_clk_prev[i])
        .collect();

    // Numerators
    let neg_enabler: Vec<PackedQM31> = enabler.iter().map(|&e| -PackedQM31::from(e)).collect();
    let pos_enabler: Vec<PackedQM31> = enabler.iter().map(|&e| PackedQM31::from(e)).collect();
    let neg_is_bitwise: Vec<PackedQM31> =
        is_bitwise.iter().map(|&b| -PackedQM31::from(b)).collect();
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

    // 10. bitwise: -is_bitwise * (rs1[0], rs2[0], rd[0], bitwise_id)
    let bitwise_0_denom = combine!(
        relations.bitwise,
        [
            cols.rs1_next_0,
            cols.rs2_next_0,
            cols.rd_next_0,
            &bitwise_id
        ]
    );

    write_pair!(
        &neg_one,
        &rc_20_rs2_denom,
        &neg_is_bitwise,
        &bitwise_0_denom,
        logup_gen
    );

    // 11. bitwise: -is_bitwise * (rs1[1], rs2[1], rd[1], bitwise_id)
    let bitwise_1_denom = combine!(
        relations.bitwise,
        [
            cols.rs1_next_1,
            cols.rs2_next_1,
            cols.rd_next_1,
            &bitwise_id
        ]
    );

    // 12. bitwise: -is_bitwise * (rs1[2], rs2[2], rd[2], bitwise_id)
    let bitwise_2_denom = combine!(
        relations.bitwise,
        [
            cols.rs1_next_2,
            cols.rs2_next_2,
            cols.rd_next_2,
            &bitwise_id
        ]
    );

    write_pair!(
        &neg_is_bitwise,
        &bitwise_1_denom,
        &neg_is_bitwise,
        &bitwise_2_denom,
        logup_gen
    );

    // 13. bitwise: -is_bitwise * (rs1[3], rs2[3], rd[3], bitwise_id)
    let bitwise_3_denom = combine!(
        relations.bitwise,
        [
            cols.rs1_next_3,
            cols.rs2_next_3,
            cols.rd_next_3,
            &bitwise_id
        ]
    );

    // 14. memory_access: -enabler * (0, rd_addr, rd_clk_prev, rd_prev_0..3)
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
        &neg_is_bitwise,
        &bitwise_3_denom,
        &neg_enabler,
        &rd_read_denom,
        logup_gen
    );

    // 15. memory_access: +enabler * (0, rd_addr, clk, rd_next_0..3)
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

    // 16. range_check_20: -1 * (clk - rd_clk_prev)
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
