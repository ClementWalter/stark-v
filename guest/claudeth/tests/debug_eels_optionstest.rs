//! Debug test to understand optionsTest_Prague state persistence issue

use claudeth::stf::process_block;
use claudeth::state::{InMemoryState, State};
use claudeth::types::{Address, BlockHeader, Transaction, LegacyTransaction, U256, Bytes};

#[test]
fn test_debug_optionstest_prague() {
    // Recreate optionsTest_Prague scenario

    let mut state = InMemoryState::new();

    // Setup pre-state for contract
    let contract_addr = Address::from_hex("0xb94f5374fce5edbc8e2a8697c15331677e6ebf0b").unwrap();
    let contract_code = hex::decode("60016000355500").unwrap();
    state.set_balance(&contract_addr, U256::from_hex("0x016345785d8a0000").unwrap());
    state.set_code(&contract_addr, contract_code);
    state.set_nonce(&contract_addr, U256::ZERO);

    // Setup pre-state for sender
    let sender_addr = Address::from_hex("0xa94f5374fce5edbc8e2a8697c15331677e6ebf0b").unwrap();
    state.set_balance(&sender_addr, U256::from_hex("0x016345785d8a0000").unwrap());
    state.set_nonce(&sender_addr, U256::ZERO);

    println!("Pre-state:");
    println!("  Contract balance: {}", state.get_balance(&contract_addr));
    println!("  Contract code: {} bytes", state.get_code(&contract_addr).len());
    println!("  Sender balance: {}", state.get_balance(&sender_addr));
    println!("  Sender nonce: {}", state.get_nonce(&sender_addr));

    // Create transaction
    let tx = Transaction::Legacy(LegacyTransaction {
        nonce: U256::ZERO,
        gas_price: U256::from_hex("0xa0").unwrap(),
        gas_limit: U256::from_hex("0x061a80").unwrap(),
        to: Some(contract_addr),
        value: U256::ZERO,
        data: Bytes::from_hex("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap(),
        v: U256::from_hex("0x1b").unwrap(),
        r: U256::from_hex("0x31baec82258594305b86198c118a42a4e854a0bc058cb1193382af1f24cae0f5").unwrap(),
        s: U256::from_hex("0x235e43030727941d615c0d62b8bef511795b1f3390d0d4596f74933f4ca6cb3e").unwrap(),
    });

    // Create dummy block headers (we'll skip parent validation)
    let parent_header = BlockHeader {
        parent_hash: Default::default(),
        ommers_hash: Default::default(),
        coinbase: Address::ZERO,
        state_root: Default::default(),
        transactions_root: Default::default(),
        receipts_root: Default::default(),
        logs_bloom: [0u8; 256],
        difficulty: U256::ZERO,
        number: 0,
        gas_limit: 10_000_000,
        gas_used: 0,
        timestamp: 0,
        extra_data: Bytes::from(vec![]),
        mix_hash: Default::default(),
        nonce: 0,
        base_fee_per_gas: Some(0),
        withdrawals_root: None,
        blob_gas_used: None,
        excess_blob_gas: None,
        parent_beacon_block_root: None,
    };

    let block_header = BlockHeader {
        parent_hash: parent_header.compute_hash(),
        ommers_hash: Default::default(),
        coinbase: Address::ZERO,
        state_root: Default::default(),
        transactions_root: Default::default(),
        receipts_root: Default::default(),
        logs_bloom: [0u8; 256],
        difficulty: U256::ZERO,
        number: 1,
        gas_limit: 10_000_000,
        gas_used: 21000, // Approximate
        timestamp: parent_header.timestamp + 1,
        extra_data: Bytes::from(vec![]),
        mix_hash: Default::default(),
        nonce: 0,
        base_fee_per_gas: Some(0),
        withdrawals_root: None,
        blob_gas_used: None,
        excess_blob_gas: None,
        parent_beacon_block_root: None,
    };

    let chain_id = U256::ONE;

    // Execute block
    println!("\nExecuting block with 1 transaction...");
    let result = process_block(&block_header, &parent_header, &[tx], &mut state, chain_id, &[]);

    match result {
        Ok(res) => {
            println!("Block execution succeeded!");
            println!("  Gas used: {}", res.gas_used);
            println!("  Transactions: {}", res.transaction_results.len());
            for (i, tx_result) in res.transaction_results.iter().enumerate() {
                println!("  TX {}: success={}, gas={}", i, tx_result.success, tx_result.gas_used);
            }
        },
        Err(e) => {
            println!("Block execution FAILED: {e:?}");
        }
    }

    // Check post-state
    println!("\nPost-state:");
    println!("  Contract balance: {}", state.get_balance(&contract_addr));
    println!("  Sender balance: {}", state.get_balance(&sender_addr));
    println!("  Sender nonce: {}", state.get_nonce(&sender_addr));

    // Check storage
    println!("\nStorage at key 0x01: {}", state.sload(&contract_addr, &U256::ONE));
    println!("Storage at key 0x02: {}", state.sload(&contract_addr, &U256::from_u64(2)));

    // The storage at key 0x01 should be 1
    assert_eq!(state.sload(&contract_addr, &U256::ONE), U256::ONE, "Storage[1] should be 1");
}

// Helper trait for from_hex
trait FromHex: Sized {
    fn from_hex(s: &str) -> Result<Self, String>;
}

impl FromHex for Address {
    fn from_hex(s: &str) -> Result<Self, String> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s).map_err(|e| format!("{e}"))?;
        if bytes.len() != 20 {
            return Err(format!("Expected 20 bytes, got {}", bytes.len()));
        }
        let mut addr = Address::ZERO;
        addr.as_bytes_mut().copy_from_slice(&bytes);
        Ok(addr)
    }
}

impl FromHex for U256 {
    fn from_hex(s: &str) -> Result<Self, String> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        // Pad to 64 hex chars (32 bytes)
        let padded = format!("{s:0>64}");
        let bytes = hex::decode(&padded).map_err(|e| format!("{e}"))?;
        Ok(U256::from_be_bytes(bytes.try_into().unwrap()))
    }
}

impl FromHex for Bytes {
    fn from_hex(s: &str) -> Result<Self, String> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s).map_err(|e| format!("{e}"))?;
        Ok(Bytes::from(bytes))
    }
}
