//! Witness generation for mul component.

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

use super::columns::MulColumns;

fn write_single_batch(
    logup_gen: &mut LogupTraceGenerator,
    numerators: &[PackedQM31],
    denominators: &[PackedQM31],
) {
    let mut col_gen = logup_gen.new_col();
    for vec_row in 0..numerators.len() {
        col_gen.write_frac(vec_row, numerators[vec_row], denominators[vec_row]);
    }
    col_gen.finalize_col();
}

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

    let cols = MulColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clock.len();

    let log_size = trace[0].domain.log_size();
    let mut logup_gen = LogupTraceGenerator::new(log_size);

    // Constants
    let zero = PackedM31::zero();
    let one = PackedM31::broadcast(BaseField::one());
    let four = PackedM31::broadcast(BaseField::from_u32_unchecked(4));
    let inv_two_pow_8 = PackedM31::broadcast(BaseField::from_u32_unchecked(1 << 8).inverse());

    let opcode_mul_id = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Mul as u32));

    let zero_col: Vec<PackedM31> = vec![zero; simd_size];
    let opcode_id_col: Vec<PackedM31> = vec![opcode_mul_id; simd_size];

    // Compute carries (same as AIR)
    let carry_0: Vec<PackedM31> = (0..simd_size)
        .map(|i| (cols.rs1_next_0[i] * cols.rs2_next_0[i] - cols.rd_next_0[i]) * inv_two_pow_8)
        .collect();

    let carry_1: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            let limb_sum = carry_0[i]
                + cols.rs1_next_1[i] * cols.rs2_next_0[i]
                + cols.rs1_next_0[i] * cols.rs2_next_1[i];
            (limb_sum - cols.rd_next_1[i]) * inv_two_pow_8
        })
        .collect();

    let carry_2: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            let limb_sum = carry_1[i]
                + cols.rs1_next_2[i] * cols.rs2_next_0[i]
                + cols.rs1_next_1[i] * cols.rs2_next_1[i]
                + cols.rs1_next_0[i] * cols.rs2_next_2[i];
            (limb_sum - cols.rd_next_2[i]) * inv_two_pow_8
        })
        .collect();

    let carry_3: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            let limb_sum = carry_2[i]
                + cols.rs1_next_3[i] * cols.rs2_next_0[i]
                + cols.rs1_next_2[i] * cols.rs2_next_1[i]
                + cols.rs1_next_1[i] * cols.rs2_next_2[i]
                + cols.rs1_next_0[i] * cols.rs2_next_3[i];
            (limb_sum - cols.rd_next_3[i]) * inv_two_pow_8
        })
        .collect();

    let pc_plus_4: Vec<PackedM31> = (0..simd_size).map(|i| cols.pc[i] + four).collect();
    let clock_plus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.clock[i] + one).collect();
    let clock_minus_rs1_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rs1_clock_prev[i])
        .collect();
    let clock_minus_rs2_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rs2_clock_prev[i])
        .collect();
    let clock_minus_rd_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rd_clock_prev[i])
        .collect();

    // Numerators
    let neg_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| -PackedQM31::from(cols.enabler[i]))
        .collect();
    let pos_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| PackedQM31::from(cols.enabler[i]))
        .collect();

    // =====================================================================
    // LogUp entries (same order as AIR)
    // =====================================================================

    // 1. program_access: -enabler * (pc, opcode_mul_id, rd_addr, rs1_addr, rs2_addr)
    let program_denom = combine!(
        relations.program_access,
        [
            cols.pc,
            &opcode_id_col,
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

    // 10. range_check_8_8: -1 * (carry[0], carry[1])
    let rc_8_8_carry_0_denom = combine!(relations.range_check_8_8, [&carry_0, &carry_1]);

    write_single_batch(&mut logup_gen, &neg_enabler, &rc_20_rs2_denom);
    write_single_batch(&mut logup_gen, &neg_enabler, &rc_8_8_carry_0_denom);

    // 11. range_check_8_8: -1 * (carry[2], carry[3])
    let rc_8_8_carry_1_denom = combine!(relations.range_check_8_8, [&carry_2, &carry_3]);

    // 12. range_check_8_8: -1 * (rd[0], rd[1])
    let rc_8_8_rd_0_denom = combine!(relations.range_check_8_8, [cols.rd_next_0, cols.rd_next_1]);

    // 13. range_check_8_8: -1 * (rd[2], rd[3])
    let rc_8_8_rd_1_denom = combine!(relations.range_check_8_8, [cols.rd_next_2, cols.rd_next_3]);

    // 14. memory_access: -enabler * (0, rd_addr, rd_clock_prev, rd_prev_0..3)
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

    write_single_batch(&mut logup_gen, &neg_enabler, &rc_8_8_carry_1_denom);
    write_pair!(
        &neg_enabler,
        &rc_8_8_rd_0_denom,
        &neg_enabler,
        &rc_8_8_rd_1_denom,
        logup_gen
    );

    // 15. memory_access: +enabler * (0, rd_addr, clock, rd_next_0..3)
    let rd_write_denom = combine!(
        relations.memory_access,
        [
            &zero_col,
            cols.rd_addr,
            cols.clock,
            cols.rd_next_0,
            cols.rd_next_1,
            cols.rd_next_2,
            cols.rd_next_3
        ]
    );

    // 16. range_check_20: -1 * (clock - rd_clock_prev)
    let rc_20_rd_denom = combine!(relations.range_check_20, [&clock_minus_rd_clock_prev]);

    write_pair!(
        &neg_enabler,
        &rd_read_denom,
        &pos_enabler,
        &rd_write_denom,
        logup_gen
    );
    write_single_batch(&mut logup_gen, &neg_enabler, &rc_20_rd_denom);

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

    let cols = MulColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clock.len();

    let inv_two_pow_8 = PackedM31::broadcast(BaseField::from_u32_unchecked(1 << 8).inverse());

    // Numerator: negated enabler (to match gen_interaction_trace)
    let neg_enabler: Vec<PackedM31> = (0..simd_size).map(|i| -cols.enabler[i]).collect();

    // Clock differences
    let clock_minus_rs1_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rs1_clock_prev[i])
        .collect();
    let clock_minus_rs2_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rs2_clock_prev[i])
        .collect();
    let clock_minus_rd_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rd_clock_prev[i])
        .collect();

    // Compute carries (same as gen_interaction_trace)
    let carry_0: Vec<PackedM31> = (0..simd_size)
        .map(|i| (cols.rs1_next_0[i] * cols.rs2_next_0[i] - cols.rd_next_0[i]) * inv_two_pow_8)
        .collect();

    let carry_1: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            let limb_sum = carry_0[i]
                + cols.rs1_next_1[i] * cols.rs2_next_0[i]
                + cols.rs1_next_0[i] * cols.rs2_next_1[i];
            (limb_sum - cols.rd_next_1[i]) * inv_two_pow_8
        })
        .collect();

    let carry_2: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            let limb_sum = carry_1[i]
                + cols.rs1_next_2[i] * cols.rs2_next_0[i]
                + cols.rs1_next_1[i] * cols.rs2_next_1[i]
                + cols.rs1_next_0[i] * cols.rs2_next_2[i];
            (limb_sum - cols.rd_next_2[i]) * inv_two_pow_8
        })
        .collect();

    let carry_3: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            let limb_sum = carry_2[i]
                + cols.rs1_next_3[i] * cols.rs2_next_0[i]
                + cols.rs1_next_2[i] * cols.rs2_next_1[i]
                + cols.rs1_next_1[i] * cols.rs2_next_2[i]
                + cols.rs1_next_0[i] * cols.rs2_next_3[i];
            (limb_sum - cols.rd_next_3[i]) * inv_two_pow_8
        })
        .collect();

    // Register range_check_20 for clock diffs with negated multiplicity
    counters
        .range_check_20
        .register_many(&neg_enabler, &[&clock_minus_rs1_clock_prev]);
    counters
        .range_check_20
        .register_many(&neg_enabler, &[&clock_minus_rs2_clock_prev]);

    // Register range_check_8_8 for carries with negated multiplicity
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[&carry_0, &carry_1]);
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[&carry_2, &carry_3]);

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
        .register_many(&neg_enabler, &[&clock_minus_rd_clock_prev]);
}
