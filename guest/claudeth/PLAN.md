# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `README.md` still claims full EELS compatibility, but full blockchain compatibility is not yet a hard gate.
- Full blockchain sweep is still `#[ignore]` and non-fatal:
  - `tests/eels_blockchain_tests.rs::test_execute_all_blockchain_tests` remains ignored.
  - `run_all_blockchain_tests_impl()` still reports totals without asserting `failed == 0 && errors == 0`.
- Release verification re-baseline (ignored sweep, deterministic frontier capture):
  - Command: `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`
  - First deterministic failing families observed:
    1. `BlockchainTests/ValidBlocks/bcStateTests/suicideStorageCheckVCreate.json::{..._Cancun,..._Prague}`
       - `Block 0: GasUsedMismatch(expected=468193, computed=184878)`
    2. `BlockchainTests/ValidBlocks/bcStateTests/callcodeOutput3partial.json::{..._Cancun,..._Prague}`
       - `Block 0: StateRootMismatch(...)`
- Execution-spec analysis for the first failing family points to CREATE-message setup mismatch:
  - In execution-specs, create-message processing increments the created account nonce *before* init-code execution.
  - Claudeth top-level create path in `src/stf/executor.rs::execute_create()` was patched to align with this behavior.
- Task 1 implementation status (this pass):
  - Added focused regressions:
    - `test_suicide_storage_check_vcreate_cancun_fixture`
    - `test_suicide_storage_check_vcreate_prague_fixture`
  - Verified with `cargo test -p claudeth --release test_suicide_storage_check_vcreate_ -- --nocapture` (both pass).
  - Full ignored sweep has not yet been rerun to completion post-fix in this pass; next recorded deterministic frontier remains `callcodeOutput3partial`.

## Completion Objective

Make implementation truthfully match `README.md` by:

- eliminating deterministic EELS fixture failures;
- making full EELS execution a default release gate;
- enforcing native/RV32 parity checks;
- closing remaining semantic gaps that still require fixture-specific caveats.

## Priority Backlog (Why / What / How)

### Task 1 (P0, DONE): Fix `suicideStorageCheckVCreate` CREATE Message Semantics

Why:
- It is the first current deterministic failing family in the ignored release sweep.
- The failure signature (large gas undercount) matches nested CREATE address/collision drift caused by created-account nonce initialization timing.

What:
- Align top-level create-message initialization with execution-spec behavior so nested CREATE observes the correct caller nonce and collision path.
- Add focused fixture regressions for both Cancun and Prague `suicideStorageCheckVCreate` cases.

How:
- In `src/stf/executor.rs::execute_create()`:
  - increment created-account nonce before executing init code (create-message setup);
  - preserve rollback behavior for failed top-level creation paths;
  - avoid double-increment on successful deployment by removing/adjusting late nonce bump.
- In `tests/eels_blockchain_tests.rs`:
  - add focused tests for:
    - `suicideStorageCheckVCreate_Cancun`
    - `suicideStorageCheckVCreate_Prague`
- Validate with:
  - `cargo test -p claudeth --release test_suicide_storage_check_vcreate_ -- --nocapture`
  - `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture` (capture next frontier)

### Task 2 (P0, FIRST): Fix `callcodeOutput3partial` State Root Mismatch

Why:
- It is the next deterministic frontier immediately after `suicideStorageCheckVCreate`.

What:
- Match post-state semantics for both Cancun and Prague fixture variants.

How:
- Add focused fixture regressions for `callcodeOutput3partial`.
- Trace CALLCODE state/accounting behavior versus execution-spec references.
- Apply minimal state-transition correction and rerun focused + ignored sweep.

### Task 3 (P0): Burn Down Remaining Deterministic EELS Failures to Zero

Why:
- README compatibility claims remain false while any deterministic fixture fails.

What:
- Continue frontier-by-frontier until ignored full blockchain suite reaches zero failures/errors.

How:
- Repeat loop:
  - run ignored release sweep;
  - capture first failing family;
  - add focused regression;
  - patch minimal semantic delta;
  - rerun focused and full ignored sweep.

### Task 4 (P0): Make Full EELS Sweep a Default Hard Gate

Why:
- Ignored/non-fatal behavior allows silent compatibility regressions and contradicts README claims.

What:
- Full blockchain EELS compatibility must fail test/CI runs on any failure.

How:
- Remove ignore posture (or run the full suite in default release validation path).
- Enforce `failed == 0 && errors == 0` in test harness assertions.

### Task 5 (P1): Enforce Native vs RV32 Parity Gate

Why:
- README states both native and RV32 execution are validated, but parity is not yet an enforced pass/fail gate.

What:
- Add deterministic parity checks on a curated fixture subset.

How:
- Execute identical fixtures on native and runner targets.
- Compare gas used, receipts root, logs bloom, and state root.

### Task 6 (P1): Close Remaining Compatibility Gaps (Including Precompiles)

Why:
- Full compatibility claims require complete fork-accurate behavior for all relevant execution paths.

What:
- Resolve remaining known semantic gaps and lock them with focused regressions.

How:
- Use execution-spec references for each gap.
- Add targeted tests for success/failure/OOG/malformed cases.
- Re-run release + ignored full-suite validation after each fix.
