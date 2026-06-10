//! AIR component for Load/Store (lb/lbu/lh/lhu/lw/sb/sh/sw) - airs.md Section 13

use num_traits::Zero;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use crate::relations::Relations;
use runner::trace::prover_columns::LoadStoreColumns;

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
        let cols = LoadStoreColumns::from_eval(&mut eval);

        // Section 13.2/13.3: derived columns and constraints, declared in
        // define_trace_tables!
        let enabler = cols.enabler();
        let expected_opcode_id = cols.expected_opcode_id();
        let src_as = cols.src_as();
        let dst_as = cols.dst_as();

        for constraint in cols.constraints() {
            eval.add_constraint(constraint);
        }

        // =====================================================================
        // LogUp Relations (Section 13.3 from airs.md)
        // =====================================================================

        // Program access (I-type for loads, S-type for stores)
        // - enabler * Program(pc, expected_opcode_id, rs1_idx, r2_idx, imm_felt)
        add_to_relation!(
            eval,
            self.relations.program_access,
            -enabler.clone(),
            cols.pc,
            expected_opcode_id.clone(),
            cols.rs1_addr,
            cols.r2_idx,
            cols.imm_felt
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

        // Read from rs1 (base address)
        // - enabler * Memory(REG_AS, rs1_idx, rs1_prev_clock, base[0..3])
        let reg_as = E::F::zero();
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
        // + enabler * Memory(REG_AS, rs1_idx, clock, base[0..3])
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

        // Check that aligned memory address / 4 is in u20.
        // aligned memory address = src_addr_selector + dst_addr_selector - r2_idx.
        // This linear form equals the selected memory address for both loads and stores.
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -enabler.clone(),
            cols.aligned_addr_quarter()
        );

        // Check that base is a M31
        // - RC_M31(base[0], base[3])
        add_to_relation!(
            eval,
            self.relations.range_check_m31,
            -enabler.clone(),
            cols.rs1_next_0,
            cols.rs1_next_3
        );

        // Read src
        // - enabler * Memory(src_as, src_addr, src_prev_clock, src[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            -enabler.clone(),
            src_as.clone(),
            cols.src_addr_selector,
            cols.src_clock_prev,
            cols.src_prev_0,
            cols.src_prev_1,
            cols.src_prev_2,
            cols.src_prev_3
        );
        // + enabler * Memory(src_as, src_addr, clock, src[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            enabler.clone(),
            src_as.clone(),
            cols.src_addr_selector,
            cols.clock,
            cols.src_next_0,
            cols.src_next_1,
            cols.src_next_2,
            cols.src_next_3
        );
        // - RC_20(clock - src_prev_clock)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -enabler.clone(),
            cols.src_clock_diff()
        );

        // Write into dst
        // - enabler * Memory(dst_as, dst_addr, dst_prev_clock, dst_prev[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            -enabler.clone(),
            dst_as.clone(),
            cols.dst_addr_selector,
            cols.dst_clock_prev,
            cols.dst_prev_0,
            cols.dst_prev_1,
            cols.dst_prev_2,
            cols.dst_prev_3
        );
        // + enabler * Memory(dst_as, dst_addr, clock, dst[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            enabler.clone(),
            dst_as.clone(),
            cols.dst_addr_selector,
            cols.clock,
            cols.dst_next_0,
            cols.dst_next_1,
            cols.dst_next_2,
            cols.dst_next_3
        );
        // - RC_20(clock - dst_prev_clock)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -enabler.clone(),
            cols.dst_clock_diff()
        );

        eval.finalize_logup_in_pairs();
        eval
    }
}
