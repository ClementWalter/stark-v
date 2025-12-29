//! AIR component for Base ALU Reg (add/sub/xor/or/and) - airs.md Section 1

use num_traits::One;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::Base_alu_regColumns;
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
        let cols = Base_alu_regColumns::from_eval(&mut eval);

        // Section 1.2: Variables
        let enabler = cols.opcode_add_flag.clone()
            + cols.opcode_sub_flag.clone()
            + cols.opcode_xor_flag.clone()
            + cols.opcode_or_flag.clone()
            + cols.opcode_and_flag.clone();

        // Section 1.3: Constraints

        // enabler is boolean
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));

        // opcode flags are booleans
        eval.add_constraint(
            cols.opcode_add_flag.clone() * (E::F::one() - cols.opcode_add_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_sub_flag.clone() * (E::F::one() - cols.opcode_sub_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_xor_flag.clone() * (E::F::one() - cols.opcode_xor_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_or_flag.clone() * (E::F::one() - cols.opcode_or_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_and_flag.clone() * (E::F::one() - cols.opcode_and_flag.clone()),
        );

        // Note: Lookups (Program, Memory, RC, Bitwise) are not implemented yet.
        // The constraints above establish the basic structure.

        eval
    }
}
