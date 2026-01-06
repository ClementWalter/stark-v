//! AIR component for Branch Equal (beq/bne) - airs.md Section 7

use crate::add_to_relation;
use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::BranchEqColumns;
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
        let cols = BranchEqColumns::from_eval(&mut eval);

        // Section 7.2: Variables
        let enabler = cols.opcode_beq_flag.clone() + cols.opcode_bne_flag.clone();
        let expected_opcode_id = cols.opcode_beq_flag.clone()
            * E::F::from(BaseField::from_u32_unchecked(Opcode::Beq as u32))
            + cols.opcode_bne_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Bne as u32));

        let cmp_eq = cols.cmp_result.clone() * cols.opcode_beq_flag.clone()
            + (E::F::one() - cols.cmp_result.clone()) * cols.opcode_bne_flag.clone();

        let diff_inv_markers = [
            cols.diff_inv_marker_0.clone(),
            cols.diff_inv_marker_1.clone(),
            cols.diff_inv_marker_2.clone(),
            cols.diff_inv_marker_3.clone(),
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

        let diff_inv_sum = diff_inv_markers
            .iter()
            .zip(rs1.iter().zip(rs2.iter()))
            .fold(cmp_eq.clone(), |acc, (marker, (a, b))| {
                acc + (a.clone() - b.clone()) * marker.clone()
            });

        // REG_AS = 0 for register address space
        let reg_as = E::F::zero();

        // to_pc for conditional branch
        let four = E::F::from(BaseField::from_u32_unchecked(4));
        let to_pc = cols.pc.clone()
            + cols.imm_felt.clone() * cols.cmp_result.clone()
            + four * (E::F::one() - cols.cmp_result.clone());

        // Section 7.3: Constraints

        // enabler, opcode_*_flags and cmp_result are booleans
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));
        eval.add_constraint(
            cols.opcode_beq_flag.clone() * (E::F::one() - cols.opcode_beq_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_bne_flag.clone() * (E::F::one() - cols.opcode_bne_flag.clone()),
        );
        eval.add_constraint(cols.cmp_result.clone() * (E::F::one() - cols.cmp_result.clone()));

        // check cmp_eq
        for (a, b) in rs1.iter().zip(rs2.iter()) {
            eval.add_constraint(cmp_eq.clone() * (a.clone() - b.clone()));
        }
        eval.add_constraint(enabler.clone() * (E::F::one() - diff_inv_sum));

        // =====================================================================
        // LogUp Relations (Section 7.3 from airs.md)
        // =====================================================================

        // Program access (B-type): - enabler * Program(pc, expected_opcode_id, rs1_idx, rs2_idx, imm_felt)
        add_to_relation!(
            eval,
            self.relations.program_access,
            -enabler.clone(),
            cols.pc,
            expected_opcode_id.clone(),
            cols.rs1_addr,
            cols.rs2_addr,
            cols.imm_felt
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

        // Register state transition (conditional branch)
        // - enabler * Registers(pc, clk)
        add_to_relation!(
            eval,
            self.relations.registers_state,
            -enabler.clone(),
            cols.pc,
            cols.clk
        );
        // + enabler * Registers(to_pc, clk + 1)
        add_to_relation!(
            eval,
            self.relations.registers_state,
            enabler.clone(),
            to_pc,
            cols.clk.clone() + E::F::one()
        );

        eval.finalize_logup_in_pairs();
        eval
    }
}
