# Claudeth Development Learnings

Date: 2026-02-09

## Consensus-Critical Execution

- EVM exceptional halts (OOG, InvalidJump, etc.) must be handled as **transaction-level failures** with all gas consumed and state reverted for that transaction, not as block-level errors.
- Reverts are not exceptional halts. Preserve remaining gas per EVM rules and roll back state changes for that call frame.

## Gas Accounting

- Memory expansion gas is quadratic and must not be capped. Let the formula drive OOG.
- Always use `saturating_add` for offset+size to avoid overflow on large inputs.

## Guest Input Decoding

- Current guest input is an RLP list of 5 items only: block header, parent header, chain ID, transactions list, and state snapshot entries.
- There is no support for withdrawals or recent block hashes in the guest input path yet.

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

- **Always** verify the code compiles (`cargo build -p claudeth`) before attempting anything else. The codebase may have been left in a broken state.
- Use `EMPTY_TRIE_ROOT` for empty tries (never `Hash::ZERO`).
- Keep state root computation deterministic (stable address ordering before trie insertion).
- Run `cargo test -p claudeth --release` and `cargo clippy -p claudeth -- -D warnings`.

**Don't**

- Cap memory expansion gas.
- Treat EVM reverts as exceptional halts.
- Leave unused `.rs` files under `src/` (pre-commit will fail).
- Quote EELS test counts without rerunning.
- Re-export types from modules that don't define them (e.g. `StorageWrite` was re-exported from `trace` but didn't exist there).

## Update (2026-02-09)

**Do**

- Apply withdrawals after transaction execution and before computing state root.
- Validate withdrawals root when `withdrawals_root` is present in the header.
- Require the guest input withdrawals list only when the header includes `withdrawals_root`.

**Don't**

- Accept non-empty withdrawals lists for headers without `withdrawals_root`.
