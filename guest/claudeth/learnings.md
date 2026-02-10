# Claudeth Development Learnings

Date: 2026-02-10

## Consensus-Critical Execution

- Exceptional halts (OOG, invalid opcode/jump) consume all remaining gas and revert the current transaction.
- `REVERT` is non-exceptional: it returns `success=false`, preserves remaining gas, and only reverts the current call frame.
- Gas refunds are capped at 1/5 of gas used (EIP-3529) and applied after execution.
- Transactions must originate from EOAs; sender accounts with code are invalid.
- SELFDESTRUCT (EIP-6780): transfer full balance immediately; delete only if created in the same transaction, with created-account tracking cleared per tx.

## Block Processing Order

- Validate the child header against its parent before any state transitions.
- Apply EIP-4788 (beacon root) and EIP-2935 (history storage) system calls before transaction execution.
- Post-execution checks include receipts root, transactions root, logs bloom, withdrawals root (if present), state root, gas used, and blob gas used.

## Header Rules (Post-Merge / Cancun)

- Enforce `difficulty == 0`, `mix_hash == 0`, `nonce == 0`, and `ommers_hash == EMPTY_OMMERS_HASH`.
- `extra_data.len() <= 32`, `gas_used <= gas_limit`, and gas limit delta bounded by parent/1024 with a minimum floor.
- Base fee must match the EIP-1559 formula derived from the parent header.
- Blob fields are all-or-nothing: `blob_gas_used` and `excess_blob_gas` must appear together, and `excess_blob_gas` must match the parent-derived formula.
- `BLOBBASEFEE` uses the execution-specs Taylor expansion when `excess_blob_gas` is present.

## Guest Input and WITNESS v1

- Input RLP list has 5–7 items: `block_header`, `parent_header`, `chain_id`, `transactions`, `state_entries` or `witness`, optional `block_hashes`, optional `withdrawals`.
- Witness input is detected by a top-level list of 3 items where the first is a u64 version (currently `1`).
- `withdrawals` must be provided iff `withdrawals_root` is present in the header; empty list is valid.
- Recent block hashes are capped at 256 and must end with `parent.compute_hash()`. Genesis (`block.number == 0`) rejects any list.
- Witness accounts are sorted by ascending address; storage entries are sorted by slot.
- Account trie keys are `keccak256(address)`; storage trie keys are `keccak256(U256 slot)`.
- Empty `account_rlp` requires an exclusion proof and empty `code` + `storage_entries`.
- `code_hash` must match `keccak256(code_bytes)`; empty code requires the empty code hash.
- Storage proofs use `rlp::encode_u256(value)` for inclusion; zero values require exclusion proofs.

## Transactions, Fees, and Blobs

- Effective gas price for EIP-1559/EIP-4844 is `min(max_fee_per_gas, base_fee + max_priority_fee_per_gas)`.
- Legacy/EIP-2930 require `gas_price >= base_fee`; EIP-1559/EIP-4844 require `max_fee_per_gas >= base_fee` and `max_priority_fee_per_gas <= max_fee_per_gas`.
- Balance checks use the max-fee cap: `gas_limit * max_fee_per_gas + value` (plus blob fee cap for type `0x03`).
- Blob tx validation: non-empty blob hashes, KZG version byte `0x01`, blob count limit, and `max_fee_per_blob_gas >= blob_base_fee`.
- Blob data fee is charged upfront and burned (not credited to coinbase).
- EIP-3860: creation tx initcode > 49,152 bytes is invalid; CREATE/CREATE2 oversize initcode returns 0 after charging gas.
- EIP-170 max code size and code-deposit gas charging apply to CREATE/CREATE2.
- EIP-3541 rejects contract code starting with `0xEF`, consuming all remaining gas on failure.

## Receipts, Logs, and Bloom

- Receipt roots use typed receipt envelopes (EIP-2718): `type || RLP(receipt)` for `0x01`, `0x02`, `0x03`.
- Receipt decoding accepts typed envelopes for `0x01..0x03` and rejects unknown prefixes; legacy receipts are plain RLP lists.
- Logs bloom uses execution-specs bit order: reverse the 11-bit index (`0x07FF - bit_to_set`) and set bits MSB-first within bytes.

## State Roots and Trie Behavior

- State root is computed by sorting addresses, using `keccak256(address)` as trie keys, and omitting empty accounts.
- Empty trie root is `keccak256(rlp([]))` (`EMPTY_TRIE_ROOT`).

## secp256k1 In-Tree Crypto

- Constants are fixed (p, n, b); field ops include add/sub/mul/pow/inv.
- Affine point arithmetic handles infinity explicitly; doubling with `y == 0` yields infinity.
- ECDSA recovery uses `x = r` (or `r + n` for high recid), quadratic-residue check, sqrt via `(p + 1) / 4`, and y-parity selection.
- ECDSA verify uses `u1 = z*s^-1`, `u2 = r*s^-1`, then checks `x(u1G + u2Q)`.
- Deterministic signing uses `keccak256(secret_key || msg_hash || attempt)` for nonce selection; retry if `r,s` invalid.
- For Ethereum transactions, keep `v` in `{27,28}` for legacy and `{0,1}` for typed txs; reject or retry if recovery id would require `x >= n`.

## Tooling and Process

- The no-orphan Rust files hook fails if any `src/*.rs` file is unreachable.
- Always run `cargo test -p claudeth --release` and `prek run` before committing.

## Known Gaps

- EELS blockchain fixtures are external and ignored by default.

## Do / Don't (Next Iteration)

**Do**
- Read the relevant `execution-specs` implementation before changing consensus-critical logic.
- Update `PLAN.md` and `learnings.md` when behavior changes.
- Keep all cargo commands scoped to `-p claudeth`.
- Keep WITNESS version detection and `WITNESS.md` in sync.

**Don't**
- Skip or disable pre-commit hooks.
- Add shell scripts (`.sh`) to this project.
- Assume `PLAN.md` is correct without re-checking the code and README.
- Change witness version parsing without updating input docs/tests.
