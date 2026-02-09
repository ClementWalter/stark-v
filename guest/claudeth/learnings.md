# Claudeth Development Learnings

## Grounded Facts (from code)

- Guest input is an RLP list: block header, parent header, chain ID, transactions,
  state snapshot entries, and optional recent block hashes. Withdrawals are not
  decoded on the guest input path yet.
- Cancun header fields (`blob_gas_used`, `excess_blob_gas`) are parsed and hashed.
- EIP-4788 (beacon root system call), EIP-4895 (withdrawals), and EIP-2935
  (historical block hashes system call) are implemented in block processing.
- Post-merge PREVRANDAO behavior is handled by setting `BlockContext.difficulty`
  to `mix_hash` when header `difficulty == 0`.
- Partial MPT computes roots and proofs; `EMPTY_TRIE_ROOT` is used for empty tries.

## EELS Test Notes

- `cargo test -p claudeth --release` passes; EELS ignored fixtures are still not
  executed by default.
- `test_execute_all_blockchain_tests` remains ignored, so current EELS failure
  details are unknown until rerun with `-- --ignored`.

## Execution-Specs Reference Notes

- The Prague interpreter treats `Revert` separately from `ExceptionalHalt`:
  exceptional halts consume all remaining gas, while `Revert` does not and
  still rolls back state to the checkpoint.
## Debugging Notes

- `ExecutionError::ExecutionFailed` now carries the underlying `EvmError`, so
  EELS failures should report the specific EVM error instead of a generic
  failure.

## Do / Don't (Always)

**Do**

- Use `EMPTY_TRIE_ROOT` for empty tries (never `Hash::ZERO`).
- Keep state root computation deterministic (sort addresses before trie insert).
- Charge CREATE code deposit gas and handle OOG correctly.
- Post-merge: set `BlockContext.difficulty` to `mix_hash` (PREVRANDAO), not the
  header `difficulty` field.
- Run `prek run` and fix hook failures.
- Always run tests with `--release`.
- Always use `-p claudeth` to scope cargo commands.
- If `prek run` fails in a sandbox, ensure the git directory is writable or run
  in a context that allows `.git/index.lock` creation.

**Don't**

- Assume EELS results without rerunning ignored fixtures.
- Rely on `HashMap` iteration order in consensus-critical paths.
- Update lint rules to hide errors.
- Treat `Revert` as an exceptional halt (gas handling differs in the spec).
