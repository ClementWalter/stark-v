//! AIR component for Branch Less Than (blt/bltu/bge/bgeu) - airs.md Section 8

use num_traits::{One, Zero};
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use crate::relations::Relations;
use runner::trace::prover_columns::BranchLtColumns;

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
        let cols = BranchLtColumns::from_eval(&mut eval);

        // Section 8.2/8.3: derived columns and constraints, declared in
        // define_trace_tables!
        let enabler = cols.enabler();
        let expected_opcode_id = cols.expected_opcode_id();
        let prefix_sum_final = cols.prefix_sum_final();

        // REG_AS = 0 for register address space
        let reg_as = E::F::zero();

        for constraint in cols.constraints() {
            eval.add_constraint(constraint);
        }

        // =====================================================================
        // LogUp Relations (Section 8.3 from airs.md)
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

        // Register state transition (conditional branch)
        // - enabler * Registers(pc, clock)
        add_to_relation!(
            eval,
            self.relations.registers_state,
            -enabler.clone(),
            cols.pc,
            cols.clock
        );
        // + enabler * Registers(branch_target, clock + 1)
        add_to_relation!(
            eval,
            self.relations.registers_state,
            enabler.clone(),
            cols.branch_target,
            cols.clock_next()
        );

        // Read from rs1
        // - enabler * Memory(REG_AS, rs1_idx, rs1_prev_clock, rs1[0..3])
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
        // + enabler * Memory(REG_AS, rs1_idx, clock, rs1[0..3])
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
        // - RC_20(clock - rs1_prev_clock)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -enabler.clone(),
            cols.rs1_clock_diff()
        );

        // Read from rs2
        // - enabler * Memory(REG_AS, rs2_idx, rs2_prev_clock, rs2[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            -enabler.clone(),
            reg_as.clone(),
            cols.rs2_addr,
            cols.rs2_clock_prev,
            cols.rs2_prev_0,
            cols.rs2_prev_1,
            cols.rs2_prev_2,
            cols.rs2_prev_3
        );
        // + enabler * Memory(REG_AS, rs2_idx, clock, rs2[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            enabler.clone(),
            reg_as.clone(),
            cols.rs2_addr,
            cols.clock,
            cols.rs2_next_0,
            cols.rs2_next_1,
            cols.rs2_next_2,
            cols.rs2_next_3
        );
        // - RC_20(clock - rs2_prev_clock)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -enabler.clone(),
            cols.rs2_clock_diff()
        );

        // Range check msl felts with sign consideration
        // - RC_8_8(rs1_msl_felt + signed * 2^(8-1), rs2_msl_felt + signed * 2^(8-1))
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            cols.rs1_msl_shifted(),
            cols.rs2_msl_shifted()
        );

        // diff_val is > 0 (when prefix_sum = 1)
        // - prefix_sum * RC_20(diff_val - 1)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -prefix_sum_final.clone(),
            cols.diff_val.clone() - E::F::one()
        );

        eval.finalize_logup_in_pairs();
        eval
    }
}
