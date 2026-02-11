//! Test Storage trie directly

use claudeth::state::Storage;
use claudeth::types::U256;

#[test]
fn test_storage_set_and_get() {
    let mut storage = Storage::new();

    // Set a value
    storage.set(&U256::from(0x079e_u64), U256::from(0x079e_u64));

    // Get it back
    let value = storage.get(&U256::from(0x079e_u64));
    assert_eq!(
        value,
        U256::from(0x079e_u64),
        "Should get back the value we set"
    );

    // Check root is not empty
    assert!(
        !storage.is_empty(),
        "Storage should not be empty after setting a value"
    );
    let root = storage.compute_root();
    println!(
        "Storage root after setting one value: 0x{}",
        hex::encode(root.as_bytes())
    );

    // Set more values
    storage.set(&U256::from(0x0b86_u64), U256::from(0x0b86_u64));
    storage.set(&U256::from(0x0f6e_u64), U256::from(0x0f6e_u64));
    storage.set(&U256::from(0x1356_u64), U256::from(0x1356_u64));

    // Get them all back
    assert_eq!(storage.get(&U256::from(0x079e_u64)), U256::from(0x079e_u64));
    assert_eq!(storage.get(&U256::from(0x0b86_u64)), U256::from(0x0b86_u64));
    assert_eq!(storage.get(&U256::from(0x0f6e_u64)), U256::from(0x0f6e_u64));
    assert_eq!(storage.get(&U256::from(0x1356_u64)), U256::from(0x1356_u64));

    let final_root = storage.compute_root();
    println!(
        "Storage root after setting four values: 0x{}",
        hex::encode(final_root.as_bytes())
    );
}
