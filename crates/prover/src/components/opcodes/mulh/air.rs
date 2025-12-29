//! AIR component for MULH (mulh/mulhsu/mulhu) - airs.md Section 15

use num_traits::One;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::MulhColumns;
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
        let cols = MulhColumns::from_eval(&mut eval);

        let enabler = cols.opcode_mulh_flag.clone()
            + cols.opcode_mulhsu_flag.clone()
            + cols.opcode_mulhu_flag.clone();

        // Boolean constraints
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));
        eval.add_constraint(
            cols.opcode_mulh_flag.clone() * (E::F::one() - cols.opcode_mulh_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_mulhsu_flag.clone() * (E::F::one() - cols.opcode_mulhsu_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_mulhu_flag.clone() * (E::F::one() - cols.opcode_mulhu_flag.clone()),
        );
        eval.add_constraint(cols.rs1_sign.clone() * (E::F::one() - cols.rs1_sign.clone()));
        eval.add_constraint(cols.rs2_sign.clone() * (E::F::one() - cols.rs2_sign.clone()));


        eval
    }
}
