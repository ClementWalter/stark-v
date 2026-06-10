//! End-to-end: felt functions compiled to AIR, proven and verified with
//! stwo. The host runs the program (`call_*` fills every table down the
//! call tree), proves, and the verifier closes the activation multiset over
//! the public entry activations.

use felt_air::{Activation, Tables, call_cube, call_poly, prove_air_fns, verify_air_fns};
use stwo::core::fields::m31::BaseField;
use stwo::core::pcs::PcsConfig;

fn felt(value: u32) -> BaseField {
    BaseField::from_u32_unchecked(value)
}

#[test]
fn test_air_fns_roundtrip() {
    let mut tables = Tables::default();
    let inputs = [felt(3), felt(5)];
    let outputs = call_poly(&mut tables, inputs);

    // cube(3) = 27, quint(5) = 3125, affine(3, 5, 27) = 3 + 10 + 81 + 7 = 101.
    assert_eq!(outputs[0], felt(27 + 3125));
    assert_eq!(outputs[1], felt(101 * 101));
    // The call tree filled every callee table.
    assert_eq!(tables.poly.len(), 1);
    assert_eq!(tables.cube.len(), 1);
    assert_eq!(tables.quint.len(), 1);
    assert_eq!(tables.affine.len(), 1);

    let activations = vec![Activation::Poly { inputs, outputs }];
    let proof = prove_air_fns(tables, activations, PcsConfig::default());
    verify_air_fns(proof, PcsConfig::default()).expect("verification failed");
}

#[test]
fn test_air_fns_repeated_activations_balance_by_multiplicity() {
    let mut tables = Tables::default();
    let first = call_cube(&mut tables, [felt(2)]);
    let second = call_cube(&mut tables, [felt(2)]);
    assert_eq!(first, second);

    let activations = vec![
        Activation::Cube {
            inputs: [felt(2)],
            outputs: first,
        },
        Activation::Cube {
            inputs: [felt(2)],
            outputs: second,
        },
    ];
    let proof = prove_air_fns(tables, activations, PcsConfig::default());
    verify_air_fns(proof, PcsConfig::default()).expect("verification failed");
}

#[test]
fn test_air_fns_rejects_forged_output() {
    let mut tables = Tables::default();
    let inputs = [felt(3), felt(5)];
    let outputs = call_poly(&mut tables, inputs);

    let mut forged = outputs;
    forged[0] += felt(1);
    let activations = vec![Activation::Poly {
        inputs,
        outputs: forged,
    }];
    let proof = prove_air_fns(tables, activations, PcsConfig::default());
    assert!(verify_air_fns(proof, PcsConfig::default()).is_err());
}

#[test]
fn test_air_fns_rejects_missing_internal_activation() {
    // Activate cube directly but claim a poly activation: the multiset
    // cannot close.
    let mut tables = Tables::default();
    let outputs = call_cube(&mut tables, [felt(3)]);
    let activations = vec![Activation::Poly {
        inputs: [felt(3), felt(5)],
        outputs: [outputs[0], felt(0)],
    }];
    let proof = prove_air_fns(tables, activations, PcsConfig::default());
    assert!(verify_air_fns(proof, PcsConfig::default()).is_err());
}

#[test]
fn test_degree_budget_materializes_quint_chain() {
    use felt_air::prover_columns::{CubeColumns, QuintColumns};

    // cube: enabler + x + one materialized square (x2 * x at budget 2
    // forces x2 into a column).
    assert_eq!(CubeColumns::<()>::SIZE, 3);
    // quint: enabler + x + three materialized cells (x2, x2 again, x4) —
    // the x2 * x2 * x chain unrolled to keep every product within budget.
    assert_eq!(QuintColumns::<()>::SIZE, 5);
}

#[test]
fn test_constraint_degrees_within_bound() {
    use stwo_constraint_framework::FrameworkEval;
    use stwo_constraint_framework::expr::ExprEvaluator;

    let relations = felt_air::AirFnRelations::dummy();
    for (name, degrees) in [
        (
            "cube",
            felt_air::cube::air::Eval {
                log_size: 4,
                relations: relations.clone(),
            }
            .evaluate(ExprEvaluator::new())
            .constraint_degree_bounds(),
        ),
        (
            "quint",
            felt_air::quint::air::Eval {
                log_size: 4,
                relations: relations.clone(),
            }
            .evaluate(ExprEvaluator::new())
            .constraint_degree_bounds(),
        ),
        (
            "poly",
            felt_air::poly::air::Eval {
                log_size: 4,
                relations: relations.clone(),
            }
            .evaluate(ExprEvaluator::new())
            .constraint_degree_bounds(),
        ),
    ] {
        let max = degrees.iter().max().copied().unwrap_or(0);
        assert!(max <= 3, "{name} breaches the degree bound: {degrees:?}");
    }
}
