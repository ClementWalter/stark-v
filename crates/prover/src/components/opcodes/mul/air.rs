//! AIR component for MUL - airs.md Section 14

use num_traits::{One, Zero};
use runner::decode::Opcode;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::MulColumns;
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
        let cols = MulColumns::from_eval(&mut eval);

        // Section 14.2: Variables
        let rd = [
            cols.rd_next_0.clone(),
            cols.rd_next_1.clone(),
            cols.rd_next_2.clone(),
            cols.rd_next_3.clone(),
        ];
        let rs1 = [
            cols.rs1_next_0.clone(),
            cols.rs1_next_1.clone(),
            cols.rs1_next_2.clone(),
            cols.rs1_next_3.clone(),
        ];
        let rs2 = [
            cols.rs2_next_0.clone(),
            cols.rs2_next_1.clone(),
            cols.rs2_next_2.clone(),
            cols.rs2_next_3.clone(),
        ];

        let inv_two_pow_8 = BaseField::from_u32_unchecked(1 << 8).inverse();
        let mut carry: [E::F; 4] = std::array::from_fn(|_| E::F::zero());
        carry[0] = (rs1[0].clone() * rs2[0].clone() - rd[0].clone()) * inv_two_pow_8;
        for i in 1..4 {
            let mut limb_sum = carry[i - 1].clone() + rs1[i].clone() * rs2[0].clone();
            for k in 1..=i {
                limb_sum += rs1[i - k].clone() * rs2[k].clone();
            }
            carry[i] = (limb_sum - rd[i].clone()) * inv_two_pow_8;
        }

        let opcode_mul_id = E::F::from(BaseField::from_u32_unchecked(Opcode::Mul as u32));
        let _ = (carry, opcode_mul_id);

        // enabler is boolean
        eval.add_constraint(cols.enabler.clone() * (E::F::one() - cols.enabler.clone()));

        // =====================================================================
        // LogUp Relations (from airs.md)
        // TODO: Implement using add_to_relation! macro
        //
        // Example usage:
        // add_to_relation!(eval, self.relations.program_access, -cols.enabler.clone(),
        //     cols.pc, opcode_id, cols.rd_addr, cols.rs1_addr, cols.rs2_addr);
        //
        // See base_alu_reg/air.rs for detailed examples.
        // =====================================================================
        eval
    }
}
