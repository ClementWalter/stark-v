//! Preprocessed columns for the prover.
//!
//! Table definitions live in the shared [`air`] crate; this module re-exports
//! them for prover-local paths and hosts cached preprocessing artifacts.

pub use air::preprocessed::*;

mod preprocessing;

pub use preprocessing::{Preprocessing, preprocess, preprocess_with_channel};
