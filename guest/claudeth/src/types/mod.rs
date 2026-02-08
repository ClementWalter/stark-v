//! Core Ethereum types
//!
//! This module provides the fundamental types used in the Ethereum protocol:
//! - Big integers (U256, U512)
//! - Addresses and hashes
//! - Block headers
//! - Transactions
//! - Dynamic byte arrays

pub mod address;
pub mod block;
pub mod bytes;
pub mod hash;
pub mod uint;

pub use address::Address;
pub use block::{BlockHeader, ValidationError};
pub use bytes::Bytes;
pub use hash::{Hash, H256};
pub use uint::{U256, U512};
