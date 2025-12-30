//! AIR component for Branch Equal (beq/bne) - airs.md Section 7

use num_traits::One;
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::BranchEqColumns;
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
        let cols = BranchEqColumns::from_eval(&mut eval);

        // Section 7.2: Variables
        let enabler = cols.opcode_beq_flag.clone() + cols.opcode_bne_flag.clone();
        let expected_opcode_id = cols.opcode_beq_flag.clone()
            * E::F::from(BaseField::from_u32_unchecked(Opcode::Beq as u32))
            + cols.opcode_bne_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Bne as u32));

        let cmp_eq = cols.cmp_result.clone() * cols.opcode_beq_flag.clone()
            + (E::F::one() - cols.cmp_result.clone()) * cols.opcode_bne_flag.clone();

        let diff_inv_markers = [
            cols.diff_inv_marker_0.clone(),
            cols.diff_inv_marker_1.clone(),
            cols.diff_inv_marker_2.clone(),
            cols.diff_inv_marker_3.clone(),
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

        let diff_inv_sum = diff_inv_markers
            .iter()
            .zip(rs1.iter().zip(rs2.iter()))
            .fold(cmp_eq.clone(), |acc, (marker, (a, b))| {
                acc + (a.clone() - b.clone()) * marker.clone()
            });

        let _ = expected_opcode_id;

        // Section 7.3: Constraints

        // enabler, opcode_*_flags and cmp_result are booleans
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));
        eval.add_constraint(
            cols.opcode_beq_flag.clone() * (E::F::one() - cols.opcode_beq_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_bne_flag.clone() * (E::F::one() - cols.opcode_bne_flag.clone()),
        );
        eval.add_constraint(cols.cmp_result.clone() * (E::F::one() - cols.cmp_result.clone()));

        // check cmp_eq
        for (a, b) in rs1.iter().zip(rs2.iter()) {
            eval.add_constraint(cmp_eq.clone() * (a.clone() - b.clone()));
        }
        eval.add_constraint(enabler.clone() * (E::F::one() - diff_inv_sum));

        eval
    }
}
