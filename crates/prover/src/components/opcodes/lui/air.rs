//! AIR component for LUI - airs.md Section 9

use crate::add_to_relation;
use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::LuiColumns;
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
        let cols = LuiColumns::from_eval(&mut eval);

        // Section 9.2: Variables
        let imm = cols.imm_0.clone()
            + pow2::<E>(4) * cols.imm_1.clone()
            + pow2::<E>(12) * cols.imm_2.clone();
        let opcode_lui_id = E::F::from(BaseField::from_u32_unchecked(Opcode::Lui as u32));

        // REG_AS = 0 for register address space
        let reg_as = E::F::zero();

        // enabler is boolean (single opcode family)
        eval.add_constraint(cols.enabler.clone() * (E::F::one() - cols.enabler.clone()));

        // =====================================================================
        // LogUp Relations (Section 9.3 from airs.md)
        // =====================================================================

        // Program access (U-type): - enabler * Program(pc, opcode_lui_id, rd_idx, imm, 0)
        add_to_relation!(
            eval,
            self.relations.program_access,
            -cols.enabler.clone(),
            cols.pc,
            opcode_lui_id.clone(),
            cols.rd_addr,
            imm.clone(),
            E::F::zero()
        );

        // Register state transition
        // - enabler * Registers(pc, clk)
        add_to_relation!(
            eval,
            self.relations.registers_state,
            -cols.enabler.clone(),
            cols.pc,
            cols.clk
        );
        // + enabler * Registers(pc + 4, clk + 1)
        add_to_relation!(
            eval,
            self.relations.registers_state,
            cols.enabler.clone(),
            cols.pc.clone() + E::F::from(BaseField::from_u32_unchecked(4)),
            cols.clk.clone() + E::F::one()
        );

        // Range check imm limbs: - RC_8_8_4(imm_1, imm_2, imm_0)
        add_to_relation!(
            eval,
            self.relations.range_check_8_8_4,
            -E::F::one(),
            cols.imm_1,
            cols.imm_2,
            cols.imm_0
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
        // + enabler * Memory(REG_AS, rd_idx, clk, 0, imm_0 * 2^4, imm_1, imm_2)
        add_to_relation!(
            eval,
            self.relations.memory_access,
            cols.enabler.clone(),
            reg_as.clone(),
            cols.rd_addr,
            cols.clk,
            E::F::zero(),
            cols.imm_0.clone() * pow2::<E>(4),
            cols.imm_1,
            cols.imm_2
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
