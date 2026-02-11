# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `README.md` claims full EELS compatibility and no caveats.
- Default release suite is green:
  - `cargo test -p claudeth --release`
- Full EELS sweep is still non-gating:
  - `test_execute_all_blockchain_tests` remains `#[ignore]`
  - `run_all_blockchain_tests_impl` still reports totals without asserting `failed == 0 && errors == 0`
- Fresh ignored sweep re-baseline after the `randomStatetest241` fix confirms the current first unresolved deterministic frontier is:
  - `BlockchainTests/ValidBlocks/bcStateTests/blockhashTests.json::{blockhashTests_Cancun,blockhashTests_Prague}`
  - `Block 3: GasUsedMismatch(expected=45352, computed=65252)` (`+19900`)
- Next observed unresolved deterministic family in the same probe:
  - `BlockchainTests/ValidBlocks/bcStateTests/suicideStorageCheckVCreate.json::{suicideStorageCheckVCreate_Cancun,suicideStorageCheckVCreate_Prague}`
  - `Block 0: GasUsedMismatch(expected=468193, computed=184878)` (`-283315`)
- Source audit highlights one explicit compatibility gap that still conflicts with README:
  - `src/evm/precompiles.rs` has non-trivial bn254 pairing pending and precompile `0x0a` intentionally unimplemented.

## Recently Confirmed

- `logRevert` is fixed and now passes in both forks during ignored-sweep execution.
- `refundReset` is fixed in focused regressions:
  - `cargo test -p claudeth --release test_refund_reset_ -- --nocapture`
  - Cancun + Prague now pass.
- `randomStatetest241` is fixed in focused regressions:
  - `cargo test -p claudeth --release test_random_statetest241_ -- --nocapture`
  - Cancun + Prague now pass.
- Root cause for `randomStatetest241` was closed:
  - PUSH immediates now follow execution-spec `buffer_read` semantics (right-pad with zeros at EOF) instead of halting with `InvalidPush`.

## Completion Objective

Make implementation truthfully match `README.md` by:

- eliminating deterministic EELS fixture failures;
- turning full EELS execution into a default release gate;
- enforcing native/RV32 parity gates;
- closing remaining spec-semantic gaps (including precompile completeness).

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST): Fix `blockhashTests` Gas Mismatch

Why:
- This is now the first deterministic failure family after fixing `randomStatetest241`.
- Its `+19900` gas delta indicates a reproducible semantic mismatch that blocks full EELS parity.

What:
- Make `blockhashTests_Cancun` and `blockhashTests_Prague` match expected gas used.
- Add focused regression coverage that protects against recurrence.

How:
- Read `execution-specs` references for `BLOCKHASH` and any related historical-hash plumbing used in this fixture.
- Reproduce with focused fixture tests and (when needed) trace output to isolate the first divergent opcode-level charge.
- Patch the minimum semantic delta in host/interpreter/state plumbing.
- Validate with:
  - `cargo test -p claudeth --release test_blockhash_ -- --nocapture` (or equivalent focused case names)
  - `cargo test -p claudeth --release test_random_statetest241_ -- --nocapture`
  - `cargo test -p claudeth --release`

### Task 2 (P0): Burn Down Remaining Deterministic Fixture Failures to Zero

Why:
- README compatibility remains false while any deterministic fixture family fails.

What:
- Eliminate every remaining deterministic mismatch family-by-family.

How:
- Repeat fixed cycle:
  - capture frontier;
  - add focused regression;
  - diff behavior against execution-spec references;
  - apply minimal patch;
  - rerun focused + release suite.

Initial known post-Task-1 candidate (already observed):
- `suicideStorageCheckVCreate` gas mismatch family.

### Task 3 (P0): Make Full EELS Sweep a Default Hard Gate

Why:
- Ignored/non-fatal full-suite behavior permits silent regressions and contradicts README claims.

What:
- Default verification must fail when any EELS fixture fails/errors.

How:
- Remove ignore posture for default gate path.
- Enforce `failed == 0 && errors == 0` in full-suite runner.
- Keep release-mode execution for this gate.

### Task 4 (P1): Add Deterministic Native-vs-RV32 Parity Gate

Why:
- README states both native and RV32 paths are validated, but there is no hard deterministic parity gate yet.

What:
- Add a curated parity suite that must match across both execution targets.

How:
- Run the same fixture set on native and runner paths.
- Fail on divergence in state root, receipts, gas used, and logs.

### Task 5 (P1): Complete Outstanding Precompile Semantics

Why:
- `src/evm/precompiles.rs` still documents intentionally incomplete behavior (`0x08` non-trivial pairing and `0x0a` point-evaluation).
- This is incompatible with README’s full-compatibility claim.

What:
- Implement remaining precompile semantics to match execution-spec behavior.

How:
- Port logic from execution-spec references exactly.
- Add malformed/success/OOG regressions.
- Re-run relevant fixture families and full release suite.
