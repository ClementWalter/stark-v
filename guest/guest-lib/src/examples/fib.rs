//! Fibonacci computation example.

use crate::types::FibResult;

/// Iterative Fibonacci computation returning a result struct.
pub fn fib(n: u32) -> FibResult {
    FibResult {
        n,
        value: fibonacci_impl(n),
    }
}

/// Iterative Fibonacci implementation returning raw value.
pub fn fibonacci_impl(n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    if n == 1 {
        return 1;
    }

    let mut a = 0u32;
    let mut b = 1u32;
    let mut i = 2u32;

    while i <= n {
        let tmp = a.wrapping_add(b);
        a = b;
        b = tmp;
        i += 1;
    }

    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fib() {
        assert_eq!(fib(20), FibResult { n: 20, value: 6765 });
    }

    #[test]
    fn test_fibonacci_impl() {
        assert_eq!(fibonacci_impl(0), 0);
        assert_eq!(fibonacci_impl(1), 1);
        assert_eq!(fibonacci_impl(2), 1);
        assert_eq!(fibonacci_impl(10), 55);
        assert_eq!(fibonacci_impl(20), 6765);
    }
}
