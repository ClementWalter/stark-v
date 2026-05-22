//! AIR component for Load/Store (lb/lbu/lh/lhu/lw/sb/sh/sw) - airs.md Section 13

use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
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

        // Section 13.2: Variables
        let enabler = cols.opcode_lb_flag.clone()
            + cols.opcode_lh_flag.clone()
            + cols.opcode_lbu_flag.clone()
            + cols.opcode_lhu_flag.clone()
            + cols.opcode_lw_flag.clone()
            + cols.opcode_sb_flag.clone()
            + cols.opcode_sh_flag.clone()
            + cols.opcode_sw_flag.clone();

        let expected_opcode_id = cols.opcode_lb_flag.clone()
            * E::F::from(BaseField::from_u32_unchecked(Opcode::Lb as u32))
            + cols.opcode_lh_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Lh as u32))
            + cols.opcode_lbu_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Lbu as u32))
            + cols.opcode_lhu_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Lhu as u32))
            + cols.opcode_lw_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Lw as u32))
            + cols.opcode_sb_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Sb as u32))
            + cols.opcode_sh_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Sh as u32))
            + cols.opcode_sw_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Sw as u32));

        let markers = [
            cols.marker_0.clone(),
            cols.marker_1.clone(),
            cols.marker_2.clone(),
            cols.marker_3.clone(),
        ];

        let dst = [
            cols.dst_next_0.clone(),
            cols.dst_next_1.clone(),
            cols.dst_next_2.clone(),
            cols.dst_next_3.clone(),
        ];
        let base = [
            cols.rs1_next_0.clone(),
            cols.rs1_next_1.clone(),
            cols.rs1_next_2.clone(),
            cols.rs1_next_3.clone(),
        ];
        let src = [
            cols.src_next_0.clone(),
            cols.src_next_1.clone(),
            cols.src_next_2.clone(),
            cols.src_next_3.clone(),
        ];

        let pow2 = |exp: u32| E::F::from(BaseField::from_u32_unchecked(1 << exp));

        let mem_addr = base[0].clone()
            + pow2(8) * base[1].clone()
            + pow2(16) * base[2].clone()
            + pow2(24) * base[3].clone()
            + cols.imm_felt.clone();

        let sum_markers = markers
            .iter()
            .fold(E::F::zero(), |acc, marker| acc + marker.clone());
        let shift_id = markers
            .iter()
            .enumerate()
            .fold(E::F::zero(), |acc, (i, marker)| {
                acc + E::F::from(BaseField::from_u32_unchecked(i as u32)) * marker.clone()
            });

        let opcode_b_flag = cols.opcode_lbu_flag.clone()
            + cols.opcode_lb_flag.clone()
            + cols.opcode_sb_flag.clone();
        let opcode_h_flag = cols.opcode_lhu_flag.clone()
            + cols.opcode_lh_flag.clone()
            + cols.opcode_sh_flag.clone();
        let opcode_w_flag = cols.opcode_lw_flag.clone() + cols.opcode_sw_flag.clone();
        let is_signed = cols.opcode_lb_flag.clone() + cols.opcode_lh_flag.clone();
        // Load-only flags for sign extension constraints (stores don't sign-extend)
        let load_b_flag = cols.opcode_lb_flag.clone() + cols.opcode_lbu_flag.clone();
        let load_h_flag = cols.opcode_lh_flag.clone() + cols.opcode_lhu_flag.clone();
        let is_store =
            cols.opcode_sb_flag.clone() + cols.opcode_sh_flag.clone() + cols.opcode_sw_flag.clone();
        let is_load = enabler.clone() - is_store.clone();

        let reg_as = E::F::zero();
        let rw_as = E::F::one();
        let src_as = reg_as.clone() * is_store.clone() + rw_as.clone() * is_load.clone();
        let dst_as = reg_as * is_load.clone() + rw_as * is_store.clone();

        let _ = mem_addr;

        // Section 13.3: Constraints

        // enabler, opcode flags and markers are booleans
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));
        eval.add_constraint(
            cols.opcode_lb_flag.clone() * (E::F::one() - cols.opcode_lb_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_lh_flag.clone() * (E::F::one() - cols.opcode_lh_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_lbu_flag.clone() * (E::F::one() - cols.opcode_lbu_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_lhu_flag.clone() * (E::F::one() - cols.opcode_lhu_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_lw_flag.clone() * (E::F::one() - cols.opcode_lw_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_sb_flag.clone() * (E::F::one() - cols.opcode_sb_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_sh_flag.clone() * (E::F::one() - cols.opcode_sh_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_sw_flag.clone() * (E::F::one() - cols.opcode_sw_flag.clone()),
        );

        for marker in markers.iter() {
            eval.add_constraint(marker.clone() * (E::F::one() - marker.clone()));
        }

        // check shift amount
        // For bytes: shift_amount = shift_id (0-3)
        // For half-words: shift_id is 1 ([1,1,0,0]) or 5 ([0,0,1,1])
        //   shift_amount = (shift_id - 1) / 2, so 0 or 2
        let half_inv = BaseField::from_u32_unchecked(2).inverse();
        eval.add_constraint(
            cols.shift_amount.clone()
                - (opcode_b_flag.clone() * shift_id.clone()
                    + opcode_h_flag.clone() * (shift_id.clone() - E::F::one()) * half_inv),
        );

        // check src/dst addresses (load/store dependent)
        eval.add_constraint(
            cols.src_addr_selector.clone()
                - (is_load.clone() * (mem_addr.clone() - cols.shift_amount.clone())
                    + is_store.clone() * cols.r2_idx.clone()),
        );
        eval.add_constraint(
            cols.dst_addr_selector.clone()
                - (is_load.clone() * cols.r2_idx.clone()
                    + is_store.clone() * (mem_addr - cols.shift_amount.clone())),
        );

        // for lbu/sb markers contains a single one when row is enabled
        eval.add_constraint(opcode_b_flag.clone() * (E::F::one() - sum_markers.clone()));

        // for lhu/sh markers is either [1,1,0,0] or [0,0,1,1]
        eval.add_constraint(
            opcode_h_flag.clone()
                * (E::F::from(BaseField::from_u32_unchecked(2)) - sum_markers.clone()),
        );
        eval.add_constraint(
            opcode_h_flag.clone()
                * (E::F::one() - shift_id.clone())
                * (E::F::from(BaseField::from_u32_unchecked(5)) - shift_id.clone()),
        );

        let signed_mask = is_signed.clone() * cols.src_msb.clone() * (pow2(8) - E::F::one());

        // check that lb/lbu loads the correct byte (sign extension for loads only)
        eval.add_constraint(load_b_flag.clone() * (signed_mask.clone() - dst[1].clone()));
        eval.add_constraint(load_b_flag.clone() * (signed_mask.clone() - dst[2].clone()));
        eval.add_constraint(load_b_flag.clone() * (signed_mask.clone() - dst[3].clone()));
        // For loads: dst[0] = src[i] (load memory byte i into register byte 0)
        // For stores: dst[i] = src[0] (store register byte 0 into memory byte i)
        for (i, marker) in markers.iter().enumerate() {
            eval.add_constraint(
                load_b_flag.clone() * (dst[0].clone() - src[i].clone()) * marker.clone(),
            );
            eval.add_constraint(
                cols.opcode_sb_flag.clone() * (dst[i].clone() - src[0].clone()) * marker.clone(),
            );
        }

        // check that lh/lhu loads the correct half word (sign extension for loads only)
        eval.add_constraint(load_h_flag.clone() * (signed_mask.clone() - dst[2].clone()));
        eval.add_constraint(load_h_flag.clone() * (signed_mask - dst[3].clone()));

        let inv_four = BaseField::from_u32_unchecked(4).inverse();
        eval.add_constraint(
            load_h_flag.clone()
                * (E::F::from(BaseField::from_u32_unchecked(5)) - shift_id.clone())
                * inv_four
                * (dst[0].clone() - src[0].clone()),
        );
        eval.add_constraint(
            load_h_flag.clone()
                * (E::F::from(BaseField::from_u32_unchecked(5)) - shift_id.clone())
                * inv_four
                * (dst[1].clone() - src[1].clone()),
        );
        eval.add_constraint(
            load_h_flag.clone()
                * (shift_id.clone() - E::F::one())
                * inv_four
                * (dst[0].clone() - src[2].clone()),
        );
        eval.add_constraint(
            load_h_flag.clone()
                * (shift_id.clone() - E::F::one())
                * inv_four
                * (dst[1].clone() - src[3].clone()),
        );
        eval.add_constraint(
            cols.opcode_sh_flag.clone()
                * (E::F::from(BaseField::from_u32_unchecked(5)) - shift_id.clone())
                * inv_four
                * (dst[0].clone() - src[0].clone()),
        );
        eval.add_constraint(
            cols.opcode_sh_flag.clone()
                * (E::F::from(BaseField::from_u32_unchecked(5)) - shift_id.clone())
                * inv_four
                * (dst[1].clone() - src[1].clone()),
        );
        eval.add_constraint(
            cols.opcode_sh_flag.clone()
                * (shift_id.clone() - E::F::one())
                * inv_four
                * (dst[2].clone() - src[0].clone()),
        );
        eval.add_constraint(
            cols.opcode_sh_flag.clone()
                * (shift_id.clone() - E::F::one())
                * inv_four
                * (dst[3].clone() - src[1].clone()),
        );

        // check that lw/sw loads all the bytes
        for i in 0..4 {
            eval.add_constraint(opcode_w_flag.clone() * (dst[i].clone() - src[i].clone()));
        }

        // =====================================================================
        // LogUp Relations (Section 13.3 from airs.md)
        // =====================================================================

        let four = E::F::from(BaseField::from_u32_unchecked(4));

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
            cols.pc.clone() + four.clone(),
            cols.clock.clone() + E::F::one()
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
            cols.clock.clone() - cols.rs1_clock_prev.clone()
        );

        // Check that aligned memory address / 4 is in u20.
        // aligned memory address = src_addr_selector + dst_addr_selector - r2_idx.
        // This linear form equals the selected memory address for both loads and stores.
        let quarter_inv = BaseField::from_u32_unchecked(4).inverse();
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -enabler.clone(),
            (cols.src_addr_selector.clone() + cols.dst_addr_selector.clone() - cols.r2_idx.clone())
                * quarter_inv
        );

        // Check that base is a M31
        // - RC_M31(base[0], base[3])
        add_to_relation!(
            eval,
            self.relations.range_check_m31,
            -enabler.clone(),
            base[0].clone(),
            base[3].clone()
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
            src[3].clone()
        );
        // - RC_20(clock - src_prev_clock)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -enabler.clone(),
            cols.clock.clone() - cols.src_clock_prev.clone()
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
            dst[0].clone(),
            dst[1].clone(),
            dst[2].clone(),
            dst[3].clone()
        );
        // - RC_20(clock - dst_prev_clock)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -enabler.clone(),
            cols.clock.clone() - cols.dst_clock_prev.clone()
        );

        eval.finalize_logup_in_pairs();
        eval
    }
}
