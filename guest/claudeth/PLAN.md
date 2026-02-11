# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `README.md` still claims:
  - full EELS compatibility;
  - release tests as the primary verification path;
  - no caveats about ignored compatibility gaps.
- The full blockchain EELS runner is still non-gating:
  - `tests/eels_blockchain_tests.rs::test_execute_all_blockchain_tests` remains `#[ignore]`.
  - `run_all_blockchain_tests_impl()` still prints totals without asserting `failed == 0 && errors == 0`.
- Fresh release re-baseline (partial full sweep, halted after deterministic frontier capture):
  - Command: `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`
  - First deterministic failing families observed in order:
    1. `BlockchainTests/ValidBlocks/bcStateTests/blockhashTests.json::{blockhashTests_Cancun,blockhashTests_Prague}`
       - `Block 3: GasUsedMismatch(expected=45352, computed=65252)` (`+19900`)
    2. `BlockchainTests/ValidBlocks/bcStateTests/suicideStorageCheckVCreate.json::{..._Cancun,..._Prague}`
       - `Block 0: GasUsedMismatch(expected=468193, computed=184878)`
    3. `BlockchainTests/ValidBlocks/bcStateTests/callcodeOutput3partial.json::{..._Cancun,..._Prague}`
       - `Block 0: StateRootMismatch(...)`
- Root-cause diagnosis for the first failure is already identified in code:
  - `src/evm/host.rs::blockhash()` truncates the requested `U256` with `as_u64()`.
  - Values above `u64::MAX` wrap/truncate and can incorrectly resolve to historical hashes instead of zero.
  - This exactly matches `blockhashTests` gas overcharge behavior (`+19900` via unexpected non-zero `SSTORE`).

## Completion Objective

Make implementation truthfully match `README.md` by:

- eliminating deterministic EELS fixture failures;
- making full EELS execution a default release gate;
- validating native/RV32 parity with an enforced gate;
- removing explicit compatibility caveats in behavior (including remaining precompile gaps).

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST): Fix `blockhashTests` U256 Handling

Why:
- It is the current first deterministic frontier.
- The root cause is concrete and isolated (`U256` truncation in host `BLOCKHASH` lookup).

What:
- Ensure `BLOCKHASH` only resolves when the stack argument is representable and in-range per execution-spec behavior; otherwise return zero.
- Add focused Cancun/Prague fixture regressions for `blockhashTests`.

How:
- Update `src/evm/host.rs::blockhash()` to avoid `as_u64()` truncation for request values.
- Use checked conversion (`u64::try_from`) semantics for the requested block number.
- Keep existing window logic (`< current`, max distance 256) intact after safe conversion.
- Add fixture-specific tests in `tests/eels_blockchain_tests.rs` for:
  - `blockhashTests_Cancun`
  - `blockhashTests_Prague`
- Validate with:
  - `cargo test -p claudeth --release test_blockhash_tests_ -- --nocapture`
  - `cargo test -p claudeth --release test_random_statetest241_ -- --nocapture`
  - `cargo test -p claudeth --release`

### Task 2 (P0): Fix `suicideStorageCheckVCreate` Gas Mismatch

Why:
- This is the next deterministic family immediately after `blockhashTests`.

What:
- Align gas accounting/state semantics for both Cancun and Prague cases.

How:
- Reproduce with focused fixture tests.
- Diff behavior against `execution-specs` transaction/create semantics.
- Patch minimal gas/state delta and lock with focused regressions.

### Task 3 (P0): Fix `callcodeOutput3partial` State Root Mismatch

Why:
- This is already confirmed as the next deterministic state-root divergence.

What:
- Match fixture post-state for both forks.

How:
- Add focused fixture regressions.
- Trace storage/balance side effects under `CALLCODE` path versus execution-spec.
- Apply minimal state-transition correction and rerun focused suite.

### Task 4 (P0): Burn Down Remaining Deterministic Failures to Zero

Why:
- README compatibility remains false while any deterministic fixture still fails.

What:
- Continue family-by-family until all deterministic failures in full blockchain suite are zero.

How:
- Iterative loop:
  - run ignored release sweep;
  - capture next frontier;
  - add focused regression;
  - patch minimal semantic delta;
  - rerun focused + release suite.

### Task 5 (P0): Make Full EELS Sweep a Default Hard Gate

Why:
- Ignored/non-fatal full-suite behavior permits silent regressions and conflicts with README claims.

What:
- Full blockchain EELS compatibility must fail CI/test runs on any failure.

How:
- Remove ignore posture (or provide a default path that always runs in release checks).
- Enforce `failed == 0 && errors == 0` assertions in runner.

### Task 6 (P1): Add Enforced Native vs RV32 Parity Gate

Why:
- README claims both native and RV32 paths are validated, but parity is not currently enforced as a deterministic gate.

What:
- Add curated parity fixtures and fail on divergence.

How:
- Execute identical fixture subset on native + runner targets.
- Compare gas used, logs, receipts root, state root.

### Task 7 (P1): Close Remaining Precompile Completeness Gaps

Why:
- Full compatibility claims require all relevant precompile semantics to match execution-spec behavior.

What:
- Resolve remaining intentionally incomplete precompile behavior and cover malformed/success/OOG cases.

How:
- Port semantics from execution-spec references.
- Add focused regressions.
- Re-run full release suite and ignored blockchain compatibility sweep.
