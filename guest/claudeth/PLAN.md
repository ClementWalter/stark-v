# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `README.md` still claims full EELS compatibility, but full blockchain compatibility is not yet a hard gate.
- Full blockchain sweep is still `#[ignore]` and non-fatal:
  - `tests/eels_blockchain_tests.rs::test_execute_all_blockchain_tests` remains ignored.
  - `run_all_blockchain_tests_impl()` still reports totals without asserting `failed == 0 && errors == 0`.
- Fresh release sweep frontier capture (2026-02-11):
  - Command: `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`
  - First deterministic failures:
    1. `BlockchainTests/ValidBlocks/bcStateTests/callcodeOutput3partial.json::callcodeOutput3partial_Cancun`
       - `Block 0: StateRootMismatch(expected=0x516006c1..., computed=0x4f87e2a7...)`
    2. `BlockchainTests/ValidBlocks/bcStateTests/callcodeOutput3partial.json::callcodeOutput3partial_Prague`
       - `Block 0: StateRootMismatch(expected=0x3cbc8f19..., computed=0x9b341bbb...)`
- `suicideStorageCheckVCreate` Cancun/Prague now pass in the same sweep and are no longer the frontier.
- Execution-spec reference for this frontier:
  - `execution-specs/src/ethereum/forks/{cancun,prague}/vm/instructions/system.py::generic_call()`
  - `memory_write(...)` copies only `min(memory_output_size, len(child_output))` bytes and does not zero-fill the untouched tail of the output slice.
  - Fixture `callcodeOutput3partial` exercises exactly this partial-output behavior (historical filename; bytecode path is `DELEGATECALL`).
- Task 2 implementation status (this pass):
  - Patched call-output memory writes to preserve untouched output-tail bytes while still applying already-charged memory expansion:
    - `src/evm/interpreter.rs`
    - `src/evm/opcodes/utils.rs`
  - Added focused regressions:
    - `tests/eels_blockchain_tests.rs::test_callcode_output3partial_cancun_fixture`
    - `tests/eels_blockchain_tests.rs::test_callcode_output3partial_prague_fixture`
    - `src/evm/interpreter.rs::test_call_opcode_preserves_output_tail_when_return_data_is_shorter`
  - Focused release validation passes:
    - `cargo test -p claudeth --release test_call_opcode_preserves_output_tail_when_return_data_is_shorter`
    - `cargo test -p claudeth --release test_callcode_output3partial_ -- --nocapture`
  - Ignored full-sweep rerun passed both `callcodeOutput3partial` variants and progressed into `bcWalletTest` before manual interruption; next deterministic frontier still needs a complete post-fix capture.

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
- The failure signature (large gas deficit) matches nested CREATE address/collision drift caused by created-account nonce initialization timing.

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
  - `cargo test -p claudeth --release test_suicide_storage_check_v_create_ -- --nocapture`
  - `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture` (capture next frontier)

### Task 2 (P0, DONE): Fix `callcodeOutput3partial` Partial Return-Data Memory Semantics

Why:
- It is the current deterministic frontier in the ignored release sweep.
- Failure is state-root only with matching gas usage, which is consistent with memory/state mutation drift rather than gas accounting.

What:
- Match `CALL*` output copy semantics from execution-spec for both Cancun and Prague:
  - copy only returned bytes into output memory;
  - leave the remaining output range unchanged (no forced zero-fill).
- Ensure memory-size side effects still follow already-charged expansion behavior.

How:
- Add focused fixture regressions for `callcodeOutput3partial`.
- Patch active interpreter output-copy helper used by `CALL`/`CALLCODE`/`DELEGATECALL`/`STATICCALL`.
- Keep helper behavior in `src/evm/opcodes/utils.rs` aligned with interpreter helper for consistency across dispatch paths.
- Validate with:
  - `cargo test -p claudeth --release test_callcode_output3partial_ -- --nocapture`
  - `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture` (capture next frontier)

### Task 3 (P0, FIRST): Capture Next Frontier and Burn Down Remaining Deterministic EELS Failures

Why:
- README compatibility claims remain false while any deterministic fixture fails.
- After Task 2, the next deterministic failing family has not yet been captured in a completed post-fix sweep.

What:
- Re-establish the first failing family after the `callcodeOutput3partial` fix, then continue frontier-by-frontier until ignored full blockchain suite reaches zero failures/errors.

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
