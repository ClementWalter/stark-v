# Claudeth Development Learnings

## EVM Execution Error Handling (Critical)

- **EVM execution failures (OOG, InvalidJump, Revert, etc.) are NOT
  transaction-level failures.** They should produce a failed receipt
  (status=false) with all gas consumed (for exceptional halts), state
  rolled back to pre-execution snapshot, and fees still paid.
- Previously, `execute_call`/`execute_create` returned `Err` on EVM
  errors, which propagated up as `TransactionExecutionError` and
  blocked block processing. Fixed by catching `Err` and returning
  `Ok((false, gas_available, ...))`.
- On failure, state must roll back to BEFORE value transfer, not just
  before EVM execution. The value transfer is part of the execution
  frame and must be reverted.

## Memory Expansion Gas (Critical)

- The EVM memory expansion gas formula is `words^2 / 512 + 3 * words`.
  This MUST NOT be capped. The quadratic growth naturally causes OOG
  for large memory requests.
- Previously had `MAX_MEMORY_WORDS` and `MAX_MEMORY_COST` caps that
  undercharged gas for large memory expansions, causing incorrect
  gas accounting and panics from trying to allocate huge vectors.
- `offset + size` calculations MUST use `saturating_add` to prevent
  usize overflow.

## U256 to usize Conversion (Critical)

- `U256::as_usize()` MUST saturate at `usize::MAX` when upper limbs
  are non-zero. Previously truncated to lowest 64-bit limb, causing
  huge U256 stack values to produce small usizes that bypassed gas
  checks.
- The old `u256_to_usize` helper in environment.rs had the same bug
  on 64-bit platforms. Replaced with `as_usize()`.

## EELS Test Notes

- All 216 test files (882 test cases) are now tested (removed `take(10)`
  limit). Result: 236/882 passing.
- Main failure category: GasUsedMismatch (638). Two patterns:
  1. `computed == gas_limit`: Transaction fails when it should succeed
     (OOG/error → all gas consumed via exceptional-halt handler)
  2. `computed < expected`: Transaction succeeds with less gas than
     expected (missing gas charges)
- EIP-4844 blob transactions (type 0x03) not supported (2 test failures).
- `GasLimitExceeded` (4 failures) and `StateRootMismatch` (2 failures)
  are secondary issues.

## Memory Safety

- `Vec::with_capacity(size)` panics on capacity overflow. After fixing
  gas calculations to OOG on huge sizes, this shouldn't happen, but
  added defense-in-depth cap in `read_memory_bytes` and
  `Memory::ensure_capacity`.

## Grounded Facts (from code)

- Guest input is an RLP list: block header, parent header, chain ID,
  transactions, state snapshot entries, and optional recent block
  hashes. Withdrawals are not decoded on the guest input path yet.
- Cancun header fields (`blob_gas_used`, `excess_blob_gas`) are parsed
  and hashed.
- EIP-4788 (beacon root system call), EIP-4895 (withdrawals), and
  EIP-2935 (historical block hashes system call) are implemented in
  block processing.
- Post-merge PREVRANDAO behavior is handled by setting
  `BlockContext.difficulty` to `mix_hash` when header `difficulty == 0`.
- Partial MPT computes roots and proofs; `EMPTY_TRIE_ROOT` is used for
  empty tries.

## Do / Don't (Always)

**Do**

- Use `EMPTY_TRIE_ROOT` for empty tries (never `Hash::ZERO`).
- Keep state root computation deterministic (sort addresses before trie
  insert).
- Use `saturating_add` for all EVM offset+size calculations.
- Handle EVM errors as failed transactions, not block-level failures.
- Run `prek run` and fix hook failures.
- Always run tests with `--release`.
- Always use `-p claudeth` to scope cargo commands.

**Don't**

- Cap memory expansion gas — let the quadratic formula do its job.
- Truncate U256 to usize without checking upper limbs.
- Treat EVM `Revert` as an exceptional halt (gas handling differs).
- Assume EELS results without rerunning tests.
- Rely on `HashMap` iteration order in consensus-critical paths.
- Update lint rules to hide errors.
