//! Simple computation example.

use crate::types::ComputeResult;

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
