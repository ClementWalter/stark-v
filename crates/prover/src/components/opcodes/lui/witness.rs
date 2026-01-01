//! Witness generation for lui component.

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
use crate::{combine, consume_col, write_pair};

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
    let simd_size = cols.clk.len();

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

    // clk + 1
    let clk_plus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.clk[i] + one).collect();

    // imm_0 * 16 (for rd write value)
    let imm_0_times_16: Vec<PackedM31> = (0..simd_size).map(|i| cols.imm_0[i] * pow2_4).collect();

    // clk - rd_clk_prev (for range check)
    let clk_minus_rd_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rd_clk_prev[i])
        .collect();

    // Compute numerators
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

    // 1. program_access: -enabler * (pc, opcode_lui_id, rd_addr, imm, 0)
    let program_denom = combine!(
        relations.program_access,
        [cols.pc, &opcode_lui_id_col, cols.rd_addr, &imm, &zero_col]
    );

    // 2. registers_state: -enabler * (pc, clk)
    let registers_read_denom = combine!(relations.registers_state, [cols.pc, cols.clk]);

    // Pair 1+2: both have -enabler numerator
    write_pair!(
        &neg_enabler,
        &program_denom,
        &neg_enabler,
        &registers_read_denom,
        logup_gen
    );

    // 3. registers_state: +enabler * (pc+4, clk+1)
    let registers_write_denom = combine!(relations.registers_state, [&pc_plus_4, &clk_plus_1]);

    // 4. range_check_8_8_4: -1 * (imm_1, imm_2, imm_0)
    let rc_8_8_4_denom = combine!(
        relations.range_check_8_8_4,
        [cols.imm_1, cols.imm_2, cols.imm_0]
    );

    // Pair 3+4: +enabler and -1
    write_pair!(
        &pos_enabler,
        &registers_write_denom,
        &neg_one,
        &rc_8_8_4_denom,
        logup_gen
    );

    // 5. memory_access: -enabler * (0, rd_addr, rd_clk_prev, rd_prev_0..3)
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

    // 6. memory_access: +enabler * (0, rd_addr, clk, 0, imm_0*16, imm_1, imm_2)
    let mem_write_denom = combine!(
        relations.memory_access,
        [
            &zero_col,
            cols.rd_addr,
            cols.clk,
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

    // 7. range_check_20: -1 * (clk - rd_clk_prev)
    let rc_20_denom = combine!(relations.range_check_20, [&clk_minus_rd_clk_prev]);

    // Leftover entry 7: -1 numerator
    consume_col!(rc_20_denom, logup_gen);

    logup_gen.finalize_last()
}

/// Register multiplicities for preprocessed lookups.
pub fn register_multiplicities(
    trace: &runner::trace::LuiTable,
    counters: &mut crate::relations::Counters,
) {
    // Compute clock differences
    let clk_minus_rd_clk_prev: Vec<u32> = trace
        .clk
        .iter()
        .zip(trace.rd_clk_prev.iter())
        .map(|(clk, prev)| clk.wrapping_sub(*prev))
        .collect();

    counters
        .range_check_20
        .register_many([&clk_minus_rd_clk_prev]);
}
