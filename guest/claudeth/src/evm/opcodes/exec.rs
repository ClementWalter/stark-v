//! Single opcode dispatcher: match opcode, delegate to arithmetic/control/environment/log,
//! apply PC updates and dynamic gas, return StepOutcome for the interpreter.

#![cfg_attr(target_arch = "riscv32", no_std)]

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::collections::BTreeSet;

#[cfg(target_arch = "riscv32")]
use alloc::collections::BTreeSet;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

use crate::evm::error::EvmError;
use crate::evm::host::{Host, compute_create_address, compute_create2_address};
use crate::evm::memory::MemoryError;
use crate::evm::opcodes::arithmetic;
use crate::evm::opcodes::control::{self, OpcodeError};
use crate::evm::opcodes::environment::{self, CallEnv};
use crate::evm::opcodes::log;
use crate::evm::opcodes::utils;
use crate::evm::{
    GAS_SSTORE_SENTRY, Memory, Stack, copy_gas_cost, create2_hash_cost, init_code_gas_cost,
    memory_expansion_cost, memory_expansion_cost_for_range,
};
use crate::state::State;
use crate::types::{Address, Hash, U256};

// =============================================================================
// Step outcome (PC / stop / return / revert / log)
// =============================================================================

/// Result of executing one opcode: how to advance PC and whether to stop/return/revert.
#[derive(Debug)]
pub enum StepOutcome {
    /// Normal advance: interpreter should set pc += advance (e.g. 1 for normal, 1+n for PUSHn).
    Continue(u32),
    /// STOP: interpreter sets stopped = true, pc += 1.
    Stop,
    /// RETURN: interpreter sets return_data = data, stopped = true.
    Return(Vec<u8>),
    /// REVERT: interpreter returns Err(EvmError::Revert(data)).
    Revert(Vec<u8>),
    /// JUMP/JUMPI: interpreter sets pc = new_pc.
    Jump(usize),
    /// LOG: interpreter pushes LogEntry(address, topics, data) to logs.
    Log(Address, Vec<Hash>, Vec<u8>),
}

// =============================================================================
// Exec context (all state needed to run one opcode)
// =============================================================================

/// Context passed to execute_opcode. Interpreter fills this from Evm state each step.
pub struct ExecContext<'a, S: State, H: Host<S>> {
    pub stack: &'a mut Stack,
    pub memory: &'a mut Memory,
    pub state: &'a mut S,
    pub host: &'a mut H,
    pub code: &'a [u8],
    pub pc: usize,
    pub gas_remaining: &'a mut u64,
    pub gas_refund: &'a mut u64,
    pub return_data: &'a mut Vec<u8>,
    pub jumpdests: &'a [bool],
    pub accessed_addresses: &'a mut BTreeSet<Address>,
    pub accessed_storage: &'a mut BTreeSet<(Address, U256)>,
    // Block
    pub block_number: U256,
    pub block_timestamp: U256,
    pub block_coinbase: Address,
    pub block_difficulty: U256,
    pub block_gas_limit: U256,
    pub block_chain_id: U256,
    pub block_base_fee: U256,
    pub block_excess_blob_gas: Option<U256>,
    // Tx
    pub tx_origin: Address,
    pub tx_gas_price: U256,
    // Call
    pub call_address: Address,
    pub call_caller: Address,
    pub call_value: U256,
    pub call_data: &'a [u8],
}

fn opcode_error_to_evm(err: OpcodeError) -> EvmError {
    match err {
        OpcodeError::InvalidJumpDestination => EvmError::InvalidJump,
        OpcodeError::InvalidPush => EvmError::InvalidPush,
        OpcodeError::Stack(e) => EvmError::StackError(e),
        OpcodeError::Memory(e) => EvmError::MemoryError(e),
        OpcodeError::InvalidProgramCounter => EvmError::PcOutOfBounds,
        OpcodeError::InvalidOffset => EvmError::MemoryError(MemoryError::InvalidOffset),
        OpcodeError::NotImplemented => EvmError::InvalidOpcode(0),
    }
}

fn access_address<S: State, H: Host<S>>(
    ctx: &mut ExecContext<'_, S, H>,
    address: &Address,
) -> bool {
    if ctx.accessed_addresses.contains(address) {
        true
    } else {
        ctx.accessed_addresses.insert(*address);
        false
    }
}

fn access_storage<S: State, H: Host<S>>(
    ctx: &mut ExecContext<'_, S, H>,
    address: &Address,
    key: &U256,
) -> bool {
    let key_pair = (*address, *key);
    if ctx.accessed_storage.contains(&key_pair) {
        true
    } else {
        ctx.accessed_storage.insert(key_pair);
        false
    }
}

// Build environment BlockContext for opcodes that need it
fn block_ctx<S: State, H: Host<S>>(ctx: &ExecContext<'_, S, H>) -> environment::BlockContext {
    environment::BlockContext {
        coinbase: ctx.block_coinbase,
        timestamp: ctx.block_timestamp,
        number: ctx.block_number,
        difficulty: ctx.block_difficulty,
        gas_limit: ctx.block_gas_limit,
        chain_id: ctx.block_chain_id,
        base_fee: ctx.block_base_fee,
        excess_blob_gas: ctx.block_excess_blob_gas,
    }
}

fn tx_ctx<S: State, H: Host<S>>(ctx: &ExecContext<'_, S, H>) -> environment::TxContext {
    environment::TxContext {
        origin: ctx.tx_origin,
        gas_price: ctx.tx_gas_price,
    }
}

fn call_env<S: State, H: Host<S>>(ctx: &ExecContext<'_, S, H>) -> CallEnv {
    CallEnv {
        self_address: ctx.call_address,
        caller: ctx.call_caller,
        call_value: ctx.call_value,
    }
}

fn memory_range_end(offset: usize, size: usize) -> usize {
    if size == 0 {
        0
    } else {
        offset.saturating_add(size)
    }
}

/// Optional storage write for tracer (SSTORE). Interpreter converts to trace::StorageWrite.
pub type StorageWriteInfo = (Address, U256, U256);

/// Execute one opcode. Interpreter has already charged base gas.
/// Returns (outcome, optional SSTORE info for tracing).
pub fn execute_opcode<S: State, H: Host<S>>(
    opcode: u8,
    ctx: &mut ExecContext<'_, S, H>,
) -> Result<(StepOutcome, Option<StorageWriteInfo>), EvmError> {
    use StepOutcome::*;

    let outcome = match opcode {
        // 0x00: STOP
        0x00 => return Ok((Stop, None)),

        // 0x01-0x0B: Arithmetic
        0x01 => {
            arithmetic::add(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x02 => {
            arithmetic::mul(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x03 => {
            arithmetic::sub(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x04 => {
            arithmetic::div(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x05 => {
            arithmetic::sdiv(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x06 => {
            arithmetic::modulo(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x07 => {
            arithmetic::smod(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x08 => {
            arithmetic::addmod(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x09 => {
            arithmetic::mulmod(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x0A => {
            // EXP: dynamic gas from exponent (top = peek(0))
            let exponent = *ctx.stack.peek(0).map_err(EvmError::from)?;
            let exp_bytes = exponent.bits().div_ceil(8);
            let extra = 50u64.saturating_mul(exp_bytes as u64);
            utils::consume_gas(ctx.gas_remaining, extra)?;
            arithmetic::exp(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x0B => {
            arithmetic::signextend(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }

        // 0x10-0x1D: Comparison and bitwise
        0x10 => {
            arithmetic::lt(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x11 => {
            arithmetic::gt(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x12 => {
            arithmetic::slt(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x13 => {
            arithmetic::sgt(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x14 => {
            arithmetic::eq(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x15 => {
            arithmetic::iszero(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x16 => {
            arithmetic::and(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x17 => {
            arithmetic::or(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x18 => {
            arithmetic::xor(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x19 => {
            arithmetic::not(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x1A => {
            arithmetic::byte(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x1B => {
            arithmetic::shl(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x1C => {
            arithmetic::shr(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }
        0x1D => {
            arithmetic::sar(ctx.stack).map_err(EvmError::from)?;
            Continue(1)
        }

        // 0x20: KECCAK256
        0x20 => {
            let offset = ctx.stack.peek(0).map_err(EvmError::from)?.as_usize();
            let size = ctx.stack.peek(1).map_err(EvmError::from)?.as_usize();
            let mem_cost = memory_expansion_cost_for_range(ctx.memory.msize(), offset, size);
            utils::consume_gas(ctx.gas_remaining, mem_cost)?;
            let words = size.div_ceil(32);
            utils::consume_gas(ctx.gas_remaining, 6 * words as u64)?;
            arithmetic::keccak256(ctx.stack, ctx.memory).map_err(EvmError::from)?;
            Continue(1)
        }

        // 0x30-0x3F: Environment
        0x30 => {
            ctx.stack
                .push(utils::address_to_u256(&ctx.call_address))
                .map_err(EvmError::from)?;
            Continue(1)
        }
        0x31 => {
            let is_warm = access_address(
                ctx,
                &utils::u256_to_address(ctx.stack.peek(0).map_err(EvmError::from)?)?,
            );
            environment::balance(ctx.stack, ctx.state, is_warm, ctx.gas_remaining)?;
            Continue(1)
        }
        0x32 => {
            let tc = tx_ctx(ctx);
            environment::origin(ctx.stack, &tc).map_err(opcode_error_to_evm)?;
            Continue(1)
        }
        0x33 => {
            ctx.stack
                .push(utils::address_to_u256(&ctx.call_caller))
                .map_err(EvmError::from)?;
            Continue(1)
        }
        0x34 => {
            ctx.stack.push(ctx.call_value).map_err(EvmError::from)?;
            Continue(1)
        }
        0x35 => {
            let offset = ctx.stack.pop().map_err(EvmError::from)?.as_usize();
            let mut data = [0u8; 32];
            let len = ctx.call_data.len();
            if offset < len {
                let end = (offset + 32).min(len);
                data[..end - offset].copy_from_slice(&ctx.call_data[offset..end]);
            }
            ctx.stack
                .push(U256::from_be_bytes(data))
                .map_err(EvmError::from)?;
            Continue(1)
        }
        0x36 => {
            ctx.stack
                .push(U256::from_u64(ctx.call_data.len() as u64))
                .map_err(EvmError::from)?;
            Continue(1)
        }
        0x37 => {
            let dest = ctx.stack.peek(0).map_err(EvmError::from)?.as_usize();
            let offset = ctx.stack.peek(1).map_err(EvmError::from)?.as_usize();
            let size = ctx.stack.peek(2).map_err(EvmError::from)?.as_usize();
            let mem_cost = memory_expansion_cost_for_range(ctx.memory.msize(), dest, size);
            utils::consume_gas(ctx.gas_remaining, mem_cost)?;
            utils::consume_gas(ctx.gas_remaining, copy_gas_cost(size))?;
            for i in 0..size {
                let byte = if offset + i < ctx.call_data.len() {
                    ctx.call_data[offset + i]
                } else {
                    0
                };
                ctx.memory.mstore8(dest + i, byte).map_err(EvmError::from)?;
            }
            ctx.stack.pop().map_err(EvmError::from)?;
            ctx.stack.pop().map_err(EvmError::from)?;
            ctx.stack.pop().map_err(EvmError::from)?;
            Continue(1)
        }
        0x38 => {
            ctx.stack
                .push(U256::from_u64(ctx.code.len() as u64))
                .map_err(EvmError::from)?;
            Continue(1)
        }
        0x39 => {
            let dest = ctx.stack.peek(0).map_err(EvmError::from)?.as_usize();
            let offset = ctx.stack.peek(1).map_err(EvmError::from)?.as_usize();
            let size = ctx.stack.peek(2).map_err(EvmError::from)?.as_usize();
            let mem_cost = memory_expansion_cost_for_range(ctx.memory.msize(), dest, size);
            utils::consume_gas(ctx.gas_remaining, mem_cost)?;
            utils::consume_gas(ctx.gas_remaining, copy_gas_cost(size))?;
            for i in 0..size {
                let byte = if offset + i < ctx.code.len() {
                    ctx.code[offset + i]
                } else {
                    0
                };
                ctx.memory.mstore8(dest + i, byte).map_err(EvmError::from)?;
            }
            ctx.stack.pop().map_err(EvmError::from)?;
            ctx.stack.pop().map_err(EvmError::from)?;
            ctx.stack.pop().map_err(EvmError::from)?;
            Continue(1)
        }
        0x3A => {
            ctx.stack.push(ctx.tx_gas_price).map_err(EvmError::from)?;
            Continue(1)
        }
        0x3B => {
            let addr = utils::u256_to_address(ctx.stack.peek(0).map_err(EvmError::from)?)?;
            let is_warm = access_address(ctx, &addr);
            environment::extcodesize(ctx.stack, ctx.state, is_warm, ctx.gas_remaining)?;
            Continue(1)
        }
        0x3C => {
            // Stack (top=peek(0)): address, dest, code_offset, size
            let addr = utils::u256_to_address(ctx.stack.peek(0).map_err(EvmError::from)?)?;
            let is_warm = access_address(ctx, &addr);
            let dest = ctx.stack.peek(1).map_err(EvmError::from)?.as_usize();
            let size = ctx.stack.peek(3).map_err(EvmError::from)?.as_usize();
            let mem_cost = memory_expansion_cost_for_range(ctx.memory.msize(), dest, size);
            utils::consume_gas(ctx.gas_remaining, mem_cost)?;
            utils::consume_gas(ctx.gas_remaining, copy_gas_cost(size))?;
            environment::extcodecopy(ctx.stack, ctx.state, ctx.memory, is_warm, ctx.gas_remaining)?;
            Continue(1)
        }
        0x3D => {
            ctx.stack
                .push(U256::from_u64(ctx.return_data.len() as u64))
                .map_err(EvmError::from)?;
            Continue(1)
        }
        0x3E => {
            let dest = ctx.stack.peek(0).map_err(EvmError::from)?.as_usize();
            let offset = ctx.stack.peek(1).map_err(EvmError::from)?.as_usize();
            let size = ctx.stack.peek(2).map_err(EvmError::from)?.as_usize();
            let end = offset
                .checked_add(size)
                .ok_or(EvmError::MemoryError(MemoryError::InvalidOffset))?;
            if end > ctx.return_data.len() {
                return Err(EvmError::MemoryError(MemoryError::InvalidOffset));
            }
            let mem_cost = memory_expansion_cost_for_range(ctx.memory.msize(), dest, size);
            utils::consume_gas(ctx.gas_remaining, mem_cost)?;
            utils::consume_gas(ctx.gas_remaining, copy_gas_cost(size))?;
            for i in 0..size {
                ctx.memory
                    .mstore8(dest + i, ctx.return_data[offset + i])
                    .map_err(EvmError::from)?;
            }
            ctx.stack.pop().map_err(EvmError::from)?;
            ctx.stack.pop().map_err(EvmError::from)?;
            ctx.stack.pop().map_err(EvmError::from)?;
            Continue(1)
        }
        0x3F => {
            let addr = utils::u256_to_address(ctx.stack.peek(0).map_err(EvmError::from)?)?;
            let is_warm = access_address(ctx, &addr);
            environment::extcodehash(ctx.stack, ctx.state, is_warm, ctx.gas_remaining)?;
            Continue(1)
        }

        // 0x40-0x4A: Block
        0x40 => {
            environment::blockhash_with_host(ctx.stack, ctx.host).map_err(opcode_error_to_evm)?;
            Continue(1)
        }
        0x41 => {
            let bc = block_ctx(ctx);
            environment::coinbase(ctx.stack, &bc).map_err(opcode_error_to_evm)?;
            Continue(1)
        }
        0x42 => {
            ctx.stack
                .push(ctx.block_timestamp)
                .map_err(EvmError::from)?;
            Continue(1)
        }
        0x43 => {
            ctx.stack.push(ctx.block_number).map_err(EvmError::from)?;
            Continue(1)
        }
        0x44 => {
            ctx.stack
                .push(ctx.block_difficulty)
                .map_err(EvmError::from)?;
            Continue(1)
        }
        0x45 => {
            ctx.stack
                .push(ctx.block_gas_limit)
                .map_err(EvmError::from)?;
            Continue(1)
        }
        0x46 => {
            ctx.stack.push(ctx.block_chain_id).map_err(EvmError::from)?;
            Continue(1)
        }
        0x47 => {
            environment::selfbalance(ctx.stack, ctx.state, ctx.call_address)?;
            Continue(1)
        }
        0x48 => {
            ctx.stack.push(ctx.block_base_fee).map_err(EvmError::from)?;
            Continue(1)
        }
        0x49 => {
            environment::blobhash_with_host(ctx.stack, ctx.host).map_err(opcode_error_to_evm)?;
            Continue(1)
        }
        0x4A => {
            environment::blobbasefee_with_host(ctx.stack, ctx.host).map_err(opcode_error_to_evm)?;
            Continue(1)
        }

        // 0x50-0x5F: Stack, memory, storage, flow
        0x50 => {
            control::pop(ctx.stack).map_err(opcode_error_to_evm)?;
            Continue(1)
        }
        0x51 => {
            let offset = ctx.stack.peek(0).map_err(EvmError::from)?.as_usize();
            let mem_cost = memory_expansion_cost(ctx.memory.msize(), offset.saturating_add(32));
            utils::consume_gas(ctx.gas_remaining, mem_cost)?;
            control::mload(ctx.stack, ctx.memory).map_err(opcode_error_to_evm)?;
            Continue(1)
        }
        0x52 => {
            let offset = ctx.stack.peek(1).map_err(EvmError::from)?.as_usize();
            let mem_cost = memory_expansion_cost(ctx.memory.msize(), offset.saturating_add(32));
            utils::consume_gas(ctx.gas_remaining, mem_cost)?;
            control::mstore(ctx.stack, ctx.memory).map_err(opcode_error_to_evm)?;
            Continue(1)
        }
        0x53 => {
            let offset = ctx.stack.peek(1).map_err(EvmError::from)?.as_usize();
            let mem_cost = memory_expansion_cost(ctx.memory.msize(), offset.saturating_add(1));
            utils::consume_gas(ctx.gas_remaining, mem_cost)?;
            control::mstore8(ctx.stack, ctx.memory).map_err(opcode_error_to_evm)?;
            Continue(1)
        }
        0x54 => {
            let key = *ctx.stack.peek(0).map_err(EvmError::from)?;
            let address = ctx.call_address;
            let is_warm = access_storage(ctx, &address, &key);
            control::sload_eip2929(ctx.stack, ctx.state, address, is_warm, ctx.gas_remaining)?;
            Continue(1)
        }
        0x55 => {
            // Stack (top=0): key, value — SSTORE pops key first, then value
            let key = *ctx.stack.peek(0).map_err(EvmError::from)?;
            let new_value = *ctx.stack.peek(1).map_err(EvmError::from)?;
            let address = ctx.call_address;
            let is_warm = access_storage(ctx, &address, &key);
            if *ctx.gas_remaining <= GAS_SSTORE_SENTRY {
                return Err(EvmError::OutOfGas);
            }
            control::sstore_eip2929(
                ctx.stack,
                ctx.state,
                address,
                is_warm,
                ctx.gas_remaining,
                ctx.gas_refund,
            )?;
            return Ok((Continue(1), Some((address, key, new_value))));
        }
        0x56 => {
            let new_pc = control::jump_bitmap(ctx.stack, ctx.code, ctx.jumpdests)
                .map_err(opcode_error_to_evm)?;
            return Ok((Jump(new_pc), None));
        }
        0x57 => {
            let new_pc = control::jumpi_bitmap(ctx.stack, ctx.code, ctx.jumpdests, ctx.pc)
                .map_err(opcode_error_to_evm)?;
            return Ok((Jump(new_pc), None));
        }
        0x58 => {
            control::pc(ctx.stack, ctx.pc).map_err(opcode_error_to_evm)?;
            Continue(1)
        }
        0x59 => {
            control::msize(ctx.stack, ctx.memory).map_err(opcode_error_to_evm)?;
            Continue(1)
        }
        0x5A => {
            control::gas(ctx.stack, *ctx.gas_remaining).map_err(opcode_error_to_evm)?;
            Continue(1)
        }
        0x5B => {
            control::jumpdest().map_err(opcode_error_to_evm)?;
            Continue(1)
        }
        0x5C => {
            control::tload(ctx.stack, ctx.state, ctx.call_address)?;
            Continue(1)
        }
        0x5D => {
            control::tstore(ctx.stack, ctx.state, ctx.call_address)?;
            Continue(1)
        }
        0x5E => {
            let dest = ctx.stack.peek(0).map_err(EvmError::from)?.as_usize();
            let src = ctx.stack.peek(1).map_err(EvmError::from)?.as_usize();
            let size = ctx.stack.peek(2).map_err(EvmError::from)?.as_usize();
            let max_offset = memory_range_end(dest.max(src), size);
            let mem_cost = memory_expansion_cost(ctx.memory.msize(), max_offset);
            utils::consume_gas(ctx.gas_remaining, mem_cost)?;
            utils::consume_gas(ctx.gas_remaining, copy_gas_cost(size))?;
            control::mcopy(ctx.stack, ctx.memory).map_err(opcode_error_to_evm)?;
            Continue(1)
        }
        0x5F => {
            control::push0(ctx.stack).map_err(opcode_error_to_evm)?;
            Continue(1)
        }

        // 0x60-0x7F: PUSH1-PUSH32
        0x60..=0x7F => {
            let n = (opcode - 0x5F) as usize;
            control::push_n_strict(ctx.stack, ctx.code, ctx.pc, n).map_err(opcode_error_to_evm)?;
            Continue(1 + n as u32)
        }

        // 0x80-0x8F: DUP1-DUP16
        0x80..=0x8F => {
            let n = (opcode - 0x7F) as usize;
            control::dup_n(ctx.stack, n).map_err(opcode_error_to_evm)?;
            Continue(1)
        }

        // 0x90-0x9F: SWAP1-SWAP16
        0x90..=0x9F => {
            let n = (opcode - 0x8F) as usize;
            control::swap_n(ctx.stack, n).map_err(opcode_error_to_evm)?;
            Continue(1)
        }

        // 0xA0-0xA4: LOG0-LOG4
        0xA0..=0xA4 => {
            let num_topics = opcode - 0xA0;
            let (topics, data) = log::execute_log(
                ctx.stack,
                ctx.memory,
                num_topics,
                ctx.call_address,
                ctx.gas_remaining,
            )?;
            return Ok((Log(ctx.call_address, topics, data), None));
        }

        // 0xF0: CREATE — stack (top=0): size, offset, value
        0xF0 => {
            let offset = ctx.stack.peek(1).map_err(EvmError::from)?.as_usize();
            let size = ctx.stack.peek(0).map_err(EvmError::from)?.as_usize();
            let mem_cost = memory_expansion_cost_for_range(ctx.memory.msize(), offset, size);
            utils::consume_gas(ctx.gas_remaining, mem_cost)?;
            utils::consume_gas(ctx.gas_remaining, init_code_gas_cost(size))?;
            let nonce = ctx.state.get_nonce(&ctx.call_address);
            let contract_address =
                compute_create_address(&ctx.call_address, nonce.saturating_sub(U256::ONE));
            access_address(ctx, &contract_address);
            environment::execute_create(
                ctx.state,
                ctx.host,
                ctx.stack,
                ctx.memory,
                &call_env(ctx),
                ctx.gas_remaining,
                ctx.return_data,
            )?;
            Continue(1)
        }

        // 0xF1: CALL — stack (top=0): out_size, out_offset, in_size, in_offset, value, to, gas
        0xF1 => {
            let to = utils::u256_to_address(ctx.stack.peek(5).map_err(EvmError::from)?)?;
            let is_warm = access_address(ctx, &to);
            let in_offset = ctx.stack.peek(3).map_err(EvmError::from)?.as_usize();
            let in_size = ctx.stack.peek(2).map_err(EvmError::from)?.as_usize();
            let out_offset = ctx.stack.peek(1).map_err(EvmError::from)?.as_usize();
            let out_size = ctx.stack.peek(0).map_err(EvmError::from)?.as_usize();
            let max_off =
                memory_range_end(in_offset, in_size).max(memory_range_end(out_offset, out_size));
            let mem_cost = memory_expansion_cost(ctx.memory.msize(), max_off);
            utils::consume_gas(ctx.gas_remaining, mem_cost)?;
            environment::execute_call(
                ctx.state,
                ctx.host,
                ctx.stack,
                ctx.memory,
                &call_env(ctx),
                ctx.gas_remaining,
                ctx.return_data,
                is_warm,
            )?;
            Continue(1)
        }

        // 0xF2: CALLCODE — stack (top=0): out_size, out_offset, in_size, in_offset, value, to, gas
        0xF2 => {
            let to = utils::u256_to_address(ctx.stack.peek(5).map_err(EvmError::from)?)?;
            let is_warm = access_address(ctx, &to);
            let in_offset = ctx.stack.peek(3).map_err(EvmError::from)?.as_usize();
            let in_size = ctx.stack.peek(2).map_err(EvmError::from)?.as_usize();
            let out_offset = ctx.stack.peek(1).map_err(EvmError::from)?.as_usize();
            let out_size = ctx.stack.peek(0).map_err(EvmError::from)?.as_usize();
            let max_off =
                memory_range_end(in_offset, in_size).max(memory_range_end(out_offset, out_size));
            let mem_cost = memory_expansion_cost(ctx.memory.msize(), max_off);
            utils::consume_gas(ctx.gas_remaining, mem_cost)?;
            environment::execute_callcode(
                ctx.state,
                ctx.host,
                ctx.stack,
                ctx.memory,
                &call_env(ctx),
                ctx.gas_remaining,
                ctx.return_data,
                is_warm,
            )?;
            Continue(1)
        }

        // 0xF3: RETURN
        0xF3 => {
            let offset = ctx.stack.pop().map_err(EvmError::from)?.as_usize();
            let size = ctx.stack.pop().map_err(EvmError::from)?.as_usize();
            let mem_cost = memory_expansion_cost_for_range(ctx.memory.msize(), offset, size);
            utils::consume_gas(ctx.gas_remaining, mem_cost)?;
            let data = utils::read_memory_bytes(ctx.memory, offset, size)?;
            return Ok((Return(data), None));
        }

        // 0xF4: DELEGATECALL — stack (top=0): gas, to, in_offset, in_size, out_offset, out_size
        0xF4 => {
            let to = utils::u256_to_address(ctx.stack.peek(1).map_err(EvmError::from)?)?;
            let is_warm = access_address(ctx, &to);
            let in_offset = ctx.stack.peek(2).map_err(EvmError::from)?.as_usize();
            let in_size = ctx.stack.peek(3).map_err(EvmError::from)?.as_usize();
            let out_offset = ctx.stack.peek(4).map_err(EvmError::from)?.as_usize();
            let out_size = ctx.stack.peek(5).map_err(EvmError::from)?.as_usize();
            let max_off =
                memory_range_end(in_offset, in_size).max(memory_range_end(out_offset, out_size));
            let mem_cost = memory_expansion_cost(ctx.memory.msize(), max_off);
            utils::consume_gas(ctx.gas_remaining, mem_cost)?;
            environment::execute_delegatecall(
                ctx.state,
                ctx.host,
                ctx.stack,
                ctx.memory,
                &call_env(ctx),
                ctx.gas_remaining,
                ctx.return_data,
                is_warm,
            )?;
            Continue(1)
        }

        // 0xF5: CREATE2 — stack (top=0): salt, size, offset, value
        0xF5 => {
            let offset = ctx.stack.peek(2).map_err(EvmError::from)?.as_usize();
            let size = ctx.stack.peek(1).map_err(EvmError::from)?.as_usize();
            let mem_cost = memory_expansion_cost_for_range(ctx.memory.msize(), offset, size);
            utils::consume_gas(ctx.gas_remaining, mem_cost)?;
            utils::consume_gas(
                ctx.gas_remaining,
                init_code_gas_cost(size) + create2_hash_cost(size),
            )?;
            let init_code = utils::read_memory_bytes(ctx.memory, offset, size)?;
            let salt = *ctx.stack.peek(0).map_err(EvmError::from)?;
            let contract_address = compute_create2_address(&ctx.call_address, &salt, &init_code);
            access_address(ctx, &contract_address);
            environment::execute_create2(
                ctx.state,
                ctx.host,
                ctx.stack,
                ctx.memory,
                &call_env(ctx),
                ctx.gas_remaining,
                ctx.return_data,
            )?;
            Continue(1)
        }

        // 0xFA: STATICCALL — stack (top=0): gas, to, in_offset, in_size, out_offset, out_size
        0xFA => {
            let to = utils::u256_to_address(ctx.stack.peek(1).map_err(EvmError::from)?)?;
            let is_warm = access_address(ctx, &to);
            let in_offset = ctx.stack.peek(2).map_err(EvmError::from)?.as_usize();
            let in_size = ctx.stack.peek(3).map_err(EvmError::from)?.as_usize();
            let out_offset = ctx.stack.peek(4).map_err(EvmError::from)?.as_usize();
            let out_size = ctx.stack.peek(5).map_err(EvmError::from)?.as_usize();
            let max_off =
                memory_range_end(in_offset, in_size).max(memory_range_end(out_offset, out_size));
            let mem_cost = memory_expansion_cost(ctx.memory.msize(), max_off);
            utils::consume_gas(ctx.gas_remaining, mem_cost)?;
            environment::execute_staticcall(
                ctx.state,
                ctx.host,
                ctx.stack,
                ctx.memory,
                &call_env(ctx),
                ctx.gas_remaining,
                ctx.return_data,
                is_warm,
            )?;
            Continue(1)
        }

        // 0xFD: REVERT
        0xFD => {
            let offset = ctx.stack.pop().map_err(EvmError::from)?.as_usize();
            let size = ctx.stack.pop().map_err(EvmError::from)?.as_usize();
            let mem_cost = memory_expansion_cost_for_range(ctx.memory.msize(), offset, size);
            utils::consume_gas(ctx.gas_remaining, mem_cost)?;
            let data = utils::read_memory_bytes(ctx.memory, offset, size)?;
            return Ok((Revert(data), None));
        }

        // 0xFE: INVALID
        0xFE => return Err(EvmError::InvalidOpcode(opcode)),

        // 0xFF: SELFDESTRUCT
        0xFF => {
            environment::execute_selfdestruct(ctx.state, ctx.stack, ctx.call_address)?;
            return Ok((Stop, None));
        }

        _ => return Err(EvmError::InvalidOpcode(opcode)),
    };

    Ok((outcome, None))
}
