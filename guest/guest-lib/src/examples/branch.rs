//! Branch test example.

use crate::types::BranchResult;

/// Branch test returning a result struct.
pub fn branch(x: u32) -> BranchResult {
    BranchResult {
        x,
        value: branch_test_impl(x),
    }
}

/// Branch test implementation: multiple conditional branches.
pub fn branch_test_impl(x: u32) -> u32 {
    let mut result = 0u32;

    if x == 0 {
        result = result.wrapping_add(1);
    }
    if x != 5 {
        result = result.wrapping_add(2);
    }
    if (x as i32) < 10 {
        result = result.wrapping_add(4);
    }
    if (x as i32) >= 0 {
        result = result.wrapping_add(8);
    }
    if x < 100 {
        result = result.wrapping_add(16);
    }
    // Always true for unsigned, but exercises bgeu
    result = result.wrapping_add(32);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch() {
        assert_eq!(branch(5), BranchResult { x: 5, value: 60 });
        assert_eq!(branch(0), BranchResult { x: 0, value: 63 });
    }

    #[test]
    fn test_branch_impl() {
        // x=5: not 0, not !=5, <10, >=0, <100, >=0 = 0+0+4+8+16+32 = 60
        assert_eq!(branch_test_impl(5), 4 + 8 + 16 + 32);
        // x=0: ==0, !=5, <10, >=0, <100, >=0 = 1+2+4+8+16+32 = 63
        assert_eq!(branch_test_impl(0), 63);
    }
}
