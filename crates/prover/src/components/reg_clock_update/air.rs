//! AIR evaluation for the register clock update component.

use super::*;
use runner::trace::prover_columns::RegClockUpdateColumns;

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
        let cols = RegClockUpdateColumns::from_eval(&mut eval);
        let enabler = cols.enabler.clone();
        let addr = cols.addr.clone();
        let clock_prev = cols.clock_prev.clone();
        let value_0 = cols.value_0.clone();
        let value_1 = cols.value_1.clone();
        let value_2 = cols.value_2.clone();
        let value_3 = cols.value_3.clone();

        let one = E::F::one();
        let diff = E::F::from(M31::from(DEFAULT_MAX_CLOCK_DIFF));

        eval.add_constraint(enabler.clone() * (one - enabler.clone()));

        let reg_as = E::F::zero();
        add_to_relation!(
            eval,
            self.relations.memory_access,
            -enabler.clone(),
            reg_as.clone(),
            addr.clone(),
            clock_prev.clone(),
            value_0.clone(),
            value_1.clone(),
            value_2.clone(),
            value_3.clone()
        );
        add_to_relation!(
            eval,
            self.relations.memory_access,
            enabler,
            reg_as,
            addr,
            clock_prev + diff,
            value_0,
            value_1,
            value_2,
            value_3
        );
        eval.finalize_logup_in_pairs();
        eval
    }
}
