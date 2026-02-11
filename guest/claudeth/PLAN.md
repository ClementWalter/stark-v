# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `README.md` claims full EELS compatibility and native/RV32 parity.
- `test_execute_all_blockchain_tests` is still `#[ignore]`, and `run_all_blockchain_tests_impl` still does not assert `failed == 0 && errors == 0`.
- `cargo test -p claudeth --release` passes for the non-ignored suite.
- Fresh ignored-suite probe (`cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`) shows the first deterministic failing family at:
  - `BlockchainTests/ValidBlocks/bcStateTests/randomStatetest324.json::randomStatetest324_{Cancun,Prague}`
  - with `GasUsedMismatch(expected=6356176, computed=6356179)` on block 0 (`+3` gas).
- `testOpcode_a0` Cancun/Prague receipt-root failures were fixed by propagating successful child-call logs into parent tx receipts; focused regressions now pass.
- Additional downstream deterministic failures still appear after the new frontier (`testOpcode_f0::testOpcode_fa`, `refundReset`, and other state-test families).
- Precompile completeness remains behind README claims:
  - `0x0a` point-evaluation is still a reserved failure path.
  - `0x08` pairing still lacks full non-trivial pairing product verification.
  - Prague BLS12 execution precompiles `0x0b..0x11` are still not implemented.

## Completion Objective

Make implementation truthfully match `README.md` by:

- eliminating deterministic EELS fixture failures;
- enforcing full-suite compatibility as a default release gate;
- preserving deterministic native/RV32 behavior on representative fixture coverage.

## Priority Backlog (Why / What / How)

### Recently Completed

- Implemented child-call log propagation so successful nested `CALL*` logs are included in parent transaction receipts.
- Added focused regressions:
  - `test_opcode_a0_a2_cancun_fixture`
  - `test_opcode_a0_a2_prague_fixture`
- Verified in release mode:
  - focused tests pass;
  - `cargo test -p claudeth --release` passes.

### Task 1 (P0, FIRST): Fix `randomStatetest324` Gas Delta (`+3`, Cancun/Prague)

Why:
- This is now the first deterministic failing family in the ignored full-suite run.
- A stable `+3` gas delta usually indicates a single opcode-base or dynamic micro-accounting error, not random drift.

What:
- Make `randomStatetest324` Cancun/Prague gas-used exactly match fixture expectation.

How:
- Add focused regression tests for:
  - `BlockchainTests/ValidBlocks/bcStateTests/randomStatetest324.json::randomStatetest324_Cancun`
  - `BlockchainTests/ValidBlocks/bcStateTests/randomStatetest324.json::randomStatetest324_Prague`
- Compare per-opcode gas decomposition against execution-spec trace expectations around the first divergence.
- Patch only the minimal gas-accounting path that explains the deterministic `+3`.
- Re-run focused regressions and confirm no non-ignored suite regression in release mode.

### Task 2 (P0): Re-Baseline the New First Deterministic Frontier

Why:
- Fixing LOG0 receipt semantics may reveal a different highest-priority family immediately.

What:
- Capture the new first deterministic failing family after Task 1.

How:
- Rerun ignored full-suite probe in release mode.
- Stop triage at the first stable mismatch family and promote it to Task 1 in the next cycle.

### Task 3 (P0): Eliminate High-Signal Deterministic Execution Families

Why:
- README compatibility remains false until deterministic failure families are reduced to zero.

What:
- Resolve currently visible post-frontier families, starting with:
  - `testOpcode_f0.json::testOpcode_fa_{Cancun,Prague}` (large gas mismatch)
  - `refundReset` families
  - remaining random state-test gas/state mismatches.

How:
- Add focused regressions per family before broad reruns.
- Validate opcode-layer gas decomposition for each delta before patching.
- Rebaseline full suite after each deterministic family fix.

### Task 4 (P0): Burn Down Remaining Deterministic Execution Families

Why:
- README compatibility claim remains false until full-suite deterministic failures are zero.

What:
- Iteratively fix non-LOG frontiers currently visible after the first family (`testOpcode_f0::fa`, `randomState*` OOG/gas families, `refundReset`, `blockhashTests`, `suicideStorageCheckVCreate`, `callcodeOutput3partial`, etc.).

How:
- For each frontier:
  - reproduce with a focused fixture test;
  - map behavior to execution-spec;
  - implement minimal spec-accurate fix;
  - rerun ignored suite and re-prioritize.

### Task 5 (P0): Make Full EELS Sweep a Hard Default Gate

Why:
- As long as the compatibility sweep is ignored/non-fatal, regressions remain easy to miss.

What:
- Make full-suite compatibility a release-mode gate.

How:
- Remove non-gating behavior:
  - enforce `failed == 0 && errors == 0` assertions;
  - unignore or otherwise default-enable the full-suite gate in CI/local verification.

### Task 6 (P1): Implement Point-Evaluation Precompile (`0x0a`)

Why:
- Cancun/Prague conformance is incomplete while `0x0a` always fails.

What:
- Implement full validation/gas/output semantics for point-evaluation precompile.

How:
- Port execution-spec behavior exactly.
- Add malformed/success/OOG test vectors and fixture coverage.

### Task 7 (P1): Implement Full BN254 Pairing (`0x08`)

Why:
- Non-trivial pairing tuples are still intentionally rejected.

What:
- Implement full pairing product verification.

How:
- Add full tuple parsing + arithmetic verification path + gas-correct behavior.
- Add execution-spec vector coverage.

### Task 8 (P1): Implement Prague BLS12 Precompiles (`0x0b..0x11`)

Why:
- Prague warm-set behavior exists, but execution support is still missing.

What:
- Implement execution and gas semantics for BLS12 precompile range.

How:
- Port execution-spec rules for each address.
- Add malformed/success/OOG tests and representative fixture regressions.

### Task 9 (P1): Enforce Native vs RV32 Deterministic Parity Gate

Why:
- README claims parity, but no hard deterministic parity gate exists for high-signal fixtures.

What:
- Add deterministic parity assertions over a curated fixture subset.

How:
- Run identical fixtures through native and runner flows in release mode.
- Fail on any state/gas/log divergence.
