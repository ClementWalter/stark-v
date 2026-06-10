//! AIR component for Base ALU Reg (add/sub/xor/or/and) - airs.md Section 1

use crate::relations::Relations;
use num_traits::Zero;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use runner::trace::prover_columns::BaseAluRegColumns;

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
        // Section 1.2/1.3: derived columns and constraints, declared in
        // define_trace_tables!
        let enabler = cols.enabler();
        let expected_opcode_id = cols.expected_opcode_id();
        let is_bitwise = cols.is_bitwise();
        let bitwise_id = cols.bitwise_id();

        for constraint in cols.constraints() {
            eval.add_constraint(constraint);
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
        // - enabler * Registers(pc, clock)
        add_to_relation!(
            eval,
            self.relations.registers_state,
            -enabler.clone(),
            cols.pc,
            cols.clock
        );
        // + enabler * Registers(pc + 4, clock + 1)
        add_to_relation!(
            eval,
            self.relations.registers_state,
            enabler.clone(),
            cols.pc_next(),
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

        // Bitwise operations (for xor/or/and)
        // - is_bitwise * Bitwise(rs1[i], rs2[i], rd[i], bitwise_id) for each limb
        add_to_relation!(
            eval,
            self.relations.bitwise,
            -is_bitwise.clone(),
            cols.rs1_next_0,
            cols.rs2_next_0,
            cols.rd_next_0,
            bitwise_id.clone()
        );
        add_to_relation!(
            eval,
            self.relations.bitwise,
            -is_bitwise.clone(),
            cols.rs1_next_1,
            cols.rs2_next_1,
            cols.rd_next_1,
            bitwise_id.clone()
        );
        add_to_relation!(
            eval,
            self.relations.bitwise,
            -is_bitwise.clone(),
            cols.rs1_next_2,
            cols.rs2_next_2,
            cols.rd_next_2,
            bitwise_id.clone()
        );
        add_to_relation!(
            eval,
            self.relations.bitwise,
            -is_bitwise.clone(),
            cols.rs1_next_3,
            cols.rs2_next_3,
            cols.rd_next_3,
            bitwise_id.clone()
        );

        // Write to rd
        // - enabler * Memory(REG_AS, rd_idx, rd_prev_clock, rd_prev[0..3])
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
        // + enabler * Memory(REG_AS, rd_idx, clock, rd[0..3])
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
        // - RC_20(clock - rd_prev_clock)
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
