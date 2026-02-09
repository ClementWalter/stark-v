//! LOG0–LOG4 opcodes: pop offset/size/topics, charge gas, read memory, return (topics, data) for the caller to append as LogEntry.

#![cfg_attr(target_arch = "riscv32", no_std)]

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

use crate::evm::error::EvmError;
use crate::evm::{Memory, Stack, log_gas_cost, memory_expansion_cost};
use crate::types::{Address, Hash};

use super::utils;

/// Execute LOGn (0xA0 + n). Pops offset, size, and n topics; charges memory expansion + log gas;
/// reads data from memory; returns (address, topics, data) for the caller to append to logs.
pub fn execute_log(
    stack: &mut Stack,
    memory: &mut Memory,
    num_topics: u8,
    _address: Address,
    gas_remaining: &mut u64,
) -> Result<(Vec<Hash>, Vec<u8>), EvmError> {
    let offset = stack.pop().map_err(EvmError::from)?.as_usize();
    let size = stack.pop().map_err(EvmError::from)?.as_usize();

    let mem_cost = memory_expansion_cost(memory.msize(), offset + size);
    utils::consume_gas(gas_remaining, mem_cost)?;
    utils::consume_gas(gas_remaining, log_gas_cost(num_topics, size))?;

    let mut topics = Vec::with_capacity(num_topics as usize);
    for _ in 0..num_topics {
        let t = stack.pop().map_err(EvmError::from)?;
        topics.push(utils::u256_to_hash(&t));
    }

    let data = utils::read_memory_bytes(memory, offset, size)?;
    Ok((topics, data))
}
