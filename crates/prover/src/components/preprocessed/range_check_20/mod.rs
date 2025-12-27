//! Range check 20-bit multiplicity component.
//!
//! Tracks how many times each value in [0, 2^20) is used by opcode traces.
//! Provides the "preprocessed side" of the LogUp relation.

pub mod air;
pub mod columns;
pub mod witness;
