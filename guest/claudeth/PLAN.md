# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `README.md` claims:
  - full `execution-spec-tests` compatibility;
  - optimized minimal-dependency STF/EVM behavior;
  - native + RV32 test coverage.
- `cargo test -p claudeth --release` currently passes, but this is **not** a full conformance guarantee.
- Full blockchain-fixture sweep is still non-gating (`test_execute_all_blockchain_tests` remains `#[ignore]`).
- Focused regressions now pass for:
  - `BlockchainTests/InvalidBlocks/bcEIP1559/valCausesOOF.json::{..._Cancun,..._Prague}`
  - `BlockchainTests/InvalidBlocks/bcEIP1559/baseFee.json::{..._Cancun,..._Prague}`
- The next full-suite deterministic frontier after this turn is **not yet re-run** end-to-end.
- `BlockchainTests/InvalidBlocks/bcEIP1559/baseFee.json::{..._Cancun,..._Prague}` now passes.
- Explicit known feature gaps still present:
  - precompile `0x0a` point-evaluation not implemented;
  - precompile `0x08` non-trivial pairing intentionally unsupported.

## Completed Baseline Work

- Branch-accurate fixture parent header/state selection by `parent_hash`.
- Trie/withdrawal root conformance fixes (child ref threshold + withdrawal index keying).
- Prague EIP-7623 calldata floor gas support.
- Top-level CREATE success/failure state semantics alignment.
- Withdrawal-family gas fixes (stipend accounting, warm-set propagation, recursive revert gas).
- LOG base gas double-charge fix:
  - `log_gas_cost` now returns dynamic-only (`topics + data`), with base still charged in opcode dispatch.
  - Added Cancun/Prague regression tests for:
    - `bcInvalidHeaderTest/DifficultyIsZero`
    - `bcInvalidHeaderTest/timeDiff0`
  - Full release test suite and `prek run -a` pass.
- `bcEIP1559/baseFee` state-root fix:
  - Root cause: `RecursiveHost::call` transferred value for all call kinds when `msg.value != 0`.
  - Correct behavior restored: value transfer now gated to `CALL`/`CALLCODE` only (matches execution-spec `should_transfer_value` semantics; `DELEGATECALL`/`STATICCALL` never transfer).
  - Added host-level regressions for delegatecall value invariants:
    - with code
    - without code
    - to precompile
  - Added Cancun/Prague fixture regressions:
    - `bcEIP1559/baseFee.json::{baseFee_Cancun, baseFee_Prague}`
  - Ignored-suite frontier moved to `bcEIP1559/valCausesOOF`.
- `bcEIP1559/valCausesOOF` gas-used fix:
  - Root cause: comparison opcodes `LT/GT/SLT/SGT` used reversed operand order.
  - Consequence: fixture loop guarded by `GT(calldataload(4), 0)` was skipped, undercharging gas.
  - Fix: aligned comparison semantics to execution-spec operand order (top-of-stack compared against next).
  - Updated opcode/interpreter tests to enforce correct ordering.
  - Added Cancun/Prague fixture regressions:
    - `bcEIP1559/valCausesOOF.json::{valCausesOOF_Cancun, valCausesOOF_Prague}`

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST): Re-Baseline Ignored Full-Suite Frontier

Why:
- Priority must always follow the first deterministic full-suite failure after each fix.
- The previous frontier (`valCausesOOF`) is now fixed; the next blocker must be identified before further implementation.

What:
- Run the ignored blockchain suite and capture the new first deterministic failing family (fixture + error type + deltas).

How:
- Execute `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`.
- Record the first deterministic `✗` with compact tx-level summary.
- Add/adjust focused regression test(s) for the new frontier.

### Task 2 (P0): Fix the New First Deterministic Failure Family

Why:
- Full-suite hard gating requires clearing deterministic failures one family at a time.

What:
- Remove the next frontier mismatch identified in Task 1.

How:
- Iterate by first deterministic failure family, patch narrowly, add fixture regressions, then rerun ignored suite.

### Task 3 (P0): Implement Precompile `0x0a` Point Evaluation

Why:
- Cancun/Prague coverage is incomplete without EIP-4844 point-evaluation precompile.

What:
- Implement full semantics, input validation, output, and gas behavior.

How:
- Port execution-spec behavior and add success/failure/malformed/OOG vectors.

### Task 4 (P0): Implement Full BN254 Pairing (`0x08`)

Why:
- Current implementation rejects non-trivial tuples, preventing full conformance.

What:
- Support complete pairing product verification for all valid tuple sets.

How:
- Implement parsing/validation/execution for arbitrary tuple counts; add fixture vectors.

### Task 5 (P0): Make Full EELS Blockchain Suite a Hard Gate

Why:
- README “full compatibility” claim is not true while full-suite test is ignored.

What:
- Remove `#[ignore]` and enforce zero failures/errors in CI/local gating path.

How:
- After conformance blockers are removed, fail test on any `failed > 0 || errors > 0`.

### Task 6 (P1): Enforce Native vs RV32 Deterministic Parity Gate

Why:
- README promises dual-target behavior; parity must be executable and deterministic.

What:
- Add curated fixture parity checks between native and RV32 runner.

How:
- Build deterministic parity harness in release mode and gate once stable.
