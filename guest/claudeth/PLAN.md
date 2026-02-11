# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `cargo test -p claudeth --release` passes (unit, integration, and doc tests).
- Full EELS blockchain execution is still non-gating (`test_execute_all_blockchain_tests` remains `#[ignore]`).
- Latest full-suite attempt (2026-02-11):
  - command: `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`
  - status: **manually interrupted** after crossing the previous crash point (`bcExploitTest/StrangeContractCreation`) without stack overflow.
- Deterministic failures observed in the latest run include (examples):
  - `BlockchainTests/InvalidBlocks/bcMultiChainTest/UncleFromSideChain.json::UncleFromSideChain_Prague` (`GasUsedMismatch`, expected `42160`, computed `42064`)
  - `BlockchainTests/InvalidBlocks/bc4895-withdrawals/accountInteractions.json::{..._Cancun,..._Prague}` (`GasUsedMismatch`, expected `77427`, computed `79727`)
  - `BlockchainTests/InvalidBlocks/bc4895-withdrawals/warmup.json::{..._Cancun,..._Prague}` (`GasUsedMismatch`, expected `1186133`, computed `1242533`)
  - `BlockchainTests/InvalidBlocks/bcStateTests/CreateTransactionReverted.json::{..._Cancun,..._Prague}` (`StateRootMismatch`)
  - `BlockchainTests/ValidBlocks/bcExample/mergeExample.json::{..._Cancun,..._Prague}` (`GasUsedMismatch`, expected `82839`, computed `62939`)
  - `BlockchainTests/ValidBlocks/bcExploitTest/SuicideIssue.json::{..._Cancun,..._Prague}` (`GasUsedMismatch`, expected `4700000`, computed `75973`)
- Known explicit code gaps remain:
  - precompile `0x0a` point evaluation is intentionally unimplemented;
  - precompile `0x08` pairing intentionally rejects non-trivial tuples.

## Completed

### Task A (DONE): Canonical Parent Selection and `BLOCKHASH` Inputs in EELS Harness

Why:
- Multi-branch fixtures cannot be processed by loop order.

What:
- Parent resolution switched to hash-indexed ancestry.
- `BLOCKHASH` history order aligned to execution-spec expectations.

How:
- Added parent-hash keyed lookup for headers and ancestry windows.

### Task B (DONE): Parent State Selection for Forked Fixtures

Why:
- Header parent correctness alone is insufficient; state snapshots must also follow branch parents.

What:
- Added per-hash state snapshots and parent-state lookup by `parent_hash`.

How:
- Indexed executed states by block hash and selected state roots from that index.

### Task C (DONE): Trie and Withdrawal Root Conformance Fixes

Why:
- MPT short-node and withdrawals trie encoding mismatches caused deterministic root drift.

What:
- Removed short-node pseudo-hash behavior.
- Matched execution-spec child reference encoding and withdrawals trie keying.

How:
- Updated trie hashing/reference logic and withdrawals root construction with regressions.

### Task D (DONE): `SELFDESTRUCT`/`EXTCODEHASH`/Recursive Warm-Set Baseline Fixes

Why:
- These were high-frequency consensus divergences in fixture runs.

What:
- Implemented Cancun/Prague `SELFDESTRUCT` dynamic gas components.
- Corrected `EXTCODEHASH` dead-account behavior.
- Propagated recursive warm-set and CREATE nonce semantics.

How:
- Patched interpreter/host behavior and added targeted EELS fixture regressions.

### Task E (DONE): Stabilize Full EELS Failure Reporting and Stack Budget

Why:
- Full-suite fixture runs were aborting before actionable totals due stack overflow in the ignored harness flow.

What:
- Replaced unbounded `BlockProcessingError` debug dumping with bounded summaries.
- Ran full-suite logic on an explicitly large-stack thread in the ignored test harness.

How:
- Added `summarize_block_processing_error`/`summarize_transaction_results` in `tests/eels_blockchain_tests.rs`.
- Added a regression that verifies large `return_data` payloads do not explode summary output.
- Wrapped full-suite execution in `std::thread::Builder::stack_size(...)`.

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST): Fix CREATE/CREATE-Transaction State Semantics (Revert/Failure Paths)

Why:
- `CreateTransactionReverted` currently yields `StateRootMismatch` on both Cancun and Prague.
- CREATE path mistakes commonly cascade into many state-root and gas mismatches.

What:
- Align failed/reverted contract-creation state transitions with execution-spec behavior.

How:
- Reproduce with minimal failing fixture subset.
- Compare transaction lifecycle against execution-spec contract creation flow.
- Add focused regression tests for nonce, code/account persistence, and touched-account outcomes.

### Task 2 (P0): Close Remaining Gas Accounting Deltas in Call/Access/Warmth Rules

Why:
- `GasUsedMismatch` remains the dominant pre-abort failure class.
- Mismatches span invalid and valid fixture families (`bc4895-withdrawals`, `bcExample`, `bcEIP1559`).

What:
- Resolve residual differences in CALL-family dynamic gas and access-list/warmness accounting.

How:
- Bucket failures by gas delta signature and fixture family.
- Patch one rule family at a time with execution-spec cross-checks.
- Add dedicated regression fixtures per corrected rule.

### Task 3 (P0): Resolve State-Root Mismatches on Valid Fixture Families

Why:
- Any valid-block root mismatch is consensus-critical and invalidates README compatibility claims.

What:
- Eliminate remaining post-state transition divergences once gas accounting is aligned.

How:
- Start with smallest valid failing fixtures (`bcExample/*`).
- Diff account/storage/code transitions against execution-spec expected outcomes.
- Add deterministic root regression tests.

### Task 4 (P0): Implement Precompile `0x0a` Point Evaluation (EIP-4844)

Why:
- Cancun/Prague conformance is incomplete while point evaluation remains intentionally unimplemented.

What:
- Implement full input validation, gas charge, and verification semantics.

How:
- Follow execution-spec point-evaluation behavior exactly.
- Add success/failure/malformed/OOG regression vectors.

### Task 5 (P0): Implement Full Non-Trivial BN254 Pairing (`0x08`)

Why:
- Current pairing implementation intentionally fails non-trivial tuples.

What:
- Implement full pairing product equation checks and canonical return encoding.

How:
- Port execution-spec-compatible pairing arithmetic/validation path.
- Add multi-tuple valid/invalid test vectors.

### Task 6 (P0): Make Full EELS Blockchain Execution a Hard Gate

Why:
- README states full compatibility, but the only test that can validate this is currently ignored and non-fatal.

What:
- Remove `#[ignore]` and enforce zero failures/errors for the full fixture run.

How:
- Enable only after Tasks 1-5 are complete.
- Turn summary counters into hard assertions (`failed == 0 && errors == 0`).

### Task 7 (P1): Enforce Native vs RV32 Deterministic Parity on Curated Fixtures

Why:
- README claims dual-target execution, but parity is not automatically validated.

What:
- Add automated native-vs-RV32 parity checks over stable fixture subsets.

How:
- Add a `uv run` PEP 723 Python driver that executes both targets and diffs outcomes.
- Add command to CI once deterministic.

### Task 8 (P1): Align README Claims with Enforced Guarantees

Why:
- Public claims must only state what test gates currently prove.

What:
- Update README wording to match measured, hard-gated behavior.

How:
- Finalize text only after full-suite gating and parity checks land.
