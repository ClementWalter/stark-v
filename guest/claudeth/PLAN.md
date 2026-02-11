# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `README.md` claims full `execution-spec-tests` compatibility and native + RV32 parity.
- `cargo test -p claudeth --release` passes locally.
- The full blockchain fixture sweep is still non-gating (`test_execute_all_blockchain_tests` is `#[ignore]`).
- `reentrencySuicide` Cancun/Prague now pass with focused regressions in release mode.
- Post-fix ignored-suite baseline rerun (stopped after first deterministic failures) now surfaces this frontier:
  - `BlockchainTests/ValidBlocks/bcStateTests/suicideStorageCheck.json::suicideStorageCheck_Cancun`
  - `BlockchainTests/ValidBlocks/bcStateTests/suicideStorageCheck.json::suicideStorageCheck_Prague`
  - error: `GasUsedMismatch(expected=473109, computed=172603)` (`-300506`)
- Immediate next deterministic family observed right after frontier:
  - CREATE2 + selfdestruct same-block collision fixture (Cancun/Prague)
  - gas undercharge on block 0 tx pair (`~ -64886`).
- Explicit known conformance gaps still present in code:
  - precompile `0x0a` point-evaluation unimplemented;
  - precompile `0x08` non-trivial pairing intentionally unsupported;
  - Prague BLS12 precompile execution (`0x0b..0x11`) unimplemented.

## Completion Objective

Make the implementation actually match `README.md` claims:
- pass the full `execution-spec-tests` blockchain fixture sweep;
- keep native and RV32 deterministic parity on the supported fixture set;
- keep this conformance enforced by default test gates.

## Completed This Turn

- Implemented fork-aware precompile warm-set propagation:
  - `src/evm/interpreter.rs`: added `BlockContext.max_precompile_address` and used it for top-level warm initialization.
  - `src/evm/host.rs`: applied the same warm range to recursive `CALL*` and `CREATE*` child frames.
  - `src/stf/block.rs`: set warm range from block fork signal (`requests_hash`: Prague `0x11`, otherwise `0x0a`).
- Added focused regression tests:
  - `src/evm/interpreter.rs`: `test_extcodesize_precompile_0x0b_cold_before_prague`, `test_extcodesize_precompile_0x0b_warm_at_prague`.
  - `tests/eels_blockchain_tests.rs`: Cancun/Prague `reentrencySuicide` fixture tests.
- Validation completed in release mode:
  - focused new unit + fixture tests pass;
  - `cargo test -p claudeth --release` passes;
  - `prek run -a` passes.

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST): Fix `suicideStorageCheck` Gas Undercharge (`-300506`)

Why:
- It is now the first deterministic failure family after fixing `reentrencySuicide`.
- Until this is fixed, full-suite conformance cannot progress in a deterministic order.

What:
- Align gas accounting and state-transition behavior for:
  - `BlockchainTests/ValidBlocks/bcStateTests/suicideStorageCheck.json::{..._Cancun,..._Prague}`
  - remove `expected 473109 / computed 172603` block gas mismatch.

How:
- Add dedicated Cancun/Prague focused fixture tests for `suicideStorageCheck`.
- Diff expected-vs-computed tx-level gas and isolate missing charged paths in tx0/tx1.
- Cross-check with execution-spec `SELFDESTRUCT` + `CREATE/CREATE2` collision/deletion semantics and patch narrow root cause.

### Task 2 (P0): Re-Baseline Ignored Full-Suite Frontier After Task 1

Why:
- Deterministic conformance work must always follow the first failing family after each fix.

What:
- Re-run ignored suite and capture the next first deterministic `✗` family with compact tx-level deltas.

How:
- Execute `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`.
- Stop analysis at first deterministic mismatch and prioritize that family next.

### Task 3 (P0): Iterate Failure-Family Burn-Down to Zero

Why:
- README compatibility claim cannot be considered true until deterministic blockchain fixture mismatches are eliminated.

What:
- Resolve remaining deterministic failure families one by one (for example the currently observed later selfdestruct valid-block family).

How:
- For each frontier: root-cause -> minimal patch -> focused Cancun/Prague regressions -> re-baseline.

### Task 4 (P0): Make Full Blockchain Suite a Hard Gate

Why:
- `#[ignore]` leaves the key compatibility claim unenforced.

What:
- Remove ignore/non-gating behavior and fail on any `failed > 0 || errors > 0`.

How:
- After deterministic failures are cleared, tighten assertions in `run_all_blockchain_tests_impl` and enable in normal CI/local path.

### Task 5 (P1): Implement Precompile `0x0a` Point Evaluation

Why:
- Cancun/Prague precompile coverage remains incomplete.

What:
- Implement full semantics, validation, and gas metering for point-evaluation precompile.

How:
- Port execution-spec behavior and add malformed/success/OOG vectors.

### Task 6 (P1): Implement Full BN254 Pairing (`0x08`)

Why:
- Non-trivial pairing tuples currently fail by design, blocking full conformance.

What:
- Support complete pairing product verification for arbitrary valid tuple sets.

How:
- Implement full tuple parsing + arithmetic path; add fixture and unit coverage.

### Task 7 (P1): Enforce Native vs RV32 Deterministic Parity Gate

Why:
- README promises native + RV32 execution parity.

What:
- Add deterministic parity checks on a curated high-signal fixture set.

How:
- Execute identical vectors through native and runner paths in release mode and gate once stable.

### Task 8 (P1): Implement Prague BLS12 Precompile Execution (`0x0b..0x11`)

Why:
- Prague execution-spec includes BLS12 precompile addresses in the precompile map.
- Warm-set parity alone fixes current frontier gas, but execution correctness for direct calls remains incomplete.

What:
- Implement functional + gas-correct handling for BLS12 precompile addresses (`G1 add/msm`, `G2 add/msm`, pairing, map ops).

How:
- Port execution-spec behavior first for validation/error/gas rules.
- Add targeted vectors for malformed input, OOG, and success cases before enabling broad fixture gates.
