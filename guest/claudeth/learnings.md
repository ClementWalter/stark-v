# Claudeth Development Learnings

Date: 2026-02-09

## Consensus-Critical Execution

- EVM exceptional halts (OOG, InvalidJump, etc.) are **transaction-level failures**: consume all gas and revert state for that transaction only, not the entire block.
- REVERT is not an exceptional halt: preserve remaining gas and revert state changes for that call frame.

## Gas Accounting

- Memory expansion gas is quadratic; never cap it. Let the formula drive OOG.
- Always use `saturating_add` for offset + size to avoid overflow on large inputs.

## Stack Operand Order

- Arithmetic ops follow execution-specs order: pop `x` then `y`, compute `x op y`. Tests must push operands accordingly.

## Guest Input Decoding

- Input is an RLP list of 5–7 items:
  - `block_header`, `parent_header`, `chain_id`, `transactions`, `state_entries`
  - Optional `block_hashes` (recent block hashes, oldest -> newest, max 256)
  - Optional `withdrawals` (required when `withdrawals_root` is present)
- If `withdrawals_root` is present, a withdrawals list must be provided.
- If `withdrawals_root` is absent, the withdrawals list must be empty.

## BLOCKHASH Data

- `BLOCKHASH` accuracy depends on providing recent hashes in guest input.
- When recent hashes are missing, only the parent hash can be returned.
- The recent hashes list must be ordered by increasing block number and capped at 256 entries.

## Pre-commit Hygiene

- The no-orphan Rust files hook will fail if any `.rs` file under `src/` is not reachable from a crate root via `mod` declarations. If a module is unused and incomplete, delete it rather than partially wiring it in.

## Testing Reality Check

- Do not quote EELS test counts unless you have run the ignored tests in the current workspace.

## Error Type Architecture

- There are three separate `EvmError` enums: `evm::error::EvmError`, `evm::interpreter::EvmError`, and `evm::opcodes::arithmetic::EvmError`. The `exec.rs` dispatcher uses `evm::error::EvmError` and needs `From` conversions from all error types it encounters.
- When adding new error variant types in opcode modules, always ensure a `From` impl exists in `evm::error::EvmError` for the module's local error type.
- The `evm/mod.rs` re-exports from `interpreter`, NOT from `error`. `exec.rs` imports from `evm::error` directly.

## Module Visibility

- `compute_create_address` and `compute_create2_address` in `evm/host.rs` must be `pub` (not private or `pub(crate)`) because `evm/opcodes/exec.rs` imports them.

## Do / Don't (Next Iteration)

**Do**

- **Always** run `cargo test -p claudeth --release` and `cargo clippy -p claudeth -- -D warnings` before finalizing.
- Use `EMPTY_TRIE_ROOT` for empty tries (never `Hash::ZERO`).
- Keep state root computation deterministic (stable address ordering before trie insertion).
- Provide recent block hashes in guest input for correct `BLOCKHASH` results.

**Don't**

- Cap memory expansion gas.
- Treat EVM reverts as exceptional halts.
- Leave unused `.rs` files under `src/` (pre-commit will fail).
- Quote EELS test counts without rerunning.
- Accept non-empty withdrawals lists for headers without `withdrawals_root`.
