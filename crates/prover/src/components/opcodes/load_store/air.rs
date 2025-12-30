//! AIR component for Load/Store (lb/lbu/lh/lhu/lw/sb/sh/sw) - airs.md Section 13

use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::LoadStoreColumns;
use crate::relations::Relations;

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
            cols.src_next_3.clone()
                + cols.src_msb.clone() * E::F::from(BaseField::from_u32_unchecked(1 << 7)),
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

        let _ = (expected_opcode_id, src_as, dst_as, mem_addr.clone());

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
        //   shift_amount = (shift_id - 1) / 4, so 0 or 1
        let quarter_inv = BaseField::from_u32_unchecked(4).inverse();
        eval.add_constraint(
            cols.shift_amount.clone()
                - (opcode_b_flag.clone() * shift_id.clone()
                    + opcode_h_flag.clone() * (shift_id.clone() - E::F::one()) * quarter_inv),
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
            opcode_h_flag.clone()
                * (E::F::from(BaseField::from_u32_unchecked(5)) - shift_id.clone())
                * inv_four
                * (dst[0].clone() - src[0].clone()),
        );
        eval.add_constraint(
            opcode_h_flag.clone()
                * (E::F::from(BaseField::from_u32_unchecked(5)) - shift_id.clone())
                * inv_four
                * (dst[1].clone() - src[1].clone()),
        );
        eval.add_constraint(
            opcode_h_flag.clone()
                * (shift_id.clone() - E::F::one())
                * inv_four
                * (dst[0].clone() - src[2].clone()),
        );
        eval.add_constraint(
            opcode_h_flag.clone()
                * (shift_id.clone() - E::F::one())
                * inv_four
                * (dst[1].clone() - src[3].clone()),
        );

        // check that lw/sw loads all the bytes
        for i in 0..4 {
            eval.add_constraint(opcode_w_flag.clone() * (dst[i].clone() - src[i].clone()));
        }

        eval
    }
}
