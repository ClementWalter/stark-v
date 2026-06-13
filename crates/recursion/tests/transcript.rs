//! Transcript replay against a real proof.

use prover::PcsConfig;
use prover::e2e::{ensure_guest_built, guest_bin_dir};
use prover::prove_rv32im;
use recursion::transcript::replay_composition_oods;
use runner::run;

/// Recursion seam check (docs/recursion.md, M1): replaying the Fiat-Shamir
/// transcript outside the verifier and recomputing the composition value at
/// the OODS point through the components' `evaluate()` must reproduce the
/// value claimed by the proof's sampled composition polynomials.
#[test]
fn test_recursion_composition_oods_replay() {
    ensure_guest_built();

    let elf_path = guest_bin_dir().join("mulhu_alias");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read mulhu_alias ELF");
    let run_result = run(&elf_bytes, 10_000_000).expect("Failed to run mulhu_alias");

    let preprocessing = prover::preprocess(PcsConfig::default());
    let proof = prove_rv32im(run_result, PcsConfig::default(), &preprocessing);

    let check = replay_composition_oods(&proof, PcsConfig::default(), &preprocessing)
        .expect("transcript replay failed");
    assert!(
        check.holds(),
        "composition OODS mismatch: claimed {:?} != replayed {:?}",
        check.claimed,
        check.replayed
    );
}
