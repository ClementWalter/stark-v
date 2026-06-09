//! Witness generation for base_alu_imm component.

use num_traits::Zero;
use stwo::core::ColumnVec;
use stwo::core::fields::m31::BaseField;
use stwo::core::fields::qm31::QM31;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::backend::simd::m31::PackedM31;
use stwo::prover::backend::simd::qm31::PackedQM31;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;
use stwo_constraint_framework::LogupTraceGenerator;

use runner::trace::prover_columns::BaseAluImmColumns;

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

    let cols = BaseAluImmColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clock.len();

    let log_size = trace[0].domain.log_size();
    let mut logup_gen = LogupTraceGenerator::new(log_size);

    // Constants
    let zero = PackedM31::zero();
    let zero_col: Vec<PackedM31> = vec![zero; simd_size];

    // Derived columns from define_trace_tables! — same expressions as the AIR.
    let enabler: Vec<PackedM31> = (0..simd_size).map(|i| cols.at(i).enabler()).collect();
    let expected_opcode_id: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.at(i).expected_opcode_id())
        .collect();
    let imm: Vec<PackedM31> = (0..simd_size).map(|i| cols.at(i).imm()).collect();

    // Sign-extended immediate limbs; limb 0 is imm_0 and limb 3 equals limb 2
    let sext_imm_0: Vec<PackedM31> = cols.imm_0.to_vec();
    let sext_imm_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.at(i).sext_imm_1()).collect();
    let sext_imm_2: Vec<PackedM31> = (0..simd_size).map(|i| cols.at(i).sext_imm_2()).collect();
    let sext_imm_3 = sext_imm_2.clone();

    let is_bitwise: Vec<PackedM31> = (0..simd_size).map(|i| cols.at(i).is_bitwise()).collect();
    let bitwise_id: Vec<PackedM31> = (0..simd_size).map(|i| cols.at(i).bitwise_id()).collect();
    let imm_1_times_256: Vec<PackedM31> =
        (0..simd_size).map(|i| cols.at(i).imm_1_shifted()).collect();

    let pc_plus_4: Vec<PackedM31> = (0..simd_size).map(|i| cols.at(i).pc_next()).collect();
    let clock_plus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.at(i).clock_next()).collect();
    let clock_minus_rs1_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.at(i).rs1_clock_diff())
        .collect();
    let clock_minus_rd_clock_prev: Vec<PackedM31> =
        (0..simd_size).map(|i| cols.at(i).rd_clock_diff()).collect();

    // Numerators
    let neg_enabler: Vec<PackedQM31> = enabler.iter().map(|&e| -PackedQM31::from(e)).collect();
    let pos_enabler: Vec<PackedQM31> = enabler.iter().map(|&e| PackedQM31::from(e)).collect();
    let neg_is_bitwise: Vec<PackedQM31> =
        is_bitwise.iter().map(|&b| -PackedQM31::from(b)).collect();

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

    // 2. range_check_8_11: -1 * (imm_0, imm_1 * 256)
    let rc_8_11_denom = combine!(relations.range_check_8_11, [cols.imm_0, &imm_1_times_256]);

    write_pair!(
        &neg_enabler,
        &program_denom,
        &neg_enabler,
        &rc_8_11_denom,
        logup_gen
    );

    // 3. registers_state: -enabler * (pc, clock)
    let registers_read_denom = combine!(relations.registers_state, [cols.pc, cols.clock]);

    // 4. registers_state: +enabler * (pc + 4, clock + 1)
    let registers_write_denom = combine!(relations.registers_state, [&pc_plus_4, &clock_plus_1]);

    write_pair!(
        &neg_enabler,
        &registers_read_denom,
        &pos_enabler,
        &registers_write_denom,
        logup_gen
    );

    // 5. memory_access: -enabler * (0, rs1_addr, rs1_clock_prev, rs1_prev_0..3)
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

    // 6. memory_access: +enabler * (0, rs1_addr, clock, rs1_next_0..3)
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

    write_pair!(
        &neg_enabler,
        &rs1_read_denom,
        &pos_enabler,
        &rs1_write_denom,
        logup_gen
    );

    // 7. range_check_20: -1 * (clock - rs1_clock_prev)
    let rc_20_rs1_denom = combine!(relations.range_check_20, [&clock_minus_rs1_clock_prev]);

    // 8. bitwise: -is_bitwise * (rs1[0], sext_imm[0], rd[0], bitwise_id)
    let bitwise_0_denom = combine!(
        relations.bitwise,
        [cols.rs1_next_0, &sext_imm_0, cols.rd_next_0, &bitwise_id]
    );

    write_pair!(
        &neg_enabler,
        &rc_20_rs1_denom,
        &neg_is_bitwise,
        &bitwise_0_denom,
        logup_gen
    );

    // 9. bitwise: -is_bitwise * (rs1[1], sext_imm[1], rd[1], bitwise_id)
    let bitwise_1_denom = combine!(
        relations.bitwise,
        [cols.rs1_next_1, &sext_imm_1, cols.rd_next_1, &bitwise_id]
    );

    // 10. bitwise: -is_bitwise * (rs1[2], sext_imm[2], rd[2], bitwise_id)
    let bitwise_2_denom = combine!(
        relations.bitwise,
        [cols.rs1_next_2, &sext_imm_2, cols.rd_next_2, &bitwise_id]
    );

    write_pair!(
        &neg_is_bitwise,
        &bitwise_1_denom,
        &neg_is_bitwise,
        &bitwise_2_denom,
        logup_gen
    );

    // 11. bitwise: -is_bitwise * (rs1[3], sext_imm[3], rd[3], bitwise_id)
    let bitwise_3_denom = combine!(
        relations.bitwise,
        [cols.rs1_next_3, &sext_imm_3, cols.rd_next_3, &bitwise_id]
    );

    // 12. range_check_8_8: -1 * (rd[0], rd[1])
    let rc_8_8_0_denom = combine!(relations.range_check_8_8, [cols.rd_next_0, cols.rd_next_1]);

    write_pair!(
        &neg_is_bitwise,
        &bitwise_3_denom,
        &neg_enabler,
        &rc_8_8_0_denom,
        logup_gen
    );

    // 13. range_check_8_8: -1 * (rd[2], rd[3])
    let rc_8_8_1_denom = combine!(relations.range_check_8_8, [cols.rd_next_2, cols.rd_next_3]);

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

    write_pair!(
        &neg_enabler,
        &rc_8_8_1_denom,
        &neg_enabler,
        &rd_read_denom,
        logup_gen
    );

    // 15. memory_access: +enabler * (0, rd_addr, clock, rd[0..3])
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
        &pos_enabler,
        &rd_write_denom,
        &neg_enabler,
        &rc_20_rd_denom,
        logup_gen
    );

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

    let cols = BaseAluImmColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clock.len();

    // Numerators (same as gen_interaction_trace, but negated to match)
    let neg_enabler: Vec<PackedM31> = (0..simd_size).map(|i| -cols.at(i).enabler()).collect();
    let neg_is_bitwise: Vec<PackedM31> = (0..simd_size).map(|i| -cols.at(i).is_bitwise()).collect();

    // Derived columns (same as gen_interaction_trace)
    let imm_1_times_256: Vec<PackedM31> =
        (0..simd_size).map(|i| cols.at(i).imm_1_shifted()).collect();

    let clock_minus_rs1_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.at(i).rs1_clock_diff())
        .collect();
    let clock_minus_rd_clock_prev: Vec<PackedM31> =
        (0..simd_size).map(|i| cols.at(i).rd_clock_diff()).collect();

    // Sign-extended immediate limbs; limb 0 is imm_0 and limb 3 equals limb 2
    let sext_imm_0: Vec<PackedM31> = cols.imm_0.to_vec();
    let sext_imm_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.at(i).sext_imm_1()).collect();
    let sext_imm_2: Vec<PackedM31> = (0..simd_size).map(|i| cols.at(i).sext_imm_2()).collect();
    let sext_imm_3 = sext_imm_2.clone();

    let bitwise_id: Vec<PackedM31> = (0..simd_size).map(|i| cols.at(i).bitwise_id()).collect();

    // Register range_check_8_11: (imm_0, imm_1 * 256) with negated multiplicity
    counters
        .range_check_8_11
        .register_many(&neg_enabler, &[cols.imm_0, &imm_1_times_256]);

    // Register range_check_20: (clock - rs1_clock_prev)
    counters
        .range_check_20
        .register_many(&neg_enabler, &[&clock_minus_rs1_clock_prev]);

    // Register bitwise: 4 limbs (rs1_next[i], sext_imm[i], rd_next[i], bitwise_id)
    counters.bitwise.register_many(
        &neg_is_bitwise,
        &[cols.rs1_next_0, &sext_imm_0, cols.rd_next_0, &bitwise_id],
    );
    counters.bitwise.register_many(
        &neg_is_bitwise,
        &[cols.rs1_next_1, &sext_imm_1, cols.rd_next_1, &bitwise_id],
    );
    counters.bitwise.register_many(
        &neg_is_bitwise,
        &[cols.rs1_next_2, &sext_imm_2, cols.rd_next_2, &bitwise_id],
    );
    counters.bitwise.register_many(
        &neg_is_bitwise,
        &[cols.rs1_next_3, &sext_imm_3, cols.rd_next_3, &bitwise_id],
    );

    // Register range_check_8_8: (rd_next[0], rd_next[1]) and (rd_next[2], rd_next[3])
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[cols.rd_next_0, cols.rd_next_1]);
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[cols.rd_next_2, cols.rd_next_3]);

    // Register range_check_20: (clock - rd_clock_prev)
    counters
        .range_check_20
        .register_many(&neg_enabler, &[&clock_minus_rd_clock_prev]);
}
