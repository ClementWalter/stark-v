//! Circle point doubling component: witness generation and AIR evaluation.
//!
//! The constraints live in the `define_component_tables!` invocation in
//! `lib.rs`: `r = 2p` on the unit circle, i.e. `r_x = 2 p_x^2 - 1` and
//! `r_y = 2 p_x p_y` over QM31 coordinates. Doubling is the workhorse of the
//! verifier's point arithmetic (`repeated_double` in the composition
//! extraction and vanishing-polynomial evaluation). stwo's `CirclePoint`
//! arithmetic is the oracle the tests check against.

use stwo::core::circle::CirclePoint;
use stwo::core::fields::qm31::SecureField;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use crate::CircleDoubleTable;
use crate::prover_columns::CircleDoubleColumns;

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
        let cols = CircleDoubleColumns::from_eval(&mut eval);
        for constraint in cols.constraints() {
            eval.add_constraint(constraint);
        }
        eval
    }
}

/// Record `2p` in the trace table and return it.
pub fn push_double(
    table: &mut CircleDoubleTable,
    p: CirclePoint<SecureField>,
) -> CirclePoint<SecureField> {
    let r = p.double();
    let p_x = p.x.to_m31_array();
    let p_y = p.y.to_m31_array();
    let r_x = r.x.to_m31_array();
    let r_y = r.y.to_m31_array();
    table.push(
        p_x[0].0, p_x[1].0, p_x[2].0, p_x[3].0, p_y[0].0, p_y[1].0, p_y[2].0, p_y[3].0, r_x[0].0,
        r_x[1].0, r_x[2].0, r_x[3].0, r_y[0].0, r_y[1].0, r_y[2].0, r_y[3].0,
    );
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::Zero;
    use rand::rngs::SmallRng;
    use rand::{Rng, SeedableRng};
    use stwo::core::circle::SECURE_FIELD_CIRCLE_GEN;
    use stwo::core::pcs::TreeVec;
    use stwo::core::poly::circle::CanonicCoset;
    use stwo_constraint_framework::assert_constraints_on_polys;

    fn random_point(rng: &mut SmallRng) -> CirclePoint<SecureField> {
        SECURE_FIELD_CIRCLE_GEN.mul(rng.r#gen::<u128>())
    }

    fn assert_table_satisfies_constraints(table: CircleDoubleTable) {
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
    fn test_circle_double_matches_point_arithmetic_oracle() {
        let mut rng = SmallRng::seed_from_u64(0);
        let p = random_point(&mut rng);
        let mut table = CircleDoubleTable::new();
        let r = push_double(&mut table, p);
        assert_eq!(r, p + p);
    }

    #[test]
    fn test_circle_double_constraints_hold_on_random_points() {
        let mut rng = SmallRng::seed_from_u64(1);
        let mut table = CircleDoubleTable::new();
        for _ in 0..100 {
            push_double(&mut table, random_point(&mut rng));
        }
        assert_table_satisfies_constraints(table);
    }

    #[test]
    #[should_panic]
    fn test_circle_double_constraints_reject_wrong_double() {
        let mut rng = SmallRng::seed_from_u64(2);
        let mut table = CircleDoubleTable::new();
        push_double(&mut table, random_point(&mut rng));
        // Corrupt one limb of the doubled point.
        let last = table.r_x_0.len() - 1;
        table.r_x_0[last] += 1;
        assert_table_satisfies_constraints(table);
    }

    #[test]
    fn test_circle_double_constraint_degrees_within_bound() {
        use stwo_constraint_framework::expr::ExprEvaluator;
        let eval = Eval { log_size: 4 };
        let expr_eval = eval.evaluate(ExprEvaluator::new());
        let degrees = expr_eval.constraint_degree_bounds();
        // enabler booleanity + 4 r_x + 4 r_y limb constraints
        assert_eq!(degrees.len(), 9);
        assert!(degrees.iter().all(|&d| d <= 2));
    }
}
