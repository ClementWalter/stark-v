# Claudeth Development Learnings

Date: 2026-02-10

## Consensus-Critical EVM Behavior

- Exceptional halts (OOG, InvalidJump, InvalidOpcode) fail only the current
  transaction: consume all gas and revert that transaction’s state changes.
- `REVERT` is non-exceptional: preserve remaining gas and revert only the
  current call frame.
- Gas refunds are capped at 1/5 of total gas used (EIP-3529); SSTORE clearing
  refunds are 4800 gas in `stf::executor`.

## Block Processing Order

- Validate the header before executing transactions.
- Apply EIP-4788 (beacon root) and EIP-2935 (history storage) system calls
  before executing transactions and before computing the post-state root.
- Root checks are post-execution: receipts root, transactions root, logs bloom,
  withdrawals root (if present), state root, gas used, and blob gas used.

## Header Validation

- Post-merge headers must have `difficulty == 0`, `mix_hash == 0`, `nonce == 0`,
  and `ommers_hash == EMPTY_OMMERS_HASH`.
- `extra_data` length is capped at 32 bytes and `gas_used <= gas_limit`.
- Base fee per gas is derived from the parent’s gas used vs target (EIP-1559).
- Blob fields are all-or-nothing: if either `blob_gas_used` or
  `excess_blob_gas` is present, both are required, and `excess_blob_gas` must
  match the parent-derived value.

## Guest Input (Current)

- Input is an RLP list of 5–7 items:
  `block_header`, `parent_header`, `chain_id`, `transactions`,
  `state_entries` or `witness`, optional `block_hashes`, optional `withdrawals`.
- State entries are `[address, nonce, balance, code_bytes, storage_entries]`.
  Storage entries are `[key_u256, value_u256]`.
- A withdrawals list must be provided iff `withdrawals_root` is present in the
  header; an empty list is valid.
- Recent block hashes are capped at 256 entries, ordered oldest → newest, and
  when provided the last entry must equal `parent.compute_hash()`.
- For genesis (`block.number == 0`), a recent block hashes list is invalid.

## Witness Validation (WITNESS v1)

- Witness entries are validated against a `state_root` using MPT proofs.
- Accounts must be sorted by ascending address with no duplicates.
- Account trie keys are `keccak256(address)`.
- If `account_rlp` is empty, the proof must be an exclusion proof and
  `code_bytes` + `storage_entries` must be empty.
- `code_hash` must equal `keccak256(code_bytes)` (and empty code requires the
  empty code hash).
- Storage entries must be sorted by ascending slot with no duplicates.
- Storage trie keys are `keccak256(U256 slot)`; inclusion proofs use
  `rlp::encode_u256(value)` and zero values require exclusion proofs.

## Trie / State

- State trie keys are `keccak256(address)` and addresses are sorted before
  insertion to keep the root deterministic.
- Storage trie keys are `keccak256(U256 slot)` and zero values delete the key.
- Empty trie root is `keccak256(rlp([]))` (`EMPTY_TRIE_ROOT`).
- Partial MPT proof verification uses RLP-encoded nodes and `verify_proof`.

## Transactions and Context

- Typed transaction decoding accepts type `0x01`, `0x02`, and `0x03`.
- `Transaction::effective_gas_price` is
  min(`max_fee_per_gas`, `base_fee + max_priority_fee_per_gas`) for EIP-1559
  and blob txs.
- Enforce base fee caps: legacy/EIP-2930 require `gas_price >= base_fee`,
  EIP-1559/EIP-4844 require `max_fee_per_gas >= base_fee`.
- Blob tx validation enforces non-empty blob hashes, KZG version byte `0x01`,
  blob count limit, and `max_fee_per_blob_gas >= blob_base_fee`.
- Blob transactions require a 20-byte `to` address (no contract creation).
- `TxContext` carries `blob_versioned_hashes`; `RecursiveHost::blobhash` reads
  from it and returns zero for out-of-range indices.
- Receipt encoding for blob txs uses prefix `0x03`.
- Signature recovery enforces EIP-2 bounds: `0 < r < SECP256K1N`,
  `0 < s <= SECP256K1N/2`, and valid `v/y_parity` per transaction type.

## Logs Bloom

- Logs bloom bit ordering follows execution-specs: reverse the 11-bit index
  (bit_index = 0x07FF - bit_to_set) and set bits MSB-first within each byte.

## Pre-commit Hygiene

- The no-orphan Rust files hook fails if any `src/*.rs` file is unreachable.
- Always run `cargo test -p claudeth --release` and `prek run` before
  committing.

## Do / Don't (Next Iteration)

**Do**

- Keep EIP-4788 and EIP-2935 system calls before transaction execution.
- Ensure `TxContext.blob_versioned_hashes` is set so `BLOBHASH` returns data
  for blob txs.
- Keep blob data fee charged upfront and burned (not credited to coinbase).
- Validate `blob_gas_used` and enforce the Cancun max blob gas per block.
- Sort addresses before computing the state root and use `keccak256(address)`
  as the trie key.
- Enforce witness ordering (addresses and storage slots) and proof validation.
- Validate base fee caps per transaction type before charging balance.
- Use execution-specs logs bloom bit ordering (reversed 11-bit index,
  MSB-first).
- Enforce EIP-2 signature bounds and `v/y_parity` constraints during sender
  recovery.

**Don't**

- Cap memory expansion gas.
- Treat EVM `REVERT` as exceptional.
- Leave unused `src/*.rs` files (pre-commit will fail).
- Accept recent block hash lists for genesis or lists with a mismatched parent.
- Accept witness accounts with mismatched code hashes or unsorted entries.
