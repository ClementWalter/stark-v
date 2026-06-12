//! 2-to-1 aggregation with recursion-proof leaves (docs/recursion.md, M6).
//!
//! Each segment leaf carries a recursion proof of its composition check —
//! the inner verifier's component-coupled core, proven in-AIR from the
//! proof's transcript — alongside its boundary. Aggregation verifies the
//! constant-size recursion proof per leaf and folds boundaries pairwise to
//! the root, exactly as the host-only tree does.
//!
//! Trust split at this stage: the composition check (all inner constraints,
//! LogUp columns, alpha weighting, vanishing denominators) is attested by
//! the recursion proof; the inner proof's FRI/Merkle openings, LogUp-sum,
//! and PoW checks run host-side via `verify_rv32im` inside
//! `verify_segment_composition`. Each further in-AIR binding (draws, FRI
//! queries) moves work from the host remainder into the recursion proof
//! without changing this structure.

use prover::Proof;
use prover::{PcsConfig, Preprocessing};
use stwo::core::channel::MerkleChannel;
use stwo::core::fields::qm31::SecureField;
use stwo::core::vcs_lifted::blake2_merkle::{Blake2sMerkleChannel, Blake2sMerkleHasher};

use crate::binding::CompositionRecorder;
use crate::circuit::lower_arena;
use crate::prover::{RecursionProof, RecursionTraces, prove_recursion, verify_recursion};
use crate::recorder::Rec;
use crate::transcript::composition_binding_data;

/// Execution boundary exposed by a segment proof or an aggregate node.
///
/// `Boundary` is the public interface of an aggregate at every level, all
/// the way to the root, whose boundary spans the entire execution regardless
/// of its length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Boundary {
    pub entry_pc: u32,
    pub exit_pc: u32,
    pub entry_regs: [u32; 32],
    pub exit_regs: [u32; 32],
    pub entry_rw_root: Option<u32>,
    pub exit_rw_root: Option<u32>,
    pub program_root: Option<u32>,
}

impl Boundary {
    /// The boundary a segment proof exposes through its public data.
    pub fn of_segment<H: stwo::core::vcs_lifted::merkle_hasher::MerkleHasherLifted>(
        proof: &Proof<H>,
    ) -> Self {
        let public_data = &proof.public_data;
        Self {
            entry_pc: public_data.initial_pc,
            exit_pc: public_data.final_pc,
            entry_regs: public_data.initial_regs,
            exit_regs: public_data.final_regs,
            entry_rw_root: public_data.initial_rw_root,
            exit_rw_root: public_data.final_rw_root,
            program_root: public_data.program_root,
        }
    }

    /// Chain two boundaries: the left exit must equal the right entry, and
    /// both must run the same program.
    pub fn chain(&self, right: &Self) -> Result<Self, &'static str> {
        if self.exit_pc != right.entry_pc {
            return Err("exit_pc != entry_pc");
        }
        if self.exit_regs != right.entry_regs {
            return Err("exit_regs != entry_regs");
        }
        if self.exit_rw_root != right.entry_rw_root {
            return Err("exit_rw_root != entry_rw_root");
        }
        if self.program_root != right.program_root {
            return Err("program_root differs");
        }
        Ok(Self {
            entry_pc: self.entry_pc,
            exit_pc: right.exit_pc,
            entry_regs: self.entry_regs,
            exit_regs: right.exit_regs,
            entry_rw_root: self.entry_rw_root,
            exit_rw_root: right.exit_rw_root,
            program_root: self.program_root,
        })
    }
}

/// A segment leaf: its boundary and the recursion proof of its composition
/// check.
pub struct SegmentNode {
    pub boundary: Boundary,
    pub recursion_proof: RecursionProof<<Blake2sMerkleChannel as MerkleChannel>::H>,
}

/// Prove a segment's composition check in the recursion AIR.
pub fn prove_segment_composition(
    proof: &Proof<Blake2sMerkleHasher>,
    config: PcsConfig,
    preprocessing: &Preprocessing,
) -> SegmentNode {
    let data =
        composition_binding_data(proof, config, preprocessing).expect("transcript replay failed");
    let recorder = CompositionRecorder::new(&data).record(&data.components);
    assert_eq!(
        recorder.accumulation.value(),
        data.claimed_composition,
        "recorded composition must match the proof's OODS claim"
    );
    let output = match &recorder.accumulation {
        Rec::Node { id, .. } => *id,
        Rec::Const(_) => panic!("composition accumulated to a constant"),
    };

    let mut traces = RecursionTraces::default();
    let claim = lower_arena(
        &mut traces,
        0,
        &recorder.arena.borrow(),
        output,
        0,
        SecureField::default(),
    );
    let recursion_proof = prove_recursion(traces, vec![], vec![], vec![], vec![claim], config);

    SegmentNode {
        boundary: Boundary::of_segment(proof),
        recursion_proof,
    }
}

/// Verify a segment leaf: the full host verification of the inner proof
/// (commitments, FRI, LogUp sum, proof of work) plus the recursion proof of
/// its composition check against the canonical circuit re-recorded from the
/// public transcript.
pub fn verify_segment_composition(
    node: SegmentNode,
    proof: &Proof<Blake2sMerkleHasher>,
    config: PcsConfig,
    preprocessing: &Preprocessing,
) -> Result<(), prover::VerificationError> {
    // Host remainder: everything the in-AIR bindings have not yet absorbed.
    // Without this the sampled values are unanchored to the commitments.
    prover::verify_rv32im(proof.clone(), config, preprocessing)?;

    let data = composition_binding_data(proof, config, preprocessing)?;
    let recorder = CompositionRecorder::new(&data).record(&data.components);
    let output = match &recorder.accumulation {
        Rec::Node { id, .. } => *id,
        Rec::Const(_) => panic!("composition accumulated to a constant"),
    };
    // The canonical circuit must claim exactly the proof's composition value.
    if recorder.accumulation.value() != data.claimed_composition {
        return Err(prover::VerificationError::Stwo(
            stwo::core::verifier::VerificationError::OodsNotMatching,
        ));
    }
    verify_recursion(node.recursion_proof, &[(recorder.arena, output)], config)
        .map_err(prover::VerificationError::from)
}

/// Aggregate segment proofs with recursion-proof leaves: verify each leaf's
/// recursion proof and fold the boundaries pairwise up the 2-to-1 tree.
pub fn aggregate_with_recursion(
    segments: Vec<(Proof<Blake2sMerkleHasher>, SegmentNode)>,
    config: PcsConfig,
    preprocessing: &Preprocessing,
) -> Result<Boundary, prover::VerificationError> {
    assert!(!segments.is_empty(), "cannot aggregate zero segments");

    let mut level: Vec<Boundary> = Vec::with_capacity(segments.len());
    for (proof, node) in segments {
        level.push(node.boundary.clone());
        verify_segment_composition(node, &proof, config, preprocessing)?;
    }

    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            match pair {
                [left, right] => next.push(left.chain(right).map_err(|what| {
                    prover::VerificationError::SegmentChainMismatch {
                        prev: 0,
                        next: 1,
                        what,
                    }
                })?),
                [odd] => next.push(odd.clone()),
                _ => unreachable!("chunks(2)"),
            }
        }
        level = next;
    }
    Ok(level.pop().expect("non-empty"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boundary(entry_pc: u32, exit_pc: u32) -> Boundary {
        Boundary {
            entry_pc,
            exit_pc,
            entry_regs: [0; 32],
            exit_regs: [0; 32],
            entry_rw_root: Some(7),
            exit_rw_root: Some(7),
            program_root: Some(42),
        }
    }

    #[test]
    fn test_chain_combines_outer_boundary() {
        let combined = boundary(0, 4).chain(&boundary(4, 8)).expect("chains");
        assert_eq!((combined.entry_pc, combined.exit_pc), (0, 8));
    }

    #[test]
    fn test_chain_rejects_pc_gap() {
        assert!(boundary(0, 4).chain(&boundary(8, 12)).is_err());
    }

    #[test]
    fn test_chain_rejects_program_mismatch() {
        let mut right = boundary(4, 8);
        right.program_root = Some(43);
        assert!(boundary(0, 4).chain(&right).is_err());
    }
}
