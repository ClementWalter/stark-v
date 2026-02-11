## Do
- Do resolve EELS fixture parents by `block_header.parent_hash` using a hash-indexed executed-header map.
- Do resolve EELS fixture parent **state snapshots** by `block_header.parent_hash` (hash-indexed state map), not only parent headers.
- Do pass `BLOCKHASH` ancestry as oldest -> newest with direct parent last.
- Do exclude expected-invalid blocks from canonical header/state indexes.
- Do validate fixture post-state against the state snapshot at fixture `lastblockhash`, not the last loop-visited block.
- Do baseline full ignored EELS runs with `cargo test -p claudeth --release ... --ignored --nocapture` before reprioritizing fixes.
- Do hash trie node RLP bytes with Keccak-256 when a 32-byte node reference is required.
- Do implement trie child references with the execution-spec threshold rule: inline encoded child when `<32` bytes, hash otherwise.
- Do key withdrawals trie entries by list position (`enumerate` index), not by `withdrawal.index`.
- Do implement `SELFDESTRUCT` dynamic gas from pre-transfer state: base + cold beneficiary surcharge + conditional new-account surcharge.
- Do return `0` from `EXTCODEHASH` when the target account is empty/non-existent (execution-spec `EMPTY_ACCOUNT` behavior).
- Do return `keccak256("")` from `EXTCODEHASH` only for alive accounts whose code is empty.

## Don't
- Don't use fixture iteration order as canonical chain order in multi-branch tests.
- Don't execute forked-chain blocks on a single linear mutable state.
- Don't pass an empty recent-hash list into execution when `BLOCKHASH` can be reached.
- Don't index ancestry with headers from blocks that are expected to fail.
- Don't synthesize trie node references by zero-padding short RLP payloads.
- Don't assume generic trie helpers are execution-spec-compatible for all root types without checking reference child-node encoding rules.
- Don't treat `SELFDESTRUCT` as a fixed `5000` gas opcode in Cancun/Prague.
- Don't skip `SELFDESTRUCT` new-account surcharge just because the beneficiary is warm.
- Don't treat `EXTCODEHASH` of a dead account as `keccak256("")`.
