//! Factorial computation example.

use serde::{Deserialize, Serialize};

/// Result of factorial computation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FactorialResult {
    pub n: u32,
    pub value: u32,
}

/// Iterative factorial computation returning a result struct.
pub fn fact(n: u32) -> FactorialResult {
    FactorialResult {
        n,
        value: factorial_impl(n),
    }
}

/// Iterative factorial implementation returning raw value.
pub fn factorial_impl(n: u32) -> u32 {
    let mut result = 1u32;
    let mut i = 1u32;

    while i <= n {
        result = result.wrapping_mul(i);
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fact() {
        assert_eq!(
            fact(10),
            FactorialResult {
                n: 10,
                value: 3628800
            }
        );
    }

    #[test]
    fn test_factorial_impl() {
        assert_eq!(factorial_impl(0), 1);
        assert_eq!(factorial_impl(1), 1);
        assert_eq!(factorial_impl(5), 120);
        assert_eq!(factorial_impl(10), 3628800);
    }
}
