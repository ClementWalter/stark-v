# Claudeth

Claudeth is a minimal Ethereum State Transition Function (STF) guest program
written in Rust for generating proofs of Ethereum mainnet blocks. It compiles
in `no_std` mode for the `riscv32im-unknown-none-elf` target.

## Current Status

**Production-Ready Features:**
- ✅ Complete EVM interpreter with all opcodes (arithmetic, control flow, memory, storage, logs)
- ✅ Transaction validation and execution (Legacy, EIP-2930, EIP-1559)
- ✅ Block processing with full validation (header, gas limits, roots, bloom filters)
- ✅ State root computation and validation via Merkle Patricia Trie
- ✅ Receipt generation with logs and bloom filters
- ✅ Gas metering and refunds (EIP-3529 compliant)
- ✅ Compiles to `riscv32im-unknown-none-elf` with `no_std`

**Verified:**
- ✅ EELS compliance testing (all blockchain test fixtures pass)

**In Progress:**
- ⚠️ Witness-based state reconstruction (currently accepts full state snapshots)
- ⚠️ Production validation (needs testing against real mainnet blocks)
- ⚠️ Dependency elimination (`k256` used for secp256k1)

## Architecture

Claudeth implements the Ethereum post-Fusaka fork STF and validates:
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
