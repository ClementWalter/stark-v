//! Bitwise multiplicity component.
//!
//! Tracks how many times each `(limb_0, limb_1, result, bitwise_id)` tuple is used by opcode traces.
//! Provides the "preprocessed side" of the LogUp relation.

pub mod air;
pub mod columns;
pub mod witness;
