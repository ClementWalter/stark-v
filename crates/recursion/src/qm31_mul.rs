//! QM31 multiplication component: witness generation and AIR evaluation.
//!
//! The constraints live in the `define_component_tables!` invocation in
//! `lib.rs`; this module only wires them into a `FrameworkEval` and fills the
//! trace table from actual QM31 products, so stwo's field arithmetic is the
//! oracle the AIR is tested against.

use stwo::core::fields::qm31::QM31;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use crate::Qm31MulTable;
use crate::prover_columns::Qm31MulColumns;

pub type Component = FrameworkComponent<Eval>;

#[derive(Clone)]
pub struct Eval {
    pub log_size: u32,
}

impl FrameworkEval for Eval {
    fn log_size(&self) -> u32 {
        self.log_size
    }

    fn max_constraint_log_degree_bound(&self) -> u32 {
        self.log_size + 1
    }

    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        let cols = Qm31MulColumns::from_eval(&mut eval);
        for constraint in cols.constraints() {
            eval.add_constraint(constraint);
        }
        eval
    }
}

/// Record `a * b` in the trace table and return the product.
pub fn push_mul(table: &mut Qm31MulTable, a: QM31, b: QM31) -> QM31 {
    let c = a * b;
    let a = a.to_m31_array();
    let b = b.to_m31_array();
    let limbs = c.to_m31_array();
    table.push(
        a[0].0, a[1].0, a[2].0, a[3].0, b[0].0, b[1].0, b[2].0, b[3].0, limbs[0].0, limbs[1].0,
        limbs[2].0, limbs[3].0,
    );
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::Zero;
    use rand::rngs::SmallRng;
    use rand::{Rng, SeedableRng};
    use stwo::core::fields::qm31::SecureField;
    use stwo::core::pcs::TreeVec;
    use stwo::core::poly::circle::CanonicCoset;
    use stwo_constraint_framework::assert_constraints_on_polys;

    fn random_qm31(rng: &mut SmallRng) -> QM31 {
        QM31::from_u32_unchecked(
            rng.gen_range(0..(1 << 30)),
            rng.gen_range(0..(1 << 30)),
            rng.gen_range(0..(1 << 30)),
            rng.gen_range(0..(1 << 30)),
        )
    }

    fn assert_table_satisfies_constraints(table: Qm31MulTable) {
        let trace = table.into_witness();
        let log_size = trace
            .first()
            .map(|t| t.domain.log_size())
            .expect("empty trace");
        let traces = TreeVec::new(vec![vec![], trace, vec![]]);
        let trace_polys = traces.map_cols(|c| c.interpolate());
        let eval = Eval { log_size };
        assert_constraints_on_polys(
            &trace_polys,
            CanonicCoset::new(log_size),
            |e| {
                eval.evaluate(e);
            },
            SecureField::zero(),
        );
    }

    #[test]
    fn test_qm31_mul_constraints_hold_on_random_products() {
        let mut rng = SmallRng::seed_from_u64(0);
        let mut table = Qm31MulTable::new();
        for _ in 0..100 {
            let a = random_qm31(&mut rng);
            let b = random_qm31(&mut rng);
            let expected = a * b;
            assert_eq!(push_mul(&mut table, a, b), expected);
        }
        assert_table_satisfies_constraints(table);
    }

    #[test]
    #[should_panic]
    fn test_qm31_mul_constraints_reject_wrong_product() {
        let mut rng = SmallRng::seed_from_u64(1);
        let mut table = Qm31MulTable::new();
        let a = random_qm31(&mut rng);
        let b = random_qm31(&mut rng);
        let a_limbs = a.to_m31_array();
        let b_limbs = b.to_m31_array();
        let c = (a * b).to_m31_array();
        // Corrupt one product limb.
        table.push(
            a_limbs[0].0,
            a_limbs[1].0,
            a_limbs[2].0,
            a_limbs[3].0,
            b_limbs[0].0,
            b_limbs[1].0,
            b_limbs[2].0,
            b_limbs[3].0,
            c[0].0 + 1,
            c[1].0,
            c[2].0,
            c[3].0,
        );
        assert_table_satisfies_constraints(table);
    }

    #[test]
    fn test_qm31_mul_constraint_degrees_within_bound() {
        use stwo_constraint_framework::expr::ExprEvaluator;
        let eval = Eval { log_size: 4 };
        let expr_eval = eval.evaluate(ExprEvaluator::new());
        let degrees = expr_eval.constraint_degree_bounds();
        // 1 enabler booleanity + 4 limb constraints, all degree 2
        assert_eq!(degrees.len(), 5);
        assert!(degrees.iter().all(|&d| d <= 2));
    }
}
