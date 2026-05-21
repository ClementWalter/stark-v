//! AIR evaluation for the program component.

use super::columns::ProgramColumns;
use super::*;

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
        let cols = ProgramColumns::from_eval(&mut eval);
        let enabler = cols.enabler.clone();
        let addr = cols.addr.clone();
        let value_0 = cols.value_0.clone();
        let value_1 = cols.value_1.clone();
        let value_2 = cols.value_2.clone();
        let value_3 = cols.value_3.clone();
        let multiplicity = cols.multiplicity.clone();
        let root = cols.root.clone();

        let leaf_depth = E::F::from(M31::from(MAX_TREE_HEIGHT - 1));
        let one = E::F::one();
        let two = one.clone() + one.clone();
        let three = two.clone() + one.clone();

        eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

        add_to_relation!(
            eval,
            self.relations.program_access,
            multiplicity.clone(),
            addr,
            value_0,
            value_1,
            value_2,
            value_3
        );

        let index_base = addr;
        add_to_relation!(
            eval,
            self.relations.merkle,
            -enabler.clone(),
            index_base.clone(),
            leaf_depth.clone(),
            value_0,
            root.clone()
        );

        add_to_relation!(
            eval,
            self.relations.merkle,
            -enabler.clone(),
            index_base.clone() + one.clone(),
            leaf_depth.clone(),
            value_1,
            root.clone()
        );
        add_to_relation!(
            eval,
            self.relations.merkle,
            -enabler.clone(),
            index_base.clone() + two.clone(),
            leaf_depth.clone(),
            value_2,
            root.clone()
        );
        add_to_relation!(
            eval,
            self.relations.merkle,
            -enabler,
            index_base + three,
            leaf_depth,
            value_3,
            root
        );
        eval.finalize_logup_in_pairs();
        eval
    }
}
