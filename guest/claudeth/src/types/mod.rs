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
pub mod transaction;
pub mod uint;

pub use address::Address;
pub use block::{BlockHeader, EMPTY_OMMERS_HASH, ValidationError, Withdrawal};
pub use bytes::Bytes;
pub use hash::{H256, Hash};
pub use transaction::{
    AccessListEntry, Eip1559Transaction, Eip2930Transaction, LegacyTransaction, Transaction,
};
pub use uint::{U256, U512};
