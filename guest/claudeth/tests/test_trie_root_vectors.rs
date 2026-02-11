//! Consensus vector tests for trie roots.
//!
//! These tests pin canonical Ethereum roots so regressions in trie encoding or
//! constants are caught immediately.

use claudeth::crypto::encode_u256;
use claudeth::state::{EMPTY_TRIE_ROOT, Trie};
use claudeth::types::{Address, Hash, U256, Withdrawal};

#[test]
fn test_empty_trie_root_matches_execution_specs_constant() {
    // Why: this is a consensus constant used across header and trie validation,
    // so even a one-byte drift must fail loudly.
    let expected = Hash::from([
        0x56, 0xe8, 0x1f, 0x17, 0x1b, 0xcc, 0x55, 0xa6, 0xff, 0x83, 0x45, 0xe6, 0x92, 0xc0, 0xf8,
        0x6e, 0x5b, 0x48, 0xe0, 0x1b, 0x99, 0x6c, 0xad, 0xc0, 0x01, 0x62, 0x2f, 0xb5, 0xe3, 0x63,
        0xb4, 0x21,
    ]);

    assert_eq!(EMPTY_TRIE_ROOT, expected);
}

#[test]
fn test_withdrawals_root_matches_shanghai_fixture_vector() {
    // Why: this fixture vector from EELS verifies key indexing + withdrawal
    // RLP encoding against a known Shanghai/Cancun withdrawalsRoot.
    let withdrawals = vec![Withdrawal {
        index: 0,
        validator_index: 0,
        address: Address::from([
            0xc9, 0x4f, 0x53, 0x74, 0xfc, 0xe5, 0xed, 0xbc, 0x8e, 0x2a, 0x86, 0x97, 0xc1, 0x53,
            0x31, 0x67, 0x7e, 0x6e, 0xbf, 0x0b,
        ]),
        amount_gwei: 10_000,
    }];

    let mut trie = Trie::new();
    for (index, withdrawal) in withdrawals.iter().enumerate() {
        let key = encode_u256(&U256::from(index as u64));
        trie.insert(&key, withdrawal.encode_rlp());
    }

    let expected = Hash::from([
        0x27, 0xf1, 0x66, 0xf1, 0xd7, 0xc7, 0x89, 0x25, 0x12, 0x99, 0x53, 0x5c, 0xb1, 0x76, 0xba,
        0x34, 0x11, 0x6e, 0x44, 0x89, 0x44, 0x76, 0xa7, 0x88, 0x6f, 0xe5, 0xd7, 0x3d, 0x9b, 0xe5,
        0xc9, 0x73,
    ]);

    assert_eq!(trie.compute_root(), expected);
}
