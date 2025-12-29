//! AIR component for Branch Equal (beq/bne) - airs.md Section 7

use num_traits::One;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::Branch_eqColumns;
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
        let cols = Branch_eqColumns::from_eval(&mut eval);

        let enabler = cols.opcode_beq_flag.clone() + cols.opcode_bne_flag.clone();

        // Boolean constraints
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));
        eval.add_constraint(
            cols.opcode_beq_flag.clone() * (E::F::one() - cols.opcode_beq_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_bne_flag.clone() * (E::F::one() - cols.opcode_bne_flag.clone()),
        );
        eval.add_constraint(cols.cmp_result.clone() * (E::F::one() - cols.cmp_result.clone()));

        eval
    }
}
