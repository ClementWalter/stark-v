//! AIR component for LUI - airs.md Section 9

use num_traits::One;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::LuiColumns;
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
        let cols = LuiColumns::from_eval(&mut eval);

        // enabler is boolean (single opcode family)
        eval.add_constraint(cols.enabler.clone() * (E::F::one() - cols.enabler.clone()));


        eval
    }
}
