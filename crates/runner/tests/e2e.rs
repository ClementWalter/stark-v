//! End-to-end tests comparing interpreter output with native execution.
//!
//! These tests:
//! 1. Build guest ELF binaries (once per test run)
//! 2. Run them through the interpreter
//! 3. Deserialize the postcard output
//! 4. Compare with native execution of the same function

use guest_lib::{
    branch, constant, fact, fib, memory, muldiv, BranchResult, ConstantResult, FactorialResult,
    FibResult, MemoryTestResult, MulDivResult,
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
fn test_compute() {
    let output: ConstantResult = run_guest("constant");
    assert_eq!(output, constant());
}

#[test]
fn test_fibonacci() {
    let output: FibResult = run_guest("fib");
    assert_eq!(output, fib(20));
}

#[test]
fn test_factorial() {
    let output: FactorialResult = run_guest("factorial");
    assert_eq!(output, fact(10));
}

#[test]
fn test_memory() {
    let output: MemoryTestResult = run_guest("memory");
    assert_eq!(output, memory());
}

#[test]
fn test_muldiv() {
    let output: MulDivResult = run_guest("muldiv");
    assert_eq!(output, muldiv());
}

#[test]
fn test_branch() {
    let output: BranchResult = run_guest("branch");
    assert_eq!(output, branch(5));
}
