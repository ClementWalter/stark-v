//! AIR component for Base ALU Imm (addi/xori/ori/andi) - airs.md Section 2

use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::BaseAluImmColumns;
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
        let cols = BaseAluImmColumns::from_eval(&mut eval);

        // Section 2.2: Variables
        let enabler = cols.opcode_add_flag.clone()
            + cols.opcode_xor_flag.clone()
            + cols.opcode_or_flag.clone()
            + cols.opcode_and_flag.clone();

        let expected_opcode_id = cols.opcode_add_flag.clone()
            * E::F::from(BaseField::from_u32_unchecked(Opcode::Addi as u32))
            + cols.opcode_xor_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Xori as u32))
            + cols.opcode_or_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Ori as u32))
            + cols.opcode_and_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Andi as u32));

        let pow2 = |exp: u32| E::F::from(BaseField::from_u32_unchecked(1 << exp));

        let imm =
            cols.imm_0.clone() + pow2(8) * cols.imm_1.clone() + pow2(11) * cols.imm_msb.clone();
        let sext_imm_0 = cols.imm_0.clone();
        let sext_imm_1 = cols.imm_1.clone()
            + E::F::from(BaseField::from_u32_unchecked(1 << 3))
                * E::F::from(BaseField::from_u32_unchecked((1 << 5) - 1))
                * cols.imm_msb.clone();
        let sext_imm_2 =
            E::F::from(BaseField::from_u32_unchecked((1 << 8) - 1)) * cols.imm_msb.clone();
        let sext_imm_3 = sext_imm_2.clone();

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
        let sext_imm = [
            sext_imm_0.clone(),
            sext_imm_1.clone(),
            sext_imm_2.clone(),
            sext_imm_3.clone(),
        ];

        let inv_two_pow_8 = BaseField::from_u32_unchecked(1 << 8).inverse();
        let mut carry_add: [E::F; 4] = std::array::from_fn(|_| E::F::zero());
        carry_add[0] = (rs1[0].clone() + sext_imm[0].clone() - rd[0].clone()) * inv_two_pow_8;
        for i in 1..4 {
            carry_add[i] = (rs1[i].clone() + sext_imm[i].clone() + carry_add[i - 1].clone()
                - rd[i].clone())
                * inv_two_pow_8;
        }

        let is_bitwise = cols.opcode_xor_flag.clone()
            + cols.opcode_or_flag.clone()
            + cols.opcode_and_flag.clone();
        let two = E::F::one() + E::F::one();
        let three = two.clone() + E::F::one();
        let bitwise_id = cols.opcode_xor_flag.clone()
            + two.clone() * cols.opcode_or_flag.clone()
            + three * cols.opcode_and_flag.clone();

        let _ = (
            expected_opcode_id,
            imm,
            bitwise_id,
            is_bitwise,
            sext_imm_2,
            sext_imm_3,
        );

        // Section 2.3: Constraints

        // enabler is boolean
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));

        // opcode flags are booleans
        eval.add_constraint(
            cols.opcode_add_flag.clone() * (E::F::one() - cols.opcode_add_flag.clone()),
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

        // imm_msb is boolean
        eval.add_constraint(cols.imm_msb.clone() * (E::F::one() - cols.imm_msb.clone()));

        // check carries
        for carry in carry_add {
            eval.add_constraint(
                cols.opcode_add_flag.clone() * carry.clone() * (E::F::one() - carry),
            );
        }

        eval
    }
}
