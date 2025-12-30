//! AIR component for MULH (mulh/mulhsu/mulhu) - airs.md Section 15

use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
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

        // Section 15.2: Variables
        let enabler = cols.opcode_mulh_flag.clone()
            + cols.opcode_mulhsu_flag.clone()
            + cols.opcode_mulhu_flag.clone();

        let expected_opcode_id = cols.opcode_mulh_flag.clone()
            * E::F::from(BaseField::from_u32_unchecked(Opcode::Mulh as u32))
            + cols.opcode_mulhsu_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Mulhsu as u32))
            + cols.opcode_mulhu_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Mulhu as u32));

        let rd = [
            cols.rd_high_0.clone(),
            cols.rd_high_1.clone(),
            cols.rd_high_2.clone(),
            cols.rd_high_3.clone(),
            cols.rd_next_0.clone(),
            cols.rd_next_1.clone(),
            cols.rd_next_2.clone(),
            cols.rd_next_3.clone(),
        ];
        let rs1 = [
            cols.rs1_next_0.clone(),
            cols.rs1_next_1.clone(),
            cols.rs1_next_2.clone(),
            cols.rs1_next_3.clone()
                + cols.rs1_sign.clone() * E::F::from(BaseField::from_u32_unchecked(1 << 7)),
        ];
        let rs2 = [
            cols.rs2_next_0.clone(),
            cols.rs2_next_1.clone(),
            cols.rs2_next_2.clone(),
            cols.rs2_next_3.clone()
                + cols.rs2_sign.clone() * E::F::from(BaseField::from_u32_unchecked(1 << 7)),
        ];

        let carry: [E::F; 8] = std::array::from_fn(|_| E::F::zero());

        let _ = (expected_opcode_id, carry, rd, rs1, rs2);

        // Section 15.3: Constraints

        // enabler, opcode_i_flag and rs_signs are booleans
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

        // check the signs of the operand extensions
        eval.add_constraint(
            (cols.opcode_mulhsu_flag.clone() + cols.opcode_mulhu_flag.clone())
                * cols.rs2_sign.clone(),
        );
        eval.add_constraint(cols.opcode_mulhu_flag.clone() * cols.rs1_sign.clone());

        eval
    }
}
