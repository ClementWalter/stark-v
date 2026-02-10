## Do
- Do treat the block header `mix_hash` field as `prev_randao` post-merge; keep `difficulty == 0`, `nonce == 0`, and empty ommers checks, but allow non-zero mix hash.
- Do validate consensus rules against `execution-specs` source files before locking invariants into `BlockHeader::validate_post_merge_fields`.
- Do run `cargo test -p claudeth --release` and `prek run --all-files` before claiming a task is complete.
- Do use ignored EELS blockchain runs to find the highest-leverage failure class; fix the dominant gate first.
- Do keep precompile failures mapped to sub-call failure semantics (CALL/STATICCALL fail without halting caller execution).
- Do keep `0x08` pairing input strict (192-byte tuples), including canonical G2 decoding order and curve checks.

## Don't
- Don't enforce `mix_hash == 0` on post-merge headers; this rejects valid Cancun/Prague fixtures before execution.
- Don't trust a passing unit-test suite alone as evidence of execution-spec compatibility when blockchain integration remains ignored.
- Don't keep workaround logic (like parent-hash rewriting) in a harness you use to claim conformance.
- Don't claim full execution-spec compatibility while dominant fixture classes still fail (currently withdrawals-root mismatch after header fix).
- Don't start `0x0a` point-evaluation implementation without an execution-spec-aligned KZG verification path.
- Don't conflate reserved/future precompile behavior with generic empty-account call semantics without fork-aware rules.
