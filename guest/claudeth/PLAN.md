# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `README.md` claims full EELS compatibility and native/RV32 parity.
- Release-mode focused regressions now pass for the previous first frontier:
  - `randomStatetest324_{Cancun,Prague}`.
- Latest ignored full-suite probe (`cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`) now advances past `randomStatetest324` and first fails at:
  - `BlockchainTests/ValidBlocks/bcStateTests/testOpcode_f0.json::testOpcode_fa_Prague`
  - `BlockchainTests/ValidBlocks/bcStateTests/testOpcode_f0.json::testOpcode_fa_Cancun`
  - with `GasUsedMismatch(expected=9889566, computed=10025861)` (`+136295`) on block 0.
- Full-suite conformance remains non-gating:
  - `test_execute_all_blockchain_tests` is still `#[ignore]`.
  - `run_all_blockchain_tests_impl` still does not assert `failed == 0 && errors == 0`.
- Precompile completeness still trails README expectations:
  - `0x0a` point-evaluation: reserved failure path.
  - `0x08` pairing: partial path, not full non-trivial verification.
  - Prague BLS12 range `0x0b..0x11`: not implemented.

## Completion Objective

Make implementation truthfully match `README.md` by:

- eliminating deterministic EELS fixture failures;
- enforcing full-suite compatibility as a default release gate;
- preserving deterministic native/RV32 behavior on representative fixture coverage.

## Recently Completed

- Fixed zero-length memory range gas accounting:
  - memory expansion now matches execution-spec behavior (`size == 0` => no expansion cost).
- Added reusable gas helper:
  - `memory_expansion_cost_for_range`.
- Updated interpreter/opcode memory-range charging to use spec-aligned range semantics.
- Added focused fixture regressions:
  - `test_random_statetest324_cancun_fixture`
  - `test_random_statetest324_prague_fixture`
- Verified in release mode:
  - `cargo test -p claudeth --release random_statetest324 -- --nocapture` passes.

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST): Fix `testOpcode_fa` Gas Delta (`+136295`, Cancun/Prague)

Why:
- This is now the first deterministic failing family in the ignored full-suite run.
- Large stable gas deltas in opcode-family fixtures usually indicate systematic call/system gas path drift (not random noise).

What:
- Make `testOpcode_f0.json::testOpcode_fa_{Cancun,Prague}` gas-used exactly match fixture expectations.

How:
- Add focused regressions for:
  - `BlockchainTests/ValidBlocks/bcStateTests/testOpcode_f0.json::testOpcode_fa_Cancun`
  - `BlockchainTests/ValidBlocks/bcStateTests/testOpcode_f0.json::testOpcode_fa_Prague`
- Read execution-spec references for F0-family opcode gas behavior (CREATE/CALL*/RETURN/REVERT/SELFDESTRUCT interaction paths used by this vector).
- Compare opcode-level gas decomposition at first divergence.
- Patch only minimal spec-accurate gas path(s) that explain the deterministic `+136295`.
- Re-run focused regressions and non-ignored release suite.

### Task 2 (P0): Re-Baseline the New Deterministic Frontier

Why:
- Fixing `testOpcode_fa` is likely to expose the next deterministic family immediately.

What:
- Capture the next first deterministic failing family after Task 1.

How:
- Re-run ignored full-suite probe in release mode.
- Stop triage at first stable mismatch and promote it to Task 1 in the next cycle.

### Task 3 (P0): Burn Down Remaining Deterministic Execution Families

Why:
- README compatibility remains false until deterministic failures reach zero.

What:
- Iteratively eliminate deterministic fixture families after each frontier move.

How:
- For each new frontier:
  - add focused regression;
  - map behavior against execution-spec;
  - implement minimal fix;
  - rebaseline ignored full suite.

### Task 4 (P0): Make Full EELS Sweep a Hard Default Gate

Why:
- Ignored/non-fatal full-suite behavior hides regressions.

What:
- Make EELS compatibility mandatory in default verification.

How:
- Remove non-gating behavior:
  - enforce `failed == 0 && errors == 0` assertions;
  - unignore or otherwise default-enable full-suite gating in CI/local flow.

### Task 5 (P1): Implement Point-Evaluation Precompile (`0x0a`)

Why:
- Cancun/Prague conformance is incomplete while `0x0a` intentionally fails.

What:
- Implement full validation/gas/output semantics for point-evaluation.

How:
- Port execution-spec behavior exactly.
- Add malformed/success/OOG vectors and fixture coverage.

### Task 6 (P1): Implement Full BN254 Pairing (`0x08`)

Why:
- Non-trivial pairing tuples are still intentionally rejected.

What:
- Implement full pairing product verification.

How:
- Add full tuple parsing + arithmetic verification + gas-correct behavior.
- Add execution-spec vector coverage.

### Task 7 (P1): Implement Prague BLS12 Precompiles (`0x0b..0x11`)

Why:
- Prague warm-set handling exists, but execution paths are missing.

What:
- Implement execution and gas semantics for BLS12 precompile range.

How:
- Port execution-spec rules per address.
- Add malformed/success/OOG tests and representative fixture regressions.

### Task 8 (P1): Enforce Native vs RV32 Deterministic Parity Gate

Why:
- README claims parity, but no hard deterministic parity gate exists.

What:
- Add deterministic parity assertions over curated high-signal fixtures.

How:
- Run identical fixtures through native and runner flows in release mode.
- Fail on any state/gas/log divergence.
