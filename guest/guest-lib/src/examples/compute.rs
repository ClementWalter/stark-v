//! Simple computation example.

use serde::{Deserialize, Serialize};

/// Result of a simple computation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComputeResult {
    pub value: u32,
}

/// Simple computation returning a constant.
pub fn compute() -> ComputeResult {
    ComputeResult { value: 42 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute() {
        assert_eq!(compute(), ComputeResult { value: 42 });
    }
}
