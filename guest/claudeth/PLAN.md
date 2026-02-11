# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `cargo test -p claudeth --release` passes (unit, integration, and doc tests).
- Full blockchain fixture suite is still ignored and non-gating (`#[ignore]`).
- Latest full EELS blockchain baseline (last full ignored run):
  - command: `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`
  - totals: `Total: 1142`, `Passed: 898`, `Failed: 244`, `Errors: 0`
- Harness improvements already landed:
  - parent resolution by `parent_hash`,
  - `BLOCKHASH` ancestry window ordering oldest -> newest,
  - expected-invalid blocks excluded from canonical executed indexes.
- Trie short-node pseudo-hash bug is fixed: node references are now Keccak hashes of encoded nodes (no zero padding surrogate).
- Withdrawal trie root computation is now aligned with execution-spec trie child reference semantics for this path:
  - keying by list position (`enumerate` index),
  - canonical withdrawal RLP values,
  - child node references inline when encoded child `<32` bytes, otherwise hashed.
- Known deterministic implementation gaps still visible in code:
  - precompile `0x0a` (point evaluation) is unimplemented,
  - precompile `0x08` pairing is partial for non-trivial tuples.

## Completed

### Task 1 (DONE): Canonical Parent Selection and `BLOCKHASH` Inputs in EELS Harness

Why:
- Linear parent tracking is wrong for multi-chain fixtures.
- Missing recent block hashes guarantees `BLOCKHASH` divergence.

What:
- Parent lookup switched to `parent_hash` over executed-header hash index.
- Recent ancestry hash window is now passed into block processing.
- Added multi-chain and `BLOCKHASH` window-order regression coverage.

How:
- Introduced hash-indexed header lookup helpers.
- Constructed bounded 256-entry ancestry windows in increasing block-number order.
- Updated executed-header bookkeeping on successful blocks.

### Task 2 (DONE): Re-baseline Full EELS Blockchain Results

Why:
- Post-harness-fix mismatch profile had to be measured before further fixes.

What:
- Re-ran the entire ignored blockchain suite and captured hard counts.

How:
- Executed the full ignored test command in `--release`.
- Recorded baseline (`1142 / 898 / 244 / 0`) for subsequent prioritization.

### Task 3 (DONE): Fix MPT Short-Node Hash Semantics (Zero-Padding Bug)

Why:
- Zero-padding short node RLP bytes produced pseudo-hashes and consensus-risky trie roots.

What:
- Replaced short-node zero-padding behavior with Keccak-256 hashing of encoded node bytes.
- Added a regression test to enforce hashed behavior for short encoded nodes.

How:
- Updated `Node::compute_hash` in `src/state/partial_mpt/node.rs`.
- Updated `test_compute_hash_inline_node` to assert `keccak256(rlp(node))` for short encodings.
- Re-ran targeted release tests for node hashing and trie-root-sensitive paths.

### Task 4 (DONE): Align Withdrawal Processing and `withdrawalsRoot` Parity

Why:
- Withdrawal-root mismatches were a deterministic consensus drift and blocked Shanghai/Cancun fixture conformance.

What:
- Re-implemented withdrawals root computation with execution-spec-compatible MPT node-reference encoding.
- Added fixture-backed regression tests for non-monotonic/duplicate `withdrawal.index` vectors and validator-index-sensitive vectors.

How:
- Confirmed reference behavior from `execution-specs`:
  - trie key is RLP(`Uint(i)`) where `i` is list position,
  - trie node child encoding follows inline-or-hash threshold at 32 bytes.
- Implemented dedicated withdrawals trie encoder in `src/stf/block.rs` using compact paths and branch/extension/leaf composition.
- Validated in `--release` with:
  - `test_calculate_withdrawals_root_matches_fixture_with_duplicate_indices`,
  - `test_calculate_withdrawals_root_matches_fixture_same_address_diff_validators`,
  - `test_process_block_withdrawals_applied`,
  - `test_process_block_withdrawals_root_mismatch`,
  - full `cargo test -p claudeth --release`.

## Priority Backlog (Why / What / How)

### Task 5 (P0, FIRST): Eliminate Systematic Gas Accounting Divergences

Why:
- `GasUsedMismatch` remains the largest known failure class in full EELS runs.

What:
- Bring per-transaction and per-block gas accounting behavior in line with execution-spec semantics.

How:
- Reproduce the smallest deterministic gas-mismatch fixtures and classify by rule type.
- Fix one rule family at a time (warm/cold access costs, memory expansion, refunds, intrinsic gas, precompile metering).
- Add focused regression tests per fixed rule and re-run the minimal failing fixtures after each patch.

### Task 6 (P0): Resolve State Root Divergences on Valid Fixtures

Why:
- `StateRootMismatch` on valid blocks indicates consensus-level STF drift.

What:
- Correct execution and state-commit semantics that produce incorrect post-state roots.

How:
- Start from smallest valid failing fixtures.
- Diff account/storage/code transitions against execution-spec expected transitions.
- Add deterministic post-state root regressions per corrected behavior.

### Task 7 (P0): Implement Precompile `0x0a` Point Evaluation (EIP-4844)

Why:
- Cancun-era conformance requires this precompile.

What:
- Implement full point-evaluation precompile semantics (validation, gas, output).

How:
- Mirror execution-spec behavior and vectors.
- Add tests for success, malformed input, invalid proof, and out-of-gas behavior.

### Task 8 (P0): Complete Non-Trivial ALT_BN128 Pairing (`0x08`)

Why:
- Pairing precompile is incomplete for non-identity tuple sets.

What:
- Implement full pairing product-equation support.

How:
- Add full Miller loop + final exponentiation flow with required subgroup checks.
- Add multi-pair valid and invalid vector coverage.

### Task 9 (P0): Turn Full EELS Blockchain Test Into a Hard Gate

Why:
- README compatibility guarantees are not defensible while the full fixture suite is ignored.

What:
- Make full blockchain fixture pass mandatory.

How:
- Remove `#[ignore]` after P0 functional gaps are resolved.
- Make the test fail on any mismatch/error (`failed == 0 && errors == 0`).

### Task 10 (P1): Add Native vs RV32 Parity Automation

Why:
- README claims dual-target execution, but parity is not enforced automatically.

What:
- Add deterministic parity checks on curated fixtures for native and RV32 runs.

How:
- Add a `uv run` PEP 723 Python driver that runs both paths and diffs outcomes.
- Gate the parity command in CI once deterministic.

### Task 11 (P1): Align README Claims With Enforced Guarantees

Why:
- Public guarantees must match hard gates and measured behavior.

What:
- Update README scope/wording to exactly match enforced checks and measured conformance.

How:
- Tighten language once the full-suite hard gate is active, or explicitly scope current guarantees.
