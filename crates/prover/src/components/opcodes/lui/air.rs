//! AIR component for LUI - airs.md Section 9

use num_traits::One;
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
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

        // Section 9.2: Variables
        let imm = cols.imm_0.clone()
            + E::F::from(BaseField::from_u32_unchecked(1 << 4)) * cols.imm_1.clone()
            + E::F::from(BaseField::from_u32_unchecked(1 << 12)) * cols.imm_2.clone();
        let opcode_lui_id = E::F::from(BaseField::from_u32_unchecked(Opcode::Lui as u32));
        let _ = (imm, opcode_lui_id);

        // enabler is boolean (single opcode family)
        eval.add_constraint(cols.enabler.clone() * (E::F::one() - cols.enabler.clone()));

        eval
    }
}
