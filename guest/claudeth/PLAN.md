# Claudeth Completion Plan

Last reviewed: 2026-02-10

## Ground Truth Snapshot

- `cargo test -p claudeth --release` passes locally.
- `tests/eels_blockchain_tests.rs::test_execute_all_blockchain_tests` is still `#[ignore]` and still contains non-conformance workarounds:
  - parent hash is overwritten before execution;
  - final `failed/errors` assertions are commented out.
- A full ignored-run baseline (before this turn's fix) reported:
  - `Total: 1142`, `Passed: 82`, `Failed: 1060`, `Errors: 0`.
  - dominant failure reason then: `InvalidHeader("mix hash must be zero in post-merge blocks")`.
- After the first fix in this turn, sampled ignored-run failures shifted to deeper execution mismatches, dominated by `WithdrawalsRootMismatch`, confirming the previous header gate was incorrect and is now removed.
- Precompile status in `src/evm/precompiles.rs`:
  - implemented: `0x01..0x07`, `0x09`;
  - partially implemented: `0x08` (strict decoding + curve checks + infinity-identity success; non-trivial pairing arithmetic still missing);
  - not implemented: `0x0a` (`POINT_EVALUATION`) remains a deterministic failure path.
- README contract is still not satisfied by enforceable checks:
  - execution-spec compatibility is not enforced by passing blockchain suite assertions;
  - native/RV32 parity is not enforced by an automated gate in this crate;
  - README says "no dependencies" but crate depends on `serde`.

## Completed This Turn

- Fixed post-merge header validation in `src/types/block.rs`:
  - removed the incorrect `mix_hash == 0` requirement;
  - retained required checks for `difficulty == 0`, `nonce == 0`, and empty ommers hash.
- Updated block header docs/tests to match execution-spec semantics where the legacy mix-hash slot carries `prev_randao` post-merge.
- Revalidated targeted tests for the updated behavior in release mode.

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST, DONE): Align Post-Merge Header Validation with PrevRandao Semantics

Why:
- Execution-spec Cancun/Prague header validation does not require post-merge `mix_hash` to be zero; enforcing it rejects valid fixtures at the header gate.

What:
- Remove non-zero mix-hash rejection from post-merge validation while keeping other consensus-constant checks.

How:
- Follow `execution-specs/src/ethereum/forks/cancun/fork.py::validate_header` behavior:
  - keep checks for zero `difficulty`, zero `nonce`, and empty ommers hash;
  - do not enforce `prev_randao/mix_hash` equality to zero.

### Task 2 (P0): Fix Withdrawals Root Computation/Validation Mismatch

Why:
- After Task 1, the dominant failing reason in ignored blockchain runs is `WithdrawalsRootMismatch`, which now blocks broad fixture execution.

What:
- Make computed withdrawals root exactly match execution-spec trie rules and fixture expectations.

How:
- Diff `calculate_withdrawals_root` path against execution-spec (`fork.py` + trie encoding behavior).
- Verify withdrawal RLP payload and trie key encoding (`rlp(index)` as trie key) end-to-end.
- Add focused regression tests for:
  - empty withdrawals list root;
  - single withdrawal root;
  - multi-withdrawal order sensitivity.

### Task 3 (P0): Remove Blockchain Harness Workarounds and Enforce Assertions

Why:
- Compatibility claims are not meaningful while the integration harness is ignored and mutates header linkage.

What:
- Convert EELS blockchain execution into an enforceable correctness gate.

How:
- Remove parent-hash overwrite workaround.
- Keep diagnostics, but hard-fail on `failed != 0` or `errors != 0` once functional blockers are resolved.
- Keep parser coverage broad for valid + invalid suites.

### Task 4 (P0): Complete ALT_BN128 Pairing for Non-Trivial Inputs (`0x08`)

Why:
- EIP-197 requires full pairing product evaluation; current behavior fails all non-trivial tuples.

What:
- Implement non-trivial pairing arithmetic and canonical `1`/`0` output behavior.

How:
- Mirror execution-spec subgroup and pairing-product semantics.
- Add field-extension arithmetic needed for Miller loop/final exponentiation.
- Add vectors for success/failure and malformed/subgroup-invalid inputs.

### Task 5 (P0): Implement POINT_EVALUATION Precompile (`0x0a`)

Why:
- Cancun/4844 compatibility requires this precompile; reserved-fail cannot satisfy conformance.

What:
- Implement strict calldata checks, fixed gas, KZG proof verification, and canonical output words.

How:
- Follow `execution-specs/src/ethereum/forks/cancun/vm/precompiled_contracts/point_evaluation.py`.
- Match versioned hash derivation and proof verification semantics.
- Add success/failure vectors from execution-spec fixtures.

### Task 6 (P0): Add Fork-Aware Rule Gating Across Execution Paths

Why:
- Fixtures span multiple protocol eras; always-on modern rules cause deterministic mismatches.

What:
- Thread explicit fork metadata/rules into validation and EVM execution.

How:
- Propagate fixture/network fork context into block and transaction execution.
- Gate precompile availability and fork-specific transaction/block checks.
- Add regression tests spanning at least two fork boundaries.

### Task 7 (P1): Add Native vs RV32im Deterministic Parity Gate

Why:
- README claims cross-target coverage; this is not currently enforced.

What:
- Add a reproducible native-vs-runner parity check over curated fixtures.

How:
- Add a `uv run` Python script (PEP 723 metadata) to execute both targets and diff outputs.
- Integrate script into the local quality workflow.

### Task 8 (P1): Align README Claims with Verifiable Guarantees

Why:
- Project claims must be true and enforceable via code/tests.

What:
- Reconcile README with verified behavior after functional parity tasks are complete.

How:
- Either implement missing guarantees or narrow statements to what CI/test gates actually prove.
- Resolve dependency claim mismatch (`serde` vs "no dependencies") explicitly.
