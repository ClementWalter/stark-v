# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `README.md` claims full EELS compatibility and dual-target validation, but the full blockchain sweep is still not a default hard gate.
- Default release validation currently passes:
  - `cargo test -p claudeth --release`
- Full blockchain sweep is still `#[ignore]` and non-fatal:
  - `tests/eels_blockchain_tests.rs::test_execute_all_blockchain_tests` remains ignored.
  - `run_all_blockchain_tests_impl()` still reports totals without asserting `failed == 0 && errors == 0`.
- Fresh ignored sweep frontier capture (2026-02-11):
  - Command: `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`
  - First deterministic failures:
    1. `BlockchainTests/ValidBlocks/bcRandomBlockhashTest/randomStatetest99BC.json::randomStatetest99BC_Prague`
       - `Block 0: StateRootMismatch(expected=0x40bd7733..., computed=0x070cbd9f...)`
    2. `BlockchainTests/ValidBlocks/bcRandomBlockhashTest/randomStatetest99BC.json::randomStatetest99BC_Cancun`
       - `Block 0: StateRootMismatch(expected=0x9a160e1c..., computed=0xe7aae47e...)`
- Execution-spec reference used for this frontier:
  - `execution-specs/src/ethereum/forks/{cancun,prague}/vm/instructions/environment.py::codecopy()`
  - `code_start_index` is handled as a full-width stack integer and fed into `buffer_read(...)`; large offsets are out-of-range reads, not low-bit-truncated indices.

## Completion Objective

Make implementation truthfully match `README.md` by:

- eliminating deterministic EELS fixture failures;
- making full EELS execution a default release hard gate;
- enforcing native/RV32 parity checks;
- closing remaining semantic gaps without fixture-specific caveats.

## Priority Backlog (Why / What / How)

### Task 1 (P0, DONE): Fix `randomStatetest99BC` CODECOPY Source-Offset Semantics

Why:
- It is the first deterministic failing family in the ignored full sweep.
- Failure pattern was state-root-only with matching gas, indicating semantic state drift rather than gas accounting drift.

What:
- Align `CODECOPY` source-offset behavior with execution-spec semantics:
  - treat source offsets as full-width `U256` values;
  - for offsets above host `usize`, read zero bytes (out-of-range), do not wrap/truncate.

How:
- Patched active interpreter `CODECOPY` path in `src/evm/interpreter.rs`.
- Kept opcode helper parity in `src/evm/opcodes/environment.rs::codecopy()`.
- Added focused regressions:
  - `tests/eels_blockchain_tests.rs::test_random_statetest99bc_cancun_fixture`
  - `tests/eels_blockchain_tests.rs::test_random_statetest99bc_prague_fixture`
  - `src/evm/interpreter.rs::test_codecopy_with_huge_source_offset_does_not_wrap`
- Added failure diagnostics in `execute_blockchain_case()` to distinguish account-state drift vs trie/root-only drift.
- Focused release validation passed:
  - `cargo test -p claudeth --release test_codecopy_with_huge_source_offset_does_not_wrap`
  - `cargo test -p claudeth --release test_random_statetest99bc_ -- --nocapture`

### Task 2 (P0, FIRST): Capture the Next Deterministic Frontier After `randomStatetest99BC`

Why:
- README compatibility claims remain false while any deterministic fixture fails.
- After Task 1, the next failing family has not been recaptured yet.

What:
- Re-run ignored full blockchain sweep and record the first remaining deterministic failing family.

How:
- Run:
  - `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`
- Stop at first deterministic failure family.
- Add focused regression tests for that family before patching.

### Task 3 (P0): Audit U256 Offset Truncation Across Remaining Copy/Read Paths

Why:
- `CODECOPY` truncation was a concrete correctness bug; similar `as_usize()` truncation patterns can silently corrupt semantics in other opcodes.

What:
- Ensure full-width `U256` offset semantics are respected across copy/read opcodes and helpers.

How:
- Audit active interpreter and opcode-helper paths for `CALLDATACOPY`, `RETURNDATACOPY`, `EXTCODECOPY`, and related memory-copy operations.
- For source offsets beyond `usize`, treat reads as out-of-range (zero-fill where applicable).
- For impossible memory destinations/sizes, preserve spec-equivalent OOG/invalid behavior.
- Add focused regressions for each corrected opcode behavior.

### Task 4 (P0): Make Full EELS Sweep a Default Hard Gate

Why:
- Ignored/non-fatal behavior allows silent compatibility regressions and contradicts README claims.

What:
- Full blockchain EELS compatibility must fail release validation on any failure/error.

How:
- Remove ignored posture for full-sweep test.
- Enforce `failed == 0 && errors == 0` assertions in `run_all_blockchain_tests_impl()`.

### Task 5 (P1): Enforce Native vs RV32 Parity Gate

Why:
- README states both native and RV32 execution are validated, but parity is not yet enforced as a pass/fail gate.

What:
- Add deterministic parity checks on a curated high-signal fixture subset.

How:
- Execute identical fixtures on native and runner targets.
- Compare gas used, receipts root, logs bloom, and state root.

### Task 6 (P1): Continue Frontier-Driven Burn Down to Zero

Why:
- Full compatibility requires zero deterministic failures across supported fixture sets.

What:
- Repeat frontier-by-frontier elimination until full ignored sweep reaches zero failures/errors.

How:
- Loop:
  - run ignored release sweep,
  - capture first failing family,
  - add focused regression,
  - patch minimal semantic delta,
  - rerun focused tests and sweep.
