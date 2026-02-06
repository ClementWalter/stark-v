//! Compiler implementation for stark-v guest programs.
//!
//! This module provides the [`StarkVCompiler`] which implements the
//! [`ere_zkvm_interface::Compiler`] trait for compiling Rust programs
//! to RISC-V ELF binaries that can be executed and proven by stark-v.

use ere_zkvm_interface::Compiler;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during guest program compilation.
#[derive(Error, Debug)]
pub enum StarkVCompilerError {
    #[error("Cargo build failed: {0}")]
    CargoBuild(String),

    #[error("Failed to read ELF binary: {0}")]
    ReadElf(#[from] std::io::Error),

    #[error("Cargo metadata error: {0}")]
    CargoMetadata(String),

    #[error("No binary found in guest crate")]
    NoBinaryFound,

    #[error("Expected exactly one binary in guest crate, found: {bin_names:?}")]
    MultipleBinariesFound { bin_names: Vec<String> },

    #[error("Guest directory does not exist: {0}")]
    GuestNotFound(String),
}

/// Compiled stark-v program (ELF bytes).
#[derive(Clone, Serialize, Deserialize)]
pub struct StarkVProgram {
    /// Raw ELF bytes of the compiled program.
    pub elf_bytes: Vec<u8>,
}

/// Compiler for stark-v guest programs.
///
/// Implements [`ere_zkvm_interface::Compiler`] to compile Rust guest programs
/// to RISC-V ELF binaries.
#[derive(Clone, Default)]
pub struct StarkVCompiler {
    /// Additional cargo features to enable.
    pub features: Vec<String>,
}

impl StarkVCompiler {
    /// Create a new compiler with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a compiler with specific features enabled.
    pub fn with_features(features: Vec<String>) -> Self {
        Self { features }
    }
}

fn select_single_binary_name(mut bin_names: Vec<String>) -> Result<String, StarkVCompilerError> {
    bin_names.sort();
    bin_names.dedup();
    match bin_names.len() {
        0 => Err(StarkVCompilerError::NoBinaryFound),
        1 => Ok(bin_names.pop().expect("single element must exist")),
        _ => Err(StarkVCompilerError::MultipleBinariesFound { bin_names }),
    }
}

#[cfg(not(target_arch = "riscv32"))]
impl Compiler for StarkVCompiler {
    type Error = StarkVCompilerError;
    type Program = StarkVProgram;

    fn compile(&self, guest_directory: &Path) -> Result<Self::Program, Self::Error> {
        use std::process::Command;

        if !guest_directory.exists() {
            return Err(StarkVCompilerError::GuestNotFound(
                guest_directory.display().to_string(),
            ));
        }

        // Build the guest program for RISC-V target
        let mut cmd = Command::new("cargo");
        cmd.current_dir(guest_directory)
            .arg("build")
            .arg("--release")
            .arg("--target")
            .arg("riscv32im-unknown-none-elf");

        if !self.features.is_empty() {
            cmd.arg("--features").arg(self.features.join(","));
        }

        let output = cmd
            .output()
            .map_err(|e| StarkVCompilerError::CargoBuild(format!("Failed to run cargo: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(StarkVCompilerError::CargoBuild(stderr.to_string()));
        }

        // Find the compiled binary using cargo metadata
        let metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(guest_directory.join("Cargo.toml"))
            .exec()
            .map_err(|e| StarkVCompilerError::CargoMetadata(e.to_string()))?;

        // Get the package name
        let package = metadata
            .root_package()
            .ok_or(StarkVCompilerError::NoBinaryFound)?;

        // Construct the path to the release binary
        use cargo_metadata::TargetKind;
        let binary_names = package
            .targets
            .iter()
            .filter(|t| t.kind.iter().any(|k| matches!(k, TargetKind::Bin)))
            .map(|t| t.name.clone())
            .collect::<Vec<_>>();
        let binary_name = select_single_binary_name(binary_names)?;

        let elf_path = guest_directory
            .join("target")
            .join("riscv32im-unknown-none-elf")
            .join("release")
            .join(&binary_name);

        let elf_bytes = std::fs::read(&elf_path)?;

        Ok(StarkVProgram { elf_bytes })
    }
}

#[cfg(target_arch = "riscv32")]
impl Compiler for StarkVCompiler {
    type Error = StarkVCompilerError;
    type Program = StarkVProgram;

    fn compile(&self, _guest_directory: &Path) -> Result<Self::Program, Self::Error> {
        // Cannot compile on RISC-V target
        Err(StarkVCompilerError::CargoBuild(
            "Cannot compile guest programs on RISC-V target".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_creation() {
        let compiler = StarkVCompiler::new();
        assert!(compiler.features.is_empty());

        let compiler = StarkVCompiler::with_features(vec!["feature1".to_string()]);
        assert_eq!(compiler.features, vec!["feature1"]);
    }

    #[test]
    fn test_select_single_binary_name_none() {
        assert!(matches!(
            select_single_binary_name(vec![]),
            Err(StarkVCompilerError::NoBinaryFound)
        ));
    }

    #[test]
    fn test_select_single_binary_name_single() {
        let name = select_single_binary_name(vec!["guest".to_string()]).unwrap();
        assert_eq!(name, "guest");
    }

    #[test]
    fn test_select_single_binary_name_multiple() {
        let err = select_single_binary_name(vec!["b".to_string(), "a".to_string()]).unwrap_err();
        match err {
            StarkVCompilerError::MultipleBinariesFound { bin_names } => {
                assert_eq!(bin_names, vec!["a".to_string(), "b".to_string()]);
            }
            _ => panic!("expected multiple binaries error"),
        }
    }
}
