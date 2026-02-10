# Claudeth Implementation Plan

Priority-ordered tasks to reach full EELS compatibility and the project goals.

Status: Precompiles 0x01-0x04 are implemented (ECRECOVER, SHA256, RIPEMD160, IDENTITY).

- Task 1: Implement remaining legacy precompiles (MODEXP, ALT_BN128 add/mul/pairing, BLAKE2F).
Why: State and blockchain tests include these precompiles; missing them blocks fixture coverage.
What: Implement 0x05-0x09 semantics and gas formulas.
How: Follow execution-specs precompile implementations and gas functions; add targeted unit tests and fixture-driven checks.

- Task 2: Implement Cancun/Prague precompiles (point evaluation, P256VERIFY) with fork gating.
Why: Cancun/Prague fixtures include EIP-4844 and EIP-7212 behavior.
What: Add precompile handlers for the newer addresses and enforce activation by fork.
How: Introduce a fork config in block/tx context, map precompile set by fork, and add tests that assert activation boundaries.

- Task 3: Fork-aware STF rules (gas schedule, opcodes, tx validity).
Why: EELS fixtures span multiple forks; current execution applies post-Cancun rules everywhere.
What: Add a fork selection mechanism based on block metadata/config, and gate rules accordingly.
How: Define a fork enum + schedule parser for fixture network fields; thread into validation and EVM gas tables.

- Task 4: Full EELS blockchain fixture execution.
Why: README goal is EELS compatibility; current harness only parses a subset and runs a handful of files.
What: Execute all valid/invalid blockchain tests with correct error mapping and reporting.
How: Implement exception mapping, expand harness to run all fixture files, and add a fast subset smoke test.

- Task 5: RV32im build + runner integration for tests.
Why: Project promises RV32im compatibility; tests currently run natively only.
What: Add a Python (uv-run) test driver that compiles to RV32im and runs via the runner.
How: Build a `uv run` script that compiles `-p claudeth` to RV32im, runs the ELF with `../../crates/runner`, and mirrors the native test list.

- Task 6: no_std enforcement and size/cycle benchmarking parity.
Why: `no_std` is a core claim; RV32im constraints need continuous verification.
What: Add a `no_std` build target and benchmark checks in CI-like scripts.
How: Provide a `uv run` script that builds with `#![no_std]` gated features and runs `benchmarks/` with fixed inputs.
