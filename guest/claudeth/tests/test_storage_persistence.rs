//! Test storage persistence issue

use claudeth::state::{InMemoryState, State};
use claudeth::types::{Address, U256};

#[test]
fn test_storage_persists_after_sstore() {
    let mut state = InMemoryState::new();

    let addr = Address::from([0x0f; 20]);

    // Set some storage values like in EELS pre-state
    state.sstore(&addr, &U256::from(0x079e_u64), U256::from(0x079e_u64));
    state.sstore(&addr, &U256::from(0x0b86_u64), U256::from(0x0b86_u64));
    state.sstore(&addr, &U256::from(0x0f6e_u64), U256::from(0x0f6e_u64));
    state.sstore(&addr, &U256::from(0x1356_u64), U256::from(0x1356_u64));

    // Check we can read them back
    assert_eq!(
        state.sload(&addr, &U256::from(0x079e_u64)),
        U256::from(0x079e_u64),
        "Storage[0x079e] should be 0x079e"
    );
    assert_eq!(
        state.sload(&addr, &U256::from(0x0b86_u64)),
        U256::from(0x0b86_u64),
        "Storage[0x0b86] should be 0x0b86"
    );
    assert_eq!(
        state.sload(&addr, &U256::from(0x0f6e_u64)),
        U256::from(0x0f6e_u64),
        "Storage[0x0f6e] should be 0x0f6e"
    );
    assert_eq!(
        state.sload(&addr, &U256::from(0x1356_u64)),
        U256::from(0x1356_u64),
        "Storage[0x1356] should be 0x1356"
    );

    // Compute state root (this is where the bug might manifest)
    let _root = state.compute_state_root();

    // Check storage is still readable after computing state root
    assert_eq!(
        state.sload(&addr, &U256::from(0x079e_u64)),
        U256::from(0x079e_u64),
        "Storage[0x079e] should still be 0x079e after compute_state_root"
    );
    assert_eq!(
        state.sload(&addr, &U256::from(0x0b86_u64)),
        U256::from(0x0b86_u64),
        "Storage[0x0b86] should still be 0x0b86 after compute_state_root"
    );
}
