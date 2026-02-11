# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `README.md` claims:
  - full EELS compatibility;
  - native and RV32 deterministic parity;
  - no optional caveats.
- Last full release ignored-suite probe before this cycle:
  - command: `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`
  - first deterministic failures observed at that time:
    - `BlockchainTests/ValidBlocks/bcStateTests/testOpcode_f0.json::testOpcode_fa_Prague`
    - `BlockchainTests/ValidBlocks/bcStateTests/testOpcode_f0.json::testOpcode_fa_Cancun`
    - mismatch: `GasUsedMismatch(expected=9889566, computed=10025861)` (`+136295`)
  - next deterministic family observed in the same run:
    - `BlockchainTests/ValidBlocks/bcStateTests/logRevert.json::{logRevert_Cancun,logRevert_Prague}`
    - mismatch: `GasUsedMismatch(expected=3627978, computed=3627993)` (`+15`)
- Active interpreter path is `src/evm/interpreter.rs` (not `src/evm/opcodes/exec.rs`), so fixes must land there.
- `testOpcode_fa` fixture bytecode drives nested `CALL -> STATICCALL` and requires static-context write rejection behavior.
- Task 1 implementation result in this cycle:
  - static context now propagates through recursive host frames;
  - active interpreter now rejects `SSTORE` in static context;
  - focused regressions now pass:
    - `test_opcode_f0_fa_cancun_fixture`
    - `test_opcode_f0_fa_prague_fixture`
  - non-ignored release suite passes:
    - `cargo test -p claudeth --release`
- Full-suite conformance remains non-gating:
  - `test_execute_all_blockchain_tests` remains `#[ignore]`.
  - `run_all_blockchain_tests_impl` still reports counts without asserting `failed == 0 && errors == 0`.
- Precompile surface still incomplete relative to README:
  - `0x0a` point-evaluation: reserved/unimplemented path.
  - `0x08` pairing: incomplete for non-trivial tuples.
  - Prague BLS12 addresses `0x0b..0x11`: not implemented.

## Completion Objective

Make implementation truthfully match `README.md` by:

- removing deterministic EELS fixture failures;
- enforcing full-suite compatibility as a required release gate;
- enforcing native/RV32 deterministic parity on representative fixtures.

## Recently Completed

- Implemented static-context propagation in the active interpreter/host path.
- Enforced static-context `SSTORE` rejection as exceptional halt behavior.
- Added focused Cancun/Prague regressions for `testOpcode_fa`.
- Verified with release-mode focused and non-ignored suite runs.

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST): Re-Baseline and Fix the New First Deterministic Frontier (`logRevert`, `+15`)

Why:
- After completing `testOpcode_fa`, the next known deterministic candidate is `logRevert` (`+15`).
- Small deterministic deltas in opcode families are high-signal accounting drift and often hide additional state/receipt divergence behind gas checks.

What:
- Confirm the current first deterministic failing family post-fix and eliminate it.

How:
- Re-run ignored full-suite release probe and capture the first stable mismatch.
- If `logRevert` remains first:
  - add focused Cancun/Prague regressions for `logRevert`;
  - compare `LOG`/`REVERT` path gas accounting against execution-spec;
  - patch minimal opcode-level accounting drift.
- Validate with focused regressions + non-ignored release suite.

### Task 2 (P0): Continue Deterministic Frontier Burn-Down

Why:
- README compatibility claim remains false while any deterministic fixture fails.

What:
- Iteratively eliminate deterministic fixture families until zero.

How:
- For each frontier:
  - add focused regression first;
  - diff behavior against execution-spec implementation;
  - apply minimal spec-accurate fix;
  - rerun full ignored probe to move frontier.

### Task 3 (P0): Make Full EELS Sweep a Default Hard Gate

Why:
- Ignored/non-fatal full-suite behavior allows silent regressions.

What:
- Fail default verification when any EELS fixture fails/errors.

How:
- Enforce `failed == 0 && errors == 0` in the full-suite runner.
- Remove ignore-only posture from default quality gates (local/CI).

### Task 4 (P1): Implement Point-Evaluation Precompile (`0x0a`)

Why:
- Cancun/Prague completeness is incomplete while `0x0a` remains reserved.

What:
- Implement full point-evaluation validation, gas, and output semantics.

How:
- Port execution-spec behavior exactly.
- Add malformed/success/OOG vectors and fixture coverage.

### Task 5 (P1): Complete BN254 Pairing Precompile (`0x08`)

Why:
- Current implementation is partial and misses full tuple verification coverage.

What:
- Implement full pairing product verification semantics.

How:
- Implement full tuple parsing/validation and final pairing check.
- Validate with execution-spec vectors (success/failure/OOG).

### Task 6 (P1): Implement Prague BLS12 Precompiles (`0x0b..0x11`)

Why:
- Prague warm-set support exists but execution paths are still missing.

What:
- Implement execution/gas semantics for all Prague BLS12 precompile addresses.

How:
- Port each address behavior from execution-spec references.
- Add malformed/success/OOG regressions per precompile.

### Task 7 (P1): Add Native-vs-RV32 Deterministic Parity Gate

Why:
- README claims parity, but no deterministic parity gate currently enforces it.

What:
- Add required parity checks over curated high-signal fixtures.

How:
- Execute identical fixtures on native and runner paths in release mode.
- Fail on state/gas/log divergence.
