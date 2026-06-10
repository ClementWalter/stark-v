use runner::trace::prover_columns::DivColumns;
use stwo::core::fields::m31::BaseField;
use stwo_constraint_framework::FrameworkEval;
use stwo_constraint_framework::expr::{BaseExpr, ExprEvaluator};

#[test]
fn probe_div_degree_map() {
    // How many algebraic constraints does the table contribute?
    let zero = DivColumns::<BaseExpr> {
        ..DivColumns::from_iter(
            (0..DivColumns::<()>::SIZE).map(|i| BaseExpr::Col((1usize, i, 0isize).into())),
        )
    };
    let constraints = zero.constraints();
    eprintln!("algebraic constraints: {}", constraints.len());
    for (i, c) in constraints.iter().enumerate() {
        let d = c.degree_bound();
        if d > 3 {
            eprintln!("  algebraic {i}: degree {d}");
        }
    }
    let entries = zero.lookup_entries();
    eprintln!("lookup entries: {}", entries.len());
    for (i, (m, values)) in entries.iter().enumerate() {
        let md = m.degree_bound();
        let vd = values.iter().map(|v| v.degree_bound()).max().unwrap_or(0);
        if md.max(vd) > 2 {
            eprintln!("  entry {i}: mult deg {md}, max elem deg {vd}");
        }
    }
    let eval = prover::components::div::air::Eval {
        log_size: 6,
        relations: prover::relations::Relations::dummy(),
    };
    let expr_eval = eval.evaluate(ExprEvaluator::new());
    eprintln!(
        "total constraints: {}",
        expr_eval.constraint_degree_bounds().len()
    );
}
