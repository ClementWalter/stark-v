## Do
- Do model Prague headers with `requests_hash` and keep it in canonical RLP order (after `parent_beacon_block_root`) to preserve fixture hash parity.
- Do encode header `nonce` as an 8-byte RLP byte string (`Bytes8`), not as an integer, when comparing against execution-spec fixtures.
- Do convert EELS type-`0x03` fixture transactions into `Transaction::Blob` with strict required fields: `chainId`, `maxFeePerGas`, `maxPriorityFeePerGas`, `maxFeePerBlobGas`, and `blobVersionedHashes`.
- Do keep fixture conversion strict and fail loudly on malformed blob fields so fixture-shape drift is detected immediately.
- Do validate harness behavior with real fixture-backed tests (not synthetic-only cases), especially for consensus-critical encoding and transaction-type coverage.
- Do run `cargo test -p claudeth --release` and `prek run --all-files` before committing.

## Don't
- Don't rewrite fixture `parentHash` values in the harness; that hides real consensus bugs.
- Don't assume Cancun-era fixture parsing is enough for Prague/Cancun blob suites; missing type `0x03` support silently drops conformance surface.
- Don't treat a passing unit-test suite as conformance proof while `test_execute_all_blockchain_tests` is ignored or non-fatal.
- Don't prioritize speculative optimizations before fixing deterministic mismatch classes first (parent linkage, gas used, state root, withdrawals root).
