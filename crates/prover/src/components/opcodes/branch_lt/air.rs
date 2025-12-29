//! AIR component for Branch Less Than (blt/bltu/bge/bgeu) - airs.md Section 8

use num_traits::One;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::Branch_ltColumns;
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
        let cols = Branch_ltColumns::from_eval(&mut eval);

        let enabler = cols.opcode_blt_flag.clone()
            + cols.opcode_bltu_flag.clone()
            + cols.opcode_bge_flag.clone()
            + cols.opcode_bgeu_flag.clone();

        // Boolean constraints
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));
        eval.add_constraint(
            cols.opcode_blt_flag.clone() * (E::F::one() - cols.opcode_blt_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_bltu_flag.clone() * (E::F::one() - cols.opcode_bltu_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_bge_flag.clone() * (E::F::one() - cols.opcode_bge_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_bgeu_flag.clone() * (E::F::one() - cols.opcode_bgeu_flag.clone()),
        );
        eval.add_constraint(cols.cmp_result.clone() * (E::F::one() - cols.cmp_result.clone()));

        // diff_marker[i] are booleans
        eval.add_constraint(
            cols.diff_marker_0.clone() * (E::F::one() - cols.diff_marker_0.clone()),
        );
        eval.add_constraint(
            cols.diff_marker_1.clone() * (E::F::one() - cols.diff_marker_1.clone()),
        );
        eval.add_constraint(
            cols.diff_marker_2.clone() * (E::F::one() - cols.diff_marker_2.clone()),
        );
        eval.add_constraint(
            cols.diff_marker_3.clone() * (E::F::one() - cols.diff_marker_3.clone()),
        );

        eval
    }
}
