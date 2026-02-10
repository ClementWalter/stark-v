## Do
- Do resolve EELS fixture parents by `block_header.parent_hash` using a hash-indexed executed-header map.
- Do pass `BLOCKHASH` inputs as an ancestry window ordered oldest -> newest with the direct parent last.
- Do keep expected-invalid blocks out of canonical state/header indexes.
- Do capture full-suite conformance baselines with `cargo test -p claudeth --release ... --ignored --nocapture` before reprioritizing fixes.
- Do redirect both stdout and stderr when collecting mismatch taxonomies, because fixture failures are often emitted with `eprintln!`.
- Do hash trie node RLP bytes with Keccak-256; never synthesize node references via zero-padding.

## Don't
- Don't use fixture iteration order as canonical chain order in multi-branch tests.
- Don't pass an empty recent-hash list into block execution when `BLOCKHASH` may be used.
- Don't index ancestry with headers from blocks that are expected to fail.
- Don't treat `<32` byte node RLP as a padded 32-byte digest surrogate.
- Don't trust baseline summaries alone for triage when error-class details were not captured from stderr.
