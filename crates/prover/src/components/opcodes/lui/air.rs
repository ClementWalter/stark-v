//! AIR component for LUI - airs.md Section 9

use num_traits::Zero;
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use crate::relations::Relations;
use runner::trace::prover_columns::LuiColumns;

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
        let cols = LuiColumns::from_eval(&mut eval);

        // Section 9.2: Variables (derived columns from define_trace_tables!)
        let imm = cols.imm();
        let opcode_lui_id = E::F::from(BaseField::from_u32_unchecked(Opcode::Lui as u32));

        // REG_AS = 0 for register address space
        let reg_as = E::F::zero();

        // Booleanity of the enabler (single opcode family)
        for constraint in cols.constraints() {
            eval.add_constraint(constraint);
        }

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
        // - enabler * Registers(pc, clock)
        add_to_relation!(
            eval,
            self.relations.registers_state,
            -cols.enabler.clone(),
            cols.pc,
            cols.clock
        );
        // + enabler * Registers(pc + 4, clock + 1)
        add_to_relation!(
            eval,
            self.relations.registers_state,
            cols.enabler.clone(),
            cols.pc_next(),
            cols.clock_next()
        );

        // Range check imm limbs: - RC_8_8_4(imm_1, imm_2, imm_0)
        add_to_relation!(
            eval,
            self.relations.range_check_8_8_4,
            -cols.enabler.clone(),
            cols.imm_1,
            cols.imm_2,
            cols.imm_0
        );

        // Write to rd
        // - enabler * Memory(REG_AS, rd_idx, rd_prev_clock, rd_prev[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            -cols.enabler.clone(),
            reg_as.clone(),
            cols.rd_addr,
            cols.rd_clock_prev,
            cols.rd_prev_0,
            cols.rd_prev_1,
            cols.rd_prev_2,
            cols.rd_prev_3
        );
        // + enabler * Memory(REG_AS, rd_idx, clock, 0, imm_0 * 2^4, imm_1, imm_2)
        add_to_relation!(
            eval,
            self.relations.memory_access,
            cols.enabler.clone(),
            reg_as.clone(),
            cols.rd_addr,
            cols.clock,
            E::F::zero(),
            cols.rd_val_1(),
            cols.imm_1,
            cols.imm_2
        );
        // - RC_20(clock - rd_prev_clock)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -cols.enabler.clone(),
            cols.rd_clock_diff()
        );

        eval.finalize_logup_in_pairs();
        eval
    }
}
