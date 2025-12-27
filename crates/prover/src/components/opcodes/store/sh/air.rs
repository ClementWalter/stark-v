//! AIR component for SH opcode (dummy constraints).

use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use crate::components::opcodes::store::sh::columns::ShColumns;
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
        let cols = ShColumns::from_eval(&mut eval);

        // DUMMY CONSTRAINT: clk - clk = 0 (always satisfied)
        eval.add_constraint(cols.clk.clone() - cols.clk.clone());

        // NOTE: finalize_logup() is not called because there are no logup relations
        // in the dummy constraints. When real constraints are implemented,
        // logup relations should be added and finalize_logup() should be called.
        eval
    }
}
