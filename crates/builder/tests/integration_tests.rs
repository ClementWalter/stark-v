//! Integration tests for the builder crate.
//!
//! These tests require the risc0 toolchain to be installed.
//! Run with: cargo test --package builder --test integration_tests

use std::path::Path;

/// Check if the risc0 toolchain is available and we're not in a coverage build.
/// Coverage builds use llvm-cov which instruments the code, and this doesn't
/// work well with cross-compilation to risc0 target.
fn risc0_available() -> bool {
    // Skip if we're in a coverage build (llvm-cov uses a specific target dir)
    if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
        if target_dir.contains("llvm-cov") {
            return false;
        }
    }

    // Also check for LLVM_PROFILE_FILE which is set during coverage runs
    if std::env::var("LLVM_PROFILE_FILE").is_ok() {
        return false;
    }

    // Check if risc0 toolchain is installed
    std::process::Command::new("rustup")
        .args(["run", "risc0", "rustc", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Test that build_guest successfully builds a guest package.
///
/// This test requires the risc0 toolchain to be installed.
#[test]
fn test_build_guest_success() {
    if !risc0_available() {
        eprintln!("Skipping test: risc0 toolchain not available");
        return;
    }

    // Path to the test guest fixture
    let guest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/guest");

    // Clean any previous build artifacts
    let target_dir = guest_path.join("target");
    if target_dir.exists() {
        std::fs::remove_dir_all(&target_dir).ok();
    }

    // Build the guest
    let result = builder::build_guest(&guest_path);

    match result {
        Ok(output) => {
            // Verify the ELF path is correct
            assert!(output.elf_path.exists(), "ELF file should exist");
            assert!(
                output.elf_path.ends_with("test-guest"),
                "ELF should be named test-guest"
            );

            // Verify we can load the ELF with the runner
            let vm_exe = runner::load_vm_exe_from_elf(&output.elf_path);
            assert!(vm_exe.is_ok(), "Should be able to load the built ELF");
        }
        Err(e) => {
            panic!("Build failed: {}", e);
        }
    }
}

/// Test that build_guest returns the correct output structure.
#[test]
fn test_build_output_structure() {
    if !risc0_available() {
        eprintln!("Skipping test: risc0 toolchain not available");
        return;
    }

    let guest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/guest");

    let result = builder::build_guest(&guest_path);

    match result {
        Ok(output) => {
            // Check that elf_path contains expected path components
            let path_str = output.elf_path.to_string_lossy();
            assert!(path_str.contains("riscv32im-risc0-zkvm-elf"));
            assert!(path_str.contains("release"));
            assert!(path_str.contains("test-guest"));
        }
        Err(e) => {
            panic!("Build failed: {}", e);
        }
    }
}
