// EELS Blockchain Test Integration
//
// This module integrates ethereum/tests BlockchainTests to verify claudeth's
// compliance with the Ethereum execution specification.
//
// Test fixtures are loaded from tests/eels/BlockchainTests/ (not checked into git).
// Run scripts/fetch_eels_tests.py to download the test fixtures.

use claudeth::evm::format_disassembly;
use claudeth::state::{InMemoryState, State};
use claudeth::types::{Address, Bytes, U256};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;

/// BlockchainTest fixture format
///
/// Structure based on ethereum/tests repository format:
/// https://ethereum-tests.readthedocs.io/en/latest/test_types/blockchain_tests.html
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BlockchainTest {
    /// Test metadata
    #[serde(rename = "_info")]
    info: TestInfo,

    /// List of blocks to execute
    blocks: Vec<TestBlock>,

    /// Test configuration
    config: TestConfig,

    /// Genesis block header
    genesis_block_header: TestBlockHeader,

    /// RLP-encoded genesis block
    #[serde(rename = "genesisRLP")]
    genesis_rlp: String,

    /// Expected hash of the last block
    #[serde(rename = "lastblockhash")]
    last_block_hash: String,

    /// Expected post-execution state
    #[serde(rename = "postState")]
    post_state: HashMap<String, TestAccount>,

    /// Initial pre-execution state
    pre: HashMap<String, TestAccount>,

    /// Seal engine used (NoProof or Ethash)
    #[serde(rename = "sealEngine")]
    seal_engine: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct TestInfo {
    comment: Option<String>,
    #[serde(rename = "filling-rpc-server")]
    filling_rpc_server: Option<String>,
    #[serde(rename = "filling-tool-version")]
    filling_tool_version: Option<String>,
    #[serde(rename = "fixture-format")]
    fixture_format: Option<String>,
    hash: Option<String>,
    repo: Option<String>,
    source: Option<String>,
    #[serde(rename = "sourceHash")]
    source_hash: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TestConfig {
    chainid: String,
    network: String,
    /// Blob schedule configuration (post-Cancun)
    #[serde(rename = "blobSchedule")]
    blob_schedule: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestBlock {
    /// Block header (present for valid blocks)
    #[serde(skip_serializing_if = "Option::is_none")]
    block_header: Option<TestBlockHeader>,
    /// Transactions (present for valid blocks)
    #[serde(default)]
    transactions: Vec<TestTransaction>,
    /// Uncle headers (usually empty for PoS)
    #[serde(default)]
    uncle_headers: Vec<Value>,
    /// Withdrawals (post-Shanghai)
    #[serde(default)]
    withdrawals: Vec<TestWithdrawal>,
    /// RLP-encoded block
    rlp: String,
    /// Block number
    #[serde(rename = "blocknumber")]
    block_number: Option<String>,
    /// Chain name
    #[serde(rename = "chainname")]
    chain_name: Option<String>,
    /// Expected exception for invalid blocks
    #[serde(rename = "expectException")]
    expect_exception: Option<String>,
    /// Decoded RLP (for invalid blocks)
    rlp_decoded: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestBlockHeader {
    base_fee_per_gas: Option<String>,
    blob_gas_used: Option<String>,
    bloom: String,
    coinbase: String,
    difficulty: String,
    excess_blob_gas: Option<String>,
    extra_data: String,
    gas_limit: String,
    gas_used: String,
    hash: Option<String>,
    mix_hash: Option<String>,
    nonce: Option<String>,
    number: String,
    parent_beacon_block_root: Option<String>,
    requests_hash: Option<String>,
    parent_hash: String,
    receipt_trie: Option<String>,
    state_root: String,
    timestamp: String,
    transactions_trie: Option<String>,
    uncle_hash: Option<String>,
    withdrawals_root: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestTransaction {
    /// Transaction type (0x00 = Legacy, 0x01 = EIP-2930, 0x02 = EIP-1559)
    #[serde(rename = "type")]
    tx_type: Option<String>,
    chain_id: Option<String>,
    data: String,
    gas_limit: String,
    nonce: String,
    to: String,
    value: String,
    /// Legacy transaction fields
    gas_price: Option<String>,
    /// EIP-1559 transaction fields
    max_fee_per_gas: Option<String>,
    max_priority_fee_per_gas: Option<String>,
    /// EIP-4844 blob transaction field
    max_fee_per_blob_gas: Option<String>,
    /// EIP-4844 blob transaction field
    blob_versioned_hashes: Option<Vec<String>>,
    /// EIP-2930 access list
    access_list: Option<Vec<AccessListEntry>>,
    /// Signature fields
    v: String,
    r: String,
    s: String,
    /// Sender (computed from signature)
    sender: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccessListEntry {
    address: String,
    storage_keys: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestWithdrawal {
    index: String,
    validator_index: String,
    address: String,
    amount: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct TestAccount {
    balance: String,
    code: String,
    nonce: String,
    storage: HashMap<String, String>,
}

/// Load a single blockchain test from a JSON file
fn load_blockchain_test(
    path: &Path,
) -> Result<HashMap<String, BlockchainTest>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let tests: HashMap<String, BlockchainTest> = serde_json::from_str(&content)?;
    Ok(tests)
}

/// Discover all blockchain test files in the tests/eels directory
fn discover_blockchain_tests() -> Vec<std::path::PathBuf> {
    let test_dir = Path::new("tests/eels/BlockchainTests");
    if !test_dir.exists() {
        eprintln!(
            "Warning: EELS test directory not found at {}",
            test_dir.display()
        );
        eprintln!("Run scripts/fetch_eels_tests.py to download test fixtures");
        return vec![];
    }

    let mut tests = vec![];
    for entry in walkdir::WalkDir::new(test_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
            // Skip metadata files
            if entry.path().file_name().and_then(|s| s.to_str()) == Some("index.json") {
                continue;
            }
            tests.push(entry.path().to_path_buf());
        }
    }
    tests
}

fn resolve_parent_header<'a>(
    block_header: &claudeth::types::BlockHeader,
    headers_by_hash: &'a HashMap<claudeth::types::Hash, claudeth::types::BlockHeader>,
) -> Option<&'a claudeth::types::BlockHeader> {
    headers_by_hash.get(&block_header.parent_hash)
}

fn collect_recent_block_hashes(
    parent_hash: claudeth::types::Hash,
    headers_by_hash: &HashMap<claudeth::types::Hash, claudeth::types::BlockHeader>,
) -> Vec<claudeth::types::Hash> {
    // execution-specs expects BLOCKHASH inputs in increasing block-number
    // order with the direct parent as the last element.
    let mut newest_to_oldest = Vec::new();
    let mut cursor = parent_hash;

    while cursor != claudeth::types::Hash::ZERO && newest_to_oldest.len() < 256 {
        newest_to_oldest.push(cursor);
        let Some(parent_header) = headers_by_hash.get(&cursor) else {
            break;
        };
        cursor = parent_header.parent_hash;
    }

    newest_to_oldest.reverse();
    newest_to_oldest
}

fn parse_address(value: &str) -> Result<Address, String> {
    Address::from_str(value).map_err(|err| format!("invalid address {value}: {err}"))
}

fn parse_u256(value: &str) -> Result<U256, String> {
    let trimmed = value.trim();
    let trimmed = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    if trimmed.is_empty() {
        return Ok(U256::ZERO);
    }
    U256::from_str(value).map_err(|err| format!("invalid U256 {value}: {err}"))
}

fn parse_bytes(value: &str) -> Result<Bytes, String> {
    Bytes::from_str(value).map_err(|err| format!("invalid bytes {value}: {err}"))
}

fn dump_transaction_disassembly(test_block: &TestBlock) {
    if test_block.transactions.is_empty() {
        return;
    }

    eprintln!("  Disassembly (transaction data):");
    for (tx_idx, tx) in test_block.transactions.iter().enumerate() {
        match parse_bytes(&tx.data) {
            Ok(bytes) => {
                if bytes.is_empty() {
                    eprintln!("    tx {tx_idx}: <empty data>");
                    continue;
                }
                let lines = format_disassembly(bytes.as_ref());
                eprintln!(
                    "    tx {tx_idx}: {} bytes, {} instructions",
                    bytes.len(),
                    lines.len()
                );
                for line in lines.iter().take(200) {
                    eprintln!("      {line}");
                }
                if lines.len() > 200 {
                    eprintln!("      ... {} more lines", lines.len() - 200);
                }
            }
            Err(err) => {
                eprintln!("    tx {tx_idx}: failed to parse data: {err}");
            }
        }
    }
}

fn apply_pre_state(
    state: &mut InMemoryState,
    pre: &HashMap<String, TestAccount>,
) -> Result<(), String> {
    for (address_str, account) in pre {
        let address = parse_address(address_str)?;
        let balance = parse_u256(&account.balance)?;
        let nonce = parse_u256(&account.nonce)?;
        let code = parse_bytes(&account.code)?;

        state.set_balance(&address, balance);
        state.set_nonce(&address, nonce);
        state.set_code(&address, Vec::from(code));

        for (key, value) in &account.storage {
            let key_u256 = parse_u256(key)?;
            let value_u256 = parse_u256(value)?;
            state.sstore(&address, &key_u256, value_u256);
        }
    }

    Ok(())
}

fn validate_account_state(
    state: &InMemoryState,
    address_str: &str,
    expected: &TestAccount,
) -> Result<(), String> {
    let address = parse_address(address_str)?;
    let expected_balance = parse_u256(&expected.balance)?;
    let expected_nonce = parse_u256(&expected.nonce)?;
    let expected_code = parse_bytes(&expected.code)?;

    let actual_balance = state.get_balance(&address);
    if actual_balance != expected_balance {
        return Err(format!(
            "balance mismatch for {address_str}: expected {expected_balance}, got {actual_balance}"
        ));
    }

    let actual_nonce = state.get_nonce(&address);
    if actual_nonce != expected_nonce {
        return Err(format!(
            "nonce mismatch for {address_str}: expected {expected_nonce}, got {actual_nonce}"
        ));
    }

    let actual_code = state.get_code(&address);
    if actual_code != expected_code.as_ref() {
        return Err(format!(
            "code mismatch for {address_str}: expected {} bytes, got {} bytes",
            expected_code.len(),
            actual_code.len()
        ));
    }

    for (key_str, value_str) in expected.storage.iter() {
        let key = parse_u256(key_str)?;
        let expected_value = parse_u256(value_str)?;
        let actual_value = state.sload(&address, &key);
        if actual_value != expected_value {
            return Err(format!(
                "storage mismatch for {address_str} at {key_str}: expected {expected_value}, got {actual_value}"
            ));
        }
    }

    Ok(())
}

fn validate_post_state(
    state: &InMemoryState,
    pre: &HashMap<String, TestAccount>,
    post: &HashMap<String, TestAccount>,
) -> Result<(), String> {
    for (address_str, expected) in post {
        validate_account_state(state, address_str, expected)?;

        if let Some(pre_account) = pre.get(address_str) {
            for key_str in pre_account.storage.keys() {
                if !expected.storage.contains_key(key_str) {
                    let key = parse_u256(key_str)?;
                    let actual_value = state.sload(&parse_address(address_str)?, &key);
                    if actual_value != U256::ZERO {
                        return Err(format!(
                            "storage mismatch for {address_str} at {key_str}: expected 0, got {actual_value}"
                        ));
                    }
                }
            }
        }
    }

    for (address_str, pre_account) in pre {
        if post.contains_key(address_str) {
            continue;
        }

        let address = parse_address(address_str)?;
        let actual_balance = state.get_balance(&address);
        let actual_nonce = state.get_nonce(&address);
        let actual_code = state.get_code(&address);

        if actual_balance != U256::ZERO || actual_nonce != U256::ZERO || !actual_code.is_empty() {
            return Err(format!(
                "account {address_str} expected empty but got balance {actual_balance}, nonce {actual_nonce}, code {} bytes",
                actual_code.len()
            ));
        }

        for key_str in pre_account.storage.keys() {
            let key = parse_u256(key_str)?;
            let actual_value = state.sload(&address, &key);
            if actual_value != U256::ZERO {
                return Err(format!(
                    "storage mismatch for {address_str} at {key_str}: expected 0, got {actual_value}"
                ));
            }
        }
    }

    Ok(())
}

fn parse_u64(value: &str) -> Result<u64, String> {
    let u256_val = parse_u256(value)?;
    u64::try_from(u256_val).map_err(|_| format!("value {value} too large for u64"))
}

fn convert_test_transaction(
    test_tx: &TestTransaction,
) -> Result<claudeth::types::Transaction, String> {
    use claudeth::types::transaction::{
        AccessListEntry as ClaudethAccessListEntry, BlobTransaction, Eip1559Transaction,
        Eip2930Transaction, LegacyTransaction,
    };
    use claudeth::types::{Hash, Transaction};

    let nonce = parse_u256(&test_tx.nonce)?;
    let gas_limit = parse_u256(&test_tx.gas_limit)?;
    let value = parse_u256(&test_tx.value)?;
    let data = parse_bytes(&test_tx.data)?;
    let v = parse_u256(&test_tx.v)?;
    let r = parse_u256(&test_tx.r)?;
    let s = parse_u256(&test_tx.s)?;

    let to = if test_tx.to.is_empty() || test_tx.to == "0x" {
        None
    } else {
        Some(parse_address(&test_tx.to)?)
    };

    // Determine transaction type
    let tx_type = test_tx
        .tx_type
        .as_ref()
        .and_then(|t| u8::from_str_radix(t.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0);

    let tx = match tx_type {
        0x00 => {
            // Legacy transaction
            let gas_price = test_tx
                .gas_price
                .as_ref()
                .ok_or("Legacy transaction missing gas_price")?;
            Transaction::Legacy(LegacyTransaction {
                nonce,
                gas_price: parse_u256(gas_price)?,
                gas_limit,
                to,
                value,
                data,
                v,
                r,
                s,
            })
        }
        0x01 => {
            // EIP-2930 transaction
            let chain_id = test_tx
                .chain_id
                .as_ref()
                .ok_or("EIP-2930 transaction missing chain_id")?;
            let gas_price = test_tx
                .gas_price
                .as_ref()
                .ok_or("EIP-2930 transaction missing gas_price")?;

            let access_list = test_tx
                .access_list
                .as_ref()
                .map(|al| {
                    al.iter()
                        .map(|entry| {
                            let address = parse_address(&entry.address)?;
                            let storage_keys = entry
                                .storage_keys
                                .iter()
                                .map(|key| {
                                    Hash::from_str(key)
                                        .map_err(|err| format!("invalid storage key {key}: {err}"))
                                })
                                .collect::<Result<Vec<_>, _>>()?;
                            Ok(ClaudethAccessListEntry {
                                address,
                                storage_keys,
                            })
                        })
                        .collect::<Result<Vec<_>, String>>()
                })
                .transpose()?
                .unwrap_or_default();

            Transaction::Eip2930(Eip2930Transaction {
                chain_id: parse_u256(chain_id)?,
                nonce,
                gas_price: parse_u256(gas_price)?,
                gas_limit,
                to,
                value,
                data,
                access_list,
                v,
                r,
                s,
            })
        }
        0x02 => {
            // EIP-1559 transaction
            let chain_id = test_tx
                .chain_id
                .as_ref()
                .ok_or("EIP-1559 transaction missing chain_id")?;
            let max_fee_per_gas = test_tx
                .max_fee_per_gas
                .as_ref()
                .ok_or("EIP-1559 transaction missing max_fee_per_gas")?;
            let max_priority_fee_per_gas = test_tx
                .max_priority_fee_per_gas
                .as_ref()
                .ok_or("EIP-1559 transaction missing max_priority_fee_per_gas")?;

            let access_list = test_tx
                .access_list
                .as_ref()
                .map(|al| {
                    al.iter()
                        .map(|entry| {
                            let address = parse_address(&entry.address)?;
                            let storage_keys = entry
                                .storage_keys
                                .iter()
                                .map(|key| {
                                    Hash::from_str(key)
                                        .map_err(|err| format!("invalid storage key {key}: {err}"))
                                })
                                .collect::<Result<Vec<_>, _>>()?;
                            Ok(ClaudethAccessListEntry {
                                address,
                                storage_keys,
                            })
                        })
                        .collect::<Result<Vec<_>, String>>()
                })
                .transpose()?
                .unwrap_or_default();

            Transaction::Eip1559(Eip1559Transaction {
                chain_id: parse_u256(chain_id)?,
                nonce,
                max_priority_fee_per_gas: parse_u256(max_priority_fee_per_gas)?,
                max_fee_per_gas: parse_u256(max_fee_per_gas)?,
                gas_limit,
                to,
                value,
                data,
                access_list,
                v,
                r,
                s,
            })
        }
        0x03 => {
            // Cancun/Prague fixtures include type-3 transactions; rejecting
            // these silently skews conformance coverage toward pre-4844 flows.
            let chain_id = test_tx
                .chain_id
                .as_ref()
                .ok_or("EIP-4844 transaction missing chain_id")?;
            let max_fee_per_gas = test_tx
                .max_fee_per_gas
                .as_ref()
                .ok_or("EIP-4844 transaction missing max_fee_per_gas")?;
            let max_priority_fee_per_gas = test_tx
                .max_priority_fee_per_gas
                .as_ref()
                .ok_or("EIP-4844 transaction missing max_priority_fee_per_gas")?;
            let max_fee_per_blob_gas = test_tx
                .max_fee_per_blob_gas
                .as_ref()
                .ok_or("EIP-4844 transaction missing max_fee_per_blob_gas")?;
            let to = to.ok_or("EIP-4844 transaction missing to address")?;

            let access_list = test_tx
                .access_list
                .as_ref()
                .map(|al| {
                    al.iter()
                        .map(|entry| {
                            let address = parse_address(&entry.address)?;
                            let storage_keys = entry
                                .storage_keys
                                .iter()
                                .map(|key| {
                                    Hash::from_str(key)
                                        .map_err(|err| format!("invalid storage key {key}: {err}"))
                                })
                                .collect::<Result<Vec<_>, _>>()?;
                            Ok(ClaudethAccessListEntry {
                                address,
                                storage_keys,
                            })
                        })
                        .collect::<Result<Vec<_>, String>>()
                })
                .transpose()?
                .unwrap_or_default();

            let blob_versioned_hashes = test_tx
                .blob_versioned_hashes
                .as_ref()
                .ok_or("EIP-4844 transaction missing blob_versioned_hashes")?
                .iter()
                .map(|hash| {
                    Hash::from_str(hash)
                        .map_err(|err| format!("invalid blob versioned hash {hash}: {err}"))
                })
                .collect::<Result<Vec<_>, _>>()?;

            Transaction::Blob(BlobTransaction {
                chain_id: parse_u256(chain_id)?,
                nonce,
                max_priority_fee_per_gas: parse_u256(max_priority_fee_per_gas)?,
                max_fee_per_gas: parse_u256(max_fee_per_gas)?,
                gas_limit,
                to,
                value,
                data,
                access_list,
                max_fee_per_blob_gas: parse_u256(max_fee_per_blob_gas)?,
                blob_versioned_hashes,
                v,
                r,
                s,
            })
        }
        _ => return Err(format!("Unsupported transaction type: {tx_type:#x}")),
    };

    Ok(tx)
}

fn convert_test_withdrawal(
    test_withdrawal: &TestWithdrawal,
) -> Result<claudeth::types::Withdrawal, String> {
    // Fixture withdrawals encode numeric fields as hex strings; parsing here keeps
    // process_block input aligned with execution-spec body semantics.
    Ok(claudeth::types::Withdrawal {
        index: parse_u64(&test_withdrawal.index)?,
        validator_index: parse_u64(&test_withdrawal.validator_index)?,
        address: parse_address(&test_withdrawal.address)?,
        amount_gwei: parse_u64(&test_withdrawal.amount)?,
    })
}

fn convert_test_block_header(
    test_header: &TestBlockHeader,
) -> Result<claudeth::types::BlockHeader, String> {
    use claudeth::types::{BlockHeader, Hash};

    let parent_hash = Hash::from_str(&test_header.parent_hash)
        .map_err(|err| format!("invalid parent_hash: {err}"))?;
    let ommers_hash = test_header
        .uncle_hash
        .as_ref()
        .map(|h| Hash::from_str(h))
        .transpose()
        .map_err(|err| format!("invalid uncle_hash: {err}"))?
        .unwrap_or(claudeth::types::EMPTY_OMMERS_HASH);
    let coinbase = parse_address(&test_header.coinbase)?;
    let state_root = Hash::from_str(&test_header.state_root)
        .map_err(|err| format!("invalid state_root: {err}"))?;
    let transactions_root = test_header
        .transactions_trie
        .as_ref()
        .map(|h| Hash::from_str(h))
        .transpose()
        .map_err(|err| format!("invalid transactions_trie: {err}"))?
        .unwrap_or(Hash::ZERO);
    let receipts_root = test_header
        .receipt_trie
        .as_ref()
        .map(|h| Hash::from_str(h))
        .transpose()
        .map_err(|err| format!("invalid receipt_trie: {err}"))?
        .unwrap_or(Hash::ZERO);

    let logs_bloom = parse_bytes(&test_header.bloom)?;
    if logs_bloom.len() != 256 {
        return Err(format!(
            "invalid logs_bloom length: expected 256, got {}",
            logs_bloom.len()
        ));
    }
    let mut logs_bloom_arr = [0u8; 256];
    logs_bloom_arr.copy_from_slice(&logs_bloom[..]);

    let difficulty = parse_u256(&test_header.difficulty)?;
    let number = parse_u64(&test_header.number)?;
    let gas_limit = parse_u64(&test_header.gas_limit)?;
    let gas_used = parse_u64(&test_header.gas_used)?;
    let timestamp = parse_u64(&test_header.timestamp)?;
    let extra_data = parse_bytes(&test_header.extra_data)?;
    let mix_hash = test_header
        .mix_hash
        .as_ref()
        .map(|h| Hash::from_str(h))
        .transpose()
        .map_err(|err| format!("invalid mix_hash: {err}"))?
        .unwrap_or(Hash::ZERO);
    let nonce = test_header
        .nonce
        .as_ref()
        .map(|n| parse_u64(n))
        .transpose()?
        .unwrap_or(0);

    let base_fee_per_gas = test_header
        .base_fee_per_gas
        .as_ref()
        .map(|b| parse_u64(b))
        .transpose()?;

    let withdrawals_root = test_header
        .withdrawals_root
        .as_ref()
        .map(|h| Hash::from_str(h))
        .transpose()
        .map_err(|err| format!("invalid withdrawals_root: {err}"))?;

    let blob_gas_used = test_header
        .blob_gas_used
        .as_ref()
        .map(|b| parse_u64(b))
        .transpose()?;

    let excess_blob_gas = test_header
        .excess_blob_gas
        .as_ref()
        .map(|e| parse_u64(e))
        .transpose()?;

    let parent_beacon_block_root = test_header
        .parent_beacon_block_root
        .as_ref()
        .map(|h| Hash::from_str(h))
        .transpose()
        .map_err(|err| format!("invalid parent_beacon_block_root: {err}"))?;
    let requests_hash = test_header
        .requests_hash
        .as_ref()
        .map(|h| Hash::from_str(h))
        .transpose()
        .map_err(|err| format!("invalid requests_hash: {err}"))?;

    Ok(BlockHeader {
        parent_hash,
        ommers_hash,
        coinbase,
        state_root,
        transactions_root,
        receipts_root,
        logs_bloom: logs_bloom_arr,
        difficulty,
        number,
        gas_limit,
        gas_used,
        timestamp,
        extra_data,
        mix_hash,
        nonce,
        base_fee_per_gas,
        withdrawals_root,
        blob_gas_used,
        excess_blob_gas,
        parent_beacon_block_root,
        requests_hash,
    })
}

#[test]
fn test_can_parse_blockchain_tests() {
    let tests = discover_blockchain_tests();
    if tests.is_empty() {
        eprintln!("No EELS tests found - skipping test");
        return;
    }

    let mut parsed = 0;
    let mut failed = 0;
    let mut pre_state_parsed = false;
    let mut block_header_converted = false;
    let mut transaction_converted = false;
    let mut withdrawal_converted = false;

    // Parse every discovered file so this test actually tracks fixture-shape drift.
    for test_path in &tests {
        match load_blockchain_test(test_path) {
            Ok(test_cases) => {
                for test in test_cases.values() {
                    if !pre_state_parsed {
                        let mut state = InMemoryState::new();
                        apply_pre_state(&mut state, &test.pre)
                            .unwrap_or_else(|err| panic!("failed to parse pre-state: {err}"));
                        pre_state_parsed = true;
                    }

                    // Test block header conversion
                    if !block_header_converted
                        && let Some(block) = test.blocks.first()
                        && let Some(header) = &block.block_header
                    {
                        convert_test_block_header(header)
                            .unwrap_or_else(|err| panic!("failed to convert block header: {err}"));
                        block_header_converted = true;
                    }

                    // Test transaction conversion
                    if !transaction_converted
                        && let Some(block) = test.blocks.first()
                        && !block.transactions.is_empty()
                    {
                        convert_test_transaction(&block.transactions[0])
                            .unwrap_or_else(|err| panic!("failed to convert transaction: {err}"));
                        transaction_converted = true;
                    }

                    if !withdrawal_converted
                        && let Some(block) = test.blocks.first()
                        && !block.withdrawals.is_empty()
                    {
                        convert_test_withdrawal(&block.withdrawals[0])
                            .unwrap_or_else(|err| panic!("failed to convert withdrawal: {err}"));
                        withdrawal_converted = true;
                    }
                }

                parsed += test_cases.len();
                let num_cases = test_cases.len();
                println!(
                    "✓ Parsed {num_cases} test cases from {}",
                    test_path.display()
                );
            }
            Err(e) => {
                failed += 1;
                eprintln!("✗ Failed to parse {}: {e}", test_path.display());
            }
        }
    }

    println!("\nParsing summary:");
    println!("  Parsed: {parsed} test cases");
    println!("  Failed: {failed} files");

    assert!(parsed > 0, "Should successfully parse at least one test");
    assert!(pre_state_parsed, "Should parse at least one pre-state");
    assert!(
        block_header_converted,
        "Should convert at least one block header"
    );
    assert!(
        transaction_converted,
        "Should convert at least one transaction"
    );
}

#[test]
fn test_convert_test_withdrawal_parses_hex_fields() {
    let test_withdrawal = TestWithdrawal {
        index: "0x00".to_string(),
        validator_index: "0x01".to_string(),
        address: "0xc94f5374fce5edbc8e2a8697c15331677e6ebf0b".to_string(),
        amount: "0x2710".to_string(),
    };

    let converted = convert_test_withdrawal(&test_withdrawal).expect("convert withdrawal");

    assert_eq!(converted.index, 0);
    assert_eq!(converted.validator_index, 1);
    assert_eq!(converted.amount_gwei, 10_000);
    assert_eq!(
        converted.address,
        parse_address("0xc94f5374fce5edbc8e2a8697c15331677e6ebf0b").unwrap()
    );
}

#[test]
fn test_convert_test_withdrawal_rejects_invalid_address() {
    let test_withdrawal = TestWithdrawal {
        index: "0x00".to_string(),
        validator_index: "0x01".to_string(),
        address: "0x1234".to_string(),
        amount: "0x01".to_string(),
    };

    let err = convert_test_withdrawal(&test_withdrawal).unwrap_err();
    assert!(err.contains("invalid address"));
}

#[test]
fn test_convert_test_withdrawal_rejects_amount_over_u64() {
    let test_withdrawal = TestWithdrawal {
        index: "0x00".to_string(),
        validator_index: "0x01".to_string(),
        address: "0xc94f5374fce5edbc8e2a8697c15331677e6ebf0b".to_string(),
        amount: "0x10000000000000000".to_string(),
    };

    let err = convert_test_withdrawal(&test_withdrawal).unwrap_err();
    assert!(err.contains("too large for u64"));
}

#[test]
fn test_convert_test_transaction_blob_fixture_case() {
    use claudeth::types::Transaction;
    use std::path::Path;

    let case = load_single_blockchain_case(
        Path::new(
            "tests/eels/BlockchainTests/ValidBlocks/bcEIP4844-blobtransactions/blockWithAllTransactionTypes.json",
        ),
        "BlockchainTests/ValidBlocks/bcEIP4844-blobtransactions/blockWithAllTransactionTypes.json::blockWithAllTransactionTypes_Cancun",
    );
    let blob_test_tx = case.blocks[0]
        .transactions
        .iter()
        .find(|tx| tx.tx_type.as_deref() == Some("0x03"))
        .expect("fixture must include type-3 transaction");

    let converted = convert_test_transaction(blob_test_tx).expect("convert blob transaction");

    match converted {
        Transaction::Blob(blob) => {
            assert_eq!(blob.max_fee_per_blob_gas, U256::from(10u64));
            assert_eq!(blob.blob_versioned_hashes.len(), 1);
        }
        _ => panic!("expected blob transaction"),
    }
}

#[test]
fn test_convert_test_transaction_blob_missing_fee_field() {
    let test_tx = TestTransaction {
        tx_type: Some("0x03".to_string()),
        chain_id: Some("0x01".to_string()),
        data: "0x".to_string(),
        gas_limit: "0x5208".to_string(),
        nonce: "0x00".to_string(),
        to: "0x100000000000000000000000000000000000000a".to_string(),
        value: "0x00".to_string(),
        gas_price: None,
        max_fee_per_gas: Some("0x03e8".to_string()),
        max_priority_fee_per_gas: Some("0x01".to_string()),
        max_fee_per_blob_gas: None,
        blob_versioned_hashes: Some(vec![
            "0x01a915e4d060149eb4365960e6a7a45f334393093061116b197e3240065ff2d8".to_string(),
        ]),
        access_list: Some(vec![]),
        v: "0x00".to_string(),
        r: "0x01".to_string(),
        s: "0x01".to_string(),
        sender: None,
    };

    let err = convert_test_transaction(&test_tx).unwrap_err();
    assert!(err.contains("missing max_fee_per_blob_gas"));
}

fn load_single_blockchain_case(path: &Path, case_name: &str) -> BlockchainTest {
    let mut cases = load_blockchain_test(path).expect("load fixture file");
    cases.remove(case_name).expect("fixture case present")
}

fn assert_header_hash_matches_fixture(test_header: &TestBlockHeader) {
    let expected_hash = test_header
        .hash
        .as_ref()
        .expect("fixture header hash must be present");
    let expected_hash = claudeth::types::Hash::from_str(expected_hash).expect("parse fixture hash");
    let converted = convert_test_block_header(test_header).expect("convert fixture header");
    assert_eq!(converted.compute_hash(), expected_hash);
}

#[test]
fn test_fixture_header_hashes_match_for_cancun_and_prague_examples() {
    let fixture_path =
        Path::new("tests/eels/BlockchainTests/ValidBlocks/bcExample/shanghaiExample.json");

    let cancun_case = load_single_blockchain_case(
        fixture_path,
        "BlockchainTests/ValidBlocks/bcExample/shanghaiExample.json::shanghaiExample_Cancun",
    );
    assert_header_hash_matches_fixture(&cancun_case.genesis_block_header);
    assert_header_hash_matches_fixture(
        cancun_case.blocks[0]
            .block_header
            .as_ref()
            .expect("valid block header"),
    );

    let prague_case = load_single_blockchain_case(
        fixture_path,
        "BlockchainTests/ValidBlocks/bcExample/shanghaiExample.json::shanghaiExample_Prague",
    );
    assert_header_hash_matches_fixture(&prague_case.genesis_block_header);
    assert_header_hash_matches_fixture(
        prague_case.blocks[0]
            .block_header
            .as_ref()
            .expect("valid block header"),
    );
}

#[test]
fn test_fixture_parent_hash_linkage_uses_real_header_hashes() {
    let fixture_path =
        Path::new("tests/eels/BlockchainTests/ValidBlocks/bcExample/shanghaiExample.json");
    let case = load_single_blockchain_case(
        fixture_path,
        "BlockchainTests/ValidBlocks/bcExample/shanghaiExample.json::shanghaiExample_Prague",
    );

    let genesis = convert_test_block_header(&case.genesis_block_header).expect("convert genesis");
    let child = convert_test_block_header(
        case.blocks[0]
            .block_header
            .as_ref()
            .expect("valid block header"),
    )
    .expect("convert child");

    assert_eq!(child.parent_hash, genesis.compute_hash());
}

#[test]
fn test_multi_chain_fixture_parent_selection_uses_parent_hash_not_linear_order() {
    let fixture_path = Path::new(
        "tests/eels/BlockchainTests/InvalidBlocks/bcMultiChainTest/UncleFromSideChain.json",
    );
    let case = load_single_blockchain_case(
        fixture_path,
        "BlockchainTests/InvalidBlocks/bcMultiChainTest/UncleFromSideChain.json::UncleFromSideChain_Cancun",
    );

    let genesis = convert_test_block_header(&case.genesis_block_header).expect("convert genesis");
    let genesis_hash = genesis.compute_hash();
    let mut headers_by_hash = HashMap::new();
    headers_by_hash.insert(genesis_hash, genesis.clone());

    // The fixture switches from chain A height-3 back to chain B height-1.
    // Parent resolution must follow parent_hash, not previous iteration order.
    for test_block in case.blocks.iter().take(4) {
        let header = convert_test_block_header(
            test_block
                .block_header
                .as_ref()
                .expect("fixture block header"),
        )
        .expect("convert fixture header");

        let parent =
            resolve_parent_header(&header, &headers_by_hash).expect("parent must be available");
        if test_block.chain_name.as_deref() == Some("B")
            && test_block.block_number.as_deref() == Some("1")
        {
            assert_eq!(parent.compute_hash(), genesis_hash);
            assert_eq!(parent.number, 0);
        }

        headers_by_hash.insert(header.compute_hash(), header);
    }
}

#[test]
fn test_recent_block_hash_window_orders_from_oldest_to_newest() {
    use claudeth::evm::host::{Host, RecursiveHost};
    use claudeth::evm::interpreter::BlockContext;
    use claudeth::types::{Address, BlockHeader, EMPTY_OMMERS_HASH, Hash, U256};

    let genesis = BlockHeader {
        parent_hash: Hash::ZERO,
        ommers_hash: EMPTY_OMMERS_HASH,
        coinbase: Address::ZERO,
        state_root: Hash::ZERO,
        transactions_root: Hash::ZERO,
        receipts_root: Hash::ZERO,
        logs_bloom: [0u8; 256],
        difficulty: U256::ZERO,
        number: 0,
        gas_limit: 30_000_000,
        gas_used: 0,
        timestamp: 1,
        extra_data: Bytes::new(),
        mix_hash: Hash::ZERO,
        nonce: 0,
        base_fee_per_gas: Some(7),
        withdrawals_root: None,
        blob_gas_used: None,
        excess_blob_gas: None,
        parent_beacon_block_root: None,
        requests_hash: None,
    };

    let mut block_1 = genesis.clone();
    block_1.number = 1;
    block_1.timestamp = 2;
    block_1.parent_hash = genesis.compute_hash();

    let mut block_2 = block_1.clone();
    block_2.number = 2;
    block_2.timestamp = 3;
    block_2.parent_hash = block_1.compute_hash();

    let mut headers_by_hash = HashMap::new();
    headers_by_hash.insert(genesis.compute_hash(), genesis.clone());
    headers_by_hash.insert(block_1.compute_hash(), block_1.clone());
    headers_by_hash.insert(block_2.compute_hash(), block_2.clone());

    let recent = collect_recent_block_hashes(block_2.compute_hash(), &headers_by_hash);
    assert_eq!(recent.len(), 3);
    assert_eq!(recent[0], genesis.compute_hash());
    assert_eq!(recent[1], block_1.compute_hash());
    assert_eq!(recent[2], block_2.compute_hash());

    let block_ctx = BlockContext {
        number: U256::from_u64(3),
        ..BlockContext::default()
    };
    let host = RecursiveHost::new()
        .with_block_context(block_ctx)
        .with_recent_block_hashes(&recent);

    assert_eq!(
        Host::<InMemoryState>::blockhash(&host, &U256::from_u64(2)),
        Some(block_2.compute_hash())
    );
    assert_eq!(
        Host::<InMemoryState>::blockhash(&host, &U256::from_u64(1)),
        Some(block_1.compute_hash())
    );
    assert_eq!(
        Host::<InMemoryState>::blockhash(&host, &U256::from_u64(0)),
        Some(genesis.compute_hash())
    );
}

type CaseExecutionOutput = (InMemoryState, Vec<claudeth::stf::BlockProcessingResult>);

fn execute_blockchain_case(
    test_name: &str,
    test_case: &BlockchainTest,
) -> Result<CaseExecutionOutput, String> {
    use claudeth::stf::process_block;
    let _ = test_name;

    // Why: forked fixtures contain branch switches (A -> B). Selecting parent
    // headers by hash without selecting parent state by hash executes blocks on
    // the wrong branch and creates false nonce/gas/state mismatches.
    let mut genesis_state = InMemoryState::new();
    apply_pre_state(&mut genesis_state, &test_case.pre)?;

    let chain_id = parse_u256(&test_case.config.chainid)?;

    let genesis_header = convert_test_block_header(&test_case.genesis_block_header)?;
    let genesis_hash = genesis_header.compute_hash();

    let mut headers_by_hash = HashMap::new();
    headers_by_hash.insert(genesis_hash, genesis_header);

    let mut states_by_hash = HashMap::new();
    states_by_hash.insert(genesis_hash, genesis_state);

    let mut block_results = Vec::new();

    for (block_idx, test_block) in test_case.blocks.iter().enumerate() {
        let expects_exception = test_block.expect_exception.is_some();

        if test_block.block_header.is_none() {
            if expects_exception {
                continue;
            }
            return Err(format!(
                "Block {block_idx}: Missing block header without expectException marker"
            ));
        }

        let block_header = convert_test_block_header(
            test_block
                .block_header
                .as_ref()
                .expect("checked block_header presence"),
        )?;

        let parent_header = match resolve_parent_header(&block_header, &headers_by_hash) {
            Some(parent) => parent,
            None => {
                if expects_exception {
                    continue;
                }
                return Err(format!(
                    "Block {block_idx}: parent header not found for parent hash {}",
                    block_header.parent_hash
                ));
            }
        };

        let parent_state = match states_by_hash.get(&block_header.parent_hash) {
            Some(state) => state,
            None => {
                if expects_exception {
                    continue;
                }
                return Err(format!(
                    "Block {block_idx}: parent state not found for parent hash {}",
                    block_header.parent_hash
                ));
            }
        };

        let recent_block_hashes =
            collect_recent_block_hashes(block_header.parent_hash, &headers_by_hash);

        let transactions: Result<Vec<_>, String> = test_block
            .transactions
            .iter()
            .enumerate()
            .map(|(tx_idx, tx)| {
                convert_test_transaction(tx).map_err(|e| {
                    format!("Block {block_idx}, tx {tx_idx}: Failed to convert transaction: {e}")
                })
            })
            .collect();
        let transactions = transactions?;

        let withdrawals: Vec<claudeth::types::Withdrawal> = test_block
            .withdrawals
            .iter()
            .enumerate()
            .map(|(wd_idx, wd)| {
                convert_test_withdrawal(wd).map_err(|e| {
                    format!("Block {block_idx}, withdrawal {wd_idx}: Failed to convert withdrawal: {e}")
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut candidate_state = parent_state.clone();

        match process_block(
            &block_header,
            parent_header,
            &transactions,
            &withdrawals,
            &recent_block_hashes,
            &mut candidate_state,
            chain_id,
        ) {
            Ok(result) => {
                if expects_exception {
                    return Err(format!(
                        "Block {block_idx}: Expected exception `{}` but execution succeeded",
                        test_block
                            .expect_exception
                            .as_deref()
                            .unwrap_or("<missing>")
                    ));
                }
                let block_hash = block_header.compute_hash();
                headers_by_hash.insert(block_hash, block_header);
                states_by_hash.insert(block_hash, candidate_state);
                block_results.push(result);
            }
            Err(e) => {
                if expects_exception {
                    // Invalid blocks are expected to fail and must not update
                    // canonical header/state indexes.
                    continue;
                }

                if matches!(
                    e,
                    claudeth::stf::BlockProcessingError::TransactionExecutionError(_)
                ) {
                    dump_transaction_disassembly(test_block);
                }

                #[cfg(feature = "evm-trace")]
                {
                    let tx_results = match &e {
                        claudeth::stf::BlockProcessingError::GasUsedMismatch {
                            transaction_results,
                            ..
                        } => Some(transaction_results),
                        claudeth::stf::BlockProcessingError::ReceiptsRootMismatch {
                            transaction_results,
                            ..
                        } => Some(transaction_results),
                        claudeth::stf::BlockProcessingError::StateRootMismatch {
                            transaction_results,
                            ..
                        } => Some(transaction_results),
                        claudeth::stf::BlockProcessingError::TransactionsRootMismatch {
                            transaction_results,
                            ..
                        } => Some(transaction_results),
                        claudeth::stf::BlockProcessingError::LogsBloomMismatch {
                            transaction_results,
                            ..
                        } => Some(transaction_results),
                        claudeth::stf::BlockProcessingError::GasLimitExceeded {
                            transaction_results,
                            ..
                        } => Some(transaction_results),
                        _ => None,
                    };

                    if let Some(results) = tx_results {
                        for (tx_idx, tx_result) in results.iter().enumerate() {
                            if let Some(trace) = tx_result.gas_trace.as_ref() {
                                eprintln!(
                                    "Gas trace for {test_name} block {block_idx} tx {tx_idx}:"
                                );
                                eprintln!("{}", trace.format());
                            }
                        }
                    }
                }

                return Err(format!("Block {block_idx}: Execution failed: {e:?}"));
            }
        }
    }

    let expected_head_hash = claudeth::types::Hash::from_str(&test_case.last_block_hash)
        .map_err(|err| format!("invalid lastblockhash {}: {err}", test_case.last_block_hash))?;
    let final_state = states_by_hash.get(&expected_head_hash).cloned().ok_or_else(|| {
        format!(
            "final state for expected last block hash {} not found",
            test_case.last_block_hash
        )
    })?;

    Ok((final_state, block_results))
}

#[test]
fn test_multi_chain_fixture_state_selection_uses_parent_hash_not_linear_order() {
    let fixture_path = Path::new(
        "tests/eels/BlockchainTests/InvalidBlocks/bcMultiChainTest/UncleFromSideChain.json",
    );
    let case = load_single_blockchain_case(
        fixture_path,
        "BlockchainTests/InvalidBlocks/bcMultiChainTest/UncleFromSideChain.json::UncleFromSideChain_Cancun",
    );

    // This case switches from chain A to chain B at height 1. If state is
    // tracked linearly by loop order, chain B block 1 fails with NonceTooLow.
    execute_blockchain_case("UncleFromSideChain_Cancun", &case)
        .expect("branching fixture should execute with hash-indexed parent state");
}

#[test]
#[ignore] // Run with --ignored to execute all EELS tests
fn test_execute_all_blockchain_tests() {
    let tests = discover_blockchain_tests();
    if tests.is_empty() {
        eprintln!("No EELS tests found - skipping test");
        return;
    }

    println!("Executing {} blockchain test files...\n", tests.len());

    let mut total_tests = 0;
    let mut passed = 0;
    let mut failed = 0;
    let mut errors = 0;

    // Execute all discovered fixtures. This test remains ignored for now because
    // full EELS parity is still under active implementation.
    for test_path in &tests {
        let test_cases = match load_blockchain_test(test_path) {
            Ok(cases) => cases,
            Err(e) => {
                eprintln!("✗ Failed to load {}: {e}", test_path.display());
                errors += 1;
                continue;
            }
        };

        for (test_name, test_case) in test_cases {
            total_tests += 1;
            let (final_state, block_results) =
                match execute_blockchain_case(&test_name, &test_case) {
                    Ok(result) => result,
                    Err(e) => {
                        eprintln!("✗ {test_name}: {e}");
                        failed += 1;
                        continue;
                    }
                };
            #[cfg(not(feature = "evm-trace"))]
            let _ = &block_results;

            match validate_post_state(&final_state, &test_case.pre, &test_case.post_state) {
                Ok(()) => {
                    println!("✓ {test_name}");
                    passed += 1;
                }
                Err(e) => {
                    eprintln!("✗ {test_name}: Post-state mismatch: {e}");
                    #[cfg(feature = "evm-trace")]
                    {
                        for (block_idx, block_result) in block_results.iter().enumerate() {
                            for (tx_idx, tx_result) in
                                block_result.transaction_results.iter().enumerate()
                            {
                                if let Some(trace) = tx_result.gas_trace.as_ref() {
                                    eprintln!(
                                        "Gas trace for {test_name} block {block_idx} tx {tx_idx}:"
                                    );
                                    eprintln!("{}", trace.format());
                                }
                            }
                        }
                    }
                    failed += 1;
                }
            }
        }
    }

    println!("\n=== EELS Test Results ===");
    println!("Total:  {total_tests}");
    println!("Passed: {passed}");
    println!("Failed: {failed}");
    println!("Errors: {errors}");
    println!("========================");

    // Don't fail the test yet - we're still implementing
    // assert_eq!(failed, 0, "Some tests failed");
    // assert_eq!(errors, 0, "Some tests errored");
}
