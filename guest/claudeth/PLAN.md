# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `README.md` claims full EELS compatibility and native/RV32 parity.
- `test_execute_all_blockchain_tests` is still `#[ignore]`, so that claim is not hard-gated.
- `cargo test -p claudeth --release` currently passes for the non-ignored suite.
- `src/evm/precompiles.rs` still intentionally leaves major conformance gaps:
  - `0x0a` point-evaluation precompile returns failure;
  - `0x08` pairing supports only identity/infinity shortcuts;
  - Prague BLS12 precompiles `0x0b..0x11` are not executed.
- Targeted analysis of `suicideStorageCheck` + execution-spec references shows a concrete mismatch:
  top-level create transactions must fail on destination collision by burning all remaining execution gas (`AddressCollision` path), while current `src/stf/executor.rs::execute_create` does not implement that short-circuit.

## Completion Objective

Make implementation truthfully match `README.md`:

- pass the full EELS blockchain fixture sweep (no ignored compatibility gate);
- preserve deterministic native/RV32 behavior for supported fixtures;
- keep that coverage enforced by default.

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST): Fix Top-Level CREATE Collision Semantics (`suicideStorageCheck`)

Why:
- Current deterministic frontier is `suicideStorageCheck` Cancun/Prague gas mismatch.
- Collision handling for top-level create is currently inconsistent with execution-spec `process_message_call` `AddressCollision` behavior.

What:
- In `execute_create`, detect destination collision (`code/nonce` or non-empty storage) before init-code execution.
- On collision:
  - do not mutate destination account/storage/code;
  - report failed creation;
  - consume full execution gas (`gas_used = gas_available`).

How:
- Add focused Cancun/Prague fixture regressions for:
  - `BlockchainTests/ValidBlocks/bcStateTests/suicideStorageCheck.json::..._Cancun`
  - `BlockchainTests/ValidBlocks/bcStateTests/suicideStorageCheck.json::..._Prague`
- Implement top-level collision short-circuit in `src/stf/executor.rs`.
- Validate with release-mode tests and ensure post-state parity.

### Task 2 (P0): Re-Baseline Full Ignored Frontier

Why:
- After each deterministic fix, the first failing family can move immediately.

What:
- Re-run the ignored full suite and capture the new first deterministic failing family.

How:
- Run:
  - `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`
- Stop prioritization at the first stable mismatch family.

### Task 3 (P0): Burn Down Deterministic Failure Families to Zero

Why:
- README compatibility is false until deterministic fixture failures are eliminated.

What:
- Resolve first-failure family iteratively until full-suite pass.

How:
- For each frontier:
  - reproduce with focused fixture test;
  - map behavior to execution-spec;
  - implement minimal fix;
  - re-baseline frontier.

### Task 4 (P0): Make Full Blockchain Sweep a Hard Gate

Why:
- Ignored compatibility test allows regressions behind a non-default path.

What:
- Remove non-gating behavior for the full EELS blockchain sweep once failures are cleared.

How:
- Tighten assertions in `run_all_blockchain_tests_impl` (`failed == 0 && errors == 0`).
- Remove/adjust `#[ignore]` so this runs in normal CI/default verification.

### Task 5 (P1): Implement Point-Evaluation Precompile (`0x0a`)

Why:
- Required for Cancun/Prague conformance completeness.

What:
- Add full validation/gas/output behavior for precompile `0x0a`.

How:
- Port execution-spec behavior.
- Add malformed/success/OOG vectors plus fixture coverage.

### Task 6 (P1): Implement Full BN254 Pairing (`0x08`)

Why:
- Current implementation intentionally rejects non-trivial tuples.

What:
- Implement full pairing product verification for valid tuple sets.

How:
- Implement tuple parsing + full arithmetic path + gas-correct behavior.
- Add execution-spec vector coverage.

### Task 7 (P1): Implement Prague BLS12 Precompiles (`0x0b..0x11`)

Why:
- Prague warm-set handling exists, but execution support is missing.

What:
- Implement execution and gas semantics for BLS12 precompile range.

How:
- Port execution-spec rules for each address.
- Add malformed/success/OOG tests and representative fixtures.

### Task 8 (P1): Enforce Native vs RV32 Deterministic Parity Gate

Why:
- README promises parity, but parity checks are not currently hard-gated.

What:
- Add deterministic parity assertions on a curated high-signal fixture subset.

How:
- Run identical vectors through native and runner paths in release mode.
- Gate after stabilization.
