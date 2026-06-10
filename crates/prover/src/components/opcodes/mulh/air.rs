//! AIR component for MULH (mulh/mulhsu/mulhu) - airs.md Section 15

use num_traits::Zero;
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use crate::relations::Relations;
use runner::trace::prover_columns::MulhColumns;

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
        let cols = MulhColumns::from_eval(&mut eval);

        // Section 15.2/15.3: derived columns (sign-extended operands, the
        // 8-limb carry chain) and constraints, declared in
        // define_trace_tables!
        let enabler = cols.enabler();
        let expected_opcode_id = cols.expected_opcode_id();

        // REG_AS = 0 for register address space
        let reg_as = E::F::zero();

        for constraint in cols.constraints() {
            eval.add_constraint(constraint);
        }

        // =====================================================================
        // LogUp Relations (Section 15.3 from airs.md)
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

        // Check carries: - RC_8_8(carry[i], carry[i+1]) for i in 0,2,4,6
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            cols.carry_0(),
            cols.carry_1()
        );
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            cols.carry_2(),
            cols.carry_3()
        );
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            cols.carry_4(),
            cols.carry_5()
        );
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            cols.carry_6(),
            cols.carry_7()
        );

        // Range check rd (low and high parts)
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            cols.rd_high_0,
            cols.rd_high_1
        );
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            cols.rd_high_2,
            cols.rd_high_3
        );
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

        // Write to rd (only high bytes, rd[4..7])
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
        // + enabler * Memory(REG_AS, rd_idx, clock, rd[4..7])
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

        // The carry range-check denominators are quadratic in the trace, so
        // they cannot share a batch with any other denominator without
        // breaching the constraint degree bound; the framework only supports
        // uniform batch sizes, hence batch size 1 for the whole component.
        eval.finalize_logup_batched(1);
        eval
    }
}
