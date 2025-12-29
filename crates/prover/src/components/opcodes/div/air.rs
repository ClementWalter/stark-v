//! AIR component for DIV (div/divu/rem/remu) - airs.md Section 16

use num_traits::One;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::DivColumns;
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
        let cols = DivColumns::from_eval(&mut eval);

        let enabler = cols.opcode_div_flag.clone()
            + cols.opcode_divu_flag.clone()
            + cols.opcode_rem_flag.clone()
            + cols.opcode_remu_flag.clone();

        // Boolean constraints
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));
        eval.add_constraint(
            cols.opcode_div_flag.clone() * (E::F::one() - cols.opcode_div_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_divu_flag.clone() * (E::F::one() - cols.opcode_divu_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_rem_flag.clone() * (E::F::one() - cols.opcode_rem_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_remu_flag.clone() * (E::F::one() - cols.opcode_remu_flag.clone()),
        );
        eval.add_constraint(cols.zero_divisor.clone() * (E::F::one() - cols.zero_divisor.clone()));
        eval.add_constraint(cols.r_zero.clone() * (E::F::one() - cols.r_zero.clone()));
        eval.add_constraint(cols.b_sign.clone() * (E::F::one() - cols.b_sign.clone()));
        eval.add_constraint(cols.c_sign.clone() * (E::F::one() - cols.c_sign.clone()));
        eval.add_constraint(cols.q_sign.clone() * (E::F::one() - cols.q_sign.clone()));
        eval.add_constraint(cols.sign_xor.clone() * (E::F::one() - cols.sign_xor.clone()));

        // lt_marker[i] are booleans
        eval.add_constraint(cols.lt_marker_0.clone() * (E::F::one() - cols.lt_marker_0.clone()));
        eval.add_constraint(cols.lt_marker_1.clone() * (E::F::one() - cols.lt_marker_1.clone()));
        eval.add_constraint(cols.lt_marker_2.clone() * (E::F::one() - cols.lt_marker_2.clone()));
        eval.add_constraint(cols.lt_marker_3.clone() * (E::F::one() - cols.lt_marker_3.clone()));

        eval
    }
}
