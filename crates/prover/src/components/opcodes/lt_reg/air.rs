//! AIR component for Less Than Reg (slt/sltu) - airs.md Section 5

use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::LtRegColumns;
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
        let cols = LtRegColumns::from_eval(&mut eval);

        // Section 5.2: Variables
        let enabler = cols.opcode_slt_flag.clone() + cols.opcode_sltu_flag.clone();
        let expected_opcode_id = cols.opcode_slt_flag.clone()
            * E::F::from(BaseField::from_u32_unchecked(Opcode::Slt as u32))
            + cols.opcode_sltu_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Sltu as u32));

        let diff_markers = [
            cols.diff_marker_0.clone(),
            cols.diff_marker_1.clone(),
            cols.diff_marker_2.clone(),
            cols.diff_marker_3.clone(),
        ];
        let rs1 = [
            cols.rs1_next_0.clone(),
            cols.rs1_next_1.clone(),
            cols.rs1_next_2.clone(),
            cols.rs1_next_3.clone(),
        ];
        let rs2 = [
            cols.rs2_next_0.clone(),
            cols.rs2_next_1.clone(),
            cols.rs2_next_2.clone(),
            cols.rs2_next_3.clone(),
        ];

        let two_pow_8 = E::F::from(BaseField::from_u32_unchecked(1 << 8));
        let two = E::F::one() + E::F::one();

        let _ = expected_opcode_id;

        // Section 5.3: Constraints

        // enabler, opcode_*_flags and diff_markers are booleans
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));
        eval.add_constraint(
            cols.opcode_slt_flag.clone() * (E::F::one() - cols.opcode_slt_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_sltu_flag.clone() * (E::F::one() - cols.opcode_sltu_flag.clone()),
        );

        for marker in diff_markers.iter() {
            eval.add_constraint(marker.clone() * (E::F::one() - marker.clone()));
        }

        // msl are the most significant limbs as felts
        let rs1_msl_gap = rs1[3].clone() - cols.rs1_msl_felt.clone();
        eval.add_constraint(rs1_msl_gap.clone() * (two_pow_8.clone() - rs1_msl_gap));

        let rs2_msl_gap = rs2[3].clone() - cols.rs2_msl_felt.clone();
        eval.add_constraint(rs2_msl_gap.clone() * (two_pow_8.clone() - rs2_msl_gap));

        // comparison logic
        let mut prefix_sum = E::F::zero();
        for (i, marker) in diff_markers.iter().enumerate().rev() {
            let limb_diff = if i == 3 {
                cols.rs2_msl_felt.clone() - cols.rs1_msl_felt.clone()
            } else {
                rs2[i].clone() - rs1[i].clone()
            };
            let diff = (two.clone() * cols.cmp_result.clone() - E::F::one()) * limb_diff;

            prefix_sum += marker.clone();
            eval.add_constraint((E::F::one() - prefix_sum.clone()) * diff.clone());
            eval.add_constraint(marker.clone() * (cols.diff_val.clone() - diff));
        }

        // prefix_sum contains at most one activation
        eval.add_constraint(prefix_sum.clone() * (E::F::one() - prefix_sum.clone()));

        // if equal, result is 0
        eval.add_constraint((E::F::one() - prefix_sum) * cols.cmp_result.clone());

        eval
    }
}
