//! FRI line-fold component: witness generation and AIR evaluation.
//!
//! The constraints live in the `define_component_tables!` invocation in
//! `lib.rs`: one fold step `folded = (f(x) + f(-x)) + alpha * (f(x) - f(-x)) * x^-1`
//! per row — stwo's `ibutterfly` followed by the alpha combination, which is
//! the per-query work of every FRI layer. The tests use stwo's `ibutterfly`
//! as the oracle.

use stwo::core::fields::m31::BaseField;
use stwo::core::fields::qm31::QM31;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use crate::FriFoldLineTable;
use crate::prover_columns::FriFoldLineColumns;

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
        let cols = FriFoldLineColumns::from_eval(&mut eval);
        for constraint in cols.constraints() {
            eval.add_constraint(constraint);
        }
        eval
    }
}

/// Record one line-fold step in the trace table and return the folded value.
///
/// Panics if `x` is zero (domain points are never zero).
pub fn push_fold_line(
    table: &mut FriFoldLineTable,
    f_x: QM31,
    f_neg_x: QM31,
    x: BaseField,
    alpha: QM31,
) -> QM31 {
    let x_inv = x.inverse();
    let t = (f_x - f_neg_x) * x_inv;
    let folded = f_x + f_neg_x + alpha * t;

    let f_x = f_x.to_m31_array();
    let f_neg_x = f_neg_x.to_m31_array();
    let t_limbs = t.to_m31_array();
    let alpha = alpha.to_m31_array();
    let folded_limbs = folded.to_m31_array();
    table.push(
        x.0,
        x_inv.0,
        f_x[0].0,
        f_x[1].0,
        f_x[2].0,
        f_x[3].0,
        f_neg_x[0].0,
        f_neg_x[1].0,
        f_neg_x[2].0,
        f_neg_x[3].0,
        t_limbs[0].0,
        t_limbs[1].0,
        t_limbs[2].0,
        t_limbs[3].0,
        alpha[0].0,
        alpha[1].0,
        alpha[2].0,
        alpha[3].0,
        folded_limbs[0].0,
        folded_limbs[1].0,
        folded_limbs[2].0,
        folded_limbs[3].0,
    );
    folded
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::Zero;
    use rand::rngs::SmallRng;
    use rand::{Rng, SeedableRng};
    use stwo::core::fft::ibutterfly;
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

    fn assert_table_satisfies_constraints(table: FriFoldLineTable) {
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
    fn test_fri_fold_line_matches_ibutterfly_oracle() {
        let mut rng = SmallRng::seed_from_u64(0);
        let f_x = random_qm31(&mut rng);
        let f_neg_x = random_qm31(&mut rng);
        let x = BaseField::from_u32_unchecked(rng.gen_range(1..(1 << 30)));
        let alpha = random_qm31(&mut rng);

        let mut table = FriFoldLineTable::new();
        let folded = push_fold_line(&mut table, f_x, f_neg_x, x, alpha);

        // stwo's fold primitive: ibutterfly then combine with alpha.
        let (mut f0, mut f1) = (f_x, f_neg_x);
        ibutterfly(&mut f0, &mut f1, x.inverse());
        assert_eq!(folded, f0 + alpha * f1);
    }

    #[test]
    fn test_fri_fold_line_constraints_hold_on_random_folds() {
        let mut rng = SmallRng::seed_from_u64(1);
        let mut table = FriFoldLineTable::new();
        for _ in 0..100 {
            let f_x = random_qm31(&mut rng);
            let f_neg_x = random_qm31(&mut rng);
            let x = BaseField::from_u32_unchecked(rng.gen_range(1..(1 << 30)));
            let alpha = random_qm31(&mut rng);
            push_fold_line(&mut table, f_x, f_neg_x, x, alpha);
        }
        assert_table_satisfies_constraints(table);
    }

    #[test]
    #[should_panic]
    fn test_fri_fold_line_constraints_reject_wrong_fold() {
        let mut rng = SmallRng::seed_from_u64(2);
        let mut table = FriFoldLineTable::new();
        let f_x = random_qm31(&mut rng);
        let f_neg_x = random_qm31(&mut rng);
        let x = BaseField::from_u32_unchecked(rng.gen_range(1..(1 << 30)));
        let alpha = random_qm31(&mut rng);
        push_fold_line(&mut table, f_x, f_neg_x, x, alpha);
        // Corrupt the folded value of the recorded row.
        let last = table.folded_0.len() - 1;
        table.folded_0[last] += 1;
        assert_table_satisfies_constraints(table);
    }

    #[test]
    fn test_fri_fold_line_constraint_degrees_within_bound() {
        use stwo_constraint_framework::expr::ExprEvaluator;
        let eval = Eval { log_size: 4 };
        let expr_eval = eval.evaluate(ExprEvaluator::new());
        let degrees = expr_eval.constraint_degree_bounds();
        // enabler booleanity + x*x_inv + 4 odd-part + 4 fold constraints
        assert_eq!(degrees.len(), 10);
        assert!(degrees.iter().all(|&d| d <= 2));
    }
}
