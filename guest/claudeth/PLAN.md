# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `cargo test -p claudeth --release` passes (unit, integration, and doc tests).
- Full EELS blockchain fixture execution is still non-gating (`test_execute_all_blockchain_tests` is `#[ignore]`).
- Latest complete full-suite baseline (from the last completed ignored run):
  - command: `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`
  - totals: `Total: 1142`, `Passed: 898`, `Failed: 244`, `Errors: 0`
- A fresh rerun on 2026-02-11 revealed an additional deterministic harness issue before deep gas/root analysis:
  - forked fixtures (for example `bcMultiChainTest/UncleFromSideChain`) still reuse a single mutable state across branches;
  - parent headers are selected by hash, but parent state is still selected implicitly by loop order;
  - this causes false `NonceTooLow` validation failures on otherwise non-exception branch blocks.
- Deterministic implementation gaps currently visible in code:
  - `SELFDESTRUCT` dynamic gas now includes execution-spec cold-beneficiary and new-account surcharges.
  - precompile `0x0a` (point evaluation) is still unimplemented.
  - precompile `0x08` pairing only handles identity/infinity paths; non-trivial tuples are unimplemented.
  - full suite parity is not enforced as a hard gate, so README compatibility claims are not currently provable by CI/test gating.

## Completed

### Task 1 (DONE): Canonical Parent Selection and `BLOCKHASH` Inputs in EELS Harness

Why:
- Multi-chain fixtures cannot be processed with linear parent tracking.
- `BLOCKHASH` parity requires a correct 256-block ancestry window.

What:
- Parent selection switched to `parent_hash` lookup over executed-header hash index.
- Recent block hashes are now passed in execution-spec order (oldest -> newest).
- Added regression coverage for multi-branch parent resolution and `BLOCKHASH` ordering.

How:
- Added hash-indexed header resolution helpers.
- Built bounded ancestry windows from parent hash walk.
- Updated executed-header bookkeeping to exclude expected-invalid blocks.

### Task 2 (DONE): Re-baseline Full EELS Blockchain Results

Why:
- Post-harness parity had to be measured before deeper fixes.

What:
- Captured a full-suite baseline (`1142 / 898 / 244 / 0`) for prioritization.

How:
- Ran ignored full fixture command in `--release` and recorded aggregate totals.

### Task 3 (DONE): Fix MPT Short-Node Hash Semantics

Why:
- Short-node zero-padding created pseudo-hashes and incorrect trie references.

What:
- Replaced pseudo-hash behavior with Keccak-256 over encoded node bytes when 32-byte references are required.

How:
- Updated `Node::compute_hash` and added/updated regression assertions.

### Task 4 (DONE): Align Withdrawal Processing and `withdrawalsRoot`

Why:
- Withdrawal-root drift was a deterministic consensus mismatch.

What:
- Implemented execution-spec-compatible withdrawals trie keying and child reference encoding.

How:
- Reworked withdrawals trie construction and added fixture-backed regressions.

### Task 5 (DONE): Implement Full Cancun/Prague `SELFDESTRUCT` Gas Semantics

Why:
- Missing dynamic `SELFDESTRUCT` gas caused systematic undercharging in selfdestruct-heavy fixtures.

What:
- Implemented missing dynamic charges for opcode `0xFF`:
  - cold beneficiary access surcharge,
  - new-account surcharge when beneficiary is not alive and originator balance is non-zero.

How:
- Mirrored execution-spec `system.py::selfdestruct` gas decision points.
- Added interpreter regressions for cold/warm beneficiary behavior and zero/non-zero originator balances.
- Validated with `cargo test -p claudeth --release test_selfdestruct` and full `cargo test -p claudeth --release`.

### Task 6 (DONE): Resolve Parent-State Selection for Forked Blockchain Fixtures

Why:
- Forked fixtures were still mutating one linear state, creating false failures
  on branch pivots (`NonceTooLow` on non-exception blocks).

What:
- Added hash-indexed parent-state selection in the EELS harness.
- Validated final post-state against fixture `lastblockhash` snapshot.
- Added a regression for `bcMultiChainTest/UncleFromSideChain`.

How:
- Introduced per-block `HashMap<Hash, InMemoryState>` snapshots keyed by executed block hash.
- Executed each block against a clone of its resolved parent state by `parent_hash`.
- Ensured expected-invalid blocks do not advance header or state indexes.

## Priority Backlog (Why / What / How)

### Task 7 (P0, FIRST): Systematically Eliminate Remaining Gas Accounting Divergences

Why:
- `GasUsedMismatch` remains a dominant post-harness failure class and blocks full fixture parity.

What:
- Close remaining gas-rule deltas across CALL-family accounting, refunds, memory expansion, and opcode-specific dynamic costs.

How:
- Reproduce smallest deterministic failing fixtures per rule family.
- Fix one family at a time with execution-spec cross-checks.
- Add fixture-linked regressions for every patched family.

### Task 8 (P0): Resolve `StateRootMismatch` on Valid Fixtures

Why:
- Any valid-block state root mismatch is a consensus-level STF deviation.

What:
- Correct state transition semantics that still diverge after gas fixes.

How:
- Start from smallest valid failing fixtures.
- Diff account/storage/code transitions against execution-spec expectations.
- Add deterministic post-state-root regression tests.

### Task 9 (P0): Implement Precompile `0x0a` Point Evaluation (EIP-4844)

Why:
- Cancun/Prague conformance requires this precompile.

What:
- Implement full input validation, fixed gas charge, and proof verification behavior.

How:
- Follow execution-spec `point_evaluation.py` semantics exactly.
- Add tests for valid proof, invalid proof, malformed input, and OOG paths.

### Task 10 (P0): Complete Non-Trivial ALT_BN128 Pairing (`0x08`)

Why:
- Current implementation intentionally fails non-trivial tuples, breaking pairing coverage.

What:
- Implement full pairing product equation path with correct validation and result encoding.

How:
- Port execution-spec-compatible pairing checks and arithmetic flow.
- Add multi-tuple valid and invalid regression vectors.

### Task 11 (P0): Make Full EELS Blockchain Test a Hard Gate

Why:
- Compatibility claims are not defensible while full suite execution is ignored.

What:
- Turn full fixture execution into a mandatory pass criterion.

How:
- Remove `#[ignore]` once P0 functional gaps are closed.
- Enforce `failed == 0 && errors == 0` in test assertions.

### Task 12 (P1): Enforce Native vs RV32 Parity on Curated Fixtures

Why:
- README claims dual-target execution but parity is not currently auto-verified.

What:
- Add automated parity checks between native and RV32 execution for deterministic fixture subsets.

How:
- Add a `uv run` PEP 723 Python driver that runs both targets and diffs outcomes.
- Gate this parity command in CI when stable.

### Task 13 (P1): Align README Claims with Enforced Guarantees

Why:
- Public claims must match what tests actually enforce.

What:
- Update README wording to match hard-gated guarantees and measured conformance.

How:
- Tighten wording after full-suite gating lands, or explicitly scope current status.
