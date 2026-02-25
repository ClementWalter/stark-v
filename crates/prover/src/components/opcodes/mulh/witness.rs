//! Witness generation for mulh component.

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

use super::columns::MulhColumns;

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

    let cols = MulhColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clk.len();

    let log_size = trace[0].domain.log_size();
    let mut logup_gen = LogupTraceGenerator::new(log_size);

    // Constants
    let zero = PackedM31::zero();
    let one = PackedM31::broadcast(BaseField::one());
    let four = PackedM31::broadcast(BaseField::from_u32_unchecked(4));
    let pow2_7 = PackedM31::broadcast(BaseField::from_u32_unchecked(128));
    let pow2_8 = PackedM31::broadcast(BaseField::from_u32_unchecked(256));
    let sign_ext = pow2_8 - one;
    let inv_two_pow_8 = PackedM31::broadcast(BaseField::from_u32_unchecked(1 << 8).inverse());

    let opcode_mulh = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Mulh as u32));
    let opcode_mulhsu = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Mulhsu as u32));
    let opcode_mulhu = PackedM31::broadcast(BaseField::from_u32_unchecked(Opcode::Mulhu as u32));

    let zero_col: Vec<PackedM31> = vec![zero; simd_size];

    // Compute derived columns
    let enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.opcode_mulh_flag[i] + cols.opcode_mulhsu_flag[i] + cols.opcode_mulhu_flag[i])
        .collect();

    let expected_opcode_id: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            cols.opcode_mulh_flag[i] * opcode_mulh
                + cols.opcode_mulhsu_flag[i] * opcode_mulhsu
                + cols.opcode_mulhu_flag[i] * opcode_mulhu
        })
        .collect();

    // Sign-extended rs1 and rs2 with sign bit
    let rs1_3_with_sign: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.rs1_next_3[i] + cols.rs1_sign[i] * pow2_7)
        .collect();
    let rs2_3_with_sign: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.rs2_next_3[i] + cols.rs2_sign[i] * pow2_7)
        .collect();

    // Compute carries for 8-byte multiplication (simplified - we track 8 carries)
    // For carry computation, we build extended operands
    let rs1_ext: [Vec<PackedM31>; 8] = std::array::from_fn(|idx| {
        (0..simd_size)
            .map(|i| {
                if idx == 0 {
                    cols.rs1_next_0[i]
                } else if idx == 1 {
                    cols.rs1_next_1[i]
                } else if idx == 2 {
                    cols.rs1_next_2[i]
                } else if idx == 3 {
                    rs1_3_with_sign[i]
                } else {
                    cols.rs1_sign[i] * sign_ext
                }
            })
            .collect()
    });
    let rs2_ext: [Vec<PackedM31>; 8] = std::array::from_fn(|idx| {
        (0..simd_size)
            .map(|i| {
                if idx == 0 {
                    cols.rs2_next_0[i]
                } else if idx == 1 {
                    cols.rs2_next_1[i]
                } else if idx == 2 {
                    cols.rs2_next_2[i]
                } else if idx == 3 {
                    rs2_3_with_sign[i]
                } else {
                    cols.rs2_sign[i] * sign_ext
                }
            })
            .collect()
    });
    let rd_full: [Vec<PackedM31>; 8] = std::array::from_fn(|idx| {
        (0..simd_size)
            .map(|i| match idx {
                0 => cols.rd_high_0[i],
                1 => cols.rd_high_1[i],
                2 => cols.rd_high_2[i],
                3 => cols.rd_high_3[i],
                4 => cols.rd_next_0[i],
                5 => cols.rd_next_1[i],
                6 => cols.rd_next_2[i],
                7 => cols.rd_next_3[i],
                _ => zero,
            })
            .collect()
    });

    // Compute carries
    let mut carry: [Vec<PackedM31>; 8] = std::array::from_fn(|_| vec![zero; simd_size]);
    for idx in 0..8 {
        for i in 0..simd_size {
            let prev_carry = if idx == 0 { zero } else { carry[idx - 1][i] };
            let mut limb_sum = prev_carry;
            for k in 0..=idx.min(7) {
                if idx - k < 8 {
                    limb_sum += rs1_ext[k][i] * rs2_ext[idx - k][i];
                }
            }
            carry[idx][i] = (limb_sum - rd_full[idx][i]) * inv_two_pow_8;
        }
    }

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
        &neg_enabler,
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

    // 10. range_check_8_8: -1 * (carry[0], carry[1])
    let rc_8_8_carry_0_denom = combine!(relations.range_check_8_8, [&carry[0], &carry[1]]);

    // 11. range_check_8_8: -1 * (carry[2], carry[3])
    let rc_8_8_carry_1_denom = combine!(relations.range_check_8_8, [&carry[2], &carry[3]]);

    // 12. range_check_8_8: -1 * (carry[4], carry[5])
    let rc_8_8_carry_2_denom = combine!(relations.range_check_8_8, [&carry[4], &carry[5]]);

    // 13. range_check_8_8: -1 * (carry[6], carry[7])
    let rc_8_8_carry_3_denom = combine!(relations.range_check_8_8, [&carry[6], &carry[7]]);

    // 14. range_check_8_8: -1 * (rd_low[0], rd_low[1])
    let rc_8_8_rd_low_0_denom =
        combine!(relations.range_check_8_8, [cols.rd_high_0, cols.rd_high_1]);

    write_pair!(
        &neg_enabler,
        &rc_20_rs2_denom,
        &neg_enabler,
        &rc_8_8_rd_low_0_denom,
        logup_gen
    );
    write_single_batch(&mut logup_gen, &neg_enabler, &rc_8_8_carry_0_denom);
    write_single_batch(&mut logup_gen, &neg_enabler, &rc_8_8_carry_1_denom);
    write_single_batch(&mut logup_gen, &neg_enabler, &rc_8_8_carry_2_denom);
    write_single_batch(&mut logup_gen, &neg_enabler, &rc_8_8_carry_3_denom);

    // 15. range_check_8_8: -1 * (rd_low[2], rd_low[3])
    let rc_8_8_rd_low_1_denom =
        combine!(relations.range_check_8_8, [cols.rd_high_2, cols.rd_high_3]);

    // 16. range_check_8_8: -1 * (rd_high[0], rd_high[1])
    let rc_8_8_rd_high_0_denom =
        combine!(relations.range_check_8_8, [cols.rd_next_0, cols.rd_next_1]);

    write_pair!(
        &neg_enabler,
        &rc_8_8_rd_low_1_denom,
        &neg_enabler,
        &rc_8_8_rd_high_0_denom,
        logup_gen
    );

    // 17. range_check_8_8: -1 * (rd_high[2], rd_high[3])
    let rc_8_8_rd_high_1_denom =
        combine!(relations.range_check_8_8, [cols.rd_next_2, cols.rd_next_3]);

    // 18. memory_access: -enabler * (0, rd_addr, rd_clk_prev, rd_prev_0..3)
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

    // 19. memory_access: +enabler * (0, rd_addr, clk, rd_next_0..3 = rd_high)
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

    // 20. range_check_20: -1 * (clk - rd_clk_prev)
    let rc_20_rd_denom = combine!(relations.range_check_20, [&clk_minus_rd_clk_prev]);

    write_pair!(
        &neg_enabler,
        &rc_8_8_rd_high_1_denom,
        &neg_enabler,
        &rd_read_denom,
        logup_gen
    );
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

    let cols = MulhColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.clk.len();

    let zero = PackedM31::zero();
    let one = PackedM31::broadcast(BaseField::one());
    let pow2_7 = PackedM31::broadcast(BaseField::from_u32_unchecked(128));
    let pow2_8 = PackedM31::broadcast(BaseField::from_u32_unchecked(256));
    let sign_ext = pow2_8 - one;
    let inv_two_pow_8 = PackedM31::broadcast(BaseField::from_u32_unchecked(1 << 8).inverse());

    // Numerator: negated enabler (to match gen_interaction_trace)
    let neg_enabler: Vec<PackedM31> = (0..simd_size)
        .map(|i| {
            -(cols.opcode_mulh_flag[i] + cols.opcode_mulhsu_flag[i] + cols.opcode_mulhu_flag[i])
        })
        .collect();

    // Clock differences
    let clk_minus_rs1_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rs1_clk_prev[i])
        .collect();
    let clk_minus_rs2_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rs2_clk_prev[i])
        .collect();
    let clk_minus_rd_clk_prev: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.clk[i] - cols.rd_clk_prev[i])
        .collect();

    // Sign-extended rs1 and rs2 with sign bit
    let rs1_3_with_sign: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.rs1_next_3[i] + cols.rs1_sign[i] * pow2_7)
        .collect();
    let rs2_3_with_sign: Vec<PackedM31> = (0..simd_size)
        .map(|i| cols.rs2_next_3[i] + cols.rs2_sign[i] * pow2_7)
        .collect();

    // Build extended operands (same as gen_interaction_trace)
    let rs1_ext: [Vec<PackedM31>; 8] = std::array::from_fn(|idx| {
        (0..simd_size)
            .map(|i| {
                if idx == 0 {
                    cols.rs1_next_0[i]
                } else if idx == 1 {
                    cols.rs1_next_1[i]
                } else if idx == 2 {
                    cols.rs1_next_2[i]
                } else if idx == 3 {
                    rs1_3_with_sign[i]
                } else {
                    cols.rs1_sign[i] * sign_ext
                }
            })
            .collect()
    });
    let rs2_ext: [Vec<PackedM31>; 8] = std::array::from_fn(|idx| {
        (0..simd_size)
            .map(|i| {
                if idx == 0 {
                    cols.rs2_next_0[i]
                } else if idx == 1 {
                    cols.rs2_next_1[i]
                } else if idx == 2 {
                    cols.rs2_next_2[i]
                } else if idx == 3 {
                    rs2_3_with_sign[i]
                } else {
                    cols.rs2_sign[i] * sign_ext
                }
            })
            .collect()
    });
    let rd_full: [Vec<PackedM31>; 8] = std::array::from_fn(|idx| {
        (0..simd_size)
            .map(|i| match idx {
                0 => cols.rd_high_0[i],
                1 => cols.rd_high_1[i],
                2 => cols.rd_high_2[i],
                3 => cols.rd_high_3[i],
                4 => cols.rd_next_0[i],
                5 => cols.rd_next_1[i],
                6 => cols.rd_next_2[i],
                7 => cols.rd_next_3[i],
                _ => zero,
            })
            .collect()
    });

    // Compute carries (same as gen_interaction_trace)
    let mut carry: [Vec<PackedM31>; 8] = std::array::from_fn(|_| vec![zero; simd_size]);
    for idx in 0..8 {
        for i in 0..simd_size {
            let prev_carry = if idx == 0 { zero } else { carry[idx - 1][i] };
            let mut limb_sum = prev_carry;
            for k in 0..=idx.min(7) {
                if idx - k < 8 {
                    limb_sum += rs1_ext[k][i] * rs2_ext[idx - k][i];
                }
            }
            carry[idx][i] = (limb_sum - rd_full[idx][i]) * inv_two_pow_8;
        }
    }

    // Register range_check_20 for clock diffs with negated multiplicity
    counters
        .range_check_20
        .register_many(&neg_enabler, &[&clk_minus_rs1_clk_prev]);
    counters
        .range_check_20
        .register_many(&neg_enabler, &[&clk_minus_rs2_clk_prev]);

    // Register range_check_8_8 for carries with negated multiplicity
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[&carry[0], &carry[1]]);
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[&carry[2], &carry[3]]);
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[&carry[4], &carry[5]]);
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[&carry[6], &carry[7]]);

    // Register range_check_8_8 for rd_low limbs (rd_high_0..3) with negated multiplicity
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[cols.rd_high_0, cols.rd_high_1]);
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[cols.rd_high_2, cols.rd_high_3]);

    // Register range_check_8_8 for rd_high limbs (rd_next_0..3) with negated multiplicity
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[cols.rd_next_0, cols.rd_next_1]);
    counters
        .range_check_8_8
        .register_many(&neg_enabler, &[cols.rd_next_2, cols.rd_next_3]);

    // Register range_check_20 for rd clock diff with negated multiplicity
    counters
        .range_check_20
        .register_many(&neg_enabler, &[&clk_minus_rd_clk_prev]);
}
