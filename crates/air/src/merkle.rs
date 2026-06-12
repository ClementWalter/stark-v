//! Merkle-tree constants shared by trace constraints and runner commitment code.

/// Maximum binary Merkle tree height for memory and proof commitments.
///
/// Leaf depth in trace lookups is `MAX_TREE_HEIGHT - 1` because depth counts
/// edges from the root to a leaf index.
pub const MAX_TREE_HEIGHT: u32 = 31;
