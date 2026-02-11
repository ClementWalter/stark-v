# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `README.md` claims:
  - full `execution-spec-tests` compatibility;
  - optimized minimal-dependency STF/EVM behavior;
  - native + RV32 test coverage.
- `cargo test -p claudeth --release` currently passes, but this is **not** a full conformance guarantee.
- Full blockchain-fixture sweep is still non-gating (`test_execute_all_blockchain_tests` remains `#[ignore]`).
- Fresh ignored-suite probe after this turn’s LOG fix advanced past `bcInvalidHeaderTest` and now first reports:
  - `BlockchainTests/InvalidBlocks/bcEIP1559/baseFee.json::{..._Cancun,..._Prague}`
  - `StateRootMismatch` on block 1 with all transactions successful.
- Next deterministic failures seen immediately after:
  - `BlockchainTests/InvalidBlocks/bcEIP1559/valCausesOOF.json::{..._Cancun,..._Prague}`
  - `GasUsedMismatch expected=323796 computed=247996` on block 10.
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

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST): Fix `bcEIP1559/baseFee` State Root Mismatch (Cancun/Prague)

Why:
- This is now the first deterministic full-suite failure after the LOG fix.
- It is a valid-execution state divergence (all txs succeed), so consensus-critical.

What:
- Make `bcEIP1559/baseFee` block 1 state transition match execution-spec post-state/root for Cancun and Prague.

How:
- Inspect fixture transactions and per-tx balance/base-fee accounting against execution-spec EIP-1559 rules.
- Build account-level pre/post diff for touched addresses (sender/coinbase/recipients) to isolate mismatch source.
- Patch one coherent state-transition rule and add dedicated Cancun/Prague fixture regressions for `baseFee`.

### Task 2 (P0): Fix `bcEIP1559/valCausesOOF` GasUsed Mismatch (Cancun/Prague)

Why:
- This is the next deterministic family observed right after `baseFee`.

What:
- Remove the `323796 vs 247996` gas-used divergence on block 10.

How:
- Decompose tx-level gas on the failing block and compare call OOG semantics with execution-spec behavior.
- Add fixture regressions for both forks once the accounting rule is corrected.

### Task 3 (P0): Continue State Root / Gas Mismatch Burn-Down After Tasks 1-2

Why:
- Full-suite hard gating requires clearing all deterministic mismatch families, not isolated fixes.

What:
- Eliminate remaining state-root and gas-used divergences exposed by the ignored suite.

How:
- Iterate by first deterministic failure family, patch narrowly, add fixture regressions, then rerun ignored suite.

### Task 4 (P0): Implement Precompile `0x0a` Point Evaluation

Why:
- Cancun/Prague coverage is incomplete without EIP-4844 point-evaluation precompile.

What:
- Implement full semantics, input validation, output, and gas behavior.

How:
- Port execution-spec behavior and add success/failure/malformed/OOG vectors.

### Task 5 (P0): Implement Full BN254 Pairing (`0x08`)

Why:
- Current implementation rejects non-trivial tuples, preventing full conformance.

What:
- Support complete pairing product verification for all valid tuple sets.

How:
- Implement parsing/validation/execution for arbitrary tuple counts; add fixture vectors.

### Task 6 (P0): Make Full EELS Blockchain Suite a Hard Gate

Why:
- README “full compatibility” claim is not true while full-suite test is ignored.

What:
- Remove `#[ignore]` and enforce zero failures/errors in CI/local gating path.

How:
- After conformance blockers are removed, fail test on any `failed > 0 || errors > 0`.

### Task 7 (P1): Enforce Native vs RV32 Deterministic Parity Gate

Why:
- README promises dual-target behavior; parity must be executable and deterministic.

What:
- Add curated fixture parity checks between native and RV32 runner.

How:
- Build deterministic parity harness in release mode and gate once stable.
