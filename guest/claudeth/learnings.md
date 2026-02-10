## Do
- Do model post-Prague headers with `requests_hash` (EIP-7685) and include it in RLP/hash ordering after `parent_beacon_block_root`.
- Do encode block-header `nonce` as `Bytes8` in RLP (fixed 8-byte string), not as a uint; this is required for fixture hash parity.
- Do anchor header-hash behavior to real fixture vectors (for example Cancun + Prague entries from `bcExample/shanghaiExample.json`) so parent-linkage bugs are caught immediately.
- Do remove harness workarounds once parity is fixed; use real fixture `parentHash` links as the validation source of truth.
- Do run `cargo test -p claudeth --release` and `prek run --all-files` before committing.

## Don't
- Don't treat `parentHash` rewrites in the EELS harness as acceptable; they hide consensus-critical header encoding defects.
- Don't assume Cancun-era header fields are sufficient for Prague fixtures; missing `requestsHash` breaks block-hash computation deterministically.
- Don't parse or serialize header `nonce` with integer RLP helpers when comparing against execution-spec fixtures.
- Don't rely on synthetic hash tests only; keep fixture-backed regression tests for real-world header layouts.
