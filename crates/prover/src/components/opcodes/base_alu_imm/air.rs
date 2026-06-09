//! AIR component for Base ALU Imm (addi/xori/ori/andi) - airs.md Section 2

use crate::relations::Relations;
use num_traits::Zero;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use runner::trace::prover_columns::BaseAluImmColumns;

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

        // Section 2.2: Variables (derived columns from define_trace_tables!)
        let enabler = cols.enabler();
        let is_bitwise = cols.is_bitwise();
        let bitwise_id = cols.bitwise_id();
        let sext_imm = [
            cols.imm_0.clone(),
            cols.sext_imm_1(),
            cols.sext_imm_2(),
            cols.sext_imm_2(),
        ];

        // Section 2.3: Constraints — booleanity of enabler/flags/imm_msb and the
        // add carry chain, all declared in define_trace_tables!
        for constraint in cols.constraints() {
            eval.add_constraint(constraint);
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
            cols.expected_opcode_id(),
            cols.rd_addr,
            cols.rs1_addr,
            cols.imm()
        );

        // Range check imm
        // - RC_8_11(imm_0, 2^8 * imm_1)
        add_to_relation!(
            eval,
            self.relations.range_check_8_11,
            -enabler.clone(),
            cols.imm_0,
            cols.imm_1_shifted()
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

        // Bitwise operations (for xor/or/and)
        // - is_bitwise * Bitwise(rs1[i], sext_imm[i], rd[i], bitwise_id) for each limb
        add_to_relation!(
            eval,
            self.relations.bitwise,
            -is_bitwise.clone(),
            cols.rs1_next_0,
            sext_imm[0].clone(),
            cols.rd_next_0,
            bitwise_id.clone()
        );
        add_to_relation!(
            eval,
            self.relations.bitwise,
            -is_bitwise.clone(),
            cols.rs1_next_1,
            sext_imm[1].clone(),
            cols.rd_next_1,
            bitwise_id.clone()
        );
        add_to_relation!(
            eval,
            self.relations.bitwise,
            -is_bitwise.clone(),
            cols.rs1_next_2,
            sext_imm[2].clone(),
            cols.rd_next_2,
            bitwise_id.clone()
        );
        add_to_relation!(
            eval,
            self.relations.bitwise,
            -is_bitwise.clone(),
            cols.rs1_next_3,
            sext_imm[3].clone(),
            cols.rd_next_3,
            bitwise_id.clone()
        );

        // Range check rd
        // - RC_8_8(rd[0], rd[1])
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            cols.rd_next_0,
            cols.rd_next_1
        );
        // - RC_8_8(rd[2], rd[3])
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            cols.rd_next_2,
            cols.rd_next_3
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
