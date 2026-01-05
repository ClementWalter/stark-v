//! stark-v SDK - Build and prove RISC-V programs.
//!
//! This crate provides a unified interface for external consumers of stark-v.
//!
//! # Modules
//!
//! - [`guest`] - Guest program utilities (I/O constants, helpers)
//! - [`prover`] - Proving and verification functions
//! - [`runner`] - Program execution and tracing
//!
//! # Example
//!
//! ```ignore
//! use stark_v_sdk::runner::run_with_input;
//! use stark_v_sdk::prover::{prove_rv32im, verify_rv32im, PcsConfig};
//!
//! // Run the program
//! let elf_bytes = std::fs::read("program.elf")?;
//! let result = run_with_input(&elf_bytes, &input, 1_000_000)?;
//!
//! // Generate proof
//! let config = PcsConfig::default();
//! let proof = prove_rv32im(result, config.clone());
//!
//! // Verify proof
//! verify_rv32im(proof, config)?;
//! ```

/// Guest program utilities: I/O memory layout, constants, and helpers.
pub use guest_lib as guest;

/// Proving and verification for RV32IM programs.
pub use prover;

/// Program execution and tracing.
pub use runner;
