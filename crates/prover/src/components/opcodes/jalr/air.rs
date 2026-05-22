//! AIR component for JALR - airs.md Section 11

use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use crate::relations::Relations;
use runner::trace::prover_columns::JalrColumns;

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
        let cols = JalrColumns::from_eval(&mut eval);

        // Section 11.2: Variables
        let rs1 = [
            cols.rs1_next_0.clone(),
            cols.rs1_next_1.clone(),
            cols.rs1_next_2.clone(),
            cols.rs1_next_3.clone(),
        ];
        let rd = [
            cols.rd_next_0.clone(),
            cols.rd_next_1.clone(),
            cols.rd_next_2.clone(),
            cols.rd_next_3.clone(),
        ];

        let rs1_felt = rs1[0].clone()
            + pow2::<E>(8) * rs1[1].clone()
            + pow2::<E>(16) * rs1[2].clone()
            + pow2::<E>(24) * rs1[3].clone();
        let rd_felt = rd[0].clone()
            + pow2::<E>(8) * rd[1].clone()
            + pow2::<E>(16) * rd[2].clone()
            + pow2::<E>(24) * rd[3].clone();

        let opcode_jalr_id = E::F::from(BaseField::from_u32_unchecked(Opcode::Jalr as u32));

        // REG_AS = 0 for register address space
        let reg_as = E::F::zero();

        // enabler is boolean
        eval.add_constraint(cols.enabler.clone() * (E::F::one() - cols.enabler.clone()));

        // to_pc_lsb is boolean
        eval.add_constraint(cols.to_pc_lsb.clone() * (E::F::one() - cols.to_pc_lsb.clone()));

        // check next pc: 2 * to_pc_over_two + to_pc_lsb = rs1_felt + imm_felt
        eval.add_constraint(
            cols.to_pc_over_two.clone() * pow2::<E>(1) + cols.to_pc_lsb.clone()
                - (rs1_felt + cols.imm_felt.clone()),
        );

        // rd is pc + 4 (gated by rd_addr for x0 writes)
        // When rd_addr = 0 (x0), the write is discarded and rd_next = 0, so skip this constraint
        eval.add_constraint(
            cols.enabler.clone()
                * cols.rd_addr.clone()
                * (rd_felt - (cols.pc.clone() + E::F::from(BaseField::from_u32_unchecked(4)))),
        );

        // =====================================================================
        // LogUp Relations (Section 11.3 from airs.md)
        // =====================================================================

        // Program access (I-type): - enabler * Program(pc, opcode_jalr_id, rd_idx, rs1_idx, imm_felt)
        add_to_relation!(
            eval,
            self.relations.program_access,
            -cols.enabler.clone(),
            cols.pc,
            opcode_jalr_id.clone(),
            cols.rd_addr,
            cols.rs1_addr,
            cols.imm_felt
        );

        // Read from rs1
        // - enabler * Memory(REG_AS, rs1_idx, rs1_prev_clock, rs1[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            -cols.enabler.clone(),
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
            cols.enabler.clone(),
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
            -cols.enabler.clone(),
            cols.clock.clone() - cols.rs1_clock_prev.clone()
        );

        // Check that rs1 is a M31
        // - RC_M31(rs1[0], rs1[3])
        add_to_relation!(
            eval,
            self.relations.range_check_m31,
            -cols.enabler.clone(),
            rs1[0].clone(),
            rs1[3].clone()
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
        // + enabler * Registers(2 * to_pc_over_two, clock + 1)
        add_to_relation!(
            eval,
            self.relations.registers_state,
            cols.enabler.clone(),
            cols.to_pc_over_two.clone() * pow2::<E>(1),
            cols.clock.clone() + E::F::one()
        );

        // Check that rd is a M31
        // - RC_8_8(rd[1], rd[2])
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -cols.enabler.clone(),
            rd[1].clone(),
            rd[2].clone()
        );
        // - RC_M31(rd[0], rd[3])
        add_to_relation!(
            eval,
            self.relations.range_check_m31,
            -cols.enabler.clone(),
            rd[0].clone(),
            rd[3].clone()
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
        // + enabler * Memory(REG_AS, rd_idx, clock, rd[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            cols.enabler.clone(),
            reg_as.clone(),
            cols.rd_addr,
            cols.clock,
            rd[0].clone(),
            rd[1].clone(),
            rd[2].clone(),
            rd[3].clone()
        );
        // - RC_20(clock - rd_prev_clock)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -cols.enabler.clone(),
            cols.clock.clone() - cols.rd_clock_prev.clone()
        );

        eval.finalize_logup_in_pairs();
        eval
    }
}
