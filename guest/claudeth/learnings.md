# Claudeth Development Learnings

## Current Facts (from code)

- The guest expects an RLP list input that includes block header, parent header,
  chain ID, transactions, state snapshot entries, and optional recent block
  hashes; withdrawals are not decoded yet in the guest input path.
- Cancun header fields (`blob_gas_used`, `excess_blob_gas`) are parsed and hashed;
  `BLOBHASH`/`BLOBBASEFEE` opcodes are implemented via the Host interface.
- EIP-4788 (beacon root system call) and EIP-4895 (withdrawals) are implemented
  in block processing.
- EIP-2935 (historical block hashes) is implemented in block processing, activated
  when `requests_hash` is present (Prague fork indicator).
- PREVRANDAO (opcode 0x44) correctly returns `mix_hash` for post-merge blocks
  and `difficulty` for pre-merge blocks.

## EELS Test Status

- 14/20 EELS blockchain tests passing (first 10 fixture files, ~20 test cases).
- Remaining 6 failures are all `TransactionExecutionError(ExecutionFailed)`:
  - `transStorageBlockchain` (Block 2): multi-block with nested CREATEs
  - `ShanghaiLove` (Block 0): empty-data tx to contract
  - `StrangeContractCreation` (Block 0): large constructor bytecode
- The `ExecutionFailed` error in executor.rs is too generic — it wraps all EVM
  errors into a single variant, losing the actual cause.

## Testing Notes

- `cargo test -p claudeth --release` (2026-02-09):
  - 1172 unit tests passed
  - 93 doc tests passed
  - Integration tests passed; EELS test runner remains ignored by default
- EELS blockchain tests require external fixtures in
  `tests/eels/BlockchainTests/` and must be run with `-- --ignored`.
- Use `--features evm-trace` to get per-opcode gas traces in EELS test output.

## Do / Don't (Always)

**Do**

- Use `EMPTY_TRIE_ROOT` for empty tries (never `Hash::ZERO`).
- Keep state root computation deterministic (sort addresses before trie insert).
- Charge CREATE code deposit gas and handle OOG correctly.
- Post-merge: set `BlockContext.difficulty` to `mix_hash` (not header `difficulty`),
  since opcode 0x44 is PREVRANDAO, not DIFFICULTY.
- Run `prek run` and fix hook failures instead of bypassing them.
- Always run tests with `--release` flag.
- Always use `-p claudeth` to scope cargo commands.

**Don't**

- Assume EELS results without running ignored fixtures.
- Rely on `HashMap` iteration order in consensus-critical paths.
- Update lint rules to hide errors.
- Set `BlockContext.difficulty` directly from the header `difficulty` field —
  use `mix_hash` for post-merge (PoS) blocks where difficulty == 0.

## Key Bug Fixes

### PREVRANDAO returns wrong value (2026-02-09)
- **Symptom**: mergeExample EELS test fails with GasUsedMismatch (expected 82839,
  got 62939, diff = 19900).
- **Root cause**: `BlockContext.difficulty` was set to `block.difficulty` (0 for
  PoS blocks), but opcode 0x44 (PREVRANDAO) should return `mix_hash` post-merge.
  This caused SSTORE to write 0 instead of the actual prev_randao value,
  changing the gas cost from 22100 (cold SET) to 2200 (cold NOOP).
- **Fix**: In `process_block()`, detect post-merge (difficulty == 0) and set
  `BlockContext.difficulty` to `U256::from_be_bytes(mix_hash)`.

### EMPTY_TRIE_ROOT byte 17 typo (earlier)
- Byte 17 was `0x96` instead of `0x48`. Always verify hash constants byte-by-byte.

## Next Iteration Do / Don't

**Do**

- Add more context to `ExecutionError::ExecutionFailed` to preserve the actual
  EVM error for debugging.
- Investigate CALL/CREATE execution paths for the remaining 6 EELS failures.
- Check that CREATE properly handles code deposit, nonce increment, and value
  transfer atomically.

**Don't**

- Add system calls without a precise storage layout and address mapping.
- Wrap EVM errors into a single generic `ExecutionFailed` — preserve the cause.
