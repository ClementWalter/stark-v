//! 2-to-1 aggregation tree over segment proofs (docs/recursion.md, M6).
//!
//! Segment proofs are paired and folded up a binary tree: each node asserts
//! that its two children verify and chain, and exposes the combined boundary
//! `(left.entry, right.exit)`. Today each node's child verification runs on
//! the host; the recursion verifier AIR replaces exactly that step with a
//! proof, leaving this tree structure unchanged — `Boundary` is the public
//! interface of an aggregate at every level, all the way to the root, whose
//! boundary spans the entire execution regardless of its length.

use stwo::core::pcs::PcsConfig;
use stwo::core::vcs_lifted::blake2_merkle::Blake2sMerkleHasher;

use crate::errors::VerificationError;
use crate::{Preprocessing, Proof, verify_rv32im};

/// Execution boundary exposed by a segment proof or an aggregate node.
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

/// Verify a sequence of segment proofs by folding them up a 2-to-1 binary
/// tree and return the root boundary spanning the whole execution.
///
/// Each level pairs adjacent nodes and chains their boundaries; an odd node
/// is carried up unchanged. Child proofs are verified on the host — the step
/// the recursion verifier AIR replaces, one tree node per recursion proof.
pub fn aggregate_segments(
    proofs: Vec<Proof<Blake2sMerkleHasher>>,
    config: PcsConfig,
    preprocessing: &Preprocessing,
) -> Result<Boundary, VerificationError> {
    assert!(!proofs.is_empty(), "cannot aggregate zero proofs");

    let mut level: Vec<Boundary> = proofs.iter().map(Boundary::of_segment).collect();

    // Host verification of every leaf (in-AIR verification replaces this).
    for proof in proofs {
        verify_rv32im(proof, config, preprocessing)?;
    }

    let mut depth = 0usize;
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for (index, pair) in level.chunks(2).enumerate() {
            match pair {
                [left, right] => {
                    let node = left.chain(right).map_err(|what| {
                        VerificationError::SegmentChainMismatch {
                            prev: (index * 2) << depth,
                            next: ((index * 2) + 1) << depth,
                            what,
                        }
                    })?;
                    next.push(node);
                }
                [odd] => next.push(odd.clone()),
                _ => unreachable!("chunks(2) yields 1 or 2 elements"),
            }
        }
        level = next;
        depth += 1;
    }

    Ok(level.pop().expect("non-empty level"))
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
