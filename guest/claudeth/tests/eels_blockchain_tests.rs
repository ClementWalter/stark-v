// EELS Blockchain Test Integration
//
// This module integrates ethereum/tests BlockchainTests to verify claudeth's
// compliance with the Ethereum execution specification.
//
// Test fixtures are loaded from tests/eels/BlockchainTests/ (not checked into git).
// Run scripts/fetch_eels_tests.py to download the test fixtures.

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
    withdrawals: Vec<Value>,
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
struct TestAccount {
    balance: String,
    code: String,
    nonce: String,
    storage: HashMap<String, String>,
}

/// Load a single blockchain test from a JSON file
fn load_blockchain_test(path: &Path) -> Result<HashMap<String, BlockchainTest>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let tests: HashMap<String, BlockchainTest> = serde_json::from_str(&content)?;
    Ok(tests)
}

/// Discover all blockchain test files in the tests/eels directory
fn discover_blockchain_tests() -> Vec<std::path::PathBuf> {
    let test_dir = Path::new("tests/eels/BlockchainTests");
    if !test_dir.exists() {
        eprintln!("Warning: EELS test directory not found at {}", test_dir.display());
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
            // Skip InvalidBlocks for initial integration (focus on valid blocks first)
            if entry.path().to_string_lossy().contains("InvalidBlocks") {
                continue;
            }
            tests.push(entry.path().to_path_buf());
        }
    }
    tests
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

fn apply_pre_state(state: &mut InMemoryState, pre: &HashMap<String, TestAccount>) -> Result<(), String> {
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

    for test_path in tests.iter().take(10) {
        // Parse first 10 tests
        match load_blockchain_test(test_path) {
            Ok(test_cases) => {
                for test in test_cases.values() {
                    if !pre_state_parsed {
                        let mut state = InMemoryState::new();
                        apply_pre_state(&mut state, &test.pre)
                            .unwrap_or_else(|err| panic!("failed to parse pre-state: {err}"));
                        pre_state_parsed = true;
                    }
                }

                parsed += test_cases.len();
                let num_cases = test_cases.len();
                println!("✓ Parsed {num_cases} test cases from {}", test_path.display());
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
}

#[test]
#[ignore] // Run with --ignored to execute all EELS tests
fn test_execute_all_blockchain_tests() {
    let tests = discover_blockchain_tests();
    if tests.is_empty() {
        eprintln!("No EELS tests found - skipping test");
        return;
    }

    println!("Executing {} blockchain test files...", tests.len());

    // TODO: Implement test execution
    // For each test:
    // 1. Parse test JSON
    // 2. Set up initial state from `pre`
    // 3. Execute blocks sequentially
    // 4. Validate final state against `postState`
    // 5. Validate block hashes match expected values

    todo!("Implement EELS test execution (Phase D Task D2)");
}
