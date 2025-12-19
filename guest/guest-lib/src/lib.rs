#![cfg_attr(target_arch = "riscv32", no_std)]

pub fn compute() -> u32 {
    42
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute() {
        assert_eq!(compute(), 42);
    }
}
