## Do
- Do treat `0x08` pairing calldata as strict 192-byte tuples (`64-byte G1 + 128-byte G2`) and fail malformed lengths immediately.
- Do decode G2 coefficients in execution-spec order: input words are `(x0, x1, y0, y1)` but field elements are interpreted as `x=(x1,x0)` and `y=(y1,y0)`.
- Do run G2 field-bound and on-curve checks before deciding pairing behavior; malformed G2 inputs must fail as exceptional precompile calls.
- Do return `U256(1)` for pairing tuples that are identity-equivalent (any tuple with infinity in G1 or G2), including non-empty inputs.
- Do keep precompile failures mapped to failed sub-calls (not caller halts) so CALL/STATICCALL value-transfer semantics remain correct.
- Do keep `cargo test -p claudeth --release` and `prek run --all-files` as mandatory gates before claiming progress.

## Don't
- Don't assume non-empty pairing input should always fail while non-trivial arithmetic is pending; infinity-only tuples are valid success cases.
- Don't hardcode G2 twist coefficient ordering without checking execution-spec semantics; a swapped coefficient silently breaks valid vectors.
- Don't claim execution-spec compatibility while `tests/eels_blockchain_tests.rs` still has ignored execution and workaround logic.
- Don't treat reserved precompile addresses as empty-code accounts; preserve explicit precompile dispatch and failure semantics.
- Don't start `0x0a` point-evaluation implementation without a concrete proof-verification path aligned to reference behavior.
