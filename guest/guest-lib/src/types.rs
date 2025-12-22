//! Result types for guest-host communication (serialized with postcard).

use serde::{Deserialize, Serialize};

/// Result of a simple computation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComputeResult {
    pub value: u32,
}

/// Result of Fibonacci computation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FibResult {
    pub n: u32,
    pub value: u32,
}

/// Result of factorial computation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FactorialResult {
    pub n: u32,
    pub value: u32,
}

/// Result of memory test.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryTestResult {
    pub sum: u32,
}

/// Result of mul/div test.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MulDivResult {
    pub value: u32,
}

/// Result of branch test.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BranchResult {
    pub x: u32,
    pub value: u32,
}
