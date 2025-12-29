//! AIR component for Shifts Reg (sll/srl/sra) - airs.md Section 3

use num_traits::One;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::Shifts_regColumns;
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
        let cols = Shifts_regColumns::from_eval(&mut eval);

        // Section 3.2: Variables
        let enabler = cols.opcode_sll_flag.clone()
            + cols.opcode_srl_flag.clone()
            + cols.opcode_sra_flag.clone();

        // Section 3.3: Constraints

        // enabler is boolean
        eval.add_constraint(enabler.clone() * (E::F::one() - enabler.clone()));

        // opcode flags are booleans
        eval.add_constraint(
            cols.opcode_sll_flag.clone() * (E::F::one() - cols.opcode_sll_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_srl_flag.clone() * (E::F::one() - cols.opcode_srl_flag.clone()),
        );
        eval.add_constraint(
            cols.opcode_sra_flag.clone() * (E::F::one() - cols.opcode_sra_flag.clone()),
        );

        // rs1_sign is boolean
        eval.add_constraint(cols.rs1_sign.clone() * (E::F::one() - cols.rs1_sign.clone()));

        // bit_shift_marker[i] are booleans
        eval.add_constraint(
            cols.bit_shift_marker_0.clone() * (E::F::one() - cols.bit_shift_marker_0.clone()),
        );
        eval.add_constraint(
            cols.bit_shift_marker_1.clone() * (E::F::one() - cols.bit_shift_marker_1.clone()),
        );
        eval.add_constraint(
            cols.bit_shift_marker_2.clone() * (E::F::one() - cols.bit_shift_marker_2.clone()),
        );
        eval.add_constraint(
            cols.bit_shift_marker_3.clone() * (E::F::one() - cols.bit_shift_marker_3.clone()),
        );
        eval.add_constraint(
            cols.bit_shift_marker_4.clone() * (E::F::one() - cols.bit_shift_marker_4.clone()),
        );
        eval.add_constraint(
            cols.bit_shift_marker_5.clone() * (E::F::one() - cols.bit_shift_marker_5.clone()),
        );
        eval.add_constraint(
            cols.bit_shift_marker_6.clone() * (E::F::one() - cols.bit_shift_marker_6.clone()),
        );
        eval.add_constraint(
            cols.bit_shift_marker_7.clone() * (E::F::one() - cols.bit_shift_marker_7.clone()),
        );

        // limb_shift_marker[i] are booleans
        eval.add_constraint(
            cols.limb_shift_marker_0.clone() * (E::F::one() - cols.limb_shift_marker_0.clone()),
        );
        eval.add_constraint(
            cols.limb_shift_marker_1.clone() * (E::F::one() - cols.limb_shift_marker_1.clone()),
        );
        eval.add_constraint(
            cols.limb_shift_marker_2.clone() * (E::F::one() - cols.limb_shift_marker_2.clone()),
        );
        eval.add_constraint(
            cols.limb_shift_marker_3.clone() * (E::F::one() - cols.limb_shift_marker_3.clone()),
        );

        // Sum of bit_shift_marker = enabler
        let bit_marker_sum = cols.bit_shift_marker_0.clone()
            + cols.bit_shift_marker_1.clone()
            + cols.bit_shift_marker_2.clone()
            + cols.bit_shift_marker_3.clone()
            + cols.bit_shift_marker_4.clone()
            + cols.bit_shift_marker_5.clone()
            + cols.bit_shift_marker_6.clone()
            + cols.bit_shift_marker_7.clone();
        eval.add_constraint(bit_marker_sum - enabler.clone());

        // Sum of limb_shift_marker = enabler
        let limb_marker_sum = cols.limb_shift_marker_0.clone()
            + cols.limb_shift_marker_1.clone()
            + cols.limb_shift_marker_2.clone()
            + cols.limb_shift_marker_3.clone();
        eval.add_constraint(limb_marker_sum - enabler.clone());


        eval
    }
}
