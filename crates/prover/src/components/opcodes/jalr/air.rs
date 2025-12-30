//! AIR component for JALR - airs.md Section 11

use num_traits::One;
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::JalrColumns;
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
        let cols = JalrColumns::from_eval(&mut eval);

        // Section 11.2: Variables
        let rs1 = [
            cols.rs1_next_0.clone(),
            cols.rs1_next_1.clone(),
            cols.rs1_next_2.clone(),
            cols.rs1_next_3.clone(),
        ];
        let rd = [
            cols.rd_next_0.clone(),
            cols.rd_next_1.clone(),
            cols.rd_next_2.clone(),
            cols.rd_next_3.clone(),
        ];

        let rs1_felt = rs1[0].clone()
            + E::F::from(BaseField::from_u32_unchecked(1 << 8)) * rs1[1].clone()
            + E::F::from(BaseField::from_u32_unchecked(1 << 16)) * rs1[2].clone()
            + E::F::from(BaseField::from_u32_unchecked(1 << 24)) * rs1[3].clone();
        let rd_felt = rd[0].clone()
            + E::F::from(BaseField::from_u32_unchecked(1 << 8)) * rd[1].clone()
            + E::F::from(BaseField::from_u32_unchecked(1 << 16)) * rd[2].clone()
            + E::F::from(BaseField::from_u32_unchecked(1 << 24)) * rd[3].clone();

        let opcode_jalr_id = E::F::from(BaseField::from_u32_unchecked(Opcode::Jalr as u32));
        let _ = opcode_jalr_id;

        // enabler is boolean
        eval.add_constraint(cols.enabler.clone() * (E::F::one() - cols.enabler.clone()));

        // to_pc_lsb is boolean
        eval.add_constraint(cols.to_pc_lsb.clone() * (E::F::one() - cols.to_pc_lsb.clone()));

        // check next pc
        eval.add_constraint(
            cols.to_pc_over_two.clone() * E::F::from(BaseField::from_u32_unchecked(2))
                + cols.to_pc_lsb.clone()
                - (rs1_felt + cols.imm_felt.clone()),
        );

        // rd is pc + 4
        eval.add_constraint(
            cols.enabler.clone()
                * (rd_felt - (cols.pc.clone() + E::F::from(BaseField::from_u32_unchecked(4)))),
        );

        eval
    }
}
