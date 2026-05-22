//! AIR evaluation for the Merkle component.

use super::*;
use runner::trace::prover_columns::MerkleColumns;

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
        let cols = MerkleColumns::from_eval(&mut eval);
        let enabler = cols.enabler.clone();
        let index = cols.index.clone();
        let depth = cols.depth.clone();
        let lhs = cols.lhs.clone();
        let rhs = cols.rhs.clone();
        let cur = cols.cur.clone();
        let lhs_mult = cols.lhs_mult.clone();
        let rhs_mult = cols.rhs_mult.clone();
        let cur_mult = cols.cur_mult.clone();
        let root = cols.root.clone();

        let one = E::F::one();
        let two = one.clone() + one.clone();
        let inv2 = E::F::from(M31::inverse(&M31::from(2)));

        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));
        eval.add_constraint(
            lhs_mult.clone() * (lhs_mult.clone() - one.clone()) * (lhs_mult.clone() - two.clone()),
        );
        eval.add_constraint(
            rhs_mult.clone() * (rhs_mult.clone() - one.clone()) * (rhs_mult.clone() - two.clone()),
        );
        eval.add_constraint(
            cur_mult.clone() * (cur_mult.clone() - one.clone()) * (cur_mult.clone() - two.clone()),
        );

        add_to_relation!(
            eval,
            self.relations.merkle,
            lhs_mult,
            index.clone(),
            depth.clone(),
            lhs.clone(),
            root.clone()
        );
        add_to_relation!(
            eval,
            self.relations.merkle,
            rhs_mult,
            index.clone() + one.clone(),
            depth.clone(),
            rhs.clone(),
            root.clone()
        );
        add_to_relation!(
            eval,
            self.relations.merkle,
            -cur_mult,
            index * inv2,
            depth - one.clone(),
            cur.clone(),
            root
        );

        add_to_relation!(eval, self.relations.poseidon2, enabler.clone(), lhs, rhs);
        add_to_relation!(eval, self.relations.poseidon2, -enabler, cur);
        eval.finalize_logup_in_pairs();
        eval
    }
}
