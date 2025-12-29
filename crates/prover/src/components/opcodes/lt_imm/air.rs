//! AIR component for Less Than Imm (slti/sltiu) - airs.md Section 6

use num_traits::One;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::Lt_immColumns;
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
        let cols = Lt_immColumns::from_eval(&mut eval);

        let enabler = cols.opcode_slti_flag.clone() + cols.opcode_sltiu_flag.clone();

        // Boolean constraints
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));
        eval.add_constraint(
            cols.opcode_slti_flag.clone() * (E::F::one() - cols.opcode_slti_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_sltiu_flag.clone() * (E::F::one() - cols.opcode_sltiu_flag.clone()),
        );
        eval.add_constraint(cols.imm_msb.clone() * (E::F::one() - cols.imm_msb.clone()));


        eval
    }
}
