# Witness Format (Draft v1)

This document defines the witness payload used to reconstruct minimal state
from Merkle Patricia Trie proofs. The design follows Ethereum Cancun state and
trie semantics in `execution-specs` and matches Claudeth’s partial MPT proof
verification (`state::partial_mpt::proof::verify_proof`).

## RLP Structure

```
WitnessV1 = RLP([
  version_u64,            // must be 1
  state_root_hash,        // 32 bytes
  accounts               // list[AccountWitness]
])

AccountWitness = RLP([
  address,               // 20 bytes
  account_proof_nodes,   // list[bytes], each entry is RLP-encoded MPT node
  account_rlp,           // bytes: RLP([nonce, balance, storage_root, code_hash])
  code_bytes,            // bytes (empty for EOAs)
  storage_entries        // list[StorageWitness]
])

StorageWitness = RLP([
  slot_key_u256,         // U256 (preimage)
  slot_value_u256,       // U256 (0 means “absent”)
  storage_proof_nodes    // list[bytes], each entry is RLP-encoded MPT node
])
```

## Validation Rules

- `version_u64` must be `1`.
- `accounts` must be sorted by `address` ascending with no duplicates.
- `account_proof_nodes` is a list of RLP-encoded nodes from root to target.
- The account trie key is `keccak256(address)`.
- Account proof verification:
  - If `account_rlp` is empty, the proof must be a valid **exclusion** proof
    for the account key under `state_root_hash`.
  - If `account_rlp` is non-empty, the proof must be a valid **inclusion**
    proof for the account key and `account_rlp` under `state_root_hash`.
- If `account_rlp` is empty:
  - `code_bytes` must be empty.
  - `storage_entries` must be empty.
- If `account_rlp` is present, decode it as:
  `Account = [nonce, balance, storage_root, code_hash]`.
- `code_hash` must equal `keccak256(code_bytes)`.
  - If `code_hash == EMPTY_CODE_HASH`, then `code_bytes` must be empty.
- `storage_entries` must be sorted by `slot_key_u256` ascending with no
  duplicates.
- Storage trie key is `keccak256(slot_key_u256.to_be_bytes())`.
- Storage proof verification uses `storage_root` from the account:
  - If `slot_value_u256 == 0`, the proof must be an **exclusion** proof.
  - If `slot_value_u256 != 0`, the proof must be an **inclusion** proof whose
    value equals `rlp::encode_u256(slot_value_u256)`.
- For `storage_root == EMPTY_TRIE_ROOT`, only exclusion proofs with empty
  proof nodes are valid (matching `verify_proof`).

## Notes

- Proof verification should use `state::partial_mpt::proof::verify_proof` with
  the RLP-encoded value bytes (or `None` for exclusion).
- This format is independent of the current guest input format; it is intended
  for the witness-based state reconstruction milestone in `PLAN.md`.
