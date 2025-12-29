//! AIR component for Base ALU Reg (add/sub/xor/or/and) - airs.md Section 1

use num_traits::{One, Zero};
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};
use stwo::core::fields::m31::BaseField;
use runner::decode::Opcode;

use super::columns::Base_alu_regColumns;
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
        let cols = Base_alu_regColumns::from_eval(&mut eval);

        // Section 1.2: Variables
        let enabler = cols.opcode_add_flag.clone()
            + cols.opcode_sub_flag.clone()
            + cols.opcode_xor_flag.clone()
            + cols.opcode_or_flag.clone()
            + cols.opcode_and_flag.clone();

        let expected_opcode_id = cols.opcode_add_flag.clone()
            * E::F::from(BaseField::from_u32_unchecked(Opcode::Add as u32))
            + cols.opcode_sub_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Sub as u32))
            + cols.opcode_xor_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Xor as u32))
            + cols.opcode_or_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Or as u32))
            + cols.opcode_and_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::And as u32));

        let rd = [
            cols.rd_next_0.clone(),
            cols.rd_next_1.clone(),
            cols.rd_next_2.clone(),
            cols.rd_next_3.clone(),
        ];
        let rs1 = [
            cols.rs1_next_0.clone(),
            cols.rs1_next_1.clone(),
            cols.rs1_next_2.clone(),
            cols.rs1_next_3.clone(),
        ];
        let rs2 = [
            cols.rs2_next_0.clone(),
            cols.rs2_next_1.clone(),
            cols.rs2_next_2.clone(),
            cols.rs2_next_3.clone(),
        ];

        let inv_two_pow_8 = BaseField::from_u32_unchecked(1 << 8).inverse();
        let mut carry_add: [E::F; 4] = std::array::from_fn(|_| E::F::zero());
        let mut carry_sub: [E::F; 4] = std::array::from_fn(|_| E::F::zero());

        carry_add[0] =
            (rs1[0].clone() + rs2[0].clone() - rd[0].clone()) * inv_two_pow_8;
        carry_sub[0] =
            (rd[0].clone() + rs2[0].clone() - rs1[0].clone()) * inv_two_pow_8;
        for i in 1..4 {
            carry_add[i] = (rs1[i].clone()
                + rs2[i].clone()
                + carry_add[i - 1].clone()
                - rd[i].clone())
                * inv_two_pow_8;
            carry_sub[i] = (rd[i].clone()
                + rs2[i].clone()
                - rs1[i].clone()
                + carry_sub[i - 1].clone())
                * inv_two_pow_8;
        }

        let is_bitwise = cols.opcode_xor_flag.clone()
            + cols.opcode_or_flag.clone()
            + cols.opcode_and_flag.clone();
        let two = E::F::one() + E::F::one();
        let three = two.clone() + E::F::one();
        let four = two.clone() + two.clone();
        let bitwise_id = cols.opcode_xor_flag.clone()
            + two.clone() * cols.opcode_or_flag.clone()
            + three.clone() * cols.opcode_and_flag.clone()
            + four * (cols.opcode_add_flag.clone() + cols.opcode_sub_flag.clone());

        // Avoid unused-variable warnings for values only used by lookup constraints.
        let _ = (expected_opcode_id, is_bitwise, bitwise_id);

        // Section 1.3: Constraints

        // enabler is boolean
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));

        // opcode flags are booleans
        eval.add_constraint(
            cols.opcode_add_flag.clone() * (E::F::one() - cols.opcode_add_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_sub_flag.clone() * (E::F::one() - cols.opcode_sub_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_xor_flag.clone() * (E::F::one() - cols.opcode_xor_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_or_flag.clone() * (E::F::one() - cols.opcode_or_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_and_flag.clone() * (E::F::one() - cols.opcode_and_flag.clone()),
        );

        // check carries
        for carry in carry_add {
            eval.add_constraint(
                cols.opcode_add_flag.clone()
                    * carry.clone()
                    * (E::F::one() - carry),
            );
        }

        for carry in carry_sub {
            eval.add_constraint(
                cols.opcode_sub_flag.clone()
                    * carry.clone()
                    * (E::F::one() - carry),
            );
        }

        eval
    }
}
