# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `README.md` claims full `execution-spec-tests` compatibility and native + RV32 parity.
- `cargo test -p claudeth --release` passes locally.
- The full blockchain fixture sweep is still non-gating (`test_execute_all_blockchain_tests` is `#[ignore]`).
- Post-fix ignored-suite probe (`cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`) now shows this first deterministic failure family:
  - `BlockchainTests/ValidBlocks/bcExploitTest/StrangeContractCreation.json::{StrangeContractCreation_Cancun, StrangeContractCreation_Prague}`
  - error: `GasUsedMismatch(expected=764553, computed=724753)`
- After that first failure, the same probe also reports later failures (for example `reentrencySuicide`), but those are not the current frontier.
- Explicit known conformance gaps still present in code:
  - precompile `0x0a` point-evaluation unimplemented;
  - precompile `0x08` non-trivial pairing intentionally unsupported.

## Completed This Turn

- Re-baselined ignored-suite frontier and identified `mergeExample` as first deterministic failure.
- Fixed post-merge opcode `0x44` context wiring:
  - `src/stf/block.rs` now maps EVM block-context `difficulty` to header `mix_hash` when `header.difficulty == 0` (PREVRANDAO semantics).
- Added focused regressions:
  - `tests/eels_blockchain_tests.rs::{test_merge_example_cancun_fixture,test_merge_example_prague_fixture}`
- Re-ran ignored-suite probe and confirmed `mergeExample` now passes; frontier moved to `StrangeContractCreation`.

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST): Fix `StrangeContractCreation` Gas Mismatch

Why:
- It is the current first deterministic failure family after the latest re-baseline.
- Until this is fixed, full-suite conformance cannot progress in a deterministic order.

What:
- Align gas accounting for `BlockchainTests/ValidBlocks/bcExploitTest/StrangeContractCreation` (Cancun + Prague), removing `expected 764553 / computed 724753` mismatch.

How:
- Reproduce with focused fixture tests for both forks.
- Compare opcode-level gas flow against execution-spec behavior for the exact creation path exercised by this fixture.
- Patch the narrowest root-cause logic (no broad refactors), add dedicated regressions, and re-run focused tests.

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
- Resolve remaining deterministic failure families one by one (for example currently observed later family: `reentrencySuicide`).

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
