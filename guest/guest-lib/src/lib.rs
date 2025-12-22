#![cfg_attr(target_arch = "riscv32", no_std)]

/// Simple computation returning a constant.
pub fn compute() -> u32 {
    42
}

/// Default main entrypoint.
pub fn main() -> u32 {
    compute()
}

/// Iterative Fibonacci computation.
pub fn fibonacci(n: u32) -> u32 {
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

/// Iterative factorial computation.
pub fn factorial(n: u32) -> u32 {
    let mut result = 1u32;
    let mut i = 1u32;

    while i <= n {
        result = result.wrapping_mul(i);
        i += 1;
    }

    result
}

/// Memory stress test: write and read back array values.
pub fn memory_test() -> u32 {
    let mut arr = [0u32; 16];

    // Write pattern
    let mut i = 0usize;
    while i < 16 {
        arr[i] = (i as u32).wrapping_mul(7).wrapping_add(3);
        i += 1;
    }

    // Read and sum
    let mut sum = 0u32;
    i = 0;
    while i < 16 {
        sum = sum.wrapping_add(arr[i]);
        i += 1;
    }

    sum
}

/// M-extension test: multiply and divide operations.
pub fn muldiv_test() -> u32 {
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

/// Branch test: multiple conditional branches.
pub fn branch_test(x: u32) -> u32 {
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
    fn test_compute() {
        assert_eq!(compute(), 42);
    }

    #[test]
    fn test_fibonacci() {
        assert_eq!(fibonacci(0), 0);
        assert_eq!(fibonacci(1), 1);
        assert_eq!(fibonacci(2), 1);
        assert_eq!(fibonacci(10), 55);
        assert_eq!(fibonacci(20), 6765);
    }

    #[test]
    fn test_factorial() {
        assert_eq!(factorial(0), 1);
        assert_eq!(factorial(1), 1);
        assert_eq!(factorial(5), 120);
        assert_eq!(factorial(10), 3628800);
    }

    #[test]
    fn test_memory() {
        // Sum of (i*7+3) for i=0..15 = 7*(0+1+...+15) + 3*16 = 7*120 + 48 = 888
        assert_eq!(memory_test(), 888);
    }

    #[test]
    fn test_branch() {
        // x=5: not 0, not !=5, <10, >=0, <100, >=0 = 0+0+4+8+16+32 = 60
        assert_eq!(branch_test(5), 4 + 8 + 16 + 32);
        // x=0: ==0, !=5, <10, >=0, <100, >=0 = 1+2+4+8+16+32 = 63
        assert_eq!(branch_test(0), 63);
    }
}
