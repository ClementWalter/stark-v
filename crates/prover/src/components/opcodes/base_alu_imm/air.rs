//! AIR component for Base ALU Imm (addi/xori/ori/andi) - airs.md Section 2

use crate::add_to_relation;
use crate::relations::Relations;
use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::BaseAluImmColumns;

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
        // Match preprocessed bitwise table: and=0, or=1, xor=2
        let bitwise_id =
            two.clone() * cols.opcode_xor_flag.clone() + cols.opcode_or_flag.clone();

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

        // =====================================================================
        // LogUp Relations (Section 2.3 from airs.md)
        // =====================================================================

        // REG_AS = 0 for register address space
        let reg_as = E::F::zero();

        // Program access (consume): read instruction from Program segment (I-type)
        // - enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, imm)
        add_to_relation!(
            eval,
            self.relations.program_access,
            -enabler.clone(),
            cols.pc,
            expected_opcode_id,
            cols.rd_addr,
            cols.rs1_addr,
            imm
        );

        // Range check imm
        // - RC_8_11(imm_0, 2^8 * imm_1)
        add_to_relation!(
            eval,
            self.relations.range_check_8_11,
            -enabler.clone(),
            cols.imm_0,
            pow2(8) * cols.imm_1.clone()
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

        // Bitwise operations (for xor/or/and)
        // - is_bitwise * Bitwise(rs1[i], sext_imm[i], rd[i], bitwise_id) for each limb
        add_to_relation!(
            eval,
            self.relations.bitwise,
            -is_bitwise.clone(),
            rs1[0].clone(),
            sext_imm[0].clone(),
            rd[0].clone(),
            bitwise_id.clone()
        );
        add_to_relation!(
            eval,
            self.relations.bitwise,
            -is_bitwise.clone(),
            rs1[1].clone(),
            sext_imm[1].clone(),
            rd[1].clone(),
            bitwise_id.clone()
        );
        add_to_relation!(
            eval,
            self.relations.bitwise,
            -is_bitwise.clone(),
            rs1[2].clone(),
            sext_imm[2].clone(),
            rd[2].clone(),
            bitwise_id.clone()
        );
        add_to_relation!(
            eval,
            self.relations.bitwise,
            -is_bitwise.clone(),
            rs1[3].clone(),
            sext_imm[3].clone(),
            rd[3].clone(),
            bitwise_id.clone()
        );

        // Range check rd
        // - RC_8_8(rd[0], rd[1])
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            rd[0].clone(),
            rd[1].clone()
        );
        // - RC_8_8(rd[2], rd[3])
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            rd[2].clone(),
            rd[3].clone()
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
