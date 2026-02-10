# Claudeth Development Learnings

Date: 2026-02-10

## Execution Semantics

- Exceptional halts (OOG, invalid opcode/jump, stack errors) burn remaining gas,
  return `success=false`, and do not abort block processing.
- `REVERT` is non-exceptional: returns `success=false`, preserves remaining gas,
  and reverts only the current call frame.
- `SELFDESTRUCT` follows EIP-6780: only contracts created in the same
  transaction are deleted; others only transfer balance.
- CREATE/CREATE2 enforce EIP-3860 initcode limits plus EIP-3541 and EIP-170
  code-size checks; failures consume all remaining gas.
- SSTORE gas/refund follows EIP-2200 + EIP-2929 + EIP-3529 with original vs
  current values; refunds are capped to 1/5 of gas used.
- Transient storage (EIP-1153), original storage, and created-account tracking
  reset at transaction boundaries and after pre-block system calls.

## Transaction Validation & Fees

- Sender must be an EOA (no code), signature must recover, and nonce must match
  exactly.
- Gas limit must cover intrinsic gas and not exceed the block gas limit.
- Legacy/EIP-2930 require `gas_price >= base_fee` when `base_fee > 0`.
- EIP-1559/EIP-4844 require `max_fee_per_gas >= base_fee` and
  `max_priority_fee_per_gas <= max_fee_per_gas`.
- Balance checks use max-fee caps: `gas_limit * max_fee_per_gas + value`, plus
  `blob_gas * max_fee_per_blob_gas` for type `0x03`.
- Effective gas price is `min(max_fee_per_gas, base_fee + max_priority_fee_per_gas)`.
- Sender is charged upfront for gas and blob data fee; unused gas is refunded.
- Coinbase receives only the priority fee; base fee and blob data fee are burned.
- Blob tx validation: non-empty versioned hashes, version byte `0x01`, count
  limit `6`, and `max_fee_per_blob_gas >= blob_base_fee`.

## Block Processing & Header Rules

- Validate child header against parent before any state transitions.
- Post-merge header rules: `difficulty == 0`, `mix_hash == 0`, `nonce == 0`,
  `ommers_hash == EMPTY_OMMERS_HASH`.
- `extra_data.len() <= 32`, `gas_used <= gas_limit`, gas-limit delta bounded by
  parent/1024, and minimum gas limit enforced.
- Base fee must match the EIP-1559 formula derived from the parent header.
- Blob fields are all-or-nothing; `excess_blob_gas` must match the
  parent-derived formula.
- Pre-transaction system calls: EIP-4788 beacon root and EIP-2935 history
  storage; fixed gas limit, no block gas accounting, no-op if target has no
  code.
- System-call state changes clear original-storage tracking on success to avoid
  contaminating per-transaction SSTORE accounting.
- Post-execution checks: receipts root, transactions root, logs bloom,
  withdrawals root (if present), state root, gas used, and blob gas used.
- Withdrawals list must be empty when `withdrawals_root` is `None`; otherwise
  block processing fails fast before root checks.

## Guest Input & WITNESS v1

- Input RLP list has 5-7 items and must fully consume input bytes:
  `block_header`, `parent_header`, `chain_id`, `transactions`, `state_entries`
  or `witness`, optional `block_hashes`, optional `withdrawals`.
- With 6 items, the last item is `withdrawals` only if `withdrawals_root` is
  present; otherwise it is `block_hashes`.
- Witness input is detected only when the state source element is a 3-item list
  whose first item decodes to u64 version `1`; otherwise it is treated as state
  entries.
- `withdrawals` must be provided iff `withdrawals_root` is present in the
  header; an empty list is valid.
- Recent block hashes are limited to `min(block.number, 256)`, must end with
  `parent.compute_hash()`, and genesis rejects any list.
- Witness accounts are strictly increasing by address; storage entries are
  strictly increasing by slot.
- Account trie keys are `keccak256(address)`; storage trie keys are
  `keccak256(U256 slot)`.
- Empty `account_rlp` requires an exclusion proof plus empty `code` and
  `storage_entries`.
- `code_hash` must match `keccak256(code_bytes)`; empty code requires the empty
  code hash.
- Storage proofs use `rlp::encode_u256(value)` for inclusion; zero values
  require exclusion proofs.

## Receipts, Logs, And Trie Behavior

- Receipt roots use typed envelopes (EIP-2718): `type || RLP(receipt)` for
  `0x01`, `0x02`, `0x03`; legacy receipts are plain RLP lists.
- Receipt status encoding is `0x01` for success and empty bytes (`0x80`) for
  failure.
- Logs bloom uses execution-specs bit order: reverse the 11-bit index
  (`0x07FF - bit_to_set`) and set bits MSB-first within bytes.
- Transactions root uses an MPT keyed by `RLP(tx_index)` with `RLP(tx)` values.
- Withdrawals root uses an MPT keyed by `RLP(withdrawal_index)` with
  `RLP(withdrawal)` values.
- State root is computed by sorting addresses, using `keccak256(address)` as
  trie keys, and omitting empty accounts.
- Empty trie root is `keccak256(rlp([]))` (`EMPTY_TRIE_ROOT`).
- Tests that build account tries directly must hash addresses with
  `keccak256(address)` to match production state roots and proof verification.

## Types & Crypto

- EIP-55 checksumming uses Keccak-256 over the lowercase hex address without
  the `0x` prefix; `types::Address::to_checksum_string` follows this.
- Address parsing accepts any hex case and does not validate checksum.
- EIP-55 test vectors must match canonical casing (e.g.,
  `0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed`).

## Tooling & Process

- The no-orphan Rust files hook fails if any `src/*.rs` file is unreachable.
- `prek run` may skip checks when no files are eligible; it still must be
  executed.
- `git commit` runs `prek` and will stash/restore unstaged changes; keep
  staging tight.
- Do not add shell scripts (`.sh`); use `uv run` Python scripts with PEP 723
  metadata.
- Keep documentation fork naming consistent with Cancun until a later fork is
  implemented.
- Always run `cargo test -p claudeth --release` before committing.

## Do / Don't (Next Iteration)

Do:
- Re-verify README claims against code before planning changes.
- Run `cargo test -p claudeth --release` and `prek run` before committing.
- Keep all cargo commands scoped to `-p claudeth`.
- Read execution-specs code before touching consensus-critical logic.
- Use `keccak256(address)` when constructing account tries in tests/tools.
- Update `PLAN.md` and this file when behavior changes.

Don't:
- Skip or disable pre-commit hooks.
- Add shell scripts (`.sh`) to this project.
- Assume `PLAN.md` is correct without re-checking the code and README.
- Change witness version parsing without updating input docs/tests.
