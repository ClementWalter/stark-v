# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `cargo test -p claudeth --release` passes (unit, integration, and doc tests).
- Full EELS blockchain fixture execution is still non-gating (`test_execute_all_blockchain_tests` remains `#[ignore]`).
- Latest complete full-suite baseline (from a completed ignored run):
  - command: `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`
  - totals: `Total: 1142`, `Passed: 898`, `Failed: 244`, `Errors: 0`
- A partial rerun on 2026-02-11 confirms `GasUsedMismatch` remains the dominant failure class (examples observed in logs: `extCodeHashOfDeletedAccountDynamic_*`, `randomStatetest123_*`, `ZeroValue_TransactionCALLwithData_OOGRevert_Prague`).
- Focused rerun on 2026-02-11:
  - `test_extcodehash_deleted_account_dynamic_{cancun,prague}_fixture` now passes in `--release`.
  - Previous `GasUsedMismatch` on this fixture family has been removed.
- Deterministic implementation gaps still visible in code:
  - precompile `0x0a` (point evaluation) is unimplemented;
  - precompile `0x08` pairing still rejects non-trivial tuples;
  - full EELS parity is not a hard test gate, so README compatibility claims are not yet enforceable.

## Completed

### Task 1 (DONE): Canonical Parent Selection and `BLOCKHASH` Inputs in EELS Harness

Why:
- Multi-chain fixtures cannot be processed with linear parent tracking.
- `BLOCKHASH` parity requires a correct 256-block ancestry window.

What:
- Parent selection switched to `parent_hash` lookup over executed-header hash index.
- Recent block hashes are now passed in execution-spec order (oldest -> newest).

How:
- Added hash-indexed header resolution helpers.
- Built bounded ancestry windows from parent hash walk.

### Task 2 (DONE): Re-baseline Full EELS Blockchain Results

Why:
- Post-harness parity had to be measured before deeper fixes.

What:
- Captured full-suite baseline (`1142 / 898 / 244 / 0`) for prioritization.

How:
- Ran ignored full fixture command in `--release` and recorded aggregate totals.

### Task 3 (DONE): Fix MPT Short-Node Hash Semantics

Why:
- Short-node zero-padding created pseudo-hashes and incorrect trie references.

What:
- Replaced pseudo-hash behavior with Keccak-256 over encoded node bytes when 32-byte references are required.

How:
- Updated `Node::compute_hash` and added regressions.

### Task 4 (DONE): Align Withdrawal Processing and `withdrawalsRoot`

Why:
- Withdrawal-root drift was a deterministic consensus mismatch.

What:
- Implemented execution-spec-compatible withdrawals trie keying and child reference encoding.

How:
- Reworked withdrawals trie construction and added fixture-backed regressions.

### Task 5 (DONE): Implement Full Cancun/Prague `SELFDESTRUCT` Gas Semantics

Why:
- Missing dynamic `SELFDESTRUCT` gas caused systematic undercharging.

What:
- Implemented cold-beneficiary and conditional new-account surcharges.

How:
- Mirrored execution-spec `system.py::selfdestruct` gas decision points and added opcode regressions.

### Task 6 (DONE): Resolve Parent-State Selection for Forked Blockchain Fixtures

Why:
- Forked fixtures were mutating one linear state, creating false branch failures.

What:
- Added hash-indexed parent-state selection in the EELS harness.

How:
- Introduced per-block `HashMap<Hash, InMemoryState>` snapshots keyed by executed block hash.

### Task 7 (DONE): Fix `EXTCODEHASH` Empty/Non-Existent Account Semantics

Why:
- Execution-spec requires `EXTCODEHASH` to push `0` for empty/non-existent accounts.
- Returning `keccak256("")` in this case creates consensus-level control-flow divergence.

What:
- Updated both interpreter and opcode helper paths to return `0` when the account is not alive.
- Added regressions for:
  - non-existent account -> `0`;
  - alive account with empty code -> `keccak256("")`.

How:
- Followed execution-spec `environment.py::extcodehash` behavior (`account == EMPTY_ACCOUNT` => `0`).
- Patched `src/evm/interpreter.rs` and `src/evm/opcodes/environment.rs` with shared liveness-based behavior.

### Task 8 (DONE): Propagate Warm-Set and CREATE Nonce Semantics in Recursive Execution

Why:
- execution-spec child messages inherit tx-level warm context; missing this caused nested cold overcharges.
- `extCodeHashOfDeletedAccountDynamic_*` also exposed missing CREATE/CREATE2 nonce transitions in recursive execution.

What:
- Warmed recursive `CALL*`/`CREATE*` frames with tx-level baseline addresses.
- Marked CREATE/CREATE2 destinations warm before execution in the parent interpreter.
- Aligned recursive CREATE nonce transitions (creator nonce increment + created account nonce=1 on successful path semantics).
- Added non-ignored EELS regressions for Cancun and Prague dynamic deleted-account EXTCODEHASH fixtures.

How:
- Patched `src/evm/host.rs` and `src/evm/interpreter.rs` to mirror execution-spec warm/nonce behavior.
- Added fixture-backed tests in `tests/eels_blockchain_tests.rs`.
- Verified with `cargo test -p claudeth --release` and `prek run --all-files`.

## Priority Backlog (Why / What / How)

### Task 9 (P0, FIRST): Eliminate Remaining `SELFDESTRUCT`/Account-Liveness Gas Divergences

Why:
- After warm-set propagation, `SELFDESTRUCT` gas/liveness edge cases may still
  remain; these are consensus-critical and appear in known failing buckets.

What:
- Align post-`SELFDESTRUCT` account liveness and subsequent gas/account-access
  behavior across same-block transaction sequences.

How:
- Reproduce with the smallest failing fixture subset.
- Compare step-by-step against execution-spec `system.py` and state helpers.
- Add regression tests for cross-transaction delete/liveness behavior in block
  execution.

### Task 10 (P0): Systematically Close Remaining CALL-Family Gas Rule Deltas

Why:
- `GasUsedMismatch` remains dominant and blocks full fixture parity.

What:
- Close remaining deltas in CALL/CALLCODE/DELEGATECALL/STATICCALL accounting
  (cold/warm access, new-account surcharge, stipend/forwarded gas boundaries).

How:
- Bucket failing fixtures by gas delta signature.
- Patch one rule family at a time with execution-spec cross-checks.
- Add targeted regressions for each corrected family.

### Task 11 (P0): Resolve `StateRootMismatch` on Valid Fixtures

Why:
- Any valid-block state root mismatch is a consensus-level STF deviation.

What:
- Correct remaining transition semantics after gas-rule parity improves.

How:
- Start from smallest valid failing fixtures.
- Diff account/storage/code transitions against execution-spec expectations.
- Add deterministic post-state-root regressions.

### Task 12 (P0): Implement Precompile `0x0a` Point Evaluation (EIP-4844)

Why:
- Cancun/Prague conformance requires this precompile.

What:
- Implement full input validation, fixed gas charge, and proof verification behavior.

How:
- Follow execution-spec `point_evaluation.py` semantics exactly.
- Add tests for valid proof, invalid proof, malformed input, and OOG paths.

### Task 13 (P0): Complete Non-Trivial ALT_BN128 Pairing (`0x08`)

Why:
- Current implementation intentionally fails non-trivial tuples, breaking pairing coverage.

What:
- Implement full pairing product equation path with correct validation and result encoding.

How:
- Port execution-spec-compatible pairing checks and arithmetic flow.
- Add multi-tuple valid and invalid regression vectors.

### Task 14 (P0): Make Full EELS Blockchain Execution a Hard Gate

Why:
- Compatibility claims are not defensible while full-suite execution is ignored.

What:
- Turn full fixture execution into a mandatory pass criterion.

How:
- Remove `#[ignore]` once P0 functional gaps are closed.
- Enforce `failed == 0 && errors == 0` in assertions.

### Task 15 (P1): Enforce Native vs RV32 Parity on Curated Fixtures

Why:
- README claims dual-target execution, but parity is not auto-verified.

What:
- Add automated parity checks between native and RV32 execution for deterministic fixture subsets.

How:
- Add a `uv run` PEP 723 Python driver that runs both targets and diffs outcomes.
- Gate the parity command in CI when stable.

### Task 16 (P1): Align README Claims with Enforced Guarantees

Why:
- Public claims must match hard-gated behavior.

What:
- Update README wording to match measured and enforced conformance.

How:
- Tighten wording after full-suite gating lands, or explicitly scope current status.
