# Claudeth Development Learnings

Date: 2026-02-09

## Consensus-Critical Behavior

- Exceptional halts (OOG, InvalidJump, InvalidOpcode) fail the current
  transaction: consume all gas and revert only that transaction’s state.
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

## Header Validation

- Post-merge headers must have `difficulty == 0`, `mix_hash == 0`, and
  `nonce == 0`, with empty ommers hash.
- `extra_data` length is capped at 32 bytes and `gas_used <= gas_limit`.
- Base fee per gas is derived from the parent’s gas used vs target (EIP-1559).
- Blob fields must be all-or-nothing: if either `blob_gas_used` or
  `excess_blob_gas` is present, both are required, and `excess_blob_gas` must
  match the parent-derived value.

## Guest Input Decoding

- Input is an RLP list of 5–7 items:
  `block_header`, `parent_header`, `chain_id`, `transactions`, `state_entries`,
  optional `block_hashes`, optional `withdrawals`.
- If `withdrawals_root` is present, a withdrawals list must be provided.
- If `withdrawals_root` is absent, the withdrawals list must be empty.
- Recent block hashes must be ordered oldest → newest and capped at 256 entries.

## Transactions and Context

- Typed transaction decoding accepts type `0x01`, `0x02`, and `0x03`.
- `Transaction::effective_gas_price` is min(`max_fee_per_gas`,
  `base_fee + max_priority_fee_per_gas`) for EIP-1559 and blob txs.
- Blob tx validation enforces non-empty blob hashes, KZG version byte `0x01`,
  blob count limit, and `max_fee_per_blob_gas >= blob_base_fee`.
- `TxContext` carries `blob_versioned_hashes`; `RecursiveHost::blobhash` reads
  from it and returns zero for out-of-range indices.
- Blob transaction encoding/decoding uses type prefix `0x03` and requires a
  20-byte `to` address (no contract creation).
- Receipt encoding for blob txs uses prefix `0x03`.

## Blob Base Fee

- `BLOBBASEFEE` uses the execution-specs Taylor expansion function over
  `excess_blob_gas`.
- If `excess_blob_gas` is absent (pre-Cancun), `BLOBBASEFEE` returns zero.

## State / Trie

- Use `EMPTY_TRIE_ROOT` for empty tries (never `Hash::ZERO`).
- Keep state root deterministic by inserting accounts in stable address order.

## Module Architecture

- There are three `EvmError` enums; opcode-local errors must convert into
  `evm::error::EvmError`.
- `evm/mod.rs` re-exports from `interpreter`, not `error`.
- `compute_create_address` and `compute_create2_address` must stay `pub` for
  opcode access.

## Pre-commit Hygiene

- The no-orphan Rust files hook fails if any `src/*.rs` file is unreachable.
- Always run `prek run` before committing; fix linting errors.

## Do / Don't (Next Iteration)

**Do**

- Run `cargo test -p claudeth --release` and `prek run` before committing.
- Provide recent block hashes in guest input for correct `BLOCKHASH` results.
- Keep EIP-4788 and EIP-2935 system calls before transaction execution.
- Set `TxContext.blob_versioned_hashes` so `BLOBHASH` returns data for blob txs.
- Enforce blob tx hash list/fee validation before execution.
- When adding blob gas accounting, track `blob_gas_used` and enforce block limits.
- Prefix blob transaction receipts with type `0x03`.

**Don't**

- Cap memory expansion gas.
- Treat EVM `REVERT` as exceptional.
- Leave unused `src/*.rs` files (pre-commit will fail).
- Quote EELS test counts without rerunning.
