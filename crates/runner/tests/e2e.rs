//! End-to-end tests comparing interpreter output with native execution.
//!
//! These tests:
//! 1. Build guest ELF binaries (once per test run)
//! 2. Run them through the interpreter
//! 3. Deserialize the postcard output
//! 4. Compare with native execution of the same function

use guest_lib::{
    branch_test, factorial, fibonacci, memory_test, muldiv_test, BranchResult, ComputeResult,
    FactorialResult, FibResult, MemoryTestResult, MulDivResult,
};
use runner::run;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Once;

static BUILD_GUEST: Once = Once::new();

/// Build guest binaries once before running tests.
fn ensure_guest_built() {
    BUILD_GUEST.call_once(|| {
        let guest_bin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../guest/guest-bin");

        let status = Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(&guest_bin_dir)
            .status()
            .expect("Failed to execute cargo build for guest-bin");

        assert!(status.success(), "Failed to build guest binaries");
    });
}

/// Path to the guest binary directory.
fn guest_bin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../guest/guest-bin/target/riscv32im-unknown-none-elf/release")
}

/// Helper to load and run an ELF, returning the deserialized output.
fn run_guest<T: serde::de::DeserializeOwned>(name: &str) -> T {
    ensure_guest_built();

    let elf_path = guest_bin_dir().join(name);
    let elf_bytes = std::fs::read(&elf_path)
        .unwrap_or_else(|e| panic!("Failed to read ELF {}: {}", elf_path.display(), e));

    let result =
        run(&elf_bytes, 10_000_000).unwrap_or_else(|e| panic!("Failed to run {}: {}", name, e));

    let output = result
        .output
        .unwrap_or_else(|| panic!("No output from {}", name));

    postcard::from_bytes(&output)
        .unwrap_or_else(|e| panic!("Failed to deserialize output from {}: {}", name, e))
}

#[test]
fn test_guest_bin_main() {
    let output: ComputeResult = run_guest("guest-bin");
    let expected = ComputeResult {
        value: guest_lib::main(),
    };
    assert_eq!(output, expected);
}

#[test]
fn test_fibonacci() {
    let output: FibResult = run_guest("fib");

    let n = 20;
    let expected = FibResult {
        n,
        value: fibonacci(n),
    };

    assert_eq!(output, expected);
}

#[test]
fn test_factorial() {
    let output: FactorialResult = run_guest("factorial");

    let n = 10;
    let expected = FactorialResult {
        n,
        value: factorial(n),
    };

    assert_eq!(output, expected);
}

#[test]
fn test_memory() {
    let output: MemoryTestResult = run_guest("memory");

    let expected = MemoryTestResult { sum: memory_test() };

    assert_eq!(output, expected);
}

#[test]
fn test_muldiv() {
    let output: MulDivResult = run_guest("muldiv");

    let expected = MulDivResult {
        value: muldiv_test(),
    };

    assert_eq!(output, expected);
}

#[test]
fn test_branch() {
    let output: BranchResult = run_guest("branch");

    let x = 5;
    let expected = BranchResult {
        x,
        value: branch_test(x),
    };

    assert_eq!(output, expected);
}
