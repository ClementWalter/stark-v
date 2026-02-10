# Claudeth Development Learnings

Date: 2026-02-10

## Consensus-Critical Execution Semantics

- Exceptional halts (OOG, invalid opcode/jump, stack errors) consume all remaining gas and return `success=false` while block processing continues; only successful executions commit state.
- `REVERT` is non-exceptional: it returns `success=false`, preserves remaining gas, and reverts only the current call frame.
- `SELFDESTRUCT` follows EIP-6780: transfer balance immediately, delete only if created in the same transaction, and reset created-account tracking per transaction.
- CREATE/CREATE2 enforce EIP-3860 initcode limits plus EIP-3541 and EIP-170 code-size checks; failures consume all remaining gas.
- SSTORE gas/refunds follow EIP-2200 + EIP-2929 + EIP-3529 using original vs current values; refunds are capped to 1/5 of gas used.
- Original storage tracking must reset at tx boundaries and after pre-block system calls.
- Coinbase only receives priority fees; base fee and blob data fee are burned.

## Transaction Validation Rules

- Sender must be an EOA (no code), signature must recover, and nonce must match exactly (too low/high both reject).
- Gas limit must cover intrinsic gas and not exceed the block gas limit.
- Legacy/EIP-2930 require `gas_price >= base_fee`.
- EIP-1559/EIP-4844 require `max_fee_per_gas >= base_fee` and `max_priority_fee_per_gas <= max_fee_per_gas`.
- Balance checks use the max-fee cap: `gas_limit * max_fee_per_gas + value` (+ blob fee cap for type `0x03`).
- Effective gas price is `min(max_fee_per_gas, base_fee + max_priority_fee_per_gas)`.
- Chain ID rules: legacy uses EIP-155 encoding in `v`, typed txs use explicit `chain_id`.
- Blob tx validation: non-empty versioned hashes, version byte `0x01`, count limit `6`, and `max_fee_per_blob_gas >= blob_base_fee` (requires excess blob gas in block context).

## Block Processing And Header Rules

- Validate child header against parent before any state transitions.
- Post-merge header rules: `difficulty == 0`, `mix_hash == 0`, `nonce == 0`, `ommers_hash == EMPTY_OMMERS_HASH`.
- `extra_data.len() <= 32`, `gas_used <= gas_limit`, gas-limit delta bounded by parent/1024, and minimum gas limit enforced.
- Base fee must match the EIP-1559 formula derived from the parent header.
- Blob fields are all-or-nothing; `excess_blob_gas` must match the parent-derived formula.
- Apply EIP-4788 (beacon root) and EIP-2935 (history storage) system calls before transactions; fixed gas limit, no block gas accounting, no-op if target has no code.
- Post-execution checks: receipts root, transactions root, logs bloom, withdrawals root (if present), state root, gas used, and blob gas used.

## Guest Input And WITNESS v1

- Input RLP list has 5-7 items and must fully consume the input bytes: `block_header`, `parent_header`, `chain_id`, `transactions`, `state_entries` or `witness`, optional `block_hashes`, optional `withdrawals`.
- With 6 items, the last item is `withdrawals` only if `withdrawals_root` is present; otherwise it is `block_hashes`.
- Witness input is detected by a top-level list of 3 items where the first item decodes to u64 version `1`.
- `withdrawals` must be provided iff `withdrawals_root` is present in the header; an empty list is valid.
- Recent block hashes are limited to `min(block.number, 256)`, must end with `parent.compute_hash()`, and genesis rejects any list.
- Witness accounts are strictly increasing by address; storage entries are strictly increasing by slot.
- Account trie keys are `keccak256(address)`; storage trie keys are `keccak256(U256 slot)`.
- Empty `account_rlp` requires an exclusion proof and empty `code` + `storage_entries`.
- `code_hash` must match `keccak256(code_bytes)`; empty code requires the empty code hash.
- Storage proofs use `rlp::encode_u256(value)` for inclusion; zero values require exclusion proofs.

## Receipts, Logs, And Trie Behavior

- Receipt roots use typed envelopes (EIP-2718): `type || RLP(receipt)` for `0x01`, `0x02`, `0x03`; legacy receipts are plain RLP lists.
- Receipt status encoding is `0x01` for success and empty bytes (`0x80`) for failure.
- Logs bloom uses execution-specs bit order: reverse the 11-bit index (`0x07FF - bit_to_set`) and set bits MSB-first within bytes.
- State root is computed by sorting addresses, using `keccak256(address)` as trie keys, and omitting empty accounts.
- Empty trie root is `keccak256(rlp([]))` (`EMPTY_TRIE_ROOT`).

## secp256k1 In-Tree Crypto

- Fixed constants (p, n, b); affine point arithmetic handles infinity and doubling with `y == 0` => infinity.
- ECDSA recovery uses `x = r` (or `r + n` for high recid), quadratic-residue check, sqrt via `(p + 1) / 4`, and y-parity selection.
- ECDSA verify uses `u1 = z*s^-1`, `u2 = r*s^-1`, then checks `x(u1G + u2Q)`.
- Deterministic signing uses `keccak256(secret_key || msg_hash || attempt)` for nonce selection; retry if `r,s` invalid.
- For Ethereum txs, keep `v` in `{27,28}` for legacy and `{0,1}` for typed txs; reject if recovery id would require `x >= n`.

## Tooling And Process

- The no-orphan Rust files hook fails if any `src/*.rs` file is unreachable.
- `prek run` may skip checks when no files are eligible; it still must be executed.
- `git commit` runs `prek` and will stash/restore unstaged changes; keep staging tight.
- Do not add shell scripts (`.sh`); use `uv run` Python scripts with PEP 723 metadata.
- Keep fork naming consistent with Cancun in docs and comments unless a later fork is explicitly implemented.

## Do / Don't (Next Iteration)

Do:
- Re-verify README claims against code before planning changes.
- Run `cargo test -p claudeth --release` and `prek run` before committing.
- Keep all cargo commands scoped to `-p claudeth`.
- Read execution-specs code before touching consensus-critical logic.
- Update `PLAN.md` and this file when behavior changes.

Don't:
- Skip or disable pre-commit hooks.
- Add shell scripts (`.sh`) to this project.
- Assume `PLAN.md` is correct without re-checking the code and README.
- Change witness version parsing without updating input docs/tests.
