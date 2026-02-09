# Claudeth Development Learnings

## Current Facts (from code)

- The guest expects an RLP list input that includes block header, parent header,
  chain ID, transactions, state snapshot entries, and optional recent block
  hashes; withdrawals are not decoded yet in the guest input path.
- Cancun header fields (`blob_gas_used`, `excess_blob_gas`) are parsed and hashed;
  `BLOBHASH`/`BLOBBASEFEE` opcodes are implemented via the Host interface.
- EIP-4788 (beacon root system call) and EIP-4895 (withdrawals) are implemented
  in block processing.
- Prague EIP-2935 Historical Block Hashes system call is not implemented; header
  `requests_hash` is parsed but unused.

## Testing Notes

- `cargo test -p claudeth --release` (2026-02-09):
  - 1172 unit tests passed
  - 93 doc tests passed
  - Integration tests passed; EELS test runner remains ignored by default
- `prek run` failed in this environment: unable to open
  `/Users/clementwalter/.cache/prek/prek.log` (permission denied).
- EELS blockchain tests require external fixtures in
  `tests/eels/BlockchainTests/` and must be run with `-- --ignored`.

## Do / Don't (Always)

**Do**

- Use `EMPTY_TRIE_ROOT` for empty tries (never `Hash::ZERO`).
- Keep state root computation deterministic (sort addresses before trie insert).
- Charge CREATE code deposit gas and handle OOG correctly.
- Record test provenance explicitly when updating docs.
- Run `prek run` and fix hook failures instead of bypassing them.

**Don't**

- Assume EELS results without running ignored fixtures.
- Rely on `HashMap` iteration order in consensus-critical paths.
- Update lint rules to hide errors.

## Next Iteration Do / Don't

**Do**

- Verify Prague/EIP-2935 behavior against a spec source before implementing.
- Use `evm-trace` only for targeted debugging to keep normal runs clean.

**Don't**

- Add system calls without a precise storage layout and address mapping.
