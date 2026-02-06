//! stark-v SDK - Build and prove RISC-V programs.
//!
//! This crate provides a unified interface for external consumers of stark-v,
//! including implementation of the [`ere_zkvm_interface`] traits for interoperability
//! with other zkVMs.
//!
//! # Modules
//!
//! - [`guest`] - Guest program utilities (I/O constants, helpers)
//! - [`prover`] - Proving and verification functions
//! - [`runner`] - Program execution and tracing
//!
//! # Example using ere-zkvm-interface
//!
//! ```ignore
//! use ere_zkvm_interface::{Compiler, Input, ProofKind, zkVM};
//! use stark_v_sdk::{StarkVCompiler, StarkV};
//! use std::path::Path;
//!
//! // Compile a guest program
//! let compiler = StarkVCompiler::new();
//! let program = compiler.compile(Path::new("guest/sha256"))?;
//!
//! // Create VM instance and prove
//! let vm = StarkV::new(program);
//! let input = Input::new().with_stdin(input_bytes);
//! let (public_values, proof, report) = vm.prove(&input, ProofKind::Compressed)?;
//!
//! // Verify
//! vm.verify(&proof)?;
//! ```

/// Guest program utilities: I/O memory layout, constants, and helpers.
pub use guest_lib as guest;

/// Proving and verification for RV32IM programs.
pub use prover;

/// Program execution and tracing.
pub use runner;

// Re-export key types for convenience
pub use prover::{PcsConfig, Proof};
pub use runner::{RunError, RunResult};

mod compiler;
mod proof_serde;
mod vm;

pub use compiler::{StarkVCompiler, StarkVCompilerError};
pub use vm::StarkV;

/// Maximum cycles for program execution (default).
pub const DEFAULT_MAX_CYCLES: u64 = 100_000_000;
