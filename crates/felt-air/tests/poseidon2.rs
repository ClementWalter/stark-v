//! Poseidon2 written as a felt function: bit-for-bit equal to the runner's
//! permutation, and provable as an activation of its own AIR.

use felt_air::poseidon2::{Activation, Tables, call_permute, prove_air_fns, verify_air_fns};
use stwo::core::fields::m31::BaseField;
use stwo::core::pcs::PcsConfig;

/// The generated straight-line `evaluation()` holds the whole frame
/// (~1500 packed cells) in one stack frame; debug builds need more than
/// the test harness default.
fn on_big_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(work)
        .expect("spawn")
        .join()
        .expect("join")
}

fn run_both(state: [u32; 16]) -> ([u32; 16], [u32; 16]) {
    let mut tables = Tables::default();
    let outputs = call_permute(&mut tables, state.map(BaseField::from_u32_unchecked));
    let mut expected = state;
    runner::poseidon2::poseidon2_permutation(&mut expected);
    (outputs.map(|v| v.0), expected)
}

#[test]
fn test_permute_matches_runner_on_zero_state() {
    let (actual, expected) = run_both([0; 16]);
    assert_eq!(actual, expected);
}

#[test]
fn test_permute_matches_runner_on_counting_state() {
    let (actual, expected) = run_both(std::array::from_fn(|i| 1 + i as u32));
    assert_eq!(actual, expected);
}

#[test]
fn test_permute_matches_runner_on_random_states() {
    use rand::rngs::SmallRng;
    use rand::{Rng, SeedableRng};
    let mut rng = SmallRng::seed_from_u64(42);
    for _ in 0..16 {
        let state: [u32; 16] = std::array::from_fn(|_| rng.gen_range(0..(1u32 << 31) - 1));
        let (actual, expected) = run_both(state);
        assert_eq!(actual, expected);
    }
}

#[test]
fn test_permute_activation_proves_and_verifies() {
    on_big_stack(|| {
        let mut tables = Tables::default();
        let inputs: [BaseField; 16] =
            std::array::from_fn(|i| BaseField::from_u32_unchecked(7 * i as u32 + 3));
        let outputs = call_permute(&mut tables, inputs);

        let activations = vec![Activation::Permute { inputs, outputs }];
        let proof = prove_air_fns(tables, activations, PcsConfig::default());
        verify_air_fns(proof, PcsConfig::default()).expect("verification failed");
    });
}

#[test]
fn test_permute_rejects_forged_output() {
    on_big_stack(|| {
        let mut tables = Tables::default();
        let inputs: [BaseField; 16] =
            std::array::from_fn(|i| BaseField::from_u32_unchecked(11 * i as u32));
        let mut outputs = call_permute(&mut tables, inputs);
        outputs[0] += BaseField::from_u32_unchecked(1);

        let activations = vec![Activation::Permute { inputs, outputs }];
        let proof = prove_air_fns(tables, activations, PcsConfig::default());
        assert!(verify_air_fns(proof, PcsConfig::default()).is_err());
    });
}

/// The degree budget reproduces the hand layout: the zkVM's hand-flattened
/// poseidon2 table commits 445 columns (enabler + initial state + 3 cells
/// per s-box per full-round lane + 3 per partial round + flags); the
/// compiler reaches the same shape automatically — 2 cells per lane in the
/// first full round (its inputs are already columns) and 3 afterwards.
#[test]
fn test_compiled_layout_matches_hand_flattened_scale() {
    use felt_air::poseidon2::prover_columns::PermuteColumns;
    let size = PermuteColumns::<()>::SIZE;
    // enabler + 16 inputs + 16*(2 + 3*7) cells (full rounds) + 3*14 (partial).
    assert_eq!(size, 1 + 16 + 16 * (2 + 3 * 7) + 3 * 14);
}
