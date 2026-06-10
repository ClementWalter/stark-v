//! AIR component for LUI - airs.md Section 9

use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use crate::relations::Relations;
use runner::trace::prover_columns::LuiColumns;

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

        // Constraints and LogUp entries both come from define_trace_tables!
        // (airs.md Sections 9.2/9.3).
        for constraint in cols.constraints() {
            eval.add_constraint(constraint);
        }
        runner::lui_lookups!(eval, cols, self.relations);
        eval
    }
}
