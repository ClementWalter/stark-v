# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `README.md` claims full `execution-spec-tests` compatibility and native + RV32 parity.
- `cargo test -p claudeth --release` passes locally.
- The full blockchain fixture sweep is still non-gating (`test_execute_all_blockchain_tests` is `#[ignore]`).
- Post-fix ignored-suite probe (`cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`) now shows this first deterministic failure family:
  - `BlockchainTests/ValidBlocks/bcValidBlockTest/reentrencySuicide.json::reentrencySuicide_Prague`
  - error: `GasUsedMismatch(expected=109012, computed=111612)` (`+2600`)
- Cancun for the same fixture currently passes; the frontier is Prague-specific.
- Explicit known conformance gaps still present in code:
  - precompile `0x0a` point-evaluation unimplemented;
  - precompile `0x08` non-trivial pairing intentionally unsupported.

## Completed This Turn

- Added focused regression coverage for:
  - `tests/eels_blockchain_tests.rs::{test_strange_contract_creation_cancun_fixture,test_strange_contract_creation_prague_fixture}`
- Fixed `StrangeContractCreation` root cause:
  - `src/evm/opcodes/arithmetic.rs`: corrected `EXP` operand order to execution-spec semantics (`base=top`, `exponent=next`).
- Hardened recursive CREATE collision semantics:
  - `src/evm/host.rs`: immediate collision failure now increments creator nonce, burns forwarded gas, and skips init-code execution.
  - `src/state/execution.rs`: added explicit `has_storage` to distinguish storage-collision from balance-only accounts.
- Added host-level regression tests for CREATE collision/non-collision edge cases:
  - `src/evm/host.rs::{test_recursive_host_create_collision_burns_forwarded_gas,test_recursive_host_create_balance_only_target_is_not_collision}`
- Re-ran ignored-suite frontier probe and confirmed `StrangeContractCreation` now passes; frontier moved to `reentrencySuicide_Prague`.

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST): Fix `reentrencySuicide_Prague` Gas Mismatch (`+2600`)

Why:
- It is the current first deterministic failure family after the latest re-baseline.
- Until this is fixed, full-suite conformance cannot progress in a deterministic order.

What:
- Align Prague gas accounting for:
  - `BlockchainTests/ValidBlocks/bcValidBlockTest/reentrencySuicide.json::reentrencySuicide_Prague`
  - remove `expected 109012 / computed 111612` mismatch.

How:
- Add focused Cancun/Prague fixture regressions for `reentrencySuicide`.
- Compare Prague-vs-Cancun opcode-level gas flow; treat `+2600` as a likely single cold-account overcharge signal.
- Patch the narrowest root-cause logic, validate fixture post-state, and re-run focused tests.

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
