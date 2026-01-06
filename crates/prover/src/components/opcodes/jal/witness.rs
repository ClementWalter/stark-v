//! Witness generation for jal component.

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

use super::columns::JalColumns;
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

    let cols = JalColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clk.len();

    let log_size = trace[0].domain.log_size();
    let mut logup_gen = LogupTraceGenerator::new(log_size);

    // Constants
    let opcode_jal_id = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Jal as u32));
    let zero = PackedM31::zero();
    let one = PackedM31::broadcast(BaseField::one());

    let opcode_id_col: Vec<PackedM31> = vec![opcode_jal_id; simd_size];
    let zero_col: Vec<PackedM31> = vec![zero; simd_size];

    // Derived columns
    let pc_plus_imm: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.pc[i] + cols.imm_felt[i])
        .collect();
    let clk_plus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.clk[i] + one).collect();
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

    // =====================================================================
    // LogUp entries (same order as AIR)
    // =====================================================================

    // 1. program_access: -enabler * (pc, opcode_jal_id, rd_addr, imm_felt, 0)
    let program_denom = combine!(
        relations.program_access,
        [
            cols.pc,
            &opcode_id_col,
            cols.rd_addr,
            cols.imm_felt,
            &zero_col
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

    // 3. registers_state: +enabler * (pc + imm_felt, clk + 1)
    let registers_write_denom = combine!(relations.registers_state, [&pc_plus_imm, &clk_plus_1]);

    // 4. range_check_8_8: -1 * (rd_next_1, rd_next_2)
    let rc_8_8_denom = combine!(relations.range_check_8_8, [cols.rd_next_1, cols.rd_next_2]);

    write_pair!(
        &pos_enabler,
        &registers_write_denom,
        &neg_enabler,
        &rc_8_8_denom,
        logup_gen
    );

    // 5. range_check_m31: -1 * (rd_next_0, rd_next_3)
    let rc_m31_denom = combine!(relations.range_check_m31, [cols.rd_next_0, cols.rd_next_3]);

    // 6. memory_access: -enabler * (0, rd_addr, rd_clk_prev, rd_prev_0..3)
    let mem_read_denom = combine!(
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
        &neg_enabler,
        &rc_m31_denom,
        &neg_enabler,
        &mem_read_denom,
        logup_gen
    );

    // 7. memory_access: +enabler * (0, rd_addr, clk, rd_next_0..3)
    let mem_write_denom = combine!(
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

    // 8. range_check_20: -1 * (clk - rd_clk_prev)
    let rc_20_denom = combine!(relations.range_check_20, [&clk_minus_rd_clk_prev]);

    write_pair!(
        &pos_enabler,
        &mem_write_denom,
        &neg_enabler,
        &rc_20_denom,
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

    let cols = JalColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clk.len();

    // Numerator: negated enabler (to match gen_interaction_trace)
    let neg_enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| -cols.enabler[i])
        .collect();

    // Derived columns (same as gen_interaction_trace)
    let clk_minus_rd_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rd_clk_prev[i])
        .collect();

    // Register range_check_8_8: (rd_next_1, rd_next_2) with negated multiplicity
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[cols.rd_next_1, cols.rd_next_2]);

    // Register range_check_m31: (rd_next_0, rd_next_3) with negated multiplicity
    counters
        .range_check_m31
        .register_many(&neg_enabler, &[cols.rd_next_0, cols.rd_next_3]);

    // Register range_check_20: (clk - rd_clk_prev) with negated multiplicity
    counters
        .range_check_20
        .register_many(&neg_enabler, &[&clk_minus_rd_clk_prev]);
}
