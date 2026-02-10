# Claudeth Completion Plan

Last reviewed: 2026-02-10

## Ground Truth Snapshot

- `cargo test -p claudeth --release` passes (unit, integration, and doc tests).
- Full blockchain fixture suite is still ignored and non-gating (`#[ignore]`).
- Latest full EELS blockchain baseline:
  - command: `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`
  - totals: `Total: 1142`, `Passed: 898`, `Failed: 244`, `Errors: 0`
- Harness improvements already landed: parent resolution by `parent_hash` and `BLOCKHASH` ancestry window ordering (oldest -> newest).
- Known deterministic implementation gaps still visible in code:
  - precompile `0x0a` (point evaluation) is unimplemented,
  - precompile `0x08` pairing is partial for non-trivial tuples.
- Short-node zero-padding in `Node::compute_hash` has been removed; short RLP nodes are now Keccak-hashed.

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
- Recorded baseline (`1142 / 898 / 244 / 0`) for all subsequent prioritization.

### Task 3 (DONE): Fix MPT Short-Node Hash Semantics (Zero-Padding Bug)

Why:
- Zero-padding short node RLP bytes produced pseudo-hashes and consensus-risky trie roots.

What:
- Replaced short-node zero-padding behavior with Keccak-256 hashing of encoded node bytes.
- Added a regression test to enforce hashed behavior for short encoded nodes.

How:
- Updated `Node::compute_hash` in `src/state/partial_mpt/node.rs`.
- Updated `test_compute_hash_inline_node` to assert `keccak256(rlp(node))` for short encodings.
- Re-ran targeted release tests:
  - `state::partial_mpt::node::tests::test_compute_hash_inline_node`
  - `test_withdrawals_root_matches_shanghai_fixture_vector`
  - `test_single_key_storage_root`

## Priority Backlog (Why / What / How)

### Task 4 (P0, FIRST): Align Withdrawal Processing and `withdrawalsRoot` Parity

Why:
- Withdrawal-root failures still block Shanghai/Cancun fixture correctness.

What:
- Validate withdrawal decoding + insertion path against execution-spec behavior end-to-end.

How:
- Confirm keying (`enumerate` index) and value RLP encoding against `execution-specs`.
- Add fixture-backed regressions for edge withdrawal lists.
- Re-run minimal failing withdrawal fixtures from the EELS suite.

### Task 5 (P0): Eliminate Systematic Gas Accounting Divergences

Why:
- `GasUsedMismatch` is still the largest visible failure class.

What:
- Bring per-transaction and per-block gas accounting in line with spec.

How:
- Reproduce smallest failing gas fixtures.
- Patch one deterministic gas rule at a time (warm/cold, refunds, precompile cost paths).
- Add fixture-driven regression tests per fixed rule.

### Task 6 (P0): Resolve State Root Divergences on Valid Fixtures

Why:
- `StateRootMismatch` on valid blocks indicates consensus-level STF drift.

What:
- Fix execution/state-commit semantics producing incorrect post-state roots.

How:
- Start from smallest valid failing fixtures.
- Compare account/storage/code transitions with execution-spec expectations.
- Add deterministic root regression tests per corrected behavior.

### Task 7 (P0): Implement Precompile `0x0a` Point Evaluation (EIP-4844)

Why:
- Cancun-era conformance requires this precompile.

What:
- Implement full point-evaluation precompile semantics (validation, gas, output).

How:
- Mirror execution-spec behavior and vectors.
- Cover success, malformed input, invalid proof, and OOG.

### Task 8 (P0): Complete Non-Trivial ALT_BN128 Pairing (`0x08`)

Why:
- Pairing precompile is incomplete for non-identity tuple sets.

What:
- Implement full pairing product equation support.

How:
- Add full Miller loop + final exponentiation flow with subgroup checks.
- Add multi-pair valid/invalid fixture coverage.

### Task 9 (P0): Turn Full EELS Blockchain Test Into a Hard Gate

Why:
- README compatibility claims are not defensible while the full suite remains ignored.

What:
- Make full blockchain fixture pass mandatory.

How:
- Remove `#[ignore]` once P0 functional gaps are resolved.
- Fail test on any mismatch/error (`failed == 0 && errors == 0`).

### Task 10 (P1): Add Native vs RV32 Parity Automation

Why:
- README claims dual-target execution, but parity is not enforced automatically.

What:
- Add deterministic parity check workflow for curated fixtures.

How:
- Add a `uv run` PEP 723 Python driver that runs native and RV32 paths and diffs outcomes.

### Task 11 (P1): Align README Claims With Enforced Guarantees

Why:
- Public guarantees must match hard gates and measured behavior.

What:
- Update README language to match what tests actually enforce.

How:
- Tighten wording once full-suite hard gate is active (or explicitly scope current guarantees).
