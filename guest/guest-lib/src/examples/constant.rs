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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant() {
        assert_eq!(constant(), ConstantResult { value: 42 });
    }
}
