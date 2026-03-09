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
//! use stark_v_sdk::{secure_pcs_config, StarkV, StarkVCompiler};
//! use std::path::Path;
//!
//! // Compile a guest program
//! let compiler = StarkVCompiler::new();
//! let program = compiler.compile(Path::new("guest/sha256"))?;
//!
//! // Create VM instance and prove
//! let vm = StarkV::new(program, secure_pcs_config());
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
pub use prover::{FriConfig, PcsConfig, Proof};
pub use runner::{RunError, RunResult};

mod compiler;
mod proof_serde;
mod vm;

pub use compiler::{StarkVCompiler, StarkVCompilerError, StarkVProgram};
pub use vm::StarkV;

/// Maximum cycles for program execution (default).
pub const DEFAULT_MAX_CYCLES: u64 = 100_000_000;

/// Returns the secure PCS configuration used by stwo-cairo.
/// See https://github.com/starkware-libs/stwo-cairo/blob/0f63409c5f8d26ca70255fc53b82ee0352922765/stwo_cairo_prover/crates/prover/src/prover.rs#L287-L312
pub fn secure_pcs_config() -> PcsConfig {
    PcsConfig {
        pow_bits: 26,
        fri_config: FriConfig::new(0, 1, 70, 1),
        lifting_log_size: None,
    }
}

#[cfg(test)]
mod tests {
    use super::secure_pcs_config;

    #[test]
    fn test_secure_pcs_config_matches_expected_parameters() {
        let config = secure_pcs_config();
        assert_eq!(config.pow_bits, 26);
        assert_eq!(config.fri_config.log_last_layer_degree_bound, 0);
        assert_eq!(config.fri_config.log_blowup_factor, 1);
        assert_eq!(config.fri_config.n_queries, 70);
        assert_eq!(config.fri_config.line_fold_step, 1);
        assert_eq!(config.lifting_log_size, None);
    }
}
