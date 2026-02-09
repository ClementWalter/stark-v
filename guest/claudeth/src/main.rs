#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::io::{self, Read, Write};
#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::{vec, vec::Vec};

use claudeth::crypto::rlp;
use claudeth::crypto::rlp::RlpError;
use claudeth::state::{InMemoryState, State};
use claudeth::stf::{BlockProcessingError, ExecutionError, process_block};
use claudeth::types::{Address, BlockHeader, Hash, Transaction, U256};

const ERROR_INVALID_HEADER: u64 = 1;
const ERROR_TX_EXECUTION: u64 = 2;
const ERROR_GAS_LIMIT_EXCEEDED: u64 = 3;
const ERROR_RECEIPTS_ROOT_MISMATCH: u64 = 4;
const ERROR_STATE_ROOT_MISMATCH: u64 = 5;
const ERROR_GAS_USED_MISMATCH: u64 = 6;
const ERROR_TRANSACTIONS_ROOT_MISMATCH: u64 = 7;
const ERROR_LOGS_BLOOM_MISMATCH: u64 = 8;
const ERROR_RLP_DECODE: u64 = 100;
const ERROR_INVALID_INPUT: u64 = 101;

// Input format:
// RLP([
//   block_header_rlp,
//   parent_header_rlp,
//   chain_id_u256,
//   transactions_rlp_list,
//   state_entries_rlp_list,
//   recent_block_hashes_rlp_list?   // optional (6th item)
// ])
//
// recent_block_hashes_rlp_list format (RLP list):
// [ [block_number_u256, block_hash], ... ]  // up to 256 entries
//
// State entry format (RLP list):
// [address, nonce, balance, code_bytes, storage_entries]
//
// Storage entry format (RLP list):
// [key_u256, value_u256]
//
// Output format:
// RLP([
//   status_u64,          // 1=success, 0=error
//   gas_used_u64,
//   receipts_root_hash,
//   state_root_hash,
//   error_code_u64,      // 0 on success
//   error_data_bytes     // RLP-encoded list of error details
// ])

#[derive(Debug)]
struct StateEntry {
    address: Address,
    nonce: U256,
    balance: U256,
    code: Vec<u8>,
    storage: Vec<(U256, U256)>,
}

#[cfg(target_arch = "riscv32")]
#[unsafe(no_mangle)]
pub extern "C" fn __zkvm_start() -> ! {
    let input = unsafe { zkvm_io::read_all_input() };
    let output = process_input(&input);
    unsafe {
        zkvm_io::write_output(&output);
        zkvm_io::halt();
    }
}

#[cfg(not(target_arch = "riscv32"))]
fn main() {
    let mut input = Vec::new();
    io::stdin()
        .read_to_end(&mut input)
        .expect("failed to read stdin");
    let output = process_input(&input);
    io::stdout()
        .write_all(&output)
        .expect("failed to write stdout");
}

fn process_input(input: &[u8]) -> Vec<u8> {
    match decode_and_execute(input) {
        Ok(result) => encode_success(result.gas_used, result.receipts_root, result.state_root),
        Err(err) => encode_error(err),
    }
}

fn decode_and_execute(input: &[u8]) -> Result<claudeth::stf::BlockProcessingResult, GuestError> {
    let (items, rest) = rlp::decode_list(input).map_err(GuestError::Rlp)?;
    if !rest.is_empty() || !(items.len() == 5 || items.len() == 6) {
        return Err(GuestError::InvalidInput);
    }

    let block = BlockHeader::decode_rlp(&items[0]).map_err(GuestError::Rlp)?;
    let parent = BlockHeader::decode_rlp(&items[1]).map_err(GuestError::Rlp)?;
    let (chain_id, _) = rlp::decode_u256(&items[2]).map_err(GuestError::Rlp)?;

    let transactions = decode_transactions(&items[3])?;
    let state_entries = decode_state_entries(&items[4])?;

    let recent_block_hashes = if items.len() == 6 {
        decode_recent_block_hashes(&items[5])?
    } else {
        Vec::new()
    };

    let mut state = InMemoryState::new();
    apply_state_entries(&mut state, &state_entries);

    process_block(
        &block,
        &parent,
        &transactions,
        &mut state,
        chain_id,
        &recent_block_hashes,
    )
    .map_err(GuestError::Block)
}

fn decode_transactions(input: &[u8]) -> Result<Vec<Transaction>, GuestError> {
    let (items, rest) = rlp::decode_list(input).map_err(GuestError::Rlp)?;
    if !rest.is_empty() {
        return Err(GuestError::InvalidInput);
    }
    let mut transactions = Vec::with_capacity(items.len());
    for item in items {
        let tx = Transaction::decode_rlp(&item).map_err(GuestError::Rlp)?;
        transactions.push(tx);
    }
    Ok(transactions)
}

fn decode_state_entries(input: &[u8]) -> Result<Vec<StateEntry>, GuestError> {
    let (items, rest) = rlp::decode_list(input).map_err(GuestError::Rlp)?;
    if !rest.is_empty() {
        return Err(GuestError::InvalidInput);
    }

    let mut entries = Vec::with_capacity(items.len());
    for item in items {
        let (fields, rest) = rlp::decode_list(&item).map_err(GuestError::Rlp)?;
        if !rest.is_empty() || fields.len() != 5 {
            return Err(GuestError::InvalidInput);
        }

        let (address, _) = rlp::decode_address(&fields[0]).map_err(GuestError::Rlp)?;
        let (nonce, _) = rlp::decode_u256(&fields[1]).map_err(GuestError::Rlp)?;
        let (balance, _) = rlp::decode_u256(&fields[2]).map_err(GuestError::Rlp)?;
        let (code, _) = rlp::decode_bytes(&fields[3]).map_err(GuestError::Rlp)?;
        let storage = decode_storage_entries(&fields[4])?;

        entries.push(StateEntry {
            address,
            nonce,
            balance,
            code,
            storage,
        });
    }

    Ok(entries)
}

fn decode_storage_entries(input: &[u8]) -> Result<Vec<(U256, U256)>, GuestError> {
    let (items, rest) = rlp::decode_list(input).map_err(GuestError::Rlp)?;
    if !rest.is_empty() {
        return Err(GuestError::InvalidInput);
    }

    let mut entries = Vec::with_capacity(items.len());
    for item in items {
        let (fields, rest) = rlp::decode_list(&item).map_err(GuestError::Rlp)?;
        if !rest.is_empty() || fields.len() != 2 {
            return Err(GuestError::InvalidInput);
        }
        let (key, _) = rlp::decode_u256(&fields[0]).map_err(GuestError::Rlp)?;
        let (value, _) = rlp::decode_u256(&fields[1]).map_err(GuestError::Rlp)?;
        entries.push((key, value));
    }

    Ok(entries)
}

fn decode_recent_block_hashes(input: &[u8]) -> Result<Vec<(u64, Hash)>, GuestError> {
    let (items, rest) = rlp::decode_list(input).map_err(GuestError::Rlp)?;
    if !rest.is_empty() {
        return Err(GuestError::InvalidInput);
    }
    if items.len() > 256 {
        return Err(GuestError::InvalidInput);
    }

    let mut entries = Vec::with_capacity(items.len());
    for item in items {
        let (fields, rest) = rlp::decode_list(&item).map_err(GuestError::Rlp)?;
        if !rest.is_empty() || fields.len() != 2 {
            return Err(GuestError::InvalidInput);
        }

        let (number, _) = rlp::decode_u256(&fields[0]).map_err(GuestError::Rlp)?;
        let (hash, _) = rlp::decode_hash(&fields[1]).map_err(GuestError::Rlp)?;
        let number = u64::try_from(number).map_err(|_| GuestError::InvalidInput)?;
        entries.push((number, hash));
    }

    Ok(entries)
}

fn apply_state_entries(state: &mut InMemoryState, entries: &[StateEntry]) {
    for entry in entries {
        state.set_balance(&entry.address, entry.balance);
        state.set_nonce(&entry.address, entry.nonce);
        state.set_code(&entry.address, entry.code.clone());
        for (key, value) in &entry.storage {
            state.sstore(&entry.address, key, *value);
        }
    }
}

fn encode_success(gas_used: u64, receipts_root: Hash, state_root: Hash) -> Vec<u8> {
    encode_output(
        1,
        gas_used,
        receipts_root,
        state_root,
        0,
        rlp::encode_list(&[]),
    )
}

fn encode_error(err: GuestError) -> Vec<u8> {
    let (error_code, error_data) = match err {
        GuestError::Block(block_err) => encode_block_error(block_err),
        GuestError::Rlp(rlp_err) => (
            ERROR_RLP_DECODE,
            rlp::encode_list(&[encode_u64(error_kind(rlp_err))]),
        ),
        GuestError::InvalidInput => (ERROR_INVALID_INPUT, rlp::encode_list(&[])),
    };

    encode_output(0, 0, Hash::ZERO, Hash::ZERO, error_code, error_data)
}

fn encode_block_error(err: BlockProcessingError) -> (u64, Vec<u8>) {
    match err {
        BlockProcessingError::InvalidHeader(_msg) => (ERROR_INVALID_HEADER, rlp::encode_list(&[])),
        BlockProcessingError::TransactionExecutionError(exec_err) => {
            let detail = match exec_err {
                ExecutionError::ValidationError(_) => 1u64,
                ExecutionError::ExecutionFailed => 2u64,
            };
            (ERROR_TX_EXECUTION, rlp::encode_list(&[encode_u64(detail)]))
        }
        BlockProcessingError::GasLimitExceeded {
            gas_limit,
            gas_used,
            ..
        } => (
            ERROR_GAS_LIMIT_EXCEEDED,
            rlp::encode_list(&[encode_u64(gas_limit), encode_u64(gas_used)]),
        ),
        BlockProcessingError::ReceiptsRootMismatch {
            expected, computed, ..
        } => (
            ERROR_RECEIPTS_ROOT_MISMATCH,
            rlp::encode_list(&[encode_hash(expected), encode_hash(computed)]),
        ),
        BlockProcessingError::StateRootMismatch {
            expected, computed, ..
        } => (
            ERROR_STATE_ROOT_MISMATCH,
            rlp::encode_list(&[encode_hash(expected), encode_hash(computed)]),
        ),
        BlockProcessingError::GasUsedMismatch {
            expected, computed, ..
        } => (
            ERROR_GAS_USED_MISMATCH,
            rlp::encode_list(&[encode_u64(expected), encode_u64(computed)]),
        ),
        BlockProcessingError::TransactionsRootMismatch {
            expected, computed, ..
        } => (
            ERROR_TRANSACTIONS_ROOT_MISMATCH,
            rlp::encode_list(&[encode_hash(expected), encode_hash(computed)]),
        ),
        BlockProcessingError::LogsBloomMismatch {
            expected, computed, ..
        } => (
            ERROR_LOGS_BLOOM_MISMATCH,
            rlp::encode_list(&[
                rlp::encode_bytes(expected.as_ref()),
                rlp::encode_bytes(computed.as_ref()),
            ]),
        ),
    }
}

fn encode_output(
    status: u64,
    gas_used: u64,
    receipts_root: Hash,
    state_root: Hash,
    error_code: u64,
    error_data: Vec<u8>,
) -> Vec<u8> {
    let items = vec![
        encode_u64(status),
        encode_u64(gas_used),
        encode_hash(receipts_root),
        encode_hash(state_root),
        encode_u64(error_code),
        rlp::encode_bytes(&error_data),
    ];
    rlp::encode_list(&items)
}

fn encode_u64(value: u64) -> Vec<u8> {
    rlp::encode_u64(value)
}

fn encode_hash(value: Hash) -> Vec<u8> {
    rlp::encode_hash(&value)
}

fn error_kind(err: RlpError) -> u64 {
    match err {
        RlpError::InvalidEncoding => 1,
        RlpError::UnexpectedEnd => 2,
        RlpError::InvalidLength => 3,
        RlpError::InputTooShort => 4,
        RlpError::LeadingZero => 5,
        RlpError::NonCanonical => 6,
    }
}

enum GuestError {
    Block(BlockProcessingError),
    Rlp(RlpError),
    InvalidInput,
}

#[cfg(target_arch = "riscv32")]
mod zkvm_io {
    use super::Vec;

    unsafe extern "C" {
        static __input_start: u8;
        static __input_end: u8;
        static __halt_flag: u8;
        static __output_len: u8;
        static __output_data: u8;
        static __output_end: u8;
    }

    pub unsafe fn read_all_input() -> Vec<u8> {
        // SAFETY: Caller ensures __input_start and __input_end are valid memory regions
        unsafe {
            let start = core::ptr::addr_of!(__input_start) as usize;
            let end = core::ptr::addr_of!(__input_end) as usize;
            let input_size = end.saturating_sub(start);
            let mut buf = Vec::with_capacity(input_size);
            for i in 0..input_size {
                let addr = start + i;
                let byte = core::ptr::read_volatile(addr as *const u8);
                buf.push(byte);
            }
            buf
        }
    }

    pub unsafe fn write_output(data: &[u8]) {
        // SAFETY: Caller ensures __output_* symbols are valid memory regions
        unsafe {
            let data_start = core::ptr::addr_of!(__output_data) as usize;
            let data_end = core::ptr::addr_of!(__output_end) as usize;
            let max_size = data_end.saturating_sub(data_start);
            let len = data.len().min(max_size);

            let len_addr = core::ptr::addr_of!(__output_len) as *mut u32;
            core::ptr::write_volatile(len_addr, len as u32);

            for (i, byte) in data.iter().take(len).enumerate() {
                let addr = data_start + i;
                core::ptr::write_volatile(addr as *mut u8, *byte);
            }
        }
    }

    pub unsafe fn halt() -> ! {
        // SAFETY: Caller ensures __halt_flag is a valid memory region
        unsafe {
            let halt_addr = core::ptr::addr_of!(__halt_flag) as *mut u32;
            core::ptr::write_volatile(halt_addr, 1);
        }
        #[allow(clippy::empty_loop)]
        loop {}
    }
}
