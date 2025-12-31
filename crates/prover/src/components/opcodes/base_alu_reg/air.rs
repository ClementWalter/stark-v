//! AIR component for Base ALU Reg (add/sub/xor/or/and) - airs.md Section 1

use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::BaseAluRegColumns;
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
        let cols = BaseAluRegColumns::from_eval(&mut eval);

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

        carry_add[0] = (rs1[0].clone() + rs2[0].clone() - rd[0].clone()) * inv_two_pow_8;
        carry_sub[0] = (rd[0].clone() + rs2[0].clone() - rs1[0].clone()) * inv_two_pow_8;
        for i in 1..4 {
            carry_add[i] = (rs1[i].clone() + rs2[i].clone() + carry_add[i - 1].clone()
                - rd[i].clone())
                * inv_two_pow_8;
            carry_sub[i] = (rd[i].clone() + rs2[i].clone() - rs1[i].clone()
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
                cols.opcode_add_flag.clone() * carry.clone() * (E::F::one() - carry),
            );
        }

        for carry in carry_sub {
            eval.add_constraint(
                cols.opcode_sub_flag.clone() * carry.clone() * (E::F::one() - carry),
            );
        }

        // =====================================================================
        // LogUp Relations (Section 1.3 from airs.md)
        // TODO: Implement these using add_to_relation! macro
        //
        // Example usage pattern:
        //
        // // Program access (consume)
        // add_to_relation!(eval, self.relations.program_access, -enabler.clone(),
        //     cols.pc, expected_opcode_id, cols.rd_addr, cols.rs1_addr, cols.rs2_addr);
        //
        // // Register state transition (consume old, emit new)
        // add_to_relation!(eval, self.relations.registers_state, -enabler.clone(),
        //     cols.pc, cols.clk);
        // add_to_relation!(eval, self.relations.registers_state, enabler.clone(),
        //     cols.pc + E::F::from(BaseField::from_u32_unchecked(4)),
        //     cols.clk + E::F::one());
        //
        // // Register rs1 access (consume prev, emit next)
        // add_to_relation!(eval, self.relations.register_access, -enabler.clone(),
        //     cols.rs1_addr, cols.rs1_prev_0, cols.rs1_prev_1, cols.rs1_prev_2, cols.rs1_prev_3);
        // add_to_relation!(eval, self.relations.register_access, enabler.clone(),
        //     cols.rs1_addr, cols.rs1_next_0, cols.rs1_next_1, cols.rs1_next_2, cols.rs1_next_3);
        //
        // // Range check clock difference
        // add_to_relation!(eval, self.relations.range_check_20, -E::EF::one(),
        //     cols.clk - cols.rs1_clk_prev);
        //
        // // Bitwise relation (for xor/or/and)
        // add_to_relation!(eval, self.relations.bitwise, -is_bitwise.clone(),
        //     cols.rs1_next_0, cols.rs2_next_0, cols.rd_next_0, bitwise_id.clone());
        // =====================================================================

        eval
    }
}
