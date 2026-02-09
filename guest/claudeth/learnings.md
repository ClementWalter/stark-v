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

## Do / Don't (Next Iteration)

**Do**

- Use `EMPTY_TRIE_ROOT` for empty tries (never `Hash::ZERO`).
- Keep state root computation deterministic (stable address ordering before trie insertion).
- Run `cargo test -p claudeth --release` and `cargo clippy -p claudeth -- -D warnings`.
- Run `uv run scripts/check_no_orphan_rust_files.py` when the pre-commit hook cannot run directly.

**Don't**

- Cap memory expansion gas.
- Treat EVM reverts as exceptional halts.
- Leave unused `.rs` files under `src/` (pre-commit will fail).
- Quote EELS test counts without rerunning.
