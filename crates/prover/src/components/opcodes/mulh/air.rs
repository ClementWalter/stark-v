//! AIR component for MULH (mulh/mulhsu/mulhu) - airs.md Section 15

use num_traits::{One, Zero};
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

        // Section 15.2: Variables
        let enabler = cols.opcode_mulh_flag.clone()
            + cols.opcode_mulhsu_flag.clone()
            + cols.opcode_mulhu_flag.clone();

        let expected_opcode_id = cols.opcode_mulh_flag.clone()
            * E::F::from(BaseField::from_u32_unchecked(Opcode::Mulh as u32))
            + cols.opcode_mulhsu_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Mulhsu as u32))
            + cols.opcode_mulhu_flag.clone()
                * E::F::from(BaseField::from_u32_unchecked(Opcode::Mulhu as u32));

        // rd[0..3] are the low bytes (rd), rd[4..7] are the high bytes (the actual result written)
        let rd_low = [
            cols.rd_high_0.clone(),
            cols.rd_high_1.clone(),
            cols.rd_high_2.clone(),
            cols.rd_high_3.clone(),
        ];
        let rd_high = [
            cols.rd_next_0.clone(),
            cols.rd_next_1.clone(),
            cols.rd_next_2.clone(),
            cols.rd_next_3.clone(),
        ];

        // Sign-extended rs1 and rs2 (only the high bit matters for sign)
        let rs1_with_sign = [
            cols.rs1_next_0.clone(),
            cols.rs1_next_1.clone(),
            cols.rs1_next_2.clone(),
            cols.rs1_next_3.clone() + cols.rs1_sign.clone() * pow2::<E>(7),
        ];
        let rs2_with_sign = [
            cols.rs2_next_0.clone(),
            cols.rs2_next_1.clone(),
            cols.rs2_next_2.clone(),
            cols.rs2_next_3.clone() + cols.rs2_sign.clone() * pow2::<E>(7),
        ];

        // Compute carries for 8-byte multiplication
        let inv_two_pow_8 = BaseField::from_u32_unchecked(1 << 8).inverse();
        let sign_ext = pow2::<E>(8) - E::F::one();

        // Extended operands for 8-byte multiplication
        let rs1_ext: [E::F; 8] = std::array::from_fn(|i| {
            if i < 4 {
                rs1_with_sign[i].clone()
            } else {
                cols.rs1_sign.clone() * sign_ext.clone()
            }
        });
        let rs2_ext: [E::F; 8] = std::array::from_fn(|i| {
            if i < 4 {
                rs2_with_sign[i].clone()
            } else {
                cols.rs2_sign.clone() * sign_ext.clone()
            }
        });
        let rd_full: [E::F; 8] = std::array::from_fn(|i| {
            if i < 4 {
                rd_low[i].clone()
            } else {
                rd_high[i - 4].clone()
            }
        });

        let mut carry: [E::F; 8] = std::array::from_fn(|_| E::F::zero());
        for i in 0..8 {
            let prev_carry = if i == 0 {
                E::F::zero()
            } else {
                carry[i - 1].clone()
            };
            let mut limb_sum = prev_carry;
            for k in 0..=i.min(7) {
                if i - k < 8 {
                    limb_sum += rs1_ext[k].clone() * rs2_ext[i - k].clone();
                }
            }
            carry[i] = (limb_sum - rd_full[i].clone()) * inv_two_pow_8;
        }

        // REG_AS = 0 for register address space
        let reg_as = E::F::zero();

        // Section 15.3: Constraints

        // enabler, opcode_i_flag and rs_signs are booleans
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));
        eval.add_constraint(
            cols.opcode_mulh_flag.clone() * (E::F::one() - cols.opcode_mulh_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_mulhsu_flag.clone() * (E::F::one() - cols.opcode_mulhsu_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_mulhu_flag.clone() * (E::F::one() - cols.opcode_mulhu_flag.clone()),
        );
        eval.add_constraint(cols.rs1_sign.clone() * (E::F::one() - cols.rs1_sign.clone()));
        eval.add_constraint(cols.rs2_sign.clone() * (E::F::one() - cols.rs2_sign.clone()));

        // check the signs of the operand extensions
        eval.add_constraint(
            (cols.opcode_mulhsu_flag.clone() + cols.opcode_mulhu_flag.clone())
                * cols.rs2_sign.clone(),
        );
        eval.add_constraint(cols.opcode_mulhu_flag.clone() * cols.rs1_sign.clone());

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
            cols.pc.clone() + E::F::from(BaseField::from_u32_unchecked(4)),
            cols.clock.clone() + E::F::one()
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
            cols.clock.clone() - cols.rs1_clock_prev.clone()
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
            cols.clock.clone() - cols.rs2_clock_prev.clone()
        );

        // Check carries: - RC_8_8(carry[i], carry[i+1]) for i in 0,2,4,6
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            carry[0].clone(),
            carry[1].clone()
        );
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            carry[2].clone(),
            carry[3].clone()
        );
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            carry[4].clone(),
            carry[5].clone()
        );
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            carry[6].clone(),
            carry[7].clone()
        );

        // Range check rd (low and high parts)
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            rd_low[0].clone(),
            rd_low[1].clone()
        );
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            rd_low[2].clone(),
            rd_low[3].clone()
        );
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            rd_high[0].clone(),
            rd_high[1].clone()
        );
        add_to_relation!(
            eval,
            self.relations.range_check_8_8,
            -enabler.clone(),
            rd_high[2].clone(),
            rd_high[3].clone()
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
            rd_high[0].clone(),
            rd_high[1].clone(),
            rd_high[2].clone(),
            rd_high[3].clone()
        );
        // - RC_20(clock - rd_prev_clock)
        add_to_relation!(
            eval,
            self.relations.range_check_20,
            -enabler.clone(),
            cols.clock.clone() - cols.rd_clock_prev.clone()
        );

        // Keep carry-related RC_8_8 entries as singleton batches to avoid multiplying
        // quadratic carry denominators with other denominators in the same batch.
        eval.finalize_logup_batched(&vec![
            0, 0, 1, 1, 2, 2, 3, 3, 4, 5, 6, 7, 8, 4, 9, 9, 10, 10, 11, 11,
        ]);
        eval
    }
}
