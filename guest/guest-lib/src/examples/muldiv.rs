//! M-extension (multiply/divide) test example.

use serde::{Deserialize, Serialize};

/// Result of mul/div test.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MulDivResult {
    pub value: u32,
}

/// M-extension test returning a result struct.
pub fn muldiv() -> MulDivResult {
    MulDivResult {
        value: muldiv_test_impl(),
    }
}

/// M-extension test implementation: multiply and divide operations.
pub fn muldiv_test_impl() -> u32 {
    let a: u32 = 12345;
    let b: u32 = 6789;

    let mul_result = a.wrapping_mul(b);
    let div_result = mul_result / b;
    let rem_result = mul_result % a;

    // Signed operations
    let sa: i32 = -1234;
    let sb: i32 = 567;
    let smul = sa.wrapping_mul(sb) as u32;
    let sdiv = (sa / sb) as u32;

    div_result
        .wrapping_add(rem_result)
        .wrapping_add(smul)
        .wrapping_add(sdiv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_muldiv() {
        assert_eq!(
            muldiv(),
            MulDivResult {
                value: muldiv_test_impl()
            }
        );
    }
}
