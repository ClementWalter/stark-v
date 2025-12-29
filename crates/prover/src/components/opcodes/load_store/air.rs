//! AIR component for Load/Store (lb/lbu/lh/lhu/lw/sb/sh/sw) - airs.md Section 13

use num_traits::One;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::Load_storeColumns;
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
        let cols = Load_storeColumns::from_eval(&mut eval);

        let enabler = cols.opcode_lb_flag.clone()
            + cols.opcode_lh_flag.clone()
            + cols.opcode_lbu_flag.clone()
            + cols.opcode_lhu_flag.clone()
            + cols.opcode_lw_flag.clone()
            + cols.opcode_sb_flag.clone()
            + cols.opcode_sh_flag.clone()
            + cols.opcode_sw_flag.clone();

        // Boolean constraints for enabler and opcode flags
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));
        eval.add_constraint(
            cols.opcode_lb_flag.clone() * (E::F::one() - cols.opcode_lb_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_lh_flag.clone() * (E::F::one() - cols.opcode_lh_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_lbu_flag.clone() * (E::F::one() - cols.opcode_lbu_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_lhu_flag.clone() * (E::F::one() - cols.opcode_lhu_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_lw_flag.clone() * (E::F::one() - cols.opcode_lw_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_sb_flag.clone() * (E::F::one() - cols.opcode_sb_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_sh_flag.clone() * (E::F::one() - cols.opcode_sh_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_sw_flag.clone() * (E::F::one() - cols.opcode_sw_flag.clone()),
        );

        // marker[i] are booleans
        eval.add_constraint(cols.marker_0.clone() * (E::F::one() - cols.marker_0.clone()));
        eval.add_constraint(cols.marker_1.clone() * (E::F::one() - cols.marker_1.clone()));
        eval.add_constraint(cols.marker_2.clone() * (E::F::one() - cols.marker_2.clone()));
        eval.add_constraint(cols.marker_3.clone() * (E::F::one() - cols.marker_3.clone()));

        eval
    }
}
