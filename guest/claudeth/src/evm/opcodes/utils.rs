//! Shared helpers for opcode execution (address/hash conversion, memory read/write, gas).

#![cfg_attr(target_arch = "riscv32", no_std)]

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

use crate::evm::error::EvmError;
use crate::evm::memory::{Memory, MemoryError};
use crate::types::{Address, Hash, U256};

// =============================================================================
// Address / Hash conversion (no unwrap/expect)
// =============================================================================

/// Convert an Address (20 bytes) to U256 (32 bytes, big-endian, zero-padded high 12 bytes).
#[inline]
pub fn address_to_u256(address: &Address) -> U256 {
    let mut bytes = [0u8; 32];
    bytes[12..32].copy_from_slice(address.as_bytes());
    U256::from_be_bytes(bytes)
}

/// Convert U256 to Address (last 20 bytes). Fails if the value cannot represent a valid address.
#[inline]
pub fn u256_to_address(u256: &U256) -> Result<Address, EvmError> {
    let bytes = u256.to_be_bytes();
    let mut addr_bytes = [0u8; 20];
    addr_bytes.copy_from_slice(&bytes[12..32]);
    Address::from_slice(&addr_bytes).ok_or(EvmError::InvalidAddress)
}

/// Convert Hash (32 bytes) to U256 big-endian.
#[inline]
pub fn hash_to_u256(hash: &Hash) -> U256 {
    U256::from_be_bytes(*hash.as_bytes())
}

/// Convert U256 to Hash (32 bytes big-endian).
#[inline]
pub fn u256_to_hash(value: &U256) -> Hash {
    Hash::from(value.to_be_bytes())
}

// =============================================================================
// Memory byte read/write (for CALL, CREATE, LOG, RETURN, REVERT)
// =============================================================================

/// Read `size` bytes from memory at `offset`. Zero-pads if range extends past current msize.
pub fn read_memory_bytes(
    memory: &mut Memory,
    offset: usize,
    size: usize,
) -> Result<Vec<u8>, EvmError> {
    if size == 0 {
        return Ok(Vec::new());
    }
    // After gas charging, size should be reasonable. Cap capacity as defense-in-depth.
    let mut out = Vec::with_capacity(size.min(32 * 1024 * 1024));
    for i in 0..size {
        let pos = offset.checked_add(i).ok_or(EvmError::OutOfGas)?;
        let byte = if pos < memory.msize() {
            let value = memory.mload(pos & !31).map_err(EvmError::MemoryError)?;
            let byte_offset = pos % 32;
            value.to_be_bytes()[byte_offset]
        } else {
            0
        };
        out.push(byte);
    }
    Ok(out)
}

/// Expand memory for `size` bytes at `offset`, then write up to `data.len()`.
///
/// Bytes in `[offset + data.len(), offset + size)` are left unchanged.
pub fn write_memory_bytes(
    memory: &mut Memory,
    offset: usize,
    data: &[u8],
    size: usize,
) -> Result<(), EvmError> {
    if size == 0 {
        return Ok(());
    }

    let end = offset
        .checked_add(size)
        .ok_or(EvmError::MemoryError(MemoryError::Overflow))?;
    // Why: mirror CALL-family semantics from execution-specs: precharged
    // output range expands memory, but only actual return-data bytes are copied.
    let _ = memory.expand(end).map_err(EvmError::MemoryError)?;

    let copy_size = size.min(data.len());
    for (i, byte) in data.iter().take(copy_size).enumerate() {
        let pos = offset
            .checked_add(i)
            .ok_or(EvmError::MemoryError(MemoryError::Overflow))?;
        memory.mstore8(pos, *byte).map_err(EvmError::MemoryError)?;
    }
    Ok(())
}

// =============================================================================
// Gas
// =============================================================================

/// Consume `amount` from `gas_remaining`. Returns `Err(EvmError::OutOfGas)` if insufficient.
pub fn consume_gas(gas_remaining: &mut u64, amount: u64) -> Result<(), EvmError> {
    if *gas_remaining < amount {
        return Err(EvmError::OutOfGas);
    }
    *gas_remaining -= amount;
    Ok(())
}
