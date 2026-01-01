//! AIR component for JAL - airs.md Section 12

use crate::add_to_relation;
use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::JalColumns;
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
        let cols = JalColumns::from_eval(&mut eval);

        // Section 12.2: Variables
        let rd = [
            cols.rd_next_0.clone(),
            cols.rd_next_1.clone(),
            cols.rd_next_2.clone(),
            cols.rd_next_3.clone(),
        ];
        let rd_felt = rd[0].clone()
            + pow2::<E>(8) * rd[1].clone()
            + pow2::<E>(16) * rd[2].clone()
            + pow2::<E>(24) * rd[3].clone();

        let opcode_jal_id = E::F::from(BaseField::from_u32_unchecked(Opcode::Jal as u32));

        // REG_AS = 0 for register address space
        let reg_as = E::F::zero();

        // enabler is boolean
        eval.add_constraint(cols.enabler.clone() * (E::F::one() - cols.enabler.clone()));

        // rd is pc + 4 (gated by enabler for padding rows and rd_addr for x0 writes)
        // When rd_addr = 0 (x0), the write is discarded and rd_next = 0, so skip this constraint
        eval.add_constraint(
            cols.enabler.clone()
                * cols.rd_addr.clone()
                * (rd_felt.clone()
                    - (cols.pc.clone() + E::F::from(BaseField::from_u32_unchecked(4)))),
        );

        // =====================================================================
        // LogUp Relations (Section 12.3 from airs.md)
        // =====================================================================

        // Program access (U-type): - enabler * Program(pc, opcode_jal_id, rd_idx, imm_felt, 0)
        add_to_relation!(
            eval,
            self.relations.program_access,
            -cols.enabler.clone(),
            cols.pc,
            opcode_jal_id.clone(),
            cols.rd_addr,
            cols.imm_felt,
            E::F::zero()
        );

        // Register state transition (unconditional jump)
        // - enabler * Registers(pc, clk)
        add_to_relation!(
            eval,
            self.relations.registers_state,
            -cols.enabler.clone(),
            cols.pc,
            cols.clk
        );
        // + enabler * Registers(pc + imm_felt, clk + 1)
        add_to_relation!(
            eval,
            self.relations.registers_state,
            cols.enabler.clone(),
            cols.pc.clone() + cols.imm_felt.clone(),
            cols.clk.clone() + E::F::one()
        );

        // Range check rd (rd is pc+4, so it's a M31)
        // - RC_8_8(rd[1], rd[2])
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -E::F::one(),
            rd[1].clone(),
            rd[2].clone()
        );
        // - RC_M31(rd[0], rd[3])
        add_to_relation!(
            eval,
            self.relations.range_check_m31,
            -E::F::one(),
            rd[0].clone(),
            rd[3].clone()
        );

        // Write to rd
        // - enabler * Memory(REG_AS, rd_idx, rd_prev_clk, rd_prev[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            -cols.enabler.clone(),
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
            cols.enabler.clone(),
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
            -E::F::one(),
            cols.clk.clone() - cols.rd_clk_prev.clone()
        );

        eval.finalize_logup_in_pairs();
        eval
    }
}
