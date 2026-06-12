//! Preprocessed lookup tables for zkVM AIR constraints.
//!
//! Each table is constant across executions and is committed before the main
//! trace during proving. Multiplicity counters are driven from trace lookup
//! entries whose relation names appear in the `preprocessed` section of
//! [`crate::relations`].

pub mod bitwise;
pub mod range_check_20;
pub mod range_check_8_11;
pub mod range_check_8_8;
pub mod range_check_8_8_4;
pub mod range_check_m31;

pub use crate::relations::{Counter, Counters, PreProcessedTrace, PreprocessedTable};
