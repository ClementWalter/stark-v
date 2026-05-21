//! Witness generation for lui component.

use num_traits::One;
use num_traits::Zero;
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

use super::columns::LuiColumns;

/// Generate interaction trace for LogUp.
///
/// Takes the trace columns (already converted from Table via into_witness).
/// Mirrors the LogUp entries from the AIR in the same order.
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

    // Get named column access - use .data to access the underlying Vec<PackedM31>
    let cols = LuiColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clock.len();

    let log_size = trace[0].domain.log_size();
    let mut logup_gen = LogupTraceGenerator::new(log_size);

    // Constants as column vectors
    let opcode_lui_id = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Lui as u32));
    let zero = PackedM31::zero();
    let one = PackedM31::one();
    let four = PackedM31::broadcast(BaseField::from_u32_unchecked(4));
    let pow2_4 = PackedM31::broadcast(BaseField::from_u32_unchecked(16));
    let pow2_12 = PackedM31::broadcast(BaseField::from_u32_unchecked(4096));

    // Create constant columns
    let opcode_lui_id_col: Vec<PackedM31> = vec![opcode_lui_id; simd_size];
    let zero_col: Vec<PackedM31> = vec![zero; simd_size];

    // Compute derived columns
    // imm = imm_0 + 2^4 * imm_1 + 2^12 * imm_2
    let imm: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.imm_0[i] + cols.imm_1[i] * pow2_4 + cols.imm_2[i] * pow2_12)
        .collect();

    // pc + 4
    let pc_plus_4: Vec<PackedM31> = (0..simd_size).map(|i| cols.pc[i] + four).collect();

    // clock + 1
    let clock_plus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.clock[i] + one).collect();

    // imm_0 * 16 (for rd write value)
    let imm_0_times_16: Vec<PackedM31> = (0..simd_size).map(|i| cols.imm_0[i] * pow2_4).collect();

    // clock - rd_clock_prev (for range check)
    let clock_minus_rd_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rd_clock_prev[i])
        .collect();

    // Compute numerators
    let neg_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| -PackedQM31::from(cols.enabler[i]))
        .collect();
    let pos_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| PackedQM31::from(cols.enabler[i]))
        .collect();

    // =====================================================================
    // LogUp entries (same order as AIR)
    // =====================================================================

    // 1. program_access: -enabler * (pc, opcode_lui_id, rd_addr, imm, 0)
    let program_denom = combine!(
        relations.program_access,
        [cols.pc, &opcode_lui_id_col, cols.rd_addr, &imm, &zero_col]
    );

    // 2. registers_state: -enabler * (pc, clock)
    let registers_read_denom = combine!(relations.registers_state, [cols.pc, cols.clock]);

    // Pair 1+2: both have -enabler numerator
    write_pair!(
        &neg_enabler,
        &program_denom,
        &neg_enabler,
        &registers_read_denom,
        logup_gen
    );

    // 3. registers_state: +enabler * (pc+4, clock+1)
    let registers_write_denom = combine!(relations.registers_state, [&pc_plus_4, &clock_plus_1]);

    // 4. range_check_8_8_4: -1 * (imm_1, imm_2, imm_0)
    let rc_8_8_4_denom = combine!(
        relations.range_check_8_8_4,
        [cols.imm_1, cols.imm_2, cols.imm_0]
    );

    // Pair 3+4: +enabler and -1
    write_pair!(
        &pos_enabler,
        &registers_write_denom,
        &neg_enabler,
        &rc_8_8_4_denom,
        logup_gen
    );

    // 5. memory_access: -enabler * (0, rd_addr, rd_clock_prev, rd_prev_0..3)
    let mem_read_denom = combine!(
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

    // 6. memory_access: +enabler * (0, rd_addr, clock, 0, imm_0*16, imm_1, imm_2)
    let mem_write_denom = combine!(
        relations.memory_access,
        [
            &zero_col,
            cols.rd_addr,
            cols.clock,
            &zero_col,
            &imm_0_times_16,
            cols.imm_1,
            cols.imm_2
        ]
    );

    // Pair 5+6: -enabler and +enabler
    write_pair!(
        &neg_enabler,
        &mem_read_denom,
        &pos_enabler,
        &mem_write_denom,
        logup_gen
    );

    // 7. range_check_20: -1 * (clock - rd_clock_prev)
    let rc_20_denom = combine!(relations.range_check_20, [&clock_minus_rd_clock_prev]);

    // Leftover entry 7: -1 numerator
    write_col!(&neg_enabler, &rc_20_denom, logup_gen);

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

    let cols = LuiColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clock.len();

    // Numerator: negated enabler (to match gen_interaction_trace)
    let neg_enabler: Vec<PackedM31> = (0..simd_size).map(|i| -cols.enabler[i]).collect();

    // Derived columns (same as gen_interaction_trace)
    let clock_minus_rd_clock_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clock[i] - cols.rd_clock_prev[i])
        .collect();

    // Register range_check_8_8_4: (imm_1, imm_2, imm_0) with negated multiplicity
    counters
        .range_check_8_8_4
        .register_many(&neg_enabler, &[cols.imm_1, cols.imm_2, cols.imm_0]);

    // Register range_check_20: (clock - rd_clock_prev) with negated multiplicity
    counters
        .range_check_20
        .register_many(&neg_enabler, &[&clock_minus_rd_clock_prev]);
}
