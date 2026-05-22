//! Witness generation for branch_eq component.

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

use runner::trace::prover_columns::BranchEqColumns;

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

    let cols = BranchEqColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clock.len();

    let log_size = trace[0].domain.log_size();
    let mut logup_gen = LogupTraceGenerator::new(log_size);

    // Constants
    let zero = PackedM31::zero();
    let one = PackedM31::broadcast(BaseField::one());
    let four = PackedM31::broadcast(BaseField::from_u32_unchecked(4));

    let opcode_beq = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Beq as u32));
    let opcode_bne = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Bne as u32));

    let zero_col: Vec<PackedM31> = vec![zero; simd_size];

    // Compute derived columns
    let enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.opcode_beq_flag[i] + cols.opcode_bne_flag[i])
        .collect();

    let expected_opcode_id: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.opcode_beq_flag[i] * opcode_beq + cols.opcode_bne_flag[i] * opcode_bne)
        .collect();

    // to_pc = pc + imm_felt * cmp_result + 4 * (1 - cmp_result)
    let to_pc: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.pc[i] + cols.imm_felt[i] * cols.cmp_result[i] + four * (one - cols.cmp_result[i])
        })
        .collect();

    let clock_plus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.clock[i] + one).collect();
    let clock_minus_rs1_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rs1_clock_prev[i])
        .collect();
    let clock_minus_rs2_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rs2_clock_prev[i])
        .collect();

    // Numerators
    let neg_enabler: Vec<PackedQM31> = enabler.iter().map(|&e| -PackedQM31::from(e)).collect();
    let pos_enabler: Vec<PackedQM31> = enabler.iter().map(|&e| PackedQM31::from(e)).collect();

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

    // 2. memory_access: -enabler * (0, rs1_addr, rs1_clock_prev, rs1_prev_0..3)
    let rs1_read_denom = combine!(
        relations.memory_access,
        [
            &zero_col,
            cols.rs1_addr,
            cols.rs1_clock_prev,
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

    // 3. memory_access: +enabler * (0, rs1_addr, clock, rs1_next_0..3)
    let rs1_write_denom = combine!(
        relations.memory_access,
        [
            &zero_col,
            cols.rs1_addr,
            cols.clock,
            cols.rs1_next_0,
            cols.rs1_next_1,
            cols.rs1_next_2,
            cols.rs1_next_3
        ]
    );

    // 4. range_check_20: -1 * (clock - rs1_clock_prev)
    let rc_20_rs1_denom = combine!(relations.range_check_20, [&clock_minus_rs1_clock_prev]);

    write_pair!(
        &pos_enabler,
        &rs1_write_denom,
        &neg_enabler,
        &rc_20_rs1_denom,
        logup_gen
    );

    // 5. memory_access: -enabler * (0, rs2_addr, rs2_clock_prev, rs2_prev_0..3)
    let rs2_read_denom = combine!(
        relations.memory_access,
        [
            &zero_col,
            cols.rs2_addr,
            cols.rs2_clock_prev,
            cols.rs2_prev_0,
            cols.rs2_prev_1,
            cols.rs2_prev_2,
            cols.rs2_prev_3
        ]
    );

    // 6. memory_access: +enabler * (0, rs2_addr, clock, rs2_next_0..3)
    let rs2_write_denom = combine!(
        relations.memory_access,
        [
            &zero_col,
            cols.rs2_addr,
            cols.clock,
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

    // 7. range_check_20: -1 * (clock - rs2_clock_prev)
    let rc_20_rs2_denom = combine!(relations.range_check_20, [&clock_minus_rs2_clock_prev]);

    // 8. registers_state: -enabler * (pc, clock)
    let registers_read_denom = combine!(relations.registers_state, [cols.pc, cols.clock]);

    write_pair!(
        &neg_enabler,
        &rc_20_rs2_denom,
        &neg_enabler,
        &registers_read_denom,
        logup_gen
    );

    // 9. registers_state: +enabler * (to_pc, clock + 1)
    let registers_write_denom = combine!(relations.registers_state, [&to_pc, &clock_plus_1]);

    write_col!(&pos_enabler, &registers_write_denom, logup_gen);

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

    let cols = BranchEqColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clock.len();

    // Numerator: negated enabler (to match gen_interaction_trace)
    let neg_enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| -(cols.opcode_beq_flag[i] + cols.opcode_bne_flag[i]))
        .collect();

    let clock_minus_rs1_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rs1_clock_prev[i])
        .collect();
    let clock_minus_rs2_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rs2_clock_prev[i])
        .collect();

    counters
        .range_check_20
        .register_many(&neg_enabler, &[&clock_minus_rs1_clock_prev]);
    counters
        .range_check_20
        .register_many(&neg_enabler, &[&clock_minus_rs2_clock_prev]);
}
