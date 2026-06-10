//! Real-proof composition binding (docs/recursion.md, M5): the recorded
//! circuit over an actual stark-v proof's sampled values must reproduce the
//! composition value the proof claims at the OODS point.

use prover::e2e::{ensure_guest_built, guest_bin_dir};
use prover::recursion::transcript::composition_binding_data;
use prover::{PcsConfig, prove_rv32im};
use recursion::binding::CompositionRecorder;
use runner::run;

#[test]
fn test_recorded_composition_matches_real_proof() {
    ensure_guest_built();

    let elf_path = guest_bin_dir().join("mulhu_alias");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read mulhu_alias ELF");
    let run_result = run(&elf_bytes, 10_000_000).expect("Failed to run mulhu_alias");

    let preprocessing = prover::preprocess(PcsConfig::default());
    let proof = prove_rv32im(run_result, PcsConfig::default(), &preprocessing);

    let data = composition_binding_data(&proof, PcsConfig::default(), &preprocessing)
        .expect("transcript replay failed");

    // Record every inner component's point evaluation into one arena —
    // through the same evaluate() the prover and host verifier run.
    let recorder = CompositionRecorder::new(&data).record(&data.components);

    assert_eq!(
        recorder.accumulation.value(),
        data.claimed_composition,
        "recorded composition circuit must reproduce the proof's OODS claim"
    );
    // The arena is a real circuit, ready for lowering.
    assert!(recorder.arena.borrow().nodes.len() > 1000);
}
