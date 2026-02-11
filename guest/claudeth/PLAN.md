# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `cargo test -p claudeth --release` passes.
- Full EELS blockchain sweep is still non-gating (`test_execute_all_blockchain_tests` is `#[ignore]`).
- Latest ignored full-suite probe (2026-02-11) confirms an immediate deterministic Prague mismatch:
  - `BlockchainTests/InvalidBlocks/bcMultiChainTest/UncleFromSideChain.json::UncleFromSideChain_Prague`
  - `GasUsedMismatch`: expected `42160`, computed `42064` (delta `96`).
- Execution-spec reference (`execution-specs/src/ethereum/forks/prague`) requires EIP-7623 calldata floor charging:
  - validation requires `tx.gas >= max(intrinsic_gas, calldata_floor_gas_cost)`;
  - post-execution gas used is floored with `max(gas_used_after_refund, calldata_floor_gas_cost)`.
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

### C. Baseline EELS Harness Stability

Why:
- Full ignored runs previously aborted before yielding useful failure diagnostics.

What:
- Bounded block-error summaries and increased full-suite runner stack.

How:
- Added compact transaction-summary formatting and ran ignored sweep in large-stack thread.

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST): Implement Prague EIP-7623 Calldata Floor Gas Rules

Why:
- Current first deterministic failure is Prague-only and exactly matches missing floor-gas semantics (`+96` on 4 non-zero calldata bytes).
- Without this, Prague gas accounting diverges across many fixtures.

What:
- Implement EIP-7623 calldata floor gas behavior in transaction validation and final gas accounting.

How:
- Add calldata-token/floor-gas helpers.
- Enforce `gas_limit >= max(intrinsic_gas, calldata_floor_gas_cost)` for Prague blocks.
- Apply post-refund floor: `final_gas_used = max(final_gas_used, calldata_floor_gas_cost)`.
- Add focused regression tests, including `UncleFromSideChain_Prague`.

### Task 2 (P0): Fix Remaining CREATE Transaction State Semantics

Why:
- Prior full-suite baselines include deterministic CREATE-related root mismatches (`CreateTransactionReverted` family).

What:
- Align contract-creation nonce/account persistence semantics on all success/failure paths.

How:
- Reproduce with targeted fixtures, diff against execution-spec state transitions, and add focused nonce/account regression tests.

### Task 3 (P0): Close Residual Gas Deltas in Cancun/Prague Fixtures

Why:
- `GasUsedMismatch` remains the dominant unresolved class after branch-handling fixes.

What:
- Eliminate remaining discrepancies in per-tx gas accounting beyond EIP-7623.

How:
- Cluster failures by delta signature and fixture family, patch one rule-family at a time, and add fixture regressions per patch.

### Task 4 (P0): Resolve Remaining Valid-Block State Root Mismatches

Why:
- Any valid-block root mismatch is consensus-critical.

What:
- Remove remaining state-transition divergences after gas alignment.

How:
- Start from smallest failing valid fixtures and compare per-account/storage/code deltas against execution-spec outcomes.

### Task 5 (P0): Implement Precompile `0x0a` Point Evaluation

Why:
- Cancun/Prague conformance remains incomplete while this precompile is intentionally unimplemented.

What:
- Implement full point-evaluation semantics with validation and gas behavior.

How:
- Port execution-spec behavior and add success/failure/malformed/OOG vectors.

### Task 6 (P0): Implement Full Non-Trivial BN254 Pairing (`0x08`)

Why:
- Current implementation intentionally rejects non-trivial tuples.

What:
- Implement full pairing product verification and canonical outputs.

How:
- Implement complete tuple handling/validation and add multi-tuple conformance vectors.

### Task 7 (P0): Make Full EELS Blockchain Suite a Hard Gate

Why:
- README claims full EELS compatibility, but the only global compatibility check is still ignored.

What:
- Turn ignored full-suite runner into a mandatory zero-failure test.

How:
- After Tasks 1-6 land, remove `#[ignore]` and assert `failed == 0 && errors == 0`.

### Task 8 (P1): Enforce Native vs RV32 Deterministic Parity Gate

Why:
- Dual-target claim needs automated parity enforcement.

What:
- Add deterministic native/RV32 parity checks over curated fixtures.

How:
- Add reproducible driver and gate in CI once stable.
