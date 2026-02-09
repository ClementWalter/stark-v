//! Test storage root with single key

use claudeth::state::Storage;
use claudeth::types::U256;

#[test]
fn test_single_key_storage_root() {
    let mut storage = Storage::new();

    // Set just the first key
    storage.set(&U256::from(0x079e_u64), U256::from(0x079e_u64));

    let root = storage.compute_root();
    println!("Root with key 0x079e = 0x079e: 0x{}", hex::encode(root.as_bytes()));

    // Check if this matches the constant we're seeing
    let observed_root_hex = "2f1228a30a70c1ee01e084800b776ce75558b8716098d852f80b6205708e9e23";
    let observed_root_bytes = hex::decode(observed_root_hex).unwrap();
    println!("Observed root in tests:           0x{observed_root_hex}");
    println!("Match: {}", root.as_bytes() == observed_root_bytes.as_slice());
}
