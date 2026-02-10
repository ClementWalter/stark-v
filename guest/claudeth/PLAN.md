# Claudeth Completion Plan

This plan is based on direct inspection of `README.md`, `src/`, `tests/`, `execution-specs/`, and `learnings.md` on 2026-02-10.

## Reality Check vs README

- `README.md` claims full execution-spec compatibility, but current automated coverage does not prove it yet:
  - `tests/eels_blockchain_tests.rs` skips `InvalidBlocks`, truncates with `.take(10)`, and keeps full execution behind `#[ignore]`.
  - The EELS execution test currently keeps a parent-hash overwrite workaround.
- Precompile surface is incomplete in `src/evm/precompiles.rs`:
  - implemented: `0x01..0x07`, `0x09`;
  - not implemented: `0x08` (`ALT_BN128_PAIRING`), `0x0a` (`POINT_EVALUATION`).
- Because `0x08` and `0x0a` are currently not dispatched as precompiles, calls to those addresses can incorrectly fall through to empty-code call behavior.

## Completed Baseline

- Native `cargo test -p claudeth --release` passes.
- Existing precompile behavior (`0x01..0x07`, `0x09`) uses EVM-style sub-call failure semantics for malformed inputs and out-of-gas.
- `ECADD`/`ECMUL` follow EIP-196 style decoding and malformed-point rejection.
- `BLAKE2F` follows EIP-152 input layout and test vectors.

## Remaining Tasks (Ordered by Priority)

### Task 1 (P0): Reserve Known Precompile Addresses 0x08/0x0a (Do Now)

Why:
- `execution-specs` maps both `0x08` and `0x0a` as precompiled contracts, so treating them as normal empty-code accounts is semantically wrong.
- Silent fallthrough can incorrectly return success and incorrectly move call value.

What:
- Treat `0x08` and `0x0a` as recognized precompile addresses immediately.
- Until full implementations land, force them to fail as precompile sub-calls rather than falling through to regular CALL empty-code behavior.
- Add regression tests to lock this behavior.

How:
- Extend `execute_precompile` dispatch to include IDs `8` and `10` with explicit failure stubs.
- Keep failure mapped to `PrecompileError::OutOfGas` so host-level call semantics remain “failed sub-call consuming forwarded gas”.
- Add unit tests in `src/evm/precompiles.rs` and `src/evm/host.rs` proving:
  - dispatcher returns `Some(Err(...))` for `0x08`/`0x0a`;
  - CALL to `0x08` fails and does not transfer value.

### Task 2 (P0): Implement ALT_BN128 Pairing Precompile (`0x08`)

Why:
- EIP-197 compatibility requires actual pairing verification, not a failure stub.

What:
- Implement pairing check over BN254 with proper output encoding and gas rules.

How:
- Follow `execution-specs/src/ethereum/forks/cancun/vm/precompiled_contracts/alt_bn128.py`:
  - 192-byte tuple decoding (`G1` + `G2`), strict field/curve validation;
  - subgroup checks (`[curve_order]P == infinity`);
  - gas `45000 + 34000 * n`;
  - output `U256(1)`/`U256(0)` as 32 bytes.

### Task 3 (P0): Implement POINT_EVALUATION Precompile (`0x0a`)

Why:
- Cancun/Prague correctness requires EIP-4844 point-evaluation behavior.

What:
- Implement strict 192-byte input verification with versioned-hash and KZG proof verification.

How:
- Follow `execution-specs/src/ethereum/forks/cancun/vm/precompiled_contracts/point_evaluation.py`:
  - fixed gas `GAS_POINT_EVALUATION`;
  - exact input parsing (`versioned_hash`, `z`, `y`, `commitment`, `proof`);
  - versioned-hash check and proof verification;
  - output two 32-byte constants (`FIELD_ELEMENTS_PER_BLOB`, `BLS_MODULUS`).

### Task 4 (P0): Make EELS Blockchain Harness Representative

Why:
- Current harness cannot justify README-level compatibility claims.

What:
- Execute representative fixtures without truncation/shortcuts and enforce expected outcomes.

How:
- Remove `InvalidBlocks` skip and `.take(10)` limits.
- Remove parent-hash overwrite workaround.
- Implement robust handling for `expectException` on invalid fixtures.
- Replace informational-only behavior with assertions suitable for CI.

### Task 5 (P0): Add Fork-Aware Rule Gating

Why:
- Current execution logic is mostly Cancun-first; historical fixtures require fork-specific behavior.

What:
- Thread fork/network context through block/transaction/precompile/opcode rules.

How:
- Parse fixture fork/network metadata and gate rule branches accordingly.
- Replace unconditional modern assumptions with fork-conditional checks.

### Task 6 (P1): Add RV32 Parity Automation via Runner

Why:
- Native passing tests do not guarantee `riscv32im-unknown-none-elf` parity.

What:
- Add automated parity checks between native and RV32 executions.

How:
- Add a `uv run` Python harness that runs representative scenarios through both paths.
- Compare outputs/state roots/receipts roots and fail on divergence.

### Task 7 (P1): Align README Claims With Continuously Verifiable Checks

Why:
- Project claims should be backed by enforceable automation.

What:
- Update documentation and/or checks so every headline claim is verifiable.

How:
- Keep claims only for validated behavior.
- Document exact commands and expected artifacts that prove each claim.
