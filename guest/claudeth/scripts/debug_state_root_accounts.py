#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""
Create a Rust test that will print all accounts included in state root computation.
"""

test_code = '''
#[test]
fn debug_shanghai_example_state() {
    use crate::state::{InMemoryState, State};
    use crate::types::{Address, U256};
    use crate::stf::process_block;

    // Load shanghaiExample test
    let json = include_str!("eels/BlockchainTests/ValidBlocks/bcExample/shanghaiExample.json");
    let data: serde_json::Value = serde_json::from_str(json).unwrap();
    let test_key = "BlockchainTests/ValidBlocks/bcExample/shanghaiExample.json::shanghaiExample_Prague";
    let test = &data[test_key];

    // Initialize state from pre
    let mut state = InMemoryState::new();
    for (addr_str, acc) in test["pre"].as_object().unwrap() {
        let addr = Address::from_hex(addr_str).unwrap();
        // ... load balance, nonce, code, storage
    }

    // Execute block
    // ...

    // Print all accounts in state before computing root
    println!("=== Accounts in InMemoryState ===");
    for (addr, acc) in state.accounts.iter() {
        println!("{:?}: nonce={}, balance={}, code_len={}, storage_root={:?}",
            addr, acc.nonce, acc.balance,
            state.code.get(addr).map(|c| c.len()).unwrap_or(0),
            acc.storage_root
        );
    }

    // Compute state root
    let root = state.compute_state_root();
    println!("Computed state root: {:?}", root);
}
'''

print("Add this test to tests/eels_blockchain_tests.rs to debug which accounts")
print("are included in the state root computation:")
print()
print(test_code)
print()
print("Or better yet, add debug println! statements directly to compute_state_root()")
print("in src/state/execution.rs around line 365-378 to see what's being included.")
