# Claudeth Completion Plan

Last reviewed: 2026-02-10

## Ground Truth Snapshot

- `cargo test -p claudeth --release` passes (unit, integration, and doc tests).
- `test_execute_all_blockchain_tests` is still `#[ignore]`, so full EELS blockchain conformance is not enforced.
- Harness now resolves block parents by real `parent_hash` and feeds a spec-ordered recent hash window for `BLOCKHASH` (oldest -> newest), including multi-chain fixture coverage.
- Deterministic conformance gaps still known in code:
  - precompile `0x0a` point evaluation is not implemented,
  - precompile `0x08` pairing is still partial for non-trivial tuples.
- README claims still overstate guaranteed conformance until full EELS blockchain pass is a hard gate.

## Completed This Turn

### Task 1 (DONE): Canonical Parent Selection and `BLOCKHASH` Inputs in EELS Harness

Why:
- Linear parent tracking is incorrect for multi-chain fixtures and can mask real consensus behavior.
- Empty `block_hashes` input makes `BLOCKHASH`-dependent execution diverge by construction.

What:
- Replaced linear `parent_header` progression with `parent_hash`-based parent lookup.
- Added recent canonical hash window generation from parent ancestry and passed it to `process_block`.
- Added regression tests for:
  - multi-chain fixture parent resolution (`A`/`B` branch switch),
  - `BLOCKHASH` window ordering + host lookup behavior.

How:
- Introduced harness helpers to resolve parent headers from an executed-header hash map.
- Built bounded (max 256) ancestry hash windows in increasing block-number order.
- Updated success-path bookkeeping to index executed headers by computed hash.

## Priority Backlog (Why / What / How)

### Task 2 (P0, FIRST): Re-baseline Full EELS Blockchain Results After Harness Fix

Why:
- Parent-selection and `BLOCKHASH` inputs were consensus-significant; mismatch distribution changed and must be measured before further fixes.

What:
- Run full ignored blockchain suite and produce an updated failure taxonomy with top fixture families and mismatch classes.

How:
- Execute `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored`.
- Capture aggregate counts and top recurring errors (`GasUsedMismatch`, `StateRootMismatch`, `WithdrawalsRootMismatch`, etc.).
- Pin the smallest reproducible fixtures per mismatch class for targeted implementation tasks.

### Task 3 (P0): Fix Withdrawal Fixture Semantics and Root Parity

Why:
- Persistent `WithdrawalsRootMismatch` prevents post-Shanghai block correctness.

What:
- Ensure fixture withdrawal decoding and trie root construction match execution-spec behavior exactly.

How:
- Audit conversion and RLP encoding against execution-spec withdrawal list-root logic.
- Add fixture-backed regression tests for duplicate indices, bounds, and zero-amount behavior.

### Task 4 (P0): Eliminate Systematic Gas Accounting Divergences

Why:
- `GasUsedMismatch` is one of the largest blockers to blockchain fixture parity.

What:
- Align transaction and block gas accounting with spec across warm/cold access, refunds, and tx-fee edge cases.

How:
- Start from minimal failing fixtures produced by Task 2.
- Trace per-transaction gas deltas and patch one deterministic rule at a time.
- Add regression tests for each corrected gas rule.

### Task 5 (P0): Resolve State Root Divergences on Valid Fixtures

Why:
- `StateRootMismatch` means STF semantics are still incorrect even when block structure validates.

What:
- Fix execution/state-commit behavior causing root drift on valid blocks.

How:
- Triage smallest failing valid fixtures from Task 2.
- Compare account/storage/code transitions with execution-spec expectations.
- Add deterministic fixture-based root regression tests.

### Task 6 (P0): Implement Precompile `0x0a` Point Evaluation (EIP-4844)

Why:
- Blob-era conformance requires point-evaluation precompile support.

What:
- Implement input validation, gas accounting, verification, and output semantics.

How:
- Mirror execution-spec behavior and add vectors for success, malformed calldata, invalid proof, and OOG.

### Task 7 (P0): Complete Non-Trivial ALT_BN128 Pairing (`0x08`)

Why:
- Current pairing implementation only handles validation + identity-equivalent paths.

What:
- Implement full pairing product equation for non-trivial tuple sets.

How:
- Add Miller loop + final exponentiation flow with subgroup validation.
- Add regression vectors covering valid/invalid multi-pairing inputs.

### Task 8 (P0): Turn Full EELS Blockchain Test Into a Hard Gate

Why:
- README-level compatibility claims are not defensible while the comprehensive suite is ignored and non-fatal.

What:
- Make `test_execute_all_blockchain_tests` mandatory and fail on any error/mismatch.

How:
- Remove `#[ignore]` once Tasks 2-7 converge.
- Enforce `failed == 0` and `errors == 0` in assertions.

### Task 9 (P1): Add Native vs RV32 Parity Automation

Why:
- Dual-target behavior is claimed, but parity is not automatically verified.

What:
- Add deterministic parity checks between native and RISC-V runs for curated fixtures.

How:
- Add a `uv run` Python script (PEP 723 metadata) that executes both paths and compares outcomes.
- Integrate into local validation workflow.

### Task 10 (P1): Align README Claims With Enforced Guarantees

Why:
- Public claims should reflect what CI and hard gates actually verify.

What:
- Reconcile README conformance/dependency/testing claims with real enforced behavior.

How:
- Update wording only after hard-gate completion, or explicitly scope claims to current coverage.
