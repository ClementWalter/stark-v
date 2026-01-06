//! AIR component for Base ALU Reg (add/sub/xor/or/and) - airs.md Section 1

use crate::relations::Relations;
use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::BaseAluRegColumns;

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
        // Match preprocessed bitwise table: and=0, or=1, xor=2
        let bitwise_id = two.clone() * cols.opcode_xor_flag.clone() + cols.opcode_or_flag.clone();

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
        // =====================================================================

        // REG_AS = 0 for register address space
        let reg_as = E::F::zero();

        // Program access (consume): read instruction from Program segment (R-type)
        // - enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, rs2_idx)
        add_to_relation!(
            eval,
            self.relations.program_access,
            -enabler.clone(),
            cols.pc,
            expected_opcode_id.clone(),
            cols.rd_addr,
            cols.rs1_addr,
            cols.rs2_addr
        );

        // Register state transition
        // - enabler * Registers(pc, clk)
        add_to_relation!(
            eval,
            self.relations.registers_state,
            -enabler.clone(),
            cols.pc,
            cols.clk
        );
        // + enabler * Registers(pc + 4, clk + 1)
        add_to_relation!(
            eval,
            self.relations.registers_state,
            enabler.clone(),
            cols.pc.clone() + E::F::from(BaseField::from_u32_unchecked(4)),
            cols.clk.clone() + E::F::one()
        );

        // Read from rs1
        // - enabler * Memory(REG_AS, rs1_idx, rs1_prev_clk, rs1[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            -enabler.clone(),
            reg_as.clone(),
            cols.rs1_addr,
            cols.rs1_clk_prev,
            cols.rs1_prev_0,
            cols.rs1_prev_1,
            cols.rs1_prev_2,
            cols.rs1_prev_3
        );
        // + enabler * Memory(REG_AS, rs1_idx, clk, rs1[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            enabler.clone(),
            reg_as.clone(),
            cols.rs1_addr,
            cols.clk,
            cols.rs1_next_0,
            cols.rs1_next_1,
            cols.rs1_next_2,
            cols.rs1_next_3
        );
        // - RC_20(clk - rs1_prev_clk)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -enabler.clone(),
            cols.clk.clone() - cols.rs1_clk_prev.clone()
        );

        // Read from rs2
        // - enabler * Memory(REG_AS, rs2_idx, rs2_prev_clk, rs2[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            -enabler.clone(),
            reg_as.clone(),
            cols.rs2_addr,
            cols.rs2_clk_prev,
            cols.rs2_prev_0,
            cols.rs2_prev_1,
            cols.rs2_prev_2,
            cols.rs2_prev_3
        );
        // + enabler * Memory(REG_AS, rs2_idx, clk, rs2[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            enabler.clone(),
            reg_as.clone(),
            cols.rs2_addr,
            cols.clk,
            cols.rs2_next_0,
            cols.rs2_next_1,
            cols.rs2_next_2,
            cols.rs2_next_3
        );
        // - RC_20(clk - rs2_prev_clk)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -enabler.clone(),
            cols.clk.clone() - cols.rs2_clk_prev.clone()
        );

        // Bitwise operations (for xor/or/and)
        // - is_bitwise * Bitwise(rs1[i], rs2[i], rd[i], bitwise_id) for each limb
        add_to_relation!(
            eval,
            self.relations.bitwise,
            -is_bitwise.clone(),
            rs1[0].clone(),
            rs2[0].clone(),
            rd[0].clone(),
            bitwise_id.clone()
        );
        add_to_relation!(
            eval,
            self.relations.bitwise,
            -is_bitwise.clone(),
            rs1[1].clone(),
            rs2[1].clone(),
            rd[1].clone(),
            bitwise_id.clone()
        );
        add_to_relation!(
            eval,
            self.relations.bitwise,
            -is_bitwise.clone(),
            rs1[2].clone(),
            rs2[2].clone(),
            rd[2].clone(),
            bitwise_id.clone()
        );
        add_to_relation!(
            eval,
            self.relations.bitwise,
            -is_bitwise.clone(),
            rs1[3].clone(),
            rs2[3].clone(),
            rd[3].clone(),
            bitwise_id.clone()
        );

        // Write to rd
        // - enabler * Memory(REG_AS, rd_idx, rd_prev_clk, rd_prev[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            -enabler.clone(),
            reg_as.clone(),
            cols.rd_addr,
            cols.rd_clk_prev,
            cols.rd_prev_0,
            cols.rd_prev_1,
            cols.rd_prev_2,
            cols.rd_prev_3
        );
        // + enabler * Memory(REG_AS, rd_idx, clk, rd[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            enabler.clone(),
            reg_as.clone(),
            cols.rd_addr,
            cols.clk,
            rd[0].clone(),
            rd[1].clone(),
            rd[2].clone(),
            rd[3].clone()
        );
        // - RC_20(clk - rd_prev_clk)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -enabler.clone(),
            cols.clk.clone() - cols.rd_clk_prev.clone()
        );

        eval.finalize_logup_in_pairs();
        eval
    }
}
