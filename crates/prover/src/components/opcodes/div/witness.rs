//! Witness generation for div component.

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

use super::columns::DivColumns;

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

    let cols = DivColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clock.len();

    let log_size = trace[0].domain.log_size();
    let mut logup_gen = LogupTraceGenerator::new(log_size);

    // Constants
    let zero = PackedM31::zero();
    let one = PackedM31::broadcast(BaseField::one());
    let four = PackedM31::broadcast(BaseField::from_u32_unchecked(4));

    let opcode_div = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Div as u32));
    let opcode_divu = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Divu as u32));
    let opcode_rem = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Rem as u32));
    let opcode_remu = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Remu as u32));

    let zero_col: Vec<PackedM31> = vec![zero; simd_size];

    // Compute derived columns
    let enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.opcode_div_flag[i]
                + cols.opcode_divu_flag[i]
                + cols.opcode_rem_flag[i]
                + cols.opcode_remu_flag[i]
        })
        .collect();

    let expected_opcode_id: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.opcode_div_flag[i] * opcode_div
                + cols.opcode_divu_flag[i] * opcode_divu
                + cols.opcode_rem_flag[i] * opcode_rem
                + cols.opcode_remu_flag[i] * opcode_remu
        })
        .collect();

    let is_div: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.opcode_div_flag[i] + cols.opcode_divu_flag[i])
        .collect();

    let special_case: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.zero_divisor[i] + cols.r_zero[i])
        .collect();

    // a[i] = is_div * q[i] + (1 - is_div) * r[i]
    let a_0: Vec<PackedM31> = (0..simd_size)
        .map(|i| is_div[i] * cols.q_0[i] + (one - is_div[i]) * cols.r_0[i])
        .collect();
    let a_1: Vec<PackedM31> = (0..simd_size)
        .map(|i| is_div[i] * cols.q_1[i] + (one - is_div[i]) * cols.r_1[i])
        .collect();
    let a_2: Vec<PackedM31> = (0..simd_size)
        .map(|i| is_div[i] * cols.q_2[i] + (one - is_div[i]) * cols.r_2[i])
        .collect();
    let a_3: Vec<PackedM31> = (0..simd_size)
        .map(|i| is_div[i] * cols.q_3[i] + (one - is_div[i]) * cols.r_3[i])
        .collect();

    let pc_plus_4: Vec<PackedM31> = (0..simd_size).map(|i| cols.pc[i] + four).collect();
    let clock_plus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.clock[i] + one).collect();
    let clock_minus_rs1_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rs1_clock_prev[i])
        .collect();
    let clock_minus_rs2_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rs2_clock_prev[i])
        .collect();
    let lt_diff_minus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.lt_diff[i] - one).collect();
    let clock_minus_rd_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rd_clock_prev[i])
        .collect();

    // Numerators
    let neg_enabler: Vec<PackedQM31> = enabler.iter().map(|&e| -PackedQM31::from(e)).collect();
    let pos_enabler: Vec<PackedQM31> = enabler.iter().map(|&e| PackedQM31::from(e)).collect();
    let neg_valid_not_special: Vec<PackedQM31> = enabler
        .iter()
        .zip(special_case.iter())
        .map(|(&e, &s)| -PackedQM31::from(e - s))
        .collect();

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

    // 2. registers_state: -enabler * (pc, clock)
    let registers_read_denom = combine!(relations.registers_state, [cols.pc, cols.clock]);

    write_pair!(
        &neg_enabler,
        &program_denom,
        &neg_enabler,
        &registers_read_denom,
        logup_gen
    );

    // 3. registers_state: +enabler * (pc + 4, clock + 1)
    let registers_write_denom = combine!(relations.registers_state, [&pc_plus_4, &clock_plus_1]);

    // 4. memory_access: -enabler * (0, rs1_addr, rs1_clock_prev, rs1_prev_0..3)
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
        &pos_enabler,
        &registers_write_denom,
        &neg_enabler,
        &rs1_read_denom,
        logup_gen
    );

    // 5. memory_access: +enabler * (0, rs1_addr, clock, rs1_next_0..3)
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

    // 6. range_check_20: -1 * (clock - rs1_clock_prev)
    let rc_20_rs1_denom = combine!(relations.range_check_20, [&clock_minus_rs1_clock_prev]);

    write_pair!(
        &pos_enabler,
        &rs1_write_denom,
        &neg_enabler,
        &rc_20_rs1_denom,
        logup_gen
    );

    // 7. memory_access: -enabler * (0, rs2_addr, rs2_clock_prev, rs2_prev_0..3)
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

    // 8. memory_access: +enabler * (0, rs2_addr, clock, rs2_next_0..3)
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

    // 9. range_check_20: -1 * (clock - rs2_clock_prev)
    let rc_20_rs2_denom = combine!(relations.range_check_20, [&clock_minus_rs2_clock_prev]);

    // 10. range_check_8_8: -1 * (q_0, q_1)
    let rc_8_8_q_0_denom = combine!(relations.range_check_8_8, [cols.q_0, cols.q_1]);

    write_pair!(
        &neg_enabler,
        &rc_20_rs2_denom,
        &neg_enabler,
        &rc_8_8_q_0_denom,
        logup_gen
    );

    // 11. range_check_8_8: -1 * (q_2, q_3)
    let rc_8_8_q_1_denom = combine!(relations.range_check_8_8, [cols.q_2, cols.q_3]);

    // 12. range_check_8_8: -1 * (r_0, r_1)
    let rc_8_8_r_0_denom = combine!(relations.range_check_8_8, [cols.r_0, cols.r_1]);

    write_pair!(
        &neg_enabler,
        &rc_8_8_q_1_denom,
        &neg_enabler,
        &rc_8_8_r_0_denom,
        logup_gen
    );

    // 13. range_check_8_8: -1 * (r_2, r_3)
    let rc_8_8_r_1_denom = combine!(relations.range_check_8_8, [cols.r_2, cols.r_3]);

    // 14. range_check_20: -(enabler - special_case) * (lt_diff - 1)
    let rc_20_lt_diff_denom = combine!(relations.range_check_20, [&lt_diff_minus_1]);

    write_pair!(
        &neg_enabler,
        &rc_8_8_r_1_denom,
        &neg_valid_not_special,
        &rc_20_lt_diff_denom,
        logup_gen
    );

    // 15. memory_access: -enabler * (0, rd_addr, rd_clock_prev, rd_prev_0..3)
    let rd_read_denom = combine!(
        relations.memory_access,
        [
            &zero_col,
            cols.rd_addr,
            cols.rd_clock_prev,
            cols.rd_prev_0,
            cols.rd_prev_1,
            cols.rd_prev_2,
            cols.rd_prev_3
        ]
    );

    // 16. memory_access: +enabler * (0, rd_addr, clock, a[0..3])
    let rd_write_denom = combine!(
        relations.memory_access,
        [&zero_col, cols.rd_addr, cols.clock, &a_0, &a_1, &a_2, &a_3]
    );

    write_pair!(
        &neg_enabler,
        &rd_read_denom,
        &pos_enabler,
        &rd_write_denom,
        logup_gen
    );

    // 17. range_check_20: -1 * (clock - rd_clock_prev)
    let rc_20_rd_denom = combine!(relations.range_check_20, [&clock_minus_rd_clock_prev]);

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

    let cols = DivColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clock.len();

    let one = PackedM31::broadcast(BaseField::one());

    // Numerators (same as gen_interaction_trace, but negated to match)
    let neg_enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            -(cols.opcode_div_flag[i]
                + cols.opcode_divu_flag[i]
                + cols.opcode_rem_flag[i]
                + cols.opcode_remu_flag[i])
        })
        .collect();
    let special_case: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.zero_divisor[i] + cols.r_zero[i])
        .collect();
    // -(enabler - special_case) for lt_diff lookup (negated to match)
    let neg_valid_not_special: Vec<PackedM31> = (0..simd_size)
        .map(|i| neg_enabler[i] + special_case[i])
        .collect();

    let clock_minus_rs1_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rs1_clock_prev[i])
        .collect();
    let clock_minus_rs2_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rs2_clock_prev[i])
        .collect();
    let clock_minus_rd_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rd_clock_prev[i])
        .collect();
    let lt_diff_minus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.lt_diff[i] - one).collect();

    counters
        .range_check_20
        .register_many(&neg_enabler, &[&clock_minus_rs1_clock_prev]);
    counters
        .range_check_20
        .register_many(&neg_enabler, &[&clock_minus_rs2_clock_prev]);

    // Register range_check_8_8 for q limbs with negated multiplicity
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[cols.q_0, cols.q_1]);
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[cols.q_2, cols.q_3]);

    // Register range_check_8_8 for r limbs with negated multiplicity
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[cols.r_0, cols.r_1]);
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[cols.r_2, cols.r_3]);

    counters
        .range_check_20
        .register_many(&neg_valid_not_special, &[&lt_diff_minus_1]);
    counters
        .range_check_20
        .register_many(&neg_enabler, &[&clock_minus_rd_clock_prev]);
}
