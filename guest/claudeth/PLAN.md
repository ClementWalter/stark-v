# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `README.md` still claims full EELS compatibility and no caveats.
- Non-ignored release suite is green:
  - `cargo test -p claudeth --release`
- Ignored full-suite probe (`test_execute_all_blockchain_tests`) is still non-gating and currently failing on deterministic fixture families.
- Deterministic failing families observed in the latest ignored sweep (before interruption):
  - `BlockchainTests/ValidBlocks/bcStateTests/logRevert.json::{logRevert_Cancun,logRevert_Prague}`
    - `GasUsedMismatch(expected=3627978, computed=3627993)` (`+15`)
  - `BlockchainTests/ValidBlocks/bcStateTests/refundReset.json::{refundReset_Cancun,refundReset_Prague}`
    - `GasUsedMismatch(expected=712850, computed=734258)` (`+21408`)
  - `BlockchainTests/ValidBlocks/bcStateTests/randomStatetest241.json::{randomStatetest241_Cancun,randomStatetest241_Prague}`
    - `GasUsedMismatch(expected=43138, computed=400000)` (`+356862`)
  - `BlockchainTests/ValidBlocks/bcStateTests/blockhashTests.json::{blockhashTests_Cancun,blockhashTests_Prague}`
    - `GasUsedMismatch(expected=45352, computed=65252)` (`+19900`)
  - `BlockchainTests/ValidBlocks/bcStateTests/suicideStorageCheckVCreate.json::{suicideStorageCheckVCreate_Cancun,suicideStorageCheckVCreate_Prague}`
    - `GasUsedMismatch(expected=468193, computed=184878)` (`-283315`)
- Full-suite harness status still conflicts with README claim:
  - `test_execute_all_blockchain_tests` remains `#[ignore]`
  - `run_all_blockchain_tests_impl` reports totals but does not assert `failed == 0 && errors == 0`

## Recently Completed (This Cycle)

- Fixed `logRevert` deterministic drift by making charged memory expansion persist for read-only ranges in the active interpreter path:
  - Added persistent range expansion helper in `src/evm/interpreter.rs`
  - Applied it to KECCAK (`0x20`) and all byte-range reads via `read_memory_bytes`
- Added focused regression coverage:
  - `test_log_revert_cancun_fixture`
  - `test_log_revert_prague_fixture`
- Verification completed in release mode:
  - `cargo test -p claudeth --release test_log_revert_ -- --nocapture`
  - `cargo test -p claudeth --release`
  - `prek run --all-files`

## Completion Objective

Make implementation truthfully match `README.md` by:

- eliminating deterministic EELS fixture failures;
- enforcing full-suite EELS compatibility as a default release gate;
- enforcing native/RV32 deterministic parity on representative fixtures.

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST): Re-Baseline the New First Unresolved Frontier

Why:
- `logRevert` was fixed in this cycle, so priority must move to the next deterministic failing family.

What:
- Re-run ignored full-suite probe and capture the first unresolved failing case after the `logRevert` fix.

How:
- Run `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`.
- Record the first `✗` fixture pair and mismatch numbers.
- If `refundReset` remains first, keep Task 2 as immediate implementation target.

### Task 2 (P0): Fix `refundReset` Gas Drift (`+21408`)

Why:
- Large deterministic gas drift indicates incorrect gas accounting in a high-signal state-transition path.

What:
- Match fixture gas-used for both Cancun and Prague `refundReset` cases.

How:
- Add/confirm focused Cancun + Prague fixture regressions.
- Diff per-tx gas with execution-spec behavior (`system.py`/`gas.py`) around refund and memory-extension side effects.
- Patch minimal logic in active interpreter/host/STF accounting path.

### Task 3 (P0): Fix `randomStatetest241` Full-Gas Burn Mismatch (`400000` vs `43138`)

Why:
- This delta suggests a control-flow/error-path bug where forwarded gas is burned incorrectly.

What:
- Restore spec-accurate gas used for the fixture on both Cancun and Prague.

How:
- Add focused regression first.
- Trace call/revert/invalid handling and gas propagation against execution-spec semantics.
- Patch minimal failing branch.

### Task 4 (P0): Fix `blockhashTests` Gas Drift (`+19900`)

Why:
- `BLOCKHASH` fixtures are deterministic and sensitive to memory/call-accounting interactions.

What:
- Eliminate gas mismatch for Cancun + Prague `blockhashTests`.

How:
- Add focused regressions.
- Diff opcode-level gas and memory-extension behavior against execution-spec.
- Patch minimal accounting bug.

### Task 5 (P0): Fix `suicideStorageCheckVCreate` Gas Drift (`-283315`)

Why:
- Large undercharge indicates missing gas components in CREATE/SELFDESTRUCT-related flow.

What:
- Match fixture gas semantics for both forks.

How:
- Add focused regression first.
- Diff create/collision/selfdestruct branch behavior against execution-spec.
- Patch missing charge points with minimal scope.

### Task 6 (P0): Continue Deterministic Frontier Burn-Down to Zero

Why:
- README compatibility remains false while any deterministic fixture family fails.

What:
- Iteratively eliminate all remaining deterministic failing families.

How:
- Repeat cycle: baseline -> focused regression -> execution-spec diff -> minimal fix -> rerun.

### Task 7 (P0): Make Full EELS Sweep a Default Hard Gate

Why:
- Ignored/non-fatal full-suite behavior allows silent regressions and contradicts README claims.

What:
- Fail default verification whenever full-suite reports any fixture failure or error.

How:
- Remove ignore posture for full-suite verification path.
- Assert `failed == 0 && errors == 0` in the runner.
- Keep release-mode execution path for this gate.

### Task 8 (P1): Close Remaining Spec-Semantics Gaps Not Yet Proven by Current Fixture Set

Why:
- Source still contains explicitly incomplete precompile behavior (`0x08` non-trivial pairing, `0x0a` point-evaluation), which is a latent compatibility risk.

What:
- Implement missing execution-spec semantics for those precompiles.

How:
- Port execution-spec behavior exactly from reference files.
- Add malformed/success/OOG regressions.
- Validate against relevant fixture families.

### Task 9 (P1): Add Native-vs-RV32 Deterministic Parity Gate

Why:
- README claims parity but no hard gate currently enforces it.

What:
- Add deterministic parity checks over curated high-signal fixtures.

How:
- Execute the same fixture set on native and runner paths in release mode.
- Fail on state root, receipts, gas-used, and logs divergence.
