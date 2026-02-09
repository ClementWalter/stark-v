# Claudeth Development Learnings

Date: 2026-02-09

## Consensus-Critical Execution

- Exceptional halts (OOG, InvalidJump, InvalidOpcode, etc.) are transaction-level failures: consume all gas and revert only that transaction’s state.
- `REVERT` is not exceptional: preserve remaining gas and revert only the current call frame.

## Block Processing Order

- Run header validation before state transition work (gas fields, extra data, post-merge fields), then validate against the parent header.
- Apply EIP-4788 (beacon root) and EIP-2935 (history storage) system calls **before** executing transactions and before computing the state root.
- Root checks are post-execution: receipts root, transactions root, logs bloom, withdrawals root (if present), state root, and gas-used vs header.

## Header Validation

- Post-merge headers must have `difficulty == 0`, `mix_hash == 0`, and `nonce == 0`.
- Extra data length is capped at 32 bytes; gas used must not exceed gas limit.
- Ommers hash is expected to be the empty ommers hash in post-merge blocks.
- Base fee per gas is derived from the parent gas used vs target (EIP-1559): tests that expect unchanged base fee should set parent `gas_used == gas_limit / 2`.
- Excess blob gas is computed from `parent.excess_blob_gas + parent.blob_gas_used`, floored at 0 if below the Cancun target (393,216).

## Guest Input Decoding

- Input is an RLP list of 5–7 items:
  - `block_header`, `parent_header`, `chain_id`, `transactions`, `state_entries`
  - Optional `block_hashes` (oldest -> newest, max 256)
  - Optional `withdrawals` (required when `withdrawals_root` is present)
- If `withdrawals_root` is present, a withdrawals list must be provided.
- If `withdrawals_root` is absent, the withdrawals list must be empty.

## System Calls

- EIP-4788 runs only when `parent_beacon_block_root` is present; use `SYSTEM_ADDRESS` and treat missing code at the beacon roots address as a no-op.
- EIP-2935 runs every block; call the history storage contract with the **parent block hash** as 32-byte calldata. It stores at `(block.number - 1) % 8191`, so run it before computing the state root.

## BLOCKHASH Data

- `BLOCKHASH` accuracy depends on providing recent hashes in guest input.
- Without recent hashes, only the parent hash can be returned.
- Recent hashes must be ordered by increasing block number and capped at 256 entries.

## Blob Base Fee

- `BLOBBASEFEE` uses the execution-specs Taylor expansion formula over `excess_blob_gas`.
- If `excess_blob_gas` is absent (pre-Cancun), return zero; if present and zero, the minimum blob gas price is 1.

## Gas Accounting

- Memory expansion is quadratic; never cap it or special-case huge inputs.
- Use `saturating_add` for offset + size to avoid overflow on large inputs.

## Stack Operand Order

- Arithmetic ops follow execution-specs order: pop `x` then `y`, compute `x op y`.

## State / Trie

- Use `EMPTY_TRIE_ROOT` for empty tries (never `Hash::ZERO`).
- Keep state root computation deterministic by inserting accounts in a stable address order.

## Testing Reality Check

- Do not quote EELS test counts unless you ran the ignored tests locally.

## Error Type Architecture

- There are three `EvmError` enums: `evm::error::EvmError`, `evm::interpreter::EvmError`, and `evm::opcodes::arithmetic::EvmError`.
- When adding opcode-local errors, add `From` conversions into `evm::error::EvmError`.
- `evm/mod.rs` re-exports from `interpreter`, not `error`; `exec.rs` imports `evm::error` directly.

## Module Visibility

- `compute_create_address` and `compute_create2_address` in `evm/host.rs` must stay `pub` because `evm/opcodes/exec.rs` imports them.

## Pre-commit Hygiene

- The no-orphan Rust files hook fails if any `src/*.rs` file is unreachable from a crate root. Delete unused modules rather than half-wiring them.
- Run `prek run` before committing; fix linting errors instead of disabling rules.

## Do / Don't (Next Iteration)

**Do**

- Run `cargo test -p claudeth --release` and `cargo clippy -p claudeth -- -D warnings`.
- Execute the EIP-2935 system call before computing the block state root.
- Provide recent block hashes in guest input for correct `BLOCKHASH` results.
- Use `EMPTY_OMMERS_HASH` for post-merge headers (including tests).
- When header base fee validation is enabled, either compute the expected base fee
  from the parent or set parent `gas_used == gas_limit / 2` in fixtures.
- Keep test helpers for empty blocks setting `block.gas_used = 0` (don’t inherit
  the parent’s gas used).

**Don't**

- Cap memory expansion gas.
- Treat EVM reverts as exceptional halts.
- Leave unused `src/*.rs` files (pre-commit will fail).
- Quote EELS test counts without rerunning.
- Accept non-empty withdrawals lists for headers without `withdrawals_root`.
