//! AIR component for Less Than Imm (slti/sltiu) - airs.md Section 6

use crate::add_to_relation;
use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::LtImmColumns;
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
        let cols = LtImmColumns::from_eval(&mut eval);

        // Section 6.2: Variables
        let enabler = cols.opcode_slti_flag.clone() + cols.opcode_sltiu_flag.clone();
        let expected_opcode_id = cols.opcode_slti_flag.clone()
            * E::F::from(BaseField::from_u32_unchecked(Opcode::Slti as u32))
            + cols.opcode_sltiu_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Sltiu as u32));

        let imm = cols.imm_0.clone()
            + pow2::<E>(8) * cols.imm_1.clone()
            + pow2::<E>(11) * cols.imm_msb.clone();
        let sext_imm_0 = cols.imm_0.clone();
        let sext_imm_1 = cols.imm_1.clone() + pow2::<E>(3) * pow2::<E>(5) * cols.imm_msb.clone()
            - pow2::<E>(3) * cols.imm_msb.clone();
        let sext_imm_2 = (pow2::<E>(8) - E::F::one()) * cols.imm_msb.clone();
        let sext_imm_3 = sext_imm_2.clone();
        let sext_imm = [
            sext_imm_0.clone(),
            sext_imm_1.clone(),
            sext_imm_2.clone(),
            sext_imm_3.clone(),
        ];

        let sext_imm_msl_felt = cols.opcode_sltiu_flag.clone() * sext_imm_3.clone()
            - cols.opcode_slti_flag.clone() * cols.imm_msb.clone();

        let rs1 = [
            cols.rs1_next_0.clone(),
            cols.rs1_next_1.clone(),
            cols.rs1_next_2.clone(),
            cols.rs1_next_3.clone(),
        ];
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

        let two = E::F::one() + E::F::one();

        // REG_AS = 0 for register address space
        let reg_as = E::F::zero();

        // Section 6.3: Constraints

        // enabler and opcode flags are booleans
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));
        eval.add_constraint(
            cols.opcode_slti_flag.clone() * (E::F::one() - cols.opcode_slti_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_sltiu_flag.clone() * (E::F::one() - cols.opcode_sltiu_flag.clone()),
        );

        // imm_msb is boolean
        eval.add_constraint(cols.imm_msb.clone() * (E::F::one() - cols.imm_msb.clone()));

        // msl are the most significant limbs as felts
        let rs1_msl_gap = rs1[3].clone() - cols.rs1_msl_felt.clone();
        eval.add_constraint(rs1_msl_gap.clone() * (pow2::<E>(8) - rs1_msl_gap));

        // diff markers are boolean and sum correctly
        for marker in diff_markers.iter() {
            eval.add_constraint(marker.clone() * (E::F::one() - marker.clone()));
        }

        let mut prefix_sum = E::F::zero();
        for (i, marker) in diff_markers.iter().enumerate().rev() {
            let limb_diff = if i == 3 {
                sext_imm_msl_felt.clone() - cols.rs1_msl_felt.clone()
            } else {
                sext_imm[i].clone() - rs1[i].clone()
            };
            let diff = (two.clone() * cols.cmp_result.clone() - E::F::one()) * limb_diff;

            prefix_sum += marker.clone();
            eval.add_constraint((E::F::one() - prefix_sum.clone()) * diff.clone());
            eval.add_constraint(marker.clone() * (cols.diff_val.clone() - diff));
        }

        // prefix_sum contains at most one activation
        eval.add_constraint(prefix_sum.clone() * (E::F::one() - prefix_sum.clone()));

        // if equal, result is 0
        eval.add_constraint((E::F::one() - prefix_sum.clone()) * cols.cmp_result.clone());

        // result is boolean
        eval.add_constraint(cols.cmp_result.clone() * (E::F::one() - cols.cmp_result.clone()));

        // =====================================================================
        // LogUp Relations (Section 6.3 from airs.md)
        // =====================================================================

        // Program access (I-type): - enabler * Program(pc, expected_opcode_id, rd_idx, rs1_idx, imm)
        add_to_relation!(
            eval,
            self.relations.program_access,
            -enabler.clone(),
            cols.pc,
            expected_opcode_id.clone(),
            cols.rd_addr,
            cols.rs1_addr,
            imm.clone()
        );

        // Range check imm and range check rs1_msl_felt with sign consideration
        // - RC_8_8_4(rs1_msl_felt + opcode_slti_flag * 2^(8-1), imm_0, 2*imm_1)
        add_to_relation!(
            eval,
            self.relations.range_check_8_8_4,
            -enabler.clone(),
            cols.rs1_msl_felt.clone() + cols.opcode_slti_flag.clone() * pow2::<E>(7),
            cols.imm_0,
            two.clone() * cols.imm_1.clone()
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

        // Range check diff_val is non-zero when prefix_sum = 1
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
