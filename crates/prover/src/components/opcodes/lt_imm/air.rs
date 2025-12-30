//! AIR component for Less Than Imm (slti/sltiu) - airs.md Section 6

use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::LtImmColumns;
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
        let cols = LtImmColumns::from_eval(&mut eval);

        // Section 6.2: Variables
        let enabler = cols.opcode_slti_flag.clone() + cols.opcode_sltiu_flag.clone();
        let expected_opcode_id = cols.opcode_slti_flag.clone()
            * E::F::from(BaseField::from_u32_unchecked(Opcode::Slti as u32))
            + cols.opcode_sltiu_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Sltiu as u32));

        let pow2 = |exp: u32| E::F::from(BaseField::from_u32_unchecked(1 << exp));

        let imm =
            cols.imm_0.clone() + pow2(8) * cols.imm_1.clone() + pow2(11) * cols.imm_msb.clone();
        let sext_imm_0 = cols.imm_0.clone();
        let sext_imm_1 = cols.imm_1.clone()
            + E::F::from(BaseField::from_u32_unchecked(1 << 3))
                * E::F::from(BaseField::from_u32_unchecked((1 << 5) - 1))
                * cols.imm_msb.clone();
        let sext_imm_2 =
            E::F::from(BaseField::from_u32_unchecked((1 << 8) - 1)) * cols.imm_msb.clone();
        let sext_imm_3 = sext_imm_2.clone();
        let sext_imm = [
            sext_imm_0.clone(),
            sext_imm_1.clone(),
            sext_imm_2.clone(),
            sext_imm_3.clone(),
        ];

        let sext_imm_msl_felt = cols.opcode_sltiu_flag.clone() * sext_imm_3.clone()
            - cols.opcode_slti_flag.clone() * cols.imm_msb.clone();

        let rs1 = [
            cols.rs1_next_0.clone(),
            cols.rs1_next_1.clone(),
            cols.rs1_next_2.clone(),
            cols.rs1_next_3.clone(),
        ];
        let diff_markers = [
            cols.diff_marker_0.clone(),
            cols.diff_marker_1.clone(),
            cols.diff_marker_2.clone(),
            cols.diff_marker_3.clone(),
        ];

        let two_pow_8 = E::F::from(BaseField::from_u32_unchecked(1 << 8));
        let two = E::F::one() + E::F::one();

        let _ = (expected_opcode_id, imm);

        // Section 6.3: Constraints

        // enabler and opcode flags are booleans
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));
        eval.add_constraint(
            cols.opcode_slti_flag.clone() * (E::F::one() - cols.opcode_slti_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_sltiu_flag.clone() * (E::F::one() - cols.opcode_sltiu_flag.clone()),
        );

        // imm_msb is boolean
        eval.add_constraint(cols.imm_msb.clone() * (E::F::one() - cols.imm_msb.clone()));

        // msl are the most significant limbs as felts
        let rs1_msl_gap = rs1[3].clone() - cols.rs1_msl_felt.clone();
        eval.add_constraint(rs1_msl_gap.clone() * (two_pow_8.clone() - rs1_msl_gap));

        // diff markers are boolean and sum correctly
        for marker in diff_markers.iter() {
            eval.add_constraint(marker.clone() * (E::F::one() - marker.clone()));
        }

        let mut prefix_sum = E::F::zero();
        for (i, marker) in diff_markers.iter().enumerate().rev() {
            let limb_diff = if i == 3 {
                sext_imm_msl_felt.clone() - cols.rs1_msl_felt.clone()
            } else {
                sext_imm[i].clone() - rs1[i].clone()
            };
            let diff = (two.clone() * cols.cmp_result.clone() - E::F::one()) * limb_diff;

            prefix_sum += marker.clone();
            eval.add_constraint((E::F::one() - prefix_sum.clone()) * diff.clone());
            eval.add_constraint(marker.clone() * (cols.diff_val.clone() - diff));
        }

        // prefix_sum contains at most one activation
        eval.add_constraint(prefix_sum.clone() * (E::F::one() - prefix_sum.clone()));

        // if equal, result is 0
        eval.add_constraint((E::F::one() - prefix_sum.clone()) * cols.cmp_result.clone());

        // result is boolean
        eval.add_constraint(cols.cmp_result.clone() * (E::F::one() - cols.cmp_result.clone()));

        eval
    }
}
