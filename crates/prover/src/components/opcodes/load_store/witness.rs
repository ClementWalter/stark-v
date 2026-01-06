//! Witness generation for load_store component.

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

use super::columns::LoadStoreColumns;
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

    let cols = LoadStoreColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clk.len();

    let log_size = trace[0].domain.log_size();
    let mut logup_gen = LogupTraceGenerator::new(log_size);

    // Constants
    let zero = PackedM31::zero();
    let one = PackedM31::broadcast(BaseField::one());
    let four = PackedM31::broadcast(BaseField::from_u32_unchecked(4));
    let pow2_14 = PackedM31::broadcast(BaseField::from_u32_unchecked(1 << 14));
    let quarter_inv = PackedM31::broadcast(BaseField::from_u32_unchecked(4).inverse());

    let opcode_lb = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Lb as u32));
    let opcode_lh = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Lh as u32));
    let opcode_lbu = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Lbu as u32));
    let opcode_lhu = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Lhu as u32));
    let opcode_lw = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Lw as u32));
    let opcode_sb = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Sb as u32));
    let opcode_sh = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Sh as u32));
    let opcode_sw = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Sw as u32));

    let zero_col: Vec<PackedM31> = vec![zero; simd_size];

    // Compute derived columns
    let enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.opcode_lb_flag[i]
                + cols.opcode_lh_flag[i]
                + cols.opcode_lbu_flag[i]
                + cols.opcode_lhu_flag[i]
                + cols.opcode_lw_flag[i]
                + cols.opcode_sb_flag[i]
                + cols.opcode_sh_flag[i]
                + cols.opcode_sw_flag[i]
        })
        .collect();

    let expected_opcode_id: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.opcode_lb_flag[i] * opcode_lb
                + cols.opcode_lh_flag[i] * opcode_lh
                + cols.opcode_lbu_flag[i] * opcode_lbu
                + cols.opcode_lhu_flag[i] * opcode_lhu
                + cols.opcode_lw_flag[i] * opcode_lw
                + cols.opcode_sb_flag[i] * opcode_sb
                + cols.opcode_sh_flag[i] * opcode_sh
                + cols.opcode_sw_flag[i] * opcode_sw
        })
        .collect();

    let is_store: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.opcode_sb_flag[i] + cols.opcode_sh_flag[i] + cols.opcode_sw_flag[i])
        .collect();

    let is_load: Vec<PackedM31> = (0..simd_size).map(|i| enabler[i] - is_store[i]).collect();

    // src_as = 0 * is_store + 1 * is_load = is_load
    // dst_as = 0 * is_load + 1 * is_store = is_store
    let src_as = is_load.clone();
    let dst_as = is_store.clone();

    // dst[3] with sign handling from msb constraint
    let dst_3: Vec<PackedM31> = cols.dst_next_3.to_vec();

    // base[0] - shift_amount alignment check value
    let alignment_check: Vec<PackedM31> = (0..simd_size)
        .map(|i| (cols.rs1_next_0[i] - cols.shift_amount[i]) * quarter_inv * pow2_14)
        .collect();

    let pc_plus_4: Vec<PackedM31> = (0..simd_size).map(|i| cols.pc[i] + four).collect();
    let clk_plus_1: Vec<PackedM31> = (0..simd_size).map(|i| cols.clk[i] + one).collect();
    let clk_minus_rs1_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rs1_clk_prev[i])
        .collect();
    let clk_minus_src_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.src_clk_prev[i])
        .collect();
    let clk_minus_dst_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.dst_clk_prev[i])
        .collect();

    // Numerators
    let neg_enabler: Vec<PackedQM31> = enabler.iter().map(|&e| -PackedQM31::from(e)).collect();
    let pos_enabler: Vec<PackedQM31> = enabler.iter().map(|&e| PackedQM31::from(e)).collect();

    // =====================================================================
    // LogUp entries (same order as AIR)
    // =====================================================================

    // 1. program_access: -enabler * (pc, expected_opcode_id, rs1_addr, r2_idx, imm_felt)
    let program_denom = combine!(
        relations.program_access,
        [
            cols.pc,
            &expected_opcode_id,
            cols.rs1_addr,
            cols.r2_idx,
            cols.imm_felt
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

    // 6. range_check_20: +1 * (clk - rs1_clk_prev) [negation moved to preprocessed side]
    let rc_20_rs1_denom = combine!(relations.range_check_20, [&clk_minus_rs1_clk_prev]);

    write_pair!(
        &pos_enabler,
        &rs1_write_denom,
        &pos_enabler,
        &rc_20_rs1_denom,
        logup_gen
    );

    // 7. range_check_20: +1 * alignment_check [negation moved to preprocessed side]
    let rc_20_align_denom = combine!(relations.range_check_20, [&alignment_check]);

    // 8. range_check_m31: +1 * (base[0], base[3]) [negation moved to preprocessed side]
    let rc_m31_base_denom = combine!(
        relations.range_check_m31,
        [cols.rs1_next_0, cols.rs1_next_3]
    );

    write_pair!(
        &pos_enabler,
        &rc_20_align_denom,
        &pos_enabler,
        &rc_m31_base_denom,
        logup_gen
    );

    // 9. memory_access: -enabler * (src_as, src_addr_selector, src_clk_prev, src_prev_0..3)
    let src_read_denom = combine!(
        relations.memory_access,
        [
            &src_as,
            cols.src_addr_selector,
            cols.src_clk_prev,
            cols.src_prev_0,
            cols.src_prev_1,
            cols.src_prev_2,
            cols.src_prev_3
        ]
    );

    // 10. memory_access: +enabler * (src_as, src_addr_selector, clk, src_next_0..3)
    let src_write_denom = combine!(
        relations.memory_access,
        [
            &src_as,
            cols.src_addr_selector,
            cols.clk,
            cols.src_next_0,
            cols.src_next_1,
            cols.src_next_2,
            cols.src_next_3
        ]
    );

    write_pair!(
        &neg_enabler,
        &src_read_denom,
        &pos_enabler,
        &src_write_denom,
        logup_gen
    );

    // 11. range_check_20: +1 * (clk - src_clk_prev) [negation moved to preprocessed side]
    let rc_20_src_denom = combine!(relations.range_check_20, [&clk_minus_src_clk_prev]);

    // 12. memory_access: -enabler * (dst_as, dst_addr_selector, dst_clk_prev, dst_prev_0..3)
    let dst_read_denom = combine!(
        relations.memory_access,
        [
            &dst_as,
            cols.dst_addr_selector,
            cols.dst_clk_prev,
            cols.dst_prev_0,
            cols.dst_prev_1,
            cols.dst_prev_2,
            cols.dst_prev_3
        ]
    );

    write_pair!(
        &pos_enabler,
        &rc_20_src_denom,
        &neg_enabler,
        &dst_read_denom,
        logup_gen
    );

    // 13. memory_access: +enabler * (dst_as, dst_addr_selector, clk, dst_next_0..3)
    let dst_write_denom = combine!(
        relations.memory_access,
        [
            &dst_as,
            cols.dst_addr_selector,
            cols.clk,
            cols.dst_next_0,
            cols.dst_next_1,
            cols.dst_next_2,
            &dst_3
        ]
    );

    // 14. range_check_20: +1 * (clk - dst_clk_prev) [negation moved to preprocessed side]
    let rc_20_dst_denom = combine!(relations.range_check_20, [&clk_minus_dst_clk_prev]);

    write_pair!(
        &pos_enabler,
        &dst_write_denom,
        &pos_enabler,
        &rc_20_dst_denom,
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

    let cols = LoadStoreColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clk.len();

    let pow2_14 = PackedM31::broadcast(BaseField::from_u32_unchecked(1 << 14));
    let quarter_inv = PackedM31::broadcast(BaseField::from_u32_unchecked(4).inverse());

    // Numerator: enabler (sum of opcode flags)
    let enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.opcode_lb_flag[i]
                + cols.opcode_lh_flag[i]
                + cols.opcode_lbu_flag[i]
                + cols.opcode_lhu_flag[i]
                + cols.opcode_lw_flag[i]
                + cols.opcode_sb_flag[i]
                + cols.opcode_sh_flag[i]
                + cols.opcode_sw_flag[i]
        })
        .collect();

    let clk_minus_rs1_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rs1_clk_prev[i])
        .collect();

    // alignment_check = (rs1_next_0 - shift_amount) * quarter_inv * pow2_14
    let alignment_check: Vec<PackedM31> = (0..simd_size)
        .map(|i| (cols.rs1_next_0[i] - cols.shift_amount[i]) * quarter_inv * pow2_14)
        .collect();

    let clk_minus_src_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.src_clk_prev[i])
        .collect();
    let clk_minus_dst_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.dst_clk_prev[i])
        .collect();

    counters
        .range_check_20
        .register_many(&enabler, &[&clk_minus_rs1_clk_prev]);
    counters
        .range_check_20
        .register_many(&enabler, &[&alignment_check]);

    // Register range_check_m31: (rs1_next_0, rs1_next_3) for base address
    counters
        .range_check_m31
        .register_many(&enabler, &[cols.rs1_next_0, cols.rs1_next_3]);

    counters
        .range_check_20
        .register_many(&enabler, &[&clk_minus_src_clk_prev]);
    counters
        .range_check_20
        .register_many(&enabler, &[&clk_minus_dst_clk_prev]);
}
