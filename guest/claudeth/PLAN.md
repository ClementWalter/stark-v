# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `cargo test -p claudeth --release` passes.
- Full EELS blockchain sweep is still non-gating (`test_execute_all_blockchain_tests` is `#[ignore]`).
- Fresh ignored-suite probe after this turn’s stipend/warm-set/revert-gas fixes shows:
  - withdrawal family now passes:
    - `bc4895-withdrawals/accountInteractions` Cancun/Prague
    - `bc4895-withdrawals/warmup` Cancun/Prague
  - first remaining deterministic failures are now:
    - `BlockchainTests/InvalidBlocks/bcInvalidHeaderTest/DifficultyIsZero.json::{..._Cancun,..._Prague}`
    - `BlockchainTests/InvalidBlocks/bcInvalidHeaderTest/timeDiff0.json::{..._Cancun,..._Prague}`
    - both with `GasUsedMismatch`: expected `22027`, computed `22402` (delta `+375`)
- Known explicit implementation gaps still present:
  - precompile `0x0a` point-evaluation not implemented;
  - precompile `0x08` non-trivial pairing still intentionally unsupported.

## Completed Before This Turn

### A. Branch-Accurate Parent Header/State Selection in EELS Harness

Why:
- Forked fixture chains cannot be executed with linear loop-order parent/state selection.

What:
- Parent header and parent state are both resolved by `parent_hash`.

How:
- Added hash-indexed header/state maps and canonical lookup by header `parent_hash`.

### B. Trie / Withdrawal Root Conformance Fixes

Why:
- Root mismatches occurred from non-canonical trie child-reference behavior and withdrawal keying.

What:
- Aligned trie child reference and withdrawal trie indexing behavior with execution-spec expectations.

How:
- Updated trie reference encoding threshold behavior and withdrawal key construction.

### C. Prague EIP-7623 Calldata Floor Gas Rules

Why:
- Prague gas accounting mismatched without calldata-floor validation/flooring.

What:
- Implemented floor-gas validation and post-refund floor application.

How:
- Added floor helpers, enforced `max(intrinsic_gas, calldata_floor_gas_cost)`, and floored final gas used.

### D. Top-Level CREATE Success/Failure State Semantics

Why:
- `CreateTransactionReverted` fixtures failed with state-root mismatch due top-level CREATE state handling.

What:
- Aligned top-level CREATE semantics with execution-spec for created-account nonce and failure behavior.

How:
- Incremented created account nonce to `1` only on successful deployment.
- Returned `contract_address = None` on failed top-level CREATE.
- Ensured failed create paths return pre-execution state snapshot.
- Added Cancun/Prague fixture regressions for `CreateTransactionReverted`.

### E. Withdrawal Family Gas Conformance (`accountInteractions`, `warmup`)

Why:
- First deterministic ignored-suite failures were gas deltas `+2300` and `+56400`.
- Deltas were rooted in three coupled call-path issues: stipend billing, warm-set propagation, and revert gas accounting in recursive host paths.

What:
- Aligned call stipend billing semantics for `CALL`/`CALLCODE`.
- Propagated EIP-2929 warm sets across successful child frames.
- Corrected recursive host revert gas accounting for `CALL` and `CREATE`.

How:
- Charged caller by `child_gas_used - stipend` and credited unused stipend when applicable.
- Extended call messages/results with accessed addresses/storage and merged successful child warm sets into parent frame.
- Handled `EvmError::Revert` as non-OOG failure with gas used based on remaining child gas (`msg.gas - evm.gas_remaining()`).
- Added fixture regressions for withdrawal `accountInteractions` and `warmup` (Cancun/Prague).
- Added interpreter-level stipend unit tests.

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST): Fix `bcInvalidHeaderTest` `+375` Gas Delta (`DifficultyIsZero`, `timeDiff0`)

Why:
- This is now the first deterministic failure family in the latest ignored-suite probe.
- Same delta appears in two fixtures and both Cancun/Prague forks, indicating a single shared gas accounting edge case.

What:
- Eliminate the `+375` overcharge in `bcInvalidHeaderTest` `DifficultyIsZero` and `timeDiff0` for Cancun/Prague.

How:
- Read both fixture transactions and the matching execution-spec gas path for every touched opcode and transaction-level gas component.
- Build a transaction-level gas decomposition to isolate the exact `375` discrepancy source.
- Patch one coherent rule in STF/EVM gas accounting and add focused Cancun/Prague fixture regressions for both files.

### Task 2 (P0): Close Remaining GasUsedMismatch Families After Task 1

Why:
- Gas accounting remains the dominant blocker before hard-gating full-suite conformance.

What:
- Remove residual gas accounting divergences outside the withdrawal family.

How:
- Re-run ignored suite, cluster by delta signature and fixture family, patch rule-by-rule with fixture regressions.

### Task 3 (P0): Resolve Residual Valid-Block State Root Mismatches

Why:
- Any valid-block state-root mismatch is consensus-critical.

What:
- Remove remaining state transition divergences after gas accounting stabilizes.

How:
- Start from smallest failing valid fixtures and compare account/storage/code deltas against execution-spec outcomes.

### Task 4 (P0): Implement Precompile `0x0a` Point Evaluation

Why:
- Cancun/Prague conformance is incomplete while point-evaluation precompile remains unimplemented.

What:
- Implement point-evaluation semantics, validation, and gas behavior.

How:
- Port execution-spec behavior and add success/failure/malformed/OOG vectors.

### Task 5 (P0): Implement Full Non-Trivial BN254 Pairing (`0x08`)

Why:
- Current pairing implementation intentionally rejects non-trivial tuples.

What:
- Implement complete pairing product verification and canonical output behavior.

How:
- Add full tuple parsing/validation/execution with multi-tuple conformance tests.

### Task 6 (P0): Make Full EELS Blockchain Suite a Hard Gate

Why:
- README claims full EELS compatibility, but global compatibility test is still ignored.

What:
- Turn full-suite runner into mandatory zero-failure coverage.

How:
- After Tasks 1-5 land, remove `#[ignore]` and assert `failed == 0 && errors == 0`.

### Task 7 (P1): Enforce Native vs RV32 Deterministic Parity Gate

Why:
- The dual-target claim needs automated parity enforcement.

What:
- Add deterministic native/RV32 parity checks over curated fixtures.

How:
- Build reproducible parity driver and gate it once stable.
