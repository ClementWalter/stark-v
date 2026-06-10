//! AIR component for Shifts Imm (slli/srli/srai) - airs.md Section 4

use crate::relations::Relations;
use num_traits::Zero;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use runner::trace::prover_columns::ShiftsImmColumns;

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
        let cols = ShiftsImmColumns::from_eval(&mut eval);

        // Section 4.2/4.3: derived columns and constraints, declared in
        // define_trace_tables!
        let enabler = cols.enabler();
        let expected_opcode_id = cols.expected_opcode_id();

        for constraint in cols.constraints() {
            eval.add_constraint(constraint);
        }

        // =====================================================================
        // LogUp Relations (Section 4.3 from airs.md)
        // =====================================================================

        let reg_as = E::F::zero();

        // Program access (consume): I-type
        add_to_relation!(
            eval,
            self.relations.program_access,
            -enabler.clone(),
            cols.pc,
            expected_opcode_id.clone(),
            cols.rd_addr,
            cols.rs1_addr,
            cols.imm_truncated
        );

        // Register state transition
        add_to_relation!(
            eval,
            self.relations.registers_state,
            -enabler.clone(),
            cols.pc,
            cols.clock
        );
        add_to_relation!(
            eval,
            self.relations.registers_state,
            enabler.clone(),
            cols.pc_next(),
            cols.clock_next()
        );

        // Read from rs1
        add_to_relation!(
            eval,
            self.relations.memory_access,
            -enabler.clone(),
            reg_as.clone(),
            cols.rs1_addr,
            cols.rs1_clock_prev,
            cols.rs1_prev_0,
            cols.rs1_prev_1,
            cols.rs1_prev_2,
            cols.rs1_prev_3
        );
        add_to_relation!(
            eval,
            self.relations.memory_access,
            enabler.clone(),
            reg_as.clone(),
            cols.rs1_addr,
            cols.clock,
            cols.rs1_next_0,
            cols.rs1_next_1,
            cols.rs1_next_2,
            cols.rs1_next_3
        );
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -enabler.clone(),
            cols.rs1_clock_diff()
        );

        // TODO: Range check shift carries (scaled by 2^8 / bit_multiplier)

        // Range check rd
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            cols.rd_next_0,
            cols.rd_next_1
        );
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            cols.rd_next_2,
            cols.rd_next_3
        );

        // Write to rd
        add_to_relation!(
            eval,
            self.relations.memory_access,
            -enabler.clone(),
            reg_as.clone(),
            cols.rd_addr,
            cols.rd_clock_prev,
            cols.rd_prev_0,
            cols.rd_prev_1,
            cols.rd_prev_2,
            cols.rd_prev_3
        );
        add_to_relation!(
            eval,
            self.relations.memory_access,
            enabler.clone(),
            reg_as.clone(),
            cols.rd_addr,
            cols.clock,
            cols.rd_next_0,
            cols.rd_next_1,
            cols.rd_next_2,
            cols.rd_next_3
        );
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -enabler.clone(),
            cols.rd_clock_diff()
        );

        eval.finalize_logup_in_pairs();
        eval
    }
}
