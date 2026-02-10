# Claudeth Development Learnings

Date: 2026-02-10

## Consensus-Critical STF Behavior

- Exceptional halts (OOG, invalid opcode/jump) revert the current transaction and consume all remaining gas.
- `REVERT` is non-exceptional: it returns `success=false`, preserves remaining gas, and only reverts the current call frame.
- Gas refunds are capped at 1/5 of gas used (EIP-3529); refunds are applied after execution.

## Block Processing Order

- Validate child header against parent before any state transitions.
- Apply EIP-4788 (beacon root) and EIP-2935 (history storage) system calls before transaction execution.
- Post-execution checks: receipts root, transactions root, logs bloom, withdrawals root (if present), state root, gas used, blob gas used.

## Header and Cancun Rules

- Post-merge headers enforce `difficulty == 0`, `mix_hash == 0`, `nonce == 0`, and `ommers_hash == EMPTY_OMMERS_HASH`.
- `extra_data.len() <= 32` and `gas_used <= gas_limit`.
- Blob fields are all-or-nothing: `blob_gas_used` and `excess_blob_gas` must appear together.
- `BLOBBASEFEE` uses the execution-specs Taylor expansion when `excess_blob_gas` is present.

## Guest Input and WITNESS v1

- Input RLP list has 5–7 items: `block_header`, `parent_header`, `chain_id`, `transactions`, `state_entries` or `witness`, optional `block_hashes`, optional `withdrawals`.
- `withdrawals` must be provided iff `withdrawals_root` is present in the header; empty list is valid.
- Recent block hashes are capped at 256 and must end with `parent.compute_hash()`. Genesis (`block.number == 0`) rejects any list.
- Witness accounts are sorted by ascending address with no duplicates; storage entries sorted by slot.
- Account trie keys are `keccak256(address)`; storage trie keys are `keccak256(U256 slot)`.
- Empty `account_rlp` requires exclusion proof and empty `code` + `storage_entries`.
- `code_hash` must match `keccak256(code_bytes)`; empty code requires the empty code hash.
- Storage proofs use `rlp::encode_u256(value)` for inclusion; zero values require exclusion proofs.

## Transactions, Fees, and Blobs

- Typed transactions accepted: `0x01`, `0x02`, `0x03`.
- Effective gas price for EIP-1559/EIP-4844 is `min(max_fee_per_gas, base_fee + max_priority_fee_per_gas)`.
- Base fee caps: legacy/EIP-2930 require `gas_price >= base_fee`; EIP-1559/EIP-4844 require `max_fee_per_gas >= base_fee`.
- EIP-3860: contract creation txs with initcode > 49,152 bytes are invalid; CREATE/CREATE2 oversize initcode returns 0 after charging gas (no initcode execution).
- Blob tx validation enforces non-empty blob hashes, KZG version byte `0x01`, blob count limit, and `max_fee_per_blob_gas >= blob_base_fee`.
- Blob data fee is charged upfront and burned (not credited to coinbase).
- `TxContext` carries `blob_versioned_hashes`; `BLOBHASH` returns zero for out-of-range indices.
- `execute_transaction` must call `validate_blob_structure` directly because block processing does not use `validate_transaction`.

## EVM Semantics and State Roots

- Logs bloom follows execution-specs bit order: reverse the 11-bit index (`0x07FF - bit_to_set`) and set bits MSB-first within bytes.
- Contract creation rejects code starting with `0xEF` (EIP-3541) and consumes all remaining gas.
- Contract creation charges code-deposit gas (200 per byte) and rejects code larger than 24KB (EIP-170), consuming all remaining gas.
- State root is computed by sorting addresses, using `keccak256(address)` as trie keys, and omitting empty accounts.
- Empty trie root is `keccak256(rlp([]))` (`EMPTY_TRIE_ROOT`).

## secp256k1 In-Tree Crypto

- Constants are fixed (p, n, b). Field ops include add/sub/mul/pow/inv.
- Affine point arithmetic handles infinity explicitly; doubling with `y == 0` yields infinity.
- ECDSA recovery uses x = r (or r + n for high recid), quadratic-residue check, sqrt via `(p + 1) / 4`, and y-parity selection.
- ECDSA verify uses `u1 = z*s^-1`, `u2 = r*s^-1`, then checks `x(u1G + u2Q)`.
- Deterministic in-tree signing uses keccak256(secret_key || msg_hash || attempt) to pick a nonce and loops until `r,s` are valid.
- For Ethereum transactions, keep `v` in `{27,28}` for legacy and `{0,1}` for typed txs; reject or retry if recovery id would require `x >= n`.

## Pre-commit Hygiene

- The no-orphan Rust files hook fails if any `src/*.rs` file is unreachable.
- Always run `cargo test -p claudeth --release` and `prek run` before committing.

## Known Gaps Observed

- EELS blockchain fixtures are external and ignored by default.

## Do / Don't (Next Iteration)

**Do**
- Keep EIP-4788 and EIP-2935 system calls before transaction execution.
- Enforce witness ordering and proof validation for accounts and storage slots.
- Validate base fee caps per tx type before charging balances.
- Validate blob versioned hashes in `execute_transaction`, not just in standalone validation.
- Enforce EIP-3860 initcode size limits in transaction validation and CREATE/CREATE2 handling.
- Enforce EIP-170 max code size and charge code-deposit gas for CREATE/CREATE2.
- Charge blob data fees upfront and burn them (not coinbase).
- Sort addresses before computing state roots and use `keccak256(address)` as trie keys.
- Enforce EIP-2 signature bounds and correct `v/y_parity` handling.
- Use the in-tree deterministic signer in tests; derive expected sender from the secret key.

**Don't**
- Treat EVM `REVERT` as exceptional.
- Accept recent block hash lists for genesis or lists without the parent hash last.
- Skip blob field all-or-nothing checks or blob count/version validations.
- Ignore point-at-infinity or `y == 0` edge cases in secp256k1 arithmetic.
