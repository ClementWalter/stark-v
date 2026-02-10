# Claudeth Completion Plan

Last reviewed: 2026-02-10

## Ground Truth Snapshot

- `cargo test -p claudeth --release` currently passes.
- `README.md` is still not aligned with current implementation:
  - says "no dependencies" but `Cargo.toml` uses `serde`;
  - says full `execution-spec-tests` compatibility, but the blockchain harness is still partial and non-assertive.
- Precompile status from `src/evm/precompiles.rs`:
  - implemented: `0x01..0x07`, `0x09`;
  - partially implemented: `0x08` only supports envelope semantics + empty-input success;
  - not implemented: `0x0a` (`POINT_EVALUATION`) is reserved-fail.
- EELS harness status from `tests/eels_blockchain_tests.rs`:
  - now parses and iterates all discovered blockchain fixtures;
  - now includes `InvalidBlocks` in discovery and basic `expectException` handling in execution flow;
  - execution test is still `#[ignore]`;
  - parent hash is still overwritten with a local workaround;
  - final failure assertions are still disabled.

## Completed This Turn

- Expanded blockchain fixture coverage in `tests/eels_blockchain_tests.rs`:
  - removed `.take(10)` limits from parser and executor loops;
  - stopped skipping `InvalidBlocks` at discovery;
  - added explicit `expectException` handling during block execution paths.
- Re-ran quality gates:
  - `cargo test -p claudeth --release`;
  - `prek run --all-files`.

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST): Implement Full ALT_BN128 Pairing Arithmetic (`0x08`, Non-Empty Input)

Why:
- EIP-197 correctness is currently missing for real pairing tuples.

What:
- Implement full tuple decoding and pairing-product validation.

How:
- Follow `execution-specs/src/ethereum/forks/cancun/vm/precompiled_contracts/alt_bn128.py`.
- Implement `bytes_to_g2`-equivalent decoding (four field elements, endian/order parity with spec).
- Enforce curve membership and subgroup checks (`multiply(point, curve_order)` at infinity).
- Return canonical output (`U256(1)` / `U256(0)`), while keeping malformed input behavior as precompile failure.

### Task 2 (P0): Implement POINT_EVALUATION Precompile (`0x0a`)

Why:
- EIP-4844/Cancun correctness requires this precompile.

What:
- Implement full `POINT_EVALUATION` behavior.

How:
- Follow `execution-specs/src/ethereum/forks/cancun/vm/precompiled_contracts/point_evaluation.py` and `execution-specs/src/ethereum/crypto/kzg.py`.
- Enforce strict 192-byte input shape and fixed gas.
- Implement `kzg_commitment_to_versioned_hash`.
- Integrate trusted KZG proof verification (`verify_kzg_proof`) equivalent semantics.
- Return `[FIELD_ELEMENTS_PER_BLOB, BLS_MODULUS]` as two 32-byte big-endian words on success.

### Task 3 (P0): Remove Remaining Harness Workarounds and Enforce Assertions

Why:
- The current harness still cannot prove compatibility because it mutates parent linkage and never fails CI.

What:
- Convert `test_execute_all_blockchain_tests` from debug harness to enforceable correctness check.

How:
- Remove the parent-hash overwrite workaround and validate canonical parent linkage.
- Turn final summary checks into hard assertions (`failed == 0`, `errors == 0`) once known gaps are resolved.
- Keep failure diagnostics, but fail deterministically.

### Task 4 (P0): Add Fork-Aware Rule Gating (Precompiles + Header/Tx Rules)

Why:
- EELS fixtures span forks; Cancun-only assumptions produce systematic mismatches on historical vectors.

What:
- Thread fork/network context through execution paths and gate rule activation correctly.

How:
- Parse fixture network/fork metadata from blockchain tests.
- Gate precompile availability by fork (especially `0x0a`).
- Gate fork-specific validation in block/transaction processing paths.
- Add focused tests that prove behavioral differences across at least two fork boundaries.

### Task 5 (P1): Add Native vs RV32im Parity Regression Checks

Why:
- Native-only passing does not prove target correctness.

What:
- Add deterministic parity checks for key scenarios.

How:
- Add a `uv run` Python harness (PEP 723 metadata) that compares native and runner outputs on selected fixtures.
- Verify block result roots and transaction-level outcomes.

### Task 6 (P1): Converge Implementation to Protected README Contract

Why:
- `README.md` is protected by repository hooks, so contract drift must be solved in code rather than documentation edits.

What:
- Make implementation satisfy the existing protected claims, or replace them through an approved process outside this repo policy.

How:
- Eliminate remaining spec gaps (pairing, point evaluation, fork gating, full harness assertions).
- If zero-dependency is still mandatory, remove serde-driven serialization from core types.
