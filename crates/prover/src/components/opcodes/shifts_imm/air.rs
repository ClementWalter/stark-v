//! AIR component for Shifts Imm (slli/srli/srai) - airs.md Section 4

use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::ShiftsImmColumns;
use crate::relations::Relations;

pub type Component = FrameworkComponent<Eval>;

#[derive(Clone)]
pub struct Eval {
    pub log_size: u32,
    pub relations: Relations,
}

impl FrameworkEval for Eval {
    fn log_size(&self) -> u32 {
        self.log_size
    }

    fn max_constraint_log_degree_bound(&self) -> u32 {
        self.log_size + 1
    }

    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        let cols = ShiftsImmColumns::from_eval(&mut eval);

        // Section 4.2: Variables
        let enabler = cols.opcode_sll_flag.clone()
            + cols.opcode_srl_flag.clone()
            + cols.opcode_sra_flag.clone();

        let expected_opcode_id = cols.opcode_sll_flag.clone()
            * E::F::from(BaseField::from_u32_unchecked(Opcode::Slli as u32))
            + cols.opcode_srl_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Srli as u32))
            + cols.opcode_sra_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Srai as u32));

        let left_shift = cols.opcode_sll_flag.clone();
        let right_shift = cols.opcode_srl_flag.clone() + cols.opcode_sra_flag.clone();
        let sign_fill = cols.opcode_sra_flag.clone() * cols.rs1_sign.clone();

        let pow2 = |exp: u32| E::F::from(BaseField::from_u32_unchecked(1 << exp));

        let bit_shift_markers = [
            cols.bit_shift_marker_0.clone(),
            cols.bit_shift_marker_1.clone(),
            cols.bit_shift_marker_2.clone(),
            cols.bit_shift_marker_3.clone(),
            cols.bit_shift_marker_4.clone(),
            cols.bit_shift_marker_5.clone(),
            cols.bit_shift_marker_6.clone(),
            cols.bit_shift_marker_7.clone(),
        ];
        let limb_shift_markers = [
            cols.limb_shift_marker_0.clone(),
            cols.limb_shift_marker_1.clone(),
            cols.limb_shift_marker_2.clone(),
            cols.limb_shift_marker_3.clone(),
        ];
        let bit_shift_carry = [
            cols.bit_shift_carry_0.clone(),
            cols.bit_shift_carry_1.clone(),
            cols.bit_shift_carry_2.clone(),
            cols.bit_shift_carry_3.clone(),
        ];
        let rd = [
            cols.rd_next_0.clone(),
            cols.rd_next_1.clone(),
            cols.rd_next_2.clone(),
            cols.rd_next_3.clone(),
        ];
        let rs1_msl = cols.rs1_next_3.clone() + pow2(7) * cols.rs1_sign.clone();
        let rs1 = [
            cols.rs1_next_0.clone(),
            cols.rs1_next_1.clone(),
            cols.rs1_next_2.clone(),
            rs1_msl.clone(),
        ];

        let bit_multiplier = bit_shift_markers
            .iter()
            .enumerate()
            .fold(E::F::zero(), |acc, (i, marker)| {
                acc + pow2(i as u32) * marker.clone()
            });

        let bit_shift =
            bit_shift_markers
                .iter()
                .enumerate()
                .fold(E::F::zero(), |acc, (i, marker)| {
                    acc + E::F::from(BaseField::from_u32_unchecked(i as u32)) * marker.clone()
                });

        let limb_shift =
            limb_shift_markers
                .iter()
                .enumerate()
                .fold(E::F::zero(), |acc, (i, marker)| {
                    acc + E::F::from(BaseField::from_u32_unchecked(i as u32)) * marker.clone()
                });

        let shift_amount = limb_shift.clone() * pow2(3) + bit_shift.clone();

        let two_pow_8 = pow2(8);
        let two_pow_8_minus_one = two_pow_8.clone() - E::F::one();

        let _ = expected_opcode_id;

        // Section 4.3: Constraints

        // enabler, opcode_*_flags and rs1_sign are booleans
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));
        eval.add_constraint(
            cols.opcode_sll_flag.clone() * (E::F::one() - cols.opcode_sll_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_srl_flag.clone() * (E::F::one() - cols.opcode_srl_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_sra_flag.clone() * (E::F::one() - cols.opcode_sra_flag.clone()),
        );
        eval.add_constraint(cols.rs1_sign.clone() * (E::F::one() - cols.rs1_sign.clone()));

        // hot-one encodings must contain at most one activation
        for marker in bit_shift_markers.iter() {
            eval.add_constraint(marker.clone() * (E::F::one() - marker.clone()));
        }
        for marker in limb_shift_markers.iter() {
            eval.add_constraint(marker.clone() * (E::F::one() - marker.clone()));
        }

        let bit_marker_sum = bit_shift_markers
            .iter()
            .fold(E::F::zero(), |acc, marker| acc + marker.clone());
        eval.add_constraint(bit_marker_sum - enabler.clone());

        let limb_marker_sum = limb_shift_markers
            .iter()
            .fold(E::F::zero(), |acc, marker| acc + marker.clone());
        eval.add_constraint(limb_marker_sum - enabler.clone());

        // bit_multiplier are correctly formed
        eval.add_constraint(
            cols.bit_multiplier_left.clone() - left_shift.clone() * bit_multiplier.clone(),
        );
        eval.add_constraint(
            cols.bit_multiplier_right.clone() - right_shift.clone() * bit_multiplier.clone(),
        );

        // imm_truncated shifts limb_shift full limbs and bit_shift bits
        eval.add_constraint(cols.imm_truncated.clone() - shift_amount.clone());

        // left shift constraints
        for i in 0..4 {
            let limb_marker = limb_shift_markers[i].clone();
            for j in 0..4 {
                if j < i {
                    eval.add_constraint(left_shift.clone() * limb_marker.clone() * rd[j].clone());
                } else if j == i {
                    let expr = left_shift.clone()
                        * limb_marker.clone()
                        * (rd[j].clone() + two_pow_8.clone() * bit_shift_carry[j - i].clone())
                        - limb_marker.clone()
                            * rs1[j - i].clone()
                            * cols.bit_multiplier_left.clone();
                    eval.add_constraint(expr);
                } else {
                    let expr = left_shift.clone()
                        * limb_marker.clone()
                        * (rd[j].clone()
                            - (bit_shift_carry[j - i - 1].clone()
                                - two_pow_8.clone() * bit_shift_carry[j - i].clone()))
                        - limb_marker.clone()
                            * rs1[j - i].clone()
                            * cols.bit_multiplier_left.clone();
                    eval.add_constraint(expr);
                }
            }
        }

        // right shift constraints
        for i in 0..4 {
            let limb_marker = limb_shift_markers[i].clone();
            for j in 0..4 {
                if j > 3 - i {
                    eval.add_constraint(
                        right_shift.clone()
                            * limb_marker.clone()
                            * (rd[j].clone() - sign_fill.clone() * two_pow_8_minus_one.clone()),
                    );
                } else if j == 3 - i {
                    let expr = limb_marker.clone()
                        * (sign_fill.clone()
                            * (cols.bit_multiplier_right.clone() - E::F::one())
                            * two_pow_8.clone()
                            + right_shift.clone()
                                * (rs1[j + i].clone() - bit_shift_carry[j + i].clone())
                            - rd[j].clone() * cols.bit_multiplier_right.clone());
                    eval.add_constraint(expr);
                } else {
                    let expr = limb_marker.clone()
                        * (bit_shift_carry[j + i + 1].clone()
                            * right_shift.clone()
                            * two_pow_8.clone()
                            + right_shift.clone()
                                * (rs1[j + i].clone() - bit_shift_carry[j + i].clone())
                            - rd[j].clone() * cols.bit_multiplier_right.clone());
                    eval.add_constraint(expr);
                }
            }
        }

        eval
    }
}
