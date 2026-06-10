//! Segmented proving and verification (docs/recursion.md, M2).
//!
//! A long execution is split by `runner::run_segments_with_input` into
//! segments of bounded cycle count, each proven independently with the
//! per-segment clock restarting at 0. Consecutive segments chain on their
//! public data: the exit state of segment `k` — program counter, register
//! file, and read-write memory Merkle root — must equal the entry state of
//! segment `k + 1`, while the program root is common to all segments.
//!
//! These chain checks plus the per-segment proofs are exactly what the
//! 2-to-1 aggregation AIR asserts for its two children; here they run on the
//! host so segmentation is sound before recursion exists.

use stwo::core::channel::MerkleChannel;
use stwo::core::pcs::PcsConfig;
use stwo::core::vcs_lifted::blake2_merkle::{Blake2sMerkleChannel, Blake2sMerkleHasher};
use stwo::core::vcs_lifted::merkle_hasher::MerkleHasherLifted;
use stwo::prover::backend::simd::SimdBackend;

use crate::errors::VerificationError;
use crate::{Preprocessing, Proof, prove_rv32im_with_channel, verify_rv32im_with_channel};

/// Prove every segment of a segmented execution (Blake2s channel).
pub fn prove_segments(
    run_results: Vec<runner::RunResult>,
    config: PcsConfig,
    preprocessing: &Preprocessing,
) -> Vec<Proof<Blake2sMerkleHasher>> {
    prove_segments_with_channel::<Blake2sMerkleChannel>(run_results, config, preprocessing)
}

/// Prove every segment of a segmented execution with any Merkle channel —
/// in particular the Poseidon2-M31 channel whose hash the recursion AIR
/// proves.
pub fn prove_segments_with_channel<MC: MerkleChannel>(
    run_results: Vec<runner::RunResult>,
    config: PcsConfig,
    preprocessing: &Preprocessing<MC::H>,
) -> Vec<Proof<MC::H>>
where
    SimdBackend: stwo::prover::backend::BackendForChannel<MC>
        + stwo::prover::backend::ColumnOps<
            <MC::H as MerkleHasherLifted>::Hash,
            Column = Vec<<MC::H as MerkleHasherLifted>::Hash>,
        >,
{
    run_results
        .into_iter()
        .map(|run_result| prove_rv32im_with_channel::<MC>(run_result, config, preprocessing))
        .collect()
}

/// Verify a chain of segment proofs (Blake2s channel).
pub fn verify_segments(
    proofs: Vec<Proof<Blake2sMerkleHasher>>,
    config: PcsConfig,
    preprocessing: &Preprocessing,
) -> Result<(), VerificationError> {
    verify_segments_with_channel::<Blake2sMerkleChannel>(proofs, config, preprocessing)
}

/// Verify a chain of segment proofs with any Merkle channel: each proof
/// individually, plus the boundary chaining between consecutive segments.
pub fn verify_segments_with_channel<MC: MerkleChannel>(
    proofs: Vec<Proof<MC::H>>,
    config: PcsConfig,
    preprocessing: &Preprocessing<MC::H>,
) -> Result<(), VerificationError> {
    for (index, pair) in proofs.windows(2).enumerate() {
        let (prev, next) = (&pair[0].public_data, &pair[1].public_data);
        let mismatch = |what| VerificationError::SegmentChainMismatch {
            prev: index,
            next: index + 1,
            what,
        };
        if prev.final_pc != next.initial_pc {
            return Err(mismatch("final_pc != initial_pc"));
        }
        if prev.final_regs != next.initial_regs {
            return Err(mismatch("final_regs != initial_regs"));
        }
        if prev.final_rw_root != next.initial_rw_root {
            return Err(mismatch("final_rw_root != initial_rw_root"));
        }
        if prev.program_root != next.program_root {
            return Err(mismatch("program_root differs"));
        }
    }

    for proof in proofs {
        verify_rv32im_with_channel::<MC>(proof, config, preprocessing)?;
    }
    Ok(())
}
