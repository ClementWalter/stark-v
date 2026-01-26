#![cfg(feature = "revm")]

use serde::{Deserialize, Serialize};

use prover::e2e::run_guest_raw_with_features;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct RevmSmokeResult {
    status: u8,
    gas_used: u64,
    return_len: u32,
    return_last: u8,
}

#[test]
fn test_revm_smoke_guest() {
    let bytes = run_guest_raw_with_features("revm_smoke", &["revm"]);
    let result: RevmSmokeResult =
        postcard::from_bytes(&bytes).expect("Failed to decode revm smoke result");

    assert_eq!(result.status, 0);
    assert_eq!(result.return_len, 32);
    assert_eq!(result.return_last, 0x2a);
}
