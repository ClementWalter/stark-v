# Claudeth

Claudeth is a minimal Ethereum State Transition Function (STF) guest program
written in Rust for the stark-v zkVM. It targets `no_std` and
`riscv32im-unknown-none-elf`.

## Current Status (2026-02-09)

**Implemented**

- EVM interpreter with full opcode coverage, including Cancun opcodes
  (`BLOBHASH`, `BLOBBASEFEE`, `PREVRANDAO`) and transient storage
  (`TLOAD`, `TSTORE`)
- Transaction validation and execution for Legacy, EIP-2930, and EIP-1559
- Block processing with header validation and root checks
  (state, receipts, transactions, logs bloom)
- Partial Merkle Patricia Trie for account/storage roots and proofs
- Block header type supports Shanghai/Cancun fields
  (`withdrawals_root`, `blob_gas_used`, `excess_blob_gas`,
  `parent_beacon_block_root`, `requests_hash`)

**Known Gaps / Limitations**

- EIP-4788 Beacon Block Root system call not implemented in block processing
- EIP-4895 withdrawals are not applied in block processing
- EIP-2935 Historical Block Hashes system call not implemented
- Guest input decoding does not include withdrawals or recent block hashes
- Witness-based state reconstruction is not implemented
- `k256` is still used for secp256k1
- EELS blockchain tests require external fixtures and are ignored by default

## Testing

- Unit and doc tests: `cargo test -p claudeth --release`
- EELS fixtures: `scripts/fetch_eels_tests.py` then
  `cargo test -p claudeth --release -- --ignored`

## Architecture

Claudeth implements the Ethereum execution layer STF and validates:

- Block headers against parent headers
- Transaction roots via MPT
- Receipt roots via MPT
- State roots via MPT
- Logs bloom filters
- Gas usage and limits

The codebase embeds a **Partial MPT** implementation capable of:

- Building tries from account/storage data
- Computing roots
- Generating and verifying Merkle proofs
- (Future) Reconstructing minimal state from witnesses

## References

- Ethereum Execution Layer Specification
  (https://github.com/ethereum/execution-specs)
- revm
  (https://github.com/bluealloy/revm)
