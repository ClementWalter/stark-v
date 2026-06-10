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
// Sponge chaining: (channel_id, step, state words). Rows of the
// channel_replay component consume their previous state claim and emit the
// next; the chain ends are anchored by public claim terms.
relation!(SpongeStepRelation, 18);
// Absorbed data: (channel_id, step, chunk words), emitted publicly from the
// proof claim and consumed by the replay row of that step.
relation!(SpongeDataRelation, 10);

#[derive(Clone)]
pub struct RecursionRelations {
    pub merkle_node: MerkleNodeRelation,
    pub sponge_step: SpongeStepRelation,
    pub sponge_data: SpongeDataRelation,
}

impl RecursionRelations {
    pub fn draw(channel: &mut impl Channel) -> Self {
        Self {
            merkle_node: MerkleNodeRelation::draw(channel),
            sponge_step: SpongeStepRelation::draw(channel),
            sponge_data: SpongeDataRelation::draw(channel),
        }
    }
}
