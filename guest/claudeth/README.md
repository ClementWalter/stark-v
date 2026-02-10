# Claudeth

Claudeth is a minimal Ethereum State Transition Function (STF) guest program
written in Rust for the stark-v zkVM. It targets `no_std` and
`riscv32im-unknown-none-elf`.

## Current Status (2026-02-10)

**Implemented**

- EVM interpreter with full opcode coverage, including Cancun opcodes
  (`BLOBHASH`, `BLOBBASEFEE`, `PREVRANDAO`) and transient storage
  (`TLOAD`, `TSTORE`)
- Transaction validation and execution for Legacy, EIP-2930, and EIP-1559
- EIP-4844 blob transaction type `0x03` encoding/decoding and signing hash
- Block processing with header validation and root checks
  (state, receipts, transactions, logs bloom)
- EIP-4895 withdrawals application and withdrawals root validation
- EIP-4788 beacon root system call during block processing
- EIP-2935 historical block hashes system call during block processing
- Guest input decoding supports optional recent block hashes for BLOCKHASH
- Partial Merkle Patricia Trie for account/storage roots and proofs
- Witness-based state reconstruction from WITNESS v1 (account/storage proofs)
- Block header type supports Shanghai/Cancun fields
  (`withdrawals_root`, `blob_gas_used`, `excess_blob_gas`,
  `parent_beacon_block_root`)
- Blob gas accounting and data-fee charging (EIP-4844)
- In-tree deterministic secp256k1 signer for tests

**Known Gaps / Limitations**

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
- Reconstructing minimal state from witnesses

## References

- Ethereum Execution Layer Specification
  (https://github.com/ethereum/execution-specs)
- revm
  (https://github.com/bluealloy/revm)
