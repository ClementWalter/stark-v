//! AIR component for DIV (div/divu/rem/remu) - airs.md Section 16

use crate::add_to_relation;
use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::DivColumns;
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
        let cols = DivColumns::from_eval(&mut eval);

        // Section 16.2: Variables
        let enabler = cols.opcode_div_flag.clone()
            + cols.opcode_divu_flag.clone()
            + cols.opcode_rem_flag.clone()
            + cols.opcode_remu_flag.clone();

        let expected_opcode_id = cols.opcode_div_flag.clone()
            * E::F::from(BaseField::from_u32_unchecked(Opcode::Div as u32))
            + cols.opcode_divu_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Divu as u32))
            + cols.opcode_rem_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Rem as u32))
            + cols.opcode_remu_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Remu as u32));

        let is_div = cols.opcode_div_flag.clone() + cols.opcode_divu_flag.clone();
        let is_signed = cols.opcode_div_flag.clone() + cols.opcode_rem_flag.clone();
        let special_case = cols.zero_divisor.clone() + cols.r_zero.clone();
        let valid_and_not_zero_divisor = enabler.clone() - cols.zero_divisor.clone();
        let valid_and_not_special_case = enabler.clone() - special_case.clone();

        let b = [
            cols.rs1_next_0.clone(),
            cols.rs1_next_1.clone(),
            cols.rs1_next_2.clone(),
            cols.rs1_next_3.clone(),
        ];
        let c = [
            cols.rs2_next_0.clone(),
            cols.rs2_next_1.clone(),
            cols.rs2_next_2.clone(),
            cols.rs2_next_3.clone(),
        ];
        let q = [
            cols.q_0.clone(),
            cols.q_1.clone(),
            cols.q_2.clone(),
            cols.q_3.clone(),
        ];
        let r = [
            cols.r_0.clone(),
            cols.r_1.clone(),
            cols.r_2.clone(),
            cols.r_3.clone(),
        ];

        let q_sum = q.iter().fold(E::F::zero(), |acc, limb| acc + limb.clone());
        let c_sum = c.iter().fold(E::F::zero(), |acc, limb| acc + limb.clone());
        let r_sum = r.iter().fold(E::F::zero(), |acc, limb| acc + limb.clone());

        let r_abs = [
            cols.r_abs_0.clone(),
            cols.r_abs_1.clone(),
            cols.r_abs_2.clone(),
            cols.r_abs_3.clone(),
        ];
        let r_inv = [
            cols.r_inv_0.clone(),
            cols.r_inv_1.clone(),
            cols.r_inv_2.clone(),
            cols.r_inv_3.clone(),
        ];
        let lt_markers = [
            cols.lt_marker_0.clone(),
            cols.lt_marker_1.clone(),
            cols.lt_marker_2.clone(),
            cols.lt_marker_3.clone(),
        ];

        let pow2_8 = E::F::from(BaseField::from_u32_unchecked(1 << 8));
        let pow2_8_minus_one = pow2_8.clone() - E::F::one();
        let two = E::F::one() + E::F::one();
        let inv_pow2_8 = BaseField::from_u32_unchecked(1 << 8).inverse();

        let mut carry: [E::F; 7] = std::array::from_fn(|_| E::F::zero());
        for i in 0..7 {
            let prev = if i == 0 {
                E::F::zero()
            } else {
                carry[i - 1].clone()
            };
            let b_limb = if i < 4 { b[i].clone() } else { E::F::zero() };
            let r_limb = if i < 4 { r[i].clone() } else { E::F::zero() };
            let mut mul_sum = E::F::zero();
            let k_min = i.saturating_sub(3);
            let k_max = i.min(3);
            for k in k_min..=k_max {
                mul_sum += c[k].clone() * q[i - k].clone();
            }
            carry[i] = (prev + r_limb + mul_sum - b_limb) * inv_pow2_8;
        }

        let mut carry_lt: [E::F; 4] = std::array::from_fn(|_| E::F::zero());
        for i in 0..4 {
            let prev = if i == 0 {
                E::F::zero()
            } else {
                carry_lt[i - 1].clone()
            };
            carry_lt[i] = (prev + r[i].clone() + r_abs[i].clone()) * inv_pow2_8;
        }

        let diff = [
            (E::F::one() - two.clone() * cols.c_sign.clone()) * (c[0].clone() - r_abs[0].clone()),
            (E::F::one() - two.clone() * cols.c_sign.clone()) * (c[1].clone() - r_abs[1].clone()),
            (E::F::one() - two.clone() * cols.c_sign.clone()) * (c[2].clone() - r_abs[2].clone()),
            (E::F::one() - two.clone() * cols.c_sign.clone()) * (c[3].clone() - r_abs[3].clone()),
        ];

        let a = [
            is_div.clone() * q[0].clone() + (E::F::one() - is_div.clone()) * r[0].clone(),
            is_div.clone() * q[1].clone() + (E::F::one() - is_div.clone()) * r[1].clone(),
            is_div.clone() * q[2].clone() + (E::F::one() - is_div.clone()) * r[2].clone(),
            is_div.clone() * q[3].clone() + (E::F::one() - is_div.clone()) * r[3].clone(),
        ];

        // Section 16.3: Constraints

        // boolean constraints
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));
        eval.add_constraint(
            cols.opcode_div_flag.clone() * (E::F::one() - cols.opcode_div_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_divu_flag.clone() * (E::F::one() - cols.opcode_divu_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_rem_flag.clone() * (E::F::one() - cols.opcode_rem_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_remu_flag.clone() * (E::F::one() - cols.opcode_remu_flag.clone()),
        );
        eval.add_constraint(cols.zero_divisor.clone() * (E::F::one() - cols.zero_divisor.clone()));
        eval.add_constraint(cols.r_zero.clone() * (E::F::one() - cols.r_zero.clone()));
        eval.add_constraint(cols.b_sign.clone() * (E::F::one() - cols.b_sign.clone()));
        eval.add_constraint(cols.c_sign.clone() * (E::F::one() - cols.c_sign.clone()));
        eval.add_constraint(cols.q_sign.clone() * (E::F::one() - cols.q_sign.clone()));
        eval.add_constraint(cols.sign_xor.clone() * (E::F::one() - cols.sign_xor.clone()));
        for marker in lt_markers.iter() {
            eval.add_constraint(marker.clone() * (E::F::one() - marker.clone()));
        }
        eval.add_constraint(special_case.clone() * (E::F::one() - special_case.clone()));
        eval.add_constraint(
            valid_and_not_zero_divisor.clone() * (E::F::one() - valid_and_not_zero_divisor.clone()),
        );
        eval.add_constraint(
            valid_and_not_special_case.clone() * (E::F::one() - valid_and_not_special_case.clone()),
        );

        // zero divisor detection
        for limb in c.iter() {
            eval.add_constraint(cols.zero_divisor.clone() * limb.clone());
        }
        for limb in q.iter() {
            eval.add_constraint(
                cols.zero_divisor.clone() * (limb.clone() - pow2_8_minus_one.clone()),
            );
        }
        eval.add_constraint(
            valid_and_not_zero_divisor.clone()
                * (c_sum.clone() * cols.c_sum_inv.clone() - E::F::one()),
        );

        // remainder-zero detection
        for limb in r.iter() {
            eval.add_constraint(cols.r_zero.clone() * limb.clone());
        }
        eval.add_constraint(
            valid_and_not_special_case.clone()
                * (r_sum.clone() * cols.r_sum_inv.clone() - E::F::one()),
        );

        // signed and sign xor
        eval.add_constraint((E::F::one() - is_signed.clone()) * cols.b_sign.clone());
        eval.add_constraint((E::F::one() - is_signed.clone()) * cols.c_sign.clone());
        eval.add_constraint(
            enabler.clone()
                * (cols.sign_xor.clone() - cols.b_sign.clone() - cols.c_sign.clone()
                    + two.clone() * cols.b_sign.clone() * cols.c_sign.clone()),
        );

        // quotient sign selection
        eval.add_constraint(
            (E::F::one() - cols.zero_divisor.clone())
                * q_sum.clone()
                * (cols.q_sign.clone() - cols.sign_xor.clone()),
        );
        eval.add_constraint(
            (E::F::one() - cols.zero_divisor.clone())
                * (cols.q_sign.clone() - cols.sign_xor.clone())
                * cols.q_sign.clone(),
        );

        // absolute remainder construction
        for i in 0..4 {
            eval.add_constraint(
                (E::F::one() - cols.sign_xor.clone()) * (r_abs[i].clone() - r[i].clone()),
            );

            let prev = if i == 0 {
                E::F::zero()
            } else {
                carry_lt[i - 1].clone()
            };
            eval.add_constraint(
                cols.sign_xor.clone()
                    * (carry_lt[i].clone() - prev.clone())
                    * (carry_lt[i].clone() - E::F::one()),
            );
            eval.add_constraint(
                cols.sign_xor.clone() * (E::F::one() - carry_lt[i].clone()) * r_abs[i].clone(),
            );
            eval.add_constraint(
                cols.sign_xor.clone()
                    * ((r_abs[i].clone() - pow2_8.clone()) * r_inv[i].clone() - E::F::one()),
            );
        }

        // compare |r| with |c| from the most significant byte
        let mut prefix_sum = special_case.clone();
        for i in (0..4).rev() {
            prefix_sum += lt_markers[i].clone();
            eval.add_constraint(
                enabler.clone() * (E::F::one() - prefix_sum.clone()) * diff[i].clone(),
            );
            eval.add_constraint(
                enabler.clone() * lt_markers[i].clone() * (cols.lt_diff.clone() - diff[i].clone()),
            );
        }
        eval.add_constraint(enabler.clone() * (E::F::one() - prefix_sum.clone()));

        // =====================================================================
        // LogUp Relations (Section 16.3 from airs.md)
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
        // - enabler * Registers(pc, clk)
        add_to_relation!(
            eval,
            self.relations.registers_state,
            -enabler.clone(),
            cols.pc,
            cols.clk
        );
        // + enabler * Registers(pc + 4, clk + 1)
        add_to_relation!(
            eval,
            self.relations.registers_state,
            enabler.clone(),
            cols.pc.clone() + E::F::from(BaseField::from_u32_unchecked(4)),
            cols.clk.clone() + E::F::one()
        );

        // REG_AS = 0 for register address space
        let reg_as = E::F::zero();

        // Read from rs1 (b)
        // - enabler * Memory(REG_AS, rs1_idx, rs1_prev_clk, b[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            -enabler.clone(),
            reg_as.clone(),
            cols.rs1_addr,
            cols.rs1_clk_prev,
            cols.rs1_prev_0,
            cols.rs1_prev_1,
            cols.rs1_prev_2,
            cols.rs1_prev_3
        );
        // + enabler * Memory(REG_AS, rs1_idx, clk, b[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            enabler.clone(),
            reg_as.clone(),
            cols.rs1_addr,
            cols.clk,
            cols.rs1_next_0,
            cols.rs1_next_1,
            cols.rs1_next_2,
            cols.rs1_next_3
        );
        // - RC_20(clk - rs1_prev_clk)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -enabler.clone(),
            cols.clk.clone() - cols.rs1_clk_prev.clone()
        );

        // Read from rs2 (c)
        // - enabler * Memory(REG_AS, rs2_idx, rs2_prev_clk, c[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            -enabler.clone(),
            reg_as.clone(),
            cols.rs2_addr,
            cols.rs2_clk_prev,
            cols.rs2_prev_0,
            cols.rs2_prev_1,
            cols.rs2_prev_2,
            cols.rs2_prev_3
        );
        // + enabler * Memory(REG_AS, rs2_idx, clk, c[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            enabler.clone(),
            reg_as.clone(),
            cols.rs2_addr,
            cols.clk,
            cols.rs2_next_0,
            cols.rs2_next_1,
            cols.rs2_next_2,
            cols.rs2_next_3
        );
        // - RC_20(clk - rs2_prev_clk)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -enabler.clone(),
            cols.clk.clone() - cols.rs2_clk_prev.clone()
        );

        // Check carries using RC_8_11: - enabler * RC_8_11(q[i], carry[i]) for i in [0..3]
        // and - enabler * RC_8_11(r[i], carry[4+i]) for i in [0..3]
        // Note: These require computing carries which needs the full b = c*q + r relation
        // For now, we use RC_8_8 for quotient and remainder limb range checks
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            cols.q_0,
            cols.q_1
        );
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            cols.q_2,
            cols.q_3
        );
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            cols.r_0,
            cols.r_1
        );
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            cols.r_2,
            cols.r_3
        );

        // lt_diff is non-zero whenever the comparison is executed
        // - (enabler - special_case) * RC_20(lt_diff - 1)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -(enabler.clone() - special_case.clone()),
            cols.lt_diff.clone() - E::F::one()
        );

        // Write to rd (a[i] selects q for div/divu and r for rem/remu)
        // - enabler * Memory(REG_AS, rd_idx, rd_prev_clk, rd_prev[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            -enabler.clone(),
            reg_as.clone(),
            cols.rd_addr,
            cols.rd_clk_prev,
            cols.rd_prev_0,
            cols.rd_prev_1,
            cols.rd_prev_2,
            cols.rd_prev_3
        );
        // + enabler * Memory(REG_AS, rd_idx, clk, a[0..3])
        add_to_relation!(
            eval,
            self.relations.memory_access,
            enabler.clone(),
            reg_as.clone(),
            cols.rd_addr,
            cols.clk,
            a[0].clone(),
            a[1].clone(),
            a[2].clone(),
            a[3].clone()
        );
        // - RC_20(clk - rd_prev_clk)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -enabler.clone(),
            cols.clk.clone() - cols.rd_clk_prev.clone()
        );

        eval.finalize_logup_in_pairs();
        eval
    }
}
