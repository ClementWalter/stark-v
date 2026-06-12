//! Shared AIR schema for the stark-v zkVM.
//!
//! Trace table definitions, LogUp relation metadata, preprocessed lookup tables,
//! and the Poseidon2 permutation live here so the runner can fill traces and
//! the prover can prove them from the same source.

#![feature(allocator_api)]

#[macro_use]
mod schema;
pub use schema::relations;

pub mod decode;
pub mod merkle;
pub mod poseidon2;
pub mod preprocessed;

#[macro_use]
pub mod trace;

pub use merkle::MAX_TREE_HEIGHT;
