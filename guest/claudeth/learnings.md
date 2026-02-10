## Do
- Do treat post-merge `mix_hash` as `prev_randao`: enforce `difficulty == 0`, `nonce == 0`, and empty ommers hash, but allow non-zero mix hash.
- Do parse and pass fixture withdrawals into `process_block`; using `vec![]` for every block invalidates Shanghai/Cancun body validation.
- Do validate consensus constants (especially trie roots) against execution-spec vectors before trusting internal constants.
- Do run `cargo test -p claudeth --release` and `prek run --all-files` before finalizing changes.
- Do use ignored EELS blockchain execution as a diagnostic signal to identify the current highest-frequency failure class.
- Do keep precompile failures mapped to EVM sub-call failure semantics (CALL/STATICCALL fail without halting caller execution).

## Don't
- Don't enforce `mix_hash == 0` post-merge; that rejects valid Cancun/Prague fixtures at header validation.
- Don't leave fixture harness shortcuts (like hardcoded empty withdrawals) in place when evaluating execution-spec conformance.
- Don't assume internal trie constants are correct without cross-checking the canonical Ethereum values.
- Don't rely on unit-test green status alone as proof of execution-spec compatibility while blockchain integration remains ignored.
- Don't keep parent-hash rewrite workarounds in the long-term conformance path.
- Don't start `0x0a` point-evaluation implementation without a spec-aligned KZG verification path.
