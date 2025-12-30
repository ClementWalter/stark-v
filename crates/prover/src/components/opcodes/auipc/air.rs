//! AIR component for AUIPC - airs.md Section 10

use num_traits::One;
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::AuipcColumns;
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
        let cols = AuipcColumns::from_eval(&mut eval);

        // Section 10.2: Variables
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
        let opcode_auipc_id = E::F::from(BaseField::from_u32_unchecked(Opcode::Auipc as u32));
        let _ = opcode_auipc_id;

        // enabler is boolean (single opcode family)
        eval.add_constraint(cols.enabler.clone() * (E::F::one() - cols.enabler.clone()));

        // check that rd is pc + imm
        eval.add_constraint(rd_felt - (cols.pc.clone() + cols.imm_felt.clone()));

        eval
    }
}
