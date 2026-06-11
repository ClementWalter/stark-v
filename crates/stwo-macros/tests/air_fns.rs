//! End-to-end coverage for `define_air_fns!`: standalone AIR generation,
//! witness filling, relation cancellation, and Poseidon2 equivalence.

#![allow(clippy::possible_missing_comma, clippy::too_many_arguments)]

use stwo::core::fields::m31::BaseField;
use stwo::core::pcs::PcsConfig;

mod examples {
    stwo_macros::define_air_fns! {
        max_degree: 3,

        fn cube(x) {
            let x2 = x * x;
            return x2 * x;
        }

        fn quint(x) {
            return x ** 5;
        }

        fn affine(a, b, c) {
            return a + 2 * b + 3 * c + 7;
        }

        fn poly(a, b) {
            let c = cube(a);
            let q = quint(b);
            let s = affine(a, b, c);
            assert (a + b) * (a + b) == a * a + 2 * a * b + b * b;
            return (c + q, s * s);
        }
    }
}

mod poseidon {
    stwo_macros::define_air_fns! {
        max_degree: 3,

        inline fn m4(x0, x1, x2, x3) {
            let t0 = x0 + x1;
            let t1 = x2 + x3;
            let t2 = 2 * x1 + t1;
            let t3 = 2 * x3 + t0;
            let t4 = 4 * t1 + t3;
            let t5 = 4 * t0 + t2;
            let t6 = t3 + t5;
            let t7 = t2 + t4;
            return (t6, t5, t7, t4);
        }

        inline fn external_matrix(state: [felt; 16]) {
            let (b0, b1, b2, b3) = m4(state[0], state[1], state[2], state[3]);
            let (b4, b5, b6, b7) = m4(state[4], state[5], state[6], state[7]);
            let (b8, b9, b10, b11) = m4(state[8], state[9], state[10], state[11]);
            let (b12, b13, b14, b15) = m4(state[12], state[13], state[14], state[15]);
            let mixed = [b0, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11, b12, b13, b14, b15];
            let sums = map(j, 0..4, mixed[j] + mixed[j + 4] + mixed[j + 8] + mixed[j + 12]);
            let out = map(k, 0..16, mixed[k] + sums[k % 4]);
            return out;
        }

        fn permute(state: [felt; 16]) {
            let state = external_matrix(state);
            for r in 0..4 {
                let state = map(j, 0..16, (state[j] + constant(runner::poseidon2::EXTERNAL_ROUND_CONSTS[r][j])) ** 5);
                let state = external_matrix(state);
            }
            for r in 0..14 {
                let state = update(state, 0, (state[0] + constant(runner::poseidon2::INTERNAL_ROUND_CONSTS[r])) ** 5);
                let total = sum(j, 0..16, state[j]);
                let state = map(j, 0..16, state[j] * constant(runner::poseidon2::INTERNAL_MATRIX[j]) + total);
            }
            for r in 4..8 {
                let state = map(j, 0..16, (state[j] + constant(runner::poseidon2::EXTERNAL_ROUND_CONSTS[r][j])) ** 5);
                let state = external_matrix(state);
            }
            return state;
        }
    }
}

fn felt(value: u32) -> BaseField {
    BaseField::from_u32_unchecked(value)
}

#[test]
fn test_air_fns_roundtrip() {
    let mut tables = examples::Tables::default();
    let inputs = [felt(3), felt(5)];
    let outputs = examples::call_poly(&mut tables, inputs);

    // cube(3) = 27, quint(5) = 3125, affine(3, 5, 27) = 3 + 10 + 81 + 7 = 101.
    assert_eq!(outputs[0], felt(27 + 3125));
    assert_eq!(outputs[1], felt(101 * 101));
    assert_eq!(tables.poly.len(), 1);
    assert_eq!(tables.cube.len(), 1);
    assert_eq!(tables.quint.len(), 1);
    assert_eq!(tables.affine.len(), 1);

    let activations = vec![examples::Activation::Poly { inputs, outputs }];
    let proof = examples::prove_air_fns(tables, activations, PcsConfig::default());
    examples::verify_air_fns(proof, PcsConfig::default()).expect("verification failed");
}

#[test]
fn test_air_fns_repeated_activations_balance_by_multiplicity() {
    let mut tables = examples::Tables::default();
    let first = examples::call_cube(&mut tables, [felt(2)]);
    let second = examples::call_cube(&mut tables, [felt(2)]);
    assert_eq!(first, second);

    let activations = vec![
        examples::Activation::Cube {
            inputs: [felt(2)],
            outputs: first,
        },
        examples::Activation::Cube {
            inputs: [felt(2)],
            outputs: second,
        },
    ];
    let proof = examples::prove_air_fns(tables, activations, PcsConfig::default());
    examples::verify_air_fns(proof, PcsConfig::default()).expect("verification failed");
}

#[test]
fn test_air_fns_rejects_forged_output() {
    let mut tables = examples::Tables::default();
    let inputs = [felt(3), felt(5)];
    let outputs = examples::call_poly(&mut tables, inputs);

    let mut forged = outputs;
    forged[0] += felt(1);
    let activations = vec![examples::Activation::Poly {
        inputs,
        outputs: forged,
    }];
    let proof = examples::prove_air_fns(tables, activations, PcsConfig::default());
    assert!(examples::verify_air_fns(proof, PcsConfig::default()).is_err());
}

#[test]
fn test_air_fns_rejects_missing_internal_activation() {
    let mut tables = examples::Tables::default();
    let outputs = examples::call_cube(&mut tables, [felt(3)]);
    let activations = vec![examples::Activation::Poly {
        inputs: [felt(3), felt(5)],
        outputs: [outputs[0], felt(0)],
    }];
    let proof = examples::prove_air_fns(tables, activations, PcsConfig::default());
    assert!(examples::verify_air_fns(proof, PcsConfig::default()).is_err());
}

#[test]
fn test_degree_budget_materializes_quint_chain() {
    use examples::prover_columns::{CubeColumns, QuintColumns};

    // cube: enabler + x + one materialized square (x2 * x at budget 2
    // forces x2 into a column).
    assert_eq!(CubeColumns::<()>::SIZE, 3);
    // quint: enabler + x + two materialized cells from exponentiation by
    // squaring under the degree budget.
    assert_eq!(QuintColumns::<()>::SIZE, 4);
}

#[test]
fn test_constraint_degrees_within_bound() {
    use stwo_constraint_framework::FrameworkEval;
    use stwo_constraint_framework::expr::ExprEvaluator;

    let relations = examples::AirFnRelations::dummy();
    for (name, degrees) in [
        (
            "cube",
            examples::cube::air::Eval {
                log_size: 4,
                relations: relations.clone(),
            }
            .evaluate(ExprEvaluator::new())
            .constraint_degree_bounds(),
        ),
        (
            "quint",
            examples::quint::air::Eval {
                log_size: 4,
                relations: relations.clone(),
            }
            .evaluate(ExprEvaluator::new())
            .constraint_degree_bounds(),
        ),
        (
            "poly",
            examples::poly::air::Eval {
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

fn run_poseidon_both(state: [u32; 16]) -> ([u32; 16], [u32; 16]) {
    let mut tables = poseidon::Tables::default();
    let outputs = poseidon::call_permute(&mut tables, state.map(BaseField::from_u32_unchecked));
    let mut expected = state;
    runner::poseidon2::poseidon2_permutation(&mut expected);
    (outputs.map(|v| v.0), expected)
}

#[test]
fn test_permute_matches_runner_on_zero_state() {
    let (actual, expected) = run_poseidon_both([0; 16]);
    assert_eq!(actual, expected);
}

#[test]
fn test_permute_matches_runner_on_counting_state() {
    let (actual, expected) = run_poseidon_both(std::array::from_fn(|i| 1 + i as u32));
    assert_eq!(actual, expected);
}

#[test]
fn test_permute_matches_runner_on_random_states() {
    use rand::rngs::SmallRng;
    use rand::{Rng, SeedableRng};
    let mut rng = SmallRng::seed_from_u64(42);
    for _ in 0..16 {
        let state: [u32; 16] = std::array::from_fn(|_| rng.gen_range(0..(1u32 << 31) - 1));
        let (actual, expected) = run_poseidon_both(state);
        assert_eq!(actual, expected);
    }
}

#[test]
fn test_permute_activation_proves_and_verifies() {
    let mut tables = poseidon::Tables::default();
    let inputs: [BaseField; 16] =
        std::array::from_fn(|i| BaseField::from_u32_unchecked(7 * i as u32 + 3));
    let outputs = poseidon::call_permute(&mut tables, inputs);

    let activations = vec![poseidon::Activation::Permute { inputs, outputs }];
    let proof = poseidon::prove_air_fns(tables, activations, PcsConfig::default());
    poseidon::verify_air_fns(proof, PcsConfig::default()).expect("verification failed");
}

#[test]
fn test_permute_rejects_forged_output() {
    let mut tables = poseidon::Tables::default();
    let inputs: [BaseField; 16] =
        std::array::from_fn(|i| BaseField::from_u32_unchecked(11 * i as u32));
    let mut outputs = poseidon::call_permute(&mut tables, inputs);
    outputs[0] += BaseField::from_u32_unchecked(1);

    let activations = vec![poseidon::Activation::Permute { inputs, outputs }];
    let proof = poseidon::prove_air_fns(tables, activations, PcsConfig::default());
    assert!(poseidon::verify_air_fns(proof, PcsConfig::default()).is_err());
}

#[test]
fn test_compiled_poseidon_layout_matches_hand_flattened_scale() {
    use poseidon::prover_columns::PermuteColumns;

    let size = PermuteColumns::<()>::SIZE;
    // enabler + 16 inputs + 16*(2 + 3*7) cells (full rounds) + 3*14 (partial).
    assert_eq!(size, 1 + 16 + 16 * (2 + 3 * 7) + 3 * 14);
}
