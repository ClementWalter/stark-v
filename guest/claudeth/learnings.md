# Claudeth Development Learnings

Date: 2026-02-10

## Consensus-Critical Behavior

- Exceptional halts (OOG, InvalidJump, InvalidOpcode) fail only the current
  transaction: consume all gas and revert that transaction’s state changes.
- `REVERT` is non-exceptional: preserve remaining gas and revert only the
  current call frame.
- Gas refunds are capped at 1/5 of total gas used (EIP-3529) in
  `stf::executor`.

## Block Processing Order

- Validate the header before executing transactions.
- Apply EIP-4788 (beacon root) and EIP-2935 (history storage) system calls
  before executing transactions and before computing the state root.
- Root checks are post-execution: receipts root, transactions root, logs bloom,
  withdrawals root (if present), state root, and gas-used vs header.
- Logs bloom bit ordering must follow execution-specs: reverse the 11-bit index
  (bit_index = 0x07FF - bit_to_set) and set bits MSB-first within each byte.

## Header Validation

- Post-merge headers must have `difficulty == 0`, `mix_hash == 0`, and
  `nonce == 0`, with empty ommers hash.
- `extra_data` length is capped at 32 bytes and `gas_used <= gas_limit`.
- Base fee per gas is derived from the parent’s gas used vs target (EIP-1559).
- Blob fields are all-or-nothing: if either `blob_gas_used` or
  `excess_blob_gas` is present, both are required, and `excess_blob_gas` must
  match the parent-derived value.

## Guest Input Decoding

- Input is an RLP list of 5–7 items:
  `block_header`, `parent_header`, `chain_id`, `transactions`, `state_entries`,
  optional `block_hashes`, optional `withdrawals`.
- A withdrawals list must be provided iff `withdrawals_root` is present in the
  header; an empty list is valid.
- Recent block hashes are capped at 256 entries, are ordered oldest → newest,
  and when provided the last entry must equal `parent.compute_hash()`.
- For genesis (`block.number == 0`), a recent block hashes list is invalid.

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

## Blob Gas Accounting (EIP-4844)

- Blob gas used per tx is `GAS_PER_BLOB * blob_count`.
- Cancun max blob gas per block is `786_432` (6 blobs * 131_072).
- Blob data fee is `blob_gas_used * blob_base_fee` and is charged upfront from
  the sender (burned, not credited to coinbase).
- Block processing enforces the max blob gas per block and validates
  `header.blob_gas_used` against the computed total.

## State / Trie

- Use `EMPTY_TRIE_ROOT` for empty tries (never `Hash::ZERO`).
- State root must be deterministic: sort account addresses before inserting into
  the state trie.
- State trie keys are `keccak256(address)` (not raw 20-byte address).

## Module Architecture

- There are three `EvmError` enums; opcode-local errors must convert into
  `evm::error::EvmError`.
- `evm/mod.rs` re-exports from `interpreter`, not `error`.
- `compute_create_address` and `compute_create2_address` must stay `pub` for
  opcode access.

## Pre-commit Hygiene

- The no-orphan Rust files hook fails if any `src/*.rs` file is unreachable.
- Always run `cargo test -p claudeth --release` and `prek run` before
  committing.

## Do / Don't (Next Iteration)

**Do**

- Keep EIP-4788 and EIP-2935 system calls before transaction execution.
- Ensure `TxContext.blob_versioned_hashes` is set so `BLOBHASH` returns data for
  blob txs.
- Keep blob data fee charged upfront and burned (not credited to coinbase).
- Validate `blob_gas_used` and enforce the Cancun max blob gas per block.
- Sort addresses before computing the state root and use `keccak256(address)`
  as the trie key.
- Enforce recent block hashes list length ≤ min(block number, 256) and require
  the last hash to match the parent hash when provided.
- Validate base fee caps per transaction type before charging balance.
- Use execution-specs logs bloom bit ordering (reversed 11-bit index, MSB-first).

**Don't**

- Cap memory expansion gas.
- Treat EVM `REVERT` as exceptional.
- Leave unused `src/*.rs` files (pre-commit will fail).
- Accept recent block hash lists for genesis or lists with a mismatched parent.
