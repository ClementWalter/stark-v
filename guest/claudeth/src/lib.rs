//! Claudeth - Dependency-free Ethereum State Transition Function for stark-v zkVM
//!
//! This library implements the Ethereum protocol execution layer with zero external dependencies,
//! designed for efficient proof generation in zkVM environments.
//!
//! ## Features
//!
//! - `no_std` compatible with `alloc` support (on riscv32 target)
//! - Minimal dependencies (serde only)
//! - Full Ethereum state transition function
//! - Optimized for Circle STARKs proof generation
//!
//! ## Architecture
//!
//! - [`types`]: Core Ethereum types (U256, Address, BlockHeader, etc.)
//! - [`crypto`]: Cryptographic primitives (Keccak256, ECDSA, etc.)
//! - [`state`]: State management and Merkle Patricia Trie
//! - [`evm`]: Ethereum Virtual Machine implementation

#![cfg_attr(target_arch = "riscv32", no_std)]

#[cfg(target_arch = "riscv32")]
extern crate alloc;

// =============================================================================
// Modules
// =============================================================================

pub mod crypto;
pub mod evm;
pub mod state;
pub mod types;

// =============================================================================
// Re-exports
// =============================================================================

// Re-export implemented types
pub use types::Bytes;

// Future re-exports will be added as more types and crypto modules are implemented
// pub use types::*;
// pub use crypto::*;

// =============================================================================
// Library metadata
// =============================================================================

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        // Verify version is set (const, so no need to check is_empty)
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }
}
