# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `README.md` claims:
  - full EELS compatibility;
  - native and RV32 deterministic parity;
  - no optional caveats.
- Current release ignored-suite probe:
  - command: `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`
  - first deterministic failures observed:
    - `BlockchainTests/ValidBlocks/bcStateTests/testOpcode_f0.json::testOpcode_fa_Prague`
    - `BlockchainTests/ValidBlocks/bcStateTests/testOpcode_f0.json::testOpcode_fa_Cancun`
    - mismatch: `GasUsedMismatch(expected=9889566, computed=10025861)` (`+136295`)
  - next deterministic family observed in same run:
    - `BlockchainTests/ValidBlocks/bcStateTests/logRevert.json::{logRevert_Cancun,logRevert_Prague}`
    - mismatch: `GasUsedMismatch(expected=3627978, computed=3627993)` (`+15`)
- Active interpreter path is `src/evm/interpreter.rs` (not `src/evm/opcodes/exec.rs`), so fixes must land there.
- `testOpcode_fa` fixture bytecode drives nested `CALL -> STATICCALL` and expects static-context write rejection behavior.
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

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST): Fix `testOpcode_fa` (`+136295`) via Static-Context Write Protection

Why:
- It is the first deterministic full-suite failure in release mode.
- The failing vector specifically stress-tests `STATICCALL` semantics with a child that attempts `SSTORE`.
- Current interpreter path does not model static-context write protection in the active opcode executor, so gas behavior diverges at call-frame level.

What:
- Make `testOpcode_f0.json::testOpcode_fa_{Cancun,Prague}` match fixture gas usage exactly.

How:
- Add focused fixture regressions for:
  - `BlockchainTests/ValidBlocks/bcStateTests/testOpcode_f0.json::testOpcode_fa_Cancun`
  - `BlockchainTests/ValidBlocks/bcStateTests/testOpcode_f0.json::testOpcode_fa_Prague`
- Port execution-spec static-call behavior into active interpreter path:
  - propagate static-context flag across recursive frames;
  - reject `SSTORE` in static context as exceptional halt path.
- Keep patch minimal to this family: no unrelated gas model rewrites.
- Validate in release mode:
  - focused `testOpcode_fa` regressions;
  - current non-ignored release suite.

### Task 2 (P0): Re-Baseline Immediately After Task 1

Why:
- After Task 1, the deterministic frontier is expected to move; planning must stay anchored to current first failure.

What:
- Confirm the new first deterministic failing family.

How:
- Re-run the ignored full-suite release probe.
- Capture only the first stable mismatch family and promote it.
- Current expected candidate after Task 1: `logRevert` (`+15`) unless displaced.

### Task 3 (P0): Burn Down Remaining Deterministic EELS Families

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

### Task 4 (P0): Make Full EELS Sweep a Default Hard Gate

Why:
- Ignored/non-fatal full-suite behavior allows silent regressions.

What:
- Fail default verification when any EELS fixture fails/errors.

How:
- Enforce `failed == 0 && errors == 0` in the full-suite runner.
- Remove ignore-only posture from default quality gates (local/CI).

### Task 5 (P1): Implement Point-Evaluation Precompile (`0x0a`)

Why:
- Cancun/Prague completeness is incomplete while `0x0a` remains reserved.

What:
- Implement full point-evaluation validation, gas, and output semantics.

How:
- Port execution-spec behavior exactly.
- Add malformed/success/OOG vectors and fixture coverage.

### Task 6 (P1): Complete BN254 Pairing Precompile (`0x08`)

Why:
- Current implementation is partial and misses full tuple verification coverage.

What:
- Implement full pairing product verification semantics.

How:
- Implement full tuple parsing/validation and final pairing check.
- Validate with execution-spec vectors (success/failure/OOG).

### Task 7 (P1): Implement Prague BLS12 Precompiles (`0x0b..0x11`)

Why:
- Prague warm-set support exists but execution paths are still missing.

What:
- Implement execution/gas semantics for all Prague BLS12 precompile addresses.

How:
- Port each address behavior from execution-spec references.
- Add malformed/success/OOG regressions per precompile.

### Task 8 (P1): Add Native-vs-RV32 Deterministic Parity Gate

Why:
- README claims parity, but no deterministic parity gate currently enforces it.

What:
- Add required parity checks over curated high-signal fixtures.

How:
- Execute identical fixtures on native and runner paths in release mode.
- Fail on state/gas/log divergence.
