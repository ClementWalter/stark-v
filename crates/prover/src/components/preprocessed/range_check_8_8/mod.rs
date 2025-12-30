//! Range check (8, 8)-bit multiplicity component.
//!
//! Tracks how many times each tuple in `[0, 2^8) × [0, 2^8)` is used by opcode traces.
//! Provides the "preprocessed side" of the LogUp relation.

pub mod air;
pub mod columns;
pub mod witness;
