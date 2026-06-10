//! Recursion-local LogUp relations.
//!
//! `merkle_node` carries node claims `(tree_id, depth, index, digest words)`
//! along decommitment paths: rows of the `merkle_path` component consume
//! their own claim and emit the on-path child's, and roots are anchored by
//! public claim terms (see `prover::verify_recursion`), so a path balances
//! to exactly one public root emission.

use stwo::core::channel::Channel;
use stwo_constraint_framework::relation;

relation!(MerkleNodeRelation, 11);

#[derive(Clone)]
pub struct RecursionRelations {
    pub merkle_node: MerkleNodeRelation,
}

impl RecursionRelations {
    pub fn draw(channel: &mut impl Channel) -> Self {
        Self {
            merkle_node: MerkleNodeRelation::draw(channel),
        }
    }
}
