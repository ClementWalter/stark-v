//! Constant computation example.

use serde::{Deserialize, Serialize};

/// Result of a constant computation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConstantResult {
    pub value: u32,
}

/// Constant computation returning a constant.
pub fn constant() -> ConstantResult {
    ConstantResult { value: 42 }
}

/// Standard test entry point for e2e testing.
pub fn test_call() -> ConstantResult {
    constant()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant() {
        assert_eq!(constant(), ConstantResult { value: 42 });
    }
}
