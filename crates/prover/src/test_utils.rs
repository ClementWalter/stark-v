//! Shared test utilities for e2e opcode tests.
//!
//! Provides infrastructure to run guest-tests binaries and validate AIR constraints.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Once;

use runner::run;

static BUILD_TESTS: Once = Once::new();

/// Build guest-tests binaries once before running tests.
pub fn ensure_tests_built() {
    BUILD_TESTS.call_once(|| {
        let guest_tests_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("guest")
            .join("guest-tests");

        let status = Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(&guest_tests_dir)
            .status()
            .expect("Failed to execute cargo build for guest-tests");

        assert!(status.success(), "Failed to build guest-tests binaries");
    });
}

/// Path to the guest-tests binary directory.
pub fn guest_tests_bin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("guest")
        .join("guest-tests")
        .join("target")
        .join("riscv32im-unknown-none-elf")
        .join("release")
}

/// Run a guest-tests binary and return the tracer.
pub fn run_test_bin(name: &str) -> runner::trace::Tracer {
    ensure_tests_built();

    let elf_path = guest_tests_bin_dir().join(name);
    let elf_bytes =
        std::fs::read(&elf_path).unwrap_or_else(|e| panic!("Failed to read ELF {elf_path:?}: {e}"));

    let result = run(&elf_bytes, 10_000).unwrap_or_else(|e| panic!("Failed to run {name}: {e}"));

    result.tracer
}
