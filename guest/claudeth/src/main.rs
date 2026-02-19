#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

#[cfg(target_arch = "riscv32")]
extern crate alloc;
#[cfg(target_arch = "riscv32")]
use core::arch::global_asm;

#[cfg(not(target_arch = "riscv32"))]
use std::io::{self, Read, Write};
#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::{vec, vec::Vec};

use claudeth::crypto::rlp::RlpError;
use claudeth::crypto::{keccak256, rlp};
use claudeth::state::{Account, EMPTY_CODE_HASH, InMemoryState, Proof, State, verify_proof};
use claudeth::stf::{BlockProcessingError, ExecutionError, process_block};
use claudeth::types::{Address, BlockHeader, Hash, Transaction, U256, Withdrawal};

#[cfg(target_arch = "riscv32")]
// Why: the runner executes the ELF entrypoint directly; a tiny `_start`
// trampoline sets gp/sp then transfers control to `__zkvm_start`.
global_asm!(
    r#"
    .section .text._start
    .globl _start
_start:
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop

    la sp, __stack_top
    call __zkvm_start
"#
);

const ERROR_INVALID_HEADER: u64 = 1;
const ERROR_TX_EXECUTION: u64 = 2;
const ERROR_GAS_LIMIT_EXCEEDED: u64 = 3;
const ERROR_RECEIPTS_ROOT_MISMATCH: u64 = 4;
const ERROR_STATE_ROOT_MISMATCH: u64 = 5;
const ERROR_GAS_USED_MISMATCH: u64 = 6;
const ERROR_TRANSACTIONS_ROOT_MISMATCH: u64 = 7;
const ERROR_LOGS_BLOOM_MISMATCH: u64 = 8;
const ERROR_UNEXPECTED_WITHDRAWALS: u64 = 9;
const ERROR_WITHDRAWALS_ROOT_MISMATCH: u64 = 10;
const ERROR_BLOB_GAS_LIMIT_EXCEEDED: u64 = 11;
const ERROR_BLOB_GAS_USED_MISMATCH: u64 = 12;
const ERROR_RLP_DECODE: u64 = 100;
const ERROR_INVALID_INPUT: u64 = 101;

// Input format:
// RLP([
//   block_header_rlp,
//   parent_header_rlp,
//   chain_id_u256,
//   transactions_rlp_list,
//   state_entries_rlp_list or witness_rlp_list (WITNESS v1),
//   block_hashes_rlp_list (optional),
//   withdrawals_rlp_list (optional, required if withdrawals_root is present)
// ])
//
// State entry format (RLP list):
// [address, nonce, balance, code_bytes, storage_entries]
//
// Storage entry format (RLP list):
// [key_u256, value_u256]
//
// Witness format: see WITNESS.md (WitnessV1)
//
// Withdrawal format (RLP list):
// [index_u64, validator_index_u64, address, amount_gwei_u64]
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

#[derive(Debug)]
struct WitnessState {
    state_root: Hash,
    accounts: Vec<WitnessAccount>,
}

#[derive(Debug)]
struct WitnessAccount {
    address: Address,
    account_proof: Proof,
    account_rlp: Vec<u8>,
    code: Vec<u8>,
    storage_entries: Vec<WitnessStorageEntry>,
}

#[derive(Debug)]
struct WitnessStorageEntry {
    key: U256,
    value: U256,
    proof: Proof,
}

enum StateSource {
    Entries(Vec<StateEntry>),
    Witness(WitnessState),
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
    if !rest.is_empty() || !(5..=7).contains(&items.len()) {
        return Err(GuestError::InvalidInput);
    }

    let block = BlockHeader::decode_rlp(&items[0]).map_err(GuestError::Rlp)?;
    let parent = BlockHeader::decode_rlp(&items[1]).map_err(GuestError::Rlp)?;
    let (chain_id, _) = rlp::decode_u256(&items[2]).map_err(GuestError::Rlp)?;

    let transactions = decode_transactions(&items[3])?;
    let state_source = decode_state_source(&items[4])?;
    let (block_hashes, withdrawals, has_withdrawals_list) = match items.len() {
        5 => (Vec::new(), Vec::new(), false),
        6 => {
            if block.withdrawals_root.is_some() {
                (Vec::new(), decode_withdrawals(&items[5])?, true)
            } else {
                (decode_block_hashes(&items[5])?, Vec::new(), false)
            }
        }
        7 => (
            decode_block_hashes(&items[5])?,
            decode_withdrawals(&items[6])?,
            true,
        ),
        _ => unreachable!("length checked above"),
    };

    validate_withdrawals_presence(&block, has_withdrawals_list)?;
    validate_block_hashes(&block, &parent, &block_hashes)?;

    let mut state = InMemoryState::new();
    match state_source {
        StateSource::Entries(entries) => apply_state_entries(&mut state, &entries),
        StateSource::Witness(witness) => apply_witness_state(&mut state, witness)?,
    }

    process_block(
        &block,
        &parent,
        &transactions,
        &withdrawals,
        &block_hashes,
        &mut state,
        chain_id,
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

fn decode_state_source(input: &[u8]) -> Result<StateSource, GuestError> {
    let (items, rest) = rlp::decode_list(input).map_err(GuestError::Rlp)?;
    if !rest.is_empty() {
        return Err(GuestError::InvalidInput);
    }

    if items.len() == 3
        && let Ok((version, rest_version)) = rlp::decode_u64(&items[0])
        && rest_version.is_empty()
    {
        let witness = decode_witness_items(version, &items)?;
        return Ok(StateSource::Witness(witness));
    }

    Ok(StateSource::Entries(decode_state_entries_items(&items)?))
}

fn decode_state_entries_items(items: &[Vec<u8>]) -> Result<Vec<StateEntry>, GuestError> {
    let mut entries = Vec::with_capacity(items.len());
    for item in items {
        let (fields, rest) = rlp::decode_list(item).map_err(GuestError::Rlp)?;
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

fn decode_witness_items(version: u64, items: &[Vec<u8>]) -> Result<WitnessState, GuestError> {
    if items.len() != 3 {
        return Err(GuestError::InvalidInput);
    }

    if version != 1 {
        return Err(GuestError::InvalidInput);
    }

    let (state_root, rest) = rlp::decode_hash(&items[1]).map_err(GuestError::Rlp)?;
    if !rest.is_empty() {
        return Err(GuestError::InvalidInput);
    }

    let accounts = decode_witness_accounts(&items[2])?;
    Ok(WitnessState {
        state_root,
        accounts,
    })
}

fn decode_witness_accounts(input: &[u8]) -> Result<Vec<WitnessAccount>, GuestError> {
    let (items, rest) = rlp::decode_list(input).map_err(GuestError::Rlp)?;
    if !rest.is_empty() {
        return Err(GuestError::InvalidInput);
    }

    let mut accounts = Vec::with_capacity(items.len());
    for item in items {
        let (fields, rest) = rlp::decode_list(&item).map_err(GuestError::Rlp)?;
        if !rest.is_empty() || fields.len() != 5 {
            return Err(GuestError::InvalidInput);
        }

        let (address, rest) = rlp::decode_address(&fields[0]).map_err(GuestError::Rlp)?;
        if !rest.is_empty() {
            return Err(GuestError::InvalidInput);
        }

        let account_proof = decode_witness_proof_nodes(&fields[1])?;
        let (account_rlp, rest) = rlp::decode_bytes(&fields[2]).map_err(GuestError::Rlp)?;
        if !rest.is_empty() {
            return Err(GuestError::InvalidInput);
        }

        let (code, rest) = rlp::decode_bytes(&fields[3]).map_err(GuestError::Rlp)?;
        if !rest.is_empty() {
            return Err(GuestError::InvalidInput);
        }

        let storage_entries = decode_witness_storage_entries(&fields[4])?;

        accounts.push(WitnessAccount {
            address,
            account_proof,
            account_rlp,
            code,
            storage_entries,
        });
    }

    Ok(accounts)
}

fn decode_witness_storage_entries(input: &[u8]) -> Result<Vec<WitnessStorageEntry>, GuestError> {
    let (items, rest) = rlp::decode_list(input).map_err(GuestError::Rlp)?;
    if !rest.is_empty() {
        return Err(GuestError::InvalidInput);
    }

    let mut entries = Vec::with_capacity(items.len());
    for item in items {
        let (fields, rest) = rlp::decode_list(&item).map_err(GuestError::Rlp)?;
        if !rest.is_empty() || fields.len() != 3 {
            return Err(GuestError::InvalidInput);
        }

        let (key, rest) = rlp::decode_u256(&fields[0]).map_err(GuestError::Rlp)?;
        if !rest.is_empty() {
            return Err(GuestError::InvalidInput);
        }

        let (value, rest) = rlp::decode_u256(&fields[1]).map_err(GuestError::Rlp)?;
        if !rest.is_empty() {
            return Err(GuestError::InvalidInput);
        }

        let proof = decode_witness_proof_nodes(&fields[2])?;

        entries.push(WitnessStorageEntry { key, value, proof });
    }

    Ok(entries)
}

fn decode_witness_proof_nodes(input: &[u8]) -> Result<Proof, GuestError> {
    let (items, rest) = rlp::decode_list(input).map_err(GuestError::Rlp)?;
    if !rest.is_empty() {
        return Err(GuestError::InvalidInput);
    }

    let mut nodes = Vec::with_capacity(items.len());
    for item in items {
        let (node, rest) = rlp::decode_bytes(&item).map_err(GuestError::Rlp)?;
        if !rest.is_empty() {
            return Err(GuestError::InvalidInput);
        }
        nodes.push(node);
    }

    Ok(Proof::from_nodes(nodes))
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

fn decode_withdrawals(input: &[u8]) -> Result<Vec<Withdrawal>, GuestError> {
    let (items, rest) = rlp::decode_list(input).map_err(GuestError::Rlp)?;
    if !rest.is_empty() {
        return Err(GuestError::InvalidInput);
    }

    let mut withdrawals = Vec::with_capacity(items.len());
    for item in items {
        let withdrawal = Withdrawal::decode_rlp(&item).map_err(GuestError::Rlp)?;
        withdrawals.push(withdrawal);
    }
    Ok(withdrawals)
}

fn decode_block_hashes(input: &[u8]) -> Result<Vec<Hash>, GuestError> {
    let (items, rest) = rlp::decode_list(input).map_err(GuestError::Rlp)?;
    if !rest.is_empty() {
        return Err(GuestError::InvalidInput);
    }

    let mut hashes = Vec::with_capacity(items.len());
    for item in items {
        let (hash, rest) = rlp::decode_hash(&item).map_err(GuestError::Rlp)?;
        if !rest.is_empty() {
            return Err(GuestError::InvalidInput);
        }
        hashes.push(hash);
    }

    Ok(hashes)
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

fn apply_witness_state(state: &mut InMemoryState, witness: WitnessState) -> Result<(), GuestError> {
    validate_witness_account_order(&witness.accounts)?;

    for account in witness.accounts {
        let account_key = keccak256(account.address.as_bytes());

        if account.account_rlp.is_empty() {
            if !account.code.is_empty() || !account.storage_entries.is_empty() {
                return Err(GuestError::InvalidInput);
            }

            if !verify_proof(
                witness.state_root,
                account_key.as_bytes(),
                None,
                &account.account_proof,
            ) {
                return Err(GuestError::InvalidInput);
            }
            continue;
        }

        if !verify_proof(
            witness.state_root,
            account_key.as_bytes(),
            Some(&account.account_rlp),
            &account.account_proof,
        ) {
            return Err(GuestError::InvalidInput);
        }

        let decoded_account = Account::decode_rlp(&account.account_rlp).map_err(GuestError::Rlp)?;
        let computed_code_hash = if account.code.is_empty() {
            EMPTY_CODE_HASH
        } else {
            keccak256(&account.code)
        };

        if computed_code_hash != decoded_account.code_hash {
            return Err(GuestError::InvalidInput);
        }

        if decoded_account.code_hash == EMPTY_CODE_HASH && !account.code.is_empty() {
            return Err(GuestError::InvalidInput);
        }

        validate_witness_storage_order(&account.storage_entries)?;
        for entry in &account.storage_entries {
            let storage_key = keccak256(&entry.key.to_be_bytes());
            let expected_value = if entry.value.is_zero() {
                None
            } else {
                Some(rlp::encode_u256(&entry.value))
            };

            let expected_value_ref = expected_value.as_deref();
            if !verify_proof(
                decoded_account.storage_root,
                storage_key.as_bytes(),
                expected_value_ref,
                &entry.proof,
            ) {
                return Err(GuestError::InvalidInput);
            }
        }

        state.set_balance(&account.address, decoded_account.balance);
        state.set_nonce(&account.address, decoded_account.nonce);
        state.set_code(&account.address, account.code.clone());

        for entry in &account.storage_entries {
            if !entry.value.is_zero() {
                state.sstore(&account.address, &entry.key, entry.value);
            }
        }
    }

    Ok(())
}

fn validate_witness_account_order(accounts: &[WitnessAccount]) -> Result<(), GuestError> {
    let mut last: Option<Address> = None;
    for account in accounts {
        if let Some(prev) = last
            && account.address <= prev
        {
            return Err(GuestError::InvalidInput);
        }
        last = Some(account.address);
    }
    Ok(())
}

fn validate_witness_storage_order(entries: &[WitnessStorageEntry]) -> Result<(), GuestError> {
    let mut last: Option<U256> = None;
    for entry in entries {
        if let Some(prev) = last
            && entry.key <= prev
        {
            return Err(GuestError::InvalidInput);
        }
        last = Some(entry.key);
    }
    Ok(())
}

fn validate_withdrawals_presence(
    block: &BlockHeader,
    has_withdrawals_list: bool,
) -> Result<(), GuestError> {
    if block.withdrawals_root.is_some() && !has_withdrawals_list {
        return Err(GuestError::InvalidInput);
    }

    if block.withdrawals_root.is_none() && has_withdrawals_list {
        return Err(GuestError::InvalidInput);
    }

    Ok(())
}

fn validate_block_hashes(
    block: &BlockHeader,
    parent: &BlockHeader,
    block_hashes: &[Hash],
) -> Result<(), GuestError> {
    if block_hashes.is_empty() {
        return Ok(());
    }

    if block.number == 0 {
        return Err(GuestError::InvalidInput);
    }

    let max_hashes = core::cmp::min(block.number as usize, 256);
    if block_hashes.len() > max_hashes {
        return Err(GuestError::InvalidInput);
    }

    let parent_hash = parent.compute_hash();
    if block_hashes.last().copied() != Some(parent_hash) {
        return Err(GuestError::InvalidInput);
    }

    Ok(())
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
        BlockProcessingError::BlobGasLimitExceeded {
            blob_gas_limit,
            blob_gas_used,
            ..
        } => (
            ERROR_BLOB_GAS_LIMIT_EXCEEDED,
            rlp::encode_list(&[encode_u64(blob_gas_limit), encode_u64(blob_gas_used)]),
        ),
        BlockProcessingError::BlobGasUsedMismatch {
            expected, computed, ..
        } => (
            ERROR_BLOB_GAS_USED_MISMATCH,
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
        BlockProcessingError::UnexpectedWithdrawals { count, .. } => (
            ERROR_UNEXPECTED_WITHDRAWALS,
            rlp::encode_list(&[encode_u64(count as u64)]),
        ),
        BlockProcessingError::WithdrawalsRootMismatch {
            expected, computed, ..
        } => (
            ERROR_WITHDRAWALS_ROOT_MISMATCH,
            rlp::encode_list(&[encode_hash(expected), encode_hash(computed)]),
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

#[derive(Debug)]
enum GuestError {
    Block(BlockProcessingError),
    Rlp(RlpError),
    InvalidInput,
}

#[cfg(test)]
mod tests {
    use super::*;
    use claudeth::state::{Storage, Trie};
    use claudeth::types::Hash;

    #[test]
    fn test_withdrawals_presence_allows_empty_list() {
        let mut header = BlockHeader::default();
        header.withdrawals_root = Some(Hash::from([0x11; 32]));

        let result = validate_withdrawals_presence(&header, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_withdrawals_presence_requires_list_when_root_present() {
        let mut header = BlockHeader::default();
        header.withdrawals_root = Some(Hash::from([0x22; 32]));

        let result = validate_withdrawals_presence(&header, false);
        assert!(matches!(result, Err(GuestError::InvalidInput)));
    }

    #[test]
    fn test_withdrawals_presence_rejects_list_when_root_absent() {
        let header = BlockHeader::default();

        let result = validate_withdrawals_presence(&header, true);
        assert!(matches!(result, Err(GuestError::InvalidInput)));
    }

    #[test]
    fn test_block_hashes_allows_empty_list() {
        let mut header = BlockHeader::default();
        header.number = 10;
        let parent = BlockHeader::default();

        let result = validate_block_hashes(&header, &parent, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_block_hashes_rejects_genesis_with_hashes() {
        let header = BlockHeader::default();
        let parent = BlockHeader::default();
        let hashes = vec![Hash::from([0x11; 32])];

        let result = validate_block_hashes(&header, &parent, &hashes);
        assert!(matches!(result, Err(GuestError::InvalidInput)));
    }

    #[test]
    fn test_block_hashes_rejects_too_many_for_height() {
        let mut header = BlockHeader::default();
        header.number = 2;
        let parent = BlockHeader::default();
        let parent_hash = parent.compute_hash();
        let hashes = vec![Hash::from([0x22; 32]), Hash::from([0x33; 32]), parent_hash];

        let result = validate_block_hashes(&header, &parent, &hashes);
        assert!(matches!(result, Err(GuestError::InvalidInput)));
    }

    #[test]
    fn test_block_hashes_requires_parent_hash_last() {
        let mut header = BlockHeader::default();
        header.number = 1;
        let parent = BlockHeader::default();
        let hashes = vec![Hash::from([0x44; 32])];

        let result = validate_block_hashes(&header, &parent, &hashes);
        assert!(matches!(result, Err(GuestError::InvalidInput)));
    }

    #[test]
    fn test_block_hashes_accepts_parent_hash_last() {
        let mut header = BlockHeader::default();
        header.number = 3;
        let parent = BlockHeader::default();
        let parent_hash = parent.compute_hash();
        let hashes = vec![parent_hash];

        let result = validate_block_hashes(&header, &parent, &hashes);
        assert!(result.is_ok());
    }

    fn encode_proof_nodes(proof: &Proof) -> Vec<u8> {
        let nodes: Vec<Vec<u8>> = proof
            .nodes
            .iter()
            .map(|node| rlp::encode_bytes(node))
            .collect();
        rlp::encode_list(&nodes)
    }

    fn build_single_account_witness() -> (Vec<u8>, Address, Account, Vec<u8>, U256, U256) {
        let address = Address::from([0x10; 20]);
        let nonce = U256::from(1u64);
        let balance = U256::from(1000u64);
        let code = vec![0x60, 0x00, 0x56];

        let mut storage = Storage::new();
        let slot = U256::from(1u64);
        let value = U256::from(2u64);
        storage.set(&slot, value);
        let storage_root = storage.compute_root();

        let code_hash = keccak256(&code);
        let account = Account::new_contract(nonce, balance, storage_root, code_hash);
        let account_rlp = account.encode_rlp();

        let mut state_trie = Trie::new();
        let account_key = keccak256(address.as_bytes());
        state_trie.insert(account_key.as_bytes(), account_rlp.clone());
        let state_root = state_trie.compute_root();

        let account_proof = state_trie.generate_proof(account_key.as_bytes()).unwrap();
        let storage_proof = storage.generate_proof(&slot).unwrap();

        let witness_account = rlp::encode_list(&[
            rlp::encode_address(&address),
            encode_proof_nodes(&account_proof),
            rlp::encode_bytes(&account_rlp),
            rlp::encode_bytes(&code),
            rlp::encode_list(&[rlp::encode_list(&[
                rlp::encode_u256(&slot),
                rlp::encode_u256(&value),
                encode_proof_nodes(&storage_proof),
            ])]),
        ]);

        let witness = rlp::encode_list(&[
            rlp::encode_u64(1),
            rlp::encode_hash(&state_root),
            rlp::encode_list(&[witness_account]),
        ]);

        (witness, address, account, code, slot, value)
    }

    #[test]
    fn test_witness_decode_and_apply() {
        let (witness, address, account, code, slot, value) = build_single_account_witness();
        let state_source = decode_state_source(&witness).expect("decode witness");
        let mut state = InMemoryState::new();

        match state_source {
            StateSource::Witness(witness) => {
                apply_witness_state(&mut state, witness).expect("apply witness");
            }
            StateSource::Entries(_) => panic!("expected witness state"),
        }

        assert_eq!(state.get_balance(&address), account.balance);
        assert_eq!(state.get_nonce(&address), account.nonce);
        assert_eq!(state.get_code(&address), code.as_slice());
        assert_eq!(state.sload(&address, &slot), value);
    }

    #[test]
    fn test_witness_rejects_code_hash_mismatch() {
        let (mut witness, _address, _account, _code, _slot, _value) =
            build_single_account_witness();

        let (items, rest) = rlp::decode_list(&witness).expect("decode witness list");
        assert!(rest.is_empty());
        let (accounts, rest) = rlp::decode_list(&items[2]).expect("decode accounts list");
        assert!(rest.is_empty());
        let (fields, rest) = rlp::decode_list(&accounts[0]).expect("decode account");
        assert!(rest.is_empty());

        let mut mutated_fields = fields.clone();
        mutated_fields[3] = rlp::encode_bytes(&[0x01, 0x02, 0x03]);

        let mutated_account = rlp::encode_list(&mutated_fields);
        let mutated_accounts = rlp::encode_list(&[mutated_account]);
        let mutated_witness =
            rlp::encode_list(&[items[0].clone(), items[1].clone(), mutated_accounts]);
        witness = mutated_witness;

        let state_source = decode_state_source(&witness).expect("decode witness");
        let mut state = InMemoryState::new();
        let result = match state_source {
            StateSource::Witness(witness) => apply_witness_state(&mut state, witness),
            StateSource::Entries(_) => Ok(()),
        };

        assert!(matches!(result, Err(GuestError::InvalidInput)));
    }

    #[test]
    fn test_witness_requires_sorted_accounts() {
        let address1 = Address::from([0x01; 20]);
        let address2 = Address::from([0x02; 20]);

        let account1 = Account::new_eoa(U256::from(1u64), U256::from(10u64));
        let account2 = Account::new_eoa(U256::from(2u64), U256::from(20u64));

        let account1_rlp = account1.encode_rlp();
        let account2_rlp = account2.encode_rlp();

        let mut state_trie = Trie::new();
        let key1 = keccak256(address1.as_bytes());
        let key2 = keccak256(address2.as_bytes());
        state_trie.insert(key1.as_bytes(), account1_rlp.clone());
        state_trie.insert(key2.as_bytes(), account2_rlp.clone());
        let state_root = state_trie.compute_root();

        let proof1 = state_trie.generate_proof(key1.as_bytes()).unwrap();
        let proof2 = state_trie.generate_proof(key2.as_bytes()).unwrap();

        let witness_account2 = rlp::encode_list(&[
            rlp::encode_address(&address2),
            encode_proof_nodes(&proof2),
            rlp::encode_bytes(&account2_rlp),
            rlp::encode_bytes(&[]),
            rlp::encode_list(&[]),
        ]);

        let witness_account1 = rlp::encode_list(&[
            rlp::encode_address(&address1),
            encode_proof_nodes(&proof1),
            rlp::encode_bytes(&account1_rlp),
            rlp::encode_bytes(&[]),
            rlp::encode_list(&[]),
        ]);

        let witness = rlp::encode_list(&[
            rlp::encode_u64(1),
            rlp::encode_hash(&state_root),
            rlp::encode_list(&[witness_account2, witness_account1]),
        ]);

        let state_source = decode_state_source(&witness).expect("decode witness");
        let mut state = InMemoryState::new();
        let result = match state_source {
            StateSource::Witness(witness) => apply_witness_state(&mut state, witness),
            StateSource::Entries(_) => Ok(()),
        };

        assert!(matches!(result, Err(GuestError::InvalidInput)));
    }
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
            let max_size = end.saturating_sub(start);
            if max_size == 0 {
                return Vec::new();
            }

            // Why: runner writes raw bytes at __input_start without a length
            // word. Deriving the total size from the leading RLP prefix avoids
            // treating zero-filled capacity tail as part of the payload.
            let first = core::ptr::read_volatile(start as *const u8);
            let mut read_len = match first {
                0x00..=0x7f => 1usize,
                0x80..=0xb7 => 1usize + (first as usize - 0x80),
                0xb8..=0xbf => {
                    let len_of_len = (first as usize).saturating_sub(0xb7);
                    if len_of_len == 0 || len_of_len > 8 || len_of_len + 1 > max_size {
                        max_size
                    } else {
                        let mut payload_len = 0usize;
                        for idx in 0..len_of_len {
                            let byte = core::ptr::read_volatile((start + 1 + idx) as *const u8);
                            payload_len = (payload_len << 8) | byte as usize;
                        }
                        1 + len_of_len + payload_len
                    }
                }
                0xc0..=0xf7 => 1usize + (first as usize - 0xc0),
                0xf8..=0xff => {
                    let len_of_len = (first as usize).saturating_sub(0xf7);
                    if len_of_len == 0 || len_of_len > 8 || len_of_len + 1 > max_size {
                        max_size
                    } else {
                        let mut payload_len = 0usize;
                        for idx in 0..len_of_len {
                            let byte = core::ptr::read_volatile((start + 1 + idx) as *const u8);
                            payload_len = (payload_len << 8) | byte as usize;
                        }
                        1 + len_of_len + payload_len
                    }
                }
            };

            if read_len > max_size {
                read_len = max_size;
            }

            let mut buf = Vec::with_capacity(read_len);
            for i in 0..read_len {
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
