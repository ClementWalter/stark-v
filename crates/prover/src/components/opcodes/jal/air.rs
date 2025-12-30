//! AIR component for JAL - airs.md Section 12

use num_traits::One;
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::JalColumns;
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
        let cols = JalColumns::from_eval(&mut eval);

        // Section 12.2: Variables
        let rd = [
            cols.rd_next_0.clone(),
            cols.rd_next_1.clone(),
            cols.rd_next_2.clone(),
            cols.rd_next_3.clone(),
        ];
        let rd_felt = rd[0].clone()
            + E::F::from(BaseField::from_u32_unchecked(1 << 8)) * rd[1].clone()
            + E::F::from(BaseField::from_u32_unchecked(1 << 16)) * rd[2].clone()
            + E::F::from(BaseField::from_u32_unchecked(1 << 24)) * rd[3].clone();

        let opcode_jal_id = E::F::from(BaseField::from_u32_unchecked(Opcode::Jal as u32));
        let _ = opcode_jal_id;

        // enabler is boolean
        eval.add_constraint(cols.enabler.clone() * (E::F::one() - cols.enabler.clone()));

        // rd is pc + 4 (gated by enabler for padding rows and rd_addr for x0 writes)
        // When rd_addr = 0 (x0), the write is discarded and rd_next = 0, so skip this constraint
        eval.add_constraint(
            cols.enabler.clone()
                * cols.rd_addr.clone()
                * (rd_felt - (cols.pc.clone() + E::F::from(BaseField::from_u32_unchecked(4)))),
        );

        eval
    }
}
