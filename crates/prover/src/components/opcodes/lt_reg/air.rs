//! AIR component for Less Than Reg (slt/sltu) - airs.md Section 5

use crate::add_to_relation;
use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::LtRegColumns;
use crate::relations::Relations;

pub type Component = FrameworkComponent<Eval>;

/// Helper: 2^n as field element
fn pow2<E: EvalAtRow>(n: u32) -> E::F {
    E::F::from(BaseField::from_u32_unchecked(1 << n))
}

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
        let cols = LtRegColumns::from_eval(&mut eval);

        // Section 5.2: Variables
        let enabler = cols.opcode_slt_flag.clone() + cols.opcode_sltu_flag.clone();
        let expected_opcode_id = cols.opcode_slt_flag.clone()
            * E::F::from(BaseField::from_u32_unchecked(Opcode::Slt as u32))
            + cols.opcode_sltu_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Sltu as u32));

        let diff_markers = [
            cols.diff_marker_0.clone(),
            cols.diff_marker_1.clone(),
            cols.diff_marker_2.clone(),
            cols.diff_marker_3.clone(),
        ];
        let prefix_sum_final = diff_markers[0].clone()
            + diff_markers[1].clone()
            + diff_markers[2].clone()
            + diff_markers[3].clone();
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

        let two_pow_8 = pow2::<E>(8);
        let two = E::F::one() + E::F::one();

        // REG_AS = 0 for register address space
        let reg_as = E::F::zero();

        // Section 5.3: Constraints

        // enabler, opcode_*_flags, cmp_result and diff_markers are booleans
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));
        eval.add_constraint(
            cols.opcode_slt_flag.clone() * (E::F::one() - cols.opcode_slt_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_sltu_flag.clone() * (E::F::one() - cols.opcode_sltu_flag.clone()),
        );
        eval.add_constraint(cols.cmp_result.clone() * (E::F::one() - cols.cmp_result.clone()));

        for marker in diff_markers.iter() {
            eval.add_constraint(marker.clone() * (E::F::one() - marker.clone()));
        }

        // msl are the most significant limbs as felts
        let rs1_msl_gap = rs1[3].clone() - cols.rs1_msl_felt.clone();
        eval.add_constraint(rs1_msl_gap.clone() * (two_pow_8.clone() - rs1_msl_gap));

        let rs2_msl_gap = rs2[3].clone() - cols.rs2_msl_felt.clone();
        eval.add_constraint(rs2_msl_gap.clone() * (two_pow_8.clone() - rs2_msl_gap));

        // comparison logic
        let mut prefix_sum = E::F::zero();
        for (i, marker) in diff_markers.iter().enumerate().rev() {
            let limb_diff = if i == 3 {
                cols.rs2_msl_felt.clone() - cols.rs1_msl_felt.clone()
            } else {
                rs2[i].clone() - rs1[i].clone()
            };
            let diff = (two.clone() * cols.cmp_result.clone() - E::F::one()) * limb_diff;

            prefix_sum += marker.clone();
            eval.add_constraint((E::F::one() - prefix_sum.clone()) * diff.clone());
            eval.add_constraint(marker.clone() * (cols.diff_val.clone() - diff));
        }

        // prefix_sum contains at most one activation
        eval.add_constraint(prefix_sum.clone() * (E::F::one() - prefix_sum.clone()));

        // if equal, result is 0
        eval.add_constraint((E::F::one() - prefix_sum) * cols.cmp_result.clone());

        // =====================================================================
        // LogUp Relations (Section 5.3 from airs.md)
        // =====================================================================

        // Program access (R-type): - enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, rs2_idx)
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

        // Range check msl felts with sign consideration
        // - RC_8_8(rs1_msl_felt + opcode_slt_flag * 2^(8-1), rs2_msl_felt + opcode_slt_flag * 2^(8-1))
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            cols.rs1_msl_felt.clone() + cols.opcode_slt_flag.clone() * pow2::<E>(7),
            cols.rs2_msl_felt.clone() + cols.opcode_slt_flag.clone() * pow2::<E>(7)
        );

        // diff_val is > 0 (when prefix_sum = 1)
        // - prefix_sum * RC_20(diff_val - 1)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -prefix_sum_final.clone(),
            cols.diff_val.clone() - E::F::one()
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
        // + enabler * Memory(REG_AS, rd_idx, clk, cmp_result, 0, 0, 0)
        add_to_relation!(
            eval,
            self.relations.memory_access,
            enabler.clone(),
            reg_as.clone(),
            cols.rd_addr,
            cols.clk,
            cols.cmp_result,
            E::F::zero(),
            E::F::zero(),
            E::F::zero()
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
