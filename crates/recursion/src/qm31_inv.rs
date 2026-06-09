//! QM31 inverse component: witness generation and AIR evaluation.
//!
//! The constraints live in the `define_component_tables!` invocation in
//! `lib.rs`: `a * inv = enabler` limb-by-limb, so padding rows hold and
//! enabled rows force `a` to be invertible. Witness generation computes the
//! inverse with stwo's field arithmetic, which the tests use as the oracle.

use stwo::core::fields::FieldExpOps;
use stwo::core::fields::qm31::QM31;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use crate::Qm31InvTable;
use crate::prover_columns::Qm31InvColumns;

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
        let cols = Qm31InvColumns::from_eval(&mut eval);
        for constraint in cols.constraints() {
            eval.add_constraint(constraint);
        }
        eval
    }
}

/// Record `a^-1` in the trace table and return it.
///
/// Panics if `a` is zero (zero has no inverse).
pub fn push_inv(table: &mut Qm31InvTable, a: QM31) -> QM31 {
    let inv = a.inverse();
    let a = a.to_m31_array();
    let limbs = inv.to_m31_array();
    table.push(
        a[0].0, a[1].0, a[2].0, a[3].0, limbs[0].0, limbs[1].0, limbs[2].0, limbs[3].0,
    );
    inv
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::{One, Zero};
    use rand::rngs::SmallRng;
    use rand::{Rng, SeedableRng};
    use stwo::core::fields::qm31::SecureField;
    use stwo::core::pcs::TreeVec;
    use stwo::core::poly::circle::CanonicCoset;
    use stwo_constraint_framework::assert_constraints_on_polys;

    fn random_nonzero_qm31(rng: &mut SmallRng) -> QM31 {
        loop {
            let value = QM31::from_u32_unchecked(
                rng.gen_range(0..(1 << 30)),
                rng.gen_range(0..(1 << 30)),
                rng.gen_range(0..(1 << 30)),
                rng.gen_range(0..(1 << 30)),
            );
            if !value.is_zero() {
                return value;
            }
        }
    }

    fn assert_table_satisfies_constraints(table: Qm31InvTable) {
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
    fn test_qm31_inv_constraints_hold_on_random_inverses() {
        let mut rng = SmallRng::seed_from_u64(0);
        let mut table = Qm31InvTable::new();
        for _ in 0..100 {
            let a = random_nonzero_qm31(&mut rng);
            let inv = push_inv(&mut table, a);
            assert_eq!(a * inv, QM31::one());
        }
        assert_table_satisfies_constraints(table);
    }

    #[test]
    #[should_panic]
    fn test_qm31_inv_constraints_reject_wrong_inverse() {
        let mut rng = SmallRng::seed_from_u64(1);
        let mut table = Qm31InvTable::new();
        let a = random_nonzero_qm31(&mut rng);
        let a_limbs = a.to_m31_array();
        let inv = a.inverse().to_m31_array();
        // Corrupt one inverse limb.
        table.push(
            a_limbs[0].0,
            a_limbs[1].0,
            a_limbs[2].0,
            a_limbs[3].0,
            inv[0].0 + 1,
            inv[1].0,
            inv[2].0,
            inv[3].0,
        );
        assert_table_satisfies_constraints(table);
    }

    #[test]
    fn test_qm31_inv_constraint_degrees_within_bound() {
        use stwo_constraint_framework::expr::ExprEvaluator;
        let eval = Eval { log_size: 4 };
        let expr_eval = eval.evaluate(ExprEvaluator::new());
        let degrees = expr_eval.constraint_degree_bounds();
        // 1 enabler booleanity + 4 limb constraints, all degree 2
        assert_eq!(degrees.len(), 5);
        assert!(degrees.iter().all(|&d| d <= 2));
    }
}
